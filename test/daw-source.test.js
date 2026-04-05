/**
 * Loads utils + xref + daw.js; tests badge CSS class, DAW row HTML, xref button state.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const vm = require('vm');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

describe('frontend/js/daw.js (vm-loaded)', () => {
  let D;

  before(() => {
    D = loadFrontendScripts(['utils.js', 'xref.js', 'daw.js'], {
      batchSelected: new Set(),
      _lastDawSearch: '',
      _lastDawMode: 'fuzzy',
      rowBadges: () => '',
    });
  });

  it('getDawBadgeClass lowercases and hyphenates DAW names for CSS', () => {
    assert.strictEqual(D.getDawBadgeClass('Ableton Live'), 'daw-ableton-live');
    assert.strictEqual(D.getDawBadgeClass('FL Studio'), 'daw-fl-studio');
    assert.strictEqual(D.getDawBadgeClass('REAPER'), 'daw-reaper');
  });

  it('getDawBadgeClass collapses multiple spaces in the DAW label', () => {
    assert.strictEqual(D.getDawBadgeClass('Bitwig  Studio'), 'daw-bitwig-studio');
  });

  it('buildDawRow embeds search and path data attributes for scan-time filtering', () => {
    const html = D.buildDawRow({
      path: '/Music/beat.als',
      name: 'My Beat',
      daw: 'Ableton Live',
      format: 'ALS',
      sizeFormatted: '2 MB',
      modified: '2024-06-01',
      directory: '/Music',
    });
    assert.ok(html.includes('data-daw-path'));
    assert.ok(html.includes('data-daw-search="my beat"'));
    assert.ok(html.includes('daw-ableton-live'));
    assert.ok(html.includes('batch-cb'));
  });

  it('buildDawRow reflects batch selection and xref cache hits', () => {
    vm.runInContext(
      '_xrefCache["/proj/session.rpp"] = [{ name: "ReaComp", pluginType: "VST3", manufacturer: "Cockos" }]',
      D
    );
    D.batchSelected.add('/proj/session.rpp');
    const html = D.buildDawRow({
      path: '/proj/session.rpp',
      name: 'Mix',
      daw: 'REAPER',
      format: 'RPP',
      sizeFormatted: '12 KB',
      modified: 't',
      directory: '/proj',
    });
    assert.ok(html.includes('checked'), 'checkbox checked when path in batchSelected');
    assert.ok(html.includes('has-plugins'), 'cached xref adds has-plugins class');
    assert.ok(html.includes('&#9889; 1'), 'button shows plugin count');
  });

  it('buildDawRow omits xref button when project format is not xref-supported', () => {
    const html = D.buildDawRow({
      path: '/tmp/x.wav',
      name: 'Bounce',
      daw: 'Export',
      format: 'WAV',
      sizeFormatted: '1 MB',
      modified: 'd',
      directory: '/tmp',
    });
    assert.ok(!html.includes('data-action="showXref"'));
    assert.ok(html.includes('format-default'));
  });
});
