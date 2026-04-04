/**
 * Loads real utils.js + batch-select.js; validates getRowPath priority used by batch checkboxes.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

describe('frontend/js/batch-select.js (vm-loaded)', () => {
  let B;

  before(() => {
    B = loadFrontendScripts(['utils.js', 'batch-select.js']);
  });

  it('getRowPath reads audioPath', () => {
    assert.strictEqual(
      B.getRowPath({ dataset: { audioPath: '/samples/kick.wav' } }),
      '/samples/kick.wav'
    );
  });

  it('getRowPath reads dawPath when no audioPath', () => {
    assert.strictEqual(
      B.getRowPath({ dataset: { dawPath: '/p/live.als' } }),
      '/p/live.als'
    );
  });

  it('getRowPath prefers audio over daw when both set (first wins in expression)', () => {
    assert.strictEqual(
      B.getRowPath({
        dataset: { audioPath: '/a.wav', dawPath: '/b.als' },
      }),
      '/a.wav'
    );
  });

  it('getRowPath reads presetPath and midiPath', () => {
    assert.strictEqual(
      B.getRowPath({ dataset: { presetPath: '/presets/x.h2p' } }),
      '/presets/x.h2p'
    );
    assert.strictEqual(
      B.getRowPath({ dataset: { midiPath: '/m/a.mid' } }),
      '/m/a.mid'
    );
  });

  it('getRowPath returns null for missing tr or no path attrs', () => {
    assert.strictEqual(B.getRowPath(null), null);
    assert.strictEqual(B.getRowPath({ dataset: {} }), null);
  });
});
