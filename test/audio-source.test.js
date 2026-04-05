/**
 * Loads utils + multi-filter + audio.js — format CSS classes and row HTML contract
 * (same vm pattern as daw-source / pdf-source).
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');
const { createTextDiv } = require('./frontend-vm-harness.js');

function loadAudioSandbox() {
  const sandbox = {
    console,
    performance: { now: () => 0 },
    KVR_MANUFACTURER_MAP: {},
    prefs: {
      getObject: () => null,
      getItem: () => null,
      setItem: () => {},
      removeItem: () => {},
    },
    batchSelected: new Set(),
    showToast: () => {},
    toastFmt: (k) => k,
    appFmt: (k) => k,
    showGlobalProgress: () => {},
    hideGlobalProgress: () => {},
    stopBackgroundAnalysis: () => {},
    initColumnResize: () => {},
    initTableColumnReorder: () => {},
    reorderNewTableRows: () => {},
    saveSortState: () => {},
    updateAudioDiskUsage: () => {},
    rowBadges: () => '',
    Audio: function AudioMock() {
      return {
        paused: true,
        duration: 0,
        currentTime: 0,
        play: async () => {},
        pause: () => {},
        addEventListener: () => {},
      };
    },
    MutationObserver: class {
      constructor() {}
      observe() {}
      disconnect() {}
    },
    requestAnimationFrame: (cb) => {
      if (typeof cb === 'function') cb();
      return 0;
    },
    cancelAnimationFrame: () => {},
    Float32Array,
    Float64Array,
    Uint8Array,
    Int32Array,
    document: {
      createElement(tag) {
        if (tag === 'div') return createTextDiv();
        return {
          style: {},
          classList: { add: () => {}, remove: () => {}, contains: () => false, toggle: () => {} },
          appendChild: () => {},
          setAttribute: () => {},
          addEventListener: () => {},
          getContext: () => ({
            clearRect: () => {},
            beginPath: () => {},
            moveTo: () => {},
            lineTo: () => {},
            stroke: () => {},
            fill: () => {},
            fillRect: () => {},
            fillText: () => {},
            createLinearGradient: () => ({ addColorStop: () => {} }),
          }),
          getBoundingClientRect: () => ({ left: 0, top: 0, width: 800, height: 120 }),
        };
      },
      getElementById(id) {
        if (id === 'audioNowPlaying') {
          return {
            style: {},
            classList: { add: () => {}, remove: () => {}, contains: () => false, toggle: () => {} },
            addEventListener: () => {},
            querySelector: () => null,
          };
        }
        return null;
      },
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
      body: { insertAdjacentHTML: () => {} },
    },
    window: {},
  };
  sandbox.window = sandbox;
  sandbox.window.innerWidth = 1280;
  sandbox.window.innerHeight = 800;
  sandbox.window.vstUpdater = {
    stopAudioScan: async () => {},
    scanAudioSamples: async () => ({ samples: [], roots: [] }),
    saveAudioScan: async () => {},
    onAudioScanProgress: () => () => {},
    dbQueryAudio: async () => ({ samples: [], totalCount: 0, totalUnfiltered: 0 }),
    dbAudioStats: async () => ({ formatCounts: {}, totalBytes: 0, sampleCount: 0 }),
    findSimilarSamples: async () => [],
  };

  vm.createContext(sandbox);
  const root = path.join(__dirname, '..', 'frontend', 'js');
  for (const rel of ['utils.js', 'multi-filter.js', 'audio.js']) {
    vm.runInContext(fs.readFileSync(path.join(root, rel), 'utf8'), sandbox);
  }
  return sandbox;
}

describe('frontend/js/audio.js (vm-loaded)', () => {
  let A;

  before(() => {
    A = loadAudioSandbox();
  });

  it('getFormatClass maps known extensions (case-insensitive) and defaults', () => {
    assert.strictEqual(A.getFormatClass('WAV'), 'format-wav');
    assert.strictEqual(A.getFormatClass('mp3'), 'format-mp3');
    assert.strictEqual(A.getFormatClass('AIFF'), 'format-aiff');
    assert.strictEqual(A.getFormatClass('aif'), 'format-aif');
    assert.strictEqual(A.getFormatClass('flac'), 'format-flac');
    assert.strictEqual(A.getFormatClass('ogg'), 'format-ogg');
    assert.strictEqual(A.getFormatClass('m4a'), 'format-m4a');
    assert.strictEqual(A.getFormatClass('aac'), 'format-aac');
    assert.strictEqual(A.getFormatClass('weird'), 'format-default');
  });

  it('buildAudioRow includes format badge class, escaped path, and channel shorthand', () => {
    vm.runInContext('audioPlayerPath = null; _lastAudioSearch = ""; _lastAudioMode = "fuzzy";', A);
    const html = A.buildAudioRow({
      name: 'Kick & snare.wav',
      path: '/sounds/Kick & snare.wav',
      format: 'WAV',
      sizeFormatted: '100 KB',
      modified: '2024-01-01',
      directory: '/sounds',
      channels: 2,
      duration: 65.3,
      bpm: 120,
      key: 'C',
      lufs: -14,
    });
    assert.ok(html.includes('format-wav'));
    assert.ok(html.includes('data-audio-path'));
    assert.ok(html.includes('&amp;'), 'path attribute should escape ampersands');
    assert.ok(html.includes('>S<'), 'stereo shorthand');
    assert.ok(html.includes('col-lufs'), 'lufs column present');
  });

  it('buildAudioRow marks playing row and loop button when path matches player', () => {
    vm.runInContext('audioPlayerPath = "/x/a.wav"; audioLooping = true; _lastAudioSearch = "";', A);
    const html = A.buildAudioRow({
      name: 'A',
      path: '/x/a.wav',
      format: 'WAV',
      sizeFormatted: '1 B',
      modified: 'd',
      directory: '/x',
    });
    assert.ok(html.includes('row-playing'));
    assert.ok(html.includes('btn-loop') && html.includes('active'));
  });

  it('buildAudioRow shows mono vs multi-channel shorthand in col-ch', () => {
    vm.runInContext('audioPlayerPath = null; _lastAudioSearch = ""; _lastAudioMode = "fuzzy";', A);
    const mono = A.buildAudioRow({
      name: 'M',
      path: '/m.wav',
      format: 'WAV',
      sizeFormatted: '1 B',
      modified: 'd',
      directory: '/',
      channels: 1,
    });
    assert.ok(mono.includes('class="col-ch"') && mono.includes('>M</td>'));
    const six = A.buildAudioRow({
      name: 'S',
      path: '/s.wav',
      format: 'WAV',
      sizeFormatted: '1 B',
      modified: 'd',
      directory: '/',
      channels: 6,
    });
    assert.ok(six.includes('>6ch</td>'));
  });
});
