#!/usr/bin/env node
/**
 * Build the JUCE `audio-engine` AudioEngine (CMake + Ninja) into `target/<debug|release>/audio-engine`.
 * Used by `beforeDevCommand` (debug) and `prepare-audio-engine-audioengine.mjs` (release).
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');
const buildType = process.env.AUDIO_ENGINE_BUILD_TYPE === 'release' ? 'Release' : 'Debug';
const buildDir = path.join(root, 'audio-engine', 'build');
const ext = process.platform === 'win32' ? '.exe' : '';

fs.mkdirSync(buildDir, { recursive: true });

const cmakeArgs = [
  '-S',
  path.join(root, 'audio-engine'),
  '-B',
  buildDir,
  '-G',
  'Ninja',
  `-DCMAKE_BUILD_TYPE=${buildType}`,
];

execFileSync('cmake', cmakeArgs, { stdio: 'inherit', cwd: root });
execFileSync('cmake', ['--build', buildDir, '--parallel'], { stdio: 'inherit', cwd: root });

const outDir = path.join(root, 'target', buildType === 'Debug' ? 'debug' : 'release');
const outName = `audio-engine${ext}`;
const built = path.join(outDir, outName);
if (!fs.existsSync(built)) {
  console.error(`build-audio-engine: expected ${built}`);
  process.exit(1);
}
console.log(`build-audio-engine: ${built}`);
