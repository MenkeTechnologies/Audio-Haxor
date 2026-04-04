#!/usr/bin/env bash
cd "$(dirname "$0")/.."
source scripts/cyberpunk.sh

cyber_banner
cyber_status "OPERATION" "CLEAN // purge build artifacts"
echo

cyber_section "DESTROYING CACHES"
BEFORE=$(du -sh src-tauri/target 2>/dev/null | awk '{print $1}' || echo "0B")
command rm -rf src-tauri/target dist node_modules/.cache
cyber_ok "freed ${BEFORE} // target + dist + node cache"

cyber_tagline "MEMORY WIPED. READY FOR FRESH BUILD."
cyber_line
