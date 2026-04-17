// ── Plugin Dependency Graph ──
// Visual map of plugin usage across DAW projects.
// Shows most-used plugins, orphaned plugins, and per-project breakdowns.

/** Canonical plugin key for xref rows — defined in xref.js (`xrefPluginRefKey`); fallback keeps vm tests working without xref. */
function _depPluginKey(p) {
    if (typeof xrefPluginRefKey === 'function') return xrefPluginRefKey(p);
    if (p.normalizedName) return p.normalizedName;
    return typeof normalizePluginName === 'function' ? normalizePluginName(p.name) : String(p.name || '').trim().toLowerCase();
}

/** When the DAW tab is SQLite-paginated, `allDawProjects` may omit paths that exist in `_xrefCache` (index built via `fetchAllDawProjectsForXref`). Fall back to path-derived labels. */
function _depProjectMetaForPath(path) {
    const fromList = typeof findByPath === 'function' && typeof allDawProjects !== 'undefined'
        ? findByPath(allDawProjects, path)
        : undefined;
    if (fromList) {
        return fromList;
    }
    if (typeof xrefProjectFromPath === 'function') return xrefProjectFromPath(path);
    const name = path.split('/').pop() || path;
    return {name, daw: '—', format: '', path};
}

function _depTypeSlug(t) {
    return String(t || 'unknown').toLowerCase().replace(/[^a-z0-9_-]/g, '') || 'unknown';
}

function buildDepGraphData() {
    const pluginProjects = {};  // normalizedName → { name, type, manufacturer, projects: Set<path> }
    const projectPlugins = {};  // path → { name, daw, plugins: PluginRef[] }

    // Build from xref cache (path metadata from `allDawProjects` when present, else filename — required for paginated DAW UI)
    for (const [path, plugins] of Object.entries(_xrefCache)) {
        const project = _depProjectMetaForPath(path);
        projectPlugins[path] = {name: project.name, daw: project.daw || project.format, plugins};
        for (const p of plugins) {
            const key = _depPluginKey(p);
            if (!pluginProjects[key]) {
                pluginProjects[key] = {
                    name: p.name,
                    type: p.pluginType,
                    manufacturer: p.manufacturer,
                    projects: new Set()
                };
            }
            pluginProjects[key].projects.add(path);
        }
    }

    // Find orphaned plugins (installed but not referenced in any scanned project)
    const orphaned = [];
    if (typeof allPlugins !== 'undefined') {
        const referencedNames = new Set(Object.keys(pluginProjects));
        for (const p of allPlugins) {
            const norm = typeof normalizePluginName === 'function' ? normalizePluginName(p.name) : p.name.toLowerCase();
            if (!referencedNames.has(norm)) {
                orphaned.push(p);
            }
        }
    }

    // Sort by usage count descending
    const sorted = Object.entries(pluginProjects)
        .map(([key, val]) => ({key, ...val, count: val.projects.size}))
        .sort((a, b) => b.count - a.count);

    // Sort projects by plugin count descending
    const projectsSorted = Object.entries(projectPlugins)
        .map(([path, val]) => ({path, ...val, count: val.plugins.length}))
        .sort((a, b) => b.count - a.count);

    return {
        pluginsByUsage: sorted,
        projectsByCount: projectsSorted,
        orphaned,
        totalProjects: Object.keys(projectPlugins).length
    };
}

