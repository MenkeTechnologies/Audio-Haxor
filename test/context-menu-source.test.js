/**
 * Loads real context-menu.js with a stub #ctxMenu; exercises showContextMenu HTML and action map.
 */
const { describe, it, before } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

function loadContextMenuSandbox() {
  const ctxMenuEl = {
    _actions: {},
    _labels: {},
    _skipEcho: {},
    innerHTML: '',
    classList: {
      _c: new Set(),
      add(name) {
        this._c.add(name);
      },
      remove(name) {
        this._c.delete(name);
      },
      contains(name) {
        return this._c.has(name);
      },
    },
    style: { left: '', top: '' },
    getBoundingClientRect: () => ({ width: 120, height: 80 }),
    addEventListener: () => {},
  };
  const sandbox = {
    console,
    document: {
      getElementById: (id) => (id === 'ctxMenu' ? ctxMenuEl : null),
      addEventListener: () => {},
      querySelector: () => null,
    },
    window: {
      innerWidth: 1920,
      innerHeight: 1080,
    },
    navigator: {
      clipboard: { writeText: () => Promise.resolve() },
    },
  };
  sandbox.window = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(
    fs.readFileSync(path.join(__dirname, '..', 'frontend', 'js', 'context-menu.js'), 'utf8'),
    sandbox
  );
  sandbox._ctxMenuEl = ctxMenuEl;
  return sandbox;
}

describe('frontend/js/context-menu.js (vm-loaded)', () => {
  let C;

  before(() => {
    C = loadContextMenuSandbox();
  });

  it('showContextMenu renders items, separators, and disabled class', () => {
    const e = { preventDefault: () => {}, clientX: 10, clientY: 20 };
    C.showContextMenu(e, [
      { icon: '&#9654;', label: 'Play', action: () => 1 },
      '---',
      { icon: 'x', label: 'Disabled', disabled: true, action: () => {} },
    ]);
    const html = C._ctxMenuEl.innerHTML;
    assert.ok(html.includes('ctx-menu-sep'));
    assert.ok(html.includes('ctx-disabled'));
    assert.ok(html.includes('data-ctx-idx="0"'));
    assert.ok(html.includes('data-ctx-idx="2"'));
    assert.strictEqual(typeof C._ctxMenuEl._actions[0], 'function');
    assert.strictEqual(C._ctxMenuEl._labels[0], 'Play');
    assert.ok(C._ctxMenuEl.classList.contains('visible'));
  });

  it('showContextMenu records skipEchoToast on items', () => {
    const e = { preventDefault: () => {}, clientX: 0, clientY: 0 };
    C.showContextMenu(e, [
      { label: 'A', action: () => {}, skipEchoToast: true },
      { label: 'B', action: () => {} },
    ]);
    assert.strictEqual(C._ctxMenuEl._skipEcho[0], true);
    assert.strictEqual(C._ctxMenuEl._skipEcho[1], undefined);
  });

  it('hideContextMenu clears action maps', () => {
    const e = { preventDefault: () => {}, clientX: 0, clientY: 0 };
    C.showContextMenu(e, [{ label: 'X', action: () => {} }]);
    C.hideContextMenu();
    assert.strictEqual(Object.keys(C._ctxMenuEl._actions).length, 0);
    assert.strictEqual(Object.keys(C._ctxMenuEl._labels).length, 0);
  });
});
