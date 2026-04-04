/**
 * Real columns.js: loadColumnWidths migration and version gate (saved layout compat).
 */
const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

function loadColumnsSandbox(prefsCache) {
  return loadFrontendScripts(['columns.js'], {
    prefs: {
      _cache: prefsCache,
      getObject(key, fallback) {
        const v = this._cache[key];
        if (v === undefined || v === null) return fallback;
        return v;
      },
      setItem(key, value) {
        this._cache[key] = value;
      },
    },
    showToast: () => {},
  });
}

describe('frontend/js/columns.js loadColumnWidths (vm-loaded)', () => {
  it('returns null when table id is absent', () => {
    const C = loadColumnsSandbox({ columnWidths: {} });
    assert.strictEqual(C.loadColumnWidths('missing'), null);
  });

  it('returns null for legacy plain-array format', () => {
    const C = loadColumnsSandbox({
      columnWidths: { pluginTable: [10, 20, 30] },
    });
    assert.strictEqual(C.loadColumnWidths('pluginTable'), null);
  });

  it('returns null when layout version mismatches COL_LAYOUT_VERSION', () => {
    const C = loadColumnsSandbox({
      columnWidths: {
        pluginTable: { v: 1, keys: ['a'], pcts: [100] },
      },
    });
    assert.strictEqual(C.loadColumnWidths('pluginTable'), null);
  });

  it('returns pcts when version and shape match', () => {
    // v must match COL_LAYOUT_VERSION in frontend/js/columns.js (currently 3)
    const C = loadColumnsSandbox({
      columnWidths: {
        pluginTable: { v: 3, keys: ['n', 'v'], pcts: [62.5, 37.5] },
      },
    });
    const pcts = C.loadColumnWidths('pluginTable');
    assert.ok(Array.isArray(pcts));
    assert.strictEqual(pcts.join(','), '62.5,37.5');
  });
});
