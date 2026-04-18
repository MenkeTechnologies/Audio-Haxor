// ── Genre Rules Dashboard ──
// Shows all manufacturer signals, category patterns, genre scores,
// and sample counts as an in-app heatmap-style dashboard.

function _grFmt(n) { return n.toLocaleString(); }

function _grScoreColor(score) {
    if (score < -0.6) return '#ff3860';
    if (score < -0.3) return '#ff6b8a';
    if (score < -0.01) return '#ffdd57';
    if (score < 0.01) return '#6c7a89';
    if (score < 0.3) return '#88d8f7';
    if (score < 0.6) return '#00e5ff';
    return '#5dfc8a';
}

function _grHardnessColor(score) {
    if (score < 0.01) return '#6c7a89';
    if (score < 0.3) return '#88d8f7';
    if (score < 0.6) return '#ffdd57';
    if (score < 0.8) return '#ff9f43';
    return '#ff3860';
}

function _grBar(pct, color) {
    return `<div style="width:0;height:12px;min-height:12px;background:${color};border-radius:2px;" data-bar-pct="${Math.min(pct, 100)}"></div>`;
}

async function showGenreRulesDashboard() {
    document.querySelectorAll('#genreRulesModal').forEach(el => el.remove());

    const html = `<div class="modal-overlay modal-visible" id="genreRulesModal" data-action-modal="closeGenreRules">
    <div class="modal-content modal-wide" style="max-width:95vw;width:95vw;max-height:95vh;height:95vh;">
      <div class="modal-header">
        <h2>Genre Classification Rules</h2>
        <button class="modal-close" data-action-modal="closeGenreRules" title="Close">&#10005;</button>
      </div>
      <div class="modal-body" style="overflow-y:auto;max-height:calc(90vh - 60px);padding:12px;">
        <div id="grContent" style="display:block;text-align:center;padding:32px;"><div class="spinner" style="display:inline-block;"></div></div>
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

function _grStatBox(val, label) {
    return `<div style="display:inline-block;text-align:center;padding:8px 16px;margin:0 4px 8px 0;border:1px solid var(--border-color, #2a2a3e);border-radius:4px;background:rgba(0,0,0,0.3);">
      <div style="font-size:20px;font-weight:700;color:var(--cyan, #00e5ff);">${val}</div>
      <div style="font-size:10px;color:var(--text-muted, #8888aa);text-transform:uppercase;letter-spacing:0.5px;">${label}</div>
    </div>`;
}

function _grSectionCard(title, tableHtml) {
    return `<div style="margin-bottom:12px;padding:10px 12px;border:1px solid var(--border-color, #2a2a3e);border-radius:4px;background:rgba(0,0,0,0.25);">
      <div style="font-size:13px;font-weight:600;margin-bottom:8px;color:var(--text-bright, #e0e0e0);border-bottom:1px solid var(--border-color, #2a2a3e);padding-bottom:4px;">${title}</div>
      ${tableHtml}
    </div>`;
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

    // Group manufacturers by genre band
    const sections = [
        { title: 'Schranz / Hard Techno', items: mfrs.filter(m => m.genre_score <= -0.5) },
        { title: 'Tech-leaning', items: mfrs.filter(m => m.genre_score > -0.5 && m.genre_score < -0.01) },
        { title: 'Neutral', items: mfrs.filter(m => m.genre_score >= -0.01 && m.genre_score <= 0.01) },
        { title: 'Progressive / Trance-leaning', items: mfrs.filter(m => m.genre_score > 0.01 && m.genre_score < 0.5) },
        { title: 'Trance', items: mfrs.filter(m => m.genre_score >= 0.5) },
    ];

    // Group categories by parent
    const catGroups = {};
    for (const c of cats) {
        const parent = c.parent_name || 'uncategorized';
        if (!catGroups[parent]) catGroups[parent] = [];
        catGroups[parent].push(c);
    }

    let out = '';

    // ── Overview stats ──
    out += `<div style="margin-bottom:14px;text-align:left;">`;
    out += _grStatBox(_grFmt(totalAnalyzed), 'Analyzed');
    out += _grStatBox(_grFmt(totalUnanalyzed), 'Unanalyzed');
    out += _grStatBox(_grFmt(mfrs.length), 'Manufacturers');
    out += _grStatBox(_grFmt(cats.length), 'Categories');
    out += _grStatBox(_grFmt(mfrs.reduce((s, m) => s + m.sample_count, 0)), 'Matched');
    out += `</div>`;

    // ── Legend ──
    out += `<div style="margin-bottom:14px;font-size:11px;color:var(--text-muted, #8888aa);">`;
    out += `<span style="color:#ff3860;">&#9632;</span> Schranz &nbsp;`;
    out += `<span style="color:#ff6b8a;">&#9632;</span> Techno &nbsp;`;
    out += `<span style="color:#ffdd57;">&#9632;</span> Tech-lean &nbsp;`;
    out += `<span style="color:#6c7a89;">&#9632;</span> Neutral &nbsp;`;
    out += `<span style="color:#88d8f7;">&#9632;</span> Progressive &nbsp;`;
    out += `<span style="color:#00e5ff;">&#9632;</span> Trance &nbsp;`;
    out += `<span style="color:#5dfc8a;">&#9632;</span> Hard Trance`;
    out += `</div>`;

    // ── Two-column layout using float (WebKit-safe, no grid/flex) ──
    out += `<div style="overflow:hidden;">`;

    // Left column: manufacturers (60%)
    out += `<div style="float:left;width:58%;padding-right:8px;box-sizing:border-box;">`;
    for (const section of sections) {
        if (!section.items.length) continue;
        let table = `<table style="width:100%;font-size:11px;border-collapse:collapse;">`;
        table += `<colgroup><col style="width:auto"><col style="width:50px"><col style="width:45px"><col style="width:60px"><col style="width:50px"><col style="width:50px"><col style="width:25%"></colgroup>`;
        table += `<thead><tr style="color:var(--text-muted, #8888aa);border-bottom:1px solid var(--border-color, #2a2a3e);">
          <th style="text-align:left;padding:3px 4px;font-weight:500;">Label</th>
          <th style="text-align:right;padding:3px 4px;font-weight:500;">Genre</th>
          <th style="text-align:right;padding:3px 4px;font-weight:500;">Hard</th>
          <th style="text-align:right;padding:3px 4px;font-weight:500;">Samples</th>
          <th style="text-align:right;padding:3px 4px;font-weight:500;">Loops</th>
          <th style="text-align:right;padding:3px 4px;font-weight:500;">Shots</th>
          <th style="padding:3px 4px;"></th>
        </tr></thead><tbody>`;
        for (const m of section.items) {
            const gc = _grScoreColor(m.genre_score);
            const hc = _grHardnessColor(m.hardness_score);
            const pct = (m.sample_count / maxSamples) * 100;
            table += `<tr style="border-bottom:1px solid rgba(255,255,255,0.04);">
              <td style="padding:3px 4px;color:var(--text-bright, #e0e0e0);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:160px;">${escapeHtml(m.name)}</td>
              <td style="text-align:right;padding:3px 4px;color:${gc};font-weight:600;">${m.genre_score.toFixed(1)}</td>
              <td style="text-align:right;padding:3px 4px;color:${hc};">${m.hardness_score.toFixed(1)}</td>
              <td style="text-align:right;padding:3px 4px;color:var(--text-bright, #e0e0e0);">${_grFmt(m.sample_count)}</td>
              <td style="text-align:right;padding:3px 4px;color:var(--text-muted, #8888aa);">${_grFmt(m.loop_count)}</td>
              <td style="text-align:right;padding:3px 4px;color:var(--text-muted, #8888aa);">${_grFmt(m.oneshot_count)}</td>
              <td style="padding:3px 4px;">${_grBar(pct, gc)}</td>
            </tr>`;
        }
        table += `</tbody></table>`;
        out += _grSectionCard(`${section.title} (${section.items.length})`, table);
    }
    out += `</div>`;

    // Right column: categories (40%)
    out += `<div style="float:left;width:42%;box-sizing:border-box;">`;
    for (const [parent, items] of Object.entries(catGroups)) {
        const groupTotal = items.reduce((s, c) => s + c.count, 0);
        let table = `<table style="width:100%;font-size:11px;border-collapse:collapse;">`;
        table += `<colgroup><col style="width:auto"><col style="width:70px"><col style="width:45%"></colgroup>`;
        table += `<thead><tr style="color:var(--text-muted, #8888aa);border-bottom:1px solid var(--border-color, #2a2a3e);">
          <th style="text-align:left;padding:3px 4px;font-weight:500;">Category</th>
          <th style="text-align:right;padding:3px 4px;font-weight:500;">Count</th>
          <th style="padding:3px 4px;"></th>
        </tr></thead><tbody>`;
        for (const c of items) {
            const pct = (c.count / maxCatCount) * 100;
            table += `<tr style="border-bottom:1px solid rgba(255,255,255,0.04);">
              <td style="padding:3px 4px;color:var(--text-bright, #e0e0e0);">${escapeHtml(c.name)}</td>
              <td style="text-align:right;padding:3px 4px;color:var(--text-bright, #e0e0e0);">${_grFmt(c.count)}</td>
              <td style="padding:3px 4px;">${_grBar(pct, '#00e5ff')}</td>
            </tr>`;
        }
        table += `</tbody></table>`;
        out += _grSectionCard(`${escapeHtml(parent)} (${_grFmt(groupTotal)})`, table);
    }
    out += `</div>`;

    out += `</div>`; // close overflow:hidden wrapper

    content.innerHTML = out;

    // Animate bars after layout resolves
    requestAnimationFrame(() => {
        requestAnimationFrame(() => {
            content.querySelectorAll('[data-bar-pct]').forEach(el => {
                el.style.width = el.dataset.barPct + '%';
                el.style.transition = 'width 0.3s ease-out';
            });
        });
    });
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
