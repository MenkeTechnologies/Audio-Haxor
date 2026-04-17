// ── Favorites ──
// Stored in SQLite via Tauri commands.

/** Same rule as file-browser: Rust may emit `\\` on Windows; `audioPlayerPath` uses `/`. */
function normalizeFavoritePathKey(p) {
    if (p == null || typeof p !== 'string') return '';
    return p.replace(/\\/g, '/');
}

let _favHasRendered = false;
let _favDirty = false;

async function getFavorites() {
    return await window.vstUpdater.favoritesList();
}

async function addFavorite(type, path, name, extra) {
    const key = normalizeFavoritePathKey(path);
    try {
        const added = await window.vstUpdater.favoritesAdd(
            type, key, name,
            (extra && extra.format) || null,
            (extra && extra.daw) || null,
            new Date().toISOString(),
        );
        if (!added) {
            showToast(toastFmt('toast.already_in_favorites', {name}));
            return;
        }
        await prepareBadgeContext();
        if (typeof window.updateFavBtn === 'function') window.updateFavBtn();
        showToast(toastFmt('toast.added_to_favorites', {name}));
        if (typeof refreshRowBadges === 'function') refreshRowBadges(key);
        _favDirty = true;
    } catch (e) {
        showToast(toastFmt('toast.failed', {err: e.message || e}), 4000, 'error');
    }
}

async function removeFavorite(path) {
    const key = normalizeFavoritePathKey(path);
    try {
        await window.vstUpdater.favoritesRemove(key);
        await prepareBadgeContext();
        if (typeof window.updateFavBtn === 'function') window.updateFavBtn();
        showToast(toastFmt('toast.removed_from_favorites'));
        if (typeof refreshRowBadges === 'function') refreshRowBadges(key);
        _favDirty = true;
        renderFavorites();
    } catch (e) {
        showToast(toastFmt('toast.failed', {err: e.message || e}), 4000, 'error');
    }
}

function isFavorite(path) {
    return _badgeCtx.favPaths.has(normalizeFavoritePathKey(path));
}

/** After tray popover toggles favorite in Rust (main webview may have been suspended).
 *
 * CRITICAL: do NOT call `syncTrayNowPlayingFromPlayback` here. Rust already owns the authoritative
 * favorite flag (it was set inside `tray_popover_toggle_favorite` before this event fired), so a
 * round-trip push is redundant. */
function applyTrayFavoritesFromHost(favorites, pathForBadge, favoriteOn) {
    if (!Array.isArray(favorites)) return;
    _badgeCtx.favPaths = new Set(favorites.map(f => f.path));
    if (typeof window.updateFavBtn === 'function') window.updateFavBtn();
    const key = pathForBadge ? normalizeFavoritePathKey(String(pathForBadge)) : '';
    if (key && typeof refreshRowBadges === 'function') refreshRowBadges(key);
    if (typeof renderFavorites === 'function') renderFavorites();
    if (typeof showToast === 'function' && typeof toastFmt === 'function' && key) {
        const row = favorites.find((f) => normalizeFavoritePathKey(f.path) === key);
        const name = row && row.name ? row.name : key.split('/').pop() || key;
        if (favoriteOn) showToast(toastFmt('toast.added_to_favorites', {name}));
        else showToast(toastFmt('toast.removed_from_favorites'));
    }
}

window.applyTrayFavoritesFromHost = applyTrayFavoritesFromHost;

