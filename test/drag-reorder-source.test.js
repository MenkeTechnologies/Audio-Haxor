/**
 * Real drag-reorder.js: initDragReorder restores child order from prefs.getObject array.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function loadDragReorderSandbox() {
  return loadFrontendScripts(['utils.js', 'drag-reorder.js'], {
    document: {
      ...defaultDocument(),
      getElementById: () => null,
      querySelector: () => null,
      querySelectorAll: () => [],
      addEventListener: () => {},
      body: { style: {}, appendChild: () => {}, removeChild: () => {} },
    },
    prefs: {
      _cache: {
        rowOrder: ['gamma', 'alpha', 'beta'],
      },
      getObject(key, fallback) {
        const v = this._cache[key];
        return v === undefined || v === null ? fallback : v;
      },
      setItem: () => {},
    },
  });
}

describe('frontend/js/drag-reorder.js initDragReorder (vm-loaded)', () => {
  let D;

  before(() => {
    D = loadDragReorderSandbox();
  });

  it('reorders children to match saved key list', () => {
    const items = [
      { dataset: { dragKey: 'alpha' } },
      { dataset: { dragKey: 'beta' } },
      { dataset: { dragKey: 'gamma' } },
    ];
    const container = {
      _children: [],
      querySelectorAll(sel) {
        if (sel === '[data-drag-key]') return [...this._children];
        return [];
      },
      appendChild(node) {
        const i = this._children.indexOf(node);
        if (i >= 0) this._children.splice(i, 1);
        this._children.push(node);
      },
      addEventListener: () => {},
      contains: () => true,
    };
    items.forEach((n) => container.appendChild(n));

    assert.strictEqual(typeof D.initDragReorder, 'function');
    D.initDragReorder(container, '[data-drag-key]', 'rowOrder', {
      getKey: (el) => el.dataset.dragKey,
    });

    assert.strictEqual(
      container._children.map((n) => n.dataset.dragKey).join(','),
      'gamma,alpha,beta',
    );
  });
});
