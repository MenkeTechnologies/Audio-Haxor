const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

// ── Replicated from frontend/js/utils.js (pure search stack) ──

const SCORE_MATCH = 16;
const SCORE_GAP_START = -3;
const SCORE_GAP_EXTENSION = -1;
const BONUS_BOUNDARY = 9;
const BONUS_NON_WORD = 8;
const BONUS_CAMEL = 7;
const BONUS_CONSECUTIVE = 4;
const BONUS_FIRST_CHAR_MULT = 2;

const SCORE_SUBSTRING_BONUS = 1000;
const SCORE_EXACT_BONUS = 2000;
const SCORE_PREFIX_BONUS = 1500;

function charClass(c) {
  if (c >= 'a' && c <= 'z') return 1;
  if (c >= 'A' && c <= 'Z') return 2;
  if (c >= '0' && c <= '9') return 3;
  return 0;
}

function positionBonus(prev, curr) {
  const pc = charClass(prev);
  const cc = charClass(curr);
  if (pc === 0 && cc !== 0) return BONUS_BOUNDARY;
  if (pc === 1 && cc === 2) return BONUS_CAMEL;
  if (cc !== 0 && pc !== 0 && pc !== cc) return BONUS_NON_WORD;
  return 0;
}

function fzfMatch(needle, haystack) {
  const nLen = needle.length;
  const hLen = haystack.length;
  if (nLen === 0) return { score: 0, indices: [] };
  if (nLen > hLen) return null;

  const nLower = needle.toLowerCase();
  const hLower = haystack.toLowerCase();

  let ni = 0;
  for (let hi = 0; hi < hLen && ni < nLen; hi++) {
    if (hLower[hi] === nLower[ni]) ni++;
  }
  if (ni < nLen) return null;

  const starts = [];
  for (let i = 0; i <= hLen - nLen; i++) {
    if (hLower[i] === nLower[0]) starts.push(i);
  }

  let bestScore = -Infinity;
  let bestIndices = null;

  for (const start of starts) {
    const indices = [start];
    let si = start;
    let valid = true;

    for (let n = 1; n < nLen; n++) {
      let found = false;
      for (let h = si + 1; h < hLen; h++) {
        if (hLower[h] === nLower[n]) {
          indices.push(h);
          si = h;
          found = true;
          break;
        }
      }
      if (!found) {
        valid = false;
        break;
      }
    }
    if (!valid) continue;

    let score = 0;
    let prevIdx = -2;
    for (let i = 0; i < indices.length; i++) {
      const idx = indices[i];
      score += SCORE_MATCH;

      const prev = idx > 0 ? haystack[idx - 1] : ' ';
      let bonus = positionBonus(prev, haystack[idx]);
      if (i === 0) bonus *= BONUS_FIRST_CHAR_MULT;
      score += bonus;

      if (prevIdx === idx - 1) {
        score += BONUS_CONSECUTIVE;
      } else if (i > 0) {
        const gap = idx - prevIdx - 1;
        score += SCORE_GAP_START + SCORE_GAP_EXTENSION * (gap - 1);
      }
      prevIdx = idx;
    }

    if (score > bestScore) {
      bestScore = score;
      bestIndices = indices;
    }
  }

  if (!bestIndices) return null;
  return { score: bestScore, indices: bestIndices };
}

function parseToken(token) {
  let negate = false;
  let type = 'fuzzy';
  let text = token;
  if (text.startsWith('!')) {
    negate = true;
    text = text.slice(1);
  }
  if (text.startsWith("'") && text.endsWith("'") && text.length > 2) {
    type = 'exact';
    text = text.slice(1, -1);
  } else if (text.startsWith("'")) {
    type = 'exact';
    text = text.slice(1);
  } else if (text.startsWith('^')) {
    type = 'prefix';
    text = text.slice(1);
  } else if (text.endsWith('$')) {
    type = 'suffix';
    text = text.slice(0, -1);
  }
  return { type, text, negate };
}

function parseFzfQuery(query) {
  const tokens = query.split(/\s+/).filter(Boolean);
  const groups = [];
  let currentGroup = [];

  for (const token of tokens) {
    if (token === '|') continue;
    if (token.startsWith('|')) {
      currentGroup.push(parseToken(token.slice(1)));
    } else if (token.endsWith('|')) {
      currentGroup.push(parseToken(token.slice(0, -1)));
      groups.push(currentGroup);
      currentGroup = [];
    } else {
      if (currentGroup.length > 0) {
        groups.push(currentGroup);
        currentGroup = [];
      }
      currentGroup = [parseToken(token)];
    }
  }
  if (currentGroup.length > 0) groups.push(currentGroup);
  return groups;
}

