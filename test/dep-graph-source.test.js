/**
 * Loads real frontend/js/utils.js + dep-graph.js and validates buildDepGraphData(),
 * which drives the plugin↔DAW dependency analytics tab.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

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
      createElement: () => ({ ...escEl }),
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
});
