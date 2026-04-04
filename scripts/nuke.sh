#!/usr/bin/env bash
cd "$(dirname "$0")/.."
source scripts/cyberpunk.sh

cyber_banner
cyber_status "OPERATION" "NUKE // total annihilation rebuild"
echo

cyber_section "PURGE WEBVIEW CACHES"
find ~/Library/WebKit/audio-haxor ~/Library/WebKit/com.menketechnologies.audio-haxor ~/Library/Caches/audio-haxor ~/Library/Caches/com.menketechnologies.audio-haxor -delete 2>/dev/null
cyber_ok "WebView caches obliterated"
echo

cyber_section "CACHE BUST"
node -e "const f=require('fs'),p='frontend/index.html';let h=f.readFileSync(p,'utf8');const v=Date.now()%100000;h=h.replace(/\?v=\d+/g,'?v='+v);f.writeFileSync(p,h);console.log('  busted to v'+v)"
echo

cyber_section "CLEAN BUILD ARTIFACTS"
command rm -rf src-tauri/target dist node_modules/.cache
cyber_ok "build caches destroyed"
echo

cyber_section "REBUILD FROM SCRATCH"
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
  cyber_ok "binary deployed // ${APP_SIZE} // ${ELAPSED}s"
  cyber_tagline "NUCLEAR LAUNCH SUCCESSFUL"
else
  cyber_fail "build failed after ${ELAPSED}s"
  cyber_tagline "LAUNCH ABORTED"
fi
cyber_line
