#!/usr/bin/env python3
"""
BHOOT Phase 2A.3 - Storage Manager: Ext4 Formatting, Persistence & Disaster Recovery
Manages LUKS2 volume mounting, Ext4 formatting via UDisks2 D-Bus, persistence verification,
and detached header recovery validation without leaking credentials into logs.
"""

import os
import subprocess
import sys
import tempfile
import time
import dbus

def format_and_populate(container_path: str, passphrase: str):
    """Unlocks container, formats decrypted mapping as Ext4, and populates BHOOT persistence hierarchy."""
    print("    Attaching loop device for production volume...")
    res = subprocess.run(
        ["udisksctl", "loop-setup", "--file", container_path, "--no-user-interaction"],
        capture_output=True, text=True, check=True
    )
    loop_dev = res.stdout.split()[-1].rstrip(".")
    print(f"    Loop device: {loop_dev}")

    with tempfile.NamedTemporaryFile("w", delete=False) as kf:
        kf.write(passphrase)
        kfname = kf.name

    try:
        print("    Unlocking LUKS2 container via udisksctl...")
        res2 = subprocess.run(
            ["udisksctl", "unlock", "-b", loop_dev, "--key-file", kfname, "--no-user-interaction"],
            capture_output=True, text=True, check=True
        )
        clear_dev = res2.stdout.split()[-1].rstrip(".")
        print(f"    Cleartext device: {clear_dev}")

        # In UDisks2, '-' is encoded as '_2d' in object paths
        dm_name = os.path.basename(clear_dev).replace("-", "_2d")
        obj_path = f"/org/freedesktop/UDisks2/block_devices/{dm_name}"

        bus = dbus.SystemBus()
        block_obj = bus.get_object("org.freedesktop.UDisks2", obj_path)
        block_iface = dbus.Interface(block_obj, "org.freedesktop.UDisks2.Block")

        print("    Formatting cleartext mapping as Ext4 (Label: BHOOT_DATA, take-ownership: True)...")
        block_iface.Format(
            "ext4",
            dbus.Dictionary({
                "label": "BHOOT_DATA",
                "take-ownership": dbus.Boolean(True)
            }, signature="sv")
        )
        print("    Ext4 filesystem formatted successfully.")

        # Give udev/udisks time to settle
        time.sleep(0.5)

        # Check mount status (GNOME automounter may have already mounted it)
        find_res = subprocess.run(["findmnt", "-n", "-o", "TARGET", clear_dev], capture_output=True, text=True)
        mnt_pt = find_res.stdout.strip()

        if not mnt_pt:
            mount_res = subprocess.run(
                ["udisksctl", "mount", "-b", clear_dev, "--no-user-interaction"],
                capture_output=True, text=True, check=True
            )
            mnt_pt = mount_res.stdout.split()[-1].rstrip(".")
        print(f"    Mounted at: {mnt_pt}")

        # Populate directory hierarchy
        print("    Populating BHOOT initial storage hierarchy (/var/bhoot/)...")
        for base in [mnt_pt, os.path.join(mnt_pt, "var", "bhoot"), os.path.join(mnt_pt, "var", "bhoot", "rw")]:
            os.makedirs(os.path.join(base, "ledger"), exist_ok=True)
            os.makedirs(os.path.join(base, "vault"), exist_ok=True)
            os.makedirs(os.path.join(base, "audit"), exist_ok=True)
            os.makedirs(os.path.join(base, "snapshots"), exist_ok=True)
            with open(os.path.join(base, "ledger", "persistence_marker.proof"), "wb") as f:
                f.write(b"BHOOT_EXT4_PERSISTENCE_VALIDATED_2026")

        # Write persistence.conf for live-boot overlay discovery
        with open(os.path.join(mnt_pt, "persistence.conf"), "w") as f:
            f.write("/var/bhoot union\n")

        # Write persistence proof marker at root as well
        proof_path = os.path.join(mnt_pt, "ledger", "persistence_marker.proof")
        with open(proof_path, "wb") as f:
            f.write(b"BHOOT_EXT4_PERSISTENCE_VALIDATED_2026")

        print("    Initial directory structure and persistence.conf staged.")

        # Unmount
        subprocess.run(["udisksctl", "unmount", "-b", clear_dev, "--no-user-interaction"], check=True)
        time.sleep(0.5)

        # Remount to verify persistence across mounts
        mount_res2 = subprocess.run(
            ["udisksctl", "mount", "-b", clear_dev, "--no-user-interaction"],
            capture_output=True, text=True, check=True
        )
        mnt_pt2 = mount_res2.stdout.split()[-1].rstrip(".")
        with open(os.path.join(mnt_pt2, "ledger", "persistence_marker.proof"), "rb") as f:
            read_back = f.read()
            assert read_back == b"BHOOT_EXT4_PERSISTENCE_VALIDATED_2026", "Persistence proof verification failed!"

        print("    Persistence verified: Read-back matches written proof token.")

        # Clean unmount and lock
        subprocess.run(["udisksctl", "unmount", "-b", clear_dev, "--no-user-interaction"], check=True)
        time.sleep(0.5)

        subprocess.run(["udisksctl", "lock", "-b", loop_dev, "--no-user-interaction"], check=False)
        time.sleep(0.5)
        subprocess.run(["udisksctl", "loop-delete", "-b", loop_dev, "--no-user-interaction"], check=False)
        print("    Volume locked and loop device detached cleanly.")

    finally:
        if os.path.exists(kfname):
            os.remove(kfname)


