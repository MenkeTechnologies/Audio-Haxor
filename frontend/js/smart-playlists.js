// ── Smart Playlists ──
// Auto-generated playlists based on rules (BPM range, format, tags, favorites, recently played, size, path)

const _spMenuNoEcho = {skipEchoToast: true};

let _smartPlaylists = [];
let _activeSmartPlaylist = null;

function loadSmartPlaylists() {
    _smartPlaylists = prefs.getObject('smartPlaylists', []);
}

function saveSmartPlaylists() {
    prefs.setItem('smartPlaylists', _smartPlaylists);
}

function createSmartPlaylist(name, rules) {
    const pl = {
        id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
        name,
        rules,
        created: new Date().toISOString()
    };
    _smartPlaylists.push(pl);
    saveSmartPlaylists();
    renderSmartPlaylists();
    return pl;
}

function deleteSmartPlaylist(id) {
    _smartPlaylists = _smartPlaylists.filter(p => p.id !== id);
    if (_activeSmartPlaylist === id) _activeSmartPlaylist = null;
    saveSmartPlaylists();
    renderSmartPlaylists();
}

function renameSmartPlaylist(id, newName) {
    const pl = _smartPlaylists.find(p => p.id === id);
    if (pl) {
        pl.name = newName;
        saveSmartPlaylists();
        renderSmartPlaylists();
    }
}

function updateSmartPlaylistRules(id, rules) {
    const pl = _smartPlaylists.find(p => p.id === id);
    if (pl) {
        pl.rules = rules;
        saveSmartPlaylists();
    }
}

// ── Rule Matching ──
function matchesSmartRule(sample, rule) {
    switch (rule.type) {
        case 'format': {
            const formats = (rule.value || '').split(',').map(f => f.trim().toUpperCase()).filter(Boolean);
            return formats.includes(sample.format);
        }
        case 'bpm_range': {
            const bpm = typeof _bpmCache !== 'undefined' ? _bpmCache[sample.path] : null;
            if (!bpm) return false;
            const [min, max] = (rule.value || '0-999').split('-').map(Number);
            return bpm >= min && bpm <= max;
        }
        case 'tag': {
            if (typeof getNote !== 'function') return false;
            const note = getNote(sample.path);
            return note && note.tags && note.tags.includes(rule.value);
        }
        case 'favorite': {
            return typeof isFavorite === 'function' && isFavorite(sample.path);
        }
        case 'recently_played': {
            return typeof recentlyPlayed !== 'undefined' && recentlyPlayed.some(r => r.path === sample.path);
        }
        case 'name_contains': {
            return sample.name.toLowerCase().includes((rule.value || '').toLowerCase());
        }
        case 'path_contains': {
            return sample.path.toLowerCase().includes((rule.value || '').toLowerCase());
        }
        case 'size_max': {
            const maxBytes = parseFloat(rule.value || '0') * 1024 * 1024; // MB
            return sample.sizeBytes <= maxBytes;
        }
        case 'size_min': {
            const minBytes = parseFloat(rule.value || '0') * 1024 * 1024;
            return sample.sizeBytes >= minBytes;
        }
        case 'key': {
            const key = typeof _keyCache !== 'undefined' ? _keyCache[sample.path] : null;
            if (!key) return false;
            return key.toLowerCase().includes((rule.value || '').toLowerCase());
        }
        case 'duration_max': {
            const maxSec = parseFloat(rule.value || '0');
            if (!(maxSec > 0) || !Number.isFinite(maxSec)) return false;
            const dur = sample.duration;
            if (dur == null || !Number.isFinite(dur) || dur <= 0) return false;
            return dur <= maxSec;
        }
        default:
            return false;
    }
}

function evaluateSmartPlaylist(playlist) {
    if (typeof allAudioSamples === 'undefined' || allAudioSamples.length === 0) return [];
    const rules = playlist.rules || [];
    if (rules.length === 0) return [];

    const matchMode = playlist.matchMode || 'all'; // 'all' or 'any'
    return allAudioSamples.filter(sample => {
        if (matchMode === 'any') return rules.some(r => matchesSmartRule(sample, r));
        return rules.every(r => matchesSmartRule(sample, r));
    });
}

