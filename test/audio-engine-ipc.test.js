/**
 * Integration tests: spawn built `audio-engine`, one JSON line per stdin line, assert stdout.
 * Covers the full stdin JSON command surface (all `cmd` values), previews, device I/O with
 * graceful handling when no hardware is present, insert-chain validation, and plugin cache.
 * Requires `audio-engine-artifacts/<debug|release>/audio-engine` (or legacy `target/…`, or `AUDIO_ENGINE_TEST_BIN`).
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
  const aeDebug = path.join(root, 'audio-engine-artifacts', 'debug', `audio-engine${ext}`);
  const aeRelease = path.join(root, 'audio-engine-artifacts', 'release', `audio-engine${ext}`);
  const debug = path.join(root, 'target', 'debug', `audio-engine${ext}`);
  const release = path.join(root, 'target', 'release', `audio-engine${ext}`);
  if (fs.existsSync(aeDebug)) {
    return aeDebug;
  }
  if (fs.existsSync(aeRelease)) {
    return aeRelease;
  }
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
    /* Arm the C++ `ParentWatchdog` (see `audio-engine/src/ParentWatchdog.cpp`) so the spawned
     * audio-engine self-destructs within ~2 s if this test runner dies (Ctrl+C, crash, timeout).
     * Without this, test-spawned engines linger forever when the node process exits abnormally,
     * and they collide with `pnpm tauri dev`'s audio-engine on the CoreAudio output device,
     * which manifests as continuous audio crackling in the main app. */
    const child = spawn(bin, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, AUDIO_HAXOR_PARENT_PID: String(process.pid) },
    });
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

/** Stereo PCM 16-bit LE WAV (silence), interleaved L/R. */
function pcm16WavStereo(sampleRate, numFrames) {
  const bitsPerSample = 16;
  const numChannels = 2;
  const blockAlign = numChannels * (bitsPerSample / 8);
  const byteRate = sampleRate * blockAlign;
  const dataSize = numFrames * blockAlign;
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
  let off = 44;
  for (let i = 0; i < numFrames; i += 1) {
    buf.writeInt16LE(0, off);
    buf.writeInt16LE(0, off + 2);
    off += 4;
  }
  return buf;
}

/**
 * @param {{ ok?: boolean, error?: string }} j
 * @param {'output' | 'input'} kind
 */
