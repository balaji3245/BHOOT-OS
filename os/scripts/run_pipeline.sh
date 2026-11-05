#!/usr/bin/env bash
# BHOOT Phase 2A - Master Pipeline Orchestrator
# Executes steps 01 through 05 sequentially, recording performance and verifying integrity.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "########################################################################"
echo "   PROJECT BHOOT: PHASE 2A BOOTABLE LINUX PREPARATION PIPELINE"
echo "########################################################################"
echo "Start time: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"

START_TIME=$(date +%s)

chmod +x "$SCRIPT_DIR"/*.sh

"$SCRIPT_DIR/01_prepare_disk_image.sh"
"$SCRIPT_DIR/02_build_esp_partition.sh"
"$SCRIPT_DIR/03_build_root_squashfs.sh"
"$SCRIPT_DIR/04_format_luks2_storage.sh"
"$SCRIPT_DIR/05_verify_integrity.sh"

END_TIME=$(date +%s)
DURATION=$(( END_TIME - START_TIME ))

echo -e "\n########################################################################"
echo "   PHASE 2A PIPELINE COMPLETED SUCCESSFULLY IN ${DURATION}s"
echo "########################################################################"

