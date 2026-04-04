#!/usr/bin/env bash
cd "$(dirname "$0")/.."
source scripts/cyberpunk.sh

cyber_banner
cyber_status "OPERATION" "REBUILD // bust + clean + build"
echo

cyber_section "CACHE BUST"
VER=$(node -e "const f=require('fs'),p='frontend/index.html';let h=f.readFileSync(p,'utf8');const v=Date.now()%100000;h=h.replace(/\?v=\d+/g,'?v='+v);f.writeFileSync(p,h);console.log(v)")
cyber_ok "assets busted to v${VER}"
echo

cyber_section "CLEAN"
command rm -rf src-tauri/target dist node_modules/.cache
cyber_ok "build caches purged"
echo

cyber_section "BUILD"
cyber_line
echo
START=$(date +%s)
pnpm tauri build 2>&1 | tail -8
END=$(date +%s)
ELAPSED=$((END - START))
echo
cyber_line

if [ -d "src-tauri/target/release/bundle/macos/AUDIO_HAXOR.app" ]; then
  APP_SIZE=$(du -sh src-tauri/target/release/bundle/macos/AUDIO_HAXOR.app | awk '{print $1}')
  cyber_ok "built in ${ELAPSED}s // ${APP_SIZE}"
  cyber_tagline "RECONSTRUCTION COMPLETE."
else
  cyber_fail "build failed after ${ELAPSED}s"
  cyber_tagline "RECONSTRUCTION FAILED."
fi
cyber_line
