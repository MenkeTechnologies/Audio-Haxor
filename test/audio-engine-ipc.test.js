/**
 * Integration tests: spawn built `audio-engine`, one JSON line per stdin line, assert stdout.
 * Requires `target/debug/audio-engine` or `target/release/audio-engine` (or `AUDIO_ENGINE_TEST_BIN`).
 * On Linux CI, run under `xvfb-run -a` (see `.github/workflows/ci.yml`) — JUCE needs a display.
 */
const fs = require('fs');
const path = require('path');
const { spawn } = require('node:child_process');
const readline = require('node:readline');
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

const root = path.join(__dirname, '..');

function readProjectVersionFromCMake() {
  const cmakePath = path.join(root, 'audio-engine', 'CMakeLists.txt');
  const text = fs.readFileSync(cmakePath, 'utf8');
  const m = text.match(/project\s*\(\s*audio_engine\s+VERSION\s+([\d.]+)/i);
  if (!m) {
    throw new Error('could not parse audio_engine VERSION from audio-engine/CMakeLists.txt');
  }
  return m[1];
}

function resolveAudioEngineBin() {
  if (process.env.AUDIO_ENGINE_TEST_BIN) {
    return process.env.AUDIO_ENGINE_TEST_BIN;
  }
  const ext = process.platform === 'win32' ? '.exe' : '';
  const debug = path.join(root, 'target', 'debug', `audio-engine${ext}`);
  const release = path.join(root, 'target', 'release', `audio-engine${ext}`);
  if (fs.existsSync(debug)) {
    return debug;
  }
  if (fs.existsSync(release)) {
    return release;
  }
  return null;
}

/**
 * @param {string} bin
 * @param {string[]} requestLines - one JSON object per line
 * @param {{ timeoutMs?: number }} [opts]
 * @returns {Promise<{ code: number | null, signal: NodeJS.Signals | null, outLines: string[], stderr: string }>}
 */
function runEngineExchange(bin, requestLines, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 45_000;
  const expected = requestLines.length;
  return new Promise((resolve, reject) => {
    const child = spawn(bin, [], { stdio: ['pipe', 'pipe', 'pipe'] });
    const outLines = [];
    const stderrChunks = [];
    let settled = false;

    child.stderr.on('data', (c) => {
      stderrChunks.push(c.toString());
    });

    const rl = readline.createInterface({ input: child.stdout });
    const timer = setTimeout(() => {
      if (settled) {
        return;
      }
      settled = true;
      child.kill('SIGKILL');
      reject(
        new Error(
          `audio-engine: timeout after ${timeoutMs}ms (stderr tail: ${stderrChunks.join('').slice(-800)})`,
        ),
      );
    }, timeoutMs);

    rl.on('line', (line) => {
      outLines.push(line);
      if (outLines.length >= expected) {
        clearTimeout(timer);
        child.stdin.end();
      }
    });

    child.on('error', (err) => {
      clearTimeout(timer);
      if (!settled) {
        settled = true;
        reject(err);
      }
    });

    for (const l of requestLines) {
      child.stdin.write(`${l}\n`);
    }

    child.on('close', (code, signal) => {
      clearTimeout(timer);
      if (settled) {
        return;
      }
      settled = true;
      if (outLines.length !== expected) {
        reject(
          new Error(
            `expected ${expected} stdout lines, got ${outLines.length}, code=${code}, signal=${signal}, stderr=${stderrChunks.join('')}`,
          ),
        );
        return;
      }
      resolve({ code, signal, outLines, stderr: stderrChunks.join('') });
    });
  });
}

const bin = resolveAudioEngineBin();
const cmakeVersion = readProjectVersionFromCMake();

if (!bin) {
  describe.skip('audio-engine IPC (no binary — build with node scripts/build-audio-engine.mjs)', () => {
    it('skipped', () => {});
  });
} else {
  describe('audio-engine IPC (stdin/stdout)', () => {
    it('ping returns ok, version matches CMake project, host juce', async () => {
      const { outLines } = await runEngineExchange(bin, ['{"cmd":"ping"}']);
      assert.equal(outLines.length, 1);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.host, 'juce');
      assert.equal(typeof j.version, 'string');
      assert.equal(j.version, cmakeVersion);
    });

    it('bad JSON line yields ok:false and bad JSON error', async () => {
      const { outLines } = await runEngineExchange(bin, ['not valid json {{{']);
      assert.equal(outLines.length, 1);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.equal(j.error, 'bad JSON');
    });

    it('two sequential pings return two lines', async () => {
      const { outLines } = await runEngineExchange(bin, ['{"cmd":"ping"}', '{"cmd":"ping"}']);
      assert.equal(outLines.length, 2);
      const a = JSON.parse(outLines[0]);
      const b = JSON.parse(outLines[1]);
      assert.equal(a.ok, true);
      assert.equal(b.ok, true);
      assert.equal(a.version, b.version);
    });

    it('playback_load with missing path returns ok:false before device init', async () => {
      const { outLines, code } = await runEngineExchange(bin, [
        '{"cmd":"playback_load","path":"/___audio_haxor_test_no_such_file___"}',
      ]);
      assert.equal(outLines.length, 1);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /not a file/);
      assert.equal(code, 0);
    });
  });
}