function scoreToken(token, value) {
  const v = value.toLowerCase();
  const t = token.text.toLowerCase();
  switch (token.type) {
    case 'exact':
      return v.includes(t) ? SCORE_SUBSTRING_BONUS + t.length * SCORE_MATCH : 0;
    case 'prefix':
      return v.startsWith(t) ? SCORE_PREFIX_BONUS + t.length * SCORE_MATCH : 0;
    case 'suffix':
      return v.endsWith(t) ? SCORE_SUBSTRING_BONUS + t.length * SCORE_MATCH : 0;
    case 'fuzzy': {
      if (v === t) return SCORE_EXACT_BONUS + t.length * SCORE_MATCH;
      if (v.includes(t)) return SCORE_SUBSTRING_BONUS + t.length * SCORE_MATCH;
      const m = fzfMatch(token.text, value);
      return m ? m.score : 0;
    }
  }
  return 0;
}

function searchScore(query, fields, mode) {
  if (!query) return 1;
  if (mode === 'regex') {
    try {
      const re = new RegExp(query, 'i');
      return fields.some(f => re.test(f)) ? 1 : 0;
    } catch {
      return fields.some(f => f.toLowerCase().includes(query.toLowerCase())) ? 1 : 0;
    }
  }
  const groups = parseFzfQuery(query);
  let totalScore = 0;
  for (const orGroup of groups) {
    let bestGroupScore = 0;
    for (const token of orGroup) {
      let tokenBest = 0;
      for (let fi = 0; fi < fields.length; fi++) {
        const fieldBonus = fi === 0 ? 500 : 0;
        const s = scoreToken(token, fields[fi]);
        if (s > 0 && s + fieldBonus > tokenBest) tokenBest = s + fieldBonus;
      }
      if (token.negate) {
        if (tokenBest > 0) return 0;
        bestGroupScore = 1;
      } else {
        if (tokenBest > bestGroupScore) bestGroupScore = tokenBest;
      }
    }
    if (bestGroupScore === 0) return 0;
    totalScore += bestGroupScore;
  }
  return totalScore;
}

function getMatchIndices(query, text, mode) {
  if (!query || !text || mode === 'regex') {
    if (mode === 'regex' && query) {
      try {
        const re = new RegExp(query, 'ig');
        const indices = [];
        let m;
        while ((m = re.exec(text)) !== null) {
          for (let i = m.index; i < m.index + m[0].length; i++) indices.push(i);
        }
        return indices;
      } catch {
        return [];
      }
    }
    return [];
  }
  const groups = parseFzfQuery(query);
  const allIndices = new Set();
  for (const group of groups) {
    for (const token of group) {
      if (token.negate) continue;
      if (token.type === 'fuzzy') {
        const m = fzfMatch(token.text, text);
        if (m) m.indices.forEach(i => allIndices.add(i));
      } else {
        const t = token.text.toLowerCase();
        const idx = text.toLowerCase().indexOf(t);
        if (idx >= 0) {
          for (let i = idx; i < idx + t.length; i++) allIndices.add(i);
        }
      }
    }
  }
  return [...allIndices].sort((a, b) => a - b);
}

// ── Tests ──

describe('parseToken', () => {
  it('parses plain fuzzy term', () => {
    assert.deepStrictEqual(parseToken('serum'), { type: 'fuzzy', text: 'serum', negate: false });
  });

  it('parses negated term', () => {
    assert.deepStrictEqual(parseToken('!test'), { type: 'fuzzy', text: 'test', negate: true });
  });

  it('parses quoted exact', () => {
    assert.deepStrictEqual(parseToken("'foo bar'"), { type: 'exact', text: 'foo bar', negate: false });
  });

  it('parses opening quote exact without closing quote', () => {
    assert.deepStrictEqual(parseToken("'partial"), { type: 'exact', text: 'partial', negate: false });
  });

  it('parses prefix', () => {
    assert.deepStrictEqual(parseToken('^pre'), { type: 'prefix', text: 'pre', negate: false });
  });

  it('parses suffix', () => {
    assert.deepStrictEqual(parseToken('ing$'), { type: 'suffix', text: 'ing', negate: false });
  });

  it('applies negate after stripping suffix', () => {
    assert.deepStrictEqual(parseToken('!foo$'), { type: 'suffix', text: 'foo', negate: true });
  });
});

