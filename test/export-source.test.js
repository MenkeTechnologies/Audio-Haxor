/**
 * Loads real export.js — export modal title resolution and format/filter definitions.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

function loadExportSandbox() {
  const sandbox = {
    console,
    appFmt: (key, vars) => (vars ? `${key}:${JSON.stringify(vars)}` : key),
    document: { addEventListener: () => {} },
    showToast: () => {},
    window: {},
  };
  sandbox.window = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(
    fs.readFileSync(path.join(__dirname, '..', 'frontend', 'js', 'export.js'), 'utf8'),
    sandbox
  );
  return sandbox;
}

describe('frontend/js/export.js (vm-loaded)', () => {
  let E;

  before(() => {
    E = loadExportSandbox();
  });

  it('resolveExportTitle prefers titleKey + vars over raw title', () => {
    assert.strictEqual(
      E.resolveExportTitle({ titleKey: 'ui.export.title', titleVars: { n: 3 } }),
      'ui.export.title:{"n":3}'
    );
    assert.strictEqual(E.resolveExportTitle({ title: 'Plain' }), 'Plain');
    assert.strictEqual(E.resolveExportTitle(null), '');
  });

  it('getExportFormatOptions lists json first and unique ids', () => {
    const opts = E.getExportFormatOptions();
    assert.strictEqual(opts[0].id, 'json');
    const ids = opts.map((o) => o.id);
    assert.strictEqual(new Set(ids).size, ids.length);
    assert.ok(opts.every((o) => o.label && o.ext));
  });

  it('getAllImportFilters covers json+toml then single-format filters', () => {
    const filters = E.getAllImportFilters();
    assert.strictEqual(filters[0].extensions.join(','), 'json,toml');
    assert.strictEqual(filters[1].extensions.join(','), 'json');
    assert.strictEqual(filters[2].extensions.join(','), 'toml');
  });

  it('exportFileName uses audiohaxor prefix and ISO-like timestamp', () => {
    const name = E.exportFileName('plugins');
    assert.ok(name.startsWith('audiohaxor-plugins-'));
    assert.ok(!name.includes('T'));
    assert.ok(!name.includes(':'));
  });

  it('pdfHeaders maps each key through appFmt for PDF table columns', () => {
    const two = E.pdfHeaders('a', 'b');
    assert.strictEqual(two.length, 2);
    assert.strictEqual(two[0], 'a');
    assert.strictEqual(two[1], 'b');
    const one = E.pdfHeaders('ui.export.col_name');
    assert.strictEqual(one.length, 1);
    assert.strictEqual(one[0], 'ui.export.col_name');
  });

  it('EXPORT_FORMAT_DEFS covers tabular and pdf export ids', () => {
    const opts = E.getExportFormatOptions();
    const ids = new Set(opts.map((o) => o.id));
    for (const id of ['json', 'toml', 'csv', 'tsv', 'pdf']) {
      assert.ok(ids.has(id), `missing format ${id}`);
    }
  });
});

