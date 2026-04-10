/**
 * Real shortcuts.js: prefs merge in getShortcuts, saveShortcuts shape, formatKey display.
 */
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function prefsStore() {
  return {
    _cache: {},
    getObject(key, fallback) {
      const v = this._cache[key];
      return v === undefined || v === null ? fallback : v;
    },
    setItem(key, value) {
      this._cache[key] = value;
    },
    removeItem(key) {
      delete this._cache[key];
    },
  };
}

function loadShortcutsSandbox(platform) {
  return loadFrontendScripts(['utils.js', 'shortcuts.js'], {
    prefs: prefsStore(),
    registerFilter: () => {},
    showToast: () => {},
    toastFmt: (k) => k,
    appFmt: (k) => k,
    navigator: { platform },
    document: {
      ...defaultDocument(),
      getElementById: () => null,
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
      body: {},
    },
  });
}

describe('frontend/js/shortcuts.js (vm-loaded)', () => {
  it('getShortcuts merges customShortcuts over defaults', () => {
    const S = loadShortcutsSandbox('MacIntel');
    S.prefs._cache.customShortcuts = {
      tab1: { key: 'q', mod: false },
    };
    const sc = S.getShortcuts();
    assert.strictEqual(sc.tab1.key, 'q');
    assert.strictEqual(sc.tab1.mod, false);
    assert.strictEqual(sc.tab2.key, '2');
    assert.strictEqual(sc.tab2.mod, true);
  });

  it('saveShortcuts persists key+mod+shift per id', () => {
    const S = loadShortcutsSandbox('MacIntel');
    S.saveShortcuts({
      tab1: { key: 'q', mod: false, label: 'ignored' },
      scanSamplesOnly: { key: 's', mod: true, shift: true, label: 'ignored' },
    });
    const slim = S.prefs._cache.customShortcuts;
    assert.strictEqual(slim.tab1.key, 'q');
    assert.strictEqual(slim.tab1.mod, false);
    assert.strictEqual(slim.tab1.label, undefined);
    assert.strictEqual(slim.tab1.shift, undefined);
    assert.strictEqual(slim.scanSamplesOnly.shift, true);
  });

  it('formatKey uses Cmd glyph on Mac and Ctrl on Windows', () => {
    const mac = loadShortcutsSandbox('MacIntel');
    assert.ok(mac.formatKey({ key: 'k', mod: true }).includes('\u2318'));
    const win = loadShortcutsSandbox('Win32');
    assert.ok(win.formatKey({ key: 'k', mod: true }).includes('Ctrl'));
  });

  it('formatKey maps Space and arrow keys to readable labels', () => {
    const S = loadShortcutsSandbox('MacIntel');
    assert.strictEqual(S.formatKey({ key: ' ', mod: false }), 'Space');
    assert.strictEqual(S.formatKey({ key: 'ArrowLeft', mod: false }).includes('\u2190'), true);
  });

  it('formatKey maps Escape and all arrow directions', () => {
    const S = loadShortcutsSandbox('MacIntel');
    assert.strictEqual(S.formatKey({ key: 'Escape', mod: false }), 'Esc');
    assert.strictEqual(S.formatKey({ key: 'ArrowRight', mod: false }), '\u2192');
    assert.strictEqual(S.formatKey({ key: 'ArrowUp', mod: false }), '\u2191');
    assert.strictEqual(S.formatKey({ key: 'ArrowDown', mod: false }), '\u2193');
  });

  it('formatKey uppercases single letter keys', () => {
    const S = loadShortcutsSandbox('Win32');
    assert.strictEqual(S.formatKey({ key: 'a', mod: false }), 'A');
  });

  it('formatKey shows shift when shortcut.shift is true', () => {
    const S = loadShortcutsSandbox('MacIntel');
    const fk = S.formatKey({ key: 'm', mod: true, shift: true });
    assert.ok(fk.includes('\u21E7'), 'includes shift glyph');
    assert.ok(fk.includes('M'));
  });

  it('formatKey no mod is only the key part', () => {
    const S = loadShortcutsSandbox('Linux x86_64');
    assert.strictEqual(S.formatKey({ key: 'z', mod: false }), 'Z');
    assert.ok(!S.formatKey({ key: 'z', mod: false }).includes('Ctrl'));
  });

  it('getShortcuts uses defaults when customShortcuts absent', () => {
    const S = loadShortcutsSandbox('MacIntel');
    assert.strictEqual(S.prefs._cache.customShortcuts, undefined);
    const sc = S.getShortcuts();
    assert.strictEqual(sc.tab11.key, 'F3');
    assert.strictEqual(sc.tab11.mod, false);
    assert.strictEqual(sc.search.key, 'f');
    assert.strictEqual(sc.search.mod, true);
  });

  it('formatKey leaves function keys as uppercase token (F3, F4)', () => {
    const S = loadShortcutsSandbox('Win32');
    assert.strictEqual(S.formatKey({ key: 'F3', mod: false }), 'F3');
    assert.strictEqual(S.formatKey({ key: 'F4', mod: true }), 'Ctrl+F4');
  });

  it('capture keydown Space calls toggleAudioPlayback (e.key Space string)', () => {
    let keydownCapture;
    let playPauseCalls = 0;
    loadFrontendScripts(['utils.js', 'shortcuts.js'], {
      prefs: prefsStore(),
      registerFilter: () => {},
      showToast: () => {},
      toastFmt: (k) => k,
      appFmt: (k) => k,
      navigator: { platform: 'MacIntel' },
      toggleAudioPlayback: () => { playPauseCalls++; },
      document: {
        ...defaultDocument(),
        getElementById: () => null,
        querySelector: () => null,
        querySelectorAll: () => [],
        addEventListener(type, fn, cap) {
          if (type === 'keydown' && cap === true) keydownCapture = fn;
        },
        body: {},
      },
    });
    assert.ok(typeof keydownCapture === 'function', 'keydown capture handler registered');
    keydownCapture({
      key: 'Space',
      code: 'Space',
      metaKey: false,
      ctrlKey: false,
      target: { tagName: 'BODY', isContentEditable: false, closest: () => null },
      preventDefault() {},
      stopPropagation() {},
    });
    assert.strictEqual(playPauseCalls, 1);
  });
});
