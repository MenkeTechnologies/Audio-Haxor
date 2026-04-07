/**
 * Mirrors frontend/js/context-menu.js settings-section move up / move down DOM semantics
 * (`.settings-section` siblings, insertBefore). Pure mocks — no browser.
 */
const { describe, it } = require('node:test');
const assert = require('node:assert/strict');

function linkSiblings(parent, sections) {
  for (let i = 0; i < sections.length; i++) {
    const s = sections[i];
    s.parentNode = parent;
    s.previousElementSibling = sections[i - 1] || null;
    s.nextElementSibling = sections[i + 1] || null;
  }
  parent._sections = [...sections];
}

function makeParent() {
  return {
    _sections: [],
    insertBefore(node, ref) {
      const list = this._sections;
      const oldIdx = list.indexOf(node);
      if (oldIdx >= 0) list.splice(oldIdx, 1);
      let refIdx = ref == null ? list.length : list.indexOf(ref);
      if (refIdx < 0) refIdx = list.length;
      list.splice(refIdx, 0, node);
      linkSiblings(this, list);
    },
  };
}

function makeSection(id) {
  return {
    id,
    classList: {
      contains(name) {
        return name === 'settings-section';
      },
    },
    parentNode: null,
    previousElementSibling: null,
    nextElementSibling: null,
  };
}

/** Same logic as context menu "move up". */
function moveSectionUp(settingsSection) {
  const prev = settingsSection.previousElementSibling;
  if (prev && prev.classList.contains('settings-section')) {
    settingsSection.parentNode.insertBefore(settingsSection, prev);
    return true;
  }
  return false;
}

/** Same logic as context menu "move down". */
function moveSectionDown(settingsSection) {
  const next = settingsSection.nextElementSibling;
  if (next && next.classList.contains('settings-section')) {
    next.parentNode.insertBefore(next, settingsSection);
    return true;
  }
  return false;
}

describe('settings section order (mirrors context-menu move up/down)', () => {
  it('move up swaps with previous sibling', () => {
    const p = makeParent();
    const a = makeSection('a');
    const b = makeSection('b');
    const c = makeSection('c');
    linkSiblings(p, [a, b, c]);
    assert.strictEqual(moveSectionUp(b), true);
    assert.deepStrictEqual(
      p._sections.map((s) => s.id),
      ['b', 'a', 'c'],
    );
  });

  it('move up is a no-op on first section', () => {
    const p = makeParent();
    const a = makeSection('a');
    const b = makeSection('b');
    linkSiblings(p, [a, b]);
    assert.strictEqual(moveSectionUp(a), false);
    assert.deepStrictEqual(
      p._sections.map((s) => s.id),
      ['a', 'b'],
    );
  });

  it('move down swaps with next sibling', () => {
    const p = makeParent();
    const a = makeSection('a');
    const b = makeSection('b');
    const c = makeSection('c');
    linkSiblings(p, [a, b, c]);
    assert.strictEqual(moveSectionDown(b), true);
    assert.deepStrictEqual(
      p._sections.map((s) => s.id),
      ['a', 'c', 'b'],
    );
  });

  it('move down is a no-op on last section', () => {
    const p = makeParent();
    const a = makeSection('a');
    const b = makeSection('b');
    linkSiblings(p, [a, b]);
    assert.strictEqual(moveSectionDown(b), false);
    assert.deepStrictEqual(
      p._sections.map((s) => s.id),
      ['a', 'b'],
    );
  });
});
