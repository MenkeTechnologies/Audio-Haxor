/**
 * Real heatmap-dashboard.js: card builders aggregate samples/plugins/projects for HTML stats.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function loadHmSandbox(extra = {}) {
  return loadFrontendScripts(['utils.js', 'heatmap-dashboard.js'], {
    appFmt: (k, vars) => (vars ? `${k}:${JSON.stringify(vars)}` : k),
    document: {
      ...defaultDocument(),
      addEventListener: () => {},
    },
    requestAnimationFrame: (cb) => {
      if (typeof cb === 'function') cb();
      return 0;
    },
    ...extra,
  });
}

describe('frontend/js/heatmap-dashboard.js card builders (vm-loaded)', () => {
  let H;

  before(() => {
    H = loadHmSandbox();
  });

  it('buildFormatCard counts from samples when audioStatCounts is empty', () => {
    const html = H.buildFormatCard([
      { format: 'WAV' },
      { format: 'WAV' },
      { format: 'MP3' },
    ]);
    assert.ok(html.includes('WAV'));
    assert.ok(html.includes('MP3'));
    assert.ok(/hm-bar-val/.test(html));
  });

  it('buildFormatCard prefers global audioStatCounts when populated', () => {
    const S = loadHmSandbox({
      audioStatCounts: { FLAC: 99 },
    });
    const html = S.buildFormatCard([]);
    assert.ok(html.includes('FLAC'));
    assert.ok(html.includes('99'));
  });

  it('buildSizeCard assigns samples to size buckets', () => {
    const html = H.buildSizeCard([
      { size: 50 },
      { sizeBytes: 200 * 1024 },
      { size: 5 * 1024 * 1024 },
    ]);
    assert.ok(html.includes('ui.hm.bucket_lt_100kb'));
    assert.ok(html.includes('ui.hm.bucket_100kb_1mb'));
    assert.ok(html.includes('ui.hm.bucket_1_10mb'));
  });

  it('buildFolderCard groups by path prefix', () => {
    const html = H.buildFolderCard([
      { path: '/Samples/Drums/k.wav' },
      { path: '/Samples/Drums/s.wav' },
    ]);
    assert.ok(html.includes('Drums'));
  });

  it('buildPluginTypeCard and buildDawFormatCard show type breakdown', () => {
    const phtml = H.buildPluginTypeCard([
      { type: 'VST3' },
      { type: 'VST3' },
      { type: 'AU' },
    ]);
    assert.ok(phtml.includes('VST3'));
    assert.ok(phtml.includes('AU'));

    const dhtml = H.buildDawFormatCard([
      { daw: 'Live' },
      { format: 'ALS' },
    ]);
    assert.ok(dhtml.includes('Live'));
    assert.ok(dhtml.includes('ALS'));
  });

  it('buildBpmCard shows empty state when _bpmCache is absent', () => {
    const html = H.buildBpmCard();
    assert.ok(html.includes('ui.hm.card_bpm_empty'));
  });
});
