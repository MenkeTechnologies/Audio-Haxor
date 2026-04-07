/**
 * Real drag-reorder.js: initDragReorder restores child order from prefs.getObject array.
 */
const { describe, it, before, beforeEach } = require('node:test');
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
    const container = {
      _children: [],
      get children() {
        return this._children;
      },
      querySelectorAll(sel) {
        if (sel === '[data-drag-key]') return [...this._children];
        return [];
      },
      appendChild(node) {
        const i = this._children.indexOf(node);
        if (i >= 0) this._children.splice(i, 1);
        node.parentElement = this;
        this._children.push(node);
      },
      addEventListener: () => {},
      contains: () => true,
    };
    const items = [
      { dataset: { dragKey: 'alpha' }, matches: (s) => s === '[data-drag-key]' },
      { dataset: { dragKey: 'beta' }, matches: (s) => s === '[data-drag-key]' },
      { dataset: { dragKey: 'gamma' }, matches: (s) => s === '[data-drag-key]' },
    ];
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

  it('does not reorder when prefsKey is null (no saved order lookup)', () => {
    const container = {
      _children: [],
      get children() {
        return this._children;
      },
      querySelectorAll(sel) {
        if (sel === '[data-drag-key]') return [...this._children];
        return [];
      },
      appendChild(node) {
        const i = this._children.indexOf(node);
        if (i >= 0) this._children.splice(i, 1);
        node.parentElement = this;
        this._children.push(node);
      },
      addEventListener: () => {},
      contains: () => true,
    };
    const items = [
      { dataset: { dragKey: 'a' }, matches: (s) => s === '[data-drag-key]' },
      { dataset: { dragKey: 'b' }, matches: (s) => s === '[data-drag-key]' },
    ];
    items.forEach((n) => container.appendChild(n));
    const D = loadFrontendScripts(['utils.js', 'drag-reorder.js'], {
      prefs: { getObject: () => ['b', 'a'], setItem: () => {} },
      document: {
        ...defaultDocument(),
        getElementById: () => null,
        querySelector: () => null,
        querySelectorAll: () => [],
        addEventListener: () => {},
        body: { style: {}, appendChild: () => {}, removeChild: () => {} },
      },
    });
    D.initDragReorder(container, '[data-drag-key]', null, {
      getKey: (el) => el.dataset.dragKey,
    });
    assert.strictEqual(container._children.map((n) => n.dataset.dragKey).join(','), 'a,b');
  });

  it('does not reorder when saved prefs value is not an array', () => {
    const container = {
      _children: [],
      get children() {
        return this._children;
      },
      querySelectorAll(sel) {
        if (sel === '[data-drag-key]') return [...this._children];
        return [];
      },
      appendChild(node) {
        const i = this._children.indexOf(node);
        if (i >= 0) this._children.splice(i, 1);
        node.parentElement = this;
        this._children.push(node);
      },
      addEventListener: () => {},
      contains: () => true,
    };
    const items = [
      { dataset: { dragKey: 'x' }, matches: (s) => s === '[data-drag-key]' },
      { dataset: { dragKey: 'y' }, matches: (s) => s === '[data-drag-key]' },
    ];
    items.forEach((n) => container.appendChild(n));
    const D = loadFrontendScripts(['utils.js', 'drag-reorder.js'], {
      prefs: {
        getObject: () => ({ order: ['y', 'x'] }),
        setItem: () => {},
      },
      document: {
        ...defaultDocument(),
        getElementById: () => null,
        querySelector: () => null,
        querySelectorAll: () => [],
        addEventListener: () => {},
        body: { style: {}, appendChild: () => {}, removeChild: () => {} },
      },
    });
    D.initDragReorder(container, '[data-drag-key]', 'rowOrder', {
      getKey: (el) => el.dataset.dragKey,
    });
    assert.strictEqual(container._children.map((n) => n.dataset.dragKey).join(','), 'x,y');
  });
});

function loadDragReorderForTableRows() {
  let tableRef = null;
  const document = {
    ...defaultDocument(),
    getElementById(id) {
      if (id === 'testTable') return tableRef;
      return null;
    },
    createDocumentFragment() {
      return {
        _children: [],
        appendChild(child) {
          this._children.push(child);
        },
      };
    },
    querySelector: () => null,
    querySelectorAll: () => [],
    addEventListener: () => {},
    body: { style: {}, appendChild: () => {}, removeChild: () => {} },
  };
  const D = loadFrontendScripts(['utils.js', 'drag-reorder.js'], {
    document,
    prefs: { _cache: {}, getObject: () => null, setItem: () => {} },
  });
  return { D, setTable(t) { tableRef = t; } };
}

