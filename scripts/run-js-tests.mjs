#!/usr/bin/env node
/**
 * Run all `test/*.test.js` files via `node --test` without shell glob expansion
 * (Windows `cmd` / PowerShell do not expand `test/*.test.js` the same as bash).
 *
 * Excludes `test/audio-engine-ipc.test.js` (spawned JUCE binary; use `run-audio-engine-tests.mjs`).
 * Spawns in batches: the full argv list (~8.4k chars for 295 files) exceeds
 * Windows CreateProcess command-line limit (~8191), so CI would fail on
 * windows-latest without chunking.
 */
import { spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { join } from 'node:path';

const files = readdirSync('test', { withFileTypes: true })
  .filter((d) => d.isFile() && d.name.endsWith('.test.js'))
  .map((d) => join('test', d.name))
  // Spawned binary + display — run via `node scripts/run-audio-engine-tests.mjs` after build (see CI).
  .filter((f) => f !== join('test', 'audio-engine-ipc.test.js'))
  .sort();

/** Stay well under Windows ~8191 char limit for the full command line. */
const BATCH_SIZE = 28;

for (let i = 0; i < files.length; i += BATCH_SIZE) {
  const batch = files.slice(i, i + BATCH_SIZE);
  const r = spawnSync(process.execPath, ['--test', ...batch], {
    stdio: 'inherit',
    shell: false,
  });
  const code = r.status === null ? 1 : r.status;
  if (code !== 0) {
    process.exit(code);
  }
}
