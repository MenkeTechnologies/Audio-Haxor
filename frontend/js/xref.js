// ── Plugin ↔ DAW Cross-Reference ──

const XREF_FORMATS = new Set(['ALS', 'RPP', 'RPP-BAK']);

// Cache: project path → PluginRef[]
const _xrefCache = {};

// Load persisted xref cache after prefs are loaded (called from app.js)
function loadXrefCache() {
  const saved = prefs.getObject('xrefCache', null);
  if (saved && typeof saved === 'object') {
    Object.assign(_xrefCache, saved);
  }
}

function saveXrefCache() {
  prefs.setItem('xrefCache', _xrefCache);
}

function isXrefSupported(format) {
  return XREF_FORMATS.has(format);
}

async function getProjectPlugins(projectPath) {
  if (_xrefCache[projectPath]) return _xrefCache[projectPath];
  try {
    const plugins = await window.vstUpdater.extractProjectPlugins(projectPath);
    _xrefCache[projectPath] = plugins;
    saveXrefCache();
    return plugins;
  } catch {
    return [];
  }
}

function showXrefModal(projectName, plugins) {
  let existing = document.getElementById('xrefModal');
  if (existing) existing.remove();

  let body;
  if (plugins.length === 0) {
    body = '<div class="xref-unsupported">No plugins found in this project.</div>';
  } else {
    body = `<div style="color:var(--text-muted);font-size:11px;margin-bottom:12px;">${plugins.length} plugin${plugins.length !== 1 ? 's' : ''} found</div>
    <ul class="xref-list">${plugins.map(p => {
      const typeCls = 'xref-type-' + p.pluginType.toLowerCase();
      return `<li class="xref-item">
        <span class="xref-item-type ${typeCls}">${escapeHtml(p.pluginType)}</span>
        <span class="xref-item-name">${escapeHtml(p.name)}</span>
        <span class="xref-item-mfg">${escapeHtml(p.manufacturer)}</span>
      </li>`;
    }).join('')}</ul>`;
  }

  const html = `<div class="modal-overlay" id="xrefModal" data-action-modal="closeXref">
    <div class="modal-content modal-wide">
      <div class="modal-header">
        <h2>Plugins in ${escapeHtml(projectName)}</h2>
        <button class="modal-close" data-action-modal="closeXref" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">${body}</div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);
}

function closeXrefModal() {
  const modal = document.getElementById('xrefModal');
  if (modal) modal.remove();
}

async function showProjectPlugins(projectPath, projectName) {
  // Show loading modal
  let existing = document.getElementById('xrefModal');
  if (existing) existing.remove();
  const loadHtml = `<div class="modal-overlay" id="xrefModal" data-action-modal="closeXref">
    <div class="modal-content modal-wide">
      <div class="modal-header">
        <h2>Plugins in ${escapeHtml(projectName)}</h2>
        <button class="modal-close" data-action-modal="closeXref" title="Close">&#10005;</button>
      </div>
      <div class="modal-body" style="text-align:center;padding:32px;">
        <div class="spinner" style="width:20px;height:20px;margin:0 auto 12px;"></div>
        <div style="color:var(--text-muted);font-size:12px;">Parsing project file...</div>
      </div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', loadHtml);

  const plugins = await getProjectPlugins(projectPath);
  closeXrefModal();
  showXrefModal(projectName, plugins);
}

// Normalize a plugin name for matching: lowercase, strip arch/platform suffixes, collapse whitespace.
// Mirrors normalize_plugin_name() in xref.rs.
function normalizePluginName(name) {
  let s = name.trim();
  const bracketRe = /\s*[\(\[](x64|x86_64|x86|arm64|aarch64|64-?bit|32-?bit|intel|apple silicon|universal|stereo|mono|vst3?|au|aax)[\)\]]$/i;
  let prev;
  do { prev = s; s = s.replace(bracketRe, ''); } while (s !== prev);
  s = s.replace(/\s+(x64|x86_64|x86|64bit|32bit)$/i, '');
  return s.replace(/\s+/g, ' ').trim().toLowerCase();
}

