#!/usr/bin/env node
/**
 * Build `audio-engine` (JUCE + CMake) and copy to `src-tauri/binaries/` with the target-triple suffix
 * required by Tauri `bundle.externalBin`. Run before `pnpm tauri build` (see `tauri.conf.json`).
 *
 * Set `AUDIO_ENGINE_TAURI_BIN_PROFILE=debug` for a faster Debug build (e.g. GitHub Actions test job).
 * Default is release (distribution builds).
 *
 * Tauri runs `beforeBuildCommand` with cwd = repository root (parent of `src-tauri/`), so
 * `tauri.conf.json` must invoke `node scripts/prepare-audio-engine-audioengine.mjs`, not `../scripts/...`.
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');
const triple = execFileSync('rustc', ['--print', 'host-tuple'], {
  encoding: 'utf8',
}).trim();
if (!triple) {
  console.error('prepare-audio-engine-audioengine: could not read rustc host tuple');
  process.exit(1);
}

const ext = process.platform === 'win32' ? '.exe' : '';
const cargoDir = path.join(root, 'src-tauri');
const useDebug = process.env.AUDIO_ENGINE_TAURI_BIN_PROFILE === 'debug';
const profileDir = useDebug ? 'debug' : 'release';
const buildTypeEnv = useDebug ? 'debug' : 'release';
const built = path.join(root, 'audio-engine-artifacts', profileDir, `audio-engine${ext}`);
const outDir = path.join(cargoDir, 'binaries');
const outName = `audio-engine-${triple}${ext}`;
const dest = path.join(outDir, outName);

execFileSync(process.execPath, [path.join(root, 'scripts', 'build-audio-engine.mjs')], {
  stdio: 'inherit',
  cwd: root,
  env: { ...process.env, AUDIO_ENGINE_BUILD_TYPE: buildTypeEnv },
});

if (!fs.existsSync(built)) {
  console.error(`prepare-audio-engine-audioengine: missing ${built}`);
  process.exit(1);
}

fs.mkdirSync(outDir, { recursive: true });
fs.copyFileSync(built, dest);
console.log(`prepare-audio-engine-audioengine: ${dest}`);
