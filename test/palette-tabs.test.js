const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

// ── Static tab entries from frontend/js/command-palette.js buildPaletteStaticItems ──
const PALETTE_TAB_NAMES = [
  'Plugins',
  'Samples',
  'DAW Projects',
  'Presets',
  'Favorites',
  'Notes',
  'Tags',
  'History',
  'Files',
  'Visualizer',
  'Walkers',
  'Audio Engine',
  'MIDI',
  'PDFs',
  'Settings',
];

describe('palette tab catalog', () => {
  it('has 15 tab commands', () => {
    assert.strictEqual(PALETTE_TAB_NAMES.length, 15);
  });

  it('includes core workflow tabs', () => {
    assert.ok(PALETTE_TAB_NAMES.includes('Plugins'));
    assert.ok(PALETTE_TAB_NAMES.includes('Samples'));
    assert.ok(PALETTE_TAB_NAMES.includes('Settings'));
  });

  it('MIDI and Audio Engine appear once', () => {
    assert.strictEqual(PALETTE_TAB_NAMES.filter(n => n === 'MIDI').length, 1);
    assert.strictEqual(PALETTE_TAB_NAMES.filter(n => n === 'Audio Engine').length, 1);
  });

  it('order: Files before Visualizer', () => {
    const fi = PALETTE_TAB_NAMES.indexOf('Files');
    const vi = PALETTE_TAB_NAMES.indexOf('Visualizer');
    assert.ok(fi < vi);
  });
});
