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

  it('returns empty pcts array when stored as [] (truthy)', () => {
    const C = loadColumnsSandbox({
      columnWidths: {
        pluginTable: { v: 3, keys: ['a'], pcts: [] },
      },
    });
    const pcts = C.loadColumnWidths('pluginTable');
    assert.ok(Array.isArray(pcts));
    assert.strictEqual(pcts.length, 0);
  });
});

describe('frontend/js/columns.js saveColumnWidths (vm-loaded)', () => {
  it('persists keys and percentage widths from table headers', () => {
    const cache = {};
    const ths = [
      { dataset: { key: 'name' }, offsetWidth: 600, className: '' },
      { dataset: { key: 'ver' }, offsetWidth: 400, className: '' },
    ];
    const table = {
      id: 'audioTable',
      offsetWidth: 1000,
      querySelectorAll(sel) {
        return sel === 'thead th' ? ths : [];
      },
    };
    const C = loadFrontendScripts(['columns.js'], {
      prefs: {
        _cache: cache,
        getObject(key, fallback) {
          const v = this._cache[key];
          return v === undefined || v === null ? fallback : v;
        },
        setItem(key, value) {
          this._cache[key] = value;
        },
      },
      document: {
        getElementById(id) {
          return id === 'audioTable' ? table : null;
        },
      },
      showToast: () => {},
    });
    C.saveColumnWidths('audioTable');
    const stored = cache.columnWidths.audioTable;
    assert.ok(stored);
    assert.strictEqual(stored.v, 3);
    // VM may wrap arrays; compare elements, not deepStrictEqual on array identity
    assert.strictEqual(stored.keys.length, 2);
    assert.strictEqual(stored.keys[0], 'name');
    assert.strictEqual(stored.keys[1], 'ver');
    assert.strictEqual(stored.pcts[0], 60);
    assert.strictEqual(stored.pcts[1], 40);
  });

  it('saveColumnWidths returns early when table width is 0', () => {
    const cache = {};
    const table = {
      id: 'z',
      offsetWidth: 0,
      querySelectorAll: () => [{ offsetWidth: 10, dataset: { key: 'a' } }],
    };
    const C = loadFrontendScripts(['columns.js'], {
      prefs: {
        _cache: cache,
        getObject: () => ({}),
        setItem: () => {},
      },
      document: { getElementById: () => table },
      showToast: () => {},
    });
    C.saveColumnWidths('z');
    assert.strictEqual(cache.columnWidths, undefined);
  });
});
