// ── Walker Status Tab ──
// Live streaming view of directory walker threads in 4 tiles.

let _walkerInterval = null;

function startWalkerPolling() {
  stopWalkerPolling();
  _walkerIdleCount = 0;
  _updateWalkerTiles();
  _walkerInterval = setInterval(_updateWalkerTiles, 500);
}

function stopWalkerPolling() {
  if (_walkerInterval) { clearInterval(_walkerInterval); _walkerInterval = null; }
}

// Auto-start/stop polling when Walkers tab is visible
document.addEventListener('click', (e) => {
  const tab = e.target.closest('[data-action="switchTab"]');
  if (tab) {
    if (tab.dataset.tab === 'walkers') startWalkerPolling();
    else stopWalkerPolling();
  }
});

let _walkerIdleCount = 0;

async function _updateWalkerTiles() {
  // Only poll if tab is visible
  const tab = document.getElementById('tabWalkers');
  if (!tab || !tab.classList.contains('active')) { stopWalkerPolling(); return; }

  try {
    const status = await window.vstUpdater.getWalkerStatus();
    _renderTile('walkerPluginBody', 'walkerTilePlugin', status.plugin, 'var(--cyan)', status.poolThreads, status.pluginScanning);

    // Unified tile shows when any file-walker (audio/daw/preset/pdf, unified
    // or single-type) is active. The WalkerStatus lists are fed by whichever
    // walker is running, so picking status.audio as the source covers both
    // scan_unified (fan-out) and single-type scans.
    const unifiedTile = document.getElementById('walkerTileUnified');
    const fileWalkerActive = status.unifiedScanning || status.audioScanning
      || status.dawScanning || status.presetScanning || status.pdfScanning;
    if (fileWalkerActive) {
      if (unifiedTile) unifiedTile.style.display = '';
      // During single-type scans, WalkerStatus populates only the active
      // type's dirs list — fall back through them in priority order.
      const dirs = (status.audio && status.audio.length) ? status.audio
        : (status.daw && status.daw.length) ? status.daw
        : (status.preset && status.preset.length) ? status.preset
        : (status.pdf && status.pdf.length) ? status.pdf
        : [];
      _renderTile('walkerUnifiedBody', 'walkerTileUnified', dirs, 'var(--accent)', status.poolThreads, true);
    } else if (unifiedTile) {
      unifiedTile.style.display = 'none';
    }

    // Stop polling after 10 consecutive idle checks (5 seconds)
    const allIdle = !status.pluginScanning && !fileWalkerActive;
    if (allIdle) { _walkerIdleCount++; if (_walkerIdleCount >= 10) stopWalkerPolling(); }
    else { _walkerIdleCount = 0; }
  } catch (err) {
    const body = document.getElementById('walkerUnifiedBody');
    if (body) body.innerHTML = `<div style="color:var(--red);padding:8px;">Error: ${err?.message || err}</div>`;
  }
}

function _renderTile(bodyId, tileId, dirs, color, poolThreads, isScanning) {
  const body = document.getElementById(bodyId);
  const tile = document.getElementById(tileId);
  if (!body || !tile) return;

  const statusEl = tile.querySelector('.walker-tile-status');
  if (!isScanning) {
    if (statusEl) statusEl.innerHTML = `<span style="color:var(--text-dim);">idle — ${poolThreads} threads in pool</span>`;
    if (!dirs || dirs.length === 0) {
      body.innerHTML = '<div style="text-align:center;color:var(--text-dim);padding:24px;font-size:11px;">Waiting for scan to start...</div>';
    }
    tile.style.borderColor = 'var(--border)';
    return;
  }

  tile.style.borderColor = color;
  if (statusEl) statusEl.innerHTML = `<span style="color:${color};font-weight:600;">scanning — ${poolThreads} threads</span> <span style="color:var(--text-dim);">| ${dirs.length} dirs in buffer</span>`;

  // Build dir list — oldest at top, newest at bottom. Buffer is sized (200)
  // to fill a full-height tile; auto-scroll to bottom so the latest dirs
  // stay visible as the stream advances.
  const html = dirs.map(d => {
    return `<div class="walker-dir walker-dir-active" title="${escapeHtml(d)}">${escapeHtml(d)}</div>`;
  }).join('');

  body.innerHTML = html;
  body.scrollTop = body.scrollHeight;
}

// Make walker tiles draggable (Trello-style reorder)
(function initWalkerDrag() {
  const grid = document.getElementById('walkerGrid');
  if (grid && typeof initDragReorder === 'function') {
    initDragReorder(grid, '.walker-tile', 'walkerTileOrder', {
      getKey: (el) => el.id || '',
    });
  }
})();
