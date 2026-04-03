let scanProgressCleanup = null;

async function scanPlugins(resume = false) {
  showGlobalProgress();
  const btn = document.getElementById('btnScan');
  const resumeBtn = document.getElementById('btnResumeScan');
  const progress = document.getElementById('progressBar');
  const progressFill = progress.querySelector('.progress-fill');
  const list = document.getElementById('pluginList');

  const stopBtn = document.getElementById('btnStopPlugins');
  const excludePaths = resume ? allPlugins.map(p => p.path) : null;

  btn.disabled = true;
  resumeBtn.style.display = 'none';
  stopBtn.style.display = '';
  btn.innerHTML = resume ? '&#8635; Resuming...' : '&#8635; Scanning...';
  progress.classList.add('active');
  progressFill.style.animation = 'none';
  progressFill.style.width = '0%';

  if (!resume) {
    list.innerHTML = '<div class="state-message"><div class="spinner"></div><h2>Scanning for plugins...</h2><p>Discovering plugin files across system directories...</p></div>';
    allPlugins = [];
  }

  // Listen for streaming progress from the worker
  let firstBatch = true;
  const eta = createETA();
  if (scanProgressCleanup) scanProgressCleanup();
  scanProgressCleanup = window.vstUpdater.onScanProgress((data) => {
    if (data.phase === 'start') {
      list.innerHTML = '';
      btn.innerHTML = `&#8635; 0 / ${data.total}`;
      eta.start();
    } else if (data.phase === 'scanning') {
      // Append new plugins to the list incrementally
      allPlugins.push(...data.plugins);
      const pct = Math.round((data.processed / data.total) * 100);
      progressFill.style.width = pct + '%';
      const etaStr = eta.estimate(data.processed, data.total);
      btn.innerHTML = `&#8635; ${data.processed} / ${data.total}${etaStr ? ' — ' + etaStr : ''}`;
      document.getElementById('totalCount').textContent = allPlugins.length;

      // Render the new batch directly into the list
      const fragment = document.createDocumentFragment();
      const temp = document.createElement('div');
      temp.innerHTML = data.plugins.map(p => buildPluginCardHtml(p)).join('');
      while (temp.firstChild) fragment.appendChild(temp.firstChild);
      list.appendChild(fragment);
    }
  });

  try {
    const customDirs = (prefs.getItem('customDirs') || '').split('\n').map(s => s.trim()).filter(Boolean);
    const result = await window.vstUpdater.scanPlugins(customDirs.length ? customDirs : undefined, excludePaths);
    // Final state -- merge with existing on resume
    if (resume) {
      allPlugins = [...allPlugins, ...result.plugins];
    } else {
      allPlugins = result.plugins;
    }

    document.getElementById('totalCount').textContent = allPlugins.length;
    document.getElementById('btnCheckUpdates').disabled = allPlugins.length === 0;

    const dirsSection = document.getElementById('dirsSection');
    dirsSection.style.display = 'block';
    document.getElementById('dirsList').innerHTML = buildDirsTable(result.directories, allPlugins);

    if (allPlugins.length === 0) {
      list.innerHTML = '<div class="state-message"><div class="state-icon">&#128270;</div><h2>No Plugins Found</h2><p>No VST2, VST3, or Audio Unit plugins were found in the standard system directories.</p></div>';
    } else {
      renderPlugins(allPlugins);
      resolveKvrDownloads();
    }
    if (result.stopped && allPlugins.length > 0) {
      resumeBtn.style.display = '';
    }
  } catch (err) {
    const errMsg = err.message || err || 'Unknown error';
    list.innerHTML = `<div class="state-message"><div class="state-icon">&#9888;</div><h2>Scan Error</h2><p>${errMsg}</p></div>`;
    showToast(`Plugin scan failed — ${errMsg}`, 4000, 'error');
  }

  if (scanProgressCleanup) { scanProgressCleanup(); scanProgressCleanup = null; }
  hideGlobalProgress();
  stopBtn.style.display = 'none';
  btn.disabled = false;
  btn.innerHTML = '&#8635; Scan Plugins';
  progressFill.style.width = '100%';
  setTimeout(() => {
    progress.classList.remove('active');
    progressFill.style.animation = '';
    progressFill.style.width = '0%';
  }, 400);
}

