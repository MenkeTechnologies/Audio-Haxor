// ── Walker Status Tab ──
// Live streaming view of directory walker threads in 4 tiles.

let _walkerInterval = null;

function startWalkerPolling() {
  if (_walkerInterval) return;
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

async function _updateWalkerTiles() {
  // Only poll if tab is visible
  const tab = document.getElementById('tabWalkers');
  if (!tab || !tab.classList.contains('active')) { stopWalkerPolling(); return; }

  try {
    const status = await window.vstUpdater.getWalkerStatus();
    _renderTile('walkerPluginBody', 'walkerTilePlugin', status.plugin, 'var(--cyan)', status.poolThreads, status.pluginScanning);
    _renderTile('walkerAudioBody', 'walkerTileAudio', status.audio, 'var(--yellow)', status.poolThreads, status.audioScanning);
    _renderTile('walkerDawBody', 'walkerTileDaw', status.daw, 'var(--magenta)', status.poolThreads, status.dawScanning);
    _renderTile('walkerPresetBody', 'walkerTilePreset', status.preset, 'var(--orange)', status.poolThreads, status.presetScanning);
  } catch (err) {
    const body = document.getElementById('walkerAudioBody');
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

  // Build dir list — show newest at top, truncate path for readability
  const html = dirs.map(d => {
    return `<div class="walker-dir walker-dir-active" title="${escapeHtml(d)}">${escapeHtml(d)}</div>`;
  }).join('');

  body.innerHTML = html;
}
