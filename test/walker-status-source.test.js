/**
 * Real walker-status.js: _renderTile scanning vs idle HTML and border styling.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const { loadFrontendScripts, defaultDocument } = require('./frontend-vm-harness.js');

function loadWalkerSandbox(bodyId, tileId) {
  const body = { innerHTML: '' };
  const statusEl = { innerHTML: '' };
  const tile = {
    style: { borderColor: '' },
    querySelector(sel) {
      return sel === '.walker-tile-status' ? statusEl : null;
    },
  };
  const base = defaultDocument();
  const document = {
    ...base,
    getElementById(id) {
      if (id === bodyId) return body;
      if (id === tileId) return tile;
      return null;
    },
  };
  const W = loadFrontendScripts(['utils.js', 'walker-status.js'], { document });
  return { W, body, statusEl, tile };
}

describe('frontend/js/walker-status.js _renderTile (vm-loaded)', () => {
  let W;
  let body;
  let statusEl;
  let tile;

  before(() => {
    ({ W, body, statusEl, tile } = loadWalkerSandbox('walkerPluginBody', 'walkerTilePlugin'));
  });

  it('when scanning, lists dirs with escaped HTML and sets accent border', () => {
    W._renderTile('walkerPluginBody', 'walkerTilePlugin', ['/a & <b>', '/c'], 'var(--cyan)', 4, true);
    assert.ok(body.innerHTML.includes('&amp;'));
    assert.ok(body.innerHTML.includes('/a'));
    assert.ok(statusEl.innerHTML.includes('scanning'));
    assert.strictEqual(tile.style.borderColor, 'var(--cyan)');
  });

  it('when idle with empty dirs, shows waiting copy and resets border', () => {
    W._renderTile('walkerPluginBody', 'walkerTilePlugin', [], 'var(--cyan)', 4, false);
    assert.ok(body.innerHTML.includes('Waiting for scan'));
    assert.strictEqual(tile.style.borderColor, 'var(--border)');
  });

  it('when idle but dirs buffer non-empty, does not overwrite body (stale list until next scan)', () => {
    body.innerHTML = '<div class="stale">keep</div>';
    W._renderTile('walkerPluginBody', 'walkerTilePlugin', ['/tmp/a'], 'var(--cyan)', 4, false);
    assert.ok(body.innerHTML.includes('stale'));
    assert.ok(statusEl.innerHTML.includes('idle'));
  });

  it('when scanning with one dir, escapes HTML in path', () => {
    W._renderTile('walkerPluginBody', 'walkerTilePlugin', ['/a<b>"c'], 'var(--cyan)', 2, true);
    assert.ok(body.innerHTML.includes('&lt;b&gt;'));
    assert.ok(body.innerHTML.includes('&quot;'));
  });
});