function buildPluginCardHtml(p) {
  const typeClass = p.type === 'VST2' ? 'type-vst2' : p.type === 'VST3' ? 'type-vst3' : 'type-au';
  let versionHtml = `<span class="version-current">v${p.version}</span>`;
  let badgeHtml = '';
  const mfgUrl = p.manufacturerUrl || null;
  const mfgBtn = mfgUrl
    ? `<button class="btn-small btn-mfg" data-action="openUpdate" data-url="${mfgUrl}" title="${mfgUrl}">&#127760;</button>`
    : `<button class="btn-small btn-no-web" disabled title="No manufacturer website">&#128683;</button>`;
  const kvrUrl = p.kvrUrl || buildKvrUrl(p.name, p.manufacturer);
  const kvrBtn = `<button class="btn-small btn-kvr" data-action="openKvr" data-url="${kvrUrl.replace(/'/g, '&apos;')}" data-name="${escapeHtml(p.name)}" title="${escapeHtml(kvrUrl)}">KVR</button>`;
  // Show download button only if plugin has an update available
  const dlUrl = (p.hasUpdate && p.updateUrl) ? p.updateUrl : null;
  const dlBtn = dlUrl
    ? `<button class="btn-small btn-download btn-dl-kvr" data-action="openUpdate" data-url="${dlUrl.replace(/'/g, '&apos;')}" title="${escapeHtml(dlUrl)}">&#11015; Download</button>`
    : '';
  let actionsHtml = dlBtn + kvrBtn + mfgBtn + `<button class="btn-small btn-folder" data-action="openFolder" data-path="${escapeHtml(p.path)}" title="${escapeHtml(p.path)}">&#128193;</button>`;

  if (p.hasUpdate !== undefined) {
    if (p.hasUpdate) {
      versionHtml = `<span class="version-current">v${p.currentVersion}</span>
        <span class="version-arrow">&#8594;</span>
        <span class="version-latest">v${p.latestVersion}</span>`;
      badgeHtml = '<span class="badge badge-update">Update Available</span>';
    } else if (p.source === 'not-found') {
      badgeHtml = '<span class="badge badge-unknown">Unknown Latest</span>';
    } else {
      badgeHtml = '<span class="badge badge-current">Up to Date</span>';
    }
  }

  return `
    <div class="plugin-card" data-path="${escapeHtml(p.path)}">
      <div class="plugin-info">
        <h3>${highlightMatch(p.name, _lastPluginSearch, _lastPluginMode)}${typeof rowBadges === 'function' ? ' ' + rowBadges(p.path) : ''}</h3>
        <div class="plugin-meta">
          <span class="plugin-type ${typeClass}">${p.type}</span>
          <span>${escapeHtml(p.manufacturer)}</span>
          <span>${p.size}</span>
          <span>${p.modified}</span>
          ${(p.architectures && p.architectures.length) ? p.architectures.map(a => `<span class="arch-badge arch-${a.toLowerCase()}">${a}</span>`).join('') : ''}
        </div>
      </div>
      <div class="plugin-version">${versionHtml}</div>
      ${badgeHtml}
      <div class="plugin-actions">${actionsHtml}</div>
    </div>`;
}

let updateProgressCleanup = null;

