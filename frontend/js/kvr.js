let kvrResolveAbort = false;

async function resolveKvrDownloads() {
  kvrResolveAbort = false;

  // Load existing cache to skip already-resolved plugins
  let kvrCache = {};
  try { kvrCache = await window.vstUpdater.getKvrCache(); } catch {}

  // Deduplicate by name+manufacturer, skip cached
  const seen = new Map();
  const queue = [];
  for (const p of allPlugins) {
    const key = kvrCacheKey(p);
    if (kvrCache[key]) continue; // already cached from previous run
    if (seen.has(key)) continue;
    seen.set(key, true);
    queue.push(p);
  }

  if (queue.length === 0) return;

  currentOperation = 'kvr-resolve';
  showStopButton();
  const statusBar = document.getElementById('statusBar');
  const statusText = document.getElementById('statusText');
  const statusStats = document.getElementById('statusStats');
  statusBar.classList.add('active');
  statusStats.innerHTML = '';

  let resolved = 0;
  let downloads = 0;

  for (const plugin of queue) {
    if (kvrResolveAbort) break;

    const kvrUrl = buildKvrUrl(plugin.name, plugin.manufacturer);
    statusText.textContent = `Resolving KVR: ${plugin.name} (${resolved + 1}/${queue.length})`;

    try {
      const result = await window.vstUpdater.resolveKvr(kvrUrl, plugin.name);
      const productUrl = result.productUrl || kvrUrl;
      const downloadUrl = result.downloadUrl || null;
      const key = kvrCacheKey(plugin);

      // Save to cache
      try {
        await window.vstUpdater.updateKvrCache([{
          key,
          kvrUrl: productUrl,
          updateUrl: downloadUrl,
          source: 'kvr',
        }]);
      } catch {}

      // Update all matching plugin cards
      for (const p of allPlugins) {
        if (p.name === plugin.name && p.manufacturer === plugin.manufacturer) {
          p.kvrUrl = productUrl;
          if (downloadUrl && p.hasUpdate) p.updateUrl = downloadUrl;

          const escapedPath = escapePath(p.path);
          const card = document.querySelector(`.plugin-card[data-path="${CSS.escape(escapedPath)}"]`);
          if (card) {
            const temp = document.createElement('div');
            temp.innerHTML = buildPluginCardHtml(p);
            card.replaceWith(temp.firstElementChild);
          }
        }
      }

      if (downloadUrl) downloads++;
    } catch {}

    resolved++;
    statusStats.innerHTML =
      `<span style="color: var(--green);">${downloads} downloads</span>` +
      `<span class="stat-pending">${queue.length - resolved} pending</span>`;

    // Rate limit
    if (!kvrResolveAbort && resolved < queue.length) {
      await new Promise(r => setTimeout(r, 2000));
    }
  }

  statusBar.classList.remove('active');
  hideStopButton();
}

function stopKvrResolve() {
  kvrResolveAbort = true;
}

function kvrCacheKey(plugin) {
  return `${(plugin.manufacturer || 'Unknown').toLowerCase()}|||${plugin.name.toLowerCase()}`;
}

// Apply cached KVR data to plugins
function applyKvrCache(plugins, cache) {
  for (const p of plugins) {
    const cached = cache[kvrCacheKey(p)];
    if (cached) {
      p.kvrUrl = cached.kvrUrl || p.kvrUrl;
      p.source = cached.source || p.source;
      if (cached.latestVersion && cached.latestVersion !== p.version) {
        p.latestVersion = cached.latestVersion;
        p.currentVersion = p.version;
        p.hasUpdate = cached.hasUpdate || false;
      }
      if (cached.updateUrl && p.hasUpdate) {
        p.updateUrl = cached.updateUrl;
      }
    }
  }
}