describe('parseFzfQuery', () => {
  it('splits AND groups on spaces', () => {
    const g = parseFzfQuery('foo bar');
    assert.strictEqual(g.length, 2);
    assert.deepStrictEqual(g[0], [{ type: 'fuzzy', text: 'foo', negate: false }]);
    assert.deepStrictEqual(g[1], [{ type: 'fuzzy', text: 'bar', negate: false }]);
  });

  it('OR within token via trailing pipe', () => {
    const g = parseFzfQuery('a| b');
    assert.strictEqual(g.length, 2);
    assert.deepStrictEqual(g[0][0].text, 'a');
    assert.deepStrictEqual(g[1][0].text, 'b');
  });

  it('OR via leading pipe merges into the same group (OR alternatives)', () => {
    const g = parseFzfQuery('x |y');
    assert.strictEqual(g.length, 1);
    assert.strictEqual(g[0].length, 2);
    assert.deepStrictEqual(g[0][0].text, 'x');
    assert.deepStrictEqual(g[0][1].text, 'y');
  });

  it('skips standalone pipe token', () => {
    const g = parseFzfQuery('a | b');
    assert.strictEqual(g.length, 2);
  });

  it('empty query yields empty groups', () => {
    assert.deepStrictEqual(parseFzfQuery(''), []);
    assert.deepStrictEqual(parseFzfQuery('   '), []);
  });
});

describe('charClass', () => {
  it('classifies letters and digits', () => {
    assert.strictEqual(charClass('a'), 1);
    assert.strictEqual(charClass('Z'), 2);
    assert.strictEqual(charClass('5'), 3);
    assert.strictEqual(charClass(' '), 0);
    assert.strictEqual(charClass('-'), 0);
  });
});

describe('positionBonus', () => {
  it('rewards word boundary', () => {
    assert.strictEqual(positionBonus(' ', 'a'), BONUS_BOUNDARY);
  });

  it('rewards camelCase step', () => {
    assert.strictEqual(positionBonus('a', 'B'), BONUS_CAMEL);
  });
});

describe('fzfMatch', () => {
  it('returns empty indices for empty needle', () => {
    const m = fzfMatch('', 'hello');
    assert.strictEqual(m.score, 0);
    assert.deepStrictEqual(m.indices, []);
  });

  it('returns null when needle longer than haystack', () => {
    assert.strictEqual(fzfMatch('abcdef', 'ab'), null);
  });

  it('returns null when characters missing in order', () => {
    assert.strictEqual(fzfMatch('xyz', 'abc'), null);
  });

  it('matches consecutive substring with high consecutive bonus', () => {
    const m = fzfMatch('ab', 'Zab');
    assert.ok(m);
    assert.deepStrictEqual(m.indices, [1, 2]);
    assert.strictEqual(m.score, 52);
    assert.deepStrictEqual(fzfMatch('ab', 'ab').indices, [0, 1]);
    assert.strictEqual(fzfMatch('ab', 'ab').score, 54);
  });

  it('is case insensitive', () => {
    const m = fzfMatch('ser', 'SeRuM');
    assert.ok(m);
    assert.ok(m.score > 0);
  });
});

describe('scoreToken', () => {
  it('exact type matches substring', () => {
    assert.ok(scoreToken({ type: 'exact', text: 'bar', negate: false }, 'foo bar baz') > 0);
    assert.strictEqual(scoreToken({ type: 'exact', text: 'missing', negate: false }, 'foo'), 0);
  });

  it('prefix and suffix', () => {
    assert.ok(scoreToken({ type: 'prefix', text: 'foo', negate: false }, 'foobar') > 0);
    assert.strictEqual(scoreToken({ type: 'prefix', text: 'foo', negate: false }, 'barfoo'), 0);
    assert.ok(scoreToken({ type: 'suffix', text: 'bar', negate: false }, 'foobar') > 0);
  });

  it('fuzzy prefers exact string', () => {
    const exact = scoreToken({ type: 'fuzzy', text: 'ab', negate: false }, 'ab');
    const fuzzy = scoreToken({ type: 'fuzzy', text: 'ab', negate: false }, 'acb');
    assert.ok(exact > fuzzy);
  });

  it('scoreToken deterministic scores match utils.js fzf constants', () => {
    assert.strictEqual(
      scoreToken({ type: 'exact', text: 'bar', negate: false }, 'foo bar baz'),
      1000 + 3 * 16
    );
    assert.strictEqual(
      scoreToken({ type: 'prefix', text: 'foo', negate: false }, 'foobar'),
      1500 + 3 * 16
    );
    assert.strictEqual(
      scoreToken({ type: 'suffix', text: 'bar', negate: false }, 'foobar'),
      1000 + 3 * 16
    );
    assert.strictEqual(
      scoreToken({ type: 'fuzzy', text: 'test', negate: false }, 'test'),
      2000 + 4 * 16
    );
  });
});

