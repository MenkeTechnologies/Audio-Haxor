/**
 * Loads utils + batch-select + presets.js; formatPresetSize, row HTML, incremental stats (MIDI excluded).
 */
const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert/strict');
const vm = require('vm');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

describe('frontend/js/presets.js (vm-loaded)', () => {
  let P;

  beforeEach(() => {
    P = loadFrontendScripts(['utils.js', 'batch-select.js', 'presets.js'], {
      batchSelected: new Set(),
      _lastPresetSearch: '',
      _lastPresetMode: 'fuzzy',
      rowBadges: () => '',
    });
  });

  it('formatPresetSize matches human-readable byte ladder', () => {
    assert.strictEqual(P.formatPresetSize(0), '0 B');
    assert.match(P.formatPresetSize(1536), /^1\.5 KB$/);
  });

  it('buildPresetRow includes path, format, and checkbox wiring', () => {
    const html = P.buildPresetRow({
      path: '/Presets/Synth/Bass.h2p',
      name: 'Fat Bass',
      format: 'H2P',
      directory: '/Presets/Synth',
      size: 2048,
      sizeFormatted: '2.0 KB',
      modified: '2024-01-02',
    });
    assert.ok(html.includes('data-preset-path'));
    assert.ok(html.includes('data-preset-format'));
    assert.ok(html.includes('openPresetFolder'));
    assert.ok(html.includes('Fat Bass'));
  });

  it('accumulatePresetStats skips MIDI formats but counts bytes for others', () => {
    vm.runInContext(
      `resetPresetStatsAccumulators();
      accumulatePresetStats([
        { format: 'FXP', size: 100 },
        { format: 'MID', size: 99999 },
        { format: 'H2P', size: 50 },
      ]);`,
      P
    );
    const counts = JSON.parse(vm.runInContext('JSON.stringify(_presetStatsFormatCounts)', P));
    const bytes = vm.runInContext('_presetStatsTotalBytes', P);
    assert.deepStrictEqual(counts, { FXP: 1, H2P: 1 });
    assert.strictEqual(bytes, 150);
  });
});
