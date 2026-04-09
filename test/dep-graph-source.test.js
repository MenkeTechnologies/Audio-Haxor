/**
 * Loads real frontend/js/utils.js + dep-graph.js and validates buildDepGraphData(),
 * which drives the plugin↔DAW dependency analytics tab.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');
const { createTextDiv } = require('./frontend-vm-harness.js');

function loadDepGraphSandbox() {
  const escEl = { textContent: '', innerHTML: '' };
  const sandbox = {
    console,
    performance: { now: () => 0 },
    requestAnimationFrame: () => 0,
    KVR_MANUFACTURER_MAP: {},
    prefs: {
      getObject: () => null,
      getItem: () => null,
      setItem: () => {},
      removeItem: () => {},
    },
    document: {
      /** `utils.js` `escapeHtml` uses a real-ish div (`textContent` → `innerHTML` entity encoding). */
      createElement: (tag) => (tag === 'div' ? createTextDiv() : { ...escEl }),
      getElementById: () => null,
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
      body: { insertAdjacentHTML: () => {} },
    },
    setTimeout: (fn) => {
      if (typeof fn === 'function') fn();
      return 0;
    },
    clearTimeout: () => {},
  };
  sandbox.window = sandbox;
  vm.createContext(sandbox);
  const root = path.join(__dirname, '..', 'frontend', 'js');
  vm.runInContext(fs.readFileSync(path.join(root, 'utils.js'), 'utf8'), sandbox);
  // Mirror xref.js plugin-key helpers so buildDepGraphData matches the live app when normalizedName is absent.
  vm.runInContext(
    `function normalizePluginName(name) {
  let s = name.trim();
  const bracketRe = /\\s*[\\(\\[](x64|x86_64|x86|arm64|aarch64|64-?bit|32-?bit|intel|apple silicon|universal|stereo|mono|vst3?|au|aax)[\\)\\]]$/i;
  let prev;
  do { prev = s; s = s.replace(bracketRe, ''); } while (s !== prev);
  s = s.replace(/\\s+(x64|x86_64|x86|64bit|32bit)$/i, '');
  return s.replace(/\\s+/g, ' ').trim().toLowerCase();
}
function xrefPluginRefKey(p) {
  if (p.normalizedName) return p.normalizedName;
  return normalizePluginName(p.name);
}
function xrefProjectFromPath(path) {
  const name = path.split('/').pop() || path;
  const directory = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
  return { name, path, daw: '—', format: '', directory };
}`,
    sandbox
  );
  vm.runInContext(fs.readFileSync(path.join(root, 'dep-graph.js'), 'utf8'), sandbox);
  return sandbox;
}

