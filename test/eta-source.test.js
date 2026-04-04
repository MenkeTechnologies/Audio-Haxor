/**
 * Real utils.js: createETA elapsed and remaining-time strings from performance.now.
 */
const { describe, it, beforeEach } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

describe('frontend/js/utils.js createETA (vm-loaded)', () => {
  let t;
  let U;

  beforeEach(() => {
    t = 0;
    U = loadFrontendScripts(['utils.js'], {
      performance: { now: () => t },
      document: defaultDocument(),
    });
  });

  it('estimate returns empty until start and positive progress', () => {
    const eta = U.createETA();
    assert.strictEqual(eta.estimate(1, 10), '');
    eta.start();
    assert.strictEqual(eta.estimate(0, 10), '');
  });

  it('elapsed formats seconds under one minute', () => {
    const eta = U.createETA();
    t = 1000;
    eta.start();
    t = 4500;
    assert.strictEqual(eta.elapsed(), '3s');
  });

  it('estimate scales remaining work from observed rate', () => {
    const eta = U.createETA();
    t = 10_000;
    eta.start();
    t = 20_000;
    const out = eta.estimate(100, 1100);
    assert.ok(out.includes('s') || out.includes('m'));
    assert.ok(out.startsWith('~'));
  });
});