describe('frontend/js/drag-reorder.js handleSelector + restoreAnchor (vm-loaded)', () => {
  let D;

  beforeEach(() => {
    D = loadFrontendScripts(['utils.js', 'drag-reorder.js'], {
      document: {
        ...defaultDocument(),
        getElementById: () => null,
        querySelector: () => null,
        querySelectorAll: () => [],
        addEventListener: () => {},
        body: { style: {}, appendChild: () => {}, removeChild: () => {} },
      },
      prefs: {
        getObject(k, fb) {
          if (k === 'secOrder') return ['b', 'a'];
          return fb;
        },
        setItem: () => {},
      },
    });
  });

  it('handleSelector: drag only starts when mousedown target is inside handle', () => {
    const handlers = [];
    const container = {
      _trelloDragInit: false,
      _children: [],
      get children() {
        return this._children;
      },
      querySelectorAll(sel) {
        if (sel === '.item') return [...this._children];
        return [];
      },
      appendChild(node) {
        const i = this._children.indexOf(node);
        if (i >= 0) this._children.splice(i, 1);
        node.parentElement = this;
        this._children.push(node);
      },
      contains() {
        return true;
      },
      addEventListener(type, fn) {
        if (type === 'mousedown') handlers.push(fn);
      },
    };

    const child = {
      dataset: { dragKey: 'x' },
      matches: (s) => s === '.item',
      getBoundingClientRect: () => ({ left: 0, top: 0, width: 10, height: 10 }),
      parentElement: container,
    };
    const handle = { parentElement: child, classList: { contains: () => false } };
    const outside = {
      parentElement: child,
      closest(sel) {
        if (sel === '.item') return child;
        if (sel === '.handle') return null;
        return null;
      },
    };
    const inside = {
      parentElement: handle,
      closest(sel) {
        if (sel === '.item') return child;
        if (sel === '.handle') return handle;
        return null;
      },
    };

    container.appendChild(child);

    D.initDragReorder(container, '.item', null, {
      handleSelector: '.handle',
    });

    let prevented = false;
    const evOut = {
      button: 0,
      clientX: 0,
      clientY: 0,
      target: outside,
      preventDefault() {
        prevented = true;
      },
    };
    handlers[0](evOut);
    assert.strictEqual(prevented, false);

    let prevented2 = false;
    const evIn = {
      button: 0,
      clientX: 0,
      clientY: 0,
      target: inside,
      preventDefault() {
        prevented2 = true;
      },
    };
    handlers[0](evIn);
    assert.strictEqual(prevented2, true);
  });

  it('restoreAnchorSelector: inserts merged keys after anchor via insertAdjacentElement', () => {
    const log = [];
    const anchor = {
      insertAdjacentElement(pos, el) {
        assert.strictEqual(pos, 'afterend');
        log.push(`anchor:${el.dataset.section}`);
      },
    };
    const secA = {
      dataset: { section: 'a' },
      matches: (s) => s === '.sec',
      parentElement: null,
      insertAdjacentElement(pos, el) {
        assert.strictEqual(pos, 'afterend');
        log.push(`a:${el.dataset.section}`);
      },
    };
    const secB = {
      dataset: { section: 'b' },
      matches: (s) => s === '.sec',
      parentElement: null,
      insertAdjacentElement(pos, el) {
        assert.strictEqual(pos, 'afterend');
        log.push(`b:${el.dataset.section}`);
      },
    };

    const container = {
      _trelloDragInit: false,
      _children: [secA, secB],
      get children() {
        return this._children;
      },
      querySelectorAll(sel) {
        if (sel === '.sec') return [...this._children];
        return [];
      },
      querySelector(sel) {
        if (sel === '.restore-anchor') return anchor;
        return null;
      },
      appendChild(node) {
        const i = this._children.indexOf(node);
        if (i >= 0) this._children.splice(i, 1);
        node.parentElement = this;
        this._children.push(node);
      },
      contains: () => true,
      addEventListener: () => {},
    };
    secA.parentElement = container;
    secB.parentElement = container;

    D.initDragReorder(container, '.sec', 'secOrder', {
      getKey: (el) => el.dataset.section,
      restoreAnchorSelector: '.restore-anchor',
    });

    assert.deepStrictEqual(log, ['anchor:b', 'b:a']);
  });
});

describe('frontend/js/drag-reorder.js reorderNewTableRows (vm-loaded)', () => {
  it('returns early when table has no _colOrder flag', () => {
    const { D, setTable } = loadDragReorderForTableRows();
    setTable({ id: 'x' });
    D.reorderNewTableRows('testTable');
  });

  it('permutes tbody cells when thead column keys differ from default order', () => {
    const { D, setTable } = loadDragReorderForTableRows();
    const cbCell = { id: 'cb' };
    const nameCell = { id: 'name' };
    const row = {
      _colReordered: false,
      cells: [cbCell, nameCell],
      appendChild(frag) {
        this._finalOrder = frag._children.map((c) => c.id);
      },
    };
    const tbody = { rows: [row] };
    const thName = { dataset: { thKey: 'name' } };
    const thCb = { dataset: { thKey: 'col-cb' } };
    const thead = { children: [thName, thCb] };
    setTable({
      _colOrder: true,
      _getColKey: (th) => th.dataset.thKey,
      querySelector(sel) {
        if (sel === 'thead tr') return thead;
        if (sel === 'tbody') return tbody;
        return null;
      },
    });
    D.reorderNewTableRows('testTable');
    assert.strictEqual(row._finalOrder.join(','), 'name,cb');
  });
});