function assertOkOrNoAudioDevice(j, kind) {
  if (j.ok) {
    return;
  }
  const re = kind === 'output' ? /no output device/i : /no input device/i;
  assert.match(String(j.error || ''), re);
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
    /** Very short PCM WAV (4 mono samples) — spectrogram path needs ≥8 samples. */
    let tmpWavTiny;
    /** Stereo PCM WAV for multi-channel decode checks. */
    let tmpWavStereo;

    before(() => {
      tmpEmptyFile = path.join(os.tmpdir(), `audio-haxor-ipc-empty-${process.pid}.bin`);
      fs.writeFileSync(tmpEmptyFile, Buffer.alloc(0));
      tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'audio-haxor-ipc-dir-'));
      tmpWav = path.join(os.tmpdir(), `audio-haxor-ipc-${process.pid}.wav`);
      fs.writeFileSync(tmpWav, pcm16WavMono(44100, 8192));
      tmpWavTiny = path.join(os.tmpdir(), `audio-haxor-ipc-tiny-${process.pid}.wav`);
      fs.writeFileSync(tmpWavTiny, pcm16WavMono(44100, 4));
      tmpWavStereo = path.join(os.tmpdir(), `audio-haxor-ipc-stereo-${process.pid}.wav`);
      fs.writeFileSync(tmpWavStereo, pcm16WavStereo(44100, 2048));
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
      try {
        fs.unlinkSync(tmpWavTiny);
      } catch {
        /* ignore */
      }
      try {
        fs.unlinkSync(tmpWavStereo);
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

    it('engine_state returns ok, version, host, stream and input_stream objects', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'engine_state' })], {
        timeoutMs: 90_000,
      });
      assert.equal(outLines.length, 1);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.host, 'juce');
      assert.equal(j.version, cmakeVersion);
      assert.ok(j.stream && typeof j.stream === 'object');
      assert.equal(j.stream.ok, true);
      assert.ok('running' in j.stream);
      assert.ok(j.input_stream && typeof j.input_stream === 'object');
      assert.equal(j.input_stream.ok, true);
      assert.ok('running' in j.input_stream);
    });

    it('unknown cmd returns ok:false and unknown cmd error', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'not_a_real_command_xyz' })]);
      assert.equal(outLines.length, 1);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown cmd/i);
    });

    it('empty object (no cmd) returns unknown cmd error', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({})], { timeoutMs: 90_000 });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown cmd/i);
    });

    it('empty cmd string returns unknown cmd error', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: '' })], { timeoutMs: 90_000 });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown cmd/i);
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

    it('playback_set_loop returns ok (no active playback required)', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_loop', loop: true })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.loop, true);
    });

    it('playback_set_loop with loop false echoes loop false', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_loop', loop: false })]);
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.loop, false);
    });

    it('playback_status returns ok with loaded false when no file loaded', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_status' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.loaded, false);
    });

    it('playback_seek without active player returns no active player', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_seek', position_sec: 1 })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /no active player/i);
    });

    it('playback_pause returns ok and paused (no playback required)', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_pause' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.paused, true);
    });

    it('playback_stop returns ok when nothing was playing', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_stop' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
    });

    it('playback_set_dsp returns ok without loaded file', async () => {
      const { outLines } = await runEngineExchange(bin, [
        jl({ cmd: 'playback_set_dsp', gain: 0.5, pan: -0.25, eq_low_db: 1, eq_mid_db: -2, eq_high_db: 0 }),
      ], { timeoutMs: 90_000 });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
    });

    it('output_stream_status returns ok with running false when stream not started', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'output_stream_status' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.running, false);
    });

    it('input_stream_status returns ok with running false when stream not started', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'input_stream_status' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.running, false);
    });

    it('set_output_device rejects empty device_id', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'set_output_device', device_id: '' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /device_id required/i);
    });

    it('set_input_device rejects empty device_id', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'set_input_device', device_id: '' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /device_id required/i);
    });

    it('start_output_stream with start_playback true rejects when no playback_load', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'start_output_stream', start_playback: true })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /playback_load required before start_playback/i);
    });

    it('waveform_preview omits width_px defaults to 800 peaks', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'waveform_preview', path: tmpWav })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.width_px, 800);
      assert.ok(Array.isArray(j.peaks));
      assert.equal(j.peaks.length, 800);
    });

    it('waveform_preview start_sec at file end yields empty segment', async () => {
      const { outLines: first } = await runEngineExchange(bin, [jl({ cmd: 'waveform_preview', path: tmpWav })], {
        timeoutMs: 90_000,
      });
      const meta = JSON.parse(first[0]);
      assert.equal(meta.ok, true);
      const fileDur = meta.file_duration_sec;
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'waveform_preview', path: tmpWav, start_sec: fileDur })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /empty segment/i);
    });

    it('spectrogram_preview clamps width_px below minimum (4 → 16)', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWav, width_px: 4, height_px: 32, fft_order: 8 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.width_px, 16);
      assert.equal(j.height_px, 32);
      assert.ok(Array.isArray(j.rows));
      assert.equal(j.rows.length, 32);
    });

    it('spectrogram_preview rejects segment too short on tiny WAV', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWavTiny, width_px: 16, height_px: 16, fft_order: 8 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /segment too short for spectrogram/i);
    });

    it('spectrogram_preview clamps height_px below minimum (4 → 16)', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWav, width_px: 32, height_px: 4, fft_order: 8 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.height_px, 16);
      assert.equal(j.rows.length, 16);
    });

    it('spectrogram_preview clamps fft_order in request and shrinks FFT to fit segment', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWav, width_px: 16, height_px: 16, fft_order: 20 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      // clampInt(..., 8, 15) caps fft_order; then fft_size is reduced to ≤ numSamples (8192 mono samples here).
      assert.equal(j.fft_size, 8192);
    });

    it('spectrogram_preview clamps width_px above maximum (9999 → 512)', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWav, width_px: 9999, height_px: 32, fft_order: 8 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.width_px, 512);
      assert.equal(j.rows[0].length, 512);
    });

    it('spectrogram_preview clamps height_px above maximum (9999 → 512)', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWav, width_px: 32, height_px: 9999, fft_order: 8 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.height_px, 512);
      assert.equal(j.rows.length, 512);
    });

    it('waveform_preview succeeds on very short WAV (4 samples)', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'waveform_preview', path: tmpWavTiny, width_px: 32 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.width_px, 32);
      assert.equal(j.peaks.length, 32);
    });

    it('playback_set_speed clamps below minimum to 0.25', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_speed', speed: 0.01 })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.speed, 0.25);
    });

    it('playback_set_speed clamps above maximum to 2', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_speed', speed: 9 })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.speed, 2);
    });

    it('playback_set_reverse true echoes reverse', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_reverse', reverse: true })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.reverse, true);
    });

    it('playback_set_reverse false echoes reverse false', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_reverse', reverse: false })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.reverse, false);
    });

    it('playback_pause with paused false returns ok', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_pause', paused: false })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.paused, false);
    });

    it('set_output_tone fails when no output stream', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'set_output_tone', tone: true })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /no output stream/i);
    });

    it('stop_output_stream returns was_running false when idle', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'stop_output_stream' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.was_running, false);
    });

    it('stop_input_stream returns was_running false when idle', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'stop_input_stream' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.was_running, false);
    });

    it('set_audio_device_type rejects empty type', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'set_audio_device_type', type: '' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /type required/i);
    });

    it('get_output_device_info rejects unknown device_id', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'get_output_device_info', device_id: '__ipc_test_no_such_output__' })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown device_id/i);
    });

    it('get_input_device_info rejects unknown device_id', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'get_input_device_info', device_id: '__ipc_test_no_such_input__' })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown device_id/i);
    });

    it('set_output_device rejects unknown device_id', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'set_output_device', device_id: '__ipc_test_unknown_output__' })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown device_id/i);
    });

    it('set_input_device rejects unknown device_id', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'set_input_device', device_id: '__ipc_test_unknown_input__' })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown device_id/i);
    });

    it('plugin_rescan returns ok (cache wipe, phase reset)', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'plugin_rescan' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
    });

    it('playback_set_inserts with empty paths array clears chain', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_inserts', paths: [] })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.ok(Array.isArray(j.insert_paths));
      assert.equal(j.insert_paths.length, 0);
    });

    it('playback_set_inserts rejects omitted paths', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_inserts' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /paths must be an array/i);
    });

    it('playback_set_inserts rejects non-array paths', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'playback_set_inserts', paths: 'not-an-array' })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /paths must be an array/i);
    });

    it('list_output_devices returns ok with devices array', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'list_output_devices' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.ok(Array.isArray(j.devices));
    });

    it('list_input_devices returns ok with devices array', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'list_input_devices' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.ok(Array.isArray(j.devices));
    });

    it('get_output_device_info with empty device_id falls back to first device', async (t) => {
      const listOut = await runEngineExchange(bin, [jl({ cmd: 'list_output_devices' })], {
        timeoutMs: 90_000,
      });
      const listed = JSON.parse(listOut.outLines[0]);
      if (!listed.ok || !Array.isArray(listed.devices) || listed.devices.length === 0) {
        t.skip('no output devices enumerated (e.g. headless CI)');
        return;
      }
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'get_output_device_info', device_id: '' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      if (!j.ok && /no output device/i.test(String(j.error || ''))) {
        t.skip('output device(s) listed but probe could not open (e.g. headless Windows CI)');
        return;
      }
      assert.equal(j.ok, true);
      assert.ok(typeof j.device_name === 'string' && j.device_name.length > 0);
      assert.ok(typeof j.sample_rate_hz === 'number');
    });

    it('get_input_device_info with empty device_id falls back to first device', async (t) => {
      const listOut = await runEngineExchange(bin, [jl({ cmd: 'list_input_devices' })], {
        timeoutMs: 90_000,
      });
      const listed = JSON.parse(listOut.outLines[0]);
      if (!listed.ok || !Array.isArray(listed.devices) || listed.devices.length === 0) {
        t.skip('no input devices enumerated (e.g. headless CI)');
        return;
      }
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'get_input_device_info', device_id: '' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      if (!j.ok && /no input device/i.test(String(j.error || ''))) {
        t.skip('input device(s) listed but probe could not open (e.g. headless Windows CI)');
        return;
      }
      assert.equal(j.ok, true);
      assert.ok(typeof j.device_name === 'string' && j.device_name.length > 0);
      assert.ok(typeof j.sample_rate_hz === 'number');
    });

    it('plugin_rescan accepts timeout_sec in valid range', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'plugin_rescan', timeout_sec: 120 })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
    });

    it('spectrogram_preview clamps fft_order below minimum (4 → 8)', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWav, width_px: 16, height_px: 16, fft_order: 4 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.fft_size, 256);
    });

    it('playback_load then playback_status in one session reports loaded', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'playback_load', path: tmpWav }), jl({ cmd: 'playback_status' })],
        { timeoutMs: 90_000 },
      );
      assert.equal(outLines.length, 2);
      const load = JSON.parse(outLines[0]);
      const st = JSON.parse(outLines[1]);
      assert.equal(load.ok, true);
      assert.equal(typeof load.duration_sec, 'number');
      assert.equal(st.ok, true);
      assert.equal(st.loaded, true);
      assert.ok(Math.abs(st.duration_sec - load.duration_sec) < 1e-3);
    });

    it('waveform_preview duration_sec zero yields empty segment', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'waveform_preview', path: tmpWav, start_sec: 0, duration_sec: 0 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /empty segment/i);
    });

    it('waveform_preview peaks contain min and max per column', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'waveform_preview', path: tmpWav, width_px: 32 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      const p0 = j.peaks[0];
      assert.ok(p0 && typeof p0 === 'object');
      assert.ok(typeof p0.min === 'number');
      assert.ok(typeof p0.max === 'number');
    });

    it('waveform_preview reports channels=2 for stereo WAV', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'waveform_preview', path: tmpWavStereo, width_px: 32 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.channels, 2);
    });

    it('spectrogram_preview includes db_min, db_max, hop', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'spectrogram_preview', path: tmpWav, width_px: 16, height_px: 16, fft_order: 8 })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.db_min, -100);
      assert.equal(j.db_max, 0);
      assert.ok(typeof j.hop === 'number' && j.hop >= 1);
    });

    it('playback_set_speed omitted defaults to 1', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_set_speed' })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.speed, 1);
    });

    it('playback_open_insert_editor without chain returns invalid insert slot', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_open_insert_editor', slot: 0 })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /invalid insert slot/i);
    });

    it('playback_close_insert_editor without chain returns invalid insert slot', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'playback_close_insert_editor', slot: 0 })], {
        timeoutMs: 90_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /invalid insert slot/i);
    });

    it('start_output_stream rejects unknown device_id', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'start_output_stream', device_id: '__ipc_bad_out__' })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown device_id/i);
    });

    it('start_input_stream rejects unknown device_id', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'start_input_stream', device_id: '__ipc_bad_in__' })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, false);
      assert.match(String(j.error || ''), /unknown device_id/i);
    });

    it('start_output_stream default device succeeds or reports no output device', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'start_output_stream' })], {
        timeoutMs: 120_000,
      });
      const j = JSON.parse(outLines[0]);
      if (j.ok) {
        assert.ok(typeof j.device_name === 'string');
        assert.equal(typeof j.sample_rate_hz, 'number');
      } else {
        assertOkOrNoAudioDevice(j, 'output');
      }
    });

    it('start_input_stream default device succeeds or reports no input device', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'start_input_stream' })], {
        timeoutMs: 120_000,
      });
      const j = JSON.parse(outLines[0]);
      if (j.ok) {
        assert.ok(typeof j.device_name === 'string');
        assert.equal(typeof j.sample_rate_hz, 'number');
      } else {
        assertOkOrNoAudioDevice(j, 'input');
      }
    });

    it('start_output_stream then stop_output_stream sets was_running when open succeeded', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'start_output_stream' }), jl({ cmd: 'stop_output_stream' })],
        { timeoutMs: 120_000 },
      );
      const start = JSON.parse(outLines[0]);
      const stop = JSON.parse(outLines[1]);
      if (!start.ok) {
        assertOkOrNoAudioDevice(start, 'output');
        return;
      }
      assert.equal(stop.ok, true);
      assert.equal(stop.was_running, true);
    });

    it('start_input_stream then stop_input_stream sets was_running when open succeeded', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'start_input_stream' }), jl({ cmd: 'stop_input_stream' })],
        { timeoutMs: 120_000 },
      );
      const start = JSON.parse(outLines[0]);
      const stop = JSON.parse(outLines[1]);
      if (!start.ok) {
        assertOkOrNoAudioDevice(start, 'input');
        return;
      }
      assert.equal(stop.ok, true);
      assert.equal(stop.was_running, true);
    });

    it('list_audio_device_types returns ok with types and current', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'list_audio_device_types' })], {
        timeoutMs: 120_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.ok(Array.isArray(j.types));
      assert.ok(typeof j.current === 'string');
    });

    it('set_audio_device_type with listed driver name returns ok', async () => {
      const { outLines: first } = await runEngineExchange(bin, [jl({ cmd: 'list_audio_device_types' })], {
        timeoutMs: 120_000,
      });
      const list = JSON.parse(first[0]);
      assert.equal(list.ok, true);
      assert.ok(list.types.length > 0, 'expected at least one audio driver type');
      const typeName = list.types[0];
      assert.ok(typeof typeName === 'string' && typeName.length > 0);
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'set_audio_device_type', type: typeName })],
        { timeoutMs: 90_000 },
      );
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.type, typeName);
    });

    it('playback_load then playback_status includes spectrum metadata keys when loaded', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'playback_load', path: tmpWav }), jl({ cmd: 'playback_status' })],
        { timeoutMs: 90_000 },
      );
      const st = JSON.parse(outLines[1]);
      assert.equal(st.ok, true);
      assert.equal(st.loaded, true);
      assert.ok('spectrum_fft_size' in st);
      assert.ok('spectrum_bins' in st);
      assert.ok('spectrum_sr_hz' in st);
    });

    it('plugin_chain returns ok with api_version 2 (may be slow on cold cache)', async () => {
      const { outLines } = await runEngineExchange(bin, [jl({ cmd: 'plugin_chain' })], {
        timeoutMs: 120_000,
      });
      const j = JSON.parse(outLines[0]);
      assert.equal(j.ok, true);
      assert.equal(j.api_version, 2);
      assert.ok(typeof j.phase === 'string');
    });

    it('engine_state and ping in one session', async () => {
      const { outLines } = await runEngineExchange(
        bin,
        [jl({ cmd: 'engine_state' }), jl({ cmd: 'ping' })],
        { timeoutMs: 90_000 },
      );
      const a = JSON.parse(outLines[0]);
      const b = JSON.parse(outLines[1]);
      assert.equal(a.ok, true);
      assert.equal(a.version, cmakeVersion);
      assert.equal(b.ok, true);
      assert.equal(b.version, cmakeVersion);
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
