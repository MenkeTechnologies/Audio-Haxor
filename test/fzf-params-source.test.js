/**
 * Real utils.js: loadFzfParams / saveFzfParams / resetFzfParams prefs round-trip for scoring globals.
 * Scoring weights are `let` bindings in the vm — read/write via vm.runInContext, not sandbox.SCORE_MATCH.
 */
const vm = require('node:vm');
const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function scoreMatch(sandbox) {
  return vm.runInContext('SCORE_MATCH', sandbox);
}

function prefsObjectStore() {
  return {
    _cache: {},
    getObject(key, fallback) {
      const v = this._cache[key];
      if (v === undefined || v === null) return fallback;
      return v;
    },
    setItem(key, value) {
      this._cache[key] = value;
    },
    getItem(key) {
      const v = this._cache[key];
      return v === undefined ? null : v;
    },
  };
}

describe('frontend/js/utils.js fzf params (vm-loaded)', () => {
  let U;

  beforeEach(() => {
    U = loadFrontendScripts(['utils.js'], {
      prefs: prefsObjectStore(),
      document: defaultDocument(),
    });
  });

  it('loadFzfParams applies saved weights to scoring globals', () => {
    U.prefs._cache.fzfParams = {
      SCORE_MATCH: 99,
      SCORE_GAP_START: -9,
      SCORE_GAP_EXTENSION: -2,
      BONUS_BOUNDARY: 1,
      BONUS_NON_WORD: 2,
      BONUS_CAMEL: 3,
      BONUS_CONSECUTIVE: 4,
      BONUS_FIRST_CHAR_MULT: 5,
    };
    U.loadFzfParams();
    assert.strictEqual(scoreMatch(U), 99);
    assert.strictEqual(vm.runInContext('SCORE_GAP_START', U), -9);
    assert.strictEqual(vm.runInContext('BONUS_FIRST_CHAR_MULT', U), 5);
  });

  it('saveFzfParams writes current globals to prefs object', () => {
    vm.runInContext('SCORE_MATCH = 42', U);
    U.saveFzfParams();
    assert.strictEqual(U.prefs._cache.fzfParams.SCORE_MATCH, 42);
  });

  it('resetFzfParams restores defaults and persists', () => {
    vm.runInContext('SCORE_MATCH = 1', U);
    U.resetFzfParams();
    assert.strictEqual(scoreMatch(U), 16);
    assert.strictEqual(U.prefs._cache.fzfParams.SCORE_MATCH, 16);
  });
});
