/**
 * Loads real utils.js + file-browser.js; fileIcon classification and fav-dir prefs flows.
 */
const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

function loadFileBrowserSandbox() {
  return loadFrontendScripts(['utils.js', 'file-browser.js'], {
    prefs: {
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
    },
    showToast: () => {},
    toastFmt: (k, vars) => (vars ? `${k}:${JSON.stringify(vars)}` : k),
    appFmt: (k) => k,
  });
}

describe('frontend/js/file-browser.js (vm-loaded)', () => {
  let F;

  beforeEach(() => {
    F = loadFileBrowserSandbox();
  });

  it('fileIcon maps directories to folder glyph', () => {
    assert.ok(F.fileIcon({ isDir: true, ext: '' }).includes('128193'));
  });

  it('fileIcon maps audio extensions', () => {
    assert.ok(F.fileIcon({ isDir: false, ext: 'wav' }).includes('127925'));
    assert.ok(F.fileIcon({ isDir: false, ext: 'flac' }).includes('127925'));
  });

  it('fileIcon maps DAW project extensions', () => {
    assert.ok(F.fileIcon({ isDir: false, ext: 'als' }).includes('127911'));
    assert.ok(F.fileIcon({ isDir: false, ext: 'rpp' }).includes('127911'));
  });

  it('fileIcon maps plugin bundle extensions', () => {
    assert.ok(F.fileIcon({ isDir: false, ext: 'vst3' }).includes('9889'));
    assert.ok(F.fileIcon({ isDir: false, ext: 'component' }).includes('9889'));
  });

  it('fileIcon maps images, docs, archives, and default', () => {
    assert.ok(F.fileIcon({ isDir: false, ext: 'png' }).includes('128247'));
    assert.ok(F.fileIcon({ isDir: false, ext: 'json' }).includes('128203'));
    assert.ok(F.fileIcon({ isDir: false, ext: 'zip' }).includes('128230'));
    assert.ok(F.fileIcon({ isDir: false, ext: 'unknownext' }).includes('128196'));
  });

  it('addFavDir / removeFavDir / isFavDir persist under prefs.favDirs', () => {
    assert.strictEqual(F.getFavDirs().length, 0);
    F.addFavDir('/Users/me/Projects/beats');
    const dirs = F.getFavDirs();
    assert.strictEqual(dirs.length, 1);
    assert.strictEqual(dirs[0].path, '/Users/me/Projects/beats');
    assert.strictEqual(dirs[0].name, 'beats');
    assert.strictEqual(F.isFavDir('/Users/me/Projects/beats'), true);
    F.addFavDir('/Users/me/Projects/beats');
    assert.strictEqual(F.getFavDirs().length, 1);
    F.removeFavDir('/Users/me/Projects/beats');
    assert.strictEqual(F.getFavDirs().length, 0);
    assert.strictEqual(F.isFavDir('/Users/me/Projects/beats'), false);
  });
});
