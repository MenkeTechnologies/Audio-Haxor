/**
 * Loads real utils.js + disk-usage.js; exercises renderDiskUsageBar HTML and sort order.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');
const { createTextDiv } = require('./frontend-vm-harness.js');

function loadDiskUsageSandbox() {
  const containers = Object.create(null);
  function mockEl(id) {
    if (!containers[id]) {
      const o = {
        id,
        style: {},
        innerHTML: '',
        querySelectorAll(sel) {
          const re = /data-bar-pct="([^"]*)"/g;
          const nodes = [];
          let m;
          while ((m = re.exec(this.innerHTML)) !== null) {
            nodes.push({
              style: {},
              dataset: { barPct: m[1] },
            });
          }
          return {
            forEach(fn) {
              nodes.forEach(fn);
            },
          };
        },
      };
      containers[id] = o;
    }
    return containers[id];
  }
  const sandbox = {
    console,
    performance: { now: () => 0 },
    KVR_MANUFACTURER_MAP: {},
    prefs: {
      getObject: () => null,
      setItem: () => {},
      removeItem: () => {},
    },
    document: {
      createElement: () => createTextDiv(),
      getElementById: (id) => mockEl(id),
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
    },
    setTimeout: () => 0,
    clearTimeout: () => {},
    requestAnimationFrame: (cb) => {
      if (typeof cb === 'function') cb();
      return 0;
    },
  };
  sandbox.window = sandbox;
  vm.createContext(sandbox);
  const root = path.join(__dirname, '..', 'frontend', 'js');
  vm.runInContext(fs.readFileSync(path.join(root, 'utils.js'), 'utf8'), sandbox);
  vm.runInContext(fs.readFileSync(path.join(root, 'disk-usage.js'), 'utf8'), sandbox);
  sandbox._containers = containers;
  return sandbox;
}

describe('frontend/js/disk-usage.js (vm-loaded)', () => {
  let D;

  before(() => {
    D = loadDiskUsageSandbox();
  });

  it('renderDiskUsageBar hides container when totalBytes is 0', () => {
    D.renderDiskUsageBar(
      'testDisk',
      [{ label: 'WAV', bytes: 100, sizeStr: '100 B' }],
      0
    );
    const el = D._containers.testDisk;
    assert.strictEqual(el.style.display, 'none');
  });

  it('renderDiskUsageBar builds segments and legend with percentages', () => {
    D.renderDiskUsageBar('testDisk', [
      { label: 'WAV', bytes: 750, sizeStr: '750 B' },
      { label: 'MP3', bytes: 250, sizeStr: '250 B' },
    ], 1000);
    const el = D._containers.testDisk;
    assert.strictEqual(el.style.display, '');
    assert.ok(el.innerHTML.includes('class="disk-bar"'));
    assert.ok(el.innerHTML.includes('disk-segment'));
    assert.ok(el.innerHTML.includes('75.0'));
    assert.ok(el.innerHTML.includes('25.0'));
    assert.ok(el.innerHTML.includes('disk-legend'));
  });

  it('renderDiskUsageBar sorts rows by bytes descending (largest segment first in DOM order)', () => {
    D.renderDiskUsageBar('testDisk', [
      { label: 'MP3', bytes: 100, sizeStr: '100 B' },
      { label: 'WAV', bytes: 900, sizeStr: '900 B' },
    ], 1000);
    const el = D._containers.testDisk;
    const firstSeg = el.innerHTML.indexOf('WAV');
    const secondSeg = el.innerHTML.indexOf('MP3');
    assert.ok(firstSeg >= 0 && secondSeg >= 0);
    assert.ok(firstSeg < secondSeg, 'WAV (larger) should appear before MP3 in bar HTML');
  });

  it('renderDiskUsageBar hides container when data array is empty', () => {
    D.renderDiskUsageBar('emptyDisk', [], 1000);
    const el = D._containers.emptyDisk;
    assert.strictEqual(el.style.display, 'none');
  });

  it('renderDiskUsageBar single format shows 100%', () => {
    D.renderDiskUsageBar('oneDisk', [{ label: 'WAV', bytes: 500, sizeStr: '500 B' }], 500);
    const el = D._containers.oneDisk;
    assert.strictEqual(el.style.display, '');
    assert.ok(el.innerHTML.includes('100.0'));
  });
});
