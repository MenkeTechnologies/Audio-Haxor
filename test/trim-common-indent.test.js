const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

// ── Min indent helper edge cases ──
function minIndentLines(lines) {
  const nonempty = lines.filter(l => l.trim());
  if (nonempty.length === 0) return lines;
  let m = Infinity;
  for (const l of nonempty) {
    const indent = l.match(/^\s*/)[0].length;
    m = Math.min(m, indent);
  }
  return lines.map(l => {
    if (!l.trim()) return '';
    return l.slice(m);
  });
}

describe('minIndentLines', () => {
  it('strips common', () => {
    const out = minIndentLines(['    a', '    b', '']);
    assert.deepStrictEqual(out, ['a', 'b', '']);
  });

  it('strips 1 space common', () => {
    const lines = [' a', ' b', ' c'];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['a', 'b', 'c']);
  });

  it('preserves empty lines', () => {
    const lines = ['', '  abc', '', '  def', ''];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['', 'abc', '', 'def', '']);
  });

  it('handles all empty', () => {
    const lines = ['', '', ''];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['', '', '']);
  });

  it('handles single line with no indent', () => {
    const lines = ['abc'];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['abc']);
  });

  it('handles single line with indent', () => {
    const lines = ['    abc'];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['abc']);
  });

  it('handles tabs', () => {
    const lines = ['\tab', '\tb'];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['ab', 'b']);
  });

  it('mix of tabs and spaces', () => {
    const lines = ['\t abc', '\t abc', ''];
    const out = minIndentLines(lines);
    // Common leading whitespace is tab + space (length 2).
    assert.deepStrictEqual(out, ['abc', 'abc', '']);
  });

  it('preserves leading empty lines', () => {
    const lines = ['', '', '   abc'];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['', '', 'abc']);
  });

  it('handles lines with only whitespace', () => {
    const lines = ['     ', '\t\t', '   abc'];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['', '', 'abc']);
  });

  it('handles unicode whitespace', () => {
    const lines = ['\u3000abc', '\u3000abc', ''];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['abc', 'abc', '']);
  });

  it('only blank lines returns unchanged', () => {
    const lines = ['', '   ', '\t'];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, lines);
  });

  it('strips minimum when indents differ', () => {
    const lines = ['  a', '    b', '  c'];
    const out = minIndentLines(lines);
    assert.deepStrictEqual(out, ['a', '  b', 'c']);
  });
});