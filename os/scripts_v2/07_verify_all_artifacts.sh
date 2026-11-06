#!/usr/bin/env bash
# BHOOT Phase 2A.6 - Step 07: Run Forensic Verification and Security Suites
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "Executing Phase 2A.6 Forensic Verification Suite..."
python3 "$SCRIPT_DIR/verify_suite.py"

echo "Executing Phase 2A.6 Security Remediation Regression Suite..."
python3 "$SCRIPT_DIR/test_phase2a6_security_suite.py"