// Reverse lookup: find all loaded DAW projects that use a given plugin name
function findProjectsUsingPlugin(pluginName) {
  const normalized = normalizePluginName(pluginName);
  const matches = [];
  for (const [path, plugins] of Object.entries(_xrefCache)) {
    if (plugins.some(p => (p.normalizedName || p.name.toLowerCase()) === normalized)) {
      const project = allDawProjects.find(d => d.path === path);
      if (project) matches.push(project);
    }
  }
  return matches;
}

function showReverseXrefModal(pluginName, projects) {
  let existing = document.getElementById('xrefModal');
  if (existing) existing.remove();

  let body;
  if (projects.length === 0) {
    body = `<div class="xref-unsupported">No scanned projects use "${escapeHtml(pluginName)}".<br><br>
      <span style="font-size:11px;color:var(--text-muted);">Tip: Click the &#9889; plugin count badges on DAW project rows to scan them first.</span></div>`;
  } else {
    body = `<div style="color:var(--text-muted);font-size:11px;margin-bottom:12px;">${projects.length} project${projects.length !== 1 ? 's' : ''} use ${escapeHtml(pluginName)}</div>
    <ul class="xref-list">${projects.map(p => {
      const dawClass = getDawBadgeClass ? getDawBadgeClass(p.daw) : 'format-default';
      return `<li class="xref-item" style="cursor:pointer;" data-xref-project="${escapeHtml(p.path)}">
        <span class="format-badge ${dawClass}" style="font-size:10px;">${escapeHtml(p.daw)}</span>
        <span class="xref-item-name">${escapeHtml(p.name)}</span>
        <span class="xref-item-mfg">${escapeHtml(p.directory)}</span>
      </li>`;
    }).join('')}</ul>`;
  }

  const html = `<div class="modal-overlay" id="xrefModal" data-action-modal="closeXref">
    <div class="modal-content modal-wide">
      <div class="modal-header">
        <h2>Projects using ${escapeHtml(pluginName)}</h2>
        <button class="modal-close" data-action-modal="closeXref" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">${body}</div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);
}

// Scan all supported DAW projects in background for xref index
async function buildXrefIndex() {
  const supported = allDawProjects.filter(p => isXrefSupported(p.format));
  let scanned = 0;
  for (const p of supported) {
    if (_xrefCache[p.path]) { scanned++; continue; }
    try {
      await getProjectPlugins(p.path);
    } catch { /* skip errors */ }
    scanned++;
    // Update progress every 10 projects
    if (scanned % 10 === 0) {
      showToast(`Indexing plugins: ${scanned}/${supported.length}...`, 1500);
    }
  }
  saveXrefCache();
  showToast(`Plugin index built: ${supported.length} projects scanned`);
}

// Event delegation
document.addEventListener('click', (e) => {
  // Close xref modal
  const modalAction = e.target.closest('[data-action-modal="closeXref"]');
  if (modalAction) {
    if (e.target === modalAction || modalAction.classList.contains('modal-close')) {
      closeXrefModal();
    }
    return;
  }

  // Click xref badge on DAW row
  const badge = e.target.closest('[data-action="showXref"]');
  if (badge) {
    e.stopPropagation();
    const path = badge.dataset.path;
    const name = badge.dataset.name;
    showProjectPlugins(path, name);
    return;
  }

  // Click project in reverse xref modal
  const projItem = e.target.closest('[data-xref-project]');
  if (projItem) {
    const path = projItem.dataset.xrefProject;
    closeXrefModal();
    switchTab('daw');
    // Focus the project in the table
    setTimeout(() => {
      const row = document.querySelector(`#dawTableBody tr[data-daw-path="${CSS.escape(path)}"]`);
      if (row) row.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }, 200);
    return;
  }
});

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && document.getElementById('xrefModal')) {
    closeXrefModal();
  }
  if (e.key === 'Escape' && document.getElementById('alsViewerModal')) {
    closeAlsViewer();
  }
});

