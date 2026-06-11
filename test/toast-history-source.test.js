/**
 * Exercises the real frontend/js/toast-history.js pure helpers in Node via vm.
 *
 * Targets concrete bug classes, not mirrors:
 *   - _thFormatTime: zero-padding of single-digit hour/minute/second (00:09:05).
 *     A `${d.getHours()}` without padStart silently drifts to "0:9:5" — caught here.
 *   - _thRenderRows: HTML-escaping of an adversarial toast message (stored-XSS class)
 *     and the empty-history branch.
 *   - _thApplyFilter: case-insensitive substring match across BOTH message and type,
 *     newest-first ordering, and count-vs-total accounting.
 */
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

/** <div> stub so escapeHtml() (used via _thEsc) matches browser entity encoding. */
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

/**
 * Load toast-history.js into a fresh vm sandbox.
 * @param {Record<string, {value?: string, textContent?: string, innerHTML?: string}>} els
 *   getElementById lookup table for _thApplyFilter's DOM reads.
 * @param {Array<{type?: string, message: string, t: number}>} history window.__toastHistory.
 */
function loadToastHistory(els = {}, history = []) {
  const codePath = path.join(__dirname, '..', 'frontend', 'js', 'toast-history.js');
  const code = fs.readFileSync(codePath, 'utf8');
  const sandbox = {
    console,
    requestAnimationFrame: (cb) => {
      if (typeof cb === 'function') cb();
      return 0;
    },
    document: {
      createElement: () => createTextDiv(),
      getElementById: (id) => els[id] || null,
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
      body: { insertAdjacentHTML: () => {} },
    },
  };
  sandbox.window = sandbox;
  sandbox.window.__toastHistory = history;
  vm.createContext(sandbox);
  vm.runInContext(code, sandbox);
  return sandbox;
}

describe('toast-history.js pure helpers', () => {
  it('_thFormatTime zero-pads single-digit hour/minute/second', () => {
    const sb = loadToastHistory();
    // 09:05:03 local — every field is single-digit and MUST stay two-wide.
    const ms = new Date(2020, 0, 1, 9, 5, 3).getTime();
    assert.equal(sb._thFormatTime(ms), '09:05:03');
    // Midnight: hour 0 -> "00", not "0".
    const midnight = new Date(2020, 0, 1, 0, 0, 0).getTime();
    assert.equal(sb._thFormatTime(midnight), '00:00:00');
  });

  it('_thRenderRows escapes adversarial message HTML and renders empty state', () => {
    const sb = loadToastHistory();
    const html = sb._thRenderRows([{ type: 'error', message: '<img src=x onerror=alert(1)>', t: 0 }]);
    // Raw `<img` must never reach innerHTML — stored-XSS guard.
    assert.ok(!html.includes('<img src=x'), 'unescaped <img must not appear');
    assert.ok(html.includes('&lt;img src=x'), 'angle bracket must be entity-encoded');
    // Empty history takes the dedicated empty-state branch, not an empty rows string.
    assert.match(sb._thRenderRows([]), /th-empty/);
  });

  it('_thApplyFilter matches type and message case-insensitively, newest-first, count vs total', () => {
    const els = {
      toastHistorySearch: { value: 'ERR' },
      toastHistoryList: { innerHTML: '' },
      toastHistoryCount: { textContent: '' },
      toastHistoryTotal: { textContent: '' },
    };
    const sb = loadToastHistory(els, [
      { type: 'info', message: 'scan started', t: 1 },
      { type: 'error', message: 'disk full', t: 2 }, // matches via TYPE "error"
      { type: 'info', message: 'ERROR boom', t: 3 }, // matches via MESSAGE "ERROR"
    ]);
    sb._thApplyFilter();
    // 'ERR' hits both the error-typed row and the ERROR-message row -> 2.
    assert.equal(els.toastHistoryCount.textContent, '2');
    // Total counts unfiltered history, independent of the active query.
    assert.equal(els.toastHistoryTotal.textContent, '3');
    const out = els.toastHistoryList.innerHTML;
    // Newest entry (t:3) renders before the older match (t:2).
    assert.ok(out.indexOf('ERROR boom') < out.indexOf('disk full'), 'reverse-chronological order');
    // Non-matching row is excluded.
    assert.ok(!out.includes('scan started'), 'non-matching message must be filtered out');
  });
});
