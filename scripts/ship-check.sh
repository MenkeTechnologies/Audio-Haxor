#!/usr/bin/env bash
# // AUDIO_HAXOR SHIP CHECK // pre-deploy diagnostics
set -uo pipefail

cd "$(dirname "$0")/.."

# Neon palette
C='\033[1;36m'  # bright cyan
M='\033[1;35m'  # magenta
G='\033[1;32m'  # green
R='\033[1;31m'  # red
Y='\033[1;33m'  # yellow
D='\033[0;90m'  # dim
W='\033[1;37m'  # white
N='\033[0m'     # reset

BAR="${D}──────────────────────────────────────────────${N}"
OK="${G}[ONLINE]${N}"
FAIL="${R}[OFFLINE]${N}"
WARN="${Y}[WARNING]${N}"

echo
echo -e "${D}  ▄▄▄       █    ██ ▓█████▄  ██▓ ▒█████${N}"
echo -e "${D} ▒████▄     ██  ▓██▒▒██▀ ██▌▓██▒▒██▒  ██▒${N}"
echo -e "${C} ██░ ██  ▄▄▄      ▒██   ██▒ ▒█████   ██▀███${N}"
echo -e "${C}▓██░ ██▒▒████▄    ▒▒ █ █ ▒░▒██▒  ██▒▓██ ▒ ██▒${N}"
echo
echo -e "${M}  ╔═══════════════════════════════════════════════╗${N}"
echo -e "${M}  ║  ${W}// SHIP CHECK v1.0 // PRE-DEPLOY DIAGNOSTICS${M}  ║${N}"
echo -e "${M}  ╚═══════════════════════════════════════════════╝${N}"
echo

# ── VERSION MATRIX ──
APP_VER=$(grep '"version"' package.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')
CARGO_VER=$(grep '^version' src-tauri/Cargo.toml | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
TAURI_VER=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')
echo -e "  ${C}> VERSION MATRIX${N}"
echo -e "  $BAR"
echo -e "  ${D}package.json${N}    ${W}$APP_VER${N}"
echo -e "  ${D}Cargo.toml${N}      ${W}$CARGO_VER${N}"
echo -e "  ${D}tauri.conf.json${N} ${W}$TAURI_VER${N}"
if [ "$APP_VER" = "$CARGO_VER" ] && [ "$APP_VER" = "$TAURI_VER" ]; then
  echo -e "  ${OK} versions synced"
else
  echo -e "  ${FAIL} VERSION DESYNC DETECTED"
fi
echo

# ── GIT STATUS ──
BRANCH=$(git branch --show-current)
DIRTY=$(git status -s | grep -v '^\?\?' | wc -l | tr -d ' ')
UNTRACKED=$(git status -s | grep '^\?\?' | wc -l | tr -d ' ')
echo -e "  ${C}> GIT UPLINK${N}"
echo -e "  $BAR"
echo -e "  ${D}branch${N}    ${M}$BRANCH${N}"
echo -e "  ${D}modified${N}  ${W}$DIRTY${N}"
echo -e "  ${D}untracked${N} ${W}$UNTRACKED${N}"
if [ "$DIRTY" = "0" ]; then
  echo -e "  ${OK} clean working tree"
else
  echo -e "  ${WARN} uncommitted mutations"
  git status -s | grep -v '^\?\?' | head -5 | sed "s/^/  ${D}  /"
fi
echo

# ── RUST SUBSYSTEM ──
echo -e "  ${C}> RUST SUBSYSTEM${N}"
echo -e "  $BAR"
RUST_OUT=$(cargo test --manifest-path src-tauri/Cargo.toml --lib 2>&1 | grep 'test result' | tail -1)
RUST_PASS=$(echo "$RUST_OUT" | grep -o '[0-9]* passed' | grep -o '[0-9]*' || echo "0")
RUST_FAIL=$(echo "$RUST_OUT" | grep -o '[0-9]* failed' | grep -o '[0-9]*' || echo "0")
echo -e "  ${D}result${N} ${W}$RUST_OUT${N}"
if [ "$RUST_FAIL" = "0" ]; then
  echo -e "  ${OK} all tests nominal"
else
  echo -e "  ${FAIL} $RUST_FAIL test(s) compromised"
fi
echo

# ── JS SUBSYSTEM ──
echo -e "  ${C}> JS SUBSYSTEM${N}"
echo -e "  $BAR"
JS_OUT=$(node --test test/*.test.js 2>&1 | grep -E '^ℹ (tests|pass|fail)' || true)
JS_TESTS=$(echo "$JS_OUT" | grep 'tests' | grep -o '[0-9]*' || echo 0)
JS_PASS=$(echo "$JS_OUT" | grep 'pass' | grep -o '[0-9]*' || echo 0)
JS_FAIL=$(echo "$JS_OUT" | grep 'fail' | grep -o '[0-9]*' || echo 0)
echo -e "  ${D}tests${N}  ${W}$JS_TESTS${N}  ${D}pass${N} ${G}$JS_PASS${N}  ${D}fail${N} ${R}$JS_FAIL${N}"
if [ "$JS_FAIL" = "0" ]; then
  echo -e "  ${OK} all tests nominal"
else
  echo -e "  ${FAIL} $JS_FAIL test(s) compromised"
fi
echo

# ── CODEBASE METRICS ──
RUST_LINES=$(wc -l src-tauri/src/*.rs | tail -1 | awk '{print $1}')
JS_LINES=$(wc -l frontend/js/*.js | tail -1 | awk '{print $1}')
HTML_LINES=$(wc -l frontend/index.html | awk '{print $1}')
RUST_FILES=$(ls src-tauri/src/*.rs | wc -l | tr -d ' ')
JS_FILES=$(ls frontend/js/*.js | wc -l | tr -d ' ')
TOTAL_TESTS=$((RUST_PASS + JS_PASS))
echo -e "  ${C}> CODEBASE METRICS${N}"
echo -e "  $BAR"
echo -e "  ${D}rust${N}   ${W}${RUST_LINES}${N} ${D}lines${N}  ${D}(${RUST_FILES} modules)${N}"
echo -e "  ${D}js${N}     ${W}${JS_LINES}${N} ${D}lines${N}  ${D}(${JS_FILES} files)${N}"
echo -e "  ${D}html${N}   ${W}${HTML_LINES}${N} ${D}lines${N}"
echo -e "  ${D}tests${N}  ${M}${TOTAL_TESTS}${N} ${D}total${N}  ${D}(${RUST_PASS} rust + ${JS_PASS} js)${N}"
echo

# ── DATABASE ──
echo -e "  ${C}> DATABASE${N}"
echo -e "  $BAR"
DB_PATH="$HOME/Library/Application Support/com.menketechnologies.audio-haxor/audio_haxor.db"
if [ -f "$DB_PATH" ]; then
  DB_SIZE=$(ls -lh "$DB_PATH" | awk '{print $5}')
  echo -e "  ${D}size${N} ${W}$DB_SIZE${N}"
  sqlite3 "$DB_PATH" "
    SELECT '  \033[0;90mplugins\033[0m  \033[1;37m' || COUNT(*) || '\033[0m' FROM plugins;
    SELECT '  \033[0;90msamples\033[0m  \033[1;37m' || COUNT(*) || '\033[0m' FROM audio_samples;
    SELECT '  \033[0;90mdaw\033[0m      \033[1;37m' || COUNT(*) || '\033[0m' FROM daw_projects;
    SELECT '  \033[0;90mpresets\033[0m  \033[1;37m' || COUNT(*) || '\033[0m' FROM presets;
    SELECT '  \033[0;90mkvr\033[0m      \033[1;37m' || COUNT(*) || '\033[0m' FROM kvr_cache;
  " 2>/dev/null || echo -e "  ${R}(query failed)${N}"
  FREE=$(sqlite3 "$DB_PATH" "SELECT freelist_count * 100 / CASE WHEN page_count > 0 THEN page_count ELSE 1 END FROM pragma_page_count, pragma_freelist_count;" 2>/dev/null || echo "?")
  echo -e "  ${D}dead space${N} ${W}${FREE}%${N}"
  if [ "$FREE" != "?" ] && [ "$FREE" -gt 20 ]; then
    echo -e "  ${WARN} run: ${C}pnpm db:vacuum${N}"
  else
    echo -e "  ${OK} compact"
  fi
else
  echo -e "  ${D}no database found${N}"
fi
echo

# ── SYSTEM LOG ──
echo -e "  ${C}> SYSTEM LOG${N}"
echo -e "  $BAR"
LOG_PATH="$HOME/Library/Application Support/com.menketechnologies.audio-haxor/app.log"
if [ -f "$LOG_PATH" ]; then
  LOG_SIZE=$(ls -lh "$LOG_PATH" | awk '{print $5}')
  LOG_LINES=$(wc -l < "$LOG_PATH" | tr -d ' ')
  ERRORS=$(grep -c -E 'ERROR|PANIC|FAILED' "$LOG_PATH" 2>/dev/null || true)
  ERRORS=${ERRORS:-0}
  STARTS=$(grep -c 'APP START' "$LOG_PATH" 2>/dev/null || echo 0)
  SHUTDOWNS=$(grep -c 'APP SHUTDOWN' "$LOG_PATH" 2>/dev/null || echo 0)
  echo -e "  ${D}size${N}      ${W}$LOG_SIZE${N} ${D}($LOG_LINES lines)${N}"
  echo -e "  ${D}starts${N}    ${W}$STARTS${N}"
  echo -e "  ${D}shutdowns${N} ${W}$SHUTDOWNS${N}"
  echo -e "  ${D}errors${N}    ${W}$ERRORS${N}"
  if [ "$ERRORS" -gt 0 ] 2>/dev/null; then
    echo -e "  ${WARN} recent incidents:"
    grep -E 'ERROR|PANIC|FAILED' "$LOG_PATH" | tail -3 | sed "s/^/  ${D}  /"
  else
    echo -e "  ${OK} no incidents"
  fi
else
  echo -e "  ${D}no log file${N}"
fi
echo

# ── BUILD ARTIFACTS ──
echo -e "  ${C}> BUILD ARTIFACTS${N}"
echo -e "  $BAR"
APP="src-tauri/target/release/bundle/macos/AUDIO_HAXOR.app"
DMG="src-tauri/target/release/bundle/dmg/AUDIO_HAXOR_${APP_VER}_aarch64.dmg"
if [ -d "$APP" ]; then
  APP_SIZE=$(du -sh "$APP" | awk '{print $1}')
  echo -e "  ${D}.app${N}  ${W}$APP_SIZE${N}"
else
  echo -e "  ${WARN} no .app — run: ${C}pnpm tauri build${N}"
fi
if [ -f "$DMG" ]; then
  DMG_SIZE=$(ls -lh "$DMG" | awk '{print $5}')
  echo -e "  ${D}.dmg${N}  ${W}$DMG_SIZE${N}"
else
  echo -e "  ${WARN} no .dmg"
fi
echo

# ── FINAL VERDICT ──
TOTAL_ISSUES=0
[ "$DIRTY" != "0" ] && TOTAL_ISSUES=$((TOTAL_ISSUES + 1))
[ "$RUST_FAIL" != "0" ] && TOTAL_ISSUES=$((TOTAL_ISSUES + 1))
[ "$JS_FAIL" != "0" ] && TOTAL_ISSUES=$((TOTAL_ISSUES + 1))
[ "$APP_VER" != "$CARGO_VER" ] && TOTAL_ISSUES=$((TOTAL_ISSUES + 1))

echo -e "  ${M}═══════════════════════════════════════════════${N}"
if [ "$TOTAL_ISSUES" = "0" ]; then
  echo
  echo -e "  ${G}  ██████  ██   ██ ██ ██████      ██ ████████${N}"
  echo -e "  ${G}  ██      ██   ██ ██ ██   ██     ██    ██${N}"
  echo -e "  ${G}  ██████  ███████ ██ ██████      ██    ██${N}"
  echo -e "  ${G}      ██  ██   ██ ██ ██          ██    ██${N}"
  echo -e "  ${G}  ██████  ██   ██ ██ ██      ██  ██    ██${N}"
  echo
  echo -e "  ${C}// SYSTEM NOMINAL — ALL CHECKS PASSED //${N}"
  echo -e "  ${D}deploy when ready: pnpm tauri build${N}"
else
  echo
  echo -e "  ${R}  ██   ██  ██████  ██      ████████${N}"
  echo -e "  ${R}  ██   ██ ██    ██ ██         ██${N}"
  echo -e "  ${R}  ███████ ████████ ██         ██${N}"
  echo -e "  ${R}  ██   ██ ██    ██ ██         ██${N}"
  echo -e "  ${R}  ██   ██ ██    ██ ███████    ██${N}"
  echo
  echo -e "  ${R}// $TOTAL_ISSUES CRITICAL ISSUE(S) DETECTED — DO NOT DEPLOY //${N}"
fi
echo