function buildAnalyticsHtml(data) {
    const plugins = data.pluginsByUsage;
    const projects = data.projectsByCount;
    if (plugins.length === 0) {
        return `<div class="dep-empty">${escapeHtml(catalogFmt('ui.dep_graph.analytics_empty'))}</div>`;
    }
    const unkLabel = () => catalogFmt('ui.hm.unknown');

    // 1. Plugin type breakdown (VST2 vs VST3 vs AU vs CLAP)
    const typeCounts = {};
    for (const p of plugins) {
        const t = p.type || 'Unknown';
        typeCounts[t] = (typeCounts[t] || 0) + p.count;
    }
    const typeTotal = Object.values(typeCounts).reduce((a, b) => a + b, 0);
    const typeRows = Object.entries(typeCounts)
        .sort((a, b) => b[1] - a[1])
        .map(([type, count]) => {
            const pct = Math.round((count / typeTotal) * 100);
            const typeCls = 'xref-type-' + _depTypeSlug(type);
            const typeLabel = type === 'Unknown' ? unkLabel() : type;
            return `<div class="dep-plugin-row">
        <div class="dep-plugin-info">
          <span class="xref-item-type ${typeCls}">${escapeHtml(typeLabel)}</span>
          <span class="dep-plugin-name">${escapeHtml(catalogFmt('ui.dep_graph.references_pct', {count, pct}))}</span>
        </div>
        <div class="dep-bar-wrap">
          <div class="dep-bar dep-bar-cyan" data-bar-pct="${pct}" style="width:0"></div>
          <span class="dep-bar-count">${pct}%</span>
        </div>
      </div>`;
        }).join('');

    // 2. Manufacturer rankings
    const mfgCounts = {};
    for (const p of plugins) {
        const m = p.manufacturer || 'Unknown';
        mfgCounts[m] = (mfgCounts[m] || 0) + p.count;
    }
    const mfgVals = Object.values(mfgCounts);
    const mfgMax = mfgVals.length > 0 ? Math.max(...mfgVals) : 1;
    const mfgRows = Object.entries(mfgCounts)
        .sort((a, b) => b[1] - a[1])
        .slice(0, 15)
        .map(([mfg, count]) => {
            const pct = Math.round((count / mfgMax) * 100);
            const mfgLabel = mfg === 'Unknown' ? unkLabel() : mfg;
            return `<div class="dep-plugin-row">
        <div class="dep-plugin-info">
          <span class="dep-plugin-name">${escapeHtml(mfgLabel)}</span>
        </div>
        <div class="dep-bar-wrap">
          <div class="dep-bar dep-bar-green" data-bar-pct="${pct}" style="width:0"></div>
          <span class="dep-bar-count">${count}</span>
        </div>
      </div>`;
        }).join('');

    // 3. Single-use plugins (used in only 1 project)
    const singleUse = plugins.filter(p => p.count === 1);

    // 4. Most versatile plugins (used across most projects)
    const versatile = plugins.slice(0, 10);

    // 5. Average plugins per project
    const avgPlugins = projects.length > 0
        ? (projects.reduce((sum, p) => sum + p.count, 0) / projects.length).toFixed(1)
        : 0;

    // 6. Projects with most unique plugin diversity
    const heaviestProject = projects.length > 0 ? projects[0] : null;
    const lightestProject = projects.length > 0 ? projects[projects.length - 1] : null;

    // 7. Plugin adoption — how many plugins are used in >50% of projects
    const widelyUsed = plugins.filter(p => p.count > data.totalProjects / 2);

    const extremePlug = (n) => (n === 1
        ? catalogFmt('ui.dep_graph.extreme_plugin_one', {n})
        : catalogFmt('ui.dep_graph.extreme_plugin_other', {n}));

    return `
    <div class="dep-analytics">
      <div class="dep-analytics-section">
        <h3 class="dep-analytics-title">${escapeHtml(catalogFmt('ui.dep_graph.section_format_breakdown'))}</h3>
        ${typeRows}
      </div>
      <div class="dep-analytics-section">
        <h3 class="dep-analytics-title">${escapeHtml(catalogFmt('ui.dep_graph.section_top_manufacturers'))}</h3>
        ${mfgRows}
      </div>
      <div class="dep-analytics-section">
        <h3 class="dep-analytics-title">${escapeHtml(catalogFmt('ui.dep_graph.section_key_insights'))}</h3>
        <div class="dep-analytics-insights">
          <div class="dep-insight"><span class="dep-insight-val">${avgPlugins}</span><span class="dep-insight-label">${escapeHtml(catalogFmt('ui.dep_graph.insight_avg_plugins'))}</span></div>
          <div class="dep-insight"><span class="dep-insight-val">${widelyUsed.length}</span><span class="dep-insight-label">${escapeHtml(catalogFmt('ui.dep_graph.insight_widely_used'))}</span></div>
          <div class="dep-insight"><span class="dep-insight-val">${singleUse.length}</span><span class="dep-insight-label">${escapeHtml(catalogFmt('ui.dep_graph.insight_single_use'))}</span></div>
          <div class="dep-insight"><span class="dep-insight-val">${data.orphaned.length}</span><span class="dep-insight-label">${escapeHtml(catalogFmt('ui.dep_graph.insight_orphaned_installed'))}</span></div>
        </div>
      </div>
      ${heaviestProject ? `<div class="dep-analytics-section">
        <h3 class="dep-analytics-title">${escapeHtml(catalogFmt('ui.dep_graph.section_project_extremes'))}</h3>
        <div style="font-size:11px;color:var(--text-muted);line-height:1.8;">
          <div><span style="color:var(--cyan);">${escapeHtml(catalogFmt('ui.dep_graph.most_complex'))}</span> ${escapeHtml(heaviestProject.name)} (${escapeHtml(extremePlug(heaviestProject.count))})</div>
          <div><span style="color:var(--green);">${escapeHtml(catalogFmt('ui.dep_graph.most_minimal'))}</span> ${escapeHtml(lightestProject.name)} (${escapeHtml(extremePlug(lightestProject.count))})</div>
        </div>
      </div>` : ''}
      ${widelyUsed.length > 0 ? `<div class="dep-analytics-section">
        <h3 class="dep-analytics-title">${escapeHtml(catalogFmt('ui.dep_graph.section_goto_plugins'))}</h3>
        ${widelyUsed.map(p => `<div class="dep-plugin-row">
          <div class="dep-plugin-info">
            <span class="xref-item-type xref-type-${_depTypeSlug(p.type)}">${escapeHtml(p.type ? p.type : unkLabel())}</span>
            <span class="dep-plugin-name">${escapeHtml(p.name)}</span>
            <span class="dep-plugin-mfg">${escapeHtml(p.manufacturer)}</span>
          </div>
          <div class="dep-bar-wrap">
            <div class="dep-bar dep-bar-yellow" data-bar-pct="${Math.round((p.count / data.totalProjects) * 100)}" style="width:0"></div>
            <span class="dep-bar-count">${p.count}/${data.totalProjects}</span>
          </div>
        </div>`).join('')}
      </div>` : ''}
      ${singleUse.length > 0 ? `<div class="dep-analytics-section">
        <h3 class="dep-analytics-title">${escapeHtml(catalogFmt('ui.dep_graph.section_single_use'))}</h3>
        <div style="max-height:200px;overflow-y:auto;">
        ${singleUse.slice(0, 30).map(p => `<div class="dep-plugin-row" style="opacity:0.7;">
          <div class="dep-plugin-info">
            <span class="xref-item-type xref-type-${_depTypeSlug(p.type)}">${escapeHtml(p.type ? p.type : unkLabel())}</span>
            <span class="dep-plugin-name">${escapeHtml(p.name)}</span>
            <span class="dep-plugin-mfg">${escapeHtml(p.manufacturer)}</span>
          </div>
        </div>`).join('')}
        ${singleUse.length > 30 ? `<div style="text-align:center;padding:6px;color:var(--text-dim);font-size:10px;">${escapeHtml(catalogFmt('ui.dep_graph.and_n_more', {n: singleUse.length - 30}))}</div>` : ''}
        </div>
      </div>` : ''}
    </div>`;
}

