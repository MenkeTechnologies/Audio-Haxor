/**
 * Real command-palette.js: filterPaletteItems empty-query gating and fuzzy + lazy plugin rows.
 */
const fs = require('fs');
const path = require('path');
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

const COMMAND_PALETTE_PATH = path.join(__dirname, '..', 'frontend', 'js', 'command-palette.js');

describe('frontend/js/command-palette.js collectPaletteItems (source)', () => {
  it('does not register the same appFmt key twice (duplicate palette rows)', () => {
    const src = fs.readFileSync(COMMAND_PALETTE_PATH, 'utf8');
    const m = src.match(/function collectPaletteItems\(\) \{[\s\S]*?\n\}\n\nfunction filterPaletteItems/);
    assert.ok(m, 'expected collectPaletteItems block before filterPaletteItems');
    const body = m[0];
    const keys = [];
    const re = /appFmt\('([^']+)'\)/g;
    let x;
    while ((x = re.exec(body)) !== null) keys.push(x[1]);
    const counts = new Map();
    for (const k of keys) counts.set(k, (counts.get(k) || 0) + 1);
    const dups = [...counts.entries()].filter(([, n]) => n > 1);
    assert.deepStrictEqual(
      dups,
      [],
      `duplicate appFmt keys in collectPaletteItems: ${dups.map(([k, n]) => `${k}×${n}`).join(', ')}`,
    );
  });
});

function loadPaletteSandbox(extra = {}) {
  return loadFrontendScripts(['utils.js', 'command-palette.js'], {
    appFmt: (k) => k,
    toastFmt: (k) => k,
    showToast: () => {},
    switchTab: () => {},
    scanPlugins: () => {},
    scanAudioSamples: () => {},
    scanDawProjects: () => {},
    scanPresets: () => {},
    checkUpdates: () => {},
    showDuplicateReport: () => {},
    resetAllScans: () => {},
    vstUpdater: {
      dbClearCaches: () => Promise.resolve(),
      clearLog: () => Promise.resolve(),
      getPrefsPath: () => Promise.resolve('/tmp/preferences.toml'),
      openPluginFolder: () => {},
      openWithApp: () => Promise.resolve(),
      openPrefsFile: () => {},
    },
    document: {
      ...defaultDocument(),
      getElementById: () => null,
      querySelector: () => null,
      querySelectorAll: () => [],
      body: { insertAdjacentHTML: () => {} },
      addEventListener: () => {},
    },
    ...extra,
  });
}

describe('frontend/js/command-palette.js filterPaletteItems (vm-loaded)', () => {
  let P;

  before(() => {
    P = loadPaletteSandbox({
      allPlugins: [
        { name: 'Serum', type: 'VST', manufacturer: 'Xfer', path: '/p' },
      ],
    });
  });

  it('empty query keeps only tab and action rows (drops bookmark/tag/etc.)', () => {
    const items = [
      { type: 'bookmark', name: 'Dir', fields: ['Dir', '/tmp'] },
      { type: 'tab', name: 'Plugins' },
      { type: 'action', name: 'Scan' },
      { type: 'tag', name: 'drums', fields: ['drums'] },
    ];
    const out = P.filterPaletteItems('', items);
    assert.strictEqual(out.length, 2);
    assert.strictEqual(out.map((i) => i.type).sort().join(','), 'action,tab');
  });

  it('non-empty query scores items by fuzzy name fields', () => {
    const items = [
      { type: 'action', name: 'Export plugins', fields: ['Export plugins'] },
      { type: 'action', name: 'Import samples', fields: ['Import samples'] },
    ];
    const out = P.filterPaletteItems('export', items);
    assert.strictEqual(out.length, 1);
    assert.strictEqual(out[0].name, 'Export plugins');
  });

  it('with query length >= 2, merges in matching plugins from allPlugins', () => {
    const items = [{ type: 'tab', name: 'Tabs only' }];
    const out = P.filterPaletteItems('ser', items);
    const pluginRow = out.find((i) => i.type === 'plugin');
    assert.ok(pluginRow);
    assert.strictEqual(pluginRow.name, 'Serum');
  });

  it('single-char query does not lazy-load plugins (length < 2)', () => {
    const items = [{ type: 'tab', name: 'Tabs' }];
    const out = P.filterPaletteItems('s', items);
    assert.strictEqual(out.find((i) => i.type === 'plugin'), undefined);
  });

});

describe('frontend/js/command-palette.js filterPaletteItems limits (no lazy plugin rows)', () => {
  let P;

  before(() => {
    P = loadPaletteSandbox({ allPlugins: [] });
  });

  it('truncates to PALETTE_MAX (50) when many rows match', () => {
    const items = Array.from({ length: 60 }, (_, i) => ({
      type: 'action',
      name: `Row${i}`,
      fields: [`match-${i}-token`],
    }));
    const out = P.filterPaletteItems('token', items);
    assert.strictEqual(out.length, 50);
  });
});
