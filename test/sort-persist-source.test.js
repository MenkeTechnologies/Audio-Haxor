/**
 * Real sort-persist.js: JSON round-trip and restoreAllSortStates wiring to globals + prefs fallback.
 */
const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

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

function loadSortPersistSandbox(extra = {}) {
  return loadFrontendScripts(['sort-persist.js'], {
    prefs: prefsStore(),
    _pluginSortKey: 'name',
    _pluginSortAsc: true,
    audioSortKey: 'name',
    audioSortAsc: true,
    dawSortKey: 'name',
    dawSortAsc: true,
    presetSortKey: 'name',
    presetSortAsc: true,
    midiSortKey: 'name',
    midiSortAsc: true,
    pdfSortKey: 'name',
    pdfSortAsc: true,
    ...extra,
  });
}

describe('frontend/js/sort-persist.js (vm-loaded)', () => {
  let S;

  beforeEach(() => {
    S = loadSortPersistSandbox();
  });

  it('saveSortState and restoreSortState round-trip plugin tab', () => {
    S.saveSortState('plugin', 'vendor', false);
    const r = S.restoreSortState('plugin');
    assert.ok(r);
    assert.strictEqual(r.key, 'vendor');
    assert.strictEqual(r.asc, false);
  });

  it('restoreSortState returns null for missing key', () => {
    assert.strictEqual(S.restoreSortState('plugin'), null);
  });

  it('restoreSortState returns null for invalid JSON', () => {
    S.prefs._cache.sort_plugin = '{broken';
    assert.strictEqual(S.restoreSortState('plugin'), null);
  });

  it('restoreAllSortStates applies saved sorts to tab globals', () => {
    S.prefs._cache.sort_plugin = JSON.stringify({ key: 'size', asc: false });
    S.prefs._cache.sort_audio = JSON.stringify({ key: 'path', asc: true });
    S.prefs._cache.sort_daw = JSON.stringify({ key: 'modified', asc: false });
    S.prefs._cache.sort_preset = JSON.stringify({ key: 'name', asc: true });
    S.prefs._cache.sort_midi = JSON.stringify({ key: 'bpm', asc: false });
    S.prefs._cache.sort_pdf = JSON.stringify({ key: 'pages', asc: true });
    S.restoreAllSortStates();
    assert.strictEqual(S._pluginSortKey, 'size');
    assert.strictEqual(S._pluginSortAsc, false);
    assert.strictEqual(S.audioSortKey, 'path');
    assert.strictEqual(S.audioSortAsc, true);
    assert.strictEqual(S.dawSortKey, 'modified');
    assert.strictEqual(S.dawSortAsc, false);
    assert.strictEqual(S.presetSortKey, 'name');
    assert.strictEqual(S.presetSortAsc, true);
    assert.strictEqual(S.midiSortKey, 'bpm');
    assert.strictEqual(S.midiSortAsc, false);
    assert.strictEqual(S.pdfSortKey, 'pages');
    assert.strictEqual(S.pdfSortAsc, true);
  });

  it('restoreAllSortStates seeds plugin from pluginSort when no runtime sort saved', () => {
    S.prefs._cache.pluginSort = 'vendor-desc';
    S.restoreAllSortStates();
    assert.strictEqual(S._pluginSortKey, 'vendor');
    assert.strictEqual(S._pluginSortAsc, false);
  });

  it('restoreAllSortStates seeds preset/midi/pdf from prefs when no runtime sort saved', () => {
    S.prefs._cache.presetSort = 'size';
    S.prefs._cache.midiSort = 'duration';
    S.prefs._cache.pdfSort = 'directory';
    S.restoreAllSortStates();
    assert.strictEqual(S.presetSortKey, 'size');
    assert.strictEqual(S.presetSortAsc, true);
    assert.strictEqual(S.midiSortKey, 'duration');
    assert.strictEqual(S.midiSortAsc, true);
    assert.strictEqual(S.pdfSortKey, 'directory');
    assert.strictEqual(S.pdfSortAsc, true);
  });

  it('pluginSort without hyphen uses ascending (d undefined)', () => {
    S.prefs._cache.pluginSort = 'name';
    S.restoreAllSortStates();
    assert.strictEqual(S._pluginSortKey, 'name');
    assert.strictEqual(S._pluginSortAsc, true);
  });

  it('pluginSort with hyphenated key preserves first segment only for key', () => {
    S.prefs._cache.pluginSort = 'name-desc-extra';
    S.restoreAllSortStates();
    assert.strictEqual(S._pluginSortKey, 'name');
    assert.strictEqual(S._pluginSortAsc, false);
  });

  it('restoreSortState returns null for empty string stored', () => {
    S.prefs._cache.sort_plugin = '';
    assert.strictEqual(S.restoreSortState('plugin'), null);
  });

  it('initSortPersistence delegates to restoreAllSortStates', () => {
    S.prefs._cache.sort_plugin = JSON.stringify({ key: 'path', asc: true });
    S._pluginSortKey = 'name';
    S._pluginSortAsc = false;
    S.initSortPersistence();
    assert.strictEqual(S._pluginSortKey, 'path');
    assert.strictEqual(S._pluginSortAsc, true);
  });
});
