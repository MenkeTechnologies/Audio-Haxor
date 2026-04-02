// ── Generic Trello-style Drag-to-Reorder ──
// Single global mousemove/mouseup — no listener accumulation.
// Each container registers only a mousedown on itself.

(function () {
  // Global drag state — only one drag at a time
  let _drag = null; // { container, childSelector, direction, dragged, ghost, placeholder, startX, startY, offsetX, offsetY, isDragging, saveOrder, onReorder }

  document.addEventListener('mousemove', (e) => {
    if (!_drag) return;
    const d = _drag;
    const dx = e.clientX - d.startX;
    const dy = e.clientY - d.startY;

    if (!d.isDragging && Math.abs(d.direction === 'horizontal' ? dx : dy) > 5) {
      d.isDragging = true;
      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'grabbing';

      const rect = d.dragged.getBoundingClientRect();
      d.placeholder = document.createElement(d.dragged.tagName);
      d.placeholder.className = 'trello-placeholder';
      if (d.direction === 'horizontal') {
        d.placeholder.style.width = rect.width + 'px';
        d.placeholder.style.height = rect.height + 'px';
        d.placeholder.style.display = 'inline-block';
      } else {
        d.placeholder.style.height = rect.height + 'px';
      }
      d.dragged.parentNode.insertBefore(d.placeholder, d.dragged);

      d.ghost = d.dragged.cloneNode(true);
      d.ghost.classList.add('trello-ghost');
      d.ghost.style.cssText = `position:fixed;z-index:20000;width:${rect.width}px;height:${rect.height}px;left:${rect.left}px;top:${rect.top}px;pointer-events:none;opacity:0.9;transform:${d.direction === 'horizontal' ? 'scale(1.05)' : 'rotate(1deg)'};box-shadow:0 8px 32px rgba(0,0,0,0.5),0 0 20px rgba(5,217,232,0.3);border:2px solid var(--cyan);border-radius:4px;background:var(--bg-primary);transition:none;`;
      document.body.appendChild(d.ghost);
      d.dragged.style.display = 'none';
    }

    if (!d.isDragging || !d.ghost) return;

    d.ghost.style.left = (e.clientX - d.offsetX) + 'px';
    d.ghost.style.top = (e.clientY - d.offsetY) + 'px';

    d.ghost.style.display = 'none';
    const el = document.elementFromPoint(e.clientX, e.clientY);
    d.ghost.style.display = '';
    const target = el?.closest(d.childSelector);

    if (target && target !== d.dragged && target !== d.placeholder && d.container.contains(target)) {
      const r = target.getBoundingClientRect();
      const mid = d.direction === 'horizontal' ? r.left + r.width / 2 : r.top + r.height / 2;
      const pos = d.direction === 'horizontal' ? e.clientX : e.clientY;
      d.container.insertBefore(d.placeholder, pos < mid ? target : target.nextSibling);
    }
  });

  document.addEventListener('mouseup', () => {
    if (!_drag) return;
    const d = _drag;
    if (d.isDragging) {
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
      if (d.placeholder?.parentNode) {
        d.placeholder.parentNode.insertBefore(d.dragged, d.placeholder);
        d.placeholder.remove();
      }
      d.dragged.style.display = '';
      if (d.ghost) { d.ghost.remove(); }
      d.saveOrder();
      if (d.onReorder) d.onReorder();
    }
    _drag = null;
  });

  // Public API
  window.initDragReorder = function (container, childSelector, prefsKey, opts) {
    if (!container || container._trelloDragInit) return;
    container._trelloDragInit = true;

    const direction = opts?.direction || 'vertical';
    const onReorder = opts?.onReorder || null;
    const handleSelector = opts?.handleSelector || null;
    const getKey = opts?.getKey || ((el, i) => el.dataset.dragKey || el.dataset.npSection || el.textContent.trim().slice(0, 30) || String(i));

    // Restore saved order
    if (prefsKey && typeof prefs !== 'undefined') {
      const saved = prefs.getObject(prefsKey, null);
      if (saved && Array.isArray(saved)) {
        const children = [...container.querySelectorAll(childSelector)];
        const map = {};
        children.forEach((c, i) => { map[getKey(c, i)] = c; });
        for (const key of saved) {
          if (map[key]) container.appendChild(map[key]);
        }
        children.forEach((c, i) => {
          if (!saved.includes(getKey(c, i))) container.appendChild(c);
        });
      }
    }

    function saveOrder() {
      if (!prefsKey || typeof prefs === 'undefined') return;
      const children = [...container.querySelectorAll(childSelector)];
      prefs.setItem(prefsKey, children.map((c, i) => getKey(c, i)));
    }

    container.addEventListener('mousedown', (e) => {
      if (e.button !== 0 || _drag) return;
      const child = e.target.closest(childSelector);
      if (!child || !container.contains(child)) return;
      if (handleSelector && !e.target.closest(handleSelector)) return;
      if (e.target.closest('input, button, select, textarea, a, .btn-small, .col-resize')) return;
      e.preventDefault();
      const rect = child.getBoundingClientRect();
      _drag = {
        container, childSelector, direction, onReorder, saveOrder,
        dragged: child, ghost: null, placeholder: null, isDragging: false,
        startX: e.clientX, startY: e.clientY,
        offsetX: e.clientX - rect.left, offsetY: e.clientY - rect.top,
      };
    });
  };
})();

