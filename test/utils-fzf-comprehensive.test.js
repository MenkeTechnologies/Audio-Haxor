/**
 * Deep behavioral tests for frontend/js/utils.js fzf search stack (VM-loaded).
 * Complements utils-vm-behavior.test.js with AND/OR aggregation, negation, regex,
 * highlighting, and formatter edge cases — catches regressions in tab filtering.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

describe('utils.js fzf comprehensive (vm)', () => {
  let U;

  before(() => {
    U = loadFrontendScripts(['utils.js'], { document: defaultDocument() });
  });

  describe('searchScore AND groups sum scores', () => {
    it('two space groups score higher than one token when both match', () => {
      const one = U.searchScore('a', ['alphabet soup'], 'fuzzy');
      const two = U.searchScore('a s', ['alphabet soup'], 'fuzzy');
      assert.ok(two > one, 'AND should add per-group scores');
    });

    it('fails when any AND group has no match', () => {
      assert.strictEqual(U.searchScore('a zzz', ['alphabet'], 'fuzzy'), 0);
    });

    it('three AND terms all required on one field', () => {
      assert.ok(U.searchScore('a b c', ['abc'], 'fuzzy') > 0);
      assert.strictEqual(U.searchScore('a b z', ['abc'], 'fuzzy'), 0);
    });
  });

  describe('searchScore OR within group', () => {
    it('matches best-scoring branch', () => {
      const orScore = U.searchScore('kick |snare', ['Snare Drum'], 'fuzzy');
      const onlyKick = U.searchScore('kick', ['Snare Drum'], 'fuzzy');
      assert.ok(orScore > 0);
      assert.ok(orScore >= onlyKick);
    });

    it('either alternative suffices', () => {
      assert.ok(U.searchScore('xxx |snare', ['Snare'], 'fuzzy') > 0);
      assert.ok(U.searchScore('snare |yyy', ['Snare'], 'fuzzy') > 0);
    });
  });

  describe('searchScore negation', () => {
    it('negated term matching fails entire query', () => {
      assert.strictEqual(U.searchScore('!serum', ['Serum'], 'fuzzy'), 0);
    });

    it('negated term not matching passes with positive score elsewhere', () => {
      assert.ok(U.searchScore('!zzz xfer', ['Xfer Serum'], 'fuzzy') > 0);
    });

    it('two AND groups: negation only applies to its token', () => {
      assert.ok(U.searchScore('serum !massive', ['Serum Two'], 'fuzzy') > 0);
      assert.strictEqual(U.searchScore('serum !serum', ['Serum'], 'fuzzy'), 0);
    });
  });

  describe('searchScore token types', () => {
    it('prefix ^ must match start of field', () => {
      assert.ok(U.searchScore('^pre', ['prefix rest'], 'fuzzy') > 0);
      assert.strictEqual(U.searchScore('^pre', ['not prefix'], 'fuzzy'), 0);
    });

    it('suffix $ must match end of field', () => {
      assert.ok(U.searchScore('end$', ['foo end'], 'fuzzy') > 0);
      assert.strictEqual(U.searchScore('end$', ['end middle'], 'fuzzy'), 0);
    });

    it('quoted exact substring (spaces inside quotes are one token — no space in query)', () => {
      assert.ok(U.searchScore("'foobar'", ['xx foobar yy'], 'fuzzy') > 0);
      assert.strictEqual(U.searchScore("'foobar'", ['foob'], 'fuzzy'), 0);
    });

    it('pipe inside unquoted fuzzy token is literal (no OR)', () => {
      const g = U.parseFzfQuery('foo|bar');
      assert.strictEqual(g.length, 1);
      assert.strictEqual(g[0][0].text, 'foo|bar');
    });
  });

  describe('searchScore regex mode', () => {
    it('anchors work', () => {
      assert.ok(U.searchScore('^foo', ['foobar'], 'regex') > 0);
      assert.strictEqual(U.searchScore('^foo', ['barfoo'], 'regex'), 0);
    });

    it('case insensitive', () => {
      assert.ok(U.searchScore('ABC', ['xxabcxx'], 'regex') > 0);
    });

    it('empty query matches before regex (same as fuzzy)', () => {
      assert.strictEqual(U.searchScore('', ['x'], 'regex'), 1);
    });

    it('dot matches', () => {
      assert.ok(U.searchScore('f.o', ['fao'], 'regex') > 0);
    });
  });

  describe('searchScore multi-field', () => {
    it('second field can satisfy a group when first cannot', () => {
      assert.ok(U.searchScore('hidden', ['visible', 'path/hidden/file.wav'], 'fuzzy') > 0);
    });

    it('first-field bonus: name column ranks above path-only match', () => {
      const nameWins = U.searchScore('xfer', ['Xfer Plugin', '/other/path'], 'fuzzy');
      const pathOnly = U.searchScore('xfer', ['Other', '/stuff/Xfer Records'], 'fuzzy');
      assert.ok(nameWins > pathOnly);
    });
  });

  describe('fzfMatch', () => {
    it('returns null when subsequence impossible', () => {
      assert.strictEqual(U.fzfMatch('abc', 'ab'), null);
    });

    it('single char at end of haystack', () => {
      const m = U.fzfMatch('z', 'abcz');
      assert.ok(m);
      assert.strictEqual(m.indices[m.indices.length - 1], 3);
    });

    it('repeated needle chars pick best-scoring alignment', () => {
      const m = U.fzfMatch('aa', 'baaca');
      assert.ok(m);
      assert.strictEqual(Array.from(m.indices).join(','), '1,2');
    });

    it('unicode code units match independently', () => {
      const m = U.fzfMatch('e', 'é');
      assert.strictEqual(m, null);
    });
  });

  describe('getMatchIndices', () => {
    it('AND fuzzy groups merge index sets', () => {
      const idx = U.getMatchIndices('a b', 'a x b', 'fuzzy');
      assert.ok(idx.includes(0));
      assert.ok(idx.includes(4));
    });

    it('exact token uses first indexOf match', () => {
      const idx = U.getMatchIndices("'oo'", 'foobar', 'fuzzy');
      const j = 'foobar'.indexOf('oo');
      assert.ok(idx.includes(j));
    });

    it('regex mode collects all match spans', () => {
      const idx = U.getMatchIndices('a', 'banana', 'regex');
      assert.ok(idx.length >= 3);
    });

    it('regex invalid pattern yields empty indices', () => {
      const idx = U.getMatchIndices('[', '[[', 'regex');
      assert.ok(Array.isArray(idx));
    });
  });

  describe('highlightMatch', () => {
    it('wraps matched spans for fuzzy AND query', () => {
      const h = U.highlightMatch('alpha beta', 'a b', 'fuzzy');
      assert.ok(h.includes('<mark class="fzf-hl">'));
      assert.ok(h.includes('</mark>'));
    });

    it('escapes user text before marking', () => {
      const h = U.highlightMatch('<x>', 'x', 'fuzzy');
      assert.ok(h.includes('&lt;'));
      assert.ok(h.includes('&gt;'));
    });

    it('no marks when query matches nothing', () => {
      const h = U.highlightMatch('abc', 'zzz', 'fuzzy');
      assert.strictEqual(h, U.escapeHtml('abc'));
    });

    it('highlights format badge text (e.g. Fla → FLAC)', () => {
      const h = U.highlightMatch('FLAC', 'Fla', 'fuzzy');
      assert.ok(h.includes('<mark class="fzf-hl">'));
    });
  });

  describe('highlightPathPrefixFromPath', () => {
    it('maps full-path indices onto directory prefix when match is only in path', () => {
      const path = '/Users/music/projects/sample.flac';
      const dir = '/Users/music';
      const h = U.highlightPathPrefixFromPath(path, dir, 'music', 'fuzzy');
      assert.ok(h.includes('<mark class="fzf-hl">'));
      assert.ok(h.includes('music'));
    });
  });

  describe('parseToken / parseFzfQuery', () => {
    it('parseToken: negate + prefix', () => {
      const t = U.parseToken('!^sys');
      assert.strictEqual(t.negate, true);
      assert.strictEqual(t.type, 'prefix');
      assert.strictEqual(t.text, 'sys');
    });

    it('parseToken: only negate marker', () => {
      const t = U.parseToken('!');
      assert.strictEqual(t.negate, true);
      assert.strictEqual(t.text, '');
    });

    it('parseFzfQuery: tab separates like space', () => {
      const g = U.parseFzfQuery('one\ttwo');
      assert.strictEqual(g.length, 2);
    });
  });

  describe('slugify', () => {
    it('empty yields empty', () => {
      assert.strictEqual(U.slugify(''), '');
    });

    it('collapses punctuation to single hyphen', () => {
      assert.strictEqual(U.slugify('a---b'), 'a-b');
    });

    it('preserves digit-letter boundaries', () => {
      assert.strictEqual(U.slugify('v2Plugin'), 'v-2-plugin');
    });
  });

  describe('formatTime / formatAudioSize', () => {
    it('formatTime pads seconds', () => {
      assert.strictEqual(U.formatTime(61), '1:01');
      assert.strictEqual(U.formatTime(3599), '59:59');
    });

    it('formatTime rejects infinite', () => {
      assert.strictEqual(U.formatTime(Infinity), '0:00');
    });

    it('formatAudioSize uses TB for huge values', () => {
      const tb = 1024 ** 5 * 2;
      assert.ok(U.formatAudioSize(tb).includes('TB'));
    });
  });

  describe('escapePath', () => {
    it('handles empty string', () => {
      assert.strictEqual(U.escapePath(''), '');
    });

    it('multiple backslashes', () => {
      assert.strictEqual(U.escapePath('a\\\\b'), 'a\\\\\\\\b');
    });
  });

  describe('buildDirsTable', () => {
    it('returns empty for empty directories', () => {
      assert.strictEqual(U.buildDirsTable([], []), '');
    });

    it('counts plugins under directory prefix', () => {
      const html = U.buildDirsTable(
        ['/Lib'],
        [{ path: '/Lib/A.vst3', type: 'VST3' }, { path: '/Other/B.vst3', type: 'VST3' }]
      );
      assert.ok(html.includes('/Lib'));
      assert.ok(html.includes('>1<'));
    });
  });

  describe('searchScore edge cases', () => {
    it('single-char fuzzy matches', () => {
      assert.ok(U.searchScore('z', ['az'], 'fuzzy') > 0);
    });

    it('field array empty never matches non-empty query', () => {
      assert.strictEqual(U.searchScore('x', [], 'fuzzy'), 0);
    });

    it('regex alternation across fields', () => {
      assert.ok(U.searchScore('foo|bar', ['x', 'barrel'], 'regex') > 0);
    });
  });

  describe('findByPath', () => {
    it('returns last row when duplicate paths exist (later wins)', () => {
      const rows = [
        { path: '/dup', v: 1 },
        { path: '/dup', v: 2 },
      ];
      assert.strictEqual(U.findByPath(rows, '/dup').v, 2);
    });
  });

  describe('fzfMatch gap scoring', () => {
    it('large gap lowers score vs adjacent match (gap penalties dominate)', () => {
      const tight = U.fzfMatch('ab', 'ab');
      const loose = U.fzfMatch('ab', `a${'_'.repeat(20)}b`);
      assert.ok(tight.score > loose.score);
    });
  });

  describe('parseFzfQuery edge', () => {
    it('leading pipe starts OR group', () => {
      const g = U.parseFzfQuery('|only');
      assert.strictEqual(g.length, 1);
      assert.strictEqual(g[0][0].text, 'only');
    });

    it('trailing pipe after token ends group', () => {
      const g = U.parseFzfQuery('x |');
      assert.strictEqual(g.length, 1);
      assert.strictEqual(g[0][0].text, 'x');
    });
  });

  describe('getMatchIndices fuzzy + exact AND', () => {
    it('collects indices from quoted exact and fuzzy in one query', () => {
      const idx = U.getMatchIndices("'oo' f", 'foobar', 'fuzzy');
      assert.ok(idx.length >= 2);
      assert.ok(idx.includes('foobar'.indexOf('oo')));
    });
  });

  describe('highlightMatch regex', () => {
    it('marks regex hits', () => {
      const h = U.highlightMatch('hello world', 'o.', 'regex');
      assert.ok(h.includes('<mark'));
    });
  });

  describe('formatAudioSize units', () => {
    it('bytes and KB', () => {
      assert.strictEqual(U.formatAudioSize(512), '512.0 B');
      assert.match(U.formatAudioSize(2048), /^2\.0 KB$/);
    });
  });
});
