/**
 * Real batch-select.js: getRowPath resolution and toggleBatchSelect driving the batch bar.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function loadBatchSandbox() {
  const bar = { style: { display: 'none' } };
  const countEl = { textContent: '' };
  const activeTab = {
    id: 'tabSamples',
    classList: { contains: (c) => c === 'active' },
  };
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
      querySelector(sel) {
        if (sel === '.tab-content.active') return activeTab;
        return null;
      },
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

describe('frontend/js/batch-select.js selectAllVisible (vm-loaded)', () => {
  it('checks visible batch-cbs and adds each row path to selection', () => {
    const bar = { style: { display: 'none' } };
    const countEl = { textContent: '' };
    const trA = { dataset: { audioPath: '/a.wav' } };
    const trB = { dataset: { audioPath: '/b.wav' } };
    const cbA = { checked: false, closest: (sel) => (sel === 'tr' ? trA : null) };
    const cbB = { checked: false, closest: (sel) => (sel === 'tr' ? trB : null) };
    const cbs = [cbA, cbB];
    const headerCb = { checked: false };
    const table = {
      querySelectorAll: () => [],
      querySelector: (sel) => (sel === '.batch-cb-all' ? headerCb : null),
    };
    const tbody = {
      querySelectorAll(sel) {
        return sel === '.batch-cb' ? cbs : [];
      },
      closest: (sel) => (sel === 'table' ? table : null),
    };
    const activeTab = {
      id: 'tabSamples',
      classList: { contains: (c) => c === 'active' },
    };
    const B = loadFrontendScripts(['utils.js', 'batch-select.js'], {
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
        querySelector(sel) {
          if (sel === '.tab-content.active tbody') return tbody;
          if (sel === '.tab-content.active') return activeTab;
          return null;
        },
        querySelectorAll(sel) {
          if (sel === '.batch-cb') return cbs;
          if (sel === '.batch-cb-all') return [];
          return [];
        },
        addEventListener: () => {},
      },
    });
    B.deselectAll();
    B.selectAllVisible();
    assert.strictEqual(cbA.checked, true);
    assert.strictEqual(cbB.checked, true);
    assert.strictEqual(bar.style.display, 'flex');
    assert.ok(countEl.textContent.includes('2'));
  });
});