// ── UI Rendering ──
function renderSmartPlaylists() {
    const container = document.getElementById('npSmartPlaylists');
    if (!container) return;

    if (_smartPlaylists.length === 0) {
        const _spEmpty = catalogFmt('ui.sp.empty_state');
        container.innerHTML = `<div style="text-align:center;color:var(--text-dim);font-size:10px;padding:6px;">&#127926; ${typeof escapeHtml === 'function' ? escapeHtml(_spEmpty) : _spEmpty}</div>`;
        return;
    }

    container.innerHTML = _smartPlaylists.map(pl => {
        const count = evaluateSmartPlaylist(pl).length;
        const active = _activeSmartPlaylist === pl.id;
        return `<div class="sp-item${active ? ' active' : ''}" data-sp-id="${escapeHtml(pl.id)}" title="Click to load, right-click for options">
      <span class="sp-icon">&#127926;</span>
      <span class="sp-name">${escapeHtml(pl.name)}</span>
      <span class="sp-count">${count}</span>
    </div>`;
    }).join('');

    // Drag reorder smart playlists
    if (typeof initDragReorder === 'function') {
        initDragReorder(container, '.sp-item', 'smartPlaylistOrder', {
            getKey: (el) => el.dataset.spId || '',
            onReorder: (keys) => {
                const reordered = keys.map(k => _smartPlaylists.find(p => p.id === k)).filter(Boolean);
                if (reordered.length === _smartPlaylists.length) {
                    _smartPlaylists.length = 0;
                    _smartPlaylists.push(...reordered);
                    saveSmartPlaylists();
                }
            },
        });
    }
}

function loadSmartPlaylistIntoPlayer(id) {
    const pl = _smartPlaylists.find(p => p.id === id);
    if (!pl) return;
    const matches = evaluateSmartPlaylist(pl);
    if (matches.length === 0) {
        if (typeof showToast === 'function') showToast(toastFmt('toast.no_matching_samples'), 3000, 'warning');
        return;
    }

    _activeSmartPlaylist = id;
    // Prepend smart playlist results to recently played (don't destroy history)
    const newItems = matches.map(s => ({
        path: s.path,
        name: s.name,
        format: s.format,
        size: s.sizeFormatted || '',
    }));
    const existingPaths = new Set(newItems.map(i => i.path));
    const kept = recentlyPlayed.filter(r => !existingPaths.has(r.path));
    recentlyPlayed = [...newItems, ...kept].slice(0, typeof MAX_RECENT !== 'undefined' ? MAX_RECENT : 50);
    if (typeof saveRecentlyPlayed === 'function') saveRecentlyPlayed();
    if (typeof renderRecentlyPlayed === 'function') renderRecentlyPlayed();
    renderSmartPlaylists();

    if (typeof showToast === 'function') showToast(toastFmt('toast.loaded_playlist', {
        name: pl.name,
        n: matches.length
    }));
    // Auto-play first track
    if (matches.length > 0 && typeof previewAudio === 'function') {
        previewAudio(matches[0].path);
    }
}

