/**
 * Real duplicates.js: findDuplicates groups items by keyFn and drops singletons.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

describe('frontend/js/duplicates.js findDuplicates (vm-loaded)', () => {
  let D;

  before(() => {
    D = loadFrontendScripts(['utils.js', 'duplicates.js'], {
      showToast: () => {},
      toastFmt: (k) => k,
    });
  });

  it('returns empty when every key is unique', () => {
    const groups = D.findDuplicates(
      [{ id: 1 }, { id: 2 }, { id: 3 }],
      (x) => String(x.id),
    );
    assert.strictEqual(groups.length, 0);
  });

  it('returns one group when two items share a key', () => {
    const groups = D.findDuplicates(
      [{ n: 'A' }, { n: 'B' }, { n: 'a' }],
      (x) => x.n.toLowerCase(),
    );
    assert.strictEqual(groups.length, 1);
    assert.strictEqual(groups[0].length, 2);
    assert.strictEqual(groups[0].map((x) => x.n).sort().join(','), 'A,a');
  });

  it('returns multiple groups when multiple keys collide', () => {
    const groups = D.findDuplicates(
      [1, 2, 1, 3, 2, 4].map((n) => ({ v: n })),
      (x) => String(x.v),
    );
    assert.strictEqual(groups.length, 2);
    const sizes = groups.map((g) => g.length).sort((a, b) => a - b);
    assert.strictEqual(sizes.join(','), '2,2');
  });
});
