#!/usr/bin/env python3
"""
BHOOT — Deterministic Build Artifact Manifest Verifier
Forensically verifies every artifact recorded in manifest.json against disk reality.
Validates SHA-256 digests, file sizes, structure, and detects unauthorized changes.
"""

import os
import sys
import json
import hashlib

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OS_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(OS_DIR, "build_real")
MANIFEST_PATH = os.path.join(BUILD_REAL, "manifest.json")


def hash_file_region(path: str, max_bytes: int = None) -> str:
    h = hashlib.sha256()
    bytes_read = 0
    with open(path, "rb") as f:
        while chunk := f.read(4 * 1024 * 1024):
            if max_bytes is not None and bytes_read + len(chunk) > max_bytes:
                h.update(chunk[:max_bytes - bytes_read])
                break
            h.update(chunk)
            bytes_read += len(chunk)
    return h.hexdigest()


def verify_manifest(manifest_file: str = MANIFEST_PATH) -> bool:
    print("=" * 72)
    print(" BHOOT Artifact Manifest Cryptographic Verifier")
    print(f" Manifest Target: {manifest_file}")
    print("=" * 72)

    if not os.path.isfile(manifest_file):
        print(f"[FATAL ERROR] Manifest file does not exist: {manifest_file}", file=sys.stderr)
        return False

    with open(manifest_file, "r") as f:
        try:
            m = json.load(f)
        except json.JSONDecodeError as e:
            print(f"[FATAL ERROR] Corrupted JSON manifest: {e}", file=sys.stderr)
            return False

    if "artifacts" not in m or not isinstance(m["artifacts"], list):
        print("[FATAL ERROR] Manifest missing required 'artifacts' list", file=sys.stderr)
        return False

    artifacts = m["artifacts"]
    print(f"Project:          {m.get('project', 'UNKNOWN')}")
    print(f"Phase:            {m.get('phase', 'UNKNOWN')}")
    print(f"Artifact Count:   {len(artifacts)}")
    print("-" * 72)

    seen_filenames = set()
    errors = []
    verified_count = 0

    for idx, item in enumerate(artifacts, 1):
        fname = item.get("filename")
        if not fname:
            errors.append(f"Artifact entry #{idx} has no 'filename' property")
            continue

        if fname in seen_filenames:
            errors.append(f"Duplicate artifact entry detected: {fname}")
            continue
        seen_filenames.add(fname)

        fpath = os.path.join(BUILD_REAL, fname)
        if not os.path.isfile(fpath):
            errors.append(f"Artifact missing on disk: {fname} (path: {fpath})")
            continue

        actual_size = os.path.getsize(fpath)
        expected_size = item.get("logical_size_bytes")
        if actual_size != expected_size:
            errors.append(f"Size mismatch on {fname}: expected {expected_size} bytes, found {actual_size} bytes")
            continue

        # Check digest
        if "sha256" in item:
            expected_digest = item["sha256"]
            actual_digest = hash_file_region(fpath)
            mode = "full"
        elif "header_16m_sha256" in item:
            expected_digest = item["header_16m_sha256"]
            actual_digest = hash_file_region(fpath, max_bytes=16 * 1024 * 1024)
            mode = "header_16m"
        elif "gpt_header_2m_sha256" in item:
            expected_digest = item["gpt_header_2m_sha256"]
            actual_digest = hash_file_region(fpath, max_bytes=2 * 1024 * 1024)
            mode = "gpt_header_2m"
        else:
            errors.append(f"Artifact {fname} contains no recognized digest field")
            continue

        if actual_digest != expected_digest:
            errors.append(f"Hash mismatch on {fname} ({mode}):\n  expected: {expected_digest}\n  actual:   {actual_digest}")
            continue

        print(f"  [{idx}/{len(artifacts)}] PASS: {fname:<28} ({mode}) -> {actual_digest[:16]}... ({actual_size} B)")
        verified_count += 1

    print("-" * 72)
    if errors:
        print(f"[VERIFICATION FAILED] {len(errors)} error(s) discovered:")
        for err in errors:
            print(f"  - {err}")
        return False
    else:
        print(f"[VERIFICATION PASSED] All {verified_count} artifacts verified with cryptographic integrity.")
        print("=" * 72)
        return True


if __name__ == "__main__":
    success = verify_manifest()
    sys.exit(0 if success else 1)