describe('searchScore', () => {
  it('returns 1 for empty query', () => {
    assert.strictEqual(searchScore('', ['a'], 'fuzzy'), 1);
  });

  it('regex mode matches case insensitively', () => {
    assert.ok(searchScore('foo', ['FooBar'], 'regex') > 0);
  });

  it('regex invalid pattern falls back to substring', () => {
    assert.ok(searchScore('[', ['[literal'], 'regex') > 0);
  });

  it('negated term fails when field matches', () => {
    assert.strictEqual(searchScore('!serum', ['Serum'], 'fuzzy'), 0);
  });

  it('negated term passes when field does not match', () => {
    assert.ok(searchScore('!serum', ['Massive'], 'fuzzy') > 0);
  });

  it('AND requires both space-separated groups', () => {
    assert.ok(searchScore('ser xfer', ['Serum', 'Xfer Records'], 'fuzzy') > 0);
    assert.strictEqual(searchScore('ser nomatch', ['Serum', 'Xfer'], 'fuzzy'), 0);
  });

  it('first field gets bonus over second', () => {
    const nameFirst = searchScore('test', ['testplugin', 'other'], 'fuzzy');
    const nameSecond = searchScore('test', ['other', 'testplugin'], 'fuzzy');
    assert.ok(nameFirst > nameSecond);
  });
});

describe('getMatchIndices', () => {
  it('returns character indices for regex matches', () => {
    assert.deepStrictEqual(getMatchIndices('foo', 'foobar', 'regex'), [0, 1, 2]);
  });

  it('collects overlapping regex indices', () => {
    const idx = getMatchIndices('o', 'foo', 'regex');
    assert.ok(idx.includes(1));
    assert.ok(idx.includes(2));
  });

  it('merges fuzzy match indices', () => {
    const idx = getMatchIndices('sr', 'Serum', 'fuzzy');
    assert.ok(idx.length > 0);
  });

  it('includes exact token positions', () => {
    const idx = getMatchIndices("'rum'", 'Serum', 'fuzzy');
    assert.ok(idx.some(i => 'Serum'.toLowerCase().indexOf('rum') === i));
  });

  it('returns empty for empty query in fuzzy mode', () => {
    assert.deepStrictEqual(getMatchIndices('', 'text', 'fuzzy'), []);
  });

  it('returns empty for empty text', () => {
    assert.deepStrictEqual(getMatchIndices('a', '', 'fuzzy'), []);
  });

  it('skips negated tokens for index collection', () => {
    const idx = getMatchIndices('!serum foo', 'foobar', 'fuzzy');
    assert.ok(idx.length > 0);
  });

  it('prefix token yields contiguous indices', () => {
    const idx = getMatchIndices('^foo', 'foobarbaz', 'fuzzy');
    assert.deepStrictEqual(idx, [0, 1, 2]);
  });
});

describe('parseToken (extended)', () => {
  it('negation before quoted exact', () => {
    assert.deepStrictEqual(parseToken("!'test'"), { type: 'exact', text: 'test', negate: true });
  });

  it('suffix with $ only', () => {
    assert.deepStrictEqual(parseToken('$'), { type: 'suffix', text: '', negate: false });
  });

  it('prefix only ^', () => {
    assert.deepStrictEqual(parseToken('^'), { type: 'prefix', text: '', negate: false });
  });
});

describe('parseFzfQuery (extended)', () => {
  it('three AND groups', () => {
    const g = parseFzfQuery('a b c');
    assert.strictEqual(g.length, 3);
  });

  it('single token single group', () => {
    assert.strictEqual(parseFzfQuery('onlyone').length, 1);
  });

  it('splits on tab like spaces', () => {
    const g = parseFzfQuery('foo\tbar');
    assert.strictEqual(g.length, 2);
    assert.strictEqual(g[0][0].text, 'foo');
    assert.strictEqual(g[1][0].text, 'bar');
  });

  it('multiple OR terms in one group via leading pipes', () => {
    const g = parseFzfQuery('a |b |c');
    assert.strictEqual(g.length, 1);
    assert.strictEqual(g[0].length, 3);
  });
});