function showDepGraph() {
    const data = buildDepGraphData();
    let existing = document.getElementById('depGraphModal');
    if (existing) existing.remove();

    if (data.pluginsByUsage.length === 0 && data.orphaned.length === 0) {
        if (typeof showToast === 'function' && typeof toastFmt === 'function') {
            showToast(toastFmt('toast.run_plugin_index_first'), 5000, 'error');
        }
        const msg = typeof catalogFmt === 'function'
            ? catalogFmt('ui.dep_graph.confirm_build_index_body')
            : 'Build plugin index now?';
        if (confirm(msg)) {
            if (typeof buildXrefIndex === 'function') {
                buildXrefIndex().then(() => {
                    if (typeof filterDawProjects === 'function') filterDawProjects();
                    showDepGraph();
                });
            }
        }
        return;
    }

    const maxCount = data.pluginsByUsage.length > 0 ? data.pluginsByUsage[0].count : 1;
    const unkLabel = () => catalogFmt('ui.hm.unknown');

    // Most-used plugins section
    const topPlugins = data.pluginsByUsage.slice(0, 50).map(p => {
        const pct = Math.round((p.count / maxCount) * 100);
        const typeCls = 'xref-type-' + _depTypeSlug(p.type);
        const projectListRaw = [...p.projects].map(path => {
            const proj = data.projectsByCount.find(pr => pr.path === path);
            return proj ? proj.name : path.split('/').pop();
        }).join(', ');
        const usedInTitle = escapeHtml(catalogFmt('ui.dep_graph.used_in_title', {names: projectListRaw}));
        return `<div class="dep-plugin-row" title="${usedInTitle}">
      <div class="dep-plugin-info">
        <span class="xref-item-type ${typeCls}">${escapeHtml(p.type ? p.type : unkLabel())}</span>
        <span class="dep-plugin-name">${escapeHtml(p.name)}</span>
        <span class="dep-plugin-mfg">${escapeHtml(p.manufacturer)}</span>
      </div>
      <div class="dep-bar-wrap">
        <div class="dep-bar" data-bar-pct="${pct}" style="width:0"></div>
        <span class="dep-bar-count">${p.count}</span>
      </div>
    </div>`;
    }).join('');

    // Projects by plugin count
    const maxPlugins = data.projectsByCount.length > 0 ? data.projectsByCount[0].count : 1;
    const topProjects = data.projectsByCount.slice(0, 30).map(p => {
        const pct = Math.round((p.count / maxPlugins) * 100);
        const dawCls = typeof getDawBadgeClass === 'function' ? getDawBadgeClass(p.daw || 'unknown') : 'format-default';
        return `<div class="dep-project-row" data-dep-project="${escapeHtml(p.path)}" title="${escapeHtml(p.path)}">
      <div class="dep-project-info">
        <span class="format-badge ${dawCls}" style="font-size:9px;">${escapeHtml(p.daw)}</span>
        <span class="dep-project-name">${escapeHtml(p.name)}</span>
      </div>
      <div class="dep-bar-wrap">
        <div class="dep-bar dep-bar-magenta" data-bar-pct="${pct}" style="width:0"></div>
        <span class="dep-bar-count">${p.count}</span>
      </div>
    </div>`;
    }).join('');

    // Orphaned plugins
    const orphanedHtml = data.orphaned.length > 0
        ? data.orphaned.slice(0, 50).map(p => {
            const typeCls = 'xref-type-' + (p.type || 'vst2').toLowerCase();
            return `<div class="dep-plugin-row dep-orphan" title="${escapeHtml(p.path)}">
          <span class="xref-item-type ${typeCls}">${escapeHtml(p.type)}</span>
          <span class="dep-plugin-name">${escapeHtml(p.name)}</span>
          <span class="dep-plugin-mfg">${escapeHtml(p.manufacturer || '')}</span>
        </div>`;
        }).join('')
        : `<div class="dep-empty">${escapeHtml(catalogFmt('ui.dep_graph.empty_all_referenced'))}</div>`;

    // Stats summary
    const totalRefs = data.pluginsByUsage.reduce((sum, p) => sum + p.count, 0);
    const uniquePlugins = data.pluginsByUsage.length;
    const statsHtml = `<div class="dep-stats">
    <div class="dep-stat"><span class="dep-stat-val">${uniquePlugins}</span><span class="dep-stat-label">${escapeHtml(catalogFmt('ui.dep_graph.stat_unique_plugins'))}</span></div>
    <div class="dep-stat"><span class="dep-stat-val">${data.totalProjects}</span><span class="dep-stat-label">${escapeHtml(catalogFmt('ui.dep_graph.stat_projects_indexed'))}</span></div>
    <div class="dep-stat"><span class="dep-stat-val">${totalRefs}</span><span class="dep-stat-label">${escapeHtml(catalogFmt('ui.dep_graph.stat_total_refs'))}</span></div>
    <div class="dep-stat"><span class="dep-stat-val">${data.orphaned.length}</span><span class="dep-stat-label">${escapeHtml(catalogFmt('ui.dep_graph.stat_orphaned'))}</span></div>
  </div>`;

    // ── Analytics tab content ──
    const analyticsHtml = buildAnalyticsHtml(data);

    const html = `<div class="modal-overlay modal-visible" id="depGraphModal" data-action-modal="closeDepGraph">
    <div class="modal-content modal-wide dep-graph-modal">
      <div class="modal-header">
        <h2>${escapeHtml(catalogFmt('ui.dep_graph.title'))}</h2>
        <button class="modal-close" data-action-modal="closeDepGraph" title="${escapeHtml(catalogFmt('menu.close'))}">&#10005;</button>
      </div>
      <div class="modal-body">
        ${statsHtml}
        <div style="margin-bottom:10px;">
          <input type="text" class="np-search-input" id="depSearchInput" placeholder="${escapeHtml(catalogFmt('ui.dep_graph.search_placeholder'))}" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" title="${escapeHtml(catalogFmt('ui.dep_graph.search_title'))}" style="width:100%;box-sizing:border-box;">
        </div>
        <div class="dep-tabs">
          <button class="dep-tab active" data-dep-tab="usage" title="${escapeHtml(catalogFmt('ui.dep_graph.tt_tab_most_used'))}">${escapeHtml(catalogFmt('ui.dep_graph.tab_most_used'))}</button>
          <button class="dep-tab" data-dep-tab="projects" title="${escapeHtml(catalogFmt('ui.dep_graph.tt_tab_by_project'))}">${escapeHtml(catalogFmt('ui.dep_graph.tab_by_project'))}</button>
          <button class="dep-tab" data-dep-tab="orphaned" title="${escapeHtml(catalogFmt('ui.dep_graph.tt_tab_orphaned'))}">${escapeHtml(catalogFmt('ui.dep_graph.tab_orphaned', {n: data.orphaned.length}))}</button>
          <button class="dep-tab" data-dep-tab="analytics" title="${escapeHtml(catalogFmt('ui.dep_graph.tt_tab_analytics'))}">${escapeHtml(catalogFmt('ui.dep_graph.tab_analytics'))}</button>
        </div>
        <div class="dep-panel active" id="depPanelUsage">${topPlugins || `<div class="state-message"><div class="state-icon">&#9889;</div><h2>${escapeHtml(catalogFmt('ui.dep_graph.empty_no_refs'))}</h2></div>`}</div>
        <div class="dep-panel" id="depPanelProjects">${topProjects || `<div class="state-message"><div class="state-icon">&#127911;</div><h2>${escapeHtml(catalogFmt('ui.dep_graph.empty_no_projects'))}</h2></div>`}</div>
        <div class="dep-panel" id="depPanelOrphaned">${orphanedHtml}</div>
        <div class="dep-panel" id="depPanelAnalytics">${analyticsHtml}</div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);

    // Store full HTML for search filtering
    const usagePanel = document.getElementById('depPanelUsage');
    const projPanel = document.getElementById('depPanelProjects');
    const orphPanel = document.getElementById('depPanelOrphaned');
    const analyticsPanel = document.getElementById('depPanelAnalytics');
    if (usagePanel) usagePanel._fullHtml = usagePanel.innerHTML;
    if (projPanel) projPanel._fullHtml = projPanel.innerHTML;
    if (orphPanel) orphPanel._fullHtml = orphPanel.innerHTML;
    if (analyticsPanel) analyticsPanel._fullHtml = analyticsPanel.innerHTML;

    // Defer bar widths until flex layout resolves
    requestAnimationFrame(() => {
        document.querySelectorAll('#depGraphModal [data-bar-pct]').forEach(el => {
            el.style.width = el.dataset.barPct + '%';
            el.style.transition = 'width 0.3s ease-out';
        });
    });

    // Search filtering with match highlighting (debounced)
    let _depTimer;
    document.getElementById('depSearchInput')?.addEventListener('input', (e) => {
        clearTimeout(_depTimer);
        _depTimer = setTimeout(() => {
            const q = e.target.value.trim();
            const ql = q.toLowerCase();
            [usagePanel, projPanel, orphPanel, analyticsPanel].forEach(panel => {
                if (!panel || !panel._fullHtml) return;
                if (!q) {
                    panel.innerHTML = panel._fullHtml;
                    return;
                }
                const tmp = document.createElement('div');
                tmp.innerHTML = panel._fullHtml;
                const rows = tmp.querySelectorAll('.dep-plugin-row, .dep-project-row, .dep-orphan');
                rows.forEach(row => {
                    const fields = [...row.querySelectorAll('.dep-plugin-name, .dep-plugin-mfg, .dep-project-name')].map(s => s.textContent);
                    const score = typeof searchScore === 'function' ? searchScore(q, fields, 'fuzzy') : (row.textContent.toLowerCase().includes(ql) ? 1 : 0);
                    if (score <= 0) {
                        row.style.display = 'none';
                    } else {
                        row.querySelectorAll('.dep-plugin-name, .dep-plugin-mfg, .dep-project-name').forEach(span => {
                            if (typeof highlightMatch === 'function') {
                                span.innerHTML = highlightMatch(span.textContent, q, 'fuzzy');
                            }
                        });
                    }
                });
                panel.innerHTML = tmp.innerHTML;
            });
        }, 200);
    });
}

