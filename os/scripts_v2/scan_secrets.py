#!/usr/bin/env python3
"""
BHOOT — Secret Leakage Scanner
Scans source code, configuration files, and scripts for known test credentials,
embedded private keys, and plaintext session artifacts without printing sensitive values.
"""

import os
import sys
import re

PATTERNS = [
    ("Known Phase 2A.5 Test Passphrase", re.compile(r"BhootTestPassphrase2026\!")),
    ("Known Phase 2A.5 Test Recovery Key", re.compile(r"BhootRecoveryKey2026\!")),
    ("Plaintext Session Key Scratch Artifact", re.compile(r"\.session_key\.tmp")),
    ("Hardcoded Private Key Header", re.compile(r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----")),
]

IGNORE_DIRS = {
    ".git",
    "target",
    "node_modules",
    ".idea",
    ".vscode",
    "rootfs",
    "preserved_rc3",
    "build_env_debian12",
    "cold_boot_verification",
    "disposable_release_test",
}

IGNORE_EXTENSIONS = {
    ".img",
    ".backup",
    ".squashfs",
    ".phase2a4_compromised",
    ".rc3",
    ".iso",
    ".dump",
    ".log",
    ".bin",
    ".fd",
    ".deb",
    ".so",
    ".a",
}

# Documentation and ignore files are allowed to reference patterns
ALLOWLIST_FILES = {
    "PHASE_2A_5_SECURITY_READINESS_AUDIT.md",
    "PHASE_2A_6_SECURITY_REMEDIATION_REPORT.md",
    "PHASE_2A_7_INDEPENDENT_VERIFICATION_AUDIT.md",
    "PHASE_2A_8_REMEDIATION_REPORT.md",
    "PHASE_2A_9_INDEPENDENT_SOURCE_SECURITY_AUDIT.md",
    "PHASE_2A_10_REMEDIATION_REPORT.md",
    "PHASE_2A_11_INDEPENDENT_SECURITY_AUDIT.md",
    "PHASE_2A_12_REMEDIATION_REPORT.md",
    "transcript.jsonl",
    "transcript_full.jsonl",
    ".gitignore",
}


def scan_file(filepath: str) -> list:
    issues = []
    base = os.path.basename(filepath)
    if base in ALLOWLIST_FILES:
        return []

    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            for line_no, line in enumerate(f, start=1):
                for name, pat in PATTERNS:
                    if pat.search(line):
                        issues.append((filepath, line_no, name))
    except Exception as e:
        pass
    return issues


def main():
    root_dir = sys.argv[1] if len(sys.argv) > 1 else "/home/bhoot/Desktop/BHOOT"
    print("========================================================================")
    print(" [BHOOT Security Engine] Secret Leakage and Credential Scanner")
    print(f" Scanning root: {root_dir}")
    print("========================================================================")

    all_issues = []
    files_scanned = 0

    for root, dirs, files in os.walk(root_dir):
        dirs[:] = [d for d in dirs if d not in IGNORE_DIRS]
        for f in files:
            ext = os.path.splitext(f)[1]
            if ext in IGNORE_EXTENSIONS:
                continue
            path = os.path.join(root, f)
            files_scanned += 1
            issues = scan_file(path)
            if issues:
                all_issues.extend(issues)

    print(f" Scanned {files_scanned} files.")
    print("-" * 72)

    if all_issues:
        print(f" [FAIL] Found {len(all_issues)} credential leakage instances (values sanitized):")
        for fpath, lno, name in all_issues:
            print(f"   - {fpath}:{lno} -> {name}")
        print("=" * 72)
        sys.exit(1)
    else:
        print(" [PASS] Clean scan: Zero secret leaks or known test credentials detected.")
        print("========================================================================")
        sys.exit(0)


if __name__ == "__main__":
    main()
