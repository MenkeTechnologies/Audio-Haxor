#!/usr/bin/env node
/**
 * Run all `test/*.test.js` files via `node --test` without shell glob expansion
 * (Windows `cmd` / PowerShell do not expand `test/*.test.js` the same as bash).
 */
import { spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { join } from 'node:path';

const files = readdirSync('test', { withFileTypes: true })
  .filter((d) => d.isFile() && d.name.endsWith('.test.js'))
  .map((d) => join('test', d.name))
  .sort();

const r = spawnSync(process.execPath, ['--test', ...files], {
  stdio: 'inherit',
  shell: false,
});
process.exit(r.status === null ? 1 : r.status);
