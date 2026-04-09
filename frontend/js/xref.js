// ── Plugin ↔ DAW Cross-Reference ──

const XREF_FORMATS = new Set(['ALS', 'RPP', 'RPP-BAK', 'BWPROJECT', 'SONG', 'DAWPROJECT', 'FLP', 'LOGICX', 'CPR', 'NPR', 'PTX', 'PTF', 'REASON']);

// Shared fzf tree search for all DAW content viewers (ALS XML, Bitwig JSON, project viewer)
function _searchTreeNodes(container, query) {
    const nodes = container.querySelectorAll('.xml-node');
    if (!query) {
        nodes.forEach(n => {
            n.style.display = '';
        });
        // Remove highlights
        container.querySelectorAll('.search-hl').forEach(hl => {
            hl.replaceWith(document.createTextNode(hl.textContent));
        });
        return;
    }
    nodes.forEach(node => {
        const textEls = node.querySelectorAll('.xml-tag, .xml-attrs, .xml-text, .json-key, .json-string, .json-number');
        const directText = textEls.length > 0 ? [...textEls].map(e => e.textContent).join(' ') : node.firstChild?.textContent || '';
        const match = searchScore(query, [directText], 'fuzzy') > 0;
        node.style.display = match ? '' : 'none';
        // Highlight matching text
        if (match && textEls.length > 0) {
            textEls.forEach(el => {
                if (typeof highlightMatch === 'function') {
                    el.innerHTML = highlightMatch(el.textContent, query, 'fuzzy');
                }
            });
        }
        // Expand parent chain
        if (match) {
            let parent = node.parentElement?.closest('.xml-node');
            while (parent) {
                parent.style.display = '';
                const ch = parent.querySelector('.xml-children');
                if (ch) ch.style.display = '';
                const tg = parent.querySelector('.xml-toggle');
                if (tg) tg.textContent = '▼';
                const sm = parent.querySelector('.xml-collapsed-summary');
                if (sm) sm.style.display = 'none';
                parent = parent.parentElement?.closest('.xml-node');
            }
        }
    });
}

// Cache: project path → PluginRef[]
const _xrefCache = {};

// Load persisted xref cache after prefs are loaded (called from app.js)
async function loadXrefCache() {
    let saved = null;
    try {
        saved = await window.vstUpdater.readCacheFile('xref-cache.json');
    } catch (e) {
        if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
    }
    if (!saved || Object.keys(saved).length === 0) saved = prefs.getObject('xrefCache', null);
    if (saved && typeof saved === 'object') {
        Object.assign(_xrefCache, saved);
    }
    // Clean old prefs key
    prefs.removeItem('xrefCache');
}

