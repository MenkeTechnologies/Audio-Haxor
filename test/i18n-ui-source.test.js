/**
 * Real i18n-ui.js: applyUiI18n fills text, placeholder, and title from window.__appStr.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts } = require('./frontend-vm-harness.js');

describe('frontend/js/i18n-ui.js applyUiI18n (vm-loaded)', () => {
  let I;
  let e1;
  let e2;
  let e3;

  before(() => {
    e1 = { dataset: { i18n: 'app.title' }, textContent: 'old', placeholder: '', title: '' };
    e2 = { dataset: { i18nPlaceholder: 'app.searchPh' }, textContent: '', placeholder: 'ph', title: '' };
    e3 = { dataset: { i18nTitle: 'app.saveTip' }, textContent: '', placeholder: '', title: 'oldtip' };
    I = loadFrontendScripts(['i18n-ui.js'], {
      __appStr: {
        'app.title': 'Audio Haxor',
        'app.searchPh': 'Search plugins…',
        'app.saveTip': 'Save changes',
      },
      document: {
        querySelectorAll(sel) {
          if (sel === '[data-i18n]') return [e1];
          if (sel === '[data-i18n-placeholder]') return [e2];
          if (sel === '[data-i18n-title]') return [e3];
          return [];
        },
        createElement: () => ({}),
        getElementById: () => null,
        addEventListener: () => {},
        body: { insertAdjacentHTML: () => {} },
      },
    });
  });

  it('replaces textContent, placeholder, and title when keys exist and non-empty', () => {
    I.applyUiI18n();
    assert.strictEqual(e1.textContent, 'Audio Haxor');
    assert.strictEqual(e2.placeholder, 'Search plugins…');
    assert.strictEqual(e3.title, 'Save changes');
  });

  it('skips keys that are missing or empty in __appStr', () => {
    const el = { dataset: { i18n: 'missing.key' }, textContent: 'unchanged' };
    const J = loadFrontendScripts(['i18n-ui.js'], {
      __appStr: { other: 'x' },
      document: {
        querySelectorAll(sel) {
          if (sel === '[data-i18n]') return [el];
          return [];
        },
        createElement: () => ({}),
        getElementById: () => null,
        addEventListener: () => {},
        body: { insertAdjacentHTML: () => {} },
      },
    });
    J.applyUiI18n();
    assert.strictEqual(el.textContent, 'unchanged');
  });
});
