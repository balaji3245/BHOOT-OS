#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Master Pipeline Orchestrator
# Builds, hardens, packages, assembles, and verifies the entire Debian 12 minimal operating system.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "========================================================================"
echo "   PROJECT BHOOT — PHASE 2A.2 REAL DEBIAN 12 OS PIPELINE START"
echo "========================================================================"
START_TIME=$(date +%s)

"$SCRIPT_DIR/01_bootstrap_debian12.sh"
"$SCRIPT_DIR/02_configure_rootfs.sh"
"$SCRIPT_DIR/03_build_real_squashfs.sh"
"$SCRIPT_DIR/03b_build_system_partition.sh"
"$SCRIPT_DIR/04_build_real_esp.sh"
"$SCRIPT_DIR/05_prepare_production_luks2.sh"
"$SCRIPT_DIR/06_assemble_disk_image.sh"
"$SCRIPT_DIR/07_verify_all_artifacts.sh"

END_TIME=$(date +%s)
TOTAL_TIME=$(( END_TIME - START_TIME ))

echo "========================================================================"
echo "   PROJECT BHOOT — PHASE 2A.2 PIPELINE COMPLETED IN $TOTAL_TIME SECONDS"
echo "========================================================================"

