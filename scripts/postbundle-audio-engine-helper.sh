#!/usr/bin/env bash
# postbundle-audio-engine-helper.sh
#
# Reshape the bundled `audio-engine` sidecar (Tauri externalBin output) into a nested macOS
# helper .app inside the parent bundle's Contents/MacOS/, then re-sign the result.
#
# Without this reshape, the audio-engine runs as a bare Mach-O sibling of the main
# `audio-haxor` binary in Contents/MacOS/. macOS resolves [NSBundle mainBundle] for that
# process by walking up to the nearest enclosing .app, which lands on AUDIO_HAXOR.app —
# so the helper inherits the parent's bundle context. audiocomponentd then refuses (or
# silently stalls) the XPC view-controller delivery for AU plugins loaded out-of-process
# via _RemoteAUv2ViewFactory, leaving plugin editor windows blank (1×1 stub NSView).
#
# After this script:
#   AUDIO_HAXOR.app/Contents/MacOS/AudioHaxorEngineHelper.app/Contents/MacOS/audio-engine
#   AUDIO_HAXOR.app/Contents/MacOS/AudioHaxorEngineHelper.app/Contents/Info.plist
#   AUDIO_HAXOR.app/Contents/MacOS/AudioHaxorEngineHelper.app/Contents/Resources/icon.icns
#
# [NSBundle mainBundle] from the helper now resolves to AudioHaxorEngineHelper.app (its own
# bundle ID, its own Info.plist, distinct from the parent), and audiocomponentd accepts the
# helper as a real Cocoa host. The Rust resolver in src-tauri/src/audio_engine.rs looks for
# the helper at this path before falling back to the legacy sibling-in-Contents/MacOS/ layout.
#
# Codesigning ordering matters: sign innermost first, work outward. The outer .app's
# CodeResources includes a hash of the inner .app's signature, so the inner .app must be
# fully signed before the outer .app re-signs. Re-signing the outer must NOT use --deep
# (which would recursively re-sign and overwrite the inner signatures we just made).

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Allow caller to override; otherwise auto-detect the just-bundled .app.
APP="${1:-}"
if [ -z "$APP" ]; then
  for candidate in \
      "$REPO_ROOT/target/release/bundle/macos/AUDIO_HAXOR.app" \
      "$REPO_ROOT/src-tauri/target/release/bundle/macos/AUDIO_HAXOR.app"; do
    if [ -d "$candidate" ]; then
      APP="$candidate"
      break
    fi
  done
fi

if [ -z "$APP" ] || [ ! -d "$APP" ]; then
  echo "postbundle-audio-engine-helper: no AUDIO_HAXOR.app found (tried target/release and src-tauri/target/release)" >&2
  exit 1
fi

# Copy the reshaped main bundle to system Applications (replaces any prior install).
install_reshaped_app_to_applications() {
  echo "postbundle-audio-engine-helper: installing reshaped bundle -> /Applications/AUDIO_HAXOR.app"
  sudo rm -rf /Applications/AUDIO_HAXOR.app || true
  sudo ditto "$APP" /Applications/AUDIO_HAXOR.app
}

