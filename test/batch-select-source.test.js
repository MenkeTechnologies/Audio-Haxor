/**
 * Real batch-select.js: getRowPath resolution and toggleBatchSelect driving the batch bar.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function loadBatchSandbox() {
  const bar = { style: { display: 'none' } };
  const countEl = { textContent: '' };
  return loadFrontendScripts(['utils.js', 'batch-select.js'], {
    appFmt: (key, vars) => (vars && vars.n != null ? `${key}:${vars.n}` : key),
    toastFmt: (k) => k,
    showToast: () => {},
    copyToClipboard: () => {},
    document: {
      ...defaultDocument(),
      getElementById(id) {
        if (id === 'batchActionBar') return bar;
        if (id === 'batchSelectionCount') return countEl;
        return null;
      },
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
    },
  });
}

describe('frontend/js/batch-select.js (vm-loaded)', () => {
  let B;
  let bar;
  let countEl;

  before(() => {
    const sandbox = loadBatchSandbox();
    B = sandbox;
    bar = B.document.getElementById('batchActionBar');
    countEl = B.document.getElementById('batchSelectionCount');
  });

  it('getRowPath prefers audio, daw, preset, then midi dataset keys', () => {
    assert.strictEqual(B.getRowPath({ dataset: { audioPath: '/a.wav' } }), '/a.wav');
    assert.strictEqual(B.getRowPath({ dataset: { dawPath: '/p.als' } }), '/p.als');
    assert.strictEqual(B.getRowPath({ dataset: { presetPath: '/x.fxp' } }), '/x.fxp');
    assert.strictEqual(B.getRowPath({ dataset: { midiPath: '/m.mid' } }), '/m.mid');
    assert.strictEqual(B.getRowPath({ dataset: {} }), null);
    assert.strictEqual(B.getRowPath(null), null);
  });

  it('toggleBatchSelect shows bar and updates count when selecting', () => {
    B.deselectAll();
    B.toggleBatchSelect('/one.wav', true);
    assert.strictEqual(bar.style.display, 'flex');
    assert.ok(countEl.textContent.includes('1'));
    B.toggleBatchSelect('/one.wav', false);
    assert.strictEqual(bar.style.display, 'none');
  });
});
