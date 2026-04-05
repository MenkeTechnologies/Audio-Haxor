/**
 * Real midi.js: sortMidiArray orders filteredMidi by midiSortKey / midiSortAsc.
 */
const vm = require('node:vm');
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function prefsStore() {
  return {
    _cache: {},
    getItem(key) {
      const v = this._cache[key];
      return v === undefined ? null : v;
    },
    setItem(key, value) {
      this._cache[key] = value;
    },
  };
}

function loadMidiSortSandbox() {
  return loadFrontendScripts(['utils.js', 'sort-persist.js', 'midi.js'], {
    document: {
      ...defaultDocument(),
      getElementById: () => null,
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
    },
    prefs: prefsStore(),
    showToast: () => {},
    toastFmt: (k) => k,
    saveSortState: () => {},
    registerFilter: () => {},
    applyFilter: () => {},
    window: { vstUpdater: { getLatestPresetScan: () => Promise.resolve(null) } },
  });
}

function names(M) {
  return vm.runInContext(`filteredMidi.map((x) => x.name).join(',')`, M);
}

describe('frontend/js/midi.js sortMidiArray (vm-loaded)', () => {
  let M;

  before(() => {
    M = loadMidiSortSandbox();
  });

  it('sorts by name ascending case-insensitively', () => {
    vm.runInContext(
      `
      filteredMidi = [
        { name: 'Zed', path: '/z', directory: '/d', size: 1 },
        { name: 'alpha', path: '/a', directory: '/d', size: 2 },
      ];
      midiSortKey = 'name';
      midiSortAsc = true;
      sortMidiArray();
    `,
      M,
    );
    assert.strictEqual(names(M), 'alpha,Zed');
  });

  it('sorts by size descending', () => {
    vm.runInContext(
      `
      filteredMidi = [
        { name: 'a', path: '/1', directory: '/d', size: 10 },
        { name: 'b', path: '/2', directory: '/d', size: 1000 },
      ];
      midiSortKey = 'size';
      midiSortAsc = false;
      sortMidiArray();
    `,
      M,
    );
    assert.strictEqual(names(M), 'b,a');
  });

  it('sorts by path uses directory (midi path column compares parent folder)', () => {
    vm.runInContext(
      `
      filteredMidi = [
        { name: 'z', path: '/z.mid', directory: '/folder-b', size: 1 },
        { name: 'a', path: '/a.mid', directory: '/folder-a', size: 1 },
      ];
      midiSortKey = 'path';
      midiSortAsc = true;
      sortMidiArray();
    `,
      M,
    );
    assert.strictEqual(names(M), 'a,z');
  });
});