ENGINE_BIN_OLD="$APP/Contents/MacOS/audio-engine"
# IMPORTANT: helper .app lives in Contents/MacOS/, NOT Contents/Frameworks/.
# Apple's HIG says helper bundles go in Frameworks/, but real DAWs (Bitwig, Reaper, etc.)
# put them in MacOS/ — and only that location works for AU plugin hosting via
# audiocomponentd. We tried Frameworks/ first; it produces an `.app` whose
# `[NSBundle mainBundle]` resolves correctly, but audiocomponentd still rejects the
# process for OOP AU view-controller delivery (the `_RemoteAUv2ViewFactory` placeholder
# NSView never gets populated). Apparently LaunchServices treats `.app` bundles inside
# `Contents/Frameworks/` as embedded frameworks rather than registrable apps. Bitwig's
# `Bitwig Plug-in Host ARM64-NEON.app` lives at `Bitwig Studio.app/Contents/MacOS/` and
# their AU plugin editors work — that's our existence proof.
HELPER_APP="$APP/Contents/MacOS/AudioHaxorEngineHelper.app"
HELPER_CONTENTS="$HELPER_APP/Contents"
HELPER_MACOS="$HELPER_CONTENTS/MacOS"
HELPER_INFO="$HELPER_CONTENTS/Info.plist"
HELPER_RESOURCES="$HELPER_CONTENTS/Resources"
HELPER_BIN="$HELPER_MACOS/audio-engine"
INFO_TEMPLATE="$REPO_ROOT/audio-engine/helper-app/Info.plist"
HELPER_ICON_SRC="$REPO_ROOT/src-tauri/icons/icon.icns"
ENTITLEMENTS="$REPO_ROOT/src-tauri/Entitlements.plist"
MAIN_BIN="$APP/Contents/MacOS/audio-haxor"
# Stale helper from previous postbundle runs that put it under Contents/Frameworks/.
LEGACY_HELPER_APP="$APP/Contents/Frameworks/AudioHaxorEngineHelper.app"

if [ ! -f "$INFO_TEMPLATE" ]; then
  echo "postbundle-audio-engine-helper: missing template $INFO_TEMPLATE" >&2
  exit 1
fi
if [ ! -f "$HELPER_ICON_SRC" ]; then
  echo "postbundle-audio-engine-helper: missing helper icon $HELPER_ICON_SRC" >&2
  exit 1
fi
if [ ! -f "$ENTITLEMENTS" ]; then
  echo "postbundle-audio-engine-helper: missing entitlements $ENTITLEMENTS" >&2
  exit 1
fi

# Clean up any stale helper from a previous run that put it under Contents/Frameworks/.
if [ -d "$LEGACY_HELPER_APP" ]; then
  echo "postbundle-audio-engine-helper: removing stale legacy helper at $LEGACY_HELPER_APP"
  command rm -rf "$LEGACY_HELPER_APP"
fi

# Idempotency: if the helper .app already exists with a binary, assume a previous
# postbundle run completed (e.g. user invoked the script twice). Skip the move but
# still re-sign in case sources changed.
if [ -f "$HELPER_BIN" ] && [ ! -f "$ENGINE_BIN_OLD" ]; then
  echo "postbundle-audio-engine-helper: helper .app already in place, re-signing only"
elif [ -f "$ENGINE_BIN_OLD" ]; then
  echo "postbundle-audio-engine-helper: moving $ENGINE_BIN_OLD -> $HELPER_BIN"
  command rm -rf "$HELPER_APP"
  mkdir -p "$HELPER_MACOS"
  mv "$ENGINE_BIN_OLD" "$HELPER_BIN"
else
  echo "postbundle-audio-engine-helper: no audio-engine binary found at $ENGINE_BIN_OLD or $HELPER_BIN" >&2
  exit 1
fi

chmod +x "$HELPER_BIN"

# Same icon as the main app — required for Dock / Cmd+Tab when LSUIElement helper is foregrounded (plugin editors).
mkdir -p "$HELPER_RESOURCES"
cp "$HELPER_ICON_SRC" "$HELPER_RESOURCES/icon.icns"
cp "$INFO_TEMPLATE" "$HELPER_INFO"

# Strip any quarantine xattrs that would invalidate codesign.
xattr -cr "$HELPER_APP" 2>/dev/null || true
xattr -cr "$APP" 2>/dev/null || true

