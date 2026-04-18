// ── Genre Rules Dashboard ──
// Shows all manufacturer signals, category patterns, genre scores,
// and sample counts as an in-app heatmap-style dashboard.
// ALL colors use static CSS classes (gr-g0..gr-g6, gr-h0..gr-h4, gr-bg0..gr-bg6)
// because release WebKit strips inline style="color:..." on table cells.

function _grFmt(n) { return n.toLocaleString(); }

function _grScoreCls(score) {
    if (score < -0.6) return 'gr-g0';
    if (score < -0.3) return 'gr-g1';
    if (score < -0.01) return 'gr-g2';
    if (score < 0.01) return 'gr-g3';
    if (score < 0.3) return 'gr-g4';
    if (score < 0.6) return 'gr-g5';
    return 'gr-g6';
}

function _grHardnessCls(score) {
    if (score < 0.01) return 'gr-h0';
    if (score < 0.3) return 'gr-h1';
    if (score < 0.6) return 'gr-h2';
    if (score < 0.8) return 'gr-h3';
    return 'gr-h4';
}

function _grBarBgCls(score) {
    if (score < -0.6) return 'gr-bg0';
    if (score < -0.3) return 'gr-bg1';
    if (score < -0.01) return 'gr-bg2';
    if (score < 0.01) return 'gr-bg3';
    if (score < 0.3) return 'gr-bg4';
    if (score < 0.6) return 'gr-bg5';
    return 'gr-bg6';
}

async function showGenreRulesDashboard() {
    document.querySelectorAll('#genreRulesModal').forEach(el => el.remove());

    const html = `<div class="modal-overlay modal-visible" id="genreRulesModal" data-action-modal="closeGenreRules">
    <div class="modal-content modal-wide">
      <div class="modal-header">
        <h2>Genre Classification Rules</h2>
        <button class="modal-close" data-action-modal="closeGenreRules" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <div id="grContent" style="text-align:center;padding:32px;"><div class="spinner" style="display:inline-block;"></div></div>
      </div>
    </div>
  </div>`;
    document.body.insertAdjacentHTML('beforeend', html);

    const root = document.getElementById('genreRulesModal');
    if (!root) return;

    requestAnimationFrame(() => {
        requestAnimationFrame(() => {
            void (async () => {
                try {
                    const vu = window.vstUpdater;
                    if (!vu || typeof vu.genreRulesReport !== 'function') {
                        document.getElementById('grContent').textContent = 'IPC not available';
                        return;
                    }
                    const report = await vu.genreRulesReport();
                    if (!document.body.contains(root)) return;
                    renderGenreRules(root, report);
                } catch (e) {
                    const el = document.getElementById('grContent');
                    if (el) el.textContent = 'Error: ' + (e.message || e);
                }
            })();
        });
    });
}

function closeGenreRules() {
    document.querySelectorAll('#genreRulesModal').forEach(el => el.remove());
}

