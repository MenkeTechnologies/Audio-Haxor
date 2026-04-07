// ── Audio Engine tab (separate-process engine: I/O device, plugin graph — UI shell only for now) ──

/**
 * Called when the Audio Engine tab becomes active (`utils.js` `switchTab` → `runPerTabWork`).
 * Idempotent — safe if called multiple times.
 */
function initAudioEngineTab() {
    const root = document.getElementById('tabAudioEngine');
    if (!root || root.dataset.aeInit === '1') return;
    root.dataset.aeInit = '1';
    const statusEl = document.getElementById('aeEngineStatus');
    if (statusEl && typeof catalogFmt === 'function') {
        statusEl.textContent = catalogFmt('ui.ae.status_stub');
    }
}
