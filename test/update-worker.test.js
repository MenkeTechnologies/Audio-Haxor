const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

// Test version comparison and parsing logic used by the update worker

describe('version utils', () => {
  function parseVersion(ver) {
    if (!ver || ver === 'Unknown') return [0, 0, 0];
    return ver.split('.').map(n => parseInt(n, 10) || 0);
  }

  function compareVersions(a, b) {
    const pa = parseVersion(a);
    const pb = parseVersion(b);
    const len = Math.max(pa.length, pb.length);
    for (let i = 0; i < len; i++) {
      const diff = (pa[i] || 0) - (pb[i] || 0);
      if (diff !== 0) return diff;
    }
    return 0;
  }

  describe('parseVersion', () => {
    it('parses standard version', () => {
      assert.deepStrictEqual(parseVersion('1.2.3'), [1, 2, 3]);
    });

    it('parses two-part version', () => {
      assert.deepStrictEqual(parseVersion('3.5'), [3, 5]);
    });

    it('parses four-part version', () => {
      assert.deepStrictEqual(parseVersion('1.2.3.4'), [1, 2, 3, 4]);
    });

    it('returns [0,0,0] for Unknown', () => {
      assert.deepStrictEqual(parseVersion('Unknown'), [0, 0, 0]);
    });

    it('returns [0,0,0] for null', () => {
      assert.deepStrictEqual(parseVersion(null), [0, 0, 0]);
    });
  });

  describe('compareVersions', () => {
    it('equal versions return 0', () => {
      assert.strictEqual(compareVersions('1.2.3', '1.2.3'), 0);
    });

    it('higher major is positive', () => {
      assert.ok(compareVersions('2.0.0', '1.0.0') > 0);
    });

    it('lower major is negative', () => {
      assert.ok(compareVersions('1.0.0', '2.0.0') < 0);
    });

    it('higher minor is positive', () => {
      assert.ok(compareVersions('1.3.0', '1.2.0') > 0);
    });

    it('higher patch is positive', () => {
      assert.ok(compareVersions('1.2.4', '1.2.3') > 0);
    });

    it('handles different length versions', () => {
      assert.ok(compareVersions('1.2.3.1', '1.2.3') > 0);
      assert.strictEqual(compareVersions('1.2.3.0', '1.2.3'), 0);
    });

    it('handles Unknown as 0.0.0', () => {
      assert.ok(compareVersions('1.0.0', 'Unknown') > 0);
      assert.ok(compareVersions('Unknown', '1.0.0') < 0);
    });
  });
});

describe('KVR URL builder', () => {
  function slugify(str) {
    return str.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
  }

  function buildKvrUrl(name, manufacturer) {
    const slug = slugify(name);
    if (manufacturer && manufacturer !== 'Unknown') {
      return `https://www.kvraudio.com/product/${slug}-by-${slugify(manufacturer)}`;
    }
    return `https://www.kvraudio.com/product/${slug}`;
  }

  it('builds URL with manufacturer', () => {
    assert.strictEqual(
      buildKvrUrl('Pro-Q 3', 'FabFilter'),
      'https://www.kvraudio.com/product/pro-q-3-by-fabfilter'
    );
  });

  it('builds URL for ADSR Sample Manager', () => {
    assert.strictEqual(
      buildKvrUrl('ADSR Sample Manager', 'ADSR'),
      'https://www.kvraudio.com/product/adsr-sample-manager-by-adsr'
    );
  });

  it('builds URL for 2RuleSynth', () => {
    assert.strictEqual(
      buildKvrUrl('2RuleSynth', '2Rule'),
      'https://www.kvraudio.com/product/2rulesynth-by-2rule'
    );
  });

  it('builds URL without manufacturer', () => {
    assert.strictEqual(
      buildKvrUrl('SomePlugin', 'Unknown'),
      'https://www.kvraudio.com/product/someplugin'
    );
  });

  it('handles special characters', () => {
    assert.strictEqual(
      buildKvrUrl('Plugin (v2) [Beta]', 'My Co.'),
      'https://www.kvraudio.com/product/plugin-v2-beta-by-my-co'
    );
  });
});

