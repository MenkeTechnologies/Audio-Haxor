/**
 * Ensures `test/pure-data-helpers.test.js` XREF_FORMATS matches `frontend/js/xref.js`.
 */
const fs = require('fs');
const path = require('path');
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

function extractFormatList(sourceText) {
  const m = sourceText.match(/const XREF_FORMATS = new Set\(\[([\s\S]*?)\]\)/);
  if (!m) return null;
  return m[1]
    .split(',')
    .map((s) => s.trim().replace(/^['"]|['"]$/g, ''))
    .filter(Boolean)
    .sort();
}

describe('xref XREF_FORMATS sync', () => {
  it('pure-data-helpers mirrors xref.js format list', () => {
    const xrefSrc = fs.readFileSync(path.join(__dirname, '..', 'frontend', 'js', 'xref.js'), 'utf8');
    const testSrc = fs.readFileSync(path.join(__dirname, 'pure-data-helpers.test.js'), 'utf8');
    const a = extractFormatList(xrefSrc);
    const b = extractFormatList(testSrc);
    assert.ok(a && a.length > 0);
    assert.ok(b && b.length > 0);
    assert.deepStrictEqual(a, b);
  });
});