function closeDepGraph() {
    const modal = document.getElementById('depGraphModal');
    if (modal) modal.remove();
}

// Event delegation
document.addEventListener('click', (e) => {
    // Close — only if clicking the overlay background or the X button
    const closeAction = e.target.closest('[data-action-modal="closeDepGraph"]');
    if (closeAction) {
        if (e.target === closeAction || closeAction.classList.contains('modal-close')) {
            closeDepGraph();
            return;
        }
    }

    // Tab switching
    const tab = e.target.closest('.dep-tab');
    if (tab && tab.dataset.depTab) {
        document.querySelectorAll('.dep-tab').forEach(t => t.classList.remove('active'));
        document.querySelectorAll('.dep-panel').forEach(p => p.classList.remove('active'));
        tab.classList.add('active');
        const panel = document.getElementById('depPanel' + tab.dataset.depTab.charAt(0).toUpperCase() + tab.dataset.depTab.slice(1));
        if (panel) panel.classList.add('active');
        return;
    }

    // Click project row → show plugins inline with back button
    const projRow = e.target.closest('[data-dep-project]');
    if (projRow) {
        const path = projRow.dataset.depProject;
        const plugins = typeof _xrefCache !== 'undefined' ? (_xrefCache[path] || []) : [];
        const project = typeof allDawProjects !== 'undefined' && findByPath(allDawProjects, path);
        const name = project ? project.name : path.split('/').pop();
        const panel = document.getElementById('depPanelProjects');
        if (panel) {
            panel._prevHtml = panel.innerHTML;
            let body;
            if (plugins.length === 0) {
                body = `<div class="state-message"><div class="state-icon">&#9889;</div><h2>${escapeHtml(catalogFmt('ui.dep_graph.empty_no_plugins_in_project'))}</h2></div>`;
            } else {
                body = plugins.map(p => {
                    const typeCls = 'xref-type-' + _depTypeSlug(p.pluginType);
                    const typeLabel = p.pluginType ? p.pluginType : catalogFmt('ui.hm.unknown');
                    return `<div class="dep-plugin-row">
            <span class="xref-item-type ${typeCls}">${escapeHtml(typeLabel)}</span>
            <span class="dep-plugin-name">${escapeHtml(p.name)}</span>
            <span class="dep-plugin-mfg">${escapeHtml(p.manufacturer)}</span>
          </div>`;
                }).join('');
            }
            const plugHdr = plugins.length === 1
                ? catalogFmt('ui.dep_graph.project_header_plugins_one', {n: plugins.length})
                : catalogFmt('ui.dep_graph.project_header_plugins_other', {n: plugins.length});
            panel.innerHTML = `<div style="margin-bottom:8px;">
        <button class="btn btn-secondary" data-dep-back title="${escapeHtml(catalogFmt('ui.dep_graph.tt_back'))}" style="padding:4px 12px;font-size:11px;">&#8592; ${escapeHtml(catalogFmt('ui.dep_graph.back'))}</button>
        <span style="margin-left:8px;font-weight:600;color:var(--cyan);">${escapeHtml(name)}</span>
        <span style="margin-left:8px;color:var(--text-muted);font-size:11px;">${escapeHtml(plugHdr)}</span>
      </div>${body}`;
        }
        return;
    }

    // Back button in project detail view
    const backBtn = e.target.closest('[data-dep-back]');
    if (backBtn) {
        const panel = document.getElementById('depPanelProjects');
        if (panel && panel._prevHtml) {
            panel.innerHTML = panel._prevHtml;
            panel._prevHtml = null;
        }
        return;
    }
});

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && document.getElementById('depGraphModal')) {
        closeDepGraph();
    }
});

// Export for menu-action handler in ipc.js
window.showDepGraph = showDepGraph;