// ── Smart Playlist Editor Modal ──
function showSmartPlaylistEditor(existingId) {
    const existing = existingId ? _smartPlaylists.find(p => p.id === existingId) : null;
    const rules = existing ? [...existing.rules] : [{type: 'format', value: 'WAV'}];
    const name = existing ? existing.name : appFmt('ui.sp_new_playlist_default');
    const matchMode = existing?.matchMode || 'all';

    const ruleTypes = [
        {value: 'format', label: appFmt('ui.sp_rule_format')},
        {value: 'bpm_range', label: appFmt('ui.sp_rule_bpm_range')},
        {value: 'tag', label: appFmt('ui.sp_rule_tag')},
        {value: 'favorite', label: appFmt('ui.sp_rule_favorite')},
        {value: 'recently_played', label: appFmt('ui.sp_rule_recently_played')},
        {value: 'name_contains', label: appFmt('ui.sp_rule_name_contains')},
        {value: 'path_contains', label: appFmt('ui.sp_rule_path_contains')},
        {value: 'size_max', label: appFmt('ui.sp_rule_size_max')},
        {value: 'size_min', label: appFmt('ui.sp_rule_size_min')},
        {value: 'duration_max', label: appFmt('ui.sp_rule_duration_max')},
        {value: 'key', label: appFmt('ui.sp_rule_key')},
    ];

    function buildRuleRow(rule, idx) {
        const opts = ruleTypes.map(t => `<option value="${t.value}"${rule.type === t.value ? ' selected' : ''}>${escapeHtml(t.label)}</option>`).join('');
        const needsValue = !['favorite', 'recently_played'].includes(rule.type);
        return `<div class="sp-rule-row" data-rule-idx="${idx}">
      <select class="sp-rule-type" data-ridx="${idx}" title="Rule type">${opts}</select>
      ${needsValue ? `<input class="sp-rule-value" data-ridx="${idx}" value="${escapeHtml(rule.value || '')}" placeholder="${escapeHtml(appFmt('ui.sp_rule_value_placeholder'))}" title="Rule value">` : '<span style="flex:1"></span>'}
      <button class="btn-small btn-stop sp-rule-del" data-ridx="${idx}" title="Remove rule" style="padding:2px 6px;font-size:10px;">&#10005;</button>
    </div>`;
    }

    const modal = document.createElement('div');
    modal.className = 'modal-overlay';
    modal.style.cssText = 'position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.7);z-index:10000;display:flex;align-items:center;justify-content:center;';

    const card = document.createElement('div');
    card.style.cssText = 'background:var(--bg-card);border:1px solid var(--border);border-radius:8px;padding:20px;width:420px;max-height:80vh;overflow-y:auto;box-shadow:0 8px 32px rgba(0,0,0,0.5);';

    function render() {
        card.innerHTML = `
      <h3 style="margin:0 0 12px 0;color:var(--text-primary);font-size:14px;">${existing ? appFmt('ui.sp_modal_title_edit') : appFmt('ui.sp_modal_title_create')}</h3>
      <input class="sp-name-input" value="${escapeHtml(name)}" placeholder="${escapeHtml(appFmt('ui.sp_playlist_name_placeholder'))}" style="width:100%;padding:6px 10px;margin-bottom:10px;background:var(--bg-input);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:12px;box-sizing:border-box;" title="Playlist name">
      <div style="margin-bottom:8px;display:flex;align-items:center;gap:8px;">
        <label style="font-size:11px;color:var(--text-muted);">${escapeHtml(appFmt('ui.sp_match_label'))}</label>
        <select class="sp-match-mode" style="font-size:11px;padding:2px 6px;background:var(--bg-input);color:var(--text-primary);border:1px solid var(--border);border-radius:3px;" title="Match mode">
          <option value="all"${matchMode === 'all' ? ' selected' : ''}>${escapeHtml(appFmt('ui.sp_match_all'))}</option>
          <option value="any"${matchMode === 'any' ? ' selected' : ''}>${escapeHtml(appFmt('ui.sp_match_any'))}</option>
        </select>
      </div>
      <div class="sp-rules-list">${rules.map((r, i) => buildRuleRow(r, i)).join('')}</div>
      <div style="display:flex;gap:8px;margin-top:10px;">
        <button class="btn-small btn-secondary sp-add-rule" style="font-size:10px;padding:4px 10px;" title="Add another rule">${escapeHtml(appFmt('ui.sp_add_rule'))}</button>
        <span style="flex:1"></span>
        <button class="btn-small btn-secondary sp-cancel" style="font-size:10px;padding:4px 12px;" title="Cancel without saving">${escapeHtml(appFmt('ui.sp_cancel'))}</button>
        <button class="btn-small btn-play sp-save" style="font-size:10px;padding:4px 12px;" title="${existing ? 'Update playlist rules' : 'Create new smart playlist'}">${existing ? escapeHtml(appFmt('ui.sp_update')) : escapeHtml(appFmt('ui.sp_create'))}</button>
      </div>
      <div class="sp-preview" style="margin-top:10px;font-size:10px;color:var(--text-dim);border-top:1px solid var(--border);padding-top:8px;"></div>
    `;

        // Preview
        const previewEl = card.querySelector('.sp-preview');
        const preview = evaluateSmartPlaylist({rules, matchMode: card.querySelector('.sp-match-mode')?.value || 'all'});
        previewEl.textContent = appFmt('ui.sp_preview_n', {n: preview.length});
    }

    render();
    modal.appendChild(card);
    document.body.appendChild(modal);

    // Events
    card.addEventListener('click', (e) => {
        if (e.target.classList.contains('sp-cancel')) {
            modal.remove();
        } else if (e.target.classList.contains('sp-save')) {
            const plName = card.querySelector('.sp-name-input').value.trim() || appFmt('ui.sp_untitled');
            const plMatchMode = card.querySelector('.sp-match-mode').value;
            if (existing) {
                existing.name = plName;
                existing.rules = rules;
                existing.matchMode = plMatchMode;
                saveSmartPlaylists();
                renderSmartPlaylists();
            } else {
                const pl = createSmartPlaylist(plName, rules);
                pl.matchMode = plMatchMode;
                saveSmartPlaylists();
            }
            modal.remove();
            if (typeof showToast === 'function') showToast(toastFmt('toast.playlist_saved', {
                name: plName,
                action: existing ? toastFmt('toast.playlist_updated') : toastFmt('toast.playlist_created')
            }));
        } else if (e.target.classList.contains('sp-add-rule')) {
            rules.push({type: 'format', value: ''});
            render();
        } else if (e.target.classList.contains('sp-rule-del')) {
            const idx = parseInt(e.target.dataset.ridx);
            rules.splice(idx, 1);
            render();
        }
    });

    card.addEventListener('change', (e) => {
        if (e.target.classList.contains('sp-rule-type')) {
            const idx = parseInt(e.target.dataset.ridx);
            rules[idx].type = e.target.value;
            if (['favorite', 'recently_played'].includes(e.target.value)) rules[idx].value = '';
            render();
        } else if (e.target.classList.contains('sp-match-mode')) {
            render();
        }
    });

    card.addEventListener('input', (e) => {
        if (e.target.classList.contains('sp-rule-value')) {
            const idx = parseInt(e.target.dataset.ridx);
            rules[idx].value = e.target.value;
            // Debounce preview
            clearTimeout(card._previewTimer);
            card._previewTimer = setTimeout(() => {
                const previewEl = card.querySelector('.sp-preview');
                if (previewEl) {
                    const mm = card.querySelector('.sp-match-mode')?.value || 'all';
                    previewEl.textContent = appFmt('ui.sp_preview_n', {
                        n: evaluateSmartPlaylist({
                            rules,
                            matchMode: mm
                        }).length
                    });
                }
            }, 200);
        }
    });

    modal.addEventListener('click', (e) => {
        if (e.target === modal) modal.remove();
    });
}

