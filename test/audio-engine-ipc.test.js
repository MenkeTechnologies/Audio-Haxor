/**
 * Integration tests: spawn built `audio-engine`, one JSON line per stdin line, assert stdout.
 * Requires `target/debug/audio-engine` or `target/release/audio-engine` (or `AUDIO_ENGINE_TEST_BIN`).
 * On Linux CI, run under `xvfb-run -a` (see `.github/workflows/ci.yml`) — JUCE needs a display.
 */
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawn } = require('node:child_process');
const readline = require('node:readline');
const { describe, it, before, after } = require('node:test');
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

/** One JSON object per stdin line (safe paths on Windows). */
function jl(obj) {
  return JSON.stringify(obj);
}

if (!bin) {
  describe.skip('audio-engine IPC (no binary — build with node scripts/build-audio-engine.mjs)', () => {
    it('skipped', () => {});
  });
} else {
  describe('audio-engine IPC (stdin/stdout)', () => {
    const missingAbsPath = path.join(root, '___audio_haxor_ipc_test_missing_file___');
    /** Exists on disk but not decodable as audio — distinct from missing path. */
    let tmpEmptyFile;

    before(() => {
      tmpEmptyFile = path.join(os.tmpdir(), `audio-haxor-ipc-empty-${process.pid}.bin`);
      fs.writeFileSync(tmpEmptyFile, Buffer.alloc(0));
    });

    after(() => {
      try {
        fs.unlinkSync(tmpEmptyFile);
      } catch {
        /* ignore */
      }
    });

    it('ping returns ok, version matches CMake project, host juce', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'ping' })]);
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

    it('two bad JSON lines yield two error lines', async () => {
      const { outLines } = await runEngineExchange(bin, ['{{{', 'also not json']);
      assert.equal(outLines.length, 2);
      for (const line of outLines) {
        const j = JSON.parse(line);
        assert.equal(j.ok, false);
        assert.equal(j.error, 'bad JSON');
      }
    });

    it('ping ignores extra JSON fields', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'ping', trace: 1, extra: 'x' })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.version, cmakeVersion);
    });

    it('cmd is matched case-insensitively (Ping)', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'Ping' })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.host, 'juce');
    });

    it('two sequential pings return two lines', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'ping' }), jl({ cmd: 'ping' })]);
      assert.equal(outLines.length, 2);
      const a = JSON.parse(outLines[0]);
      const b = JSON.parse(outLines[1]);
      assert.equal(a.ok, true);
      assert.equal(b.ok, true);
      assert.equal(a.version, b.version);
    });

    it('playback_load missing file returns not a file (absolute path, no device init)', async () => {
      const { outLines, code } = await runEngineExchange(bin, [jl({ cmd: 'playback_load', path: missingAbsPath })]);
      assert.equal(outLines.length, 1);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /not a file/);
      assert.equal(code, 0);
    });

    it('playback_load rejects empty path', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_load', path: '' })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /path required/);
    });

    it('playback_load rejects omitted path', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_load' })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /path required/);
    });

    it('waveform_preview rejects omitted path', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'waveform_preview' })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /path required/);
    });

    it('waveform_preview rejects missing file', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'waveform_preview', path: missingAbsPath })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /not a file/);
    });

    it('spectrogram_preview rejects omitted path', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'spectrogram_preview' })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /path required/);
    });

    it('spectrogram_preview rejects missing file', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'spectrogram_preview', path: missingAbsPath })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /not a file/);
    });

    it('playback_load rejects empty on-disk file (no supported format)', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_load', path: tmpEmptyFile })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unsupported or unreadable/);
    });

    it('waveform_preview rejects empty on-disk file', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'waveform_preview', path: tmpEmptyFile })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unsupported or unreadable/);
    });

    it('spectrogram_preview rejects empty on-disk file', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'spectrogram_preview', path: tmpEmptyFile })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unsupported or unreadable/);
    });

    it('mixed ping + preview validation in one session', async () => {
      const lines = [
        jl({ cmd: 'ping' }),
        jl({ cmd: 'waveform_preview', path: missingAbsPath }),
        jl({ cmd: 'ping' }),
      ];
      const { outLines } = await runEngineExchange(bin, lines);
      assert.equal(outLines.length, 3);
      const p0 = JSON.parse(outLines[0]);
      const p1 = JSON.parse(outLines[1]);
      const p2 = JSON.parse(outLines[2]);
      assert.equal(p0.ok, true);
      assert.equal(p1.ok, false);
      assert.match(String(p1.error || ''), /not a file/);
      assert.equal(p2.ok, true);
      assert.equal(p2.version, p0.version);
    });
  });
}