def test_disaster_recovery(source_container: str, passphrase: str, header_backup_path: str):
    """Simulates physical sector corruption of LUKS2 header and validates detached header recovery."""
    print("  [Disaster Recovery Test] Validating Detached LUKS2 Header Recovery...")

    disposable_img = f"{source_container}.disposable_test"
    try:
        # 1. Create a disposable copy
        print(f"    Creating disposable test copy at: {disposable_img}...")
        subprocess.run(["cp", "--sparse=always", source_container, disposable_img], check=True)

        # Verify it is initially valid LUKS2
        res = subprocess.run(["cryptsetup", "isLuks", "--type", "luks2", disposable_img])
        assert res.returncode == 0, "Initial test container is not valid LUKS2!"
        print("    Initial container verified as valid LUKS2.")

        # 2. Intentionally corrupt the primary and secondary LUKS2 headers (first 8 MiB)
        print("    Simulating catastrophic sector corruption (zeroing first 8 MiB of header)...")
        subprocess.run(
            ["dd", "if=/dev/zero", f"of={disposable_img}", "bs=1M", "count=8", "conv=notrunc", "status=none"],
            check=True
        )

        # Verify corruption has broken LUKS2 recognition
        res_corrupt = subprocess.run(["cryptsetup", "isLuks", "--type", "luks2", disposable_img], capture_output=True)
        assert res_corrupt.returncode != 0, "Container should fail isLuks after corruption!"
        print("    Verified corruption: Container is unrecognized and unbootable.")

        # 3. Restore the LUKS2 header from detached backup
        print(f"    Executing luksHeaderRestore using detached backup: {header_backup_path}...")
        subprocess.run([
            "cryptsetup", "luksHeaderRestore",
            "--header-backup-file", header_backup_path,
            "--batch-mode",
            disposable_img
        ], check=True)

        # 4. Verify LUKS2 recognition is fully restored
        res_restored = subprocess.run(["cryptsetup", "isLuks", "--type", "luks2", disposable_img])
        assert res_restored.returncode == 0, "Container failed isLuks after header restore!"
        print("    Verified restoration: Container header structure restored successfully.")

        # 5. Verify the restored container can be unlocked with the passphrase
        print("    Verifying cryptographic unlock with original passphrase...")
        with tempfile.NamedTemporaryFile("w", delete=False) as kf:
            kf.write(passphrase)
            kfname = kf.name

        try:
            loop_out = subprocess.run(
                ["udisksctl", "loop-setup", "--file", disposable_img, "--no-user-interaction"],
                capture_output=True, text=True, check=True
            )
            test_loop = loop_out.stdout.split()[-1].rstrip(".")

            unlock_out = subprocess.run(
                ["udisksctl", "unlock", "-b", test_loop, "--key-file", kfname, "--no-user-interaction"],
                capture_output=True, text=True, check=True
            )
            test_clear = unlock_out.stdout.split()[-1].rstrip(".")
            print(f"    Unlock SUCCESS: Cleartext block mapping active at {test_clear}.")

            # Clean up
            time.sleep(0.5)
            subprocess.run(["udisksctl", "lock", "-b", test_loop, "--no-user-interaction"], check=False)
            time.sleep(0.5)
            subprocess.run(["udisksctl", "loop-delete", "-b", test_loop, "--no-user-interaction"], check=False)
        finally:
            if os.path.exists(kfname):
                os.remove(kfname)

        print("    Detached header disaster recovery cycle PASSED 100%!")

    finally:
        if os.path.exists(disposable_img):
            os.remove(disposable_img)


if __name__ == "__main__":
    action = sys.argv[1]
    if action == "format_and_populate":
        container = sys.argv[2]
        key = sys.stdin.read().strip()
        format_and_populate(container, key)
    elif action == "test_disaster_recovery":
        container = sys.argv[2]
        backup = sys.argv[3]
        key = sys.stdin.read().strip()
        test_disaster_recovery(container, key, backup)
    else:
        print(f"Unknown action: {action}", file=sys.stderr)
        sys.exit(1)