// ── ALS XML Viewer ──
async function showAlsViewer(filePath, projectName) {
  let existing = document.getElementById('alsViewerModal');
  if (existing) existing.remove();

  const loadHtml = `<div class="modal-overlay" id="alsViewerModal" data-action-modal="closeAlsViewer">
    <div class="modal-content" style="max-width:90vw;max-height:90vh;width:900px;">
      <div class="modal-header">
        <h2>${escapeHtml(projectName)} — XML</h2>
        <button class="modal-close" data-action-modal="closeAlsViewer" title="Close">&#10005;</button>
      </div>
      <div class="modal-body" style="padding:0;">
        <div style="text-align:center;padding:32px;"><div class="spinner" style="width:20px;height:20px;margin:0 auto 12px;"></div>Decompressing...</div>
      </div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', loadHtml);

  try {
    const xml = await window.vstUpdater.readAlsXml(filePath);
    const modal = document.getElementById('alsViewerModal');
    if (!modal) return;
    const body = modal.querySelector('.modal-body');

    // Format XML with indentation highlighting
    const escaped = escapeHtml(xml);
    const lines = escaped.split('\n');
    const lineCount = lines.length;

    body.innerHTML = `<div style="display:flex;flex-direction:column;height:calc(90vh - 80px);">
      <div style="padding:8px 12px;background:var(--bg-secondary);border-bottom:1px solid var(--border);display:flex;gap:12px;align-items:center;flex-shrink:0;">
        <span style="font-size:11px;color:var(--text-muted);">${lineCount.toLocaleString()} lines | ${formatAudioSize(xml.length)} uncompressed</span>
        <input type="text" class="np-search-input" id="alsSearchInput" placeholder="Search XML..." style="flex:1;max-width:300px;" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" title="Search within XML content">
        <button class="btn btn-secondary" id="alsExportBtn" style="padding:4px 10px;font-size:10px;" title="Save decompressed XML to file">&#8615; Export XML</button>
      </div>
      <pre id="alsXmlContent" style="flex:1;overflow:auto;margin:0;padding:12px;font-family:'Share Tech Mono',monospace;font-size:11px;line-height:1.5;color:var(--text);background:var(--bg-primary);white-space:pre-wrap;word-break:break-all;tab-size:2;">${escaped}</pre>
    </div>`;

    // Search — fzf fuzzy match with highlighted characters, show only matching lines
    const rawLines = xml.split('\n');
    document.getElementById('alsSearchInput')?.addEventListener('input', (e) => {
      const q = e.target.value.trim();
      const pre = document.getElementById('alsXmlContent');
      if (!pre) return;
      if (!q) { pre.innerHTML = escaped; return; }
      const matched = [];
      for (let i = 0; i < rawLines.length; i++) {
        const line = rawLines[i];
        const score = typeof searchScore === 'function' ? searchScore(q, [line], 'fuzzy') : (line.toLowerCase().includes(q.toLowerCase()) ? 1 : 0);
        if (score > 0) {
          const highlighted = typeof highlightMatch === 'function' ? highlightMatch(line, q, 'fuzzy') : escapeHtml(line);
          matched.push(`<span style="color:var(--text-dim);user-select:none;">${String(i + 1).padStart(5)}</span>  ${highlighted}`);
        }
      }
      pre.innerHTML = matched.length > 0
        ? matched.join('\n')
        : '<span style="color:var(--text-dim);">No matches</span>';
    });

    // Export
    document.getElementById('alsExportBtn')?.addEventListener('click', async () => {
      const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
      if (!dialogApi) return;
      const savePath = await dialogApi.save({
        title: 'Save Decompressed XML',
        defaultPath: projectName.replace(/\\.als$/i, '') + '.xml',
        filters: [{ name: 'XML', extensions: ['xml'] }],
      });
      if (savePath) {
        await window.__TAURI__.core.invoke('write_text_file', { filePath: savePath, contents: xml });
        showToast('XML exported');
      }
    });
  } catch (err) {
    const modal = document.getElementById('alsViewerModal');
    if (modal) {
      modal.querySelector('.modal-body').innerHTML = `<div style="padding:24px;color:var(--red);">Failed to read: ${escapeHtml(err.message || String(err))}</div>`;
    }
  }
}

function closeAlsViewer() {
  const modal = document.getElementById('alsViewerModal');
  if (modal) modal.remove();
}

document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action-modal="closeAlsViewer"]');
  if (action) {
    if (e.target === action || action.classList.contains('modal-close')) {
      closeAlsViewer();
    }
  }
});