async function exportFavorites() {
    const favs = await getFavorites();
    if (favs.length === 0) {
        showToast(toastFmt('toast.no_favorites_export'));
        return;
    }
    _exportCtx = {
        title: catalogFmt('ui.dialog.favorites'),
        defaultName: exportFileName('favorites', favs.length),
        exportFn: async (fmt, filePath) => {
            const list = typeof capExportList === 'function' ? capExportList(favs.slice()) : favs;
            if (fmt === 'pdf') {
                const headers = [
                    catalogFmt('ui.export.col_name'),
                    catalogFmt('ui.export.col_type'),
                    catalogFmt('ui.export.col_format'),
                    catalogFmt('ui.export.col_path'),
                ];
                const rows = list.map(f => [f.name, f.type, f.format || f.daw || '', f.path]);
                await window.vstUpdater.exportPdf(catalogFmt('ui.dialog.favorites'), headers, rows, filePath);
            } else if (fmt === 'csv' || fmt === 'tsv') {
                const sep = fmt === 'tsv' ? '\t' : ',';
                const esc = (v) => {
                    const s = String(v || '');
                    return s.includes(sep) || s.includes('"') || s.includes('\n') ? '"' + s.replace(/"/g, '""') + '"' : s;
                };
                const lines = [
                    catalogFmt('ui.export.col_name') +
                        sep +
                        catalogFmt('ui.export.col_type') +
                        sep +
                        catalogFmt('ui.export.col_format') +
                        sep +
                        catalogFmt('ui.export.col_path'),
                ];
                for (const f of list) lines.push([f.name, f.type, f.format || f.daw || '', f.path].map(esc).join(sep));
                await window.__TAURI__.core.invoke('write_text_file', {filePath, contents: lines.join('\n')});
            } else if (fmt === 'toml') {
                await window.vstUpdater.exportToml({favorites: list}, filePath);
            } else {
                await window.__TAURI__.core.invoke('write_text_file', {
                    filePath,
                    contents: JSON.stringify(list, null, 2)
                });
            }
        }
    };
    showExportModal('favorites', catalogFmt('ui.dialog.favorites'), favs.length);
}

async function importFavorites() {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;
    const selected = await dialogApi.open({title: catalogFmt('ui.dialog.import_favorites'), multiple: false, filters: ALL_IMPORT_FILTERS});
    if (!selected) return;
    const filePath = typeof selected === 'string' ? selected : selected.path;
    if (!filePath) return;
    try {
        let imported;
        if (filePath.endsWith('.toml')) {
            const data = await window.vstUpdater.importToml(filePath);
            imported = data.favorites || data;
        } else {
            const text = await window.__TAURI__.core.invoke('read_text_file', {filePath});
            imported = JSON.parse(text);
        }
        if (!Array.isArray(imported)) throw new Error('Expected an array');
        let added = 0;
        for (const item of imported) {
            if (!item.path) continue;
            const k = normalizeFavoritePathKey(item.path);
            const ok = await window.vstUpdater.favoritesAdd(
                item.type || 'sample', k, item.name || k.split('/').pop() || k,
                item.format || null, item.daw || null,
                item.addedAt || new Date().toISOString(),
            );
            if (ok) added++;
        }
        await prepareBadgeContext();
        renderFavorites();
        showToast(toastFmt('toast.imported_favorites', {added, dup: imported.length - added}));
    } catch (e) {
        showToast(toastFmt('toast.import_failed_favs', {err: e.message || e}), 4000, 'error');
    }
}

async function clearFavorites() {
    if (!confirm(appFmt('confirm.remove_all_favorites'))) return;
    try {
        await window.vstUpdater.favoritesClear();
        await prepareBadgeContext();
        if (typeof window.updateFavBtn === 'function') window.updateFavBtn();
        showToast(toastFmt('toast.all_favorites_cleared'));
        renderFavorites();
    } catch (e) {
        showToast(toastFmt('toast.failed', {err: e.message || e}), 4000, 'error');
    }
}

let _favSearch = '';
let _lastFavMode = 'fuzzy';

registerFilter('filterFavorites', {
    inputId: 'favSearchInput',
    regexToggleId: 'regexFavorites',
    resetOffset() {
        _favRenderCount = 0;
    },
    fetchFn() {
        _favSearch = this.lastSearch || '';
        _lastFavMode = this.lastMode || 'fuzzy';
        renderFavorites();
    },
});

async function renderFavorites() {
    if (typeof saveAllFilterStates === 'function') saveAllFilterStates();
    const list = document.getElementById('favList');
    const empty = document.getElementById('favEmptyState');
    if (!list) return;

    const favs = await getFavorites();
    const search = _favSearch || (document.getElementById('favSearchInput')?.value || '').trim();
    const typeSet = typeof getMultiFilterValues === 'function' ? getMultiFilterValues('favTypeFilter') : null;

    let filtered = favs.filter(f => {
        if (typeSet && !typeSet.has(f.type)) return false;
        return true;
    });
    if (search) {
        const scored = filtered.map(f => ({
            f,
            score: searchScore(search, [f.name, f.path], _lastFavMode)
        })).filter(s => s.score > 0);
        scored.sort((a, b) => b.score - a.score);
        filtered = scored.map(s => s.f);
    }

    if (filtered.length === 0) {
        list.innerHTML = '';
        if (empty) empty.style.display = '';
        if (favs.length > 0 && filtered.length === 0) {
            list.innerHTML = '<div class="state-message"><div class="state-icon">&#128269;</div><h2>No matching favorites</h2></div>';
            if (empty) empty.style.display = 'none';
        }
        return;
    }
    if (empty) empty.style.display = 'none';

    const FAV_PAGE = 200;
    _favFiltered = filtered;
    _favRenderCount = 0;
    const page = filtered.slice(0, FAV_PAGE);
    list.innerHTML = page.map(f => {
        const typeLabel = {
            plugin: 'Plugin',
            sample: 'Sample',
            video: 'Video',
            daw: 'DAW Project',
            preset: 'Preset',
            folder: 'Folder',
            file: 'File'
        }[f.type] || f.type;
        const typeClass = {
            plugin: 'type-vst3',
            sample: 'format-wav',
            video: 'format-default',
            daw: 'daw-ableton-live',
            preset: 'format-default',
            folder: 'format-default',
            file: 'format-default'
        }[f.type] || 'format-default';
        const extra = f.format ? `<span class="format-badge format-default">${escapeHtml(f.format)}</span>` : '';
        const daw = f.daw ? `<span class="format-badge ${getDawBadgeClass ? getDawBadgeClass(f.daw) : 'format-default'}">${escapeHtml(f.daw)}</span>` : '';
        const hp = escapeHtml(f.path);
        const isPlaying =
            f.type === 'sample' &&
            typeof audioPlayerPath !== 'undefined' &&
            normalizeFavoritePathKey(audioPlayerPath) === normalizeFavoritePathKey(f.path) &&
            (typeof isAudioPlaying === 'function' ? isAudioPlaying() : typeof audioPlayer !== 'undefined' && audioPlayer && !audioPlayer.paused);
        const playBtn = f.type === 'sample'
            ? `<button class="btn-small btn-play${isPlaying ? ' playing' : ''}" data-action="previewAudio" data-path="${hp}" title="Play">${isPlaying ? '&#9646;&#9646;' : '&#9654;'}</button>`
            : '';
        const loopActive = isPlaying && typeof audioLooping !== 'undefined' && audioLooping;
        const loopBtn = f.type === 'sample'
            ? `<button class="btn-small btn-loop${loopActive ? ' active' : ''}" data-action="toggleRowLoop" data-path="${hp}" title="Loop">&#8634;</button>`
            : '';
        const goTab = typeof _itemTypeToTab === 'function' ? _itemTypeToTab(f.type) : null;
        const goBtn = goTab
            ? `<button class="btn-small btn-secondary" data-action="favGoToTable" data-path="${hp}" data-tab="${goTab}" style="padding:2px 6px;font-size:9px;" title="Show in ${typeLabel} tab">&#8599;</button>`
            : '';
        const nameContent = _favSearch && typeof highlightMatch === 'function' ? highlightMatch(f.name, _favSearch, _lastFavMode) : escapeHtml(f.name);
        const nameEl = goTab
            ? `<a class="fav-name fav-name-link" data-action="favGoToTable" data-path="${hp}" data-tab="${goTab}" title="${hp}">${nameContent}</a>`
            : `<span class="fav-name" title="${hp}">${nameContent}</span>`;
        const cursor = (f.type === 'sample' || f.type === 'daw') ? ' style="cursor:pointer;"' : '';
        return `<div class="fav-item" data-path="${hp}" data-type="${f.type}" data-name="${escapeHtml(f.name)}"${cursor}>
      <span class="fav-star">&#9733;</span>
      <span class="fav-type"><span class="format-badge ${typeClass}">${typeLabel}</span></span>
      ${nameEl}
      ${extra}${daw}
      <span class="fav-actions">
        ${playBtn}${loopBtn}${goBtn}
        <button class="btn-small btn-folder" data-action="openFavFolder" data-path="${hp}" data-type="${f.type}" title="Reveal in Finder">&#128193;</button>
        <button class="btn-small btn-stop" data-action="removeFav" data-path="${hp}" title="Remove from favorites">&#10005;</button>
      </span>
    </div>`;
    }).join('');
    _favRenderCount = page.length;
    if (_favRenderCount < filtered.length) {
        list.insertAdjacentHTML('beforeend',
            `<div id="favLoadMore" data-action="loadMoreFavs" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;font-size:12px;">
        Showing ${_favRenderCount} of ${filtered.length} — click to load more
      </div>`);
    }
    if (typeof initFavDragReorder === 'function') requestAnimationFrame(initFavDragReorder);
    _favHasRendered = true;
    _favDirty = false;
}

let _favFiltered = [];
let _favRenderCount = 0;

function loadMoreFavs() {
    const FAV_PAGE = 200;
    const list = document.getElementById('favList');
    const more = document.getElementById('favLoadMore');
    if (more) more.remove();
    const next = _favFiltered.slice(_favRenderCount, _favRenderCount + FAV_PAGE);
    // Reuse the same rendering from renderFavorites — inline here
    list.insertAdjacentHTML('beforeend', next.map(f => {
        const typeLabel = {
            plugin: 'Plugin',
            sample: 'Sample',
            video: 'Video',
            daw: 'DAW Project',
            preset: 'Preset',
            folder: 'Folder',
            file: 'File'
        }[f.type] || f.type;
        const typeClass = {
            plugin: 'type-vst3',
            sample: 'format-wav',
            video: 'format-default',
            daw: 'daw-ableton-live',
            preset: 'format-default',
            folder: 'format-default',
            file: 'format-default'
        }[f.type] || 'format-default';
        const extra = f.format ? `<span class="format-badge format-default">${escapeHtml(f.format)}</span>` : '';
        const hp = escapeHtml(f.path);
        const goTab = typeof _itemTypeToTab === 'function' ? _itemTypeToTab(f.type) : null;
        const goBtn = goTab
            ? `<button class="btn-small btn-secondary" data-action="favGoToTable" data-path="${hp}" data-tab="${goTab}" style="padding:2px 6px;font-size:9px;" title="Show in ${typeLabel} tab">&#8599;</button>`
            : '';
        const nameContent = _favSearch && typeof highlightMatch === 'function' ? highlightMatch(f.name, _favSearch, _lastFavMode) : escapeHtml(f.name);
        const nameEl = goTab
            ? `<a class="fav-name fav-name-link" data-action="favGoToTable" data-path="${hp}" data-tab="${goTab}" title="${hp}">${nameContent}</a>`
            : `<span class="fav-name" title="${hp}">${nameContent}</span>`;
        const playBtn = f.type === 'sample'
            ? `<button class="btn-small btn-play" data-action="previewAudio" data-path="${hp}" title="Play">&#9654;</button>`
            : '';
        const loopActiveMore = f.type === 'sample' &&
            typeof audioPlayerPath !== 'undefined' &&
            normalizeFavoritePathKey(audioPlayerPath) === normalizeFavoritePathKey(f.path) &&
            typeof audioLooping !== 'undefined' && audioLooping;
        const loopBtn = f.type === 'sample'
            ? `<button class="btn-small btn-loop${loopActiveMore ? ' active' : ''}" data-action="toggleRowLoop" data-path="${hp}" title="Loop">&#8634;</button>`
            : '';
        return `<div class="fav-item" data-path="${hp}" data-type="${f.type}" data-name="${escapeHtml(f.name)}">
      <span class="fav-star">&#9733;</span>
      <span class="fav-type"><span class="format-badge ${typeClass}">${typeLabel}</span></span>
      ${nameEl}${extra}
      <span class="fav-actions">
        ${playBtn}${loopBtn}${goBtn}
        <button class="btn-small btn-folder" data-action="openFavFolder" data-path="${hp}" data-type="${f.type}" title="Reveal in Finder">&#128193;</button>
        <button class="btn-small btn-stop" data-action="removeFav" data-path="${hp}" title="Remove">&#10005;</button>
      </span>
    </div>`;
    }).join(''));
    _favRenderCount += next.length;
    if (_favRenderCount < _favFiltered.length) {
        list.insertAdjacentHTML('beforeend',
            `<div id="favLoadMore" data-action="loadMoreFavs" style="text-align:center;padding:12px;color:var(--text-muted);cursor:pointer;font-size:12px;">
        Showing ${_favRenderCount} of ${_favFiltered.length} — click to load more
      </div>`);
    }
}

// ── Favorites expanded meta panel (waveform + spectrogram) ──
let _favExpandedPath = null;
let _favMetaDrawSeq = 0;

function closeFavMeta() {
    const panel = document.getElementById('favMetaPanel');
    if (panel) panel.remove();
    const prev = document.querySelector('.fav-item.fav-expanded');
    if (prev) prev.classList.remove('fav-expanded');
    _favExpandedPath = null;
}

async function expandFavMeta(filePath) {
    // Close any existing audio-table meta row so only one panel is open
    if (typeof closeMetaRow === 'function') closeMetaRow();
    closeFavMeta();

    const list = document.getElementById('favList');
    if (!list) return;
    const favItem = list.querySelector(`.fav-item[data-path="${CSS.escape(filePath)}"]`);
    if (!favItem) return;

    _favExpandedPath = filePath;
    favItem.classList.add('fav-expanded');

    // Insert loading panel after fav item
    const panel = document.createElement('div');
    panel.id = 'favMetaPanel';
    panel.className = 'fav-meta-panel';
    panel.innerHTML = `<div class="audio-meta-panel" style="justify-items:center;"><div class="spinner" style="width:18px;height:18px;"></div></div>`;
    favItem.after(panel);
    favItem.scrollIntoView({behavior: 'smooth', block: 'nearest'});

    try {
        const meta = await window.vstUpdater.getAudioMetadata(filePath);
        if (_favExpandedPath !== filePath) return;

        let items = '';
        if (typeof metaItem === 'function') {
            const _fmt = typeof _audioFmt === 'function' ? _audioFmt : (k) => k;
            items += metaItem(_fmt('ui.audio.meta_label_file_name'), meta.fileName, true);
            items += metaItem(_fmt('ui.audio.meta_label_format'), meta.format);
            items += metaItem(_fmt('ui.audio.meta_label_size'), typeof formatAudioSize === 'function' ? formatAudioSize(meta.sizeBytes) : meta.sizeBytes);
            if (meta.sampleRate) items += metaItem(_fmt('ui.audio.meta_label_sample_rate'), meta.sampleRate.toLocaleString() + ' Hz');
            if (meta.bitsPerSample) items += metaItem(_fmt('ui.audio.meta_label_bit_depth'), meta.bitsPerSample + '-bit');
            if (meta.channels) {
                const chVal = meta.channels === 1 ? 'Mono' : meta.channels === 2 ? 'Stereo' : meta.channels + ' ch';
                items += metaItem(_fmt('ui.audio.meta_label_channels'), chVal);
            }
            if (meta.duration) items += metaItem(_fmt('ui.audio.meta_label_duration'), typeof formatTime === 'function' ? formatTime(meta.duration) : meta.duration);
        }

        const waveformHtml = `<div class="meta-waveform" id="metaWaveformBox" data-path="${escapeHtml(filePath)}" title="Click to seek">
      <canvas id="metaWaveformCanvas"></canvas>
      <div class="waveform-progress-fill"></div>
      <div class="waveform-loop-region" style="display:none;"></div>
      <div class="waveform-loop-brace waveform-loop-brace-start" data-loop-brace="start" style="display:none;left:25%;" title="Drag to set loop start"></div>
      <div class="waveform-loop-brace waveform-loop-brace-end" data-loop-brace="end" style="display:none;left:75%;" title="Drag to set loop end"></div>
      <button type="button" class="waveform-loop-toggle" data-action="toggleMetaLoopRegion" title="Toggle loop region">L</button>
      <div class="waveform-cursor" style="left:0;"></div>
      <div class="waveform-time-label">${meta.duration && typeof formatTime === 'function' ? formatTime(meta.duration) : ''}</div>
    </div>
    <div class="meta-waveform" style="height:80px;cursor:default;" title="Spectrogram">
      <canvas id="metaSpectrogramCanvas" width="800" height="80" style="position:absolute;top:0;left:0;width:100%;height:100;"></canvas>
      <span style="position:absolute;top:2px;left:4px;font-size:8px;color:var(--text-dim);pointer-events:none;">Spectrogram</span>
    </div>`;

        panel.innerHTML = `<div class="audio-meta-panel"><span class="meta-close-btn" data-action="closeFavMeta" title="Close">&#10005;</span>${waveformHtml}${items}</div>`;

        // Hydrate loop-region braces
        if (typeof applyMetaLoopRegionUI === 'function') applyMetaLoopRegionUI(filePath);

        // Set expandedMetaPath so playback cursor updates work
        if (typeof expandedMetaPath !== 'undefined') expandedMetaPath = filePath;

        // Draw waveform + spectrogram
        _favMetaDrawSeq++;
        const seq = _favMetaDrawSeq;
        if (typeof _metaPanelDrawSeq !== 'undefined') _metaPanelDrawSeq = seq;

        const doDraw = () => {
            if (typeof scheduleIdleVisualWork === 'function') {
                scheduleIdleVisualWork(() => {
                    if (_favExpandedPath !== filePath) return;
                    if (typeof drawMetaPanelVisuals === 'function') {
                        void drawMetaPanelVisuals(filePath, seq);
                    }
                }, {delayMs: 0});
            } else if (typeof drawMetaPanelVisuals === 'function') {
                void drawMetaPanelVisuals(filePath, seq);
            }
        };

        if (filePath === audioPlayerPath) {
            requestAnimationFrame(doDraw);
        } else {
            doDraw();
        }

        // Sync playhead if already playing this file
        if (typeof audioPlayerPath !== 'undefined' && audioPlayerPath === filePath) {
            requestAnimationFrame(() => {
                if (typeof updatePlaybackTime === 'function') updatePlaybackTime();
            });
        }
    } catch (err) {
        panel.innerHTML = `<div class="audio-meta-panel"><span style="color:var(--red);">Failed to load metadata</span></div>`;
    }
}

// Wire up fav actions via delegation
document.addEventListener('click', (e) => {
    const el = e.target.closest('[data-action="removeFav"]');
    if (el) {
        removeFavorite(el.dataset.path);
        return;
    }
    const folder = e.target.closest('[data-action="openFavFolder"]');
    if (folder) {
        const type = folder.dataset.type;
        const path = folder.dataset.path;
        if (type === 'plugin') openFolder(path);
        else if (type === 'sample' || type === 'video') openAudioFolder(path);
        else if (type === 'daw') openDawFolder(path);
        else if (type === 'preset') openPresetFolder(path);
        return;
    }
    // "Go to table" link / button
    const goEl = e.target.closest('[data-action="favGoToTable"]');
    if (goEl) {
        e.preventDefault();
        const path = goEl.dataset.path;
        const tab = goEl.dataset.tab;
        if (tab && typeof _goToTableItem === 'function') {
            void _goToTableItem(tab, path);
        }
        return;
    }
    // Close fav meta panel
    if (e.target.closest('[data-action="closeFavMeta"]')) {
        closeFavMeta();
        return;
    }
    // Single click on sample favorite → play + expand meta
    const favItem = e.target.closest('.fav-item[data-type="sample"]');
    if (favItem && !e.target.closest('.fav-actions') && !e.target.closest('button') && !e.target.closest('a')) {
        const path = favItem.dataset.path;
        if (path && typeof previewAudio === 'function') {
            previewAudio(path).then(() => {
                if (_favExpandedPath === path) {
                    closeFavMeta();
                } else {
                    expandFavMeta(path);
                }
            });
        }
    }
});

// Double-click on DAW favorite → open in DAW, plugin → open KVR
document.addEventListener('dblclick', (e) => {
    const favItem = e.target.closest('.fav-item');
    if (!favItem || e.target.closest('.fav-actions') || e.target.closest('button')) return;
    const type = favItem.dataset.type;
    const path = favItem.dataset.path;
    const name = favItem.dataset.name || '';

    if (type === 'daw') {
        const daw = favItem.querySelector('.format-badge')?.textContent || 'DAW';
        showToast(toastFmt('toast.opening_in_daw', {name, daw}));
        window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.daw_not_installed', {
            daw,
            err
        }), 4000, 'error'));
    } else if (type === 'plugin') {
        const plugin = typeof allPlugins !== 'undefined' && findByPath(allPlugins, path);
        const kvrUrl = plugin ? (plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer)) : buildKvrUrl(name, '');
        window.vstUpdater.openUpdate(kvrUrl);
    } else if (type === 'preset') {
        if (typeof openPresetFolder === 'function') openPresetFolder(path);
    }
});

// Export for menu-action handler in ipc.js
window.clearFavorites = clearFavorites;
