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

    it('items-desc sort uses folder `itemsCount` from the bg walk', () => {
      const folderEntries = [
        { name: 'small', path: '/x/small', isDir: true, ext: '', size: 0, modified: '', itemsCount: 5 },
        { name: 'huge', path: '/x/huge', isDir: true, ext: '', size: 0, modified: '', itemsCount: 5000 },
        { name: 'medium', path: '/x/medium', isDir: true, ext: '', size: 0, modified: '', itemsCount: 100 },
      ];
      F._fileSortKey = 'items'; F._fileSortAsc = false;
      const sorted = F.applyFileSort(folderEntries);
      assert.deepStrictEqual(sorted.map(e => e.name), ['huge', 'medium', 'small']);
    });

    it('created-desc sort uses the `created` field (newest first)', () => {
      const dated = [
        { name: 'old', path: '/x/old', isDir: false, ext: '', size: 1, modified: '', created: '2020-01-01 00:00' },
        { name: 'new', path: '/x/new', isDir: false, ext: '', size: 1, modified: '', created: '2026-01-01 00:00' },
        { name: 'middle', path: '/x/middle', isDir: false, ext: '', size: 1, modified: '', created: '2023-06-15 00:00' },
      ];
      F._fileSortKey = 'created'; F._fileSortAsc = false;
      const sorted = F.applyFileSort(dated);
      assert.deepStrictEqual(sorted.map(e => e.name), ['new', 'middle', 'old']);
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

  describe('history nav (_navHistoryRecord / fileNavBack / fileNavForward)', () => {
    beforeEach(() => {
      F._fbHistory = [];
      F._fbHistoryIdx = -1;
      F._fbHistorySkipPush = false;
    });

    it('records each new path and advances the index', () => {
      F._navHistoryRecord('/a');
      F._navHistoryRecord('/b');
      F._navHistoryRecord('/c');
      assert.deepStrictEqual([...F._fbHistory], ['/a', '/b', '/c']);
      assert.strictEqual(F._fbHistoryIdx, 2);
    });

    it('no-op when recording the same path twice in a row', () => {
      F._navHistoryRecord('/a');
      F._navHistoryRecord('/a');
      assert.deepStrictEqual([...F._fbHistory], ['/a']);
      assert.strictEqual(F._fbHistoryIdx, 0);
    });

    it('skips push when the skip flag is set (back/forward triggered loads)', () => {
      F._navHistoryRecord('/a');
      F._fbHistorySkipPush = true;
      F._navHistoryRecord('/b');
      assert.deepStrictEqual([...F._fbHistory], ['/a']);
      assert.strictEqual(F._fbHistoryIdx, 0);
      assert.strictEqual(F._fbHistorySkipPush, false, 'flag must reset after consuming');
    });

    it('drops forward history when navigating to a new path after back', () => {
      F._navHistoryRecord('/a');
      F._navHistoryRecord('/b');
      F._navHistoryRecord('/c');
      F._fbHistoryIdx = 0; // simulate two back-clicks
      F._navHistoryRecord('/x');
      assert.deepStrictEqual([...F._fbHistory], ['/a', '/x']);
      assert.strictEqual(F._fbHistoryIdx, 1);
    });
  });

  describe('ext filter (_fbExtMatches)', () => {
    it('all → matches every entry', () => {
      assert.ok(F._fbExtMatches({isDir: false, ext: 'wav'}, 'all'));
      assert.ok(F._fbExtMatches({isDir: false, ext: 'xyz'}, 'all'));
    });

    it('folders are always kept regardless of category', () => {
      assert.ok(F._fbExtMatches({isDir: true, ext: ''}, 'audio'));
      assert.ok(F._fbExtMatches({isDir: true, ext: ''}, 'other'));
    });

    it('audio matches AUDIO_EXTS, rejects other types', () => {
      assert.ok(F._fbExtMatches({isDir: false, ext: 'wav'}, 'audio'));
      assert.ok(!F._fbExtMatches({isDir: false, ext: 'pdf'}, 'audio'));
    });

    it('other is the inverse of all categorized types', () => {
      assert.ok(F._fbExtMatches({isDir: false, ext: 'xyz'}, 'other'));
      assert.ok(!F._fbExtMatches({isDir: false, ext: 'wav'}, 'other'));
      assert.ok(!F._fbExtMatches({isDir: false, ext: 'pdf'}, 'other'));
    });
  });

  describe('_fbBulkRenameComputeName', () => {
    const audio = { name: 'kick.wav', path: '/x/kick.wav', isDir: false };
    const folder = { name: 'beats', path: '/x/beats', isDir: true };

    it('find/replace operates on basename only (extension preserved)', () => {
      const out = F._fbBulkRenameComputeName(
        audio,
        { find: 'kick', replace: 'snare', regex: false, prefix: '', suffix: '', numStart: 1, numPad: 1 },
        0,
      );
      assert.strictEqual(out, 'snare.wav');
    });

    it('regex find/replace supports backreferences', () => {
      const out = F._fbBulkRenameComputeName(
        { name: 'sample_42.wav', path: '/x/sample_42.wav', isDir: false },
        { find: '(\\w+)_(\\d+)', replace: '$2_$1', regex: true, prefix: '', suffix: '', numStart: 1, numPad: 1 },
        0,
      );
      assert.strictEqual(out, '42_sample.wav');
    });

    it('prefix prepended, suffix inserted before extension', () => {
      const out = F._fbBulkRenameComputeName(
        audio,
        { find: '', replace: '', regex: false, prefix: 'MASTER_', suffix: '_v2', numStart: 1, numPad: 1 },
        0,
      );
      assert.strictEqual(out, 'MASTER_kick_v2.wav');
    });

    it('{n} placeholder substituted with padded index', () => {
      const out = F._fbBulkRenameComputeName(
        audio,
        { find: '', replace: '', regex: false, prefix: '{n}_', suffix: '', numStart: 1, numPad: 3 },
        4,
      );
      assert.strictEqual(out, '005_kick.wav'); // numStart=1 + index=4 = 5, padded to 3 digits
    });

    it('folder names: no extension handling, suffix appended at end', () => {
      const out = F._fbBulkRenameComputeName(
        folder,
        { find: '', replace: '', regex: false, prefix: 'old_', suffix: '_pack', numStart: 1, numPad: 1 },
        0,
      );
      assert.strictEqual(out, 'old_beats_pack');
    });

    it('{name} and {ext} placeholders resolve to the original parts', () => {
      const out = F._fbBulkRenameComputeName(
        audio,
        { find: '', replace: '', regex: false, prefix: '{name}_copy', suffix: '_{ext}', numStart: 1, numPad: 1 },
        0,
      );
      assert.strictEqual(out, 'kick_copykick_wav.wav');
    });
  });

  describe('_fbFormatItemCount', () => {
    it('formats sub-thousand as plain integer', () => {
      assert.strictEqual(F._fbFormatItemCount(0), '0');
      assert.strictEqual(F._fbFormatItemCount(47), '47');
      assert.strictEqual(F._fbFormatItemCount(999), '999');
    });

    it('formats thousands as k with one decimal under 10k, none above', () => {
      assert.strictEqual(F._fbFormatItemCount(1000), '1.0k');
      assert.strictEqual(F._fbFormatItemCount(9999), '10.0k');
      assert.strictEqual(F._fbFormatItemCount(12_345), '12k');
      assert.strictEqual(F._fbFormatItemCount(999_999), '1000k');
    });

    it('formats millions as M', () => {
      assert.strictEqual(F._fbFormatItemCount(1_000_000), '1.0M');
      assert.strictEqual(F._fbFormatItemCount(3_500_000), '3.5M');
    });

    it('invalid input → empty string', () => {
      assert.strictEqual(F._fbFormatItemCount(NaN), '');
      assert.strictEqual(F._fbFormatItemCount(-1), '');
      assert.strictEqual(F._fbFormatItemCount(Infinity), '');
    });
  });

  describe('saveFileColumnWidths / loadFileColumnWidths', () => {
    it('persists known columns + ignores unknown ones on load', () => {
      const fakeTab = {
        style: {
          _props: {},
          setProperty(name, value) { this._props[name] = value; },
          getPropertyValue(name) { return this._props[name] || ''; },
        },
      };
      F.document = {
        getElementById: (id) => (id === 'tabFiles' ? fakeTab : null),
      };
      fakeTab.style.setProperty('--fb-w-size', '95px');
      fakeTab.style.setProperty('--fb-w-date', '180px');
      F.saveFileColumnWidths();
      // Reset and reload
      fakeTab.style._props = {};
      // Inject corrupt + valid entries
      F.prefs.setItem('fileBrowserColWidths', {size: '95px', date: '180px', bogus: '100px'});
      F.loadFileColumnWidths();
      assert.strictEqual(fakeTab.style.getPropertyValue('--fb-w-size'), '95px');
      assert.strictEqual(fakeTab.style.getPropertyValue('--fb-w-date'), '180px');
      assert.strictEqual(fakeTab.style.getPropertyValue('--fb-w-bogus'), '', 'unknown col ignored');
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