// Reorder cells in new tbody rows to match current thead column order
function reorderNewTableRows(tableId) {
  const table = document.getElementById(tableId);
  if (!table || !table._colOrder) return;
  const thead = table.querySelector('thead tr');
  if (!thead) return;
  const getColKey = table._getColKey;
  const currentOrder = [...thead.children].map(th => getColKey(th));
  // Build index map: original position → current position
  const defaultOrder = ['col-cb', 'name', 'format', 'col-size', 'col-bpm', 'col-key', 'col-dur', 'col-ch', 'col-lufs', 'modified', 'directory', 'col-actions'];
  if (currentOrder.length === 0) return;
  const indexMap = [];
  for (const key of currentOrder) {
    const origIdx = defaultOrder.indexOf(key);
    indexMap.push(origIdx >= 0 ? origIdx : indexMap.length);
  }
  // Only reorder if order differs from default
  const isDefault = indexMap.every((v, i) => v === i);
  if (isDefault) return;
  const tbody = table.querySelector('tbody');
  if (!tbody) return;
  for (const row of tbody.rows) {
    if (row._colReordered) continue;
    row._colReordered = true;
    const cells = [...row.cells];
    if (cells.length !== indexMap.length) continue;
    const frag = document.createDocumentFragment();
    for (const idx of indexMap) { if (cells[idx]) frag.appendChild(cells[idx]); }
    row.appendChild(frag);
  }
}

// ── Auto-init common reorderable areas ──

document.addEventListener('DOMContentLoaded', () => {
  const headerStats = document.getElementById('headerStats');
  if (headerStats) initDragReorder(headerStats, '.header-info-item', 'headerStatsOrder', { direction: 'horizontal', getKey: (el) => el.textContent.trim().split(/\s+/)[0] });

  const statsBar = document.getElementById('statsBar');
  if (statsBar) initDragReorder(statsBar, '.stat', 'statsBarOrder', { direction: 'horizontal', getKey: (el) => el.textContent.trim().replace(/\d+/g, '').trim() });

  ['audioStats', 'dawStats', 'presetStats'].forEach(id => {
    const bar = document.getElementById(id);
    if (bar) initDragReorder(bar, 'span', id + 'Order', { direction: 'horizontal', getKey: (el) => el.textContent.trim().replace(/[\d,.]+/g, '').replace(/\s+/g, ' ').trim() });
  });

  setTimeout(() => {
    const favGrid = document.getElementById('fileFavsGrid');
    if (favGrid) initDragReorder(favGrid, '.file-fav-chip', 'fileFavOrder', { direction: 'horizontal', getKey: (el) => el.dataset.fileNav || el.textContent.trim() });
  }, 1000);

  // Scan buttons and dashboard — draggable between containers
  initFloatingElement('scanBtnsGroup', 'scanBtnsParent');
  initFloatingElement('dashBtnGroup', 'dashBtnParent');

  // All toolbar buttons — reorderable within their toolbar
  document.querySelectorAll('.audio-toolbar').forEach((toolbar, i) => {
    const tabContent = toolbar.closest('.tab-content');
    const tabId = tabContent?.id || 'toolbar' + i;
    initDragReorder(toolbar, '.btn, .search-box, .filter-select, select', tabId + 'BtnOrder', {
      direction: 'horizontal',
      getKey: (el) => el.dataset.action || el.id || el.textContent.trim().slice(0, 20),
    });
  });
});

