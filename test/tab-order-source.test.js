/**
 * Real utils.js: saveTabOrder / restoreTabOrder reorder .tab-btn nodes from prefs JSON.
 */
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function prefsStore() {
  return {
    _cache: {},
    getItem(key) {
      const v = this._cache[key];
      return v === undefined ? null : v;
    },
    setItem(key, value) {
      this._cache[key] = value;
    },
    removeItem(key) {
      delete this._cache[key];
    },
  };
}

function createTabNav(initialTabs) {
  const nav = {
    _children: [],
    querySelectorAll(sel) {
      if (sel === '.tab-btn') return [...this._children];
      return [];
    },
    appendChild(node) {
      const i = this._children.indexOf(node);
      if (i >= 0) this._children.splice(i, 1);
      this._children.push(node);
    },
  };
  for (const tab of initialTabs) {
    nav.appendChild({ dataset: { tab } });
  }
  return nav;
}

function loadTabOrderSandbox(nav) {
  return loadFrontendScripts(['utils.js'], {
    prefs: prefsStore(),
    showToast: () => {},
    toastFmt: (k) => k,
    document: {
      ...defaultDocument(),
      querySelector(sel) {
        if (sel === '.tab-nav') return nav;
        return null;
      },
      querySelectorAll(sel) {
        if (sel === '.tab-nav .tab-btn') return [...nav._children];
        return [];
      },
    },
  });
}

describe('frontend/js/utils.js tab order (vm-loaded)', () => {
  it('restoreTabOrder re-appends buttons to match saved prefs array', () => {
    const nav = createTabNav(['plugins', 'samples', 'daw']);
    const U = loadTabOrderSandbox(nav);
    U.prefs._cache.tabOrder = JSON.stringify(['daw', 'plugins', 'samples']);
    U.restoreTabOrder();
    assert.strictEqual(
      nav._children.map((b) => b.dataset.tab).join(','),
      'daw,plugins,samples',
    );
  });

  it('restoreTabOrder appends tabs missing from saved order at the end', () => {
    const nav = createTabNav(['plugins', 'samples', 'daw']);
    const U = loadTabOrderSandbox(nav);
    U.prefs._cache.tabOrder = JSON.stringify(['samples', 'plugins']);
    U.restoreTabOrder();
    assert.strictEqual(nav._children.map((b) => b.dataset.tab).join(','), 'samples,plugins,daw');
  });

  it('saveTabOrder writes dataset.tab sequence to prefs', () => {
    const nav = createTabNav(['a', 'b', 'c']);
    const U = loadTabOrderSandbox(nav);
    U.saveTabOrder();
    assert.strictEqual(U.prefs._cache.tabOrder, JSON.stringify(['a', 'b', 'c']));
  });
});
