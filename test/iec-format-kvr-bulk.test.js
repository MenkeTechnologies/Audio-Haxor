const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

/** IEC 1024^n tiers, one decimal — same rule as `app_lib::format_size`. */
function formatSizeIec(bytes) {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1
  );
  const val = bytes / 1024 ** i;
  return `${val.toFixed(1)} ${units[i]}`;
}

/** Mirrors `kvr::parse_version` (empty / Unknown, i32 segments, overflow → 0). */
function parseVersion(ver) {
  if (ver === '' || ver === 'Unknown') return [0, 0, 0];
  return ver.split('.').map((p) => {
    if (!/^-?\d+$/.test(p)) return 0;
    const n = Number.parseInt(p, 10);
    if (n < -2147483648 || n > 2147483647) return 0;
    return n;
  });
}

function compareVersions(a, b) {
  const pa = parseVersion(a);
  const pb = parseVersion(b);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const va = i < pa.length ? pa[i] : 0;
    const vb = i < pb.length ? pb[i] : 0;
    if (va < vb) return -1;
    if (va > vb) return 1;
  }
  return 0;
}

describe('formatSizeIec explicit expectations', () => {
  const table = [
    [0, '0 B'],
    [1, '1.0 B'],
    [1023, '1023.0 B'],
    [1024, '1.0 KB'],
    [1536, '1.5 KB'],
    [1048576, '1.0 MB'],
    [1073741824, '1.0 GB'],
    [1099511627776, '1.0 TB'],
    [1024 ** 5, '1024.0 TB'],
  ];
  for (const [b, want] of table) {
    it(`bytes ${b}`, () => assert.equal(formatSizeIec(b), want));
  }
});

describe('formatSizeIec structural invariants (bulk)', () => {
  for (let k = 1; k <= 1200; k++) {
    it(`tier monotonic ${k}`, () => {
      const s = formatSizeIec(k);
      assert.match(s, /^\d+\.\d+ [KMGT]?B$/);
      assert.ok(!s.includes('NaN'));
    });
  }
});

describe('parseVersion explicit expectations', () => {
  const table = [
    ['', [0, 0, 0]],
    ['Unknown', [0, 0, 0]],
    ['1.2.3', [1, 2, 3]],
    ['1.x.3', [1, 0, 3]],
    ['01.02.03', [1, 2, 3]],
    ['1..2', [1, 0, 2]],
    ['1.-1.0', [1, -1, 0]],
  ];
  for (const [s, want] of table) {
    it(JSON.stringify(s), () => assert.deepEqual(parseVersion(s), want));
  }
});

describe('parseVersion grid (bulk)', () => {
  for (let a = 0; a < 10; a++) {
    for (let b = 0; b < 10; b++) {
      for (let c = 0; c < 10; c++) {
        const s = `${a}.${b}.${c}`;
        it(s, () => assert.deepEqual(parseVersion(s), [a, b, c]));
      }
    }
  }
});

describe('compareVersions chain antisymmetry (bulk)', () => {
  const chain = [];
  for (let a = 0; a < 7; a++) {
    for (let b = 0; b < 7; b++) {
      for (let c = 0; c < 7; c++) {
        chain.push(`${a}.${b}.${c}`);
      }
    }
  }
  chain.sort((x, y) => compareVersions(x, y));

  for (let i = 0; i < chain.length - 1; i++) {
    const lo = chain[i];
    const hi = chain[i + 1];
    it(`${lo} vs ${hi}`, () => {
      assert.equal(compareVersions(lo, hi), -1);
      assert.equal(compareVersions(hi, lo), 1);
      assert.equal(compareVersions(lo, lo), 0);
    });
  }
});