// ── Floating element drag (move between containers) ──
function initFloatingElement(elementId, prefsKey) {
  const el = document.getElementById(elementId);
  if (!el) return;

  // Restore saved parent
  if (typeof prefs !== 'undefined') {
    const savedParent = prefs.getItem(prefsKey);
    if (savedParent) {
      const target = document.getElementById(savedParent) || document.querySelector(savedParent);
      if (target && target !== el.parentElement) target.appendChild(el);
    }
  }

  let dragging = false, ghost = null, startX = 0, startY = 0, offsetX = 0, offsetY = 0;

  el.addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.target.closest('input, select, textarea')) return;
    const rect = el.getBoundingClientRect();
    startX = e.clientX; startY = e.clientY;
    offsetX = e.clientX - rect.left; offsetY = e.clientY - rect.top;
    dragging = false;

    const onMove = (ev) => {
      if (!dragging && (Math.abs(ev.clientX - startX) > 5 || Math.abs(ev.clientY - startY) > 5)) {
        dragging = true;
        document.body.style.userSelect = 'none';
        document.body.style.cursor = 'grabbing';
        ghost = el.cloneNode(true);
        ghost.classList.add('trello-ghost');
        ghost.style.cssText = `position:fixed;z-index:20000;width:${rect.width}px;left:${rect.left}px;top:${rect.top}px;pointer-events:none;opacity:0.9;transform:scale(1.05);box-shadow:0 4px 16px rgba(0,0,0,0.5);border:2px solid var(--cyan);border-radius:4px;background:var(--bg-primary);padding:4px 8px;`;
        document.body.appendChild(ghost);
        el.style.opacity = '0.3';
      }
      if (!dragging) return;
      ghost.style.left = (ev.clientX - offsetX) + 'px';
      ghost.style.top = (ev.clientY - offsetY) + 'px';

      // Highlight potential drop targets
      ghost.style.display = 'none';
      const under = document.elementFromPoint(ev.clientX, ev.clientY);
      ghost.style.display = '';
      const dropTarget = under?.closest('.header-actions, .stats-bar, .tab-nav, .audio-toolbar');
      document.querySelectorAll('.header-actions, .stats-bar, .tab-nav').forEach(c => c.style.outline = '');
      if (dropTarget) dropTarget.style.outline = '2px dashed var(--cyan)';
    };

    const onUp = (ev) => {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      if (!dragging) return;
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
      el.style.opacity = '';
      if (ghost) { ghost.remove(); ghost = null; }
      document.querySelectorAll('.header-actions, .stats-bar, .tab-nav').forEach(c => c.style.outline = '');

      // Find drop target and insert at cursor position
      const under = document.elementFromPoint(ev.clientX, ev.clientY);
      const dropTarget = under?.closest('.header-actions, .stats-bar, .tab-nav');
      if (dropTarget) {
        // Find the sibling element nearest to cursor for position-aware insertion
        const sibling = under?.closest('.stat, .header-info-item, .tab-btn, .btn, .scan-btns-group');
        if (sibling && sibling !== el && dropTarget.contains(sibling)) {
          const r = sibling.getBoundingClientRect();
          const mid = r.left + r.width / 2;
          dropTarget.insertBefore(el, ev.clientX < mid ? sibling : sibling.nextSibling);
        } else {
          dropTarget.appendChild(el);
        }
        if (typeof prefs !== 'undefined') prefs.setItem(prefsKey, dropTarget.id || dropTarget.className.split(' ')[0]);
      }
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  });
}

function initFavDragReorder() {
  const favList = document.getElementById('favList');
  if (favList) initDragReorder(favList, '.fav-item', 'favItemOrder', { getKey: (el) => el.querySelector('.fav-name')?.textContent?.trim() || '' });
}

function initRecentlyPlayedDragReorder() {
  const histList = document.getElementById('npHistoryList');
  if (!histList) return;
  histList._trelloDragInit = false; // allow re-init after re-render
  initDragReorder(histList, '.np-history-item', null, {
    getKey: (el) => el.dataset.path || el.getAttribute('data-path') || '',
    onReorder: () => {
      if (typeof recentlyPlayed === 'undefined') return;
      const items = [...histList.querySelectorAll('.np-history-item')];
      const pathOrder = items.map(el => el.dataset.path || el.getAttribute('data-path'));
      const reordered = [];
      for (const p of pathOrder) { const f = recentlyPlayed.find(r => r.path === p); if (f) reordered.push(f); }
      for (const r of recentlyPlayed) { if (!reordered.some(x => x.path === r.path)) reordered.push(r); }
      recentlyPlayed.length = 0;
      recentlyPlayed.push(...reordered);
      if (typeof saveRecentlyPlayed === 'function') saveRecentlyPlayed();
    },
  });
}