function renderGenreRules(root, report) {
    const content = root.querySelector('#grContent');
    if (!content) return;

    const mfrs = report.manufacturers || [];
    const cats = report.categories || [];
    const totalAnalyzed = report.total_analyzed || 0;
    const totalUnanalyzed = report.total_unanalyzed || 0;

    const maxSamples = Math.max(1, ...mfrs.map(m => m.sample_count));
    const maxCatCount = Math.max(1, ...cats.map(c => c.count));

    const sections = [
        { title: 'Schranz / Hard Techno', items: mfrs.filter(m => m.genre_score <= -0.5) },
        { title: 'Tech-leaning', items: mfrs.filter(m => m.genre_score > -0.5 && m.genre_score < -0.01) },
        { title: 'Neutral', items: mfrs.filter(m => m.genre_score >= -0.01 && m.genre_score <= 0.01) },
        { title: 'Progressive / Trance-leaning', items: mfrs.filter(m => m.genre_score > 0.01 && m.genre_score < 0.5) },
        { title: 'Trance', items: mfrs.filter(m => m.genre_score >= 0.5) },
    ];

    const catGroups = {};
    for (const c of cats) {
        const parent = c.parent_name || 'uncategorized';
        if (!catGroups[parent]) catGroups[parent] = [];
        catGroups[parent].push(c);
    }

    let h = '';

    // ── Stats ──
    h += '<div class="gr-stats">';
    h += _grStat(_grFmt(totalAnalyzed), 'Analyzed');
    h += _grStat(_grFmt(totalUnanalyzed), 'Unanalyzed');
    h += _grStat(_grFmt(mfrs.length), 'Manufacturers');
    h += _grStat(_grFmt(cats.length), 'Categories');
    h += _grStat(_grFmt(mfrs.reduce((s, m) => s + m.sample_count, 0)), 'Matched');
    h += '</div>';

    // ── Legend (all colors via CSS classes, no inline styles) ──
    h += '<div class="gr-legend">';
    h += '<span class="gr-g0">&#9632;</span> Schranz &nbsp;';
    h += '<span class="gr-g1">&#9632;</span> Techno &nbsp;';
    h += '<span class="gr-g2">&#9632;</span> Tech-lean &nbsp;';
    h += '<span class="gr-g3">&#9632;</span> Neutral &nbsp;';
    h += '<span class="gr-g4">&#9632;</span> Progressive &nbsp;';
    h += '<span class="gr-g5">&#9632;</span> Trance &nbsp;';
    h += '<span class="gr-g6">&#9632;</span> Hard Trance';
    h += '</div>';

    // ── Two columns ──
    h += '<div class="gr-columns">';

    // Left: manufacturers
    h += '<div class="gr-col-left">';
    for (const section of sections) {
        if (!section.items.length) continue;
        h += '<div class="gr-card">';
        h += `<div class="gr-card-title">${escapeHtml(section.title)} (${section.items.length})</div>`;
        h += '<table class="gr-table">';
        h += '<colgroup><col class="gc-label"><col class="gc-genre"><col class="gc-hard"><col class="gc-count"><col class="gc-loops"><col class="gc-shots"><col class="gc-bar"></colgroup>';
        h += '<thead><tr><th>Label</th><th class="r">Genre</th><th class="r">Hard</th><th class="r">Samples</th><th class="r">Loops</th><th class="r">Shots</th><th></th></tr></thead>';
        h += '<tbody>';
        for (const m of section.items) {
            const gc = _grScoreCls(m.genre_score);
            const hc = _grHardnessCls(m.hardness_score);
            const bgc = _grBarBgCls(m.genre_score);
            const pct = (m.sample_count / maxSamples) * 100;
            h += '<tr>';
            h += `<td class="label">${escapeHtml(m.name)}</td>`;
            h += `<td class="r gr-bold ${gc}">${m.genre_score.toFixed(1)}</td>`;
            h += `<td class="r ${hc}">${m.hardness_score.toFixed(1)}</td>`;
            h += `<td class="r">${_grFmt(m.sample_count)}</td>`;
            h += `<td class="r dim">${_grFmt(m.loop_count)}</td>`;
            h += `<td class="r dim">${_grFmt(m.oneshot_count)}</td>`;
            h += `<td><div class="gr-bar ${bgc}" data-bar-pct="${Math.min(pct, 100)}"></div></td>`;
            h += '</tr>';
        }
        h += '</tbody></table></div>';
    }
    h += '</div>';

    // Right: categories
    h += '<div class="gr-col-right">';
    for (const [parent, items] of Object.entries(catGroups)) {
        const groupTotal = items.reduce((s, c) => s + c.count, 0);
        h += '<div class="gr-card">';
        h += `<div class="gr-card-title">${escapeHtml(parent)} (${_grFmt(groupTotal)})</div>`;
        h += '<table class="gr-table">';
        h += '<colgroup><col class="gc-cat"><col class="gc-catcnt"><col class="gc-catbar"></colgroup>';
        h += '<thead><tr><th>Category</th><th class="r">Count</th><th></th></tr></thead>';
        h += '<tbody>';
        for (const c of items) {
            const pct = (c.count / maxCatCount) * 100;
            h += '<tr>';
            h += `<td>${escapeHtml(c.name)}</td>`;
            h += `<td class="r">${_grFmt(c.count)}</td>`;
            h += `<td><div class="gr-bar gr-bg5" data-bar-pct="${Math.min(pct, 100)}"></div></td>`;
            h += '</tr>';
        }
        h += '</tbody></table></div>';
    }
    h += '</div>';

    h += '</div>'; // gr-columns

    content.innerHTML = h;

    // Animate bars after layout
    requestAnimationFrame(() => {
        requestAnimationFrame(() => {
            content.querySelectorAll('.gr-bar[data-bar-pct]').forEach(el => {
                el.style.width = el.dataset.barPct + '%';
                el.style.transition = 'width 0.3s ease-out';
            });
        });
    });
}

function _grStat(val, label) {
    return `<div class="gr-stat"><div class="gr-stat-val">${val}</div><div class="gr-stat-label">${label}</div></div>`;
}

// ── Event Handlers ──

document.addEventListener('click', (e) => {
    const close = e.target.closest('[data-action-modal="closeGenreRules"]');
    if (close) {
        if (e.target === close || close.classList.contains('modal-close')) {
            closeGenreRules();
        }
    }
});

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && document.getElementById('genreRulesModal')) {
        closeGenreRules();
    }
});
