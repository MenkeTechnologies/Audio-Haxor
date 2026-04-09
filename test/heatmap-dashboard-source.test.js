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

  it('buildKeyCard shows empty state when _keyCache absent', () => {
    const html = H.buildKeyCard();
    assert.ok(html.includes('ui.hm.card_key_empty'));
  });

  it('buildKeyCard renders canvas when _keyCache has values', () => {
    const S = loadHmSandbox({ _keyCache: { a: 'C', b: 'D' } });
    const html = S.buildKeyCard();
    assert.ok(html.includes('hmKeyCanvas'));
    assert.ok(html.includes('ui.hm.card_key_title_analyzed'));
  });

  it('buildTimelineCard returns empty string for no samples', () => {
    assert.strictEqual(H.buildTimelineCard([]), '');
  });

  it('buildTimelineCard includes canvas when samples exist', () => {
    const html = H.buildTimelineCard([{ path: '/a.wav' }]);
    assert.ok(html.includes('hmTimelineCanvas'));
    assert.ok(html.includes('data-hm-card="timeline"'));
  });

  it('buildPluginTypeCard returns empty string for no plugins and no DB aggregate', () => {
    assert.strictEqual(H.buildPluginTypeCard([]), '');
  });

  it('buildPluginTypeCard uses agg.plugins.byType when in-memory list is empty', () => {
    const html = H.buildPluginTypeCard([], {
      plugins: {count: 10, byType: {VST3: 7, AU: 3}},
    });
    assert.ok(html.includes('VST3'));
    assert.ok(html.includes('AU'));
  });

  it('buildDawFormatCard uses agg.daw.byType when project list is empty', () => {
    const html = H.buildDawFormatCard([], {
      daw: {count: 5, byType: {'Ableton Live': 3, 'Logic Pro': 2}},
    });
    assert.ok(html.includes('Ableton Live'));
    assert.ok(html.includes('Logic Pro'));
  });

  it('buildFolderCard shows hm-empty when samples array is empty', () => {
    const html = H.buildFolderCard([]);
    assert.ok(html.includes('ui.hm.empty_no_data'));
  });

  it('buildFolderCard uses DB topFolders when samples empty', () => {
    const html = H.buildFolderCard([], {
      audio: {
        count: 100,
        topFolders: [{path: '/Music/Samples', count: 42}, {path: '/Loops', count: 10}],
      },
    });
    assert.ok(html.includes('Samples'));
    assert.ok(html.includes('42'));
    assert.ok(!html.includes('ui.hm.empty_no_data'));
  });

  it('buildBpmCard uses bpmAnalyzedCount from aggregate when present', () => {
    const html = H.buildBpmCard({
      audio: {
        bpmAnalyzedCount: 500,
        bpmBuckets: new Array(34).fill(0).map((_, i) => (i === 10 ? 3 : 0)),
      },
    });
    assert.ok(html.includes('hmBpmCanvas'));
    assert.ok(html.includes('ui.hm.card_bpm_title_analyzed'));
    assert.ok(html.includes('500'));
  });
});