async function checkUpdates() {
  showGlobalProgress();
  const btn = document.getElementById('btnCheckUpdates');
  const progress = document.getElementById('progressBar');
  const progressFill = progress.querySelector('.progress-fill');

  const statusBar = document.getElementById('statusBar');
  const statusText = document.getElementById('statusText');
  const statusStats = document.getElementById('statusStats');

  currentOperation = 'updates';
  showStopButton();
  btn.disabled = true;
  btn.innerHTML = '&#9889; Checking...';
  progress.classList.add('active');
  progressFill.style.animation = 'none';
  progressFill.style.width = '0%';
  statusBar.classList.add('active');
  statusText.textContent = 'Initializing update check...';
  statusStats.innerHTML = '';

  // Track which plugins have been updated (by path)
  const updatedByPath = new Map();

  const updateEta = createETA();
  if (updateProgressCleanup) updateProgressCleanup();
  updateProgressCleanup = window.vstUpdater.onUpdateProgress((data) => {
    if (data.phase === 'start') {
      btn.innerHTML = `&#9889; 0 / ${data.total}`;
      statusText.textContent = `Searching for updates across ${data.total} plugins...`;
      updateEta.start();
    } else if (data.phase === 'checking') {
      const pct = Math.round((data.processed / data.total) * 100);
      progressFill.style.width = pct + '%';
      const updateEtaStr = updateEta.estimate(data.processed, data.total);
      btn.innerHTML = `&#9889; ${data.processed} / ${data.total}${updateEtaStr ? ' — ' + updateEtaStr : ''}`;

      // Show current plugin being checked
      const lastPlugin = data.plugins[data.plugins.length - 1];
      if (lastPlugin) {
        const mfg = lastPlugin.manufacturer !== 'Unknown' ? lastPlugin.manufacturer + ' ' : '';
        const updateRemaining = updateEta.estimate(data.processed, data.total);
        statusText.textContent = `Checking ${mfg}${lastPlugin.name} (${data.processed}/${data.total})${updateRemaining ? ' — ' + updateRemaining + ' remaining' : ''}`;
      }

      // Update individual plugin cards in-place and save to KVR cache
      const cacheEntries = [];
      for (const p of data.plugins) {
        updatedByPath.set(p.path, p);
        // Update allPlugins array entry
        const idx = allPlugins.findIndex(ap => ap.path === p.path);
        if (idx !== -1) allPlugins[idx] = p;
        // Queue cache entry
        if (p.source && p.source !== 'not-found') {
          cacheEntries.push({
            key: kvrCacheKey(p),
            kvrUrl: p.kvrUrl || null,
            updateUrl: p.updateUrl || null,
            latestVersion: p.latestVersion || null,
            hasUpdate: p.hasUpdate || false,
            source: p.source,
          });
        }
        // Find and replace the card in the DOM
        const htmlPath = escapeHtml(p.path);
        const card = document.querySelector(`.plugin-card[data-path="${CSS.escape(htmlPath)}"]`);
        if (card) {
          const temp = document.createElement('div');
          temp.innerHTML = buildPluginCardHtml(p);
          const newCard = temp.firstElementChild;
          card.replaceWith(newCard);
        }
      }

      // Persist to KVR cache
      if (cacheEntries.length > 0) {
        window.vstUpdater.updateKvrCache(cacheEntries).catch(() => showToast('Cache write failed', 4000, 'error'));
      }

      // Update live stats
      const checkedPlugins = [...updatedByPath.values()];
      const withUpdates = checkedPlugins.filter(p => p.hasUpdate).length;
      const unknown = checkedPlugins.filter(p => !p.hasUpdate && p.source === 'not-found').length;
      const upToDate = checkedPlugins.filter(p => !p.hasUpdate && p.source !== 'not-found').length;
      const kvrFound = checkedPlugins.filter(p => p.source === 'kvr' || p.source === 'kvr-ddg').length;
      const pending = data.total - data.processed;
      document.getElementById('updateCount').textContent = withUpdates;
      document.getElementById('upToDateCount').textContent = upToDate;
      document.getElementById('unknownCount').textContent = unknown;

      statusStats.innerHTML =
        `<span class="stat-avail">${withUpdates} updates</span>` +
        `<span class="stat-up">${upToDate} current</span>` +
        `<span style="color: var(--text-muted);">${unknown} unknown</span>` +
        `<span style="color: var(--yellow);">${kvrFound} KVR</span>` +
        `<span class="stat-pending">${pending} pending</span>`;
    }
  });

  try {
    allPlugins = await window.vstUpdater.checkUpdates(allPlugins);
    pluginsWithUpdates = allPlugins.filter(p => p.hasUpdate);

    const finalUnknown = allPlugins.filter(p => !p.hasUpdate && p.source === 'not-found').length;
    document.getElementById('upToDateCount').textContent =
      allPlugins.filter(p => !p.hasUpdate && p.source !== 'not-found').length;
    document.getElementById('updateCount').textContent = pluginsWithUpdates.length;
    document.getElementById('unknownCount').textContent = finalUnknown;

    if (pluginsWithUpdates.length > 0) {
      const batchBar = document.getElementById('batchBar');
      batchBar.classList.add('visible');
      document.getElementById('batchCount').textContent =
        `${pluginsWithUpdates.length} plugin${pluginsWithUpdates.length > 1 ? 's' : ''} with updates`;
      batchIndex = 0;
      updateBatchUI();
    }

    renderPlugins(allPlugins);
  } catch (err) {
    const updateErr = err.message || err || 'Unknown error';
    if (updateErr !== 'stopped') {
      showToast(`Update check failed — ${updateErr}`, 4000, 'error');
    }
  }

  if (updateProgressCleanup) { updateProgressCleanup(); updateProgressCleanup = null; }
  hideGlobalProgress();
  hideStopButton();
  statusBar.classList.remove('active');
  btn.disabled = false;
  btn.innerHTML = '&#9889; Check Updates';
  progressFill.style.width = '100%';
  setTimeout(() => {
    progress.classList.remove('active');
    progressFill.style.animation = '';
    progressFill.style.width = '0%';
  }, 400);
}