function saveXrefCache() {
    window.vstUpdater.writeCacheFile('xref-cache.json', _xrefCache).catch(() => showToast(toastFmt('toast.cache_write_failed'), 4000, 'error'));
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
        body = '<div class="state-message"><div class="state-icon">&#9889;</div><h2>No plugins found in this project</h2></div>';
    } else {
        body = `<div style="color:var(--text-muted);font-size:11px;margin-bottom:12px;">${plugins.length} plugin${plugins.length !== 1 ? 's' : ''} found</div>
    <ul class="xref-list">${plugins.map(p => {
            const typeCls = 'xref-type-' + p.pluginType.toLowerCase();
            return `<li class="xref-item" data-xref-plugin="${escapeHtml(p.name)}" style="cursor:pointer;">
        <span class="xref-item-type ${typeCls}">${escapeHtml(p.pluginType)}</span>
        <span class="xref-item-name">${escapeHtml(p.name)}</span>
        <span class="xref-item-mfg">${escapeHtml(p.manufacturer)}</span>
      </li>`;
        }).join('')}</ul>`;
    }

    const exportBtn = plugins.length > 0
        ? `<button class="btn btn-secondary" data-action="exportXrefPlugins" style="margin-left:auto;font-size:11px;" title="Export plugin list">&#8615; Export</button>`
        : '';
    window._xrefExportPlugins = plugins;
    window._xrefExportProjectName = projectName;
    window._xrefExportProjectPath = window._xrefLastProjectPath || '';

    const html = `<div class="modal-overlay" id="xrefModal" data-action-modal="closeXref">
    <div class="modal-content modal-wide">
      <div class="modal-header" style="display:flex;align-items:center;gap:8px;">
        <h2>Plugins in ${escapeHtml(projectName)}</h2>
        ${exportBtn}
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
    window._xrefLastProjectPath = projectPath;
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
    do {
        prev = s;
        s = s.replace(bracketRe, '');
    } while (s !== prev);
    s = s.replace(/\s+(x64|x86_64|x86|64bit|32bit)$/i, '');
    return s.replace(/\s+/g, ' ').trim().toLowerCase();
}

/** Stable key for xref PluginRef rows: must match `normalizePluginName` for installed plugins (orphan detection, dep graph). */
function xrefPluginRefKey(p) {
    if (p.normalizedName) return p.normalizedName;
    return typeof normalizePluginName === 'function' ? normalizePluginName(p.name) : String(p.name || '').trim().toLowerCase();
}

/** When the DAW list omits a path (SQLite pagination), derive labels from the path — same shape as dep-graph fallbacks. */
function xrefProjectFromPath(path) {
    const name = path.split('/').pop() || path;
    const directory = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
    return {name, path, daw: '—', format: '', directory};
}

// Reverse lookup: find all loaded DAW projects that use a given plugin name
function findProjectsUsingPlugin(pluginName) {
    const normalized = normalizePluginName(pluginName);
    const matches = [];
    for (const [path, plugins] of Object.entries(_xrefCache)) {
        if (plugins.some(p => xrefPluginRefKey(p) === normalized)) {
            const project = typeof findByPath === 'function' && typeof allDawProjects !== 'undefined'
                ? findByPath(allDawProjects, path)
                : undefined;
            matches.push(project || xrefProjectFromPath(path));
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
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.building_plugin_index'), 5000);
    }
    if (typeof yieldToBrowser === 'function') await yieldToBrowser();

    let supported = [];
    if (typeof fetchAllDawProjectsForXref === 'function') {
        try {
            const fromDb = await fetchAllDawProjectsForXref();
            supported = fromDb.filter(p => isXrefSupported(p.format));
        } catch { /* fall through to in-memory list */
        }
    }
    if (supported.length === 0) {
        supported = allDawProjects.filter(p => isXrefSupported(p.format));
    }
    if (supported.length === 0) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.plugin_index_no_daw_projects'), 4500, 'warning');
        }
        return;
    }

    if (typeof showGlobalProgress === 'function') showGlobalProgress();
    try {
        const total = supported.length;
        let scanned = 0;
        let lastBand = -1;
        for (const p of supported) {
            if (_xrefCache[p.path]) {
                scanned++;
                continue;
            }
            try {
                await getProjectPlugins(p.path);
            } catch { /* skip errors */
            }
            scanned++;
            const pct = total ? (100 * scanned) / total : 100;
            const band = Math.min(4, Math.floor(pct / 25));
            if (band > lastBand && band < 4) {
                lastBand = band;
                showToast(toastFmt('toast.indexing_plugins', {scanned, total}), 2500);
            } else if (band > lastBand) {
                lastBand = band;
            }
        }
        saveXrefCache();
        showToast(toastFmt('toast.plugin_index_built', {n: supported.length}));
    } finally {
        if (typeof hideGlobalProgress === 'function') hideGlobalProgress();
    }
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
            if (row) row.scrollIntoView({behavior: 'smooth', block: 'center'});
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

// ── ALS XML Tree Builder ──
function buildXmlTree(node, depth) {
    const el = document.createElement('div');
    el.className = 'xml-node';
    el.style.paddingLeft = (depth * 16) + 'px';

    if (node.nodeType === 3) {
        // Text node
        const text = node.textContent.trim();
        if (text) {
            el.innerHTML = `<span class="xml-text">${escapeHtml(text)}</span>`;
        }
        return el;
    }

    if (node.nodeType !== 1) return el; // skip non-element nodes

    const tagName = node.tagName;
    const childElements = [...node.childNodes].filter(c =>
        (c.nodeType === 1) || (c.nodeType === 3 && c.textContent.trim())
    );
    const hasChildren = childElements.length > 0;

    // Build attributes string
    let attrsHtml = '';
    if (node.attributes.length > 0) {
        const attrs = [...node.attributes].map(a =>
            ` <span class="xml-attr-name">${escapeHtml(a.name)}</span>=<span class="xml-attr-val">"${escapeHtml(a.value)}"</span>`
        ).join('');
        attrsHtml = `<span class="xml-attrs">${attrs}</span>`;
    }

    if (hasChildren) {
        const childCount = [...node.children].length;
        const summary = childCount > 0 ? `<span class="xml-collapsed-summary" style="display:none;color:var(--text-dim);font-size:10px;"> ...${childCount} children</span>` : '';
        el.innerHTML = `<span class="xml-toggle" title="Click to collapse/expand" style="cursor:pointer;color:var(--cyan);display:inline-block;width:14px;text-align:center;user-select:none;">▼</span><span class="xml-tag" style="color:var(--cyan);">&lt;${escapeHtml(tagName)}</span>${attrsHtml}<span style="color:var(--cyan);">&gt;</span>${summary}`;

        const childContainer = document.createElement('div');
        childContainer.className = 'xml-children';
        for (const child of childElements) {
            childContainer.appendChild(buildXmlTree(child, depth + 1));
        }
        el.appendChild(childContainer);

        // Closing tag
        const closeTag = document.createElement('div');
        closeTag.style.paddingLeft = (depth * 16) + 'px';
        closeTag.innerHTML = `<span style="display:inline-block;width:14px;"></span><span style="color:var(--cyan);">&lt;/${escapeHtml(tagName)}&gt;</span>`;
        closeTag.className = 'xml-close-tag';
        childContainer.appendChild(closeTag);

        // Auto-collapse deep nodes (depth > 2) to keep initial view manageable
        if (depth > 2) {
            childContainer.style.display = 'none';
            el.querySelector('.xml-toggle').textContent = '▶';
            const sm = el.querySelector('.xml-collapsed-summary');
            if (sm) sm.style.display = '';
        }
    } else {
        // Self-closing or text-only
        const textContent = node.textContent.trim();
        if (textContent && !node.children.length) {
            el.innerHTML = `<span style="display:inline-block;width:14px;"></span><span class="xml-tag" style="color:var(--cyan);">&lt;${escapeHtml(tagName)}</span>${attrsHtml}<span style="color:var(--cyan);">&gt;</span><span class="xml-text">${escapeHtml(textContent)}</span><span style="color:var(--cyan);">&lt;/${escapeHtml(tagName)}&gt;</span>`;
        } else {
            el.innerHTML = `<span style="display:inline-block;width:14px;"></span><span class="xml-tag" style="color:var(--cyan);">&lt;${escapeHtml(tagName)}</span>${attrsHtml}<span style="color:var(--cyan);"> /&gt;</span>`;
        }
    }

    return el;
}

// ── Generic Project Viewer (routes to XML or Tree) ──
async function showProjectViewer(filePath, projectName) {
    const ext = filePath.split('.').pop().toLowerCase();
    // XML-based formats: use readProjectFile which returns {type, format, content}
    const xmlFormats = ['als', 'song', 'dawproject'];
    const textFormats = ['rpp', 'rpp-bak'];
    if (xmlFormats.includes(ext)) {
        showXmlProjectViewer(filePath, projectName);
    } else if (textFormats.includes(ext)) {
        showTextProjectViewer(filePath, projectName);
    } else {
        // All binary formats: Bitwig, FLP, Logic, Cubase, Pro Tools, Reason
        showBinaryProjectViewer(filePath, projectName);
    }
}

async function showXmlProjectViewer(filePath, projectName) {
    let existing = document.getElementById('projectViewerModal');
    if (existing) existing.remove();
    const loadHtml = `<div class="modal-overlay" id="projectViewerModal" data-action-modal="closeProjectViewer">
    <div class="modal-content" style="max-width:90vw;max-height:90vh;width:900px;">
      <div class="modal-header"><h2>${escapeHtml(projectName)} — XML</h2><button class="modal-close" data-action-modal="closeProjectViewer" title="Close">&#10005;</button></div>
      <div class="modal-body" style="padding:0;"><div style="text-align:center;padding:32px;"><div class="spinner" style="width:20px;height:20px;margin:0 auto 12px;"></div>Loading...</div></div>
    </div></div>`;
    document.body.insertAdjacentHTML('beforeend', loadHtml);
    try {
        const result = await window.vstUpdater.readProjectFile(filePath);
        const modal = document.getElementById('projectViewerModal');
        if (!modal) return;
        const body = modal.querySelector('.modal-body');
        const xml = result.content || '';
        const lineCount = xml.split('\n').length;
        // Reuse the ALS viewer rendering
        body.innerHTML = `<div style="display:flex;flex-direction:column;height:calc(90vh - 80px);">
      <div style="padding:8px 12px;background:var(--bg-secondary);border-bottom:1px solid var(--border);display:flex;gap:12px;align-items:center;flex-shrink:0;">
        <span style="font-size:11px;color:var(--text-muted);">${result.format || 'XML'} | ${lineCount.toLocaleString()} lines</span>
        <input type="text" class="np-search-input" id="projSearchInput" placeholder="Search XML..." style="flex:1;max-width:300px;" autocomplete="off">
        <button class="btn btn-secondary" id="projCollapseAllBtn" style="padding:4px 10px;font-size:10px;">Collapse All</button>
        <button class="btn btn-secondary" id="projExpandAllBtn" style="padding:4px 10px;font-size:10px;">Expand All</button>
        <button class="btn btn-secondary" id="projExportBtn" style="padding:4px 10px;font-size:10px;">&#8615; Export</button>
      </div>
      <div id="projXmlTree" style="flex:1;overflow:auto;padding:8px 12px;font-family:'Share Tech Mono',monospace;font-size:11px;line-height:1.6;color:var(--text);background:var(--bg-primary);"></div>
    </div>`;
        // Parse XML and render collapsible tree (cap at 10MB to prevent OOM)
        const treeContainer = document.getElementById('projXmlTree');
        if (xml.length > 10_000_000) {
            treeContainer.innerHTML = `<pre style="white-space:pre-wrap;word-break:break-all;">${escapeHtml(xml.slice(0, 500_000))}\n\n<!-- File too large for tree view (${Math.round(xml.length / 1024 / 1024)}MB). Showing first 500KB as text. --></pre>`;
        } else {
            const parser = new DOMParser();
            const xmlDoc = parser.parseFromString(xml, 'text/xml');
            if (xmlDoc.documentElement && typeof buildXmlTree === 'function') {
                treeContainer.appendChild(buildXmlTree(xmlDoc.documentElement, 0));
            } else {
                treeContainer.textContent = xml;
            }
        }
        // Collapse/expand click handler
        treeContainer.addEventListener('click', (ev) => {
            const toggle = ev.target.closest('.xml-toggle');
            if (!toggle) return;
            const node = toggle.closest('.xml-node');
            if (!node) return;
            const children = node.querySelector('.xml-children');
            if (!children) return;
            const collapsed = children.style.display === 'none';
            children.style.display = collapsed ? '' : 'none';
            toggle.textContent = collapsed ? '▼' : '▶';
            const summary = node.querySelector('.xml-collapsed-summary');
            if (summary) summary.style.display = collapsed ? 'none' : '';
        });
        // Collapse All / Expand All
        document.getElementById('projCollapseAllBtn')?.addEventListener('click', () => {
            treeContainer.querySelectorAll('.xml-children').forEach(c => {
                c.style.display = 'none';
            });
            treeContainer.querySelectorAll('.xml-toggle').forEach(t => {
                t.textContent = '▶';
            });
            treeContainer.querySelectorAll('.xml-collapsed-summary').forEach(s => {
                s.style.display = '';
            });
        });
        document.getElementById('projExpandAllBtn')?.addEventListener('click', () => {
            treeContainer.querySelectorAll('.xml-children').forEach(c => {
                c.style.display = '';
            });
            treeContainer.querySelectorAll('.xml-toggle').forEach(t => {
                t.textContent = '▼';
            });
            treeContainer.querySelectorAll('.xml-collapsed-summary').forEach(s => {
                s.style.display = 'none';
            });
        });
        // Search
        let _projSearchTimer;
        document.getElementById('projSearchInput')?.addEventListener('input', (e) => {
            clearTimeout(_projSearchTimer);
            _projSearchTimer = setTimeout(() => _searchTreeNodes(treeContainer, e.target.value.trim()), 200);
        });
        // Export
        document.getElementById('projExportBtn')?.addEventListener('click', async () => {
            const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
            if (!dialogApi) return;
            const savePath = await dialogApi.save({
                title: catalogFmt('ui.dialog.export_xml'),
                defaultPath: projectName + '.xml',
                filters: [{name: catalogFmt('ui.file_filter.xml'), extensions: ['xml']}],
            });
            if (savePath) {
                await window.vstUpdater.writeTextFile(savePath, xml);
                showToast(toastFmt('toast.xml_exported'));
            }
        });
    } catch (e) {
        const modal = document.getElementById('projectViewerModal');
        if (modal) modal.querySelector('.modal-body').innerHTML = '<div style="padding:20px;color:var(--red);">Error: ' + escapeHtml(String(e)) + '</div>';
    }
}

async function showTextProjectViewer(filePath, projectName) {
    let existing = document.getElementById('projectViewerModal');
    if (existing) existing.remove();
    const loadHtml = `<div class="modal-overlay" id="projectViewerModal" data-action-modal="closeProjectViewer">
    <div class="modal-content" style="max-width:90vw;max-height:90vh;width:900px;">
      <div class="modal-header"><h2>${escapeHtml(projectName)} — REAPER</h2><button class="modal-close" data-action-modal="closeProjectViewer" title="Close">&#10005;</button></div>
      <div class="modal-body" style="padding:0;"><div style="text-align:center;padding:32px;"><div class="spinner" style="width:20px;height:20px;margin:0 auto 12px;"></div>Loading...</div></div>
    </div></div>`;
    document.body.insertAdjacentHTML('beforeend', loadHtml);
    try {
        const result = await window.vstUpdater.readProjectFile(filePath);
        const modal = document.getElementById('projectViewerModal');
        if (!modal) return;
        const body = modal.querySelector('.modal-body');
        const text = result.content || '';
        body.innerHTML = `<div style="display:flex;flex-direction:column;height:calc(90vh - 80px);">
      <div style="padding:8px 12px;background:var(--bg-secondary);border-bottom:1px solid var(--border);display:flex;gap:12px;align-items:center;flex-shrink:0;">
        <span style="font-size:11px;color:var(--text-muted);">REAPER Project | ${text.split('\\n').length.toLocaleString()} lines</span>
        <input type="text" class="np-search-input" id="projSearchInput" placeholder="Search..." style="flex:1;max-width:300px;" autocomplete="off">
      </div>
      <pre style="flex:1;overflow:auto;padding:8px 12px;font-family:'Share Tech Mono',monospace;font-size:11px;line-height:1.6;color:var(--text);margin:0;white-space:pre-wrap;tab-size:2;background:var(--bg-primary);" id="projTextContent"></pre>
    </div>`;
        document.getElementById('projTextContent').textContent = text;
    } catch (e) {
        const modal = document.getElementById('projectViewerModal');
        if (modal) modal.querySelector('.modal-body').innerHTML = '<div style="padding:20px;color:var(--red);">Error: ' + escapeHtml(String(e)) + '</div>';
    }
}

async function showBinaryProjectViewer(filePath, projectName) {
    let existing = document.getElementById('bwViewerModal');
    if (existing) existing.remove();
    const loadHtml = `<div class="modal-overlay" id="bwViewerModal" data-action-modal="closeBwViewer">
    <div class="modal-content" style="max-width:90vw;max-height:90vh;width:900px;">
      <div class="modal-header"><h2>${escapeHtml(projectName)}</h2><button class="modal-close" data-action-modal="closeBwViewer" title="Close">&#10005;</button></div>
      <div class="modal-body" style="padding:0;"><div style="text-align:center;padding:32px;"><div class="spinner" style="width:20px;height:20px;margin:0 auto 12px;"></div>Parsing binary data...</div></div>
    </div></div>`;
    document.body.insertAdjacentHTML('beforeend', loadHtml);
    try {
        const data = await window.vstUpdater.readProjectFile(filePath);
        const modal = document.getElementById('bwViewerModal');
        if (!modal) return;
        // Update title with format
        const h2 = modal.querySelector('h2');
        if (h2 && data._format) h2.textContent = `${projectName} — ${data._format}`;
        const body = modal.querySelector('.modal-body');
        const treeData = data.type === 'xml' || data.type === 'text' ? data : data;
        body.innerHTML = `<div style="display:flex;flex-direction:column;height:calc(90vh - 80px);">
      <div style="padding:8px 12px;background:var(--bg-secondary);border-bottom:1px solid var(--border);display:flex;gap:12px;align-items:center;flex-shrink:0;">
        <span style="font-size:11px;color:var(--text-muted);">${treeData._format || 'Binary'} | ${treeData.plugins ? treeData.plugins.length : 0} plugins | ${treeData.pluginStateCount || 0} preset states | ${treeData._size || '?'}</span>
        <input type="text" class="np-search-input" id="bwSearchInput" placeholder="Search..." style="flex:1;max-width:300px;" autocomplete="off">
        <button class="btn btn-secondary" id="bwCollapseAllBtn" style="padding:4px 10px;font-size:10px;">Collapse All</button>
        <button class="btn btn-secondary" id="bwExpandAllBtn" style="padding:4px 10px;font-size:10px;">Expand All</button>
      </div>
      <div id="bwJsonTree" style="flex:1;overflow:auto;padding:8px 12px;font-family:'Share Tech Mono',monospace;font-size:11px;line-height:1.6;color:var(--text);background:var(--bg-primary);"></div>
    </div>`;
        document.getElementById('bwJsonTree').appendChild(typeof buildJsonTree === 'function' ? buildJsonTree(treeData, 0) : document.createTextNode(JSON.stringify(treeData, null, 2)));
    } catch (e) {
        const modal = document.getElementById('bwViewerModal');
        if (modal) modal.querySelector('.modal-body').innerHTML = '<div style="padding:20px;color:var(--red);">Error: ' + escapeHtml(String(e)) + '</div>';
    }
}

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

        const lineCount = xml.split('\n').length;

        body.innerHTML = `<div style="display:flex;flex-direction:column;height:calc(90vh - 80px);">
      <div style="padding:8px 12px;background:var(--bg-secondary);border-bottom:1px solid var(--border);display:flex;gap:12px;align-items:center;flex-shrink:0;">
        <span style="font-size:11px;color:var(--text-muted);">${lineCount.toLocaleString()} lines | ${typeof formatAudioSize === 'function' ? formatAudioSize(xml.length) : Math.round(xml.length / 1024) + ' KB'} uncompressed</span>
        <input type="text" class="np-search-input" id="alsSearchInput" placeholder="Search XML..." style="flex:1;max-width:300px;" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" title="Search within XML content">
        <button class="btn btn-secondary" id="alsCollapseAllBtn" style="padding:4px 10px;font-size:10px;" title="Collapse all XML nodes">Collapse All</button>
        <button class="btn btn-secondary" id="alsExpandAllBtn" style="padding:4px 10px;font-size:10px;" title="Expand all XML nodes">Expand All</button>
        <button class="btn btn-secondary" id="alsExportBtn" style="padding:4px 10px;font-size:10px;" title="Save decompressed XML to file">&#8615; Export</button>
      </div>
      <div id="alsXmlTree" style="flex:1;overflow:auto;margin:0;padding:8px 12px;font-family:'Share Tech Mono',monospace;font-size:11px;line-height:1.6;color:var(--text);background:var(--bg-primary);"></div>
    </div>`;

        // Parse XML to DOM and render collapsible tree (cap at 10MB)
        const treeContainer = document.getElementById('alsXmlTree');
        if (xml.length > 10_000_000) {
            treeContainer.innerHTML = `<pre style="white-space:pre-wrap;word-break:break-all;">${escapeHtml(xml.slice(0, 500_000))}\n\n<!-- File too large for tree view (${Math.round(xml.length / 1024 / 1024)}MB). Showing first 500KB. Use Export to save full XML. --></pre>`;
        } else {
            const parser = new DOMParser();
            const doc = parser.parseFromString(xml, 'text/xml');
            if (doc.documentElement) {
                treeContainer.appendChild(buildXmlTree(doc.documentElement, 0));
            }

            // Collapse/expand click handler
            treeContainer.addEventListener('click', (ev) => {
                const toggle = ev.target.closest('.xml-toggle');
                if (!toggle) return;
                const node = toggle.closest('.xml-node');
                if (!node) return;
                const children = node.querySelector('.xml-children');
                if (!children) return;
                const collapsed = children.style.display === 'none';
                children.style.display = collapsed ? '' : 'none';
                toggle.textContent = collapsed ? '▼' : '▶';
                // Show/hide the inline collapsed summary
                const summary = node.querySelector('.xml-collapsed-summary');
                if (summary) summary.style.display = collapsed ? 'none' : '';
            });

            // Collapse All / Expand All
            document.getElementById('alsCollapseAllBtn')?.addEventListener('click', () => {
                treeContainer.querySelectorAll('.xml-children').forEach(c => c.style.display = 'none');
                treeContainer.querySelectorAll('.xml-toggle').forEach(t => t.textContent = '▶');
                treeContainer.querySelectorAll('.xml-collapsed-summary').forEach(s => s.style.display = '');
            });
            document.getElementById('alsExpandAllBtn')?.addEventListener('click', () => {
                treeContainer.querySelectorAll('.xml-children').forEach(c => c.style.display = '');
                treeContainer.querySelectorAll('.xml-toggle').forEach(t => t.textContent = '▼');
                treeContainer.querySelectorAll('.xml-collapsed-summary').forEach(s => s.style.display = 'none');
            });

            // Search — filter tree nodes
            const rawLines = xml.split('\n');
            let _alsSearchTimer;
            document.getElementById('alsSearchInput')?.addEventListener('input', (e) => {
                clearTimeout(_alsSearchTimer);
                _alsSearchTimer = setTimeout(() => _searchTreeNodes(treeContainer, e.target.value.trim()), 200);
            });

            // Export
            document.getElementById('alsExportBtn')?.addEventListener('click', async () => {
                const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
                if (!dialogApi) return;
                const savePath = await dialogApi.save({
                    title: catalogFmt('ui.dialog.save_decompressed_xml'),
                    defaultPath: projectName.replace(/\\.als$/i, '') + '.xml',
                    filters: [{name: catalogFmt('ui.file_filter.xml'), extensions: ['xml']}],
                });
                if (savePath) {
                    await window.__TAURI__.core.invoke('write_text_file', {filePath: savePath, contents: xml});
                    showToast(toastFmt('toast.xml_exported'));
                }
            });
        } // end else (not too large)
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
    const bwAction = e.target.closest('[data-action-modal="closeBwViewer"]');
    if (bwAction) {
        if (e.target === bwAction || bwAction.classList.contains('modal-close')) {
            const modal = document.getElementById('bwViewerModal');
            if (modal) modal.remove();
        }
    }
    const projAction = e.target.closest('[data-action-modal="closeProjectViewer"]');
    if (projAction) {
        if (e.target === projAction || projAction.classList.contains('modal-close')) {
            const modal = document.getElementById('projectViewerModal');
            if (modal) modal.remove();
        }
    }
});

// ── JSON Tree Builder (for Bitwig and other structured data) ──
function buildJsonTree(data, depth) {
    const el = document.createElement('div');
    el.className = 'xml-node';
    el.style.paddingLeft = (depth * 16) + 'px';

    if (data === null || data === undefined) {
        el.innerHTML = `<span style="display:inline-block;width:14px;"></span><span class="xml-text" style="color:var(--text-dim);">null</span>`;
        return el;
    }

    if (typeof data === 'string') {
        el.innerHTML = `<span style="display:inline-block;width:14px;"></span><span class="xml-attr-val">"${escapeHtml(data)}"</span>`;
        return el;
    }

    if (typeof data === 'number' || typeof data === 'boolean') {
        el.innerHTML = `<span style="display:inline-block;width:14px;"></span><span class="xml-text" style="color:var(--orange);">${data}</span>`;
        return el;
    }

    if (Array.isArray(data)) {
        const count = data.length;
        const summary = `<span class="xml-collapsed-summary" style="display:none;color:var(--text-dim);font-size:10px;"> ...${count} items</span>`;
        el.innerHTML = `<span class="xml-toggle" title="Click to collapse/expand" style="cursor:pointer;color:var(--cyan);display:inline-block;width:14px;text-align:center;user-select:none;">▼</span><span style="color:var(--magenta);">[</span> <span style="color:var(--text-dim);font-size:10px;">${count} items</span>${summary}`;
        const children = document.createElement('div');
        children.className = 'xml-children';
        for (let i = 0; i < data.length; i++) {
            const row = document.createElement('div');
            row.className = 'xml-node';
            row.style.paddingLeft = ((depth + 1) * 16) + 'px';
            row.innerHTML = `<span style="color:var(--text-dim);font-size:9px;">${i}: </span>`;
            const val = buildJsonTree(data[i], 0);
            val.style.paddingLeft = '0';
            val.style.display = 'inline';
            row.appendChild(val);
            children.appendChild(row);
        }
        const close = document.createElement('div');
        close.style.paddingLeft = (depth * 16) + 'px';
        close.innerHTML = `<span style="display:inline-block;width:14px;"></span><span style="color:var(--magenta);">]</span>`;
        children.appendChild(close);
        el.appendChild(children);
        if (depth > 2) {
            children.style.display = 'none';
            el.querySelector('.xml-toggle').textContent = '▶';
            const sm = el.querySelector('.xml-collapsed-summary');
            if (sm) sm.style.display = '';
        }
        return el;
    }

    if (typeof data === 'object') {
        const keys = Object.keys(data);
        const summary = `<span class="xml-collapsed-summary" style="display:none;color:var(--text-dim);font-size:10px;"> ...${keys.length} keys</span>`;
        el.innerHTML = `<span class="xml-toggle" title="Click to collapse/expand" style="cursor:pointer;color:var(--cyan);display:inline-block;width:14px;text-align:center;user-select:none;">▼</span><span style="color:var(--cyan);">{</span> <span style="color:var(--text-dim);font-size:10px;">${keys.length} keys</span>${summary}`;
        const children = document.createElement('div');
        children.className = 'xml-children';
        for (const key of keys) {
            const row = document.createElement('div');
            row.className = 'xml-node';
            row.style.paddingLeft = ((depth + 1) * 16) + 'px';
            row.innerHTML = `<span class="xml-attr-name">"${escapeHtml(key)}"</span><span style="color:var(--text-muted);">: </span>`;
            const val = buildJsonTree(data[key], depth + 1);
            val.style.paddingLeft = '0';
            val.style.display = 'inline';
            row.appendChild(val);
            children.appendChild(row);
        }
        const close = document.createElement('div');
        close.style.paddingLeft = (depth * 16) + 'px';
        close.innerHTML = `<span style="display:inline-block;width:14px;"></span><span style="color:var(--cyan);">}</span>`;
        children.appendChild(close);
        el.appendChild(children);
        if (depth > 2) {
            children.style.display = 'none';
            el.querySelector('.xml-toggle').textContent = '▶';
            const sm = el.querySelector('.xml-collapsed-summary');
            if (sm) sm.style.display = '';
        }
        return el;
    }

    return el;
}

// ── Bitwig Project Viewer ──
async function showBwViewer(filePath, projectName) {
    let existing = document.getElementById('bwViewerModal');
    if (existing) existing.remove();

    const loadHtml = `<div class="modal-overlay" id="bwViewerModal" data-action-modal="closeBwViewer">
    <div class="modal-content" style="max-width:90vw;max-height:90vh;width:900px;">
      <div class="modal-header">
        <h2>${escapeHtml(projectName)} — Bitwig Project</h2>
        <button class="modal-close" data-action-modal="closeBwViewer" title="Close">&#10005;</button>
      </div>
      <div class="modal-body" style="padding:0;">
        <div style="text-align:center;padding:32px;"><div class="spinner" style="width:20px;height:20px;margin:0 auto 12px;"></div>Parsing binary data...</div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', loadHtml);

    try {
        const data = await window.vstUpdater.readBwproject(filePath);
        const modal = document.getElementById('bwViewerModal');
        if (!modal) return;
        const body = modal.querySelector('.modal-body');

        const jsonStr = JSON.stringify(data, null, 2);

        body.innerHTML = `<div style="display:flex;flex-direction:column;height:calc(90vh - 80px);">
      <div style="padding:8px 12px;background:var(--bg-secondary);border-bottom:1px solid var(--border);display:flex;gap:12px;align-items:center;flex-shrink:0;">
        <span style="font-size:11px;color:var(--text-muted);">${data.plugins ? data.plugins.length : 0} plugins | ${data.pluginStateCount || 0} preset states | ${data._size || '?'}</span>
        <input type="text" class="np-search-input" id="bwSearchInput" placeholder="Search..." style="flex:1;max-width:300px;" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" title="Search within project data">
        <button class="btn btn-secondary" id="bwCollapseAllBtn" style="padding:4px 10px;font-size:10px;" title="Collapse all nodes">Collapse All</button>
        <button class="btn btn-secondary" id="bwExpandAllBtn" style="padding:4px 10px;font-size:10px;" title="Expand all nodes">Expand All</button>
        <button class="btn btn-secondary" id="bwExportBtn" style="padding:4px 10px;font-size:10px;" title="Export as JSON">&#8615; Export JSON</button>
      </div>
      <div id="bwJsonTree" style="flex:1;overflow:auto;padding:8px 12px;font-family:'Share Tech Mono',monospace;font-size:11px;line-height:1.6;color:var(--text);background:var(--bg-primary);"></div>
    </div>`;

        const treeContainer = document.getElementById('bwJsonTree');
        treeContainer.appendChild(buildJsonTree(data, 0));

        // Collapse/expand click handler (reuse same pattern as XML)
        treeContainer.addEventListener('click', (ev) => {
            const toggle = ev.target.closest('.xml-toggle');
            if (!toggle) return;
            const node = toggle.closest('.xml-node');
            if (!node) return;
            const children = node.querySelector('.xml-children');
            if (!children) return;
            const collapsed = children.style.display === 'none';
            children.style.display = collapsed ? '' : 'none';
            toggle.textContent = collapsed ? '▼' : '▶';
            const summary = node.querySelector('.xml-collapsed-summary');
            if (summary) summary.style.display = collapsed ? 'none' : '';
        });

        document.getElementById('bwCollapseAllBtn')?.addEventListener('click', () => {
            treeContainer.querySelectorAll('.xml-children').forEach(c => c.style.display = 'none');
            treeContainer.querySelectorAll('.xml-toggle').forEach(t => t.textContent = '▶');
            treeContainer.querySelectorAll('.xml-collapsed-summary').forEach(s => s.style.display = '');
        });
        document.getElementById('bwExpandAllBtn')?.addEventListener('click', () => {
            treeContainer.querySelectorAll('.xml-children').forEach(c => c.style.display = '');
            treeContainer.querySelectorAll('.xml-toggle').forEach(t => t.textContent = '▼');
            treeContainer.querySelectorAll('.xml-collapsed-summary').forEach(s => s.style.display = 'none');
        });

        // Search
        let _bwSearchTimer;
        document.getElementById('bwSearchInput')?.addEventListener('input', (e) => {
            clearTimeout(_bwSearchTimer);
            _bwSearchTimer = setTimeout(() => _searchTreeNodes(treeContainer, e.target.value.trim()), 200);
        });

        // Export JSON
        document.getElementById('bwExportBtn')?.addEventListener('click', async () => {
            const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
            if (!dialogApi) return;
            const savePath = await dialogApi.save({
                title: catalogFmt('ui.dialog.export_bitwig_project_data'),
                defaultPath: projectName.replace(/\.bwproject$/i, '') + '.json',
                filters: [{name: catalogFmt('ui.file_filter.json'), extensions: ['json']}],
            });
            if (savePath) {
                await window.__TAURI__.core.invoke('write_text_file', {filePath: savePath, contents: jsonStr});
                showToast(toastFmt('toast.json_exported'));
            }
        });
    } catch (err) {
        const modal = document.getElementById('bwViewerModal');
        if (modal) {
            modal.querySelector('.modal-body').innerHTML = `<div style="padding:24px;color:var(--red);">Failed to read: ${escapeHtml(err.message || String(err))}</div>`;
        }
    }
}
