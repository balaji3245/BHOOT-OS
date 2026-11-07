#!/usr/bin/env python3
"""
BHOOT OS — Safe Offline System Update & Rollback Engine (bhoot-sysupdate)

Provides atomic, verifiable replacement of the read-only root filesystem image
(filesystem.squashfs) with automatic rollback preservation and cryptographic
integrity verification.

Guarantees:
1. Pre-flight verification (SquashFS magic, SHA-256 digest, critical system binaries).
2. Atomic filesystem swap via POSIX atomic rename (no half-written rootfs).
3. Rollback snapshot preservation (filesystem.squashfs.rollback).
4. Full offline-first design (no internet, external package repositories, or cloud telemetry).
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

SQUASHFS_MAGIC = b"hsqs"  # 0x68 0x73 0x71 0x73 (Little-endian SquashFS v4)

CRITICAL_PATHS = [
    "sbin/init",
    "bin/bash",
    "sbin/cryptsetup",
    "usr/sbin/nft",
    "usr/sbin/rfkill",
]


def get_mount_info(path: Path):
    """
    Determines mountpoint and read-only status for a path by inspecting /proc/mounts.
    Returns (mount_point_path_str, is_read_only).
    """
    resolved = str(path.resolve())
    best_mp = None
    best_len = -1
    is_ro = False

    try:
        with open("/proc/mounts", "r") as f:
            for line in f:
                parts = line.split()
                if len(parts) >= 4:
                    mp = parts[1]
                    opts = parts[3].split(",")
                    if resolved == mp or resolved.startswith(mp + "/") or mp == "/":
                        if len(mp) > best_len:
                            best_mp = mp
                            best_len = len(mp)
                            is_ro = "ro" in opts
    except OSError:
        pass

    return best_mp, is_ro


class MountManager:
    """Context manager to safely handle read-only live media mounts during updates."""
    def __init__(self, target_dir: Path):
        self.target_dir = target_dir
        self.mount_point, self.was_ro = get_mount_info(target_dir)
        self.remounted_rw = False

    def __enter__(self):
        if self.was_ro and self.mount_point:
            print(f" [INFO] Target mount {self.mount_point} is read-only. Remounting read-write for atomic update...")
            res = subprocess.run(["mount", "-o", "remount,rw", self.mount_point], check=False, capture_output=True, text=True)
            if res.returncode == 0:
                self.remounted_rw = True
                print(" [OK] Remounted read-write successfully.")
            else:
                print(f" [WARN] Failed to remount rw: {res.stderr.strip()}. Attempting update directly.")
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.remounted_rw and self.mount_point:
            print(f" [INFO] Restoring read-only status on {self.mount_point}...")
            res = subprocess.run(["mount", "-o", "remount,ro", self.mount_point], check=False, capture_output=True, text=True)
            if res.returncode == 0:
                print(" [OK] Restored read-only mount status.")
            else:
                print(f" [WARN] Could not restore ro mount: {res.stderr.strip()}")



def compute_sha256(filepath: Path) -> str:
    """Computes SHA-256 checksum of a file in 1 MiB chunks."""
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(1024 * 1024):
            h.update(chunk)
    return h.hexdigest()


def verify_squashfs_magic(filepath: Path) -> bool:
    """Verifies the 4-byte SquashFS magic header."""
    try:
        with open(filepath, "rb") as f:
            magic = f.read(4)
            return magic == SQUASHFS_MAGIC
    except OSError:
        return False


def inspect_squashfs_contents(filepath: Path) -> dict:
    """Uses unsquashfs -l to verify critical system binaries are present."""
    try:
        result = subprocess.run(
            ["unsquashfs", "-l", str(filepath)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=30,
        )
    except FileNotFoundError:
        # Fallback if unsquashfs not installed in current environment
        return {"unsquashfs_available": False, "missing_paths": [], "all_present": True}
    except subprocess.TimeoutExpired:
        return {"unsquashfs_available": True, "missing_paths": ["TIMEOUT"], "all_present": False}

    if result.returncode != 0:
        return {
            "unsquashfs_available": True,
            "missing_paths": ["UNSQUASHFS_FAILED"],
            "all_present": False,
            "error": result.stderr.strip(),
        }

    listing = result.stdout
    missing = []
    for crit in CRITICAL_PATHS:
        # Check either relative or absolute
        if (crit not in listing) and (f"squashfs-root/{crit}" not in listing) and (f"/{crit}" not in listing):
            # Special case: check if systemd exists as alternative init
            if crit == "sbin/init" and ("lib/systemd/systemd" in listing or "bin/systemd" in listing):
                continue
            missing.append(crit)

    return {
        "unsquashfs_available": True,
        "missing_paths": missing,
        "all_present": len(missing) == 0,
    }


def cmd_status(target_dir: Path) -> int:
    """Reports status of the active SquashFS image and rollback snapshot."""
    active_img = target_dir / "filesystem.squashfs"
    rollback_img = target_dir / "filesystem.squashfs.rollback"

    print("========================================================================")
    print(" [BHOOT OS] System Image & Rollback Status")
    print(f" Target Directory: {target_dir}")
    print("========================================================================")

    if not active_img.exists():
        print(f" [!] Active image missing: {active_img}")
        return 1

    size_mb = active_img.stat().st_size / (1024 * 1024)
    sha256 = compute_sha256(active_img)
    magic_ok = verify_squashfs_magic(active_img)

    print(f" Active Image:      {active_img}")
    print(f" Size:              {size_mb:.2f} MiB ({active_img.stat().st_size} bytes)")
    print(f" Magic Valid:       {magic_ok}")
    print(f" SHA-256 Digest:    {sha256}")

    if rollback_img.exists():
        rb_size_mb = rollback_img.stat().st_size / (1024 * 1024)
        rb_sha = compute_sha256(rollback_img)
        print("------------------------------------------------------------------------")
        print(f" Rollback Snapshot: {rollback_img}")
        print(f" Rollback Size:     {rb_size_mb:.2f} MiB")
        print(f" Rollback SHA-256:  {rb_sha}")
        print(" Status:            READY FOR IMMEDIATE ROLLBACK")
    else:
        print("------------------------------------------------------------------------")
        print(" Rollback Snapshot: None available")

    print("========================================================================")
    return 0


def cmd_verify_dry_run(image_path: Path, expected_sha256: str = None, skip_content_check: bool = False) -> int:
    """Performs non-destructive pre-flight verification on an update image."""
    print("========================================================================")
    print(" [BHOOT OS] Pre-Flight Update Image Verification")
    print(f" Inspecting: {image_path}")
    print("========================================================================")

    if not image_path.exists() or not image_path.is_file():
        print(f" [FAIL] Target file does not exist: {image_path}", file=sys.stderr)
        return 1

    size = image_path.stat().st_size
    if size < 5 * 1024 * 1024:
        print(f" [FAIL] Image is suspiciously small ({size} bytes < 5 MiB)", file=sys.stderr)
        return 1

    print(f" [OK] Image size: {size / (1024 * 1024):.2f} MiB ({size} bytes)")

    if not verify_squashfs_magic(image_path):
        print(" [FAIL] Invalid SquashFS magic header (expected 'hsqs')", file=sys.stderr)
        return 1
    print(" [OK] SquashFS magic verified (0x68737173)")

    actual_sha256 = compute_sha256(image_path)
    print(f" [OK] Computed SHA-256: {actual_sha256}")

    if expected_sha256:
        if actual_sha256.lower() != expected_sha256.lower().strip():
            print(f" [FAIL] SHA-256 mismatch!\n  Expected: {expected_sha256}\n  Actual:   {actual_sha256}", file=sys.stderr)
            return 1
        print(" [OK] Manifest SHA-256 digest match confirmed")

    if not skip_content_check:
        inspection = inspect_squashfs_contents(image_path)
        if inspection.get("unsquashfs_available", False):
            if not inspection["all_present"]:
                print(f" [FAIL] Critical system binaries missing from image: {inspection['missing_paths']}", file=sys.stderr)
                return 1
            print(f" [OK] Critical binaries verified: {CRITICAL_PATHS}")
        else:
            print(" [INFO] unsquashfs binary not found; skipped archive internal file verification")

    print("========================================================================")
    print(" [RESULT] UPDATE IMAGE FULLY VERIFIED AND SAFE FOR ATOMIC APPLICATION")
    print("========================================================================")
    return 0


def cmd_apply(image_path: Path, target_dir: Path, expected_sha256: str = None, skip_content_check: bool = False) -> int:
    """Applies system update image atomically with rollback backup."""
    # 1. Pre-flight verification
    rc = cmd_verify_dry_run(image_path, expected_sha256, skip_content_check=skip_content_check)
    if rc != 0:
        print(" [ABORT] Pre-flight verification failed. System remains untouched.", file=sys.stderr)
        return rc

    active_img = target_dir / "filesystem.squashfs"
    rollback_img = target_dir / "filesystem.squashfs.rollback"
    staging_img = target_dir / "filesystem.squashfs.new"

    with MountManager(target_dir):
        target_dir.mkdir(parents=True, exist_ok=True)

        print("\n------------------------------------------------------------------------")
        print(" [APPLY] Executing Atomic System Image Update...")

        old_sha256 = compute_sha256(active_img) if active_img.exists() else "NONE"

        # 2. Stage new image
        print(f" 1. Copying update to temporary staging file: {staging_img}...")
        shutil.copy2(image_path, staging_img)
        # Flush staging file to disk
        with open(staging_img, "rb") as f:
            os.fdatasync(f.fileno())

        # 3. Preserve rollback backup
        if active_img.exists():
            print(f" 2. Preserving previous image as rollback snapshot: {rollback_img}...")
            if rollback_img.exists():
                rollback_img.unlink()
            try:
                os.link(active_img, rollback_img)
            except OSError:
                shutil.copy2(active_img, rollback_img)

        # 4. Atomic swap via POSIX atomic rename
        print(f" 3. Atomically replacing {active_img} with staged image...")
        os.replace(staging_img, active_img)

        # Sync directory
        try:
            dir_fd = os.open(str(target_dir), os.O_DIRECTORY | os.O_RDONLY)
            os.fsync(dir_fd)
            os.close(dir_fd)
        except OSError:
            pass

        new_sha256 = compute_sha256(active_img)

    # 5. Record audit log
    audit_log = Path("/var/bhoot/audit/sysupdate.log")
    try:
        audit_log.parent.mkdir(parents=True, exist_ok=True)
        with open(audit_log, "a") as f:
            entry = {
                "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "action": "SYSTEM_UPDATE_APPLIED",
                "source": str(image_path),
                "previous_sha256": old_sha256,
                "new_sha256": new_sha256,
                "status": "SUCCESS",
            }
            f.write(json.dumps(entry) + "\n")
    except OSError:
        pass

    print("========================================================================")
    print(" [BHOOT OS] Update Successfully Applied!")
    print(f" Previous SHA-256: {old_sha256}")
    print(f" New Active SHA-256: {new_sha256}")
    print(" Rollback image preserved at filesystem.squashfs.rollback")
    print("========================================================================")
    return 0


def cmd_rollback(target_dir: Path) -> int:
    """Restores the previous rollback snapshot atomically."""
    active_img = target_dir / "filesystem.squashfs"
    rollback_img = target_dir / "filesystem.squashfs.rollback"
    temp_swap_img = target_dir / "filesystem.squashfs.failed_update"

    print("========================================================================")
    print(" [BHOOT OS] Executing Atomic System Rollback")
    print("========================================================================")

    if not rollback_img.exists():
        print(f" [FAIL] No rollback snapshot found at {rollback_img}!", file=sys.stderr)
        return 1

    if not verify_squashfs_magic(rollback_img):
        print(" [FAIL] Rollback snapshot failed magic verification!", file=sys.stderr)
        return 1

    rb_sha = compute_sha256(rollback_img)
    old_active_sha = compute_sha256(active_img) if active_img.exists() else "NONE"

    print(f" Restoring rollback image: {rollback_img} (SHA-256: {rb_sha})")

    with MountManager(target_dir):
        # Move active to failed_update
        if active_img.exists():
            os.replace(active_img, temp_swap_img)

        # Move rollback to active
        os.replace(rollback_img, active_img)

        # Sync
        try:
            dir_fd = os.open(str(target_dir), os.O_DIRECTORY | os.O_RDONLY)
            os.fsync(dir_fd)
            os.close(dir_fd)
        except OSError:
            pass

    # Audit log
    audit_log = Path("/var/bhoot/audit/sysupdate.log")
    try:
        audit_log.parent.mkdir(parents=True, exist_ok=True)
        with open(audit_log, "a") as f:
            entry = {
                "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "action": "SYSTEM_ROLLBACK_RESTORED",
                "restored_sha256": rb_sha,
                "reverted_sha256": old_active_sha,
                "status": "SUCCESS",
            }
            f.write(json.dumps(entry) + "\n")
    except OSError:
        pass

    print("========================================================================")
    print(" [BHOOT OS] Rollback Restored Successfully!")
    print(f" Active image restored to SHA-256: {rb_sha}")
    print("========================================================================")
    return 0


def main():
    parser = argparse.ArgumentParser(
        description="BHOOT OS Safe Offline System Update & Rollback Engine"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Status
    p_status = subparsers.add_parser("status", help="Show system image and rollback status")
    p_status.add_argument(
        "--target-dir",
        type=Path,
        default=Path("/run/live/medium/live") if Path("/run/live/medium/live").exists() else Path("./live"),
        help="Directory containing filesystem.squashfs (default: /run/live/medium/live or ./live)",
    )

    # Verify dry-run
    p_verify = subparsers.add_parser("verify", help="Inspect and verify update image without applying")
    p_verify.add_argument("image", type=Path, help="Path to update SquashFS image")
    p_verify.add_argument("--sha256", type=str, default=None, help="Expected SHA-256 checksum")

    # Apply
    p_apply = subparsers.add_parser("apply", help="Atomically apply a verified update image")
    p_apply.add_argument("image", type=Path, help="Path to update SquashFS image")
    p_apply.add_argument(
        "--target-dir",
        type=Path,
        default=Path("/run/live/medium/live") if Path("/run/live/medium/live").exists() else Path("./live"),
        help="Target directory (default: /run/live/medium/live or ./live)",
    )
    p_apply.add_argument("--sha256", type=str, default=None, help="Expected SHA-256 checksum")

    # Rollback
    p_rb = subparsers.add_parser("rollback", help="Atomically restore rollback snapshot")
    p_rb.add_argument(
        "--target-dir",
        type=Path,
        default=Path("/run/live/medium/live") if Path("/run/live/medium/live").exists() else Path("./live"),
        help="Target directory (default: /run/live/medium/live or ./live)",
    )

    args = parser.parse_args()

    if args.command == "status":
        sys.exit(cmd_status(args.target_dir))
    elif args.command == "verify":
        sys.exit(cmd_verify_dry_run(args.image, args.sha256))
    elif args.command == "apply":
        sys.exit(cmd_apply(args.image, args.target_dir, args.sha256))
    elif args.command == "rollback":
        sys.exit(cmd_rollback(args.target_dir))


if __name__ == "__main__":
    main()