function renderPlugins(plugins) {
  updateExportButton();
  const list = document.getElementById('pluginList');

  if (plugins.length === 0) {
    list.innerHTML = '<div class="state-message"><div class="state-icon">&#128269;</div><h2>No matching plugins</h2></div>';
    return;
  }

  list.innerHTML = plugins.map(p => buildPluginCardHtml(p)).join('');
  list.classList.add('fade-in');
  if (typeof updatePluginDiskUsage === 'function') updatePluginDiskUsage();
  if (typeof initDragReorder === 'function') {
    initDragReorder(list, '.plugin-card', null, {
      getKey: (el) => el.dataset.path || '',
    });
  }
}

// Debounce helper — fires immediately on first call, then debounces
function debounce(fn, ms) {
  let timer;
  let lastCall = 0;
  return function(...args) {
    clearTimeout(timer);
    const now = performance.now();
    if (now - lastCall >= ms) {
      lastCall = now;
      fn.apply(this, args);
    } else {
      timer = setTimeout(() => {
        lastCall = performance.now();
        fn.apply(this, args);
      }, ms);
    }
  };
}

// Cache lowercased names to avoid repeated allocations during filtering
function ensureSearchCache(plugins) {
  for (const p of plugins) {
    if (p._nameLower === undefined) {
      p._nameLower = p.name.toLowerCase();
      p._mfgLower = (p.manufacturer || '').toLowerCase();
    }
  }
}

let _lastPluginSearch = '';
let _lastPluginMode = 'fuzzy';

