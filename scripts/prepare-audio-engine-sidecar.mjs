#!/usr/bin/env node
/**
 * Build `audio-engine` in release (JUCE + CMake) and copy to `src-tauri/binaries/` with the target-triple suffix
 * required by Tauri `bundle.externalBin`. Run before `pnpm tauri build` (see `tauri.conf.json`).
 *
 * Tauri runs `beforeBuildCommand` with cwd = repository root (parent of `src-tauri/`), so
 * `tauri.conf.json` must invoke `node scripts/prepare-audio-engine-sidecar.mjs`, not `../scripts/...`.
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
  console.error('prepare-audio-engine-sidecar: could not read rustc host tuple');
  process.exit(1);
}

const ext = process.platform === 'win32' ? '.exe' : '';
const cargoDir = path.join(root, 'src-tauri');
const built = path.join(root, 'target', 'release', `audio-engine${ext}`);
const outDir = path.join(cargoDir, 'binaries');
const outName = `audio-engine-${triple}${ext}`;
const dest = path.join(outDir, outName);

process.env.AUDIO_ENGINE_BUILD_TYPE = 'release';
execFileSync(process.execPath, [path.join(root, 'scripts', 'build-audio-engine.mjs')], {
  stdio: 'inherit',
  cwd: root,
  env: { ...process.env, AUDIO_ENGINE_BUILD_TYPE: 'release' },
});

if (!fs.existsSync(built)) {
  console.error(`prepare-audio-engine-sidecar: missing ${built}`);
  process.exit(1);
}

fs.mkdirSync(outDir, { recursive: true });
fs.copyFileSync(built, dest);
console.log(`prepare-audio-engine-sidecar: ${dest}`);
