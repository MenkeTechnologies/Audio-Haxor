// ── Column Resize ──
function initColumnResize(table) {
  if (!table) return;
  const tableId = table.id;

  requestAnimationFrame(() => {
    const ths = Array.from(table.querySelectorAll('thead th'));
    const tableWidth = table.offsetWidth;

    // Restore saved percentage widths or snapshot current layout (only if visible)
    if (tableWidth > 0) {
      const saved = loadColumnWidths(tableId);
      if (saved && saved.length === ths.length && saved.every(w => w > 0)) {
        ths.forEach((th, i) => { th.style.width = (saved[i] / 100 * tableWidth) + 'px'; });
      } else {
        ths.forEach(th => { th.style.width = th.offsetWidth + 'px'; });
      }
    }

    // Always register resize handlers regardless of visibility
    table.querySelectorAll('thead .col-resize').forEach(handle => {
      handle.addEventListener('mousedown', (e) => {
        e.preventDefault();
        e.stopPropagation();

        const th = handle.closest('th');
        const nextTh = th.nextElementSibling;
        if (!nextTh) return;

        const startX = e.clientX;
        const startWidth = th.offsetWidth;
        const nextStartWidth = nextTh.offsetWidth;
        const minWidth = 40;

        handle.classList.add('resizing');
        document.body.classList.add('col-resizing');

        function onMouseMove(e) {
          const delta = e.clientX - startX;
          const newWidth = Math.max(minWidth, startWidth + delta);
          const newNextWidth = Math.max(minWidth, nextStartWidth - delta);
          if (newWidth >= minWidth && newNextWidth >= minWidth) {
            th.style.width = newWidth + 'px';
            nextTh.style.width = newNextWidth + 'px';
          }
        }

        function onMouseUp() {
          handle.classList.remove('resizing');
          document.body.classList.remove('col-resizing');
          document.removeEventListener('mousemove', onMouseMove);
          document.removeEventListener('mouseup', onMouseUp);
          saveColumnWidths(tableId);
        }

        document.addEventListener('mousemove', onMouseMove);
        document.addEventListener('mouseup', onMouseUp);
      });
    });
  });
}

function saveColumnWidths(tableId) {
  const table = document.getElementById(tableId);
  if (!table) return;
  const tableWidth = table.offsetWidth;
  if (tableWidth <= 0) return;
  const pcts = Array.from(table.querySelectorAll('thead th')).map(th => +(th.offsetWidth / tableWidth * 100).toFixed(2));
  try {
    const allWidths = prefs.getObject('columnWidths', {});
    allWidths[tableId] = pcts;
    prefs.setItem('columnWidths', allWidths);
  } catch {}
}

function loadColumnWidths(tableId) {
  try {
    const allWidths = prefs.getObject('columnWidths', {});
    return allWidths[tableId] || null;
  } catch { return null; }
}
