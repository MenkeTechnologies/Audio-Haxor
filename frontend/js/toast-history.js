// ── Toast History Viewer ──
// Modal listing every showToast() recorded in window.__toastHistory. Triggered from
// Settings → Notifications. History is recorded in ipc.js::recordToastHistory.

function _thFmt(key, vars) {
    return typeof catalogFmt === 'function' ? catalogFmt(key, vars) : key;
}

function _thEsc(s) {
    if (typeof escapeHtml === 'function') return escapeHtml(s);
    return String(s ?? '').replace(/[&<>"']/g, (c) => ({'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'}[c]));
}

function _thHistory() {
    return Array.isArray(window.__toastHistory) ? window.__toastHistory : [];
}

function _thFormatTime(ms) {
    const d = new Date(ms);
    const hh = String(d.getHours()).padStart(2, '0');
    const mm = String(d.getMinutes()).padStart(2, '0');
    const ss = String(d.getSeconds()).padStart(2, '0');
    return `${hh}:${mm}:${ss}`;
}

function _thFormatFullStamp(ms) {
    const d = new Date(ms);
    return d.toLocaleString();
}

function _thRenderRows(rows) {
    if (!rows.length) {
        return `<p class="th-empty">${_thEsc(_thFmt('ui.toast_history.empty'))}</p>`;
    }
    return rows.map(e => {
        const typeLabel = e.type || 'info';
        const typeClass = `th-type-${typeLabel}`;
        return `<div class="th-row ${typeClass}">
  <span class="th-time" title="${_thEsc(_thFormatFullStamp(e.t))}">${_thEsc(_thFormatTime(e.t))}</span>
  <span class="th-badge ${typeClass}">${_thEsc(typeLabel)}</span>
  <span class="th-msg">${_thEsc(e.message)}</span>
</div>`;
    }).join('');
}

function _thApplyFilter() {
    const input = document.getElementById('toastHistorySearch');
    const list = document.getElementById('toastHistoryList');
    const cnt = document.getElementById('toastHistoryCount');
    const tot = document.getElementById('toastHistoryTotal');
    if (!list) return;
    const all = _thHistory().slice().reverse();
    const q = input ? input.value.trim().toLowerCase() : '';
    const filtered = q
        ? all.filter(e => e.message.toLowerCase().includes(q) || (e.type || '').toLowerCase().includes(q))
        : all;
    list.innerHTML = _thRenderRows(filtered);
    if (cnt) cnt.textContent = String(filtered.length);
    if (tot) tot.textContent = String(all.length);
}

function showToastHistory() {
    document.querySelectorAll('#toastHistoryModal').forEach((el) => el.remove());

    const html = `<div class="modal-overlay modal-visible" id="toastHistoryModal" data-action-modal="closeToastHistory">
  <div class="modal-content th-modal-content">
    <div class="modal-header">
      <h2>${_thEsc(_thFmt('ui.h2.toast_history'))}</h2>
      <button class="modal-close" data-action-modal="closeToastHistory" title="${_thEsc(_thFmt('ui.tt.close_modal'))}">&times;</button>
    </div>
    <div class="modal-body modal-body-list">
      <div class="modal-row-inline">
        <input type="text" class="modal-input-flex" id="toastHistorySearch"
               placeholder="${_thEsc(_thFmt('ui.ph.search_toasts'))}"
               autocomplete="off" autocorrect="off" spellcheck="false">
        <span class="modal-filter-count" id="toastHistoryCount">0</span>
      </div>
      <div class="modal-list-entries th-list" id="toastHistoryList"></div>
      <div class="modal-row-footer">
        <span class="modal-footer-count"><span id="toastHistoryTotal">0</span> ${_thEsc(_thFmt('ui.toast_history.entries_suffix'))}</span>
        <button class="btn btn-secondary btn-compact" data-action-modal="clearToastHistory"
                title="${_thEsc(_thFmt('ui.tt.clear_toast_history'))}">${_thEsc(_thFmt('ui.btn.clear_all'))}</button>
      </div>
    </div>
  </div>
</div>`;
    document.body.insertAdjacentHTML('beforeend', html);

    _thApplyFilter();

    const input = document.getElementById('toastHistorySearch');
    if (input) {
        input.addEventListener('input', () => _thApplyFilter());
        // Focus search field once the modal paints
        requestAnimationFrame(() => { try { input.focus(); } catch (_) {} });
    }
}

function closeToastHistory() {
    document.querySelectorAll('#toastHistoryModal').forEach((el) => el.remove());
}

function clearToastHistory() {
    if (Array.isArray(window.__toastHistory)) window.__toastHistory.length = 0;
    _thApplyFilter();
}

window.showToastHistory = showToastHistory;
window.closeToastHistory = closeToastHistory;
window.clearToastHistory = clearToastHistory;

// Live-update the viewer when toasts arrive while it's open
document.addEventListener('toast-history-update', () => {
    if (document.getElementById('toastHistoryModal')) _thApplyFilter();
});

// ── Event Handlers ──

document.addEventListener('click', (e) => {
    const t = e.target.closest('[data-action-modal]');
    if (!t) return;
    const modal = t.closest('#toastHistoryModal');
    if (!modal) return;
    const act = t.dataset.actionModal;
    if (act === 'closeToastHistory') {
        // Honour clicks on the overlay backdrop and on the close-button only — not stray inner clicks.
        if (e.target === modal || t.classList.contains('modal-close') || e.target === t) {
            closeToastHistory();
        }
    } else if (act === 'clearToastHistory') {
        clearToastHistory();
    }
});

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && document.getElementById('toastHistoryModal')) {
        closeToastHistory();
        e.stopPropagation();
    }
});
