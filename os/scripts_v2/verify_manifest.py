#!/usr/bin/env python3
"""
BHOOT — Deterministic Build Artifact Manifest Verifier
Forensically verifies every artifact recorded in manifest.json against disk reality.
Validates SHA-256 digests, file sizes, structure, and detects unauthorized changes.
Explicitly distinguishes between FULL_FILE_SHA256, HEADER_ONLY_SHA256, and PARTITION_HEADER_SHA256.
"""

import os
import sys
import json
import hashlib

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OS_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(OS_DIR, "build_real")
MANIFEST_PATH = os.path.join(BUILD_REAL, "manifest.json")

VALID_SCOPES = {
    "FULL_FILE_SHA256",
    "HEADER_ONLY_SHA256",
    "PARTITION_HEADER_SHA256"
}


def hash_file_region(path: str, max_bytes: int = None) -> tuple[str, int]:
    h = hashlib.sha256()
    bytes_read = 0
    chunk_size = 4 * 1024 * 1024
    with open(path, "rb") as f:
        while chunk := f.read(chunk_size):
            if max_bytes is not None and bytes_read + len(chunk) > max_bytes:
                needed = max_bytes - bytes_read
                h.update(chunk[:needed])
                bytes_read += needed
                break
            h.update(chunk)
            bytes_read += len(chunk)
    return h.hexdigest(), bytes_read


def verify_manifest(manifest_file: str = MANIFEST_PATH) -> bool:
    print("=" * 76)
    print(" BHOOT Artifact Manifest Cryptographic Verifier")
    print(f" Manifest Target: {manifest_file}")
    print("=" * 76)

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
    print("-" * 76)

    seen_filenames = set()
    errors = []
    full_count = 0
    header_count = 0

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

        st = os.stat(fpath)
        actual_size = st.st_size
        physical_alloc = st.st_blocks * 512
        expected_size = item.get("logical_size_bytes")
        if actual_size != expected_size:
            errors.append(f"Size mismatch on {fname}: expected {expected_size} B, found {actual_size} B")
            continue

        scope = item.get("verification_scope")
        if not scope or scope not in VALID_SCOPES:
            if "sha256" in item:
                scope = "FULL_FILE_SHA256"
            elif "header_16m_sha256" in item:
                scope = "HEADER_ONLY_SHA256"
            elif "gpt_header_2m_sha256" in item:
                scope = "PARTITION_HEADER_SHA256"
            else:
                errors.append(f"Artifact {fname} contains no recognized scope or digest field")
                continue

        if scope == "FULL_FILE_SHA256":
            expected_digest = item.get("sha256")
            actual_digest, hashed_bytes = hash_file_region(fpath)
            full_count += 1
        elif scope == "HEADER_ONLY_SHA256":
            expected_digest = item.get("header_16m_sha256") or item.get("sha256")
            actual_digest, hashed_bytes = hash_file_region(fpath, max_bytes=16 * 1024 * 1024)
            header_count += 1
        elif scope == "PARTITION_HEADER_SHA256":
            expected_digest = item.get("gpt_header_2m_sha256") or item.get("sha256")
            actual_digest, hashed_bytes = hash_file_region(fpath, max_bytes=2 * 1024 * 1024)
            header_count += 1
        else:
            errors.append(f"Invalid scope '{scope}' for artifact {fname}")
            continue

        coverage_pct = (hashed_bytes / actual_size * 100) if actual_size > 0 else 100.0

        if actual_digest != expected_digest:
            errors.append(f"Hash mismatch on {fname} ({scope}):\n  expected: {expected_digest}\n  actual:   {actual_digest}")
            continue

        print(f"  [{idx}/{len(artifacts)}] PASS: {fname:<26} [{scope:<22}]")
        print(f"         Logical: {actual_size:>11} B | Allocated: {physical_alloc:>11} B | Hashed: {hashed_bytes:>11} B ({coverage_pct:6.2f}%)")
        print(f"         SHA-256: {actual_digest[:32]}...")

    print("-" * 76)
    if errors:
        print(f"[VERIFICATION FAILED] {len(errors)} error(s) discovered:")
        for err in errors:
            print(f"  - {err}")
        return False
    else:
        print(f"[VERIFICATION COMPLETED]")
        print(f"  Full File SHA-256 Validated (100% data coverage): {full_count} artifacts")
        print(f"  Header/Partition Table Validated (partial scope):  {header_count} artifacts")
        print(f"  [NOTE] Header-only hashes verify metadata/GPT/LUKS headers; they do not attest payloads outside the hashed header offset.")
        print("=" * 76)
        return True


if __name__ == "__main__":
    success = verify_manifest()
    sys.exit(0 if success else 1)

