// ── Plugin Dependency Graph ──
// Visual map of plugin usage across DAW projects.
// Shows most-used plugins, orphaned plugins, and per-project breakdowns.

function buildDepGraphData() {
  const pluginProjects = {};  // normalizedName → { name, type, manufacturer, projects: Set<path> }
  const projectPlugins = {};  // path → { name, daw, plugins: PluginRef[] }

  // Build from xref cache
  for (const [path, plugins] of Object.entries(_xrefCache)) {
    const project = allDawProjects.find(d => d.path === path);
    if (!project) continue;
    projectPlugins[path] = { name: project.name, daw: project.daw || project.format, plugins };
    for (const p of plugins) {
      const key = p.normalizedName || p.name.toLowerCase();
      if (!pluginProjects[key]) {
        pluginProjects[key] = { name: p.name, type: p.pluginType, manufacturer: p.manufacturer, projects: new Set() };
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
    .map(([key, val]) => ({ key, ...val, count: val.projects.size }))
    .sort((a, b) => b.count - a.count);

  // Sort projects by plugin count descending
  const projectsSorted = Object.entries(projectPlugins)
    .map(([path, val]) => ({ path, ...val, count: val.plugins.length }))
    .sort((a, b) => b.count - a.count);

  return { pluginsByUsage: sorted, projectsByCount: projectsSorted, orphaned, totalProjects: Object.keys(projectPlugins).length };
}

function showDepGraph() {
  const data = buildDepGraphData();
  let existing = document.getElementById('depGraphModal');
  if (existing) existing.remove();

  if (data.pluginsByUsage.length === 0 && data.orphaned.length === 0) {
    if (typeof tauriConfirm === 'function') {
      tauriConfirm('No plugin index data.\n\nYou must run "Plugin Index" first to scan DAW projects for plugin references.\n\nBuild the index now?', 'Plugin Dependency Graph').then(ok => {
        if (ok && typeof buildXrefIndex === 'function') {
          buildXrefIndex().then(() => { filterDawProjects(); showDepGraph(); });
        }
      });
    } else {
      showToast('Run "Plugin Index" on the DAW tab first.', 4000);
    }
    return;
  }

  const maxCount = data.pluginsByUsage.length > 0 ? data.pluginsByUsage[0].count : 1;

  // Most-used plugins section
  const topPlugins = data.pluginsByUsage.slice(0, 50).map(p => {
    const pct = Math.round((p.count / maxCount) * 100);
    const typeCls = 'xref-type-' + p.type.toLowerCase();
    const projectList = [...p.projects].map(path => {
      const proj = data.projectsByCount.find(pr => pr.path === path);
      return proj ? escapeHtml(proj.name) : escapeHtml(path.split('/').pop());
    }).join(', ');
    return `<div class="dep-plugin-row" title="Used in: ${escapeHtml(projectList)}">
      <div class="dep-plugin-info">
        <span class="xref-item-type ${typeCls}">${escapeHtml(p.type)}</span>
        <span class="dep-plugin-name">${escapeHtml(p.name)}</span>
        <span class="dep-plugin-mfg">${escapeHtml(p.manufacturer)}</span>
      </div>
      <div class="dep-bar-wrap">
        <div class="dep-bar" style="width:${pct}%"></div>
        <span class="dep-bar-count">${p.count}</span>
      </div>
    </div>`;
  }).join('');

  // Projects by plugin count
  const maxPlugins = data.projectsByCount.length > 0 ? data.projectsByCount[0].count : 1;
  const topProjects = data.projectsByCount.slice(0, 30).map(p => {
    const pct = Math.round((p.count / maxPlugins) * 100);
    const dawCls = typeof getDawBadgeClass === 'function' ? getDawBadgeClass(p.daw) : 'format-default';
    return `<div class="dep-project-row" data-dep-project="${escapeHtml(p.path)}" title="${escapeHtml(p.path)}">
      <div class="dep-project-info">
        <span class="format-badge ${dawCls}" style="font-size:9px;">${escapeHtml(p.daw)}</span>
        <span class="dep-project-name">${escapeHtml(p.name)}</span>
      </div>
      <div class="dep-bar-wrap">
        <div class="dep-bar dep-bar-magenta" style="width:${pct}%"></div>
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
    : '<div class="dep-empty">All installed plugins are referenced in scanned projects.</div>';

  // Stats summary
  const totalRefs = data.pluginsByUsage.reduce((sum, p) => sum + p.count, 0);
  const uniquePlugins = data.pluginsByUsage.length;
  const statsHtml = `<div class="dep-stats">
    <div class="dep-stat"><span class="dep-stat-val">${uniquePlugins}</span><span class="dep-stat-label">Unique Plugins</span></div>
    <div class="dep-stat"><span class="dep-stat-val">${data.totalProjects}</span><span class="dep-stat-label">Projects Indexed</span></div>
    <div class="dep-stat"><span class="dep-stat-val">${totalRefs}</span><span class="dep-stat-label">Total References</span></div>
    <div class="dep-stat"><span class="dep-stat-val">${data.orphaned.length}</span><span class="dep-stat-label">Orphaned Plugins</span></div>
  </div>`;

  const html = `<div class="modal-overlay" id="depGraphModal" data-action-modal="closeDepGraph">
    <div class="modal-content modal-wide dep-graph-modal">
      <div class="modal-header">
        <h2>Plugin Dependency Graph</h2>
        <button class="modal-close" data-action-modal="closeDepGraph" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        ${statsHtml}
        <div class="dep-tabs">
          <button class="dep-tab active" data-dep-tab="usage" title="Plugins ranked by how many projects use them">Most Used</button>
          <button class="dep-tab" data-dep-tab="projects" title="Projects ranked by plugin count">By Project</button>
          <button class="dep-tab" data-dep-tab="orphaned" title="Installed plugins not used in any scanned project">Orphaned (${data.orphaned.length})</button>
        </div>
        <div class="dep-panel active" id="depPanelUsage">${topPlugins || '<div class="dep-empty">No plugin references found.</div>'}</div>
        <div class="dep-panel" id="depPanelProjects">${topProjects || '<div class="dep-empty">No projects indexed.</div>'}</div>
        <div class="dep-panel" id="depPanelOrphaned">${orphanedHtml}</div>
      </div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);
}

function closeDepGraph() {
  const modal = document.getElementById('depGraphModal');
  if (modal) modal.remove();
}

// Event delegation
document.addEventListener('click', (e) => {
  // Close
  const closeAction = e.target.closest('[data-action-modal="closeDepGraph"]');
  if (closeAction) {
    if (e.target === closeAction || closeAction.classList.contains('modal-close')) {
      closeDepGraph();
    }
    return;
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

  // Click project row → show plugins in that project
  const projRow = e.target.closest('[data-dep-project]');
  if (projRow) {
    const path = projRow.dataset.depProject;
    const project = allDawProjects.find(d => d.path === path);
    if (project) {
      closeDepGraph();
      showProjectPlugins(path, project.name);
    }
    return;
  }
});

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && document.getElementById('depGraphModal')) {
    closeDepGraph();
  }
});