// ── Click handlers ──
document.addEventListener('click', (e) => {
    const spItem = e.target.closest('.sp-item');
    if (spItem) {
        loadSmartPlaylistIntoPlayer(spItem.dataset.spId);
        return;
    }

    const action = e.target.closest('[data-action]');
    if (!action) return;

    if (action.dataset.action === 'createSmartPlaylist') {
        showSmartPlaylistEditor(null);
    }
});

// ── Context menu on smart playlist items ──
document.addEventListener('contextmenu', (e) => {
    const spItem = e.target.closest('.sp-item');
    if (!spItem) return;
    e.preventDefault();
    const id = spItem.dataset.spId;
    const pl = _smartPlaylists.find(p => p.id === id);
    if (!pl) return;

    const matchCount = typeof evaluateSmartPlaylist === 'function' ? evaluateSmartPlaylist(pl).length : 0;
    const items = [
        {
            icon: '&#9654;',
            label: appFmt('menu.sp_load_playlist_tracks', {n: matchCount}),
            action: () => loadSmartPlaylistIntoPlayer(id)
        },
        {icon: '&#9998;', label: appFmt('menu.sp_edit_rules'), action: () => showSmartPlaylistEditor(id)},
        {
            icon: '&#128221;', label: appFmt('menu.sp_rename'), action: () => {
                const newName = prompt(appFmt('ui.sp_prompt_rename'), pl.name);
                if (newName) renameSmartPlaylist(id, newName.trim());
            }
        },
        {
            icon: '&#128203;', label: appFmt('menu.sp_clone'), action: () => {
                const clone = JSON.parse(JSON.stringify(pl));
                clone.id = Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
                clone.name = pl.name + appFmt('ui.sp_clone_suffix');
                _smartPlaylists.push(clone);
                saveSmartPlaylists();
                renderSmartPlaylists();
                showToast(toastFmt('toast.cloned_playlist', {name: pl.name}));
            }
        },
        {
            icon: '&#128203;', label: appFmt('menu.sp_copy_rules_json'), ..._spMenuNoEcho, action: () => {
                if (typeof copyToClipboard === 'function') copyToClipboard(JSON.stringify(pl.rules, null, 2));
            }
        },
        '---',
        {
            icon: '&#128465;', label: appFmt('menu.delete'), action: () => {
                if (confirm(appFmt('confirm.delete_smart_playlist', {name: pl.name}))) deleteSmartPlaylist(id);
            }
        },
    ];

    if (typeof showContextMenu === 'function') showContextMenu(e, items);
});

// ── Built-in playlist presets ──
function getSmartPlaylistPresets() {
    return [
        {name: appFmt('ui.sp_preset_favorites_only'), rules: [{type: 'favorite', value: ''}], matchMode: 'all'},
        {name: appFmt('ui.sp_preset_wav_files'), rules: [{type: 'format', value: 'WAV'}], matchMode: 'all'},
        {name: appFmt('ui.sp_preset_small_samples'), rules: [{type: 'size_max', value: '1'}], matchMode: 'all'},
        {name: appFmt('ui.sp_preset_recently_played'), rules: [{type: 'recently_played', value: ''}], matchMode: 'all'},
        {
            name: appFmt('ui.sp_preset_drums'),
            rules: [{type: 'name_contains', value: 'kick'}, {
                type: 'name_contains',
                value: 'snare'
            }, {type: 'name_contains', value: 'hat'}],
            matchMode: 'any'
        },
        {name: appFmt('ui.sp_preset_bass_sounds'), rules: [{type: 'name_contains', value: 'bass'}], matchMode: 'all'},
    ];
}

// Init — called after prefs loaded
function initSmartPlaylists() {
    loadSmartPlaylists();
    renderSmartPlaylists();
}