describe('positionBonus (extended)', () => {
  it('digit to letter boundary', () => {
    assert.strictEqual(positionBonus('3', 'a'), BONUS_NON_WORD);
  });

  it('letter to digit', () => {
    assert.strictEqual(positionBonus('a', '3'), BONUS_NON_WORD);
  });

  it('space before letter', () => {
    assert.strictEqual(positionBonus(' ', 'X'), BONUS_BOUNDARY);
  });
});

describe('fzfMatch (extended)', () => {
  it('full string match', () => {
    const m = fzfMatch('abc', 'abc');
    assert.ok(m);
    assert.deepStrictEqual(m.indices, [0, 1, 2]);
  });

  it('single char', () => {
    const m = fzfMatch('z', 'azb');
    assert.ok(m);
    assert.deepStrictEqual(m.indices, [1]);
  });

  it('picks higher score among multiple start positions', () => {
    const m = fzfMatch('aa', 'xaa');
    assert.ok(m);
    assert.deepStrictEqual(m.indices, [1, 2]);
  });

  it('may match combining characters as separate code units', () => {
    const m = fzfMatch('ab', 'a\u0300b');
    assert.ok(m);
    assert.deepStrictEqual(m.indices, [0, 2]);
  });
});

describe('scoreToken (extended)', () => {
  it('exact empty text matches empty haystack includes', () => {
    assert.ok(scoreToken({ type: 'exact', text: '', negate: false }, '') > 0);
  });

  it('prefix empty matches any string', () => {
    assert.ok(scoreToken({ type: 'prefix', text: '', negate: false }, 'anything') > 0);
  });

  it('suffix empty matches any string', () => {
    assert.ok(scoreToken({ type: 'suffix', text: '', negate: false }, 'z') > 0);
  });

  it('fuzzy empty needle matches via substring rule (includes empty)', () => {
    assert.ok(scoreToken({ type: 'fuzzy', text: '', negate: false }, 'hello') > 0);
  });
});

describe('searchScore (extended)', () => {
  it('OR in one group matches either term (pipe must attach to next token, not standalone)', () => {
    assert.ok(searchScore('serum |massive', ['Massive X'], 'fuzzy') > 0);
    assert.ok(searchScore('serum |massive', ['Serum X'], 'fuzzy') > 0);
  });

  it('OR group fails when neither matches', () => {
    assert.strictEqual(searchScore('aaa |bbb', ['zzz'], 'fuzzy'), 0);
  });

  it('ser |xfer OR in one group (no space before xfer)', () => {
    assert.ok(searchScore('ser |xfer', ['Serum'], 'fuzzy') > 0);
  });

  it('three AND terms all required', () => {
    assert.ok(searchScore('a b c', ['abc'], 'fuzzy') > 0);
    assert.strictEqual(searchScore('a b z', ['abc'], 'fuzzy'), 0);
  });

  it('regex alternation', () => {
    assert.ok(searchScore('foo|bar', ['bazbar', 'none'], 'regex') > 0);
  });

  it('regex dot matches', () => {
    assert.ok(searchScore('f.o', ['fao'], 'regex') > 0);
  });

  it('regex case insensitive flag', () => {
    assert.ok(searchScore('ABC', ['xyzabc'], 'regex') > 0);
  });

  it('negate combined with positive in separate AND groups', () => {
    assert.ok(searchScore('serum !massive', ['Serum Plugin'], 'fuzzy') > 0);
    assert.strictEqual(searchScore('serum !serum', ['Serum'], 'fuzzy'), 0);
  });

  it('quoted exact AND fuzzy', () => {
    assert.ok(searchScore("'rum' ser", ['Serum'], 'fuzzy') > 0);
  });

  it('^prefix across two fields', () => {
    assert.ok(searchScore('^xfer', ['Other', 'Xfer Records'], 'fuzzy') > 0);
  });
});

describe('searchMatch boolean', () => {
  function searchMatch(query, fields, mode) {
    return searchScore(query, fields, mode) > 0;
  }

  it('true when score positive', () => {
    assert.strictEqual(searchMatch('x', ['abcx'], 'fuzzy'), true);
  });

  it('false when score zero', () => {
    assert.strictEqual(searchMatch('nomatch', ['a', 'b'], 'fuzzy'), false);
  });
});
