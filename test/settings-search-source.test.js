/**
 * Real settings-search.js: filter rows/sections by normalized query; section-heading
 * match reveals all rows; select option text is searchable (e.g. color scheme names).
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

function makeRow({ title = '', desc = '', selectOptions = null }) {
  const titleEl = { textContent: title };
  const descEl = { textContent: desc };
  const selects = [];
  if (selectOptions && selectOptions.length) {
    selects.push({
      options: selectOptions.map((t) => ({ textContent: t })),
    });
  }
  return {
    style: { display: '' },
    querySelector(sel) {
      if (sel === '.settings-title') return titleEl;
      if (sel === '.settings-desc') return descEl;
      return null;
    },
    querySelectorAll(sel) {
      if (sel === 'select') return selects;
      return [];
    },
  };
}

function makeSection(headingText, rows) {
  const heading = { textContent: headingText };
  return {
    style: { display: '' },
    querySelector(sel) {
      if (sel === '.settings-heading') return heading;
      return null;
    },
    querySelectorAll(sel) {
      if (sel === '.settings-row') return rows;
      return [];
    },
  };
}

function loadSettingsSearch(dom) {
  const sandbox = {
    console,
    document: dom,
    window: {},
    setTimeout: (fn, _ms) => {
      if (typeof fn === 'function') fn();
      return 0;
    },
    clearTimeout: () => {},
  };
  sandbox.window = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(
    fs.readFileSync(path.join(__dirname, '..', 'frontend', 'js', 'settings-search.js'), 'utf8'),
    sandbox
  );
  return sandbox;
}

describe('frontend/js/settings-search.js (vm-loaded)', () => {
  let dom;
  let filterSettings;

  before(() => {
    const searchInput = { value: '', style: {} };
    const clearBtn = { style: { display: 'none' } };
    const emptyEl = { style: { display: 'none' } };

    const rowCache = makeRow({ title: 'Cache size', desc: 'MB limit' });
    const rowTheme = makeRow({
      title: 'Color scheme',
      desc: 'Pick a theme',
      selectOptions: ['Default', 'Cyberpunk', 'Solarized'],
    });
    const rowOther = makeRow({ title: 'Unrelated', desc: 'Nothing here' });

    const secGeneral = makeSection('General', [rowCache, rowOther]);
    const secAppearance = makeSection('Appearance', [rowTheme]);

    dom = {
      sections: [secGeneral, secAppearance],
      getElementById(id) {
        if (id === 'settingsSearchInput') return searchInput;
        if (id === 'clearSettingsSearchBtn') return clearBtn;
        if (id === 'settingsSearchEmpty') return emptyEl;
        return null;
      },
      querySelectorAll(sel) {
        if (sel === '#tabSettings .settings-section') return this.sections;
        return [];
      },
      addEventListener: () => {},
    };

    const S = loadSettingsSearch(dom);
    filterSettings = S.filterSettings;
    dom._searchInput = searchInput;
    dom._clearBtn = clearBtn;
    dom._emptyEl = emptyEl;
    dom._rowCache = rowCache;
    dom._rowTheme = rowTheme;
    dom._rowOther = rowOther;
    dom._secGeneral = secGeneral;
    dom._secAppearance = secAppearance;
  });

  it('empty query shows all rows and sections; hides empty state', () => {
    dom._searchInput.value = '';
    filterSettings();
    assert.strictEqual(dom._rowCache.style.display, '');
    assert.strictEqual(dom._rowTheme.style.display, '');
    assert.strictEqual(dom._rowOther.style.display, '');
    assert.strictEqual(dom._secGeneral.style.display, '');
    assert.strictEqual(dom._secAppearance.style.display, '');
    assert.strictEqual(dom._emptyEl.style.display, 'none');
    assert.strictEqual(dom._clearBtn.style.display, 'none');
  });

  it('matches row by title substring (case-insensitive)', () => {
    dom._searchInput.value = 'CACHE';
    filterSettings();
    assert.strictEqual(dom._rowCache.style.display, '');
    assert.strictEqual(dom._rowOther.style.display, 'none');
    assert.strictEqual(dom._clearBtn.style.display, '');
  });

  it('matches row via select option text (theme name), not only title/desc', () => {
    dom._searchInput.value = 'cyberpunk';
    filterSettings();
    assert.strictEqual(dom._rowTheme.style.display, '');
    assert.strictEqual(dom._rowCache.style.display, 'none');
    assert.strictEqual(dom._rowOther.style.display, 'none');
  });

  it('section heading match shows all rows in that section', () => {
    dom._searchInput.value = 'appearance';
    filterSettings();
    assert.strictEqual(dom._rowTheme.style.display, '');
    assert.strictEqual(dom._secAppearance.style.display, '');
    assert.strictEqual(dom._rowCache.style.display, 'none');
  });

  it('no matching rows shows empty state and hides clear query UX when cleared', () => {
    dom._searchInput.value = 'zzznomatchzzz';
    filterSettings();
    assert.strictEqual(dom._emptyEl.style.display, '');
    dom._searchInput.value = '';
    filterSettings();
    assert.strictEqual(dom._emptyEl.style.display, 'none');
  });

  it('hides a section when no row matches and the heading does not match', () => {
    dom._searchInput.value = 'cyberpunk';
    filterSettings();
    assert.strictEqual(dom._rowTheme.style.display, '');
    assert.strictEqual(dom._secAppearance.style.display, '');
    assert.strictEqual(dom._secGeneral.style.display, 'none');
    assert.strictEqual(dom._rowCache.style.display, 'none');
  });

  it('whitespace-only query behaves like empty (shows all)', () => {
    dom._searchInput.value = '   \t  ';
    filterSettings();
    assert.strictEqual(dom._rowCache.style.display, '');
    assert.strictEqual(dom._rowTheme.style.display, '');
    assert.strictEqual(dom._emptyEl.style.display, 'none');
  });

  it('matches description text without title token', () => {
    dom._searchInput.value = 'mb limit';
    filterSettings();
    assert.strictEqual(dom._rowCache.style.display, '');
    assert.strictEqual(dom._rowOther.style.display, 'none');
  });
});
