/**
 * Handwritten pure logic mirrored from frontend/js (duplicates, kvr, xref).
 * Keeps grouping/dedup contracts testable without DOM or Tauri.
 */
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

// ── duplicates.js ──
function findDuplicates(items, keyFn) {
  const groups = {};
  for (const item of items) {
    const key = keyFn(item);
    if (!groups[key]) groups[key] = [];
    groups[key].push(item);
  }
  return Object.values(groups).filter((g) => g.length > 1);
}

// ── kvr.js ──
function kvrCacheKey(plugin) {
  return `${(plugin.manufacturer || 'Unknown').toLowerCase()}|||${plugin.name.toLowerCase()}`;
}

/** Must stay in sync with `frontend/js/xref.js` `XREF_FORMATS`. */
const XREF_FORMATS = new Set([
  'ALS',
  'RPP',
  'RPP-BAK',
  'BWPROJECT',
  'SONG',
  'DAWPROJECT',
  'FLP',
  'LOGICX',
  'CPR',
  'NPR',
  'PTX',
  'PTF',
  'REASON',
]);

function isXrefSupported(format) {
  return XREF_FORMATS.has(format);
}

describe('findDuplicates', () => {
  it('empty input yields empty', () => {
    assert.deepStrictEqual(findDuplicates([], (x) => x.id), []);
  });

  it('returns nothing when all keys unique', () => {
    const items = [{ id: 'a' }, { id: 'b' }];
    assert.deepStrictEqual(findDuplicates(items, (x) => x.id), []);
  });

  it('groups pairs by key', () => {
    const items = [{ k: 1, n: 'a' }, { k: 1, n: 'b' }, { k: 2, n: 'c' }];
    const dups = findDuplicates(items, (x) => x.k);
    assert.strictEqual(dups.length, 1);
    assert.strictEqual(dups[0].length, 2);
    assert.deepStrictEqual(
      dups[0].map((x) => x.n).sort(),
      ['a', 'b']
    );
  });

  it('triple collision is one group of three', () => {
    const items = [{ p: '/a' }, { p: '/b' }, { p: '/c' }];
    const dups = findDuplicates(items, () => 'same');
    assert.strictEqual(dups.length, 1);
    assert.strictEqual(dups[0].length, 3);
  });

  it('keyFn can return composite string', () => {
    const items = [
      { name: 'Kick', fmt: 'WAV' },
      { name: 'Kick', fmt: 'WAV' },
      { name: 'Kick', fmt: 'MP3' },
    ];
    const dups = findDuplicates(items, (s) => `${s.name}|${s.fmt}`);
    assert.strictEqual(dups.length, 1);
    assert.strictEqual(dups[0].length, 2);
  });

  it('numeric keys stringify consistently', () => {
    const items = [{ id: 1 }, { id: 1 }];
    const dups = findDuplicates(items, (x) => x.id);
    assert.strictEqual(dups[0].length, 2);
  });
});

describe('kvrCacheKey', () => {
  it('joins lowercased mfg and name with delimiter', () => {
    assert.strictEqual(
      kvrCacheKey({ name: 'Serum', manufacturer: 'Xfer' }),
      'xfer|||serum'
    );
  });

  it('uses Unknown when manufacturer missing', () => {
    assert.strictEqual(
      kvrCacheKey({ name: 'Foo' }),
      'unknown|||foo'
    );
  });

  it('uses Unknown when manufacturer is empty string', () => {
    assert.strictEqual(
      kvrCacheKey({ name: 'Foo', manufacturer: '' }),
      'unknown|||foo'
    );
  });

  it('folds case on both sides', () => {
    const a = kvrCacheKey({ name: 'ABC', manufacturer: 'Big Co' });
    const b = kvrCacheKey({ name: 'abc', manufacturer: 'BIG CO' });
    assert.strictEqual(a, b);
  });

  it('preserves distinct names after lowercasing', () => {
    assert.notStrictEqual(
      kvrCacheKey({ name: 'A', manufacturer: 'M' }),
      kvrCacheKey({ name: 'B', manufacturer: 'M' })
    );
  });
});

describe('XREF_FORMATS / isXrefSupported', () => {
  it('contains expected DAW project extensions', () => {
    assert.strictEqual(isXrefSupported('ALS'), true);
    assert.strictEqual(isXrefSupported('RPP'), true);
    assert.strictEqual(isXrefSupported('LOGICX'), true);
    assert.strictEqual(isXrefSupported('FLP'), true);
    assert.strictEqual(isXrefSupported('DAWPROJECT'), true);
  });

  it('is case-sensitive (uppercase only)', () => {
    assert.strictEqual(isXrefSupported('als'), false);
    assert.strictEqual(isXrefSupported('rpp'), false);
  });

  it('rejects unrelated formats', () => {
    assert.strictEqual(isXrefSupported('VST3'), false);
    assert.strictEqual(isXrefSupported('WAV'), false);
    assert.strictEqual(isXrefSupported(''), false);
  });

  it('RPP-BAK variant is listed', () => {
    assert.strictEqual(isXrefSupported('RPP-BAK'), true);
  });

  it('set size matches array length (no duplicates in source list)', () => {
    assert.strictEqual(XREF_FORMATS.size, [...XREF_FORMATS].length);
  });
});
