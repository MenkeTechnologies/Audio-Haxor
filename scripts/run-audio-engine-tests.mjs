#!/usr/bin/env node
/**
 * Runs `test/audio-engine-ipc.test.js` only (not part of `scripts/run-js-tests.mjs`).
 * Run after building the binary: `node scripts/build-audio-engine.mjs` or CI `prepare-audio-engine-audioengine.mjs`.
 * Linux: use `xvfb-run -a node scripts/run-audio-engine-tests.mjs` when no display (see CI).
 */
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');
const testFile = path.join(root, 'test', 'audio-engine-ipc.test.js');

if (!fs.existsSync(testFile)) {
  console.error('run-audio-engine-tests: missing', testFile);
  process.exit(1);
}

const r = spawnSync(process.execPath, ['--test', testFile], {
  stdio: 'inherit',
  cwd: root,
  shell: false,
});

process.exit(r.status === null ? 1 : r.status);
