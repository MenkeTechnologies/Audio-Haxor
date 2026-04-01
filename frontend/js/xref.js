// ── Plugin ↔ DAW Cross-Reference ──

const XREF_FORMATS = new Set(['ALS', 'RPP', 'RPP-BAK']);

// Cache: project path → PluginRef[]
const _xrefCache = {};

// Load persisted xref cache from prefs on startup
(function loadXrefCache() {
  const saved = prefs.getObject('xrefCache', null);
  if (saved && typeof saved === 'object') {
    Object.assign(_xrefCache, saved);
  }
})();

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
});
