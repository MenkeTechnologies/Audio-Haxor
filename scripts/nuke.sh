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
node -e "const fs=require('fs');const v=Date.now()%100000;for(const p of['frontend/index.html','frontend/tray-popover.html']){let h=fs.readFileSync(p,'utf8');h=h.replace(/\?v=\d+/g,'?v='+v);fs.writeFileSync(p,h);}console.log('  busted to v'+v)"
echo

cyber_section "CLEAN BUILD ARTIFACTS"
# Workspace `target/` lives at repo root; legacy `src-tauri/target` may exist — remove both.
command rm -rf target src-tauri/target dist node_modules/.cache
cyber_ok "build caches destroyed"
echo

cyber_section "REBUILD FROM SCRATCH"
cyber_line
echo
START=$(date +%s)
# On macOS we tell Tauri to build ONLY the .app (skip its own DMG creation) so the
# audio-engine helper-app reshape can run BEFORE the DMG is built. The reshape +
# DMG creation both happen in the postbundle step below using the vendored
# scripts/bundle_dmg.sh, so the resulting DMG already contains the reshaped .app
# in a single pass (no double-build, no stale rw.*.dmg mounts piling up in Finder).
# On other platforms, "all" stays in effect (deb / rpm / nsis / msi as appropriate).
TAURI_BUNDLE_ARGS=()
if [[ "$(uname -s)" == "Darwin" ]]; then
  TAURI_BUNDLE_ARGS=(--bundles app)
fi
# Full log on failure (do not pipe to tail — hides beforeBuildCommand / cargo errors).
if ! pnpm tauri build "${TAURI_BUNDLE_ARGS[@]}"; then
  END=$(date +%s)
  ELAPSED=$((END - START))
  echo
  cyber_fail "tauri build failed after ${ELAPSED}s (see log above)"
  cyber_tagline "LAUNCH ABORTED"
  exit 1
fi
END=$(date +%s)
ELAPSED=$((END - START))
echo
cyber_line

# macOS only: reshape the bundled audio-engine sidecar into a nested helper .app under
# Contents/Frameworks/. Required for AU plugin editor windows — see the script and
# audio-engine/README.md "Helper .app architecture" for the full rationale.
if [[ "$(uname -s)" == "Darwin" ]]; then
  echo
  cyber_section "AUDIO ENGINE HELPER .APP RESHAPE"
  if ! bash scripts/postbundle-audio-engine-helper.sh; then
    cyber_fail "audio-engine helper .app reshape failed"
    cyber_tagline "LAUNCH ABORTED"
    exit 1
  fi
  cyber_ok "helper .app installed + signed"
  echo
fi

# Cargo workspace: bundle may be under repo `target/` or legacy `src-tauri/target/`.
BUNDLE_MAC=""
for d in target/release/bundle/macos/AUDIO_HAXOR.app src-tauri/target/release/bundle/macos/AUDIO_HAXOR.app; do
  if [ -d "$d" ]; then
    BUNDLE_MAC=$d
    break
  fi
done
if [ -n "$BUNDLE_MAC" ]; then
  APP_SIZE=$(du -sh "$BUNDLE_MAC" | awk '{print $1}')
  cyber_ok "binary deployed // ${APP_SIZE} // ${ELAPSED}s"
  cyber_tagline "NUCLEAR LAUNCH SUCCESSFUL"
else
  cyber_fail "build finished but .app bundle not found after ${ELAPSED}s"
  cyber_tagline "LAUNCH ABORTED"
fi
cyber_line
