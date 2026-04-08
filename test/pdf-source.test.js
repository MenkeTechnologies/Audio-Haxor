/**
 * Loads utils + pdf.js; tests PDF table row HTML and table scaffold.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

describe('frontend/js/pdf.js (vm-loaded)', () => {
  let P;

  before(() => {
    P = loadFrontendScripts(['utils.js', 'pdf.js'], {
      showToast: () => {},
      toastFmt: (key) => key,
      rowBadges: () => '',
    });
  });

  it('buildPdfRow sets path/name data attributes and escape-sensitive cells', () => {
    const html = P.buildPdfRow({
      path: '/Docs/report & co/file.pdf',
      name: 'Report & Co',
      directory: '/Docs/report & co',
      sizeFormatted: '12 KB',
      modified: '2024-06-01',
      size: 12000,
    });
    assert.ok(html.includes('data-pdf-path'));
    assert.ok(html.includes('data-pdf-name="report &amp; co"'));
    assert.ok(html.includes('data-action="openPdfFile"'));
    assert.ok(html.includes('batch-cb'));
    assert.ok(html.includes('&amp;'), 'name should be HTML-escaped in cells');
  });

  it('buildPdfRow reflects batch selection', () => {
    P.batchSetForTabId('tabPdf').add('/x/a.pdf');
    const html = P.buildPdfRow({
      path: '/x/a.pdf',
      name: 'A',
      directory: '/x',
      sizeFormatted: '1 B',
      modified: 'd',
      size: 1,
    });
    assert.ok(html.includes('checked'));
    P.batchSetForTabId('tabPdf').delete('/x/a.pdf');
  });

  it('buildPdfTableHtml defines pdfTable, sort headers, and load-more tbody', () => {
    const html = P.buildPdfTableHtml();
    assert.ok(html.includes('id="pdfTable"'));
    assert.ok(html.includes('data-action="sortPdf"'));
    assert.ok(html.includes('id="pdfTableBody"'));
    assert.ok(html.includes('data-batch-action="toggleAll"'));
  });

  it('buildPdfRow escapes double quotes in data-pdf-name (lower-cased)', () => {
    const html = P.buildPdfRow({
      path: '/Docs/x.pdf',
      name: 'Say "Hello"',
      directory: '/Docs',
      sizeFormatted: '1 KB',
      modified: 'd',
      size: 1000,
    });
    assert.ok(html.includes('data-pdf-name="say &quot;hello&quot;"'));
  });

  it('patchPdfPagesCell finds td by decoded path (ampersands break CSS attribute selectors)', () => {
    const tricky = '/tmp/report & co/file.pdf';
    const td = {
      getAttribute(n) {
        return n === 'data-pdf-pages-cell' ? tricky : null;
      },
      innerHTML: '',
      textContent: '',
    };
    const tbody = {
      querySelectorAll(sel) {
        return sel === 'td[data-pdf-pages-cell]' ? [td] : [];
      },
    };
    const base = defaultDocument();
    const doc = {
      ...base,
      getElementById(id) {
        if (id === 'pdfTableBody') return tbody;
        return base.getElementById(id);
      },
    };
    const Px = loadFrontendScripts(['utils.js', 'pdf.js'], {
      document: doc,
      showToast: () => {},
      toastFmt: (k) => k,
      rowBadges: () => '',
    });
    assert.equal(typeof Px.patchPdfPagesCell, 'function');
    Px.patchPdfPagesCell(tricky, 42);
    assert.equal(td.textContent, '42');
  });
});
