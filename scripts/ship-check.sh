#!/usr/bin/env bash
# // AUDIO_HAXOR SHIP CHECK // pre-deploy system diagnostics
set -uo pipefail
cd "$(dirname "$0")/.."

C='\033[1;36m'; M='\033[1;35m'; G='\033[1;32m'; R='\033[1;31m'
Y='\033[1;33m'; D='\033[0;90m'; W='\033[1;37m'; N='\033[0m'

APP_VER=$(grep '"version"' package.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')
CARGO_VER=$(grep '^version' src-tauri/Cargo.toml | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
TAURI_VER=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')

echo
echo -e " ${C}  █████╗ ██╗   ██╗██████╗ ██╗ ██████╗${N}"
echo -e " ${C} ██╔══██╗██║   ██║██╔══██╗██║██╔═══██╗${N}"
echo -e " ${C} ███████║██║   ██║██║  ██║██║██║   ██║${N}"
echo -e " ${C} ██╔══██║██║   ██║██║  ██║██║██║   ██║${N}"
echo -e " ${C} ██║  ██║╚██████╔╝██████╔╝██║╚██████╔╝${N}"
echo -e " ${C} ╚═╝  ╚═╝ ╚═════╝ ╚═════╝ ╚═╝ ╚═════╝${N}"
echo -e " ${M} ██╗  ██╗ █████╗ ██╗  ██╗ ██████╗ ██████╗${N}"
echo -e " ${M} ██║  ██║██╔══██╗╚██╗██╔╝██╔═══██╗██╔══██╗${N}"
echo -e " ${M} ███████║███████║ ╚███╔╝ ██║   ██║██████╔╝${N}"
echo -e " ${M} ██╔══██║██╔══██║ ██╔██╗ ██║   ██║██╔══██╗${N}"
echo -e " ${M} ██║  ██║██║  ██║██╔╝ ██╗╚██████╔╝██║  ██║${N}"
echo -e " ${M} ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝${N}"
echo -e " ${D}┌──────────────────────────────────────────────────────┐${N}"
echo -e " ${D}│${N} ${W}STATUS:${N} ${G}ONLINE${N}  ${D}//${N} ${W}SIGNAL:${N} ${C}████████░░${N} ${D}//${N} ${W}v${APP_VER}${N}   ${D}│${N}"
echo -e " ${D}└──────────────────────────────────────────────────────┘${N}"
echo -e "  ${D}>> SHIP CHECK // PRE-DEPLOY SYSTEM DIAGNOSTICS <<${N}"
echo

# ── VERSION MATRIX ─────────────────────────────────────
echo -e "  ${D}── VERSION MATRIX ─────────────────────────────────────${N}"
echo -e "  ${D}package.json${N}    ${W}$APP_VER${N}"
echo -e "  ${D}Cargo.toml${N}      ${W}$CARGO_VER${N}"
echo -e "  ${D}tauri.conf.json${N} ${W}$TAURI_VER${N}"
if [ "$APP_VER" = "$CARGO_VER" ] && [ "$APP_VER" = "$TAURI_VER" ]; then
  echo -e "  ${G}[SYNCED]${N} ${D}// all versions locked${N}"
else
  echo -e "  ${R}[DESYNC]${N} ${D}// version mismatch detected${N}"
fi
echo

# ── GIT UPLINK ─────────────────────────────────────────
BRANCH=$(git branch --show-current)
DIRTY=$(git status -s | grep -v '^\?\?' | wc -l | tr -d ' ')
UNTRACKED=$(git status -s | grep '^\?\?' | wc -l | tr -d ' ')
echo -e "  ${D}── GIT UPLINK ─────────────────────────────────────────${N}"
echo -e "  ${D}branch${N}    ${M}$BRANCH${N}"
echo -e "  ${D}modified${N}  ${W}$DIRTY${N}"
echo -e "  ${D}untracked${N} ${W}$UNTRACKED${N}"
if [ "$DIRTY" = "0" ]; then
  echo -e "  ${G}[CLEAN]${N} ${D}// working tree nominal${N}"
else
  echo -e "  ${Y}[DIRTY]${N} ${D}// uncommitted mutations${N}"
  git status -s | grep -v '^\?\?' | head -5 | sed "s/^/  ${D}  /" | sed "s/$/${N}/"
fi
echo

# ── RUST SUBSYSTEM ─────────────────────────────────────
echo -e "  ${D}── RUST SUBSYSTEM ─────────────────────────────────────${N}"
RUST_OUT=$(cargo test --manifest-path src-tauri/Cargo.toml --lib 2>&1 | grep 'test result' | tail -1)
RUST_PASS=$(echo "$RUST_OUT" | grep -o '[0-9]* passed' | grep -o '[0-9]*' || echo "0")
RUST_FAIL=$(echo "$RUST_OUT" | grep -o '[0-9]* failed' | grep -o '[0-9]*' || echo "0")
echo -e "  ${W}$RUST_OUT${N}"
if [ "$RUST_FAIL" = "0" ]; then
  echo -e "  ${G}[PASS]${N} ${D}// all systems nominal${N}"
else
  echo -e "  ${R}[FAIL]${N} ${D}// $RUST_FAIL test(s) compromised${N}"
fi
echo

# ── JS SUBSYSTEM ───────────────────────────────────────
echo -e "  ${D}── JS SUBSYSTEM ───────────────────────────────────────${N}"
JS_OUT=$(node --test test/*.test.js 2>&1 | grep -E '^ℹ (tests|pass|fail)' || true)
JS_TESTS=$(echo "$JS_OUT" | grep 'tests' | grep -o '[0-9]*' || echo 0)
JS_PASS=$(echo "$JS_OUT" | grep 'pass' | grep -o '[0-9]*' || echo 0)
JS_FAIL=$(echo "$JS_OUT" | grep 'fail' | grep -o '[0-9]*' || echo 0)
echo -e "  ${D}tests${N} ${W}$JS_TESTS${N}  ${D}pass${N} ${G}$JS_PASS${N}  ${D}fail${N} ${R}$JS_FAIL${N}"
if [ "$JS_FAIL" = "0" ]; then
  echo -e "  ${G}[PASS]${N} ${D}// all systems nominal${N}"
else
  echo -e "  ${R}[FAIL]${N} ${D}// $JS_FAIL test(s) compromised${N}"
fi
echo

# ── CODEBASE METRICS ───────────────────────────────────
RUST_LINES=$(wc -l src-tauri/src/*.rs | tail -1 | awk '{print $1}')
JS_LINES=$(wc -l frontend/js/*.js | tail -1 | awk '{print $1}')
HTML_LINES=$(wc -l frontend/index.html | awk '{print $1}')
RUST_FILES=$(ls src-tauri/src/*.rs | wc -l | tr -d ' ')
JS_FILES=$(ls frontend/js/*.js | wc -l | tr -d ' ')
TOTAL_TESTS=$((RUST_PASS + JS_PASS))
echo -e "  ${D}── CODEBASE METRICS ───────────────────────────────────${N}"
echo -e "  ${D}rust${N}   ${W}${RUST_LINES}${N} ${D}lines // ${RUST_FILES} modules${N}"
echo -e "  ${D}js${N}     ${W}${JS_LINES}${N} ${D}lines // ${JS_FILES} files${N}"
echo -e "  ${D}html${N}   ${W}${HTML_LINES}${N} ${D}lines${N}"
echo -e "  ${D}tests${N}  ${C}${TOTAL_TESTS}${N} ${D}total // ${RUST_PASS} rust + ${JS_PASS} js${N}"
echo

# ── DATABASE ───────────────────────────────────────────
echo -e "  ${D}── DATABASE ───────────────────────────────────────────${N}"
DB_PATH="$HOME/Library/Application Support/com.menketechnologies.audio-haxor/audio_haxor.db"
if [ -f "$DB_PATH" ]; then
  DB_SIZE=$(ls -lh "$DB_PATH" | awk '{print $5}')
  echo -e "  ${D}size${N}     ${W}$DB_SIZE${N}"
  PLUGINS=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM plugins;" 2>/dev/null || echo "?")
  SAMPLES=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM audio_samples;" 2>/dev/null || echo "?")
  DAW=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM daw_projects;" 2>/dev/null || echo "?")
  PRESETS=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM presets;" 2>/dev/null || echo "?")
  KVR=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM kvr_cache;" 2>/dev/null || echo "?")
  echo -e "  ${D}plugins${N}  ${W}$PLUGINS${N}  ${D}samples${N} ${W}$SAMPLES${N}  ${D}daw${N} ${W}$DAW${N}"
  echo -e "  ${D}presets${N}  ${W}$PRESETS${N}  ${D}kvr${N} ${W}$KVR${N}"
  FREE=$(sqlite3 "$DB_PATH" "SELECT freelist_count * 100 / CASE WHEN page_count > 0 THEN page_count ELSE 1 END FROM pragma_page_count, pragma_freelist_count;" 2>/dev/null || echo "?")
  echo -e "  ${D}dead${N}     ${W}${FREE}%${N}"
  if [ "$FREE" != "?" ] && [ "$FREE" -gt 20 ]; then
    echo -e "  ${Y}[BLOAT]${N} ${D}// run: pnpm db:vacuum${N}"
  else
    echo -e "  ${G}[COMPACT]${N} ${D}// storage optimal${N}"
  fi
else
  echo -e "  ${D}no database found${N}"
fi
echo

# ── SYSTEM LOG ─────────────────────────────────────────
echo -e "  ${D}── SYSTEM LOG ─────────────────────────────────────────${N}"
LOG_PATH="$HOME/Library/Application Support/com.menketechnologies.audio-haxor/app.log"
if [ -f "$LOG_PATH" ]; then
  LOG_SIZE=$(ls -lh "$LOG_PATH" | awk '{print $5}')
  LOG_LINES=$(wc -l < "$LOG_PATH" | tr -d ' ')
  ERRORS=$(grep -c -E 'ERROR|PANIC|FAILED' "$LOG_PATH" 2>/dev/null || true)
  ERRORS=${ERRORS:-0}
  STARTS=$(grep -c 'APP START' "$LOG_PATH" 2>/dev/null || echo 0)
  SHUTDOWNS=$(grep -c 'APP SHUTDOWN' "$LOG_PATH" 2>/dev/null || echo 0)
  echo -e "  ${D}size${N}      ${W}$LOG_SIZE${N} ${D}// $LOG_LINES lines${N}"
  echo -e "  ${D}starts${N}    ${W}$STARTS${N}  ${D}shutdowns${N} ${W}$SHUTDOWNS${N}  ${D}errors${N} ${W}$ERRORS${N}"
  if [ "$ERRORS" -gt 0 ] 2>/dev/null; then
    echo -e "  ${R}[ALERT]${N} ${D}// incidents detected${N}"
    grep -E 'ERROR|PANIC|FAILED' "$LOG_PATH" | tail -3 | sed "s/^/  ${D}  /"
  else
    echo -e "  ${G}[CLEAR]${N} ${D}// no incidents${N}"
  fi
else
  echo -e "  ${D}no log file${N}"
fi
echo

# ── BUILD ARTIFACTS ────────────────────────────────────
echo -e "  ${D}── BUILD ARTIFACTS ────────────────────────────────────${N}"
APP="src-tauri/target/release/bundle/macos/AUDIO_HAXOR.app"
DMG="src-tauri/target/release/bundle/dmg/AUDIO_HAXOR_${APP_VER}_aarch64.dmg"
if [ -d "$APP" ]; then
  APP_SIZE=$(du -sh "$APP" | awk '{print $1}')
  echo -e "  ${D}.app${N}  ${W}$APP_SIZE${N}"
else
  echo -e "  ${Y}[MISSING]${N} ${D}// run: pnpm tauri build${N}"
fi
if [ -f "$DMG" ]; then
  DMG_SIZE=$(ls -lh "$DMG" | awk '{print $5}')
  echo -e "  ${D}.dmg${N}  ${W}$DMG_SIZE${N}"
else
  echo -e "  ${Y}[MISSING]${N} ${D}// .dmg not built${N}"
fi
echo

# ── FINAL VERDICT ──────────────────────────────────────
TOTAL_ISSUES=0
[ "$DIRTY" != "0" ] && TOTAL_ISSUES=$((TOTAL_ISSUES + 1))
[ "$RUST_FAIL" != "0" ] && TOTAL_ISSUES=$((TOTAL_ISSUES + 1))
[ "$JS_FAIL" != "0" ] && TOTAL_ISSUES=$((TOTAL_ISSUES + 1))
[ "$APP_VER" != "$CARGO_VER" ] && TOTAL_ISSUES=$((TOTAL_ISSUES + 1))

echo -e " ${D}░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░${N}"
if [ "$TOTAL_ISSUES" = "0" ]; then
  echo
  echo -e "  ${G}>>> JACK IN. DEPLOY. OWN YOUR AUDIO. <<<${N}"
  echo -e "  ${C}// SYSTEM NOMINAL — ALL CHECKS PASSED //${N}"
  echo
else
  echo
  echo -e "  ${R}>>> $TOTAL_ISSUES CRITICAL ISSUE(S) — DO NOT DEPLOY <<<${N}"
  echo -e "  ${Y}// FIX BEFORE SHIPPING //${N}"
  echo
fi
echo -e " ${D}░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░${N}"
echo