const _filterPluginsImmediate = function() {
  if (typeof saveAllFilterStates === 'function') saveAllFilterStates();
  const search = document.getElementById('searchInput').value;
  const typeEl = document.getElementById('typeFilter');
  autoSelectDropdown(typeEl, search);
  const typeSet = getMultiFilterValues('typeFilter');
  const statusSet = getMultiFilterValues('statusFilter');
  const mode = getSearchMode('regexPlugins');
  _lastPluginSearch = search;
  _lastPluginMode = mode;

  let scored = [];
  for (const p of allPlugins) {
    if (typeof passesGlobalTagFilter === 'function' && !passesGlobalTagFilter(p.path)) continue;
    if (typeSet && !typeSet.has(p.type)) continue;
    if (statusSet) {
      let matchesStatus = false;
      if (statusSet.has('update') && p.hasUpdate === true) matchesStatus = true;
      if (statusSet.has('current') && p.hasUpdate === false && p.source !== 'not-found') matchesStatus = true;
      if (statusSet.has('unknown') && !p.hasUpdate && p.source === 'not-found') matchesStatus = true;
      if (!matchesStatus) continue;
    }
    const score = searchScore(search, [p.name, p.manufacturer || ''], mode);
    if (score > 0) scored.push({ plugin: p, score });
  }
  // Sort by score descending when searching, preserve original order otherwise
  if (search) scored.sort((a, b) => b.score - a.score);
  renderPlugins(scored.map(s => s.plugin));
};

const filterPlugins = debounce(_filterPluginsImmediate, 120);

function openUpdate(url) {
  window.vstUpdater.openUpdateUrl(url);
}

async function openKvr(btn, directUrl, pluginName) {
  const origText = btn.textContent;
  btn.textContent = '...';
  btn.disabled = true;
  try {
    const result = await window.vstUpdater.resolveKvr(directUrl, pluginName);
    const productUrl = result.productUrl || directUrl;
    btn.title = productUrl;
    btn.onclick = () => openUpdate(productUrl);
    window.vstUpdater.openUpdateUrl(productUrl);

    // If a download link was found and plugin has an update, add download button
    if (result.downloadUrl) {
      const card = btn.closest('.plugin-card');
      const pluginPath = card ? card.dataset.path : null;
      const plugin = pluginPath && allPlugins.find(p => p.path === pluginPath);
      if (plugin && plugin.hasUpdate && card && !card.querySelector('.btn-dl-kvr')) {
        const dlBtn = document.createElement('button');
        dlBtn.className = 'btn-small btn-download btn-dl-kvr';
        dlBtn.title = result.downloadUrl;
        dlBtn.innerHTML = '&#11015; Download';
        dlBtn.onclick = () => openUpdate(result.downloadUrl);
        btn.parentNode.insertBefore(dlBtn, btn);
      }
    }
  } catch {
    window.vstUpdater.openUpdateUrl(directUrl);
  }
  btn.textContent = origText;
  btn.disabled = false;
}

function openFolder(pluginPath) {
  window.vstUpdater.openPluginFolder(pluginPath);
}

let batchIndex = 0;

function updateBatchUI() {
  const progress = document.getElementById('batchProgress');
  const nameEl = document.getElementById('batchCurrentName');
  const btnNext = document.getElementById('btnNext');
  const btnSkip = document.getElementById('btnSkip');

  if (batchIndex >= pluginsWithUpdates.length) {
    progress.textContent = 'All done!';
    nameEl.textContent = '';
    btnNext.disabled = true;
    btnNext.textContent = 'All Done';
    btnSkip.style.display = 'none';
    return;
  }

  const current = pluginsWithUpdates[batchIndex];
  progress.textContent = `${batchIndex + 1} of ${pluginsWithUpdates.length}`;
  nameEl.textContent = `Next: ${current.name}`;
  btnNext.disabled = false;
  btnNext.textContent = 'Open Next Update';
  btnSkip.style.display = '';
}

function openNextUpdate() {
  if (batchIndex >= pluginsWithUpdates.length) return;
  const plugin = pluginsWithUpdates[batchIndex];
  if (plugin.updateUrl) {
    window.vstUpdater.openUpdateUrl(plugin.updateUrl);
  }
  batchIndex++;
  updateBatchUI();
}

function skipUpdate() {
  if (batchIndex >= pluginsWithUpdates.length) return;
  batchIndex++;
  updateBatchUI();
}