# Step 1: sign the helper .app bundle with hardened runtime + plugin host entitlements.
#
# Signing a *bundle* (a directory ending in .app) with `codesign` re-signs the bundle's
# main executable (CFBundleExecutable from Info.plist) as part of the same operation.
# That means we MUST pass --options runtime and --entitlements here, or the inner binary
# will end up with `flags=0x2(adhoc)` only — without the runtime flag, hardened runtime
# is OFF and the entitlements are inert (Apple ignores entitlements on non-runtime binaries).
#
# The plugin host entitlements (com.apple.security.cs.disable-library-validation,
# allow-jit, allow-unsigned-executable-memory, disable-executable-page-protection) are
# required for the helper to dlopen 3rd-party VST3/AU bundles signed by other Team IDs
# and for JIT-using plugins (Serum 2 wavetables, etc.) to allocate executable memory.
#
# Result on the inner binary: `flags=0x10002(adhoc,runtime)` + the four entitlement keys
# (verifiable with `codesign -dvv` and `codesign -d --entitlements - "$HELPER_BIN"`).
codesign --force --options runtime \
  --entitlements "$ENTITLEMENTS" \
  --sign - \
  "$HELPER_APP"

# Step 2: re-sign the outer .app bundle. Tauri already signed it with hardened runtime
# + the same Entitlements.plist for the `audio-haxor` main binary; the only thing we
# need to redo is the bundle's CodeResources (which was sealed before we moved the
# audio-engine sidecar into Contents/MacOS/AudioHaxorEngineHelper.app/). The main
# `audio-haxor` binary's existing signature is still valid (we never touched it).
#
# NOT --deep — that would recursively re-sign the inner helper .app we just signed in
# step 1, dropping the runtime flag again (codesign on a bundle without --options
# runtime always strips runtime even with --force). This is a JUCE-host classic that
# bites every plugin host that ships nested helper bundles.
#
# We DO pass --options runtime + --entitlements to be consistent with how Tauri signed
# the outer bundle originally; without these, re-signing the outer bundle would re-sign
# `audio-haxor` and strip ITS runtime flag, breaking 3rd-party VST3 loading in the
# main process (the same plugin-host entitlements failure mode).
codesign --force --options runtime \
  --entitlements "$ENTITLEMENTS" \
  --sign - \
  "$APP"

# Sanity check: verify both signatures and confirm the runtime flag survived.
codesign --verify --strict --verbose=2 "$HELPER_APP" 2>&1 | sed 's/^/postbundle-audio-engine-helper: helper verify: /'
codesign --verify --strict --verbose=2 "$APP" 2>&1 | sed 's/^/postbundle-audio-engine-helper: outer verify: /'

# Confirm runtime flag is present on both binaries — fail loudly if Apple stripped it,
# because the whole reason we wrote this script is for hardened-runtime + entitlements
# to take effect inside the helper.
for bin in "$HELPER_BIN" "$MAIN_BIN"; do
  flags=$(codesign -dvv "$bin" 2>&1 | grep -E '^CodeDirectory' | sed -n 's/.*flags=\([^ ]*\).*/\1/p')
  if [[ "$flags" != *"runtime"* ]]; then
    echo "postbundle-audio-engine-helper: ERROR: $bin missing 'runtime' flag (got: $flags)" >&2
    exit 1
  fi
  echo "postbundle-audio-engine-helper: $bin flags=$flags ✓"
done

# ---------------------------------------------------------------------------
# DMG regeneration
# ---------------------------------------------------------------------------
#
# Tauri's `pnpm tauri build` creates the .app and then runs `bundle_dmg.sh` to
# produce a `.dmg` containing it. That DMG is built BEFORE this postbundle
# script reshapes the .app, so the DMG that Tauri ships out the door contains
# the OLD (un-reshaped) .app — `Contents/MacOS/audio-engine` as a bare sidecar
# instead of the nested helper .app. End-user DMG installs would then have AU
# plugins regress to the original blank-window state.
#
# We can't hook into Tauri between .app creation and DMG creation (Tauri 2 has
# no `afterAppCommand` / `beforeDmgCommand` hooks), so the only correct fix is
# to discard the stale Tauri-built DMG and regenerate it from the reshaped
# .app using the same `bundle_dmg.sh` Tauri vendored into the build dir.

