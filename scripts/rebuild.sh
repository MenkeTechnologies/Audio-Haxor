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
# macOS: skip Tauri's DMG bundling (--bundles app); the postbundle step below
# creates the DMG itself AFTER the audio-engine helper-app reshape, so we get
# a single DMG-creation pass with the reshaped .app inside. Other platforms
# build all native bundle targets.
TAURI_BUNDLE_ARGS=()
if [[ "$(uname -s)" == "Darwin" ]]; then
  TAURI_BUNDLE_ARGS=(--bundles app)
fi
pnpm tauri build "${TAURI_BUNDLE_ARGS[@]}" 2>&1 | tail -8
TAURI_RC=${PIPESTATUS[0]}
END=$(date +%s)
ELAPSED=$((END - START))
echo
cyber_line

if [ "$TAURI_RC" -ne 0 ]; then
  cyber_fail "build failed after ${ELAPSED}s"
  cyber_tagline "RECONSTRUCTION FAILED."
  exit "$TAURI_RC"
fi

# macOS only: nest audio-engine into Contents/MacOS/AudioHaxorEngine.app so the
# helper has its own bundle identity for audiocomponentd / OOP AU plugin loading. See
# scripts/postbundle-audio-engine-helper.sh for details.
if [[ "$(uname -s)" == "Darwin" ]]; then
  echo
  cyber_section "AUDIO ENGINE HELPER .APP RESHAPE"
  if ! bash scripts/postbundle-audio-engine-helper.sh; then
    cyber_fail "audio-engine helper .app reshape failed"
    cyber_tagline "RECONSTRUCTION FAILED."
    exit 1
  fi
  cyber_ok "helper .app installed + signed"
  echo
fi

BUNDLE_MAC=""
for d in target/release/bundle/macos/AUDIO_HAXOR.app src-tauri/target/release/bundle/macos/AUDIO_HAXOR.app; do
  if [ -d "$d" ]; then
    BUNDLE_MAC=$d
    break
  fi
done
if [ -n "$BUNDLE_MAC" ]; then
  APP_SIZE=$(du -sh "$BUNDLE_MAC" | awk '{print $1}')
  cyber_ok "built in ${ELAPSED}s // ${APP_SIZE}"
  cyber_tagline "RECONSTRUCTION COMPLETE."
else
  cyber_fail ".app bundle not found after ${ELAPSED}s"
  cyber_tagline "RECONSTRUCTION FAILED."
fi
cyber_line
