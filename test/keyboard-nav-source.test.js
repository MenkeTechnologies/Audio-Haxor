/**
 * Real keyboard-nav.js: getNavigableItems per tab, setNavIndex clamps and selects.
 */
const vm = require('node:vm');
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function makeCard(navEls) {
  const card = {
    scrollIntoView: () => {},
    classList: {
      add(c) {
        if (c === 'nav-selected') navEls.push(card);
      },
      remove(c) {
        if (c === 'nav-selected') {
          const i = navEls.indexOf(card);
          if (i >= 0) navEls.splice(i, 1);
        }
      },
    },
  };
  return card;
}

/** @param {object} activeTab @param {unknown[]} [navSelectedEls] Same array makeCard() pushes into, so clearNavSelection sees real selection. */
function loadKeyboardNavSandbox(activeTab, navSelectedEls = []) {
  const document = {
    ...defaultDocument(),
    querySelector(sel) {
      if (sel === '.tab-content.active') return activeTab;
      return null;
    },
    querySelectorAll(sel) {
      // Snapshot — forEach + splice during clearNavSelection must not mutate array mid-iteration.
      if (sel === '.nav-selected') return navSelectedEls.slice();
      return [];
    },
    addEventListener: () => {},
  };
  return loadFrontendScripts(['utils.js', 'keyboard-nav.js'], {
    document,
    switchTab: () => {},
    previewAudio: () => {},
    openPresetFolder: () => {},
    openKvr: () => {},
    toggleHelpOverlay: () => {},
    showToast: () => {},
    toastFmt: (k) => k,
    batchSelected: { size: 0 },
    deselectAll: () => {},
    selectAllVisible: () => {},
    window: { vstUpdater: { openDawProject: () => Promise.resolve() } },
  });
}

describe('frontend/js/keyboard-nav.js (vm-loaded)', () => {
  it('getNavigableItems returns plugin cards on tabPlugins', () => {
    const c1 = {};
    const c2 = {};
    const activeTab = {
      id: 'tabPlugins',
      querySelectorAll(sel) {
        if (sel === '.plugin-card') return [c1, c2];
        return [];
      },
    };
    const K = loadKeyboardNavSandbox(activeTab);
    const items = K.getNavigableItems();
    assert.strictEqual(items.length, 2);
    assert.strictEqual(items[0], c1);
  });

  it('getNavigableItems returns empty for unsupported tab id', () => {
    const activeTab = {
      id: 'tabVisualizer',
      querySelectorAll: () => [],
    };
    const K = loadKeyboardNavSandbox(activeTab);
    assert.strictEqual(K.getNavigableItems().length, 0);
  });

  it('setNavIndex clamps index and applies nav-selected', () => {
    const navEls = [];
    const a = makeCard(navEls);
    const b = makeCard(navEls);
    const activeTab = {
      id: 'tabPlugins',
      querySelectorAll(sel) {
        if (sel === '.plugin-card') return [a, b];
        return [];
      },
    };
    const K = loadKeyboardNavSandbox(activeTab, navEls);
    K.setNavIndex(99);
    assert.strictEqual(vm.runInContext('_navIndex', K), 1);
    assert.strictEqual(navEls.length, 1);
    assert.strictEqual(navEls[0], b);
    K.setNavIndex(0);
    assert.strictEqual(navEls.length, 1);
    assert.strictEqual(navEls[0], a);
  });

  it('getNavigableItems returns #audioTableBody rows on tabSamples', () => {
    const tr = { dataset: { audioPath: '/s/k.wav' } };
    const activeTab = {
      id: 'tabSamples',
      querySelectorAll(sel) {
        if (sel === '#audioTableBody tr[data-audio-path]') return [tr];
        return [];
      },
    };
    const K = loadKeyboardNavSandbox(activeTab);
    assert.strictEqual(K.getNavigableItems().length, 1);
    assert.strictEqual(K.getNavigableItems()[0], tr);
  });

  it('getNavigableItems returns .fav-item nodes on tabFavorites', () => {
    const fav = {};
    const activeTab = {
      id: 'tabFavorites',
      querySelectorAll(sel) {
        if (sel === '.fav-item') return [fav];
        return [];
      },
    };
    const K = loadKeyboardNavSandbox(activeTab);
    assert.strictEqual(K.getNavigableItems()[0], fav);
  });

  it('clearNavSelection removes nav-selected from tracked elements', () => {
    const removed = [];
    const el = {
      classList: {
        remove(c) {
          if (c === 'nav-selected') removed.push(1);
        },
      },
    };
    const activeTab = { id: 'tabPlugins', querySelectorAll: () => [] };
    const K = loadKeyboardNavSandbox(activeTab, [el]);
    K.clearNavSelection();
    assert.strictEqual(removed.length, 1);
  });

  it('_getSelectedPath reads dataset.path on plugin card', () => {
    const card = {
      dataset: { path: '/Plugins/X.vst3' },
      getAttribute: () => null,
      classList: { add: () => {}, remove: () => {} },
      scrollIntoView: () => {},
    };
    const activeTab = {
      id: 'tabPlugins',
      querySelectorAll(sel) {
        if (sel === '.plugin-card') return [card];
        return [];
      },
    };
    const K = loadKeyboardNavSandbox(activeTab);
    K.setNavIndex(0);
    assert.strictEqual(vm.runInContext('_getSelectedPath()', K), '/Plugins/X.vst3');
  });
});
