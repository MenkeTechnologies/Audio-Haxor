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
    G.normalizePluginName = (n) => n.toLowerCase();
    const d = G.buildDepGraphData();
    assert.strictEqual(d.orphaned.length, 1);
    assert.strictEqual(d.orphaned[0].name, 'Orphan FX');
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
});