// ── Table column reorder (single global listener pair) ──
(function () {
  let _colDrag = null; // { table, thead, prefsKey, getColKey, th, origIdx, ... }

  document.addEventListener('mousemove', (e) => {
    if (!_colDrag) return;
    const c = _colDrag;
    if (!c.isDragging && Math.abs(e.clientX - c.startX) > 5) {
      c.isDragging = true;
      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'grabbing';
      const rect = c.th.getBoundingClientRect();
      c.placeholder = document.createElement('th');
      c.placeholder.className = 'trello-placeholder';
      c.placeholder.style.width = rect.width + 'px';
      c.th.parentNode.insertBefore(c.placeholder, c.th);
      c.ghost = c.th.cloneNode(true);
      c.ghost.classList.add('trello-ghost');
      c.ghost.style.cssText = `position:fixed;z-index:20000;width:${rect.width}px;height:${rect.height}px;left:${rect.left}px;top:${rect.top}px;pointer-events:none;opacity:0.9;transform:scale(1.05);box-shadow:0 4px 16px rgba(0,0,0,0.5);border:2px solid var(--cyan);border-radius:2px;background:var(--bg-primary);`;
      document.body.appendChild(c.ghost);
      c.th.style.display = 'none';
    }
    if (!c.isDragging || !c.ghost) return;
    c.ghost.style.left = (e.clientX - c.offsetX) + 'px';
    c.ghost.style.display = 'none';
    const el = document.elementFromPoint(e.clientX, e.clientY);
    c.ghost.style.display = '';
    const target = el?.closest('th');
    if (target && target !== c.th && target !== c.placeholder && c.thead.contains(target)) {
      const r = target.getBoundingClientRect();
      c.thead.insertBefore(c.placeholder, e.clientX < r.left + r.width / 2 ? target : target.nextSibling);
    }
  });

  document.addEventListener('mouseup', () => {
    if (!_colDrag) return;
    const c = _colDrag;
    if (c.isDragging) {
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
      const newIdx = [...c.thead.children].indexOf(c.placeholder);
      if (c.placeholder?.parentNode) { c.placeholder.parentNode.insertBefore(c.th, c.placeholder); c.placeholder.remove(); }
      c.th.style.display = '';
      if (c.ghost) c.ghost.remove();
      if (c.origIdx !== newIdx && newIdx >= 0) {
        const tbody = c.table.querySelector('tbody');
        if (tbody) {
          for (const row of tbody.rows) {
            const cells = [...row.cells];
            if (c.origIdx < cells.length && newIdx < cells.length) {
              const cell = cells[c.origIdx];
              const ref = cells[newIdx];
              row.insertBefore(cell, c.origIdx < newIdx ? ref.nextSibling : ref);
            }
          }
        }
      }
      if (typeof prefs !== 'undefined') prefs.setItem(c.prefsKey, [...c.thead.children].map(th => c.getColKey(th)));
    }
    _colDrag = null;
  });

  window.initTableColumnReorder = function (tableId, prefsKey) {
    const table = document.getElementById(tableId);
    if (!table) return;
    const thead = table.querySelector('thead tr');
    if (!thead || thead._colDragInit) return;
    thead._colDragInit = true;

    const getColKey = (th) => th.dataset.key || th.className.split(' ').find(c => c.startsWith('col-')) || th.textContent.trim().split(/\s/)[0];

    // Restore saved column order
    const saved = typeof prefs !== 'undefined' ? prefs.getObject(prefsKey, null) : null;
    if (saved && Array.isArray(saved)) {
      const ths = [...thead.children];
      const thMap = {};
      ths.forEach(th => { thMap[getColKey(th)] = th; });
      const newOrder = [];
      for (const key of saved) { if (thMap[key]) { newOrder.push(ths.indexOf(thMap[key])); thead.appendChild(thMap[key]); } }
      ths.forEach(th => { if (!saved.includes(getColKey(th))) { newOrder.push(ths.indexOf(th)); thead.appendChild(th); } });
      const tbody = table.querySelector('tbody');
      if (tbody && newOrder.length > 0) {
        for (const row of tbody.rows) {
          const cells = [...row.cells];
          const frag = document.createDocumentFragment();
          for (const idx of newOrder) { if (cells[idx]) frag.appendChild(cells[idx]); }
          for (const cell of cells) { if (!frag.contains(cell)) frag.appendChild(cell); }
          row.appendChild(frag);
        }
      }
    }

    // Store column order for reordering new rows
    table._colOrder = saved || null;
    table._getColKey = getColKey;

    thead.addEventListener('mousedown', (e) => {
      if (e.button !== 0 || _colDrag) return;
      const th = e.target.closest('th');
      if (!th || e.target.closest('.col-resize, input, button')) return;
      e.preventDefault();
      const rect = th.getBoundingClientRect();
      _colDrag = { table, thead, prefsKey, getColKey, th, origIdx: [...thead.children].indexOf(th), startX: e.clientX, offsetX: e.clientX - rect.left, isDragging: false, ghost: null, placeholder: null };
    });
  };
})();
