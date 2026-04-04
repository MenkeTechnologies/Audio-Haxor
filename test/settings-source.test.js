/**
 * Real settings.js: hexToRgba, formatCacheSize, applyColorScheme prefs + CSS vars.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function prefsStore() {
  return {
    _cache: {},
    getObject(key, fallback) {
      const v = this._cache[key];
      if (v === undefined || v === null) return fallback;
      return v;
    },
    setItem(key, value) {
      this._cache[key] = value;
    },
    removeItem(key) {
      delete this._cache[key];
    },
    getItem(key) {
      const v = this._cache[key];
      return v === undefined ? null : v;
    },
  };
}

function loadSettingsSandbox() {
  const rootOps = [];
  const root = {
    removeProperty(key) {
      rootOps.push(['remove', key]);
    },
    setProperty(key, val) {
      rootOps.push(['set', key, val]);
    },
  };
  const doc = {
    ...defaultDocument(),
    documentElement: {
      style: root,
      getAttribute: (name) => (name === 'data-theme' ? 'dark' : null),
    },
  };
  const S = loadFrontendScripts(['utils.js', 'settings.js'], {
    document: doc,
    getComputedStyle: () => ({
      getPropertyValue: () => ' #0a0a14 ',
    }),
    prefs: prefsStore(),
    refreshSettingsUI: () => {},
    showToast: () => {},
    toastFmt: (k) => k,
    appFmt: (k) => k,
  });
  // settings.js defines its own refreshSettingsUI — stub after load so applyColorScheme does not touch DOM.
  S.refreshSettingsUI = () => {};
  return { S, rootOps };
}

describe('frontend/js/settings.js (vm-loaded)', () => {
  let S;
  let rootOps;

  before(() => {
    ({ S, rootOps } = loadSettingsSandbox());
  });

  it('hexToRgba converts #RRGGBB to rgba()', () => {
    assert.strictEqual(S.hexToRgba('#ff0000', 0.5), 'rgba(255, 0, 0, 0.5)');
    assert.strictEqual(S.hexToRgba('#00aabb', 0), 'rgba(0, 170, 187, 0)');
  });

  it('formatCacheSize uses B / KB / MB / GB thresholds', () => {
    assert.strictEqual(S.formatCacheSize(0), '0 B');
    assert.strictEqual(S.formatCacheSize(512), '512 B');
    assert.match(S.formatCacheSize(1536), /1\.5 KB/);
    assert.match(S.formatCacheSize(3 * 1024 * 1024), /3\.0 MB/);
  });

  it('applyColorScheme stores scheme name and applies CSS variables', () => {
    const fresh = loadSettingsSandbox();
    fresh.S.applyColorScheme('cyberpunk');
    assert.strictEqual(fresh.S.prefs._cache.colorScheme, 'cyberpunk');
    assert.ok(!('customSchemeVars' in fresh.S.prefs._cache));
    assert.ok(
      fresh.rootOps.some((op) => op[0] === 'set' && op[1] === '--accent' && op[2].includes('#')),
    );
  });
});
