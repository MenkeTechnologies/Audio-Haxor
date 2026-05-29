/**
 * `_pathDeleteItems` helper invariants (v1.28.14).
 *
 * The helper is the single source of truth for Move-to-Trash / Delete
 * Permanently / Secure Delete menu items. It is wired into every file
 * browser file row + every inventory table row (audio / DAW / MIDI /
 * preset / PDF / video). These tests pin:
 *   1. exact item count (3 for files, 2 for dirs — secure delete is
 *      file-only since the Rust command rejects directories)
 *   2. labels resolve via i18n keys (no raw English)
 *   3. successful delete removes the matching inventory row from the
 *      DOM (the bug fix in this commit — prior version left a stale row)
 *   4. file browser refresh path uses a directory-boundary-aware
 *      prefix check (`/foo` must NOT match `/foobar/x`)
 */
const fs = require('fs');
const path = require('path');
const vm = require('vm');
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

const CM_SRC = fs.readFileSync(
  path.join(__dirname, '..', 'frontend', 'js', 'context-menu.js'),
  'utf8',
);

/** Load `_pathDeleteItems` in isolation (it lives near the top of context-menu.js). */
function loadHelper(extras = {}) {
  const fnStart = CM_SRC.indexOf('function _pathDeleteItems(');
  const fnEnd = CM_SRC.indexOf('window._pathDeleteItems = _pathDeleteItems;');
  const fnSrc = CM_SRC.slice(fnStart, fnEnd) + 'window._pathDeleteItems = _pathDeleteItems;';
  const sandbox = {
    appFmt: (k) => k,
    toastFmt: (k, vars) => (vars ? `${k}|${JSON.stringify(vars)}` : k),
    showToast: () => {},
    confirmAction: async () => true,
    confirm: () => true,
    shortcutTip: () => ({}),
    _noEcho: { skipEchoToast: true },
    loadDirectory: () => { loadDirectoryCalls.push('called'); },
    _fileBrowserPath: '/home/user/music',
    document: {
      querySelectorAll: (sel) => sandbox.__rows.filter((r) => r.__matches(sel)),
    },
    window: {
      vstUpdater: {
        moveToTrash: async (p) => { ops.push(['trash', p]); },
        deleteFile:  async (p) => { ops.push(['delete', p]); },
        fsSecureDelete: async (p) => { ops.push(['secure', p]); },
      },
    },
    __rows: [],
    __lastPath: null,
    ...extras,
  };
  const ops = [];
  const loadDirectoryCalls = [];
  vm.createContext(sandbox);
  vm.runInContext(fnSrc, sandbox);
  return { helper: sandbox._pathDeleteItems || sandbox.window._pathDeleteItems, ops, loadDirectoryCalls, sandbox };
}

describe('_pathDeleteItems (context-menu shared helper)', () => {

  it('returns exactly 3 items for a file (trash / delete / secure delete)', () => {
    const { helper } = loadHelper();
    const items = helper('/path/to/song.wav', 'song.wav', false);
    assert.strictEqual(items.length, 3);
    assert.match(items[0].label, /menu\.fb_move_to_trash/);
    assert.match(items[1].label, /menu\.fb_delete_permanently/);
    assert.match(items[2].label, /menu\.fb_secure_delete/);
  });

  it('returns exactly 2 items for a directory (no secure delete)', () => {
    const { helper } = loadHelper();
    const items = helper('/path/to/Folder', 'Folder', true);
    assert.strictEqual(items.length, 2);
    assert.match(items[0].label, /menu\.fb_move_to_trash/);
    assert.match(items[1].label, /menu\.fb_delete_permanently/);
    for (const it of items) assert.ok(!/secure/.test(it.label), 'no secure delete on dirs');
  });

  it('every item routes its label through appFmt (no raw English)', () => {
    const { helper } = loadHelper();
    const items = helper('/x/file.wav', 'file.wav', false);
    for (const it of items) {
      assert.ok(it.label.startsWith('menu.'), `${it.label} should be an i18n key, not raw English`);
    }
  });

  it('on successful Move-to-Trash, removes the inventory row matching the path', async () => {
    // Build fake DOM rows: one audio row that matches, one that does NOT
    // (different path), one DAW row with a different path.
    const target = '/lib/sample.wav';
    const removedFlags = { match: false, other: false, daw: false };
    const rows = [
      mockRow('audio', target, () => { removedFlags.match = true; }),
      mockRow('audio', '/lib/other.wav', () => { removedFlags.other = true; }),
      mockRow('daw', '/lib/other.als', () => { removedFlags.daw = true; }),
    ];
    const { helper, ops } = loadHelper({ __rows: rows });
    const items = helper(target, 'sample.wav', false);
    await items[0].action(); // Move to Trash
    assert.deepStrictEqual(ops, [['trash', target]]);
    assert.ok(removedFlags.match, 'matching audio row must be removed from DOM');
    assert.ok(!removedFlags.other, 'non-matching row must NOT be touched');
    assert.ok(!removedFlags.daw, 'non-matching DAW row must NOT be touched');
  });

  it('on successful Secure Delete, removes the matching row', async () => {
    const target = '/lib/secret.txt';
    let removed = false;
    const rows = [ mockRow('pdf', target, () => { removed = true; }) ];
    const { helper } = loadHelper({ __rows: rows });
    const items = helper(target, 'secret.txt', false);
    await items[2].action();
    assert.ok(removed, 'matching row removed after secure delete');
  });

  it('file-browser loadDirectory triggers ONLY when deleted path lives under current dir (boundary-safe)', async () => {
    // Case A — file lives inside the file browser path: should reload.
    {
      const { helper, loadDirectoryCalls } = loadHelper({
        _fileBrowserPath: '/home/user/music',
      });
      const items = helper('/home/user/music/track.wav', 'track.wav', false);
      await items[0].action();
      assert.deepStrictEqual(loadDirectoryCalls, ['called']);
    }
    // Case B — sibling directory with overlapping prefix: must NOT reload.
    {
      const { helper, loadDirectoryCalls } = loadHelper({
        _fileBrowserPath: '/home/user/music',
      });
      const items = helper('/home/user/musical-instruments/song.wav', 'song.wav', false);
      await items[0].action();
      assert.deepStrictEqual(loadDirectoryCalls, [], 'prefix collision must NOT trigger reload');
    }
    // Case C — completely unrelated path: must NOT reload.
    {
      const { helper, loadDirectoryCalls } = loadHelper({
        _fileBrowserPath: '/home/user/music',
      });
      const items = helper('/tmp/unrelated.wav', 'unrelated.wav', false);
      await items[0].action();
      assert.deepStrictEqual(loadDirectoryCalls, [], 'unrelated path must NOT trigger reload');
    }
  });

});

/**
 * Fake DOM row matching the inventory table selectors the helper uses.
 * The helper calls `document.querySelectorAll('tr[data-audio-path], tr[data-daw-path], ...')`
 * and inspects `row.dataset.<type>Path`. We model just enough of that.
 */
function mockRow(type, dataPath, onRemove) {
  const dataset = {};
  dataset[`${type}Path`] = dataPath;
  return {
    dataset,
    remove() { onRemove(); },
    __matches(sel) {
      // The helper uses a multi-selector list separated by commas.
      // For the test, accept any row whose dataset has a matching key.
      const types = ['audio', 'daw', 'midi', 'preset', 'pdf', 'video'];
      return types.some((t) => sel.includes(`data-${t}-path`) && dataset[`${t}Path`]);
    },
  };
}