describe('parseVersion edge cases', () => {
  function parseVersion(ver) {
    if (!ver || ver === 'Unknown') return [0, 0, 0];
    return ver.split('.').map(n => parseInt(n, 10) || 0);
  }

  it('returns [0,0,0] for empty string', () => {
    assert.deepStrictEqual(parseVersion(''), [0, 0, 0]);
  });

  it('handles version with text', () => {
    assert.deepStrictEqual(parseVersion('1.2.beta'), [1, 2, 0]);
  });

  it('handles single number', () => {
    assert.deepStrictEqual(parseVersion('5'), [5]);
  });

  it('handles leading zeros', () => {
    assert.deepStrictEqual(parseVersion('01.02.03'), [1, 2, 3]);
  });
});

describe('compareVersions edge cases', () => {
  function parseVersion(ver) {
    if (!ver || ver === 'Unknown') return [0, 0, 0];
    return ver.split('.').map(n => parseInt(n, 10) || 0);
  }

  function compareVersions(a, b) {
    const pa = parseVersion(a);
    const pb = parseVersion(b);
    const len = Math.max(pa.length, pb.length);
    for (let i = 0; i < len; i++) {
      const diff = (pa[i] || 0) - (pb[i] || 0);
      if (diff !== 0) return diff;
    }
    return 0;
  }

  it('both Unknown returns 0', () => {
    assert.strictEqual(compareVersions('Unknown', 'Unknown'), 0);
  });

  it('both null returns 0', () => {
    assert.strictEqual(compareVersions(null, null), 0);
  });

  it('four-part vs three-part', () => {
    assert.ok(compareVersions('1.0.0.1', '1.0.0') > 0);
  });

  it('same major.minor, different patch', () => {
    assert.ok(compareVersions('2.1.5', '2.1.3') > 0);
    assert.ok(compareVersions('2.1.3', '2.1.5') < 0);
  });

  it('handles very large version numbers', () => {
    assert.ok(compareVersions('100.200.300', '100.200.299') > 0);
  });

  it('identical versions return 0', () => {
    assert.strictEqual(compareVersions('3.5.1', '3.5.1'), 0);
    assert.strictEqual(compareVersions('0.0.1', '0.0.1'), 0);
  });

  it('very long version numbers like 1.2.3.4.5', () => {
    assert.ok(compareVersions('1.2.3.4.5', '1.2.3.4.4') > 0);
    assert.ok(compareVersions('1.2.3.4.5', '1.2.3.4.6') < 0);
    assert.strictEqual(compareVersions('1.2.3.4.5', '1.2.3.4.5'), 0);
  });

  it('version with leading "v" prefix', () => {
    // parseVersion splits on "." and parseInt("v1") yields NaN → 0
    // so "v1.2.3" becomes [0, 2, 3] which is less than [1, 2, 3]
    assert.ok(compareVersions('v1.2.3', '1.2.3') < 0);
  });
});

describe('parseVersion pre-release strings', () => {
  function parseVersion(ver) {
    if (!ver || ver === 'Unknown') return [0, 0, 0];
    return ver.split('.').map(n => parseInt(n, 10) || 0);
  }

  it('parses 1.2.3-beta (hyphen suffix stripped by parseInt)', () => {
    // parseInt("3-beta") returns 3
    assert.deepStrictEqual(parseVersion('1.2.3-beta'), [1, 2, 3]);
  });

  it('parses 1.0.0-rc.1 (rc segment becomes 0)', () => {
    // "1.0.0-rc.1" splits into ["1","0","0-rc","1"]
    // parseInt("0-rc") = 0
    assert.deepStrictEqual(parseVersion('1.0.0-rc.1'), [1, 0, 0, 1]);
  });

  it('parses version with leading v prefix', () => {
    // parseInt("v1") returns NaN → 0
    assert.deepStrictEqual(parseVersion('v1.2.3'), [0, 2, 3]);
  });
});