describe('frontend/js/dep-graph.js buildDepGraphData (vm-loaded)', () => {
  let G;

  before(() => {
    G = loadDepGraphSandbox();
  });

  it('aggregates xref plugins by normalized key and project count', () => {
    G.allDawProjects = [
      { path: '/p/one.als', name: 'One', daw: 'Ableton Live', format: 'ALS' },
      { path: '/p/two.als', name: 'Two', daw: 'Ableton Live', format: 'ALS' },
    ];
    G._xrefCache = {
      '/p/one.als': [
        {
          name: 'Serum',
          normalizedName: 'serum',
          manufacturer: 'Xfer',
          pluginType: 'VST3',
        },
      ],
      '/p/two.als': [
        {
          name: 'Serum',
          normalizedName: 'serum',
          manufacturer: 'Xfer',
          pluginType: 'VST3',
        },
      ],
    };
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.totalProjects, 2);
    const serum = d.pluginsByUsage.find((p) => p.key === 'serum');
    assert.ok(serum);
    assert.strictEqual(serum.count, 2);
    assert.strictEqual(serum.projects.size, 2);
    assert.strictEqual(d.projectsByCount.length, 2);
  });

  it('marks installed plugins not referenced in any xref project as orphaned', () => {
    G.allDawProjects = [{ path: '/p/a.als', name: 'A', daw: 'Ableton Live', format: 'ALS' }];
    G._xrefCache = {
      '/p/a.als': [
        {
          name: 'Serum',
          normalizedName: 'serum',
          manufacturer: 'Xfer',
          pluginType: 'VST3',
        },
      ],
    };
    G.allPlugins = [{ name: 'Orphan FX', path: '/x.vst3', type: 'VST3' }];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.orphaned.length, 1);
    assert.strictEqual(d.orphaned[0].name, 'Orphan FX');
  });

  it('xref rows without normalizedName still match installed plugins via normalizePluginName (orphan accuracy)', () => {
    G.allDawProjects = [];
    G._xrefCache = {
      '/p/a.als': [{ name: 'Serum (x64)', manufacturer: 'Xfer', pluginType: 'VST3' }],
    };
    G.allPlugins = [{ name: 'Serum', path: '/lib/Serum.vst3', type: 'VST3' }];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.orphaned.length, 0);
  });

  it('sorts projects by plugin count descending', () => {
    G.allDawProjects = [
      { path: '/p/heavy.rpp', name: 'Heavy', daw: 'REAPER', format: 'RPP' },
      { path: '/p/light.rpp', name: 'Light', daw: 'REAPER', format: 'RPP' },
    ];
    G._xrefCache = {
      '/p/heavy.rpp': [
        { name: 'A', normalizedName: 'a', manufacturer: 'M', pluginType: 'VST3' },
        { name: 'B', normalizedName: 'b', manufacturer: 'M', pluginType: 'VST3' },
        { name: 'C', normalizedName: 'c', manufacturer: 'M', pluginType: 'VST3' },
      ],
      '/p/light.rpp': [
        { name: 'A', normalizedName: 'a', manufacturer: 'M', pluginType: 'VST3' },
      ],
    };
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.projectsByCount[0].path, '/p/heavy.rpp');
    assert.strictEqual(d.projectsByCount[0].count, 3);
    assert.strictEqual(d.projectsByCount[1].count, 1);
  });

  it('empty xref cache and no projects yields zero totals', () => {
    G.allDawProjects = [];
    G._xrefCache = {};
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.totalProjects, 0);
    assert.strictEqual(d.pluginsByUsage.length, 0);
    assert.strictEqual(d.projectsByCount.length, 0);
    assert.strictEqual(d.orphaned.length, 0);
  });

  it('xref entries without matching DAW project row use path fallback (paginated DAW tab)', () => {
    G.allDawProjects = [];
    G._xrefCache = {
      '/ghost/path.als': [
        { name: 'Ghost', normalizedName: 'ghost', manufacturer: 'X', pluginType: 'VST3' },
      ],
    };
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.totalProjects, 1);
    assert.strictEqual(d.pluginsByUsage.length, 1);
    assert.strictEqual(d.pluginsByUsage[0].name, 'Ghost');
    assert.strictEqual(d.projectsByCount[0].name, 'path.als');
  });

  it('returns empty orphaned list when allPlugins is undefined (guard)', () => {
    G.allDawProjects = [{ path: '/p/a.als', name: 'A', daw: 'Live', format: 'ALS' }];
    G._xrefCache = {
      '/p/a.als': [
        { name: 'Used', normalizedName: 'used', manufacturer: 'M', pluginType: 'VST3' },
      ],
    };
    G.allPlugins = undefined;
    const d = G.buildDepGraphData();
    assert.strictEqual(d.orphaned.length, 0);
  });

  it('buildAnalyticsHtml shows empty-state when no plugin usage rows', () => {
    const html = G.buildAnalyticsHtml({
      pluginsByUsage: [],
      projectsByCount: [],
      orphaned: [],
      totalProjects: 0,
    });
    assert.ok(html.includes('dep-empty'));
    assert.ok(html.includes('No data to analyze'));
  });

  it('dedupes the same plugin key multiple times in one project (Set per project path)', () => {
    G.allDawProjects = [{ path: '/p/a.als', name: 'A', daw: 'Live', format: 'ALS' }];
    G._xrefCache = {
      '/p/a.als': [
        { name: 'Serum', normalizedName: 'serum', manufacturer: 'Xfer', pluginType: 'VST3' },
        { name: 'Serum', normalizedName: 'serum', manufacturer: 'Xfer', pluginType: 'VST3' },
      ],
    };
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    const serum = d.pluginsByUsage.find((p) => p.key === 'serum');
    assert.ok(serum);
    assert.strictEqual(serum.count, 1);
    assert.strictEqual(serum.projects.size, 1);
  });

  it('counts projects with empty plugin arrays toward totalProjects and zero-plugin rows', () => {
    G.allDawProjects = [];
    G._xrefCache = {
      '/empty.als': [],
      '/one.als': [
        { name: 'X', normalizedName: 'x', manufacturer: 'M', pluginType: 'VST3' },
      ],
    };
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.totalProjects, 2);
    const emptyRow = d.projectsByCount.find((p) => p.path === '/empty.als');
    assert.ok(emptyRow);
    assert.strictEqual(emptyRow.count, 0);
    assert.strictEqual(d.pluginsByUsage.length, 1);
  });

  it('sorts pluginsByUsage by project count descending', () => {
    G.allDawProjects = [
      { path: '/p/a.als', name: 'A', daw: 'Live', format: 'ALS' },
      { path: '/p/b.als', name: 'B', daw: 'Live', format: 'ALS' },
      { path: '/p/c.als', name: 'C', daw: 'Live', format: 'ALS' },
    ];
    G._xrefCache = {
      '/p/a.als': [
        { name: 'Often', normalizedName: 'often', manufacturer: 'M', pluginType: 'VST3' },
        { name: 'Once', normalizedName: 'once', manufacturer: 'M', pluginType: 'VST3' },
      ],
      '/p/b.als': [{ name: 'Often', normalizedName: 'often', manufacturer: 'M', pluginType: 'VST3' }],
      '/p/c.als': [{ name: 'Often', normalizedName: 'often', manufacturer: 'M', pluginType: 'VST3' }],
    };
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.pluginsByUsage[0].key, 'often');
    assert.strictEqual(d.pluginsByUsage[0].count, 3);
    assert.strictEqual(d.pluginsByUsage[1].key, 'once');
    assert.strictEqual(d.pluginsByUsage[1].count, 1);
  });

  it('sum of plugin usage counts equals per-project plugin list lengths', () => {
    G.allDawProjects = [];
    G._xrefCache = {
      '/p/1.als': [
        { name: 'A', normalizedName: 'a', manufacturer: 'M', pluginType: 'VST3' },
        { name: 'B', normalizedName: 'b', manufacturer: 'M', pluginType: 'AU' },
      ],
      '/p/2.als': [{ name: 'A', normalizedName: 'a', manufacturer: 'M', pluginType: 'VST3' }],
    };
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    const totalRefs = d.pluginsByUsage.reduce((s, p) => s + p.count, 0);
    assert.strictEqual(totalRefs, 3);
    const perProject = d.projectsByCount.reduce((s, p) => s + p.count, 0);
    assert.strictEqual(perProject, 3);
  });

  it('buildAnalyticsHtml includes avg, format breakdown, and key insight numbers', () => {
    const html = G.buildAnalyticsHtml({
      pluginsByUsage: [
        {
          key: 'a',
          name: 'PlugA',
          type: 'VST3',
          manufacturer: 'Acme',
          count: 2,
          projects: new Set(['/p/1', '/p/2']),
        },
        {
          key: 'b',
          name: 'PlugB',
          type: 'AU',
          manufacturer: 'Beta',
          count: 1,
          projects: new Set(['/p/1']),
        },
      ],
      projectsByCount: [
        { path: '/p/1', name: 'One', daw: 'ALS', count: 2, plugins: [] },
        { path: '/p/2', name: 'Two', daw: 'ALS', count: 1, plugins: [] },
      ],
      orphaned: [{ name: 'Unused', path: '/u.vst3', type: 'VST3' }],
      totalProjects: 2,
    });
    assert.ok(html.includes('Plugin Format Breakdown'));
    assert.ok(html.includes('Top Manufacturers'));
    assert.ok(html.includes('Key Insights'));
    assert.ok(
      html.includes(
        '<span class="dep-insight-val">1.5</span><span class="dep-insight-label">Avg plugins per project</span>'
      )
    );
    assert.ok(
      html.includes(
        '<span class="dep-insight-val">1</span><span class="dep-insight-label">Unused installed plugins</span>'
      )
    );
    assert.ok(
      html.includes(
        '<span class="dep-insight-val">1</span><span class="dep-insight-label">Single-use plugins</span>'
      )
    );
  });

  it('buildAnalyticsHtml lists go-to plugins when used in more than half of projects', () => {
    const html = G.buildAnalyticsHtml({
      pluginsByUsage: [
        {
          key: 'core',
          name: 'CoreSynth',
          type: 'VST3',
          manufacturer: 'M',
          count: 3,
          projects: new Set(['/a', '/b', '/c']),
        },
        {
          key: 'rare',
          name: 'Rare',
          type: 'AU',
          manufacturer: 'M',
          count: 1,
          projects: new Set(['/a']),
        },
      ],
      projectsByCount: [
        { path: '/a', name: 'A', daw: 'RPP', count: 2, plugins: [] },
        { path: '/b', name: 'B', daw: 'RPP', count: 1, plugins: [] },
        { path: '/c', name: 'C', daw: 'RPP', count: 1, plugins: [] },
      ],
      orphaned: [],
      totalProjects: 3,
    });
    assert.ok(html.includes('Your Go-To Plugins'));
    assert.ok(html.includes('CoreSynth'));
    assert.ok(html.includes('3/3'));
  });

  it('buildAnalyticsHtml uses singular "plugin" on the minimal-project line when count is 1', () => {
    const html = G.buildAnalyticsHtml({
      pluginsByUsage: [
        { key: 'x', name: 'X', type: 'VST3', manufacturer: 'M', count: 1, projects: new Set(['/a']) },
      ],
      projectsByCount: [{ path: '/a', name: 'Only', daw: 'ALS', count: 1, plugins: [] }],
      orphaned: [],
      totalProjects: 1,
    });
    // "Most complex" always prints the word "plugins" in `dep-graph.js`; "Most minimal" pluralizes correctly.
    assert.ok(html.includes('Most minimal:') && html.includes('Only (1 plugin)'));
  });

  it('buildAnalyticsHtml escapes HTML in manufacturer names (Top Manufacturers)', () => {
    const html = G.buildAnalyticsHtml({
      pluginsByUsage: [
        {
          key: 'a',
          name: 'P',
          type: 'VST3',
          manufacturer: 'Evil<script>',
          count: 1,
          projects: new Set(['/p']),
        },
      ],
      projectsByCount: [{ path: '/p', name: 'Proj', daw: 'ALS', count: 1, plugins: [] }],
      orphaned: [],
      totalProjects: 1,
    });
    assert.ok(html.includes('Evil&lt;script&gt;'));
    assert.ok(!html.includes('<script>'));
  });

  it('buildAnalyticsHtml omits go-to section when no plugin exceeds half of projects', () => {
    const html = G.buildAnalyticsHtml({
      pluginsByUsage: [
        {
          key: 'a',
          name: 'Half',
          type: 'VST3',
          manufacturer: 'M',
          count: 2,
          projects: new Set(['/p/1', '/p/2']),
        },
        {
          key: 'b',
          name: 'Other',
          type: 'AU',
          manufacturer: 'M',
          count: 2,
          projects: new Set(['/p/3', '/p/4']),
        },
      ],
      projectsByCount: [
        { path: '/p/1', name: 'A', daw: 'ALS', count: 1, plugins: [] },
        { path: '/p/2', name: 'B', daw: 'ALS', count: 1, plugins: [] },
        { path: '/p/3', name: 'C', daw: 'ALS', count: 1, plugins: [] },
        { path: '/p/4', name: 'D', daw: 'ALS', count: 1, plugins: [] },
      ],
      orphaned: [],
      totalProjects: 4,
    });
    assert.ok(!html.includes('Your Go-To Plugins'));
  });

  it('buildAnalyticsHtml labels missing pluginType as Unknown in format breakdown', () => {
    const html = G.buildAnalyticsHtml({
      pluginsByUsage: [
        {
          key: 'x',
          name: 'NoType',
          type: undefined,
          manufacturer: 'M',
          count: 1,
          projects: new Set(['/a']),
        },
      ],
      projectsByCount: [{ path: '/a', name: 'P', daw: 'ALS', count: 1, plugins: [] }],
      orphaned: [],
      totalProjects: 1,
    });
    assert.ok(html.includes('>Unknown</span>'));
    assert.ok(html.includes('xref-type-unknown'));
  });

  it('buildAnalyticsHtml truncates single-use list with a count when more than 30', () => {
    const singleUse = Array.from({ length: 31 }, (_, i) => ({
      key: `k${i}`,
      name: `Plug${i}`,
      type: 'VST3',
      manufacturer: 'M',
      count: 1,
      projects: new Set([`/p${i}`]),
    }));
    const html = G.buildAnalyticsHtml({
      pluginsByUsage: singleUse,
      projectsByCount: singleUse.map((_, i) => ({
        path: `/p${i}`,
        name: `Proj${i}`,
        daw: 'ALS',
        count: 1,
        plugins: [],
      })),
      orphaned: [],
      totalProjects: 31,
    });
    assert.ok(html.includes('...and 1 more'));
  });

  it('uses project format as daw label when daw field is missing (SQLite row shape)', () => {
    G.allDawProjects = [{ path: '/p/x.als', name: 'ProjX', format: 'ALS' }];
    G._xrefCache = {
      '/p/x.als': [{ name: 'Z', normalizedName: 'z', manufacturer: 'M', pluginType: 'VST3' }],
    };
    G.allPlugins = [];
    const d = G.buildDepGraphData();
    const row = d.projectsByCount.find((p) => p.path === '/p/x.als');
    assert.ok(row);
    assert.strictEqual(row.daw, 'ALS');
  });

  it('treats installed plugin as used when its normalized name matches xref key (display name differs by suffix)', () => {
    G.allDawProjects = [];
    G._xrefCache = {
      '/p/a.als': [{ name: 'Serum', normalizedName: 'serum', manufacturer: 'Xfer', pluginType: 'VST3' }],
    };
    G.allPlugins = [{ name: 'Serum (VST3)', path: '/x.vst3', type: 'VST3' }];
    const d = G.buildDepGraphData();
    assert.strictEqual(d.orphaned.length, 0);
  });
});
