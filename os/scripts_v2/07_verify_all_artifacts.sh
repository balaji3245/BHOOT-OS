#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Step 07: Run Forensic Verification Suite
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "$SCRIPT_DIR/verify_suite.py"
