/**
 * Exercises the real frontend/js/utils.js in Node via vm + minimal DOM stubs.
 * Catches drift between production search/indexing code and mirrored unit tests.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

function loadUtilsSandbox() {
  const utilsPath = path.join(__dirname, '..', 'frontend', 'js', 'utils.js');
  const code = fs.readFileSync(utilsPath, 'utf8');
  /** Minimal <div> so escapeHtml() matches browser (textContent → innerHTML entities). */
  function createTextDiv() {
    let raw = '';
    return {
      set textContent(v) {
        raw = v == null ? '' : String(v);
      },
      get textContent() {
        return raw;
      },
      get innerHTML() {
        return raw
          .replace(/&/g, '&amp;')
          .replace(/</g, '&lt;')
          .replace(/>/g, '&gt;')
          .replace(/"/g, '&quot;');
      },
    };
  }
  const sandbox = {
    console,
    performance: { now: () => 0 },
    KVR_MANUFACTURER_MAP: {
      'native-instruments': 'native-instruments',
      'u-he': 'u-he',
    },
    prefs: {
      getObject: () => null,
      getItem: () => null,
      setItem: () => {},
      removeItem: () => {},
    },
    document: {
      createElement: () => createTextDiv(),
      getElementById: () => null,
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
    },
    setTimeout: () => 0,
    clearTimeout: () => {},
  };
  sandbox.window = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(code, sandbox);
  return sandbox;
}

