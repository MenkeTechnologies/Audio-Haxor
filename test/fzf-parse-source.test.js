/**
 * Real utils.js: parseFzfQuery groups (AND across space groups, OR via |prefix token).
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

describe('frontend/js/utils.js parseFzfQuery (vm-loaded)', () => {
  let U;

  before(() => {
    U = loadFrontendScripts(['utils.js'], { document: defaultDocument() });
  });

  it('splits space-separated tokens into AND groups (one token per group)', () => {
    const g = U.parseFzfQuery('foo bar');
    assert.strictEqual(g.length, 2);
    assert.strictEqual(g[0][0].text, 'foo');
    assert.strictEqual(g[1][0].text, 'bar');
  });

  it('uses pipe-with-space as AND between groups', () => {
    const g = U.parseFzfQuery('a | b');
    assert.strictEqual(g.length, 2);
    assert.strictEqual(g[0][0].text, 'a');
    assert.strictEqual(g[1][0].text, 'b');
  });

  it('uses |prefix on a token for OR within one group', () => {
    const g = U.parseFzfQuery('serum |massive');
    assert.strictEqual(g.length, 1);
    assert.strictEqual(g[0].length, 2);
    assert.strictEqual(g[0][0].text, 'serum');
    assert.strictEqual(g[0][1].text, 'massive');
  });

  it('parses negate, prefix, and suffix token shapes', () => {
    const g = U.parseFzfQuery('!foo ^bar baz$');
    assert.strictEqual(g.length, 3);
    assert.strictEqual(g[0][0].negate, true);
    assert.strictEqual(g[0][0].text, 'foo');
    assert.strictEqual(g[1][0].type, 'prefix');
    assert.strictEqual(g[1][0].text, 'bar');
    assert.strictEqual(g[2][0].type, 'suffix');
    assert.strictEqual(g[2][0].text, 'baz');
  });
});
