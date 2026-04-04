/**
 * Loads utils + pdf.js; tests PDF table row HTML and table scaffold.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

describe('frontend/js/pdf.js (vm-loaded)', () => {
  let P;

  before(() => {
    P = loadFrontendScripts(['utils.js', 'pdf.js'], {
      batchSelected: new Set(),
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
    P.batchSelected.add('/x/a.pdf');
    const html = P.buildPdfRow({
      path: '/x/a.pdf',
      name: 'A',
      directory: '/x',
      sizeFormatted: '1 B',
      modified: 'd',
      size: 1,
    });
    assert.ok(html.includes('checked'));
    P.batchSelected.delete('/x/a.pdf');
  });

  it('buildPdfTableHtml defines pdfTable, sort headers, and load-more tbody', () => {
    const html = P.buildPdfTableHtml();
    assert.ok(html.includes('id="pdfTable"'));
    assert.ok(html.includes('data-action="sortPdf"'));
    assert.ok(html.includes('id="pdfTableBody"'));
    assert.ok(html.includes('data-batch-action="toggleAll"'));
  });
});
