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

  it('fileIcon uses default doc glyph for .mid (not in AUDIO_EXTS)', () => {
    assert.ok(F.fileIcon({ isDir: false, ext: 'mid' }).includes('128196'));
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

  describe('applyFileSort', () => {
    const entries = [
      { name: 'zebra.wav', ext: 'wav', isDir: false, size: 500, modified: '2024-03-01 10:00' },
      { name: 'apple.wav', ext: 'wav', isDir: false, size: 9999, modified: '2024-01-15 09:00' },
      { name: 'NewFolder', ext: '', isDir: true, size: 0, modified: '2024-05-20 08:00' },
      { name: 'beats.mp3', ext: 'mp3', isDir: false, size: 200, modified: '2024-02-10 12:00' },
      { name: 'old_dir', ext: '', isDir: true, size: 0, modified: '2023-01-01 00:00' },
    ];

    it('folders always come first regardless of name-asc sort', () => {
      F._fileSortKey && (F._fileSortKey = 'name');
      // Direct mutation isn't exposed; instead test the helper directly.
      const sorted = F.applyFileSort(entries);
      assert.ok(sorted[0].isDir, 'first entry must be a folder');
      assert.ok(sorted[1].isDir, 'second entry must be a folder');
      assert.ok(!sorted[2].isDir, 'third entry must be a file');
    });

    it('folders sort alphabetically within the folders group', () => {
      const sorted = F.applyFileSort(entries);
      const folderNames = sorted.filter(e => e.isDir).map(e => e.name);
      assert.deepStrictEqual([...folderNames], ['NewFolder', 'old_dir']);
    });

    it('files sort alphabetically (case-insensitive) by default', () => {
      const sorted = F.applyFileSort(entries);
      const fileNames = sorted.filter(e => !e.isDir).map(e => e.name);
      assert.deepStrictEqual([...fileNames], ['apple.wav', 'beats.mp3', 'zebra.wav']);
    });

    it('size-desc sort puts largest files first (folders still on top)', () => {
      F.loadFileSortFromPrefs(); // baseline
      // Simulate header click → size, asc=false
      F._fileSortKey = 'size'; F._fileSortAsc = false;
      // Re-load applies internal globals; helper reads them directly.
      const sorted = F.applyFileSort(entries);
      const filesOnly = sorted.filter(e => !e.isDir).map(e => e.name);
      assert.deepStrictEqual([...filesOnly], ['apple.wav', 'zebra.wav', 'beats.mp3']);
    });

    it('date-desc sort puts newest first', () => {
      F._fileSortKey = 'date'; F._fileSortAsc = false;
      const sorted = F.applyFileSort(entries);
      const filesOnly = sorted.filter(e => !e.isDir).map(e => e.name);
      assert.deepStrictEqual([...filesOnly], ['zebra.wav', 'beats.mp3', 'apple.wav']);
    });
  });

  describe('multi-select state helpers', () => {
    beforeEach(() => {
      F._fileSelected.clear();
      F._fileSelectLastIdx = -1;
      F._fileBrowserEntries = [
        { name: 'a.wav', path: '/x/a.wav', isDir: false, ext: 'wav', size: 100, modified: '' },
        { name: 'b.wav', path: '/x/b.wav', isDir: false, ext: 'wav', size: 200, modified: '' },
        { name: 'sub', path: '/x/sub', isDir: true, ext: '', size: 0, modified: '' },
      ];
    });

    it('toggleFileSelect adds/removes from the selection set', () => {
      F.toggleFileSelect('/x/a.wav', true);
      F.toggleFileSelect('/x/b.wav', true);
      assert.strictEqual(F._fileSelected.size, 2);
      F.toggleFileSelect('/x/a.wav', false);
      assert.strictEqual(F._fileSelected.size, 1);
      assert.ok(F._fileSelected.has('/x/b.wav'));
    });

    it('clearFileSelection empties the set and resets the shift-anchor', () => {
      F.toggleFileSelect('/x/a.wav', true);
      F.toggleFileSelect('/x/b.wav', true);
      F._fileSelectLastIdx = 5;
      F.clearFileSelection();
      assert.strictEqual(F._fileSelected.size, 0);
      assert.strictEqual(F._fileSelectLastIdx, -1);
    });

    it('_fileBulkSelectionAsPaths filters by predicate (folders-only)', () => {
      F.toggleFileSelect('/x/a.wav', true);
      F.toggleFileSelect('/x/sub', true);
      const dirs = F._fileBulkSelectionAsPaths((entry) => entry.isDir);
      assert.deepStrictEqual([...dirs], ['/x/sub']);
      const files = F._fileBulkSelectionAsPaths((entry) => !entry.isDir);
      assert.deepStrictEqual([...files], ['/x/a.wav']);
    });

    it('_fileBulkSelectionAsPaths skips paths no longer in the listing', () => {
      F.toggleFileSelect('/x/a.wav', true);
      F.toggleFileSelect('/x/gone.wav', true); // not in entries
      const all = F._fileBulkSelectionAsPaths(null);
      assert.deepStrictEqual([...all], ['/x/a.wav']);
    });
  });

  describe('saveFileSortToPrefs / loadFileSortFromPrefs round-trip', () => {
    it('persists key + direction and reloads them', () => {
      F._fileSortKey = 'size'; F._fileSortAsc = false;
      F.saveFileSortToPrefs();
      F._fileSortKey = 'name'; F._fileSortAsc = true; // reset
      F.loadFileSortFromPrefs();
      assert.strictEqual(F._fileSortKey, 'size');
      assert.strictEqual(F._fileSortAsc, false);
    });

    it('ignores unknown keys (defensive on corrupt prefs)', () => {
      F.prefs.setItem('fileSort', JSON.stringify({key: 'nonsense', asc: 'maybe'}));
      F._fileSortKey = 'name'; F._fileSortAsc = true;
      F.loadFileSortFromPrefs();
      // Both keys rejected → defaults preserved
      assert.strictEqual(F._fileSortKey, 'name');
      assert.strictEqual(F._fileSortAsc, true);
    });
  });
});