# Always look for the DMG resources next to the .app we just reshaped, regardless
# of whether the user passed `target/release/...` or `src-tauri/target/release/...`
APP_PARENT="$(cd "$(dirname "$APP")"/.. && pwd)"
DMG_DIR="$APP_PARENT/dmg"

# Prefer the vendored bundle_dmg.sh in scripts/. The wrapper scripts pass
# `--bundles app` to `pnpm tauri build` so Tauri builds only the .app and
# never creates target/release/bundle/dmg/bundle_dmg.sh — that's intentional,
# we want a single DMG-creation pass owned by THIS script (post-reshape).
# Fall back to whatever Tauri left behind for callers who invoked the script
# manually after a full `pnpm tauri build` (no `--bundles app`).
if [ -f "$REPO_ROOT/scripts/bundle_dmg.sh" ]; then
  DMG_BUNDLE_SCRIPT="$REPO_ROOT/scripts/bundle_dmg.sh"
elif [ -f "$DMG_DIR/bundle_dmg.sh" ]; then
  DMG_BUNDLE_SCRIPT="$DMG_DIR/bundle_dmg.sh"
else
  DMG_BUNDLE_SCRIPT=""
fi
# Prefer the canonical icon in src-tauri/icons/ — always present in the repo
# regardless of whether Tauri's DMG step ran (and so doesn't depend on the
# vendored target/release/bundle/dmg/icon.icns side-effect file).
if [ -f "$REPO_ROOT/src-tauri/icons/icon.icns" ]; then
  DMG_VOL_ICON="$REPO_ROOT/src-tauri/icons/icon.icns"
elif [ -f "$DMG_DIR/icon.icns" ]; then
  DMG_VOL_ICON="$DMG_DIR/icon.icns"
else
  DMG_VOL_ICON=""
fi

if [ -z "$DMG_BUNDLE_SCRIPT" ]; then
  echo "postbundle-audio-engine-helper: WARNING: bundle_dmg.sh not found in scripts/ or target/release/bundle/dmg/, skipping DMG regen" >&2
  install_reshaped_app_to_applications
  echo "postbundle-audio-engine-helper: helper .app installed at $HELPER_APP"
  exit 0
fi
mkdir -p "$DMG_DIR"

# Pull product name + version from tauri.conf.json so we don't drift if either changes.
TAURI_CONF="$REPO_ROOT/src-tauri/tauri.conf.json"
PRODUCT_NAME="$(node -e "console.log(require('$TAURI_CONF').productName)")"
VERSION="$(node -e "console.log(require('$TAURI_CONF').version)")"
# Tauri's DMG filename uses the bare CPU arch from the rustc host triple (aarch64
# for arm64, x86_64 for Intel) — see `target/release/bundle/macos/rw.*.dmg` left
# behind by the original Tauri DMG step for the exact pattern.
HOST_TRIPLE="$(rustc --print host-tuple)"
DMG_ARCH="${HOST_TRIPLE%%-*}"
DMG_NAME="${PRODUCT_NAME}_${VERSION}_${DMG_ARCH}.dmg"
DMG_OUT="$DMG_DIR/$DMG_NAME"

echo "postbundle-audio-engine-helper: regenerating DMG -> $DMG_OUT"

# Stage just the reshaped .app in a temp dir so bundle_dmg.sh sees nothing else
# (the bundle_dmg.sh AppleScript step iterates everything in the source folder).
DMG_STAGE="$(mktemp -d -t audio-haxor-dmg-stage.XXXXXX)"
trap 'command rm -rf "$DMG_STAGE"' EXIT
cp -R "$APP" "$DMG_STAGE/"