describe('frontend/js/utils.js (vm-loaded source)', () => {
  let U;

  before(() => {
    U = loadUtilsSandbox();
  });

  it('charClass classifies lower, upper, digit, non-word', () => {
    assert.strictEqual(U.charClass('z'), 1);
    assert.strictEqual(U.charClass('M'), 2);
    assert.strictEqual(U.charClass('0'), 3);
    assert.strictEqual(U.charClass('_'), 0);
  });

  it('positionBonus: boundary, camelCase, non-word class transitions', () => {
    assert.strictEqual(U.positionBonus(' ', 'a'), 9);
    assert.strictEqual(U.positionBonus('a', 'B'), 7);
    assert.strictEqual(U.positionBonus('a', '1'), 8);
    assert.strictEqual(U.positionBonus('a', 'b'), 0);
  });

  it('searchScore: empty query matches any row (score > 0)', () => {
    assert.strictEqual(U.searchScore('', ['x'], 'fuzzy'), 1);
  });

  it('searchMatch is true iff searchScore is positive', () => {
    assert.strictEqual(U.searchMatch('serum', ['Serum'], 'fuzzy'), true);
    assert.strictEqual(U.searchMatch('serum', ['Massive'], 'fuzzy'), false);
  });

  it('searchScore: negate removes rows that match the term', () => {
    assert.strictEqual(U.searchScore('!serum', ['Serum'], 'fuzzy'), 0);
    assert.ok(U.searchScore('!serum', ['Massive'], 'fuzzy') > 0);
  });

  it('searchScore: OR group matches if any branch matches', () => {
    assert.ok(U.searchScore('serum |massive', ['Massive X'], 'fuzzy') > 0);
    assert.strictEqual(U.searchScore('aaa |bbb', ['zzz'], 'fuzzy'), 0);
  });

  it('searchScore: AND across groups (tokens in same group are OR; groups are AND)', () => {
    assert.ok(U.searchScore('ser xfer', ['Serum', 'Xfer Records'], 'fuzzy') > 0);
    assert.strictEqual(U.searchScore('ser nomatch', ['Serum', 'Xfer'], 'fuzzy'), 0);
  });

  it('searchScore: first field gets bonus vs second (plugin list behavior)', () => {
    const a = U.searchScore('test', ['testplugin', 'other'], 'fuzzy');
    const b = U.searchScore('test', ['other', 'testplugin'], 'fuzzy');
    assert.ok(a > b);
  });

  it('fzfMatch: greedy extension per start + fzf-style bonuses (default SCORE_* / BONUS_*)', () => {
    const m = U.fzfMatch('ab', 'Zab');
    assert.ok(m);
    assert.deepStrictEqual(Array.from(m.indices), [1, 2]);
    // 'Z'→'a' is upper→lower (BONUS_NON_WORD×first_char_mult), not boundary; + consecutive 'a'→'b'
    assert.strictEqual(m.score, 52);
    const full = U.fzfMatch('ab', 'ab');
    assert.ok(full);
    assert.strictEqual(full.score, 54);
    assert.deepStrictEqual(Array.from(full.indices), [0, 1]);
  });

  it('scoreToken: exact / prefix / suffix / fuzzy use fzf-weighted scores (default constants)', () => {
    assert.strictEqual(
      U.scoreToken({ type: 'exact', text: 'bar', negate: false }, 'foo bar baz'),
      1000 + 3 * 16
    );
    assert.strictEqual(
      U.scoreToken({ type: 'prefix', text: 'foo', negate: false }, 'foobar'),
      1500 + 3 * 16
    );
    assert.strictEqual(
      U.scoreToken({ type: 'suffix', text: 'bar', negate: false }, 'foobar'),
      1000 + 3 * 16
    );
    assert.strictEqual(
      U.scoreToken({ type: 'fuzzy', text: 'test', negate: false }, 'test'),
      2000 + 4 * 16
    );
    const fuzzyOnly = U.scoreToken({ type: 'fuzzy', text: 'fb', negate: false }, 'FooBar');
    assert.ok(fuzzyOnly > 0);
    assert.ok(fuzzyOnly < 2000);
  });

  it('searchScore: invalid regex falls back to substring', () => {
    assert.ok(U.searchScore('[', ['[literal'], 'regex') > 0);
  });

  it('searchScore: valid regex mode', () => {
    assert.ok(U.searchScore('f.o', ['fao'], 'regex') > 0);
  });

  it('getMatchIndices: regex collects match spans', () => {
    const idx = U.getMatchIndices('a.', 'abc', 'regex');
    assert.ok(idx.includes(0));
  });

  it('highlightMatch: escapes HTML and wraps fuzzy indices', () => {
    const h = U.highlightMatch('a<b', 'ab', 'fuzzy');
    assert.ok(h.includes('&lt;'));
    assert.ok(h.includes('<mark class="fzf-hl">'));
  });

  it('formatAudioSize and formatTime match tabular display contract', () => {
    assert.match(U.formatAudioSize(1024), /1\.0 KB/);
    assert.strictEqual(U.formatTime(0), '0:00');
    assert.strictEqual(U.formatTime(125.7), '2:05');
    assert.strictEqual(U.formatTime(NaN), '0:00');
  });

  it('escapePath escapes backslashes and single quotes for SQL-like consumers', () => {
    assert.strictEqual(U.escapePath("a\\b'c"), "a\\\\b\\'c");
  });

  it('slugify produces KVR-style slugs', () => {
    assert.strictEqual(U.slugify('MadronaLabs'), 'madrona-labs');
    assert.strictEqual(U.slugify('Plugin3'), 'plugin-3');
  });

  it('buildKvrUrl: without manufacturer uses name slug only', () => {
    assert.strictEqual(
      U.buildKvrUrl('Pro-Q 3', 'Unknown'),
      'https://www.kvraudio.com/product/pro-q-3'
    );
  });

  it('buildKvrUrl: maps known manufacturer slug from KVR_MANUFACTURER_MAP', () => {
    const url = U.buildKvrUrl('Massive', 'Native Instruments');
    assert.ok(url.includes('native-instruments'));
    assert.ok(url.startsWith('https://www.kvraudio.com/product/'));
  });

  it('buildDirsTable: counts plugins under each directory prefix', () => {
    const html = U.buildDirsTable(
      ['/Lib/Plugins'],
      [
        { path: '/Lib/Plugins/A.vst3', type: 'VST3' },
        { path: '/Lib/Plugins/B.vst3', type: 'VST3' },
        { path: '/Other/x.vst3', type: 'VST3' },
      ]
    );
    assert.ok(html.includes('/Lib/Plugins'));
    assert.ok(html.includes('>2<'));
    assert.ok(html.includes('VST3: 2'));
  });

  it('findByPath: builds index incrementally on append', () => {
    const rows = [{ path: '/a.wav' }, { path: '/b.wav' }];
    assert.strictEqual(U.findByPath(rows, '/b.wav').path, '/b.wav');
    rows.push({ path: '/c.wav' });
    assert.strictEqual(U.findByPath(rows, '/c.wav').path, '/c.wav');
  });

  it('findByPath: reindex after in-place truncation (WeakMap cache invalidation)', () => {
    const rows = [{ path: '/a.wav' }, { path: '/b.wav' }];
    U.findByPath(rows, '/a.wav');
    rows.length = 0;
    rows.push({ path: '/c.wav' });
    assert.strictEqual(U.findByPath(rows, '/c.wav').path, '/c.wav');
    assert.strictEqual(U.findByPath(rows, '/a.wav'), undefined);
  });

  it('findByPath: reindex flag forces rebuild', () => {
    const rows = [{ path: '/x.wav' }];
    assert.strictEqual(U.findByPath(rows, '/x.wav', true).path, '/x.wav');
  });

  it('findByPath returns undefined when arr or path is falsy', () => {
    assert.strictEqual(U.findByPath(null, '/a.wav'), undefined);
    assert.strictEqual(U.findByPath([{ path: '/a.wav' }], ''), undefined);
    assert.strictEqual(U.findByPath([{ path: '/a.wav' }], null), undefined);
  });

  it('applyFilter binds cfg so fetchFn can read this.lastSearch / this.lastMode', () => {
    let capturedSearch = '';
    let capturedMode = '';
    const searchInput = { value: '  needle  ' };
    const origGet = U.document.getElementById;
    U.document.getElementById = (id) => (id === 'searchInput' ? searchInput : origGet(id));
    try {
      U.registerFilter('applyFilterBindTest', {
        inputId: 'searchInput',
        fetchFn() {
          capturedSearch = this.lastSearch || '';
          capturedMode = this.lastMode || '';
        },
      });
      U.applyFilter('applyFilterBindTest');
    } finally {
      U.document.getElementById = origGet;
    }
    assert.strictEqual(capturedSearch, 'needle');
    assert.strictEqual(capturedMode, 'fuzzy');
  });
});
