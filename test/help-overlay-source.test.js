/**
 * Real help-overlay.js: toggleHelpOverlay opens modal HTML then removes on second toggle.
 */
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function loadHelpSandbox() {
  let helpEl = null;
  const metrics = { inserts: 0 };
  const base = defaultDocument();
  const document = {
    ...base,
    getElementById(id) {
      if (id === 'helpOverlay') return helpEl;
      return null;
    },
    body: {
      insertAdjacentHTML(_pos, html) {
        if (html.includes('helpOverlay')) {
          metrics.inserts += 1;
          helpEl = {
            remove() {
              helpEl = null;
            },
          };
        }
      },
    },
  };
  const H = loadFrontendScripts(['utils.js', 'help-overlay.js'], {
    document,
    appFmt: (k) => k,
  });
  return { H, metrics };
}

describe('frontend/js/help-overlay.js toggleHelpOverlay (vm-loaded)', () => {
  it('inserts overlay HTML on first open', () => {
    const { H, metrics } = loadHelpSandbox();
    H.toggleHelpOverlay();
    assert.strictEqual(metrics.inserts, 1);
  });

  it('removes overlay on second toggle without inserting again', () => {
    const { H, metrics } = loadHelpSandbox();
    H.toggleHelpOverlay();
    H.toggleHelpOverlay();
    assert.strictEqual(metrics.inserts, 1);
  });

  it('open close open inserts twice', () => {
    const { H, metrics } = loadHelpSandbox();
    H.toggleHelpOverlay();
    H.toggleHelpOverlay();
    H.toggleHelpOverlay();
    assert.strictEqual(metrics.inserts, 2);
  });
});