# Detach any stale Tauri-built DMGs that are still mounted in /Volumes/. Tauri's
# bundle_dmg.sh AppleScript step ("set icon positions") attaches the intermediate
# rw.NNNNN.<name>.dmg to a /Volumes/dmg.XXXXXX mount point and is supposed to detach
# at the end, but if anything in that step throws (no display, sandboxed shell, etc.)
# the mount is left dangling. After enough `pnpm tauri build` runs you accumulate
# multiple stale mounts and Finder pops one of them up at the end of every build —
# that's what makes you think the regenerated DMG is wrong (you're actually looking
# at the un-reshaped .app inside an older mount, not the new DMG file on disk).
#
# We extract the parent /dev/diskN device name (NOT the s1 partition) for each
# image whose path matches rw.NNNNN.AUDIO_HAXOR*.dmg, then `hdiutil detach -force`
# the whole device. The block shape from `hdiutil info` is:
#     ============
#     image-path      : /…/rw.55129.AUDIO_HAXOR_<version>_aarch64.dmg
#     image-encrypted : false
#     ... other key:value lines ...
#     /dev/disk4              GUID_partition_scheme
#     /dev/disk4s1            48465300-0000-...    /Volumes/dmg.sZGHg4
# Stop on the FIRST /dev/disk line that's followed by whitespace (the parent
# disk line) so we get `/dev/disk4`, not `/dev/disk4s1`.
echo "postbundle-audio-engine-helper: detaching any stale tauri rw.* DMG mounts"
hdiutil info 2>/dev/null \
  | awk '
      /^image-path[[:space:]]*:.*\/rw\.[0-9]+\.AUDIO_HAXOR.*\.dmg$/ { found = 1; next }
      found && /^\/dev\/disk[0-9]+[[:space:]]/ { print $1; found = 0 }
    ' \
  | while read -r dev; do
      [ -z "$dev" ] && continue
      echo "postbundle-audio-engine-helper:   detaching $dev"
      hdiutil detach "$dev" -force >/dev/null 2>&1 || true
    done

# Throw away any stale temp DMG Tauri left behind in macos/, plus any prior
# successful DMG in dmg/, so we don't accidentally ship a half-reshaped one.
command rm -f "$APP_PARENT/macos/"rw.*.dmg "$DMG_OUT"

# Window-size / icon-positions match Tauri's vendored defaults so the user-visible
# layout (where the .app icon is, where the /Applications drop link is) is
# identical to what stock Tauri would have produced.
bash "$DMG_BUNDLE_SCRIPT" \
  --volname "$PRODUCT_NAME" \
  ${DMG_VOL_ICON:+--volicon "$DMG_VOL_ICON"} \
  --icon "${PRODUCT_NAME}.app" 180 170 \
  --app-drop-link 480 170 \
  --window-size 660 400 \
  --hide-extension "${PRODUCT_NAME}.app" \
  --no-internet-enable \
  --skip-jenkins \
  --hdiutil-quiet \
  "$DMG_OUT" \
  "$DMG_STAGE/"

if [ ! -f "$DMG_OUT" ]; then
  echo "postbundle-audio-engine-helper: ERROR: DMG not produced at $DMG_OUT" >&2
  exit 1
fi
echo "postbundle-audio-engine-helper: DMG ready: $DMG_OUT $(du -h "$DMG_OUT" | awk '{print $1}')"

# Re-codesign the regenerated DMG so its signature is consistent with the inner
# .app (Tauri does this for the original DMG; if we don't repeat it here the
# DMG ends up unsigned and macOS Gatekeeper warns harder on first install).
codesign --force --sign - "$DMG_OUT" 2>&1 | sed 's/^/postbundle-audio-engine-helper: dmg sign: /' || true

install_reshaped_app_to_applications
echo kill pid
pid="$(pgrep audio-haxor | awk '{print $1}')"

if [[ -n "$pid" ]]; then
    kill $pid
fi

( /Applications/AUDIO_HAXOR.app/Contents/MacOS/audio-haxor & )
