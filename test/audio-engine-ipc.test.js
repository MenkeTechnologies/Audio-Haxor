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
 * @param {string[]} requestLines - lines sent on stdin (each followed by \\n). Empty / whitespace-only lines produce no stdout (Main.cpp).
 * @param {{ timeoutMs?: number, expectedOutputLines?: number }} [opts] — default expectedOutputLines = requestLines.length
 * @returns {Promise<{ code: number | null, signal: NodeJS.Signals | null, outLines: string[], stderr: string }>}
 */
function runEngineExchange(bin, requestLines, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 45_000;
  const expectedOut = opts.expectedOutputLines ?? requestLines.length;
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
      if (outLines.length >= expectedOut) {
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
      if (outLines.length !== expectedOut) {
        reject(
          new Error(
            `expected ${expectedOut} stdout lines, got ${outLines.length}, code=${code}, signal=${signal}, stderr=${stderrChunks.join('')}`,
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

/** Mono PCM 16-bit LE WAV (silence). JUCE `registerBasicFormats` accepts standard WAVE. */
function pcm16WavMono(sampleRate, numSamples) {
  const bitsPerSample = 16;
  const numChannels = 1;
  const blockAlign = numChannels * (bitsPerSample / 8);
  const byteRate = sampleRate * blockAlign;
  const dataSize = numSamples * blockAlign;
  const buf = Buffer.alloc(44 + dataSize);
  buf.write('RIFF', 0);
  buf.writeUInt32LE(36 + dataSize, 4);
  buf.write('WAVE', 8);
  buf.write('fmt ', 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(numChannels, 22);
  buf.writeUInt32LE(sampleRate, 24);
  buf.writeUInt32LE(byteRate, 28);
  buf.writeUInt16LE(blockAlign, 32);
  buf.writeUInt16LE(bitsPerSample, 34);
  buf.write('data', 36);
  buf.writeUInt32LE(dataSize, 40);
  return buf;
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
    /** Exists as a directory — JUCE `existsAsFile` is false. */
    let tmpDir;
    /** Short PCM WAV (~8k mono samples @ 44.1kHz) for decode success paths. */
    let tmpWav;

    before(() => {
      tmpEmptyFile = path.join(os.tmpdir(), `audio-haxor-ipc-empty-${process.pid}.bin`);
      fs.writeFileSync(tmpEmptyFile, Buffer.alloc(0));
      tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'audio-haxor-ipc-dir-'));
      tmpWav = path.join(os.tmpdir(), `audio-haxor-ipc-${process.pid}.wav`);
      fs.writeFileSync(tmpWav, pcm16WavMono(44100, 8192));
    });

    after(() => {
      try {
        fs.unlinkSync(tmpEmptyFile);
      } catch {
        /* ignore */
      }
      try {
        fs.rmSync(tmpDir, { recursive: true, force: true });
      } catch {
        /* ignore */
      }
      try {
        fs.unlinkSync(tmpWav);
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

    it('four bad JSON lines yield four error lines', async () => {
      const { outLines } = await runEngineExchange(bin, ['x', 'y', '{', ']']);
      assert.equal(outLines.length, 4);
      for (const line of outLines) {
        assert.equal(JSON.parse(line).error, 'bad JSON');
      }
    });

    it('ping, bad JSON, ping in one session', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'ping' }), '{', jl({ cmd: 'ping' })]);
      assert.equal(outLines.length, 3);
      assert.equal(JSON.parse(outLines[0]).ok, true);
      assert.equal(JSON.parse(outLines[1]).error, 'bad JSON');
      assert.equal(JSON.parse(outLines[2]).ok, true);
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

    it('ping succeeds when unrelated path field is present', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'ping', path: missingAbsPath })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.version, cmakeVersion);
    });

    it('empty and whitespace-only stdin lines produce no stdout line', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        ['', '   ', '\t', jl({ cmd: 'ping' })],
        { expectedOutputLines: 1 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
    });

    it('trailing blank stdin line after ping does not add a second stdout line', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'ping' }), ''], { expectedOutputLines: 1 });
      assert.equal(JSON.parse(outLines[0]).ok, true);
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

    it('playback_load rejects a directory path (not a file)', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_load', path: tmpDir })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /not a file/);
    });

    it('waveform_preview rejects a directory path', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'waveform_preview', path: tmpDir })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /not a file/);
    });

    it('playback_load rejects empty path', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_load', path: '' })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /path required/);
    });

    it('playback_load fails for whitespace-only path (not empty; not a file)', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_load', path: '   ' })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /not a file/);
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

    it('playback_load succeeds on minimal PCM WAV', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_load', path: tmpWav })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.sample_rate_hz, 44100);
      assert.ok(typeof j.duration_sec === 'number' && j.duration_sec > 0);
    });

    it('waveform_preview clamps width_px (10 → 32) and returns peaks', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'waveform_preview', path: tmpWav, width_px: 10 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.width_px, 32);
      assert.ok(Array.isArray(j.peaks));
      assert.equal(j.peaks.length, 32);
    });

    it('waveform_preview clamps width_px max (8192)', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'waveform_preview', path: tmpWav, width_px: 99999 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.width_px, 8192);
      assert.equal(j.peaks.length, 8192);
    });

    it('spectrogram_preview returns rows grid on minimal WAV', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWav, width_px: 32, height_px: 32, fft_order: 8 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.ok(Array.isArray(j.rows));
      assert.equal(j.rows.length, 32);
      assert.ok(Array.isArray(j.rows[0]));
      assert.equal(j.rows[0].length, 32);
      assert.equal(j.width_px, 32);
      assert.equal(j.height_px, 32);
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
