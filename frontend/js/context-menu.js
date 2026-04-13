// ── Context Menu ──
const ctxMenu = document.getElementById('ctxMenu');
/** Spread into menu items that already toast or should not echo the label (locale-safe; no English heuristics). */
const _noEcho = {skipEchoToast: true};

/**
 * Attach a shortcuts.js id so the row tooltip includes formatKey(...) in parentheses.
 * (scripts load before context-menu.js may still call this at contextmenu time.)
 */
function shortcutTip(shortcutId) {
    return shortcutId ? {shortcutId} : {};
}

function escapeAttr(s) {
    return String(s).replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/\r?\n/g, ' ');
}

/** Tooltip: label, or label + (shortcut) when shortcutId / shortcutHint is set. */
function ctxMenuItemTitle(item) {
    const label = item.label != null ? String(item.label) : '';
    let hint = '';
    if (item.shortcutId && typeof getShortcuts === 'function' && typeof formatKey === 'function') {
        const sc = getShortcuts()[item.shortcutId];
        if (sc && sc.key !== undefined) hint = formatKey(sc);
    } else if (item.shortcutHint) {
        hint = String(item.shortcutHint);
    }
    if (!hint) return escapeAttr(label);
    return escapeAttr(`${label} (${hint})`);
}

function showContextMenu(e, items) {
    e.preventDefault();
    // Store callbacks and render
    ctxMenu._actions = {};
    ctxMenu._labels = {};
    ctxMenu._skipEcho = {};
    ctxMenu.innerHTML = items.map((item, i) => {
        if (item === '---') return '<div class="ctx-menu-sep"></div>';
        if (item.action) {
            ctxMenu._actions[i] = item.action;
            ctxMenu._labels[i] = item.label;
            if (item.skipEchoToast) ctxMenu._skipEcho[i] = true;
        }
        const cls = item.disabled ? ' ctx-disabled' : '';
        const title = ctxMenuItemTitle(item);
        const titleAttr = title ? ` title="${title}"` : '';
        const rawLabel = item.label != null ? String(item.label) : '';
        const safeLabel = typeof escapeHtml === 'function' ? escapeHtml(rawLabel) : rawLabel;
        return `<div class="ctx-menu-item${cls}" data-ctx-idx="${i}"${titleAttr}>
      <span class="ctx-icon">${item.icon || ''}</span>${safeLabel}
    </div>`;
    }).join('');

    ctxMenu.classList.add('visible');

    // Position — keep within viewport
    const rect = ctxMenu.getBoundingClientRect();
    let x = e.clientX, y = e.clientY;
    if (x + rect.width > window.innerWidth) x = window.innerWidth - rect.width - 4;
    if (y + rect.height > window.innerHeight) y = window.innerHeight - rect.height - 4;
    ctxMenu.style.left = x + 'px';
    ctxMenu.style.top = y + 'px';
}

function hideContextMenu() {
    ctxMenu.classList.remove('visible');
    ctxMenu._actions = {};
    ctxMenu._labels = {};
    ctxMenu._skipEcho = {};
}

// Click on menu item
ctxMenu.addEventListener('click', (e) => {
    const item = e.target.closest('.ctx-menu-item');
    if (!item || item.classList.contains('ctx-disabled')) return;
    const idx = item.dataset.ctxIdx;
    const action = ctxMenu._actions[idx];
    const label = ctxMenu._labels?.[idx];
    const skipEcho = ctxMenu._skipEcho?.[idx];
    hideContextMenu();
    if (action) action();
    if (label && !skipEcho) showToast(label);
});

// Dismiss on click outside or Escape
document.addEventListener('click', (e) => {
    if (!ctxMenu.contains(e.target)) hideContextMenu();
});
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hideContextMenu();
});

// Open file with specific app
function openWithApp(filePath, appName) {
    window.vstUpdater.openWithApp(filePath, appName).then(() => {
        showToast(toastFmt('toast.opening_in_app', {app: appName}));
    }).catch(err => {
        showToast(toastFmt('toast.app_not_available', {app: appName, err}), 4000, 'error');
    });
}

// Copy helper
function copyToClipboard(text) {
    navigator.clipboard.writeText(text).then(() => {
        showToast(toastFmt('toast.copied_clipboard'));
    }).catch(e => {
        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
    });
}

/** Settings → Playback `videoAudioRoute` — engine vs WebView `<video>` (see `video.js`). */
function ctxMenuVideoAudioRouteItems() {
    const rt =
        typeof prefs !== 'undefined' && typeof prefs.getItem === 'function' && prefs.getItem('videoAudioRoute') === 'html5'
            ? 'html5'
            : 'engine';
    const act = typeof catalogFmt === 'function' ? catalogFmt('ui.palette.autoplay_source_active') : 'Active';
    const engBase = catalogFmt('menu.video_audio_route_engine');
    const h5Base = catalogFmt('menu.video_audio_route_html5');
    return [
        {
            icon: '&#127898;',
            label: rt === 'engine' ? `${engBase} · ${act}` : engBase,
            ..._noEcho,
            ...shortcutTip('videoAudioRouteEngine'),
            action: () => {
                if (typeof settingSetVideoAudioRoute === 'function') settingSetVideoAudioRoute('engine');
            },
        },
        {
            icon: '&#128187;',
            label: rt === 'html5' ? `${h5Base} · ${act}` : h5Base,
            ..._noEcho,
            ...shortcutTip('videoAudioRouteHtml5'),
            action: () => {
                if (typeof settingSetVideoAudioRoute === 'function') settingSetVideoAudioRoute('html5');
            },
        },
    ];
}

/**
 * When no specific surface matched, still offer copy / navigation so every app chrome
 * surface gets a context menu (dock overlay, empty tab panels, chrome between cards, etc.).
 */
function buildFallbackShellContextMenu(e) {
    const t = e.target;
    const items = [];
    const sel = window.getSelection?.()?.toString?.()?.trim();
    if (sel) {
        items.push({icon: '&#128203;', label: appFmt('menu.copy'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(sel)});
    }
    if (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA') {
        const iv = t.value || '';
        if (iv) items.push({
            icon: '&#128203;',
            label: appFmt('menu.copy'), ..._noEcho, ...shortcutTip('copyPath'),
            action: () => copyToClipboard(iv)
        });
    }
    const actionable = t.closest('[data-action]');
    if (actionable && typeof actionable.click === 'function') {
        const tag = actionable.tagName;
        if (!['INPUT', 'SELECT', 'TEXTAREA'].includes(tag)) {
            items.push({
                icon: '&#9654;',
                label: appFmt('menu.ctx_activate'), ..._noEcho,
                action: () => actionable.click()
            });
        }
    }
    const focusable = t.closest('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])');
    if (focusable && typeof focusable.focus === 'function') {
        items.push({
            icon: '&#128269;', label: appFmt('menu.ctx_focus'), ..._noEcho, action: () => {
                try {
                    focusable.focus();
                } catch (_) {
                }
            }
        });
    }
    if (!sel && t.tagName !== 'INPUT' && t.tagName !== 'TEXTAREA') {
        const vis = (t.innerText || t.textContent || '').trim().replace(/\s+/g, ' ');
        if (vis.length > 0 && vis.length <= 10000) {
            items.push({
                icon: '&#128203;',
                label: appFmt('menu.ctx_copy_visible_text'), ..._noEcho, ...shortcutTip('copyPath'),
                action: () => copyToClipboard(vis.slice(0, 16000))
            });
        }
    }
    if (items.length > 0) items.push('---');
    items.push({
        icon: '&#128179;', label: appFmt('menu.cmd_palette'), ...shortcutTip('commandPalette'), action: () => {
            if (typeof openPalette === 'function') void openPalette();
        }
    });
    items.push({
        icon: '&#10068;', label: appFmt('menu.help_keyboard_shortcuts'), ...shortcutTip('help'), action: () => {
            if (typeof toggleHelpOverlay === 'function') toggleHelpOverlay();
        }
    });
    items.push({icon: '&#9881;', label: appFmt('menu.tab_settings'), ...shortcutTip('openPrefs'), action: () => switchTab('settings')});
    return items;
}

/**
 * Videos tab row menu (right-click).
 * @param {HTMLElement} videoRow `#videoTableBody tr[data-video-path]`
 */
function getVideoTableRowContextMenuItems(videoRow) {
    const path = videoRow.dataset.videoPath;
    const name = videoRow.querySelector('.col-name')?.getAttribute('title')?.trim() ||
        videoRow.querySelector('.col-name')?.textContent?.trim() || '';
    const isPlaying =
        typeof audioPlayerPath !== 'undefined' &&
        audioPlayerPath === path &&
        typeof videoPlayerPath !== 'undefined' &&
        videoPlayerPath === path &&
        typeof isAudioPlaying === 'function' &&
        isAudioPlaying();
    return [
        {
            icon: isPlaying ? '&#9646;&#9646;' : '&#9654;',
            label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'),
            ..._noEcho,
            ...shortcutTip('playPause'),
            action: () => {
                if (typeof previewVideo === 'function') void previewVideo(path, { minimizeFloatingPlayer: true });
            },
        },
        {
            icon: '&#8634;',
            label: appFmt('menu.loop'),
            ..._noEcho,
            ...shortcutTip('toggleLoop'),
            action: () => {
                if (typeof toggleVideoRowLoop === 'function') toggleVideoRowLoop(path, new MouseEvent('click'));
            },
        },
        '---',
        ...ctxMenuVideoAudioRouteItems(),
        '---',
        {
            icon: '&#128193;',
            label: appFmt('menu.reveal_in_finder'),
            ..._noEcho,
            ...shortcutTip('revealFile'),
            action: () => openVideoFile(path),
        },
        {
            icon: '&#9889;',
            label: appFmt('menu.open_with_default_app'),
            ..._noEcho,
            action: () =>
                window.vstUpdater.openFileDefault(path).catch(err =>
                    showToast(toastFmt('toast.failed_open_file', { err: err.message || err }), 4000, 'error'),
                ),
        },
        {
            icon: '&#128194;',
            label: appFmt('menu.show_file_browser'),
            ..._noEcho,
            action: () => {
                switchTab('files');
                setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
            },
        },
        '---',
        { icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name) },
        {
            icon: '&#128203;',
            label: appFmt('menu.copy_path'),
            ..._noEcho,
            ...shortcutTip('copyPath'),
            action: () => copyToClipboard(path),
        },
        '---',
        ...[(() => {
            const f = typeof isFavorite === 'function' && isFavorite(path);
            return {
                icon: f ? '&#9734;' : '&#9733;',
                label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'),
                ..._noEcho,
                ...shortcutTip('toggleFavorite'),
                action: () =>
                    f
                        ? removeFavorite(path)
                        : addFavorite('video', path, name, {
                              format: videoRow.querySelector('.col-format')?.textContent?.trim() || '',
                          }),
            };
        })()],
        { icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name) },
        ...quickTagItems(path, name),
    ];
}

// ── Right-click handlers ──
document.addEventListener('contextmenu', (e) => {
    // Always suppress default browser menu on app content
    if (e.target.closest('.app, .audio-now-playing, .header, .stats-bar, .tab-nav, #dockOverlay, .dock-zone-overlay, #splashScreen')) {
        e.preventDefault();
    }

    try {

        // ── Audio player song rows (recently played / search results) ──
        const npItem = e.target.closest('.np-history-item');
        if (npItem) {
            const path = npItem.dataset.path || '';
            const name = npItem.querySelector('.np-h-name')?.textContent?.trim() || '';
            const isPlaying = typeof audioPlayerPath !== 'undefined' && audioPlayerPath === path && (typeof isAudioPlaying === 'function' ? isAudioPlaying() : typeof audioPlayer !== 'undefined' && audioPlayer && !audioPlayer.paused);
            const items = [
                {
                    icon: isPlaying ? '&#9646;&#9646;' : '&#9654;',
                    label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'), ..._noEcho, ...shortcutTip('playPause'),
                    action: () => typeof previewAudio === 'function' && previewAudio(path)
                },
                {
                    icon: '&#8634;',
                    label: appFmt('menu.loop'), ..._noEcho, ...shortcutTip('toggleLoop'),
                    action: () => typeof toggleRowLoop === 'function' && toggleRowLoop(path, new MouseEvent('click'))
                },
                '---',
                {
                    icon: '&#127926;',
                    label: appFmt('menu.open_in_music'), ..._noEcho,
                    action: () => typeof openWithApp === 'function' && openWithApp(path, 'Music')
                },
                {
                    icon: '&#127911;',
                    label: appFmt('menu.open_in_quicktime'), ..._noEcho,
                    action: () => typeof openWithApp === 'function' && openWithApp(path, 'QuickTime Player')
                },
                {
                    icon: '&#127908;',
                    label: appFmt('menu.open_audacity'), ..._noEcho,
                    action: () => typeof openWithApp === 'function' && openWithApp(path, 'Audacity')
                },
                '---',
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => typeof openAudioFolder === 'function' && openAudioFolder(path)
                },
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => {
                            if (typeof loadDirectory === 'function') loadDirectory(path.replace(/\/[^/]+$/, ''));
                        }, 200);
                    }
                },
                {
                    icon: '&#127925;', label: appFmt('menu.show_in_samples_tab'), ..._noEcho, action: () => {
                        switchTab('samples');
                        setTimeout(() => {
                            const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
                            if (row) {
                                row.scrollIntoView({behavior: 'smooth', block: 'center'});
                                row.classList.add('row-playing');
                                setTimeout(() => row.classList.remove('row-playing'), 2000);
                            } else {
                                // Sample not in table — add it on the spot from recently played data
                                const recent = typeof recentlyPlayed !== 'undefined' ? recentlyPlayed.find(r => r.path === path) : null;
                                if (recent) {
                                    const sample = {
                                        name: recent.name || name,
                                        path,
                                        directory: path.replace(/\/[^/]+$/, ''),
                                        format: recent.format || path.split('.').pop().toUpperCase(),
                                        size: 0,
                                        sizeFormatted: recent.size || '?',
                                        modified: ''
                                    };
                                    if (typeof allAudioSamples !== 'undefined') allAudioSamples.push(sample);
                                    if (typeof filteredAudioSamples !== 'undefined') filteredAudioSamples.push(sample);
                                    const tbody = document.getElementById('audioTableBody');
                                    if (tbody && typeof buildAudioRow === 'function') {
                                        tbody.insertAdjacentHTML('beforeend', buildAudioRow(sample));
                                        const newRow = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
                                        if (newRow) {
                                            newRow.scrollIntoView({behavior: 'smooth', block: 'center'});
                                            newRow.classList.add('row-playing');
                                            setTimeout(() => newRow.classList.remove('row-playing'), 2000);
                                        }
                                    }
                                    showToast(toastFmt('toast.added_name_to_samples', {name}));
                                } else {
                                    const input = document.getElementById('audioSearchInput');
                                    if (input) {
                                        input.value = name;
                                        input.dispatchEvent(new Event('input'));
                                    }
                                }
                            }
                        }, 200);
                    }
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name)},
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                '---',
            ];
            if (typeof isFavorite === 'function') {
                const fav = isFavorite(path);
                items.push({
                    icon: fav ? '&#9734;' : '&#9733;',
                    label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                    action: () => fav ? removeFavorite(path) : addFavorite('sample', path, name)
                });
            }
            if (typeof showNoteEditor === 'function') {
                items.push({
                    icon: '&#128221;',
                    label: appFmt('menu.add_note_tags'),
                    ...shortcutTip('addNote'),
                    action: () => showNoteEditor(path, name)
                });
            }
            items.push(...quickTagItems(path, name));
            items.push('---');
            items.push({
                icon: '&#128270;',
                label: appFmt('menu.find_similar_samples'),
                ...shortcutTip('findSimilar'), action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path)
            });
            showContextMenu(e, items);
            return;
        }

        // ── Similarity result rows ──
        const simRow = e.target.closest('[data-similar-path]');
        if (simRow) {
            const path = simRow.dataset.similarPath || '';
            const name = path.split('/').pop().replace(/\.[^.]+$/, '');
            const items = [
                {
                    icon: '&#9654;',
                    label: appFmt('menu.play'), ..._noEcho, ...shortcutTip('playPause'),
                    action: () => typeof previewAudio === 'function' && previewAudio(path)
                },
                {
                    icon: '&#8634;',
                    label: appFmt('menu.loop'), ..._noEcho, ...shortcutTip('toggleLoop'),
                    action: () => typeof toggleRowLoop === 'function' && toggleRowLoop(path, new MouseEvent('click'))
                },
                '---',
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => typeof openAudioFolder === 'function' && openAudioFolder(path)
                },
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => {
                            if (typeof loadDirectory === 'function') loadDirectory(path.replace(/\/[^/]+$/, ''));
                        }, 200);
                    }
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                {
                    icon: '&#128270;', label: appFmt('menu.find_similar_to_this'), action: () => {
                        typeof closeSimilarPanel === 'function' && closeSimilarPanel();
                        typeof findSimilarSamples === 'function' && findSimilarSamples(path);
                    }
                },
            ];
            if (typeof isFavorite === 'function') {
                const fav = isFavorite(path);
                items.push({
                    icon: fav ? '&#9734;' : '&#9733;',
                    label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                    action: () => fav ? removeFavorite(path) : addFavorite('sample', path, name)
                });
            }
            showContextMenu(e, items);
            return;
        }

        // ── Plugin cards ──
        const pluginCard = e.target.closest('#pluginList .plugin-card');

        // Helper: build quick-tag menu items for a path
        function quickTagItems(path, name) {
            const items = [];
            if (typeof getNote !== 'function' || typeof getAllTags !== 'function') return items;
            const note = getNote(path);
            const currentTags = note?.tags || [];
            const allTags = getAllTags();
            if (allTags.length > 0) {
                items.push('---');
                for (const tag of allTags.slice(0, 8)) {
                    const has = currentTags.includes(tag);
                    items.push({
                        icon: has ? '&#10003;' : '&#9634;',
                        label: has ? appFmt('menu.remove_tag_named', {tag}) : appFmt('menu.add_tag_named', {tag}), ..._noEcho,
                        action: () => {
                            if (has) removeTagFromItem(path, tag); else addTagToItem(path, tag);
                            showToast(has ? toastFmt('toast.tag_removed', {tag}) : toastFmt('toast.tag_added', {tag}));
                        }
                    });
                }
            }
            return items;
        }

        if (pluginCard) {
            e.preventDefault();
            const name = pluginCard.querySelector('h3')?.textContent || '';
            const path = pluginCard.dataset.path || '';
            const kvrBtn = pluginCard.querySelector('[data-action="openKvr"]');
            const mfgBtn = pluginCard.querySelector('[data-action="openUpdate"][title]');
            const folderBtn = pluginCard.querySelector('[data-action="openFolder"]');
            const archBadges = [...pluginCard.querySelectorAll('.arch-badge')].map(b => b.textContent).join(', ');
            const items = [
                {
                    icon: '&#128269;',
                    label: appFmt('menu.open_kvr'), ..._noEcho,
                    action: () => kvrBtn && openKvr(kvrBtn, kvrBtn.dataset.url, kvrBtn.dataset.name)
                },
            ];
            if (mfgBtn && !mfgBtn.disabled) {
                items.push({
                    icon: '&#127760;',
                    label: appFmt('menu.open_manufacturer_site'), ..._noEcho,
                    action: () => openUpdate(mfgBtn.dataset.url)
                });
            }
            items.push({
                icon: '&#128193;',
                label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                action: () => folderBtn && openFolder(folderBtn.dataset.path)
            });
            items.push({
                icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                    switchTab('files');
                    setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
                }
            });
            items.push('---');
            items.push({
                icon: '&#128203;',
                label: appFmt('menu.copy_name'), ..._noEcho,
                action: () => copyToClipboard(name)
            });
            items.push({
                icon: '&#128203;',
                label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'),
                action: () => copyToClipboard(path)
            });
            if (archBadges) {
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy_architecture'), ..._noEcho,
                    action: () => copyToClipboard(archBadges)
                });
            }
            items.push('---');
            if (typeof isFavorite === 'function') {
                const pluginFav = isFavorite(path);
                items.push({
                    icon: pluginFav ? '&#9734;' : '&#9733;',
                    label: pluginFav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                    action: () => pluginFav ? removeFavorite(path) : addFavorite('plugin', path, name, {format: pluginCard.querySelector('.plugin-type')?.textContent})
                });
            }
            if (typeof showNoteEditor === 'function') items.push({
                icon: '&#128221;',
                label: appFmt('menu.add_note'),
                ...shortcutTip('addNote'), action: () => showNoteEditor(path, name)
            });
            if (typeof findProjectsUsingPlugin === 'function') {
                items.push({
                    icon: '&#9889;', label: appFmt('menu.find_projects_using'), action: () => {
                        const projects = findProjectsUsingPlugin(name);
                        showReverseXrefModal(name, projects);
                    }
                });
            }
            items.push(...quickTagItems(path, name));
            showContextMenu(e, items);
            return;
        }

        // ── Audio sample rows ──
        const audioRow = e.target.closest('#audioTableBody tr[data-audio-path]');
        if (audioRow) {
            const path = audioRow.getAttribute('data-audio-path');
            const name = audioRow.querySelector('.col-name')?.textContent || '';
            const isPlaying = audioPlayerPath === path && (typeof isAudioPlaying === 'function' ? isAudioPlaying() : !audioPlayer.paused);
            const items = [
                {
                    icon: isPlaying ? '&#9646;&#9646;' : '&#9654;',
                    label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'), ..._noEcho, ...shortcutTip('playPause'),
                    action: () => previewAudio(path)
                },
                {
                    icon: '&#8634;', label: appFmt('menu.loop'), ..._noEcho, ...shortcutTip('toggleLoop'), action: () => {
                        toggleRowLoop(path, new MouseEvent('click'));
                    }
                },
                '---',
                {
                    icon: '&#127926;',
                    label: appFmt('menu.open_in_music'), ..._noEcho,
                    action: () => openWithApp(path, 'Music')
                },
                {
                    icon: '&#127911;',
                    label: appFmt('menu.open_in_quicktime'), ..._noEcho,
                    action: () => openWithApp(path, 'QuickTime Player')
                },
                {
                    icon: '&#127908;',
                    label: appFmt('menu.open_audacity'), ..._noEcho,
                    action: () => openWithApp(path, 'Audacity')
                },
                {
                    icon: '&#9889;',
                    label: appFmt('menu.open_default_app'), ..._noEcho,
                    action: () => window.vstUpdater.openDawProject(path).catch(e => {
                        if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                    })
                },
                '---',
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => openAudioFolder(path)
                },
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
                    }
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name)},
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                '---',
                ...[(() => {
                    const f = typeof isFavorite === 'function' && isFavorite(path);
                    return {
                        icon: f ? '&#9734;' : '&#9733;',
                        label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                        action: () => f ? removeFavorite(path) : addFavorite('sample', path, name, {format: audioRow.querySelector('.format-badge')?.textContent})
                    };
                })()],
                {icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name)},
                ...quickTagItems(path, name),
                '---',
                {
                    icon: '&#128270;',
                    label: appFmt('menu.find_similar_samples'),
                    ...shortcutTip('findSimilar'), action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path)
                },
                '---',
                ...[(() => {
                    const on = prefs.getItem('expandOnClick') !== 'off';
                    return {
                        icon: on ? '&#9660;' : '&#9654;',
                        label: on ? appFmt('menu.disable_row_expand') : appFmt('menu.enable_row_expand'), ..._noEcho,
                        action: () => {
                            if (on) {
                                // Disable: close any expanded row
                                prefs.setItem('expandOnClick', 'off');
                                const meta = document.getElementById('audioMetaRow');
                                if (meta) {
                                    meta.remove();
                                    expandedMetaPath = null;
                                }
                                const exp = document.querySelector('#audioTableBody tr.row-expanded');
                                if (exp) exp.classList.remove('row-expanded');
                            } else {
                                // Enable, play, and expand the right-clicked row
                                prefs.setItem('expandOnClick', 'on');
                                if (typeof previewAudio === 'function') previewAudio(path);
                                const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
                                if (row) row.click();
                            }
                            if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
                            showToast(on ? toastFmt('toast.row_expand_disabled') : toastFmt('toast.row_expand_enabled'));
                        }
                    };
                })()],
                ...[(() => {
                    const ap = prefs.getItem('autoplayNext') !== 'off';
                    return {
                        icon: ap ? '&#9209;' : '&#9654;',
                        label: ap ? appFmt('menu.disable_autoplay_next') : appFmt('menu.enable_autoplay_next'), ..._noEcho,
                        action: () => {
                            prefs.setItem('autoplayNext', ap ? 'off' : 'on');
                            if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
                            showToast(ap ? toastFmt('toast.autoplay_next_disabled') : toastFmt('toast.autoplay_next_enabled'));
                        }
                    };
                })()],
                '---',
                ...ctxMenuVideoAudioRouteItems(),
            ];
            showContextMenu(e, items);
            return;
        }

        // ── MIDI file rows ──
        const midiRow = e.target.closest('#midiTableBody tr[data-midi-path]');
        if (midiRow) {
            const path = midiRow.getAttribute('data-midi-path');
            const name = midiRow.querySelector('.col-name')?.textContent || '';
            const items = [
                {
                    icon: '&#9654;',
                    label: appFmt('menu.open_garageband'), ..._noEcho,
                    action: () => window.vstUpdater.openWithApp(path, 'GarageBand').catch(() => showToast(toastFmt('toast.garageband_not_found'), 4000, 'error'))
                },
                {
                    icon: '&#127911;',
                    label: appFmt('menu.open_in_logic_pro'), ..._noEcho,
                    action: () => window.vstUpdater.openWithApp(path, 'Logic Pro').catch(() => showToast(toastFmt('toast.logic_not_found'), 4000, 'error'))
                },
                {
                    icon: '&#127925;',
                    label: appFmt('menu.open_ableton_live'), ..._noEcho,
                    action: () => window.vstUpdater.openWithApp(path, 'Ableton Live 12 Standard').catch(() => window.vstUpdater.openWithApp(path, 'Ableton Live 11 Suite').catch(() => showToast(toastFmt('toast.ableton_not_found'), 4000, 'error')))
                },
                {
                    icon: '&#9889;',
                    label: appFmt('menu.open_with_default_app'), ..._noEcho,
                    action: () => window.vstUpdater.openDawProject(path).catch(e => showToast(toastFmt('toast.no_midi_handler', {err: e}), 4000, 'error'))
                },
                '---',
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => typeof openAudioFolder === 'function' && openAudioFolder(path)
                },
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => {
                            if (typeof loadDirectory === 'function') loadDirectory(path.replace(/\/[^/]+$/, ''));
                        }, 200);
                    }
                },
                '---',
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_name'), ..._noEcho,
                    action: () => typeof copyToClipboard === 'function' && copyToClipboard(name)
                },
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'),
                    action: () => typeof copyToClipboard === 'function' && copyToClipboard(path)
                },
                '---',
                ...[(() => {
                    const f = typeof isFavorite === 'function' && isFavorite(path);
                    return {
                        icon: f ? '&#9734;' : '&#9733;',
                        label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                        action: () => f ? removeFavorite(path) : addFavorite('midi', path, name)
                    };
                })()],
                {
                    icon: '&#128221;',
                    label: appFmt('menu.add_note'),
                    ...shortcutTip('addNote'), action: () => typeof showNoteEditor === 'function' && showNoteEditor(path, name)
                },
                ...(typeof quickTagItems === 'function' ? quickTagItems(path, name) : []),
            ];
            showContextMenu(e, items);
            return;
        }

        // ── DAW project rows ──
        const dawRow = e.target.closest('#dawTableBody tr[data-daw-path]');
        if (dawRow) {
            const path = dawRow.dataset.dawPath;
            const name = dawRow.querySelector('.col-name')?.textContent || '';
            const dawName = dawRow.querySelector('.format-badge')?.textContent || 'DAW';
            const items = [
                {
                    icon: '&#9654;', label: appFmt('menu.open_in_daw', {daw: dawName}), ..._noEcho, action: () => {
                        showToast(toastFmt('toast.opening_in_daw', {name, daw: dawName}));
                        window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.daw_not_installed', {
                            daw: dawName,
                            err
                        }), 4000, 'error'));
                    }
                },
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => openDawFolder(path)
                },
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
                    }
                },
                ...(typeof isXrefSupported === 'function' && isXrefSupported(dawRow.querySelector('.format-badge.format-default')?.textContent || '')
                    ? [{
                        icon: '&#9889;',
                        label: appFmt('menu.show_plugins_used'),
                        action: () => showProjectPlugins(path, name)
                    }]
                    : []),
                ...(typeof showProjectViewer === 'function'
                    ? [{
                        icon: '&#128196;',
                        label: appFmt('menu.explore_project_contents'),
                        action: () => showProjectViewer(path, name)
                    }]
                    : []),
                {
                    icon: '&#128221;', label: appFmt('menu.open_in_text_editor'), ..._noEcho, action: () => {
                        const ext = path.split('.').pop().toLowerCase();
                        const xmlFormats = ['als', 'rpp', 'song', 'dawproject'];
                        if (xmlFormats.includes(ext)) {
                            // Decompress ALS first, others open directly
                            if (ext === 'als') {
                                window.vstUpdater.readAlsXml(path).then(xml => {
                                    const tmp = path.replace(/\.als$/i, '_decompressed.xml');
                                    window.vstUpdater.writeTextFile(tmp, xml).then(() => {
                                        window.vstUpdater.openWithApp(tmp, 'TextEdit').catch(e => {
                                            if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                                        });
                                        showToast(toastFmt('toast.decompressed_xml_textedit'));
                                    });
                                }).catch(e => {
                                    if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                                });
                            } else {
                                window.vstUpdater.openWithApp(path, 'TextEdit').catch(() => window.vstUpdater.openDawProject(path).catch(e => {
                                    if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                                }));
                            }
                        } else {
                            // Binary — open with hex editor or default
                            window.vstUpdater.openDawProject(path).catch(e => {
                                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
                            });
                        }
                    }
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name)},
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                '---',
                ...[(() => {
                    const f = typeof isFavorite === 'function' && isFavorite(path);
                    return {
                        icon: f ? '&#9734;' : '&#9733;',
                        label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                        action: () => f ? removeFavorite(path) : addFavorite('daw', path, name, {
                            format: dawRow.querySelector('.format-badge:last-of-type')?.textContent,
                            daw: dawName
                        })
                    };
                })()],
                {icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name)},
                ...quickTagItems(path, name),
            ];
            showContextMenu(e, items);
            return;
        }

        // ── PDF rows ──
        const pdfRow = e.target.closest('#pdfTableBody tr[data-pdf-path]');
        if (pdfRow) {
            const path = pdfRow.dataset.pdfPath;
            const name = pdfRow.querySelector('.col-name')?.getAttribute('title')?.trim() ||
                pdfRow.querySelector('.col-name')?.textContent?.trim() || '';
            const items = [
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => openPdfFile(path)
                },
                {
                    icon: '&#9889;',
                    label: appFmt('menu.open_with_default_app'), ..._noEcho,
                    action: () => window.vstUpdater.openFileDefault(path).catch(err => showToast(toastFmt('toast.failed_open_file', {err: err.message || err}), 4000, 'error'))
                },
                {
                    icon: '&#128195;',
                    label: appFmt('menu.open_in_preview'), ..._noEcho,
                    action: () => openWithApp(path, 'Preview')
                },
                {
                    icon: '&#128196;',
                    label: appFmt('menu.open_in_adobe_acrobat'), ..._noEcho,
                    action: () => openWithApp(path, 'Adobe Acrobat')
                },
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
                    }
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name)},
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                '---',
                ...[(() => {
                    const f = typeof isFavorite === 'function' && isFavorite(path);
                    return {
                        icon: f ? '&#9734;' : '&#9733;',
                        label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                        action: () => f ? removeFavorite(path) : addFavorite('pdf', path, name)
                    };
                })()],
                {icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name)},
                ...quickTagItems(path, name),
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Video rows ──
        const videoRow = e.target.closest('#videoTableBody tr[data-video-path]');
        if (videoRow) {
            showContextMenu(e, getVideoTableRowContextMenuItems(videoRow));
            return;
        }

        // ── Preset rows ──
        const presetRow = e.target.closest('#presetTableBody tr[data-preset-path]');
        if (presetRow) {
            const path = presetRow.dataset.presetPath;
            const name = presetRow.querySelector('td')?.textContent || '';
            const items = [
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => openPresetFolder(path)
                },
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
                    }
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name)},
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                '---',
                ...[(() => {
                    const f = typeof isFavorite === 'function' && isFavorite(path);
                    return {
                        icon: f ? '&#9734;' : '&#9733;',
                        label: f ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                        action: () => f ? removeFavorite(path) : addFavorite('preset', path, name, {format: presetRow.querySelector('.format-badge')?.textContent})
                    };
                })()],
                {icon: '&#128221;', label: appFmt('menu.add_note'), action: () => showNoteEditor(path, name)},
                ...quickTagItems(path, name),
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Table column headers ──
        const th = e.target.closest('th[data-action]');
        if (th) {
            const action = th.dataset.action;
            const key = th.dataset.key;
            const items = [
                {
                    icon: '&#9650;', label: appFmt('menu.sort_ascending'), action: () => {
                        if (action === 'sortAudio') {
                            sortAudio(key, true);
                        } else if (action === 'sortDaw') {
                            sortDaw(key, true);
                        } else if (action === 'sortPreset') {
                            sortPreset(key, true);
                        } else if (action === 'sortPdf') {
                            sortPdf(key, true);
                        } else if (action === 'sortMidi') {
                            sortMidi(key, true);
                        } else if (action === 'sortVideo') {
                            sortVideo(key, true);
                        }
                    }
                },
                {
                    icon: '&#9660;', label: appFmt('menu.sort_descending'), action: () => {
                        if (action === 'sortAudio') {
                            sortAudio(key, false);
                        } else if (action === 'sortDaw') {
                            sortDaw(key, false);
                        } else if (action === 'sortPreset') {
                            sortPreset(key, false);
                        } else if (action === 'sortPdf') {
                            sortPdf(key, false);
                        } else if (action === 'sortMidi') {
                            sortMidi(key, false);
                        } else if (action === 'sortVideo') {
                            sortVideo(key, false);
                        }
                    }
                },
                '---',
                {icon: '&#8596;', label: appFmt('menu.reset_columns'), action: () => settingResetColumns()},
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Search boxes ──
        const searchBox = e.target.closest('.search-box');
        if (searchBox) {
            const input = searchBox.querySelector('input');
            const regexBtn = searchBox.querySelector('.btn-regex');
            if (input) {
                const hasText = input.value.length > 0;
                const isRegex = regexBtn?.classList.contains('active');
                const items = [
                    {
                        icon: '&#10005;', label: appFmt('menu.clear_search'), action: () => {
                            input.value = '';
                            input.dispatchEvent(new Event('input', {bubbles: true}));
                        }, disabled: !hasText
                    },
                    {
                        icon: '&#128203;', label: appFmt('menu.paste_and_search'), action: async () => {
                            try {
                                const text = await navigator.clipboard.readText();
                                input.value = text;
                                input.dispatchEvent(new Event('input', {bubbles: true}));
                            } catch (e) {
                                if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
                            }
                        }
                    },
                    '---',
                    {
                        icon: '.*',
                        label: isRegex ? appFmt('menu.switch_to_fuzzy') : appFmt('menu.switch_to_regex'),
                        action: () => regexBtn && toggleRegex(regexBtn)
                    },
                ];
                showContextMenu(e, items);
                return;
            }
        }

        // ── Multi-select format filters (replaces `.filter-select` in the DOM) ──
        const multiFilter = e.target.closest('.multi-filter');
        if (multiFilter && multiFilter._select) {
            const selectEl = multiFilter._select;
            const items = [
                {
                    icon: '&#8635;',
                    label: catalogFmt('menu.reset_to_all'),
                    action: () => {
                        multiFilter._selected.clear();
                        const dropdown = multiFilter.querySelector('.multi-filter-dropdown');
                        if (dropdown) {
                            dropdown.querySelectorAll('input[data-value]').forEach((c) => {
                                c.checked = c.dataset.value === 'all';
                            });
                        }
                        const allLabel = selectEl.options[0]?.text || '';
                        if (typeof updateMultiFilterLabel === 'function') {
                            updateMultiFilterLabel(multiFilter, allLabel);
                        }
                        if (typeof syncMultiToSelect === 'function') {
                            syncMultiToSelect(multiFilter);
                        }
                        try {
                            selectEl.dispatchEvent(new Event('change', {bubbles: true}));
                        } catch (_) {
                        }
                        if (typeof triggerFilter === 'function') {
                            triggerFilter(multiFilter._action);
                        }
                        if (typeof saveAllFilterStates === 'function') saveAllFilterStates();
                    },
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Filter dropdowns ──
        const filterSelect = e.target.closest('.filter-select');
        if (filterSelect) {
            const items = [
                {
                    icon: '&#8635;', label: catalogFmt('menu.reset_to_all'), action: () => {
                        filterSelect.value = 'all';
                        filterSelect.dispatchEvent(new Event('change', {bubbles: true}));
                    }
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Toolbar areas ──
        const toolbar = e.target.closest('.audio-toolbar');
        if (toolbar) {
            const tab = toolbar.closest('.tab-content');
            const tabId = tab?.id;
            const items = [];
            if (tabId === 'tabPlugins') {
                items.push({icon: '&#8635;', label: appFmt('menu.scan_plugins'), action: () => scanPlugins()});
                items.push({
                    icon: '&#9889;',
                    label: appFmt('menu.check_updates'),
                    action: () => checkUpdates(),
                    disabled: allPlugins.length === 0
                });
                items.push('---');
                items.push({
                    icon: '&#8615;',
                    label: appFmt('menu.export_plugins'),
                    ...shortcutTip('exportTab'), action: () => {
                        if (typeof runExport === 'function') runExport(exportPlugins); else if (typeof exportPlugins === 'function') exportPlugins();
                    },
                    disabled: (typeof getPluginExportableCount === 'function' ? getPluginExportableCount() : allPlugins.length) === 0,
                });
                items.push({icon: '&#8613;', label: appFmt('menu.import_plugins'), ...shortcutTip('importTab'), action: () => importPlugins()});
            } else if (tabId === 'tabSamples') {
                items.push({icon: '&#127925;', label: appFmt('menu.scan_samples'), action: () => scanAudioSamples()});
                items.push('---');
                items.push({
                    icon: '&#8615;',
                    label: appFmt('menu.export_samples'),
                    ...shortcutTip('exportTab'), action: () => (typeof runExport === 'function' ? runExport(exportAudio) : exportAudio()),
                    disabled: Math.max(audioTotalCount || 0, allAudioSamples.length) === 0
                });
                items.push({icon: '&#8613;', label: appFmt('menu.import_samples'), ...shortcutTip('importTab'), action: () => importAudio()});
            } else if (tabId === 'tabDaw') {
                items.push({icon: '&#127911;', label: appFmt('menu.scan_daw'), action: () => scanDawProjects()});
                items.push('---');
                items.push({
                    icon: '&#8615;',
                    label: appFmt('menu.export_projects'),
                    ...shortcutTip('exportTab'), action: () => (typeof runExport === 'function' ? runExport(exportDaw) : exportDaw()),
                    disabled: Math.max(_dawTotalCount || 0, allDawProjects.length) === 0
                });
                items.push({icon: '&#8613;', label: appFmt('menu.import_projects_short'), ...shortcutTip('importTab'), action: () => importDaw()});
            } else if (tabId === 'tabPresets') {
                items.push({icon: '&#127924;', label: appFmt('menu.scan_presets'), action: () => scanPresets()});
                items.push('---');
                items.push({
                    icon: '&#8615;',
                    label: appFmt('menu.export_presets'),
                    ...shortcutTip('exportTab'), action: () => {
                        if (typeof runExport === 'function') runExport(exportPresets); else if (typeof exportPresets === 'function') exportPresets();
                    },
                    disabled: Math.max(_presetTotalCount || 0, allPresets.length) === 0,
                });
                items.push({icon: '&#8613;', label: appFmt('menu.import_presets'), ...shortcutTip('importTab'), action: () => importPresets()});
            } else if (tabId === 'tabMidi') {
                items.push({
                    icon: '&#127929;', label: appFmt('menu.scan_midi'), action: () => {
                        if (typeof scanMidi === 'function') scanMidi();
                    }
                });
                items.push('---');
                items.push({
                    icon: '&#8615;',
                    label: appFmt('menu.export_midi_files'),
                    ...shortcutTip('exportTab'), action: () => {
                        if (typeof exportMidi === 'function' && typeof runExport === 'function') runExport(exportMidi); else if (typeof exportMidi === 'function') exportMidi();
                    },
                    disabled: Math.max(_midiTotalCount || 0, typeof allMidiFiles !== 'undefined' ? allMidiFiles.length : 0) === 0
                });
                items.push({
                    icon: '&#8613;', label: appFmt('menu.import_midi_list'), ...shortcutTip('importTab'), action: () => {
                        if (typeof importAudio === 'function') importAudio();
                    }
                });
            } else if (tabId === 'tabPdf') {
                items.push({
                    icon: '&#8635;', label: appFmt('menu.scan_pdf'), action: () => {
                        if (typeof scanPdfs === 'function') scanPdfs();
                    }
                });
                items.push('---');
                items.push({
                    icon: '&#8615;',
                    label: appFmt('menu.export_pdfs'),
                    ...shortcutTip('exportTab'), action: () => {
                        if (typeof exportPdfs === 'function' && typeof runExport === 'function') runExport(exportPdfs); else if (typeof exportPdfs === 'function') exportPdfs();
                    },
                    disabled: Math.max(_pdfTotalCount || 0, typeof allPdfs !== 'undefined' ? allPdfs.length : 0) === 0
                });
                items.push({
                    icon: '&#8613;', label: appFmt('menu.import_pdfs'), ...shortcutTip('importTab'), action: () => {
                        if (typeof importPdfs === 'function') importPdfs();
                    }
                });
            } else if (tabId === 'tabVideos') {
                items.push({
                    icon: '&#127909;', label: appFmt('menu.scan_videos'), action: () => {
                        if (typeof scanVideos === 'function') scanVideos();
                    }
                });
                items.push('---');
                items.push({
                    icon: '&#8615;',
                    label: appFmt('menu.export_videos'),
                    ...shortcutTip('exportTab'), action: () => {
                        if (typeof exportVideos === 'function' && typeof runExport === 'function') runExport(exportVideos); else if (typeof exportVideos === 'function') exportVideos();
                    },
                    disabled: Math.max(typeof _videoTotalCount !== 'undefined' ? _videoTotalCount || 0 : 0, typeof allVideos !== 'undefined' ? allVideos.length : 0) === 0
                });
                items.push({
                    icon: '&#8613;', label: appFmt('menu.import_videos'), ...shortcutTip('importTab'), action: () => {
                        if (typeof importVideos === 'function') importVideos();
                    }
                });
            }
            if (items.length) {
                items.push('---');
                items.push({
                    icon: '&#128270;',
                    label: appFmt('menu.find_duplicates'),
                    ...shortcutTip('findDuplicates'), action: () => showDuplicateReport()
                });
                showContextMenu(e, items);
                return;
            }
        }

        // ── Stats bar ──
        const statsBar = e.target.closest('.stats-bar');
        if (statsBar) {
            const statsText = [...statsBar.querySelectorAll('.stat')].map(s => s.textContent.trim()).join(' | ');
            const items = [
                {
                    icon: '&#128203;',
                    label: catalogFmt('menu.copy_stats'), ..._noEcho, ...shortcutTip('copyPath'),
                    action: () => copyToClipboard(statsText)
                },
                '---',
                {icon: '&#9889;', label: appFmt('menu.scan_all'), ...shortcutTip('scanAll'), action: () => scanAll()},
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Header area / logo ──
        const header = e.target.closest('.header');
        if (header) {
            const headerInfo = e.target.closest('.header-info');
            if (headerInfo) {
                const statsText = [...headerInfo.querySelectorAll('.header-info-item')].map(s => s.textContent.trim()).join(' | ');
                const items = [
                    {
                        icon: '&#128203;',
                        label: catalogFmt('menu.copy_process_stats'), ..._noEcho, ...shortcutTip('copyPath'),
                        action: () => copyToClipboard(statsText)
                    },
                ];
                showContextMenu(e, items);
                return;
            }
            const items = [
                {
                    icon: '&#128202;', label: appFmt('menu.heatmap_dashboard'), ...shortcutTip('heatmapDash'), action: () => {
                        if (typeof showHeatmapDashboard === 'function') void showHeatmapDashboard();
                    }
                },
                {
                    icon: '&#128200;', label: appFmt('menu.dep_graph'), ...shortcutTip('depGraph'), action: () => {
                        if (typeof showDepGraph === 'function') showDepGraph();
                    }
                },
                '---',
                {
                    icon: '&#127760;',
                    label: appFmt('menu.open_github_repository'),
                    action: () => openUpdate('https://github.com/MenkeTechnologies/Audio-Haxor')
                },
                {icon: '&#9881;', label: appFmt('menu.tab_settings'), ...shortcutTip('openPrefs'), action: () => switchTab('settings')},
                '---',
                {icon: '&#9889;', label: appFmt('menu.scan_all'), ...shortcutTip('scanAll'), action: () => scanAll()},
            ];
            showContextMenu(e, items);
            return;
        }

        // ── History entries ──
        const historyRow = e.target.closest('.history-item');
        if (historyRow) {
            const id = historyRow.dataset.id;
            const type = historyRow.dataset.type;
            if (id) {
                const items = [
                    {icon: '&#128269;', label: appFmt('menu.view_details'), action: () => selectScan(id, type)},
                    {
                        icon: '&#128465;', label: appFmt('menu.delete_entry'), ...shortcutTip('deleteItem'), action: () => {
                            if (type === 'audio') deleteAudioScanEntry(id);
                            else if (type === 'daw') deleteDawScanEntry(id);
                            else if (type === 'preset') deletePresetScanEntry(id);
                            else if (type === 'pdf') deletePdfScanEntry(id);
                            else if (type === 'midi') deleteMidiScanEntry(id);
                            else deleteScanEntry(id);
                        }
                    },
                ];
                showContextMenu(e, items);
                return;
            }
        }

        // ── History tab (empty area) ──
        const historyTab = e.target.closest('#tabHistory');
        if (historyTab) {
            const items = [
                {icon: '&#128465;', label: appFmt('menu.clear_history'), ...shortcutTip('clearPlayHistory'), action: () => settingClearAllHistory()},
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Audio Engine tab — output diagnostic graphs (same baseline as Visualizer tiles: export PNG, copy label, freeze)
        const aeDiagCell =
            typeof e.target.closest === 'function' && e.target.closest('#tabAudioEngine')
                ? e.target.closest('.ae-graph-cell')
                : null;
        if (aeDiagCell) {
            const cEl = aeDiagCell.querySelector('canvas.ae-graph-canvas');
            const label =
                aeDiagCell.querySelector('.ae-graph-label')?.textContent?.trim()
                || cEl?.getAttribute('aria-label')
                || (cEl?.id ? cEl.id : appFmt('ui.ae.graphs_heading'));
            const cid = cEl && cEl.id ? cEl.id : '';
            const gf =
                typeof window.aeCanvasIdToGraphFreezeId === 'function' ? window.aeCanvasIdToGraphFreezeId(cid) : null;
            const animOn = !(gf && typeof window.isGraphFrozen === 'function' && window.isGraphFrozen(gf));
            const items = [
                {
                    icon: '&#128247;',
                    label: appFmt('menu.export_snapshot_png'),
                    action: () => {
                        if (!cEl || typeof exportCanvasSnapshotPng !== 'function') return;
                        void exportCanvasSnapshotPng(cEl, label);
                    },
                    disabled: !cEl,
                },
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_tile_name'),
                    ..._noEcho,
                    ...shortcutTip('copyPath'),
                    action: () => typeof copyToClipboard === 'function' && copyToClipboard(label),
                },
                '---',
                {
                    icon: animOn ? '&#10003;' : '&#9634;',
                    label: animOn ? appFmt('menu.viz_graph_freeze') : appFmt('menu.viz_graph_unfreeze'),
                    action: () => {
                        if (gf && typeof window.toggleGraphFrozen === 'function') window.toggleGraphFrozen(gf);
                    },
                    ..._noEcho,
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Audio Engine tab — parametric EQ canvas / resize handle (`ae:eq`)
        const onAeEqFft = e.target.closest('#aeEqCanvas, #aeEqCanvasWrap, #aeEqCanvasResizeHandle');
        if (onAeEqFft && e.target.closest('#tabAudioEngine')) {
            const G = typeof window.GRAPH_FREEZE_ID !== 'undefined' ? window.GRAPH_FREEZE_ID : null;
            const gf = G && G.AE_EQ ? G.AE_EQ : 'ae:eq';
            const animOn = !(typeof window.isGraphFrozen === 'function' && window.isGraphFrozen(gf));
            const eqCanvas = document.getElementById('aeEqCanvas');
            const eqLabel = appFmt('ui.ae.eq_rack_label');
            const items = [
                {
                    icon: '&#128247;',
                    label: appFmt('menu.export_snapshot_png'),
                    action: () => {
                        if (!eqCanvas || typeof exportCanvasSnapshotPng !== 'function') return;
                        void exportCanvasSnapshotPng(eqCanvas, eqLabel);
                    },
                    disabled: !eqCanvas,
                },
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_tile_name'),
                    ..._noEcho,
                    ...shortcutTip('copyPath'),
                    action: () => typeof copyToClipboard === 'function' && copyToClipboard(eqLabel),
                },
                '---',
                {
                    icon: animOn ? '&#10003;' : '&#9634;',
                    label: animOn ? appFmt('menu.viz_graph_freeze') : appFmt('menu.viz_graph_unfreeze'),
                    action: () => {
                        if (typeof window.toggleGraphFrozen === 'function') window.toggleGraphFrozen(gf);
                    },
                    ..._noEcho,
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Floating player ──
        const player = e.target.closest('#audioNowPlaying');
        if (player && player.classList.contains('active')) {
            // Smart playlist rows (`smart-playlists.js` listener runs first) — do not replace that menu.
            if (e.target.closest('.sp-item')) return;
            const isPlaying = audioPlayerPath && (typeof isAudioPlaying === 'function' ? isAudioPlaying() : !audioPlayer.paused);
            const isExpanded = player.classList.contains('expanded');
            const items = [];
            const onMiniFft =
                typeof e.target.closest === 'function' &&
                (e.target.closest('#npFftCanvas') || e.target.closest('#npVisualizer'));
            const onNpEqFft =
                typeof e.target.closest === 'function' &&
                e.target.closest('#npEqCanvas, #npEqCanvasWrap, #npEqCanvasResizeHandle');
            if (audioPlayerPath) {
                items.push({
                    icon: isPlaying ? '&#9646;&#9646;' : '&#9654;',
                    label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'), ..._noEcho, ...shortcutTip('playPause'),
                    action: () => toggleAudioPlayback()
                });
                items.push({
                    icon: '&#8634;',
                    label: audioLooping ? appFmt('menu.disable_loop') : appFmt('menu.enable_loop'), ..._noEcho, ...shortcutTip('toggleLoop'),
                    action: () => toggleAudioLoop()
                });
            }
            if (onMiniFft) {
                const G = typeof window.GRAPH_FREEZE_ID !== 'undefined' ? window.GRAPH_FREEZE_ID : null;
                const gf = G && G.NP_FFT ? G.NP_FFT : 'np:fft';
                const animOn = !(typeof window.isGraphFrozen === 'function' && window.isGraphFrozen(gf));
                const fftCanvas = document.getElementById('npFftCanvas');
                items.push({
                    icon: '&#128247;',
                    label: appFmt('menu.export_snapshot_png'),
                    action: () => {
                        if (fftCanvas && typeof exportCanvasSnapshotPng === 'function') {
                            void exportCanvasSnapshotPng(fftCanvas, 'FFT Spectrum');
                        }
                    },
                    disabled: !fftCanvas,
                });
                items.push({
                    icon: animOn ? '&#10003;' : '&#9634;',
                    label: animOn ? appFmt('menu.viz_graph_freeze') : appFmt('menu.viz_graph_unfreeze'),
                    action: () => {
                        if (typeof window.toggleGraphFrozen === 'function') window.toggleGraphFrozen(gf);
                    },
                    ..._noEcho,
                });
                items.push('---');
            }
            if (onNpEqFft) {
                const G = typeof window.GRAPH_FREEZE_ID !== 'undefined' ? window.GRAPH_FREEZE_ID : null;
                const gf = G && G.NP_EQ ? G.NP_EQ : 'np:eq';
                const animOn = !(typeof window.isGraphFrozen === 'function' && window.isGraphFrozen(gf));
                const eqCanvas = document.getElementById('npEqCanvas');
                items.push({
                    icon: '&#128247;',
                    label: appFmt('menu.export_snapshot_png'),
                    action: () => {
                        if (eqCanvas && typeof exportCanvasSnapshotPng === 'function') {
                            void exportCanvasSnapshotPng(eqCanvas, 'Parametric EQ');
                        }
                    },
                    disabled: !eqCanvas,
                });
                items.push({
                    icon: animOn ? '&#10003;' : '&#9634;',
                    label: animOn ? appFmt('menu.viz_graph_freeze') : appFmt('menu.viz_graph_unfreeze'),
                    action: () => {
                        if (typeof window.toggleGraphFrozen === 'function') window.toggleGraphFrozen(gf);
                    },
                    ..._noEcho,
                });
                items.push('---');
            }
            if (audioPlayerPath) {
                items.push({
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => openAudioFolder(audioPlayerPath)
                });
                items.push('---');
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'),
                    action: () => copyToClipboard(audioPlayerPath)
                });
                items.push('---');
            }
            items.push({
                icon: isExpanded ? '&#9660;' : '&#9650;',
                label: isExpanded ? appFmt('menu.player_collapse') : appFmt('menu.player_expand'), ..._noEcho, ...shortcutTip('togglePlayerExpand'),
                action: () => togglePlayerExpanded()
            });
            items.push({icon: '&#9868;', label: appFmt('menu.hide_player'), ...shortcutTip('togglePlayer'), action: () => hidePlayer()});
            items.push({
                icon: '&#10005;',
                label: appFmt('menu.stop_and_close'), ..._noEcho,
                action: () => stopAudioPlayback()
            });
            showContextMenu(e, items);
            return;
        }

        // ── Favorite items ──
        const favItem = e.target.closest('.fav-item');
        if (favItem) {
            const path = favItem.dataset.path || '';
            const name = favItem.querySelector('.fav-name')?.textContent?.trim() || '';
            const type = favItem.dataset.type || '';
            const items = [];

            if (type === 'sample') {
                const isPlaying = typeof audioPlayerPath !== 'undefined' && audioPlayerPath === path && (typeof isAudioPlaying === 'function' ? isAudioPlaying() : !audioPlayer.paused);
                items.push({
                    icon: isPlaying ? '&#9646;&#9646;' : '&#9654;',
                    label: isPlaying ? appFmt('menu.pause') : appFmt('menu.play'), ..._noEcho, ...shortcutTip('playPause'),
                    action: () => previewAudio(path)
                });
                items.push({
                    icon: '&#8634;',
                    label: appFmt('menu.loop'), ..._noEcho, ...shortcutTip('toggleLoop'),
                    action: () => toggleRowLoop(path, new MouseEvent('click'))
                });
                items.push('---');
                items.push({
                    icon: '&#127926;',
                    label: appFmt('menu.open_in_music'), ..._noEcho,
                    action: () => openWithApp(path, 'Music')
                });
                items.push({
                    icon: '&#127911;',
                    label: appFmt('menu.open_in_quicktime'), ..._noEcho,
                    action: () => openWithApp(path, 'QuickTime Player')
                });
                items.push({
                    icon: '&#127908;',
                    label: appFmt('menu.open_audacity'), ..._noEcho,
                    action: () => openWithApp(path, 'Audacity')
                });
                items.push('---');
            } else if (type === 'daw') {
                const daw = favItem.querySelector('.format-badge')?.textContent || 'DAW';
                items.push({
                    icon: '&#9654;', label: appFmt('menu.open_in_daw', {daw}), ..._noEcho, action: () => {
                        showToast(toastFmt('toast.opening_in_daw', {name, daw}));
                        window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.daw_not_installed', {
                            daw,
                            err
                        }), 4000, 'error'));
                    }
                });
                items.push('---');
            } else if (type === 'plugin') {
                const plugin = typeof allPlugins !== 'undefined' && findByPath(allPlugins, path);
                const kvrUrl = plugin ? (plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer)) : buildKvrUrl(name, '');
                items.push({
                    icon: '&#127760;',
                    label: appFmt('menu.open_kvr'), ..._noEcho,
                    action: () => window.vstUpdater.openUpdate(kvrUrl)
                });
                if (typeof findProjectsUsingPlugin === 'function') {
                    items.push({
                        icon: '&#9889;', label: appFmt('menu.find_projects_using'), action: () => {
                            const projects = findProjectsUsingPlugin(name);
                            showReverseXrefModal(name, projects);
                        }
                    });
                }
                items.push('---');
            } else if (type === 'pdf') {
                items.push({
                    icon: '&#9889;',
                    label: appFmt('menu.open_with_default_app'), ..._noEcho,
                    action: () => window.vstUpdater.openFileDefault(path).catch(err => showToast(toastFmt('toast.failed_open_file', {err: err.message || err}), 4000, 'error'))
                });
                items.push({
                    icon: '&#128195;',
                    label: appFmt('menu.open_in_preview'), ..._noEcho,
                    action: () => openWithApp(path, 'Preview')
                });
                items.push({
                    icon: '&#128196;',
                    label: appFmt('menu.open_in_adobe_acrobat'), ..._noEcho,
                    action: () => openWithApp(path, 'Adobe Acrobat')
                });
                items.push('---');
            }

            items.push({
                icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'), action: () => {
                    if (type === 'sample') openAudioFolder(path);
                    else if (type === 'daw') openDawFolder(path);
                    else if (type === 'preset') openPresetFolder(path);
                    else if (type === 'pdf') openPdfFile(path);
                    else openFolder(path);
                }
            });
            items.push({
                icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                    switchTab('files');
                    setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
                }
            });
            items.push('---');
            items.push({
                icon: '&#128203;',
                label: appFmt('menu.copy_name'), ..._noEcho,
                action: () => copyToClipboard(name)
            });
            items.push({
                icon: '&#128203;',
                label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'),
                action: () => copyToClipboard(path)
            });
            items.push('---');
            items.push({icon: '&#128221;', label: appFmt('menu.add_note'), ...shortcutTip('addNote'), action: () => showNoteEditor(path, name)});
            items.push(...quickTagItems(path, name));
            items.push('---');
            items.push({
                icon: '&#9734;', label: appFmt('menu.remove_from_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'), action: () => {
                    removeFavorite(path);
                    if (typeof renderFavorites === 'function') renderFavorites();
                }
            });

            showContextMenu(e, items);
            return;
        }

        // ── Note items ──
        const noteItem = e.target.closest('.note-item');
        if (noteItem) {
            const path = noteItem.dataset.path || '';
            const name = noteItem.querySelector('.note-item-name')?.textContent?.trim() || '';
            const items = [
                {icon: '&#128221;', label: appFmt('menu.edit_note'), ...shortcutTip('addNote'), action: () => showNoteEditor(path, name)},
                {icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'), action: () => openFolder(path)},
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
                    }
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name)},
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                '---',
                {
                    icon: '&#9733;',
                    label: isFavorite(path) ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                    action: () => isFavorite(path) ? removeFavorite(path) : addFavorite('item', path, name)
                },
                {
                    icon: '&#128465;', label: appFmt('menu.delete_note'), ...shortcutTip('deleteItem'), action: () => {
                        if (typeof deleteNote === 'function') {
                            deleteNote(path);
                            if (typeof renderNotesTab === 'function') renderNotesTab();
                        }
                    }
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Tag items ──
        const tagItem = e.target.closest('.tag-badge[data-tag]');
        if (tagItem) {
            const tag = tagItem.dataset.tag || '';
            const items = [
                {
                    icon: '&#128269;', label: appFmt('menu.filter_by_this_tag'), action: () => {
                        if (typeof setGlobalTag === 'function') setGlobalTag(tag);
                    }
                },
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_tag_name'), ..._noEcho,
                    action: () => copyToClipboard(tag)
                },
                '---',
                {
                    icon: '&#128465;', label: appFmt('menu.delete_tag_globally'), ...shortcutTip('deleteItem'), action: () => {
                        if (typeof deleteTagGlobally === 'function' && confirm(appFmt('confirm.delete_tag_globally', {tag}))) {
                            deleteTagGlobally(tag);
                        }
                    }
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Note cards ──
        const noteCard = e.target.closest('.note-card');
        if (noteCard) {
            const pathEl = noteCard.querySelector('.note-card-path');
            const nameEl = noteCard.querySelector('.note-card-name');
            const path = pathEl?.textContent?.trim() || '';
            const name = nameEl?.textContent?.trim() || '';
            const editBtn = noteCard.querySelector('[data-action-note="edit"]');
            const items = [
                {
                    icon: '&#128221;', label: appFmt('menu.edit_note'), ...shortcutTip('addNote'), action: () => {
                        if (editBtn) editBtn.click(); else if (typeof showNoteEditor === 'function') showNoteEditor(path, name);
                    }
                },
                {icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'), action: () => openFolder(path)},
                {
                    icon: '&#128194;', label: appFmt('menu.show_file_browser'), ..._noEcho, action: () => {
                        switchTab('files');
                        setTimeout(() => loadDirectory(path.replace(/\/[^/]+$/, '')), 200);
                    }
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name)},
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                '---',
                {
                    icon: '&#128465;', label: appFmt('menu.delete_note'), ...shortcutTip('deleteItem'), action: () => {
                        if (typeof deleteNote === 'function') {
                            deleteNote(path);
                            if (typeof renderNotesTab === 'function') renderNotesTab();
                        }
                    }
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Xref plugin items (plugins found in DAW projects) ──
        const xrefItem = e.target.closest('.xref-item[data-xref-plugin]');
        if (xrefItem) {
            const pluginName = xrefItem.dataset.xrefPlugin;
            const items = [
                {
                    icon: '&#128269;', label: appFmt('menu.find_in_plugins_tab'), action: () => {
                        switchTab('plugins');
                        const input = document.getElementById('searchInput');
                        if (input) {
                            input.value = pluginName;
                            input.dispatchEvent(new Event('input', {bubbles: true}));
                        }
                        showToast(toastFmt('toast.searching_plugins_for', {pluginName}));
                    }
                },
                {
                    icon: '&#128203;', label: appFmt('menu.copy_plugin_name'), ..._noEcho, action: () => {
                        navigator.clipboard.writeText(pluginName);
                        showToast(toastFmt('toast.copied_plugin_name', {pluginName}));
                    }
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Dep graph plugin rows ──
        const depRow = e.target.closest('.dep-plugin-row');
        if (depRow) {
            const name = depRow.querySelector('.dep-plugin-name')?.textContent?.trim() || '';
            const mfg = depRow.querySelector('.dep-plugin-mfg')?.textContent?.trim() || '';
            const items = [
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_plugin_name'), ..._noEcho,
                    action: () => copyToClipboard(name)
                },
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_manufacturer'), ..._noEcho,
                    action: () => copyToClipboard(mfg)
                },
            ];
            if (typeof findProjectsUsingPlugin === 'function') {
                items.push('---');
                items.push({
                    icon: '&#9889;', label: appFmt('menu.find_projects_using'), action: () => {
                        const projects = findProjectsUsingPlugin(name);
                        showReverseXrefModal(name, projects);
                    }
                });
            }
            const plugin = typeof allPlugins !== 'undefined' && allPlugins.find(p => p.name === name);
            if (plugin) {
                const kvrUrl = plugin.kvrUrl || buildKvrUrl(plugin.name, plugin.manufacturer);
                items.push({
                    icon: '&#127760;',
                    label: appFmt('menu.open_kvr'), ..._noEcho,
                    action: () => window.vstUpdater.openUpdate(kvrUrl)
                });
                items.push({
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => openFolder(plugin.path)
                });
            }
            showContextMenu(e, items);
            return;
        }

        // ── Dep graph project rows ──
        const depProject = e.target.closest('.dep-project-row');
        if (depProject) {
            const path = depProject.dataset.depProject || '';
            const name = depProject.querySelector('.dep-project-name')?.textContent?.trim() || '';
            const daw = depProject.querySelector('.format-badge')?.textContent?.trim() || '';
            const items = [
                {
                    icon: '&#9654;', label: appFmt('menu.open_in_daw', {daw: daw || 'DAW'}), ..._noEcho, action: () => {
                        showToast(toastFmt('toast.opening_name', {name}));
                        window.vstUpdater.openDawProject(path).catch(err => showToast(toastFmt('toast.failed_dash', {err}), 4000, 'error'));
                    }
                },
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => typeof openDawFolder === 'function' && openDawFolder(path)
                },
                '---',
                {icon: '&#128203;', label: appFmt('menu.copy_name'), ..._noEcho, action: () => copyToClipboard(name)},
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Dep graph orphaned plugin rows ──
        const depOrphan = e.target.closest('.dep-orphan');
        if (depOrphan) {
            const name = depOrphan.querySelector('.dep-plugin-name')?.textContent?.trim() || '';
            const path = depOrphan.getAttribute('title') || '';
            const items = [
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_plugin_name'), ..._noEcho,
                    action: () => copyToClipboard(name)
                },
                {icon: '&#128203;', label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'), action: () => copyToClipboard(path)},
                {
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => typeof openFolder === 'function' && openFolder(path)
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Tab buttons ──
        const tabBtn = e.target.closest('.tab-btn');
        if (tabBtn) {
            const tab = tabBtn.dataset.tab;
            const exportMap = {
                plugins: 'exportPlugins',
                samples: 'exportAudio',
                daw: 'exportDaw',
                presets: 'exportPresets',
                pdf: 'exportPdfs',
                midi: 'exportMidi'
            };
            const scanMap = {
                plugins: 'scanPlugins',
                samples: 'scanAudioSamples',
                daw: 'scanDawProjects',
                presets: 'scanPresets',
                pdf: 'scanPdfs',
                midi: 'scanMidi'
            };
            /** Tab bar rescan: same labels as Scan menu / toolbar (not generic "Rescan Tab Data"). */
            const scanLabelKeyByTab = {
                plugins: 'menu.scan_plugins',
                samples: 'menu.scan_samples',
                daw: 'menu.scan_daw',
                presets: 'menu.scan_presets',
                pdf: 'menu.scan_pdf',
                midi: 'menu.scan_midi',
            };
            const tabNameLabel = tabBtn.querySelector('[data-i18n]')?.textContent?.trim() || '';
            const switchToTabLabel = tabNameLabel
                ? appFmt('menu.switch_to_tab_named', {name: tabNameLabel})
                : appFmt('menu.switch_to_tab');
            const items = [
                {icon: '&#8635;', label: switchToTabLabel, action: () => switchTab(tab)},
                '---',
            ];
            const scanFn = scanMap[tab];
            if (scanFn && typeof window[scanFn] === 'function') {
                const scanKey = scanLabelKeyByTab[tab] || 'menu.rescan_tab_data';
                const scanLabel = catalogFmt(scanKey);
                items.push({icon: '&#9889;', label: scanLabel, action: () => window[scanFn]()});
            }
            const exportFn = exportMap[tab];
            if (exportFn && typeof window[exportFn] === 'function') {
                items.push({
                    icon: '&#8615;',
                    label: appFmt('menu.export_tab_data'),
                    ...shortcutTip('exportTab'), action: () => {
                        const fn = window[exportFn];
                        if (typeof runExport === 'function') runExport(fn);
                        else fn();
                    },
                });
            }
            if (scanFn || exportFn) items.push('---');
            items.push({icon: '&#8644;', label: appFmt('menu.reset_tabs'), action: () => settingResetTabOrder()});
            showContextMenu(e, items);
            return;
        }

        // ── Tab nav bar (empty area) ──
        const tabNav = e.target.closest('.tab-nav');
        if (tabNav) {
            const items = [
                {icon: '&#8644;', label: appFmt('menu.reset_tabs'), action: () => settingResetTabOrder()},
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Settings rows ──
        const settingsRow = e.target.closest('.settings-row');
        if (settingsRow) {
            const toggle = settingsRow.querySelector('.settings-toggle');
            const textarea = settingsRow.querySelector('.settings-textarea');
            const selectEl = settingsRow.querySelector('select');
            const rangeEl = settingsRow.querySelector('input[type="range"]');
            const rowTitle = settingsRow.querySelector('.settings-title')?.textContent?.trim() || '';
            const items = [];
            if (toggle) {
                const isOn = toggle.classList.contains('active');
                items.push({
                    icon: isOn ? '&#9711;' : '&#9679;',
                    label: isOn ? appFmt('menu.turn_off') : appFmt('menu.turn_on'),
                    action: () => toggle.click()
                });
            }
            if (textarea) {
                items.push({
                    icon: '&#10005;', label: appFmt('menu.clear'), ..._noEcho, action: () => {
                        textarea.value = '';
                    }
                });
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy'), ..._noEcho, ...shortcutTip('copyPath'),
                    action: () => copyToClipboard(textarea.value)
                });
            }
            if (selectEl) {
                const optLabel = selectEl.options[selectEl.selectedIndex]?.text || selectEl.value;
                items.push({
                    icon: '&#128203;',
                    label: rowTitle ? appFmt('menu.copy_field_label', {label: rowTitle}) : appFmt('menu.copy'), ..._noEcho,
                    action: () => copyToClipboard(optLabel)
                });
                items.push({
                    icon: '&#128269;', label: appFmt('menu.ctx_focus'), ..._noEcho, action: () => {
                        try {
                            selectEl.focus();
                        } catch (_) {
                        }
                    }
                });
            }
            if (rangeEl) {
                items.push({
                    icon: '&#8634;', label: appFmt('menu.ctx_reset_default'), action: () => {
                        rangeEl.value = rangeEl.defaultValue;
                        rangeEl.dispatchEvent(new Event('input', {bubbles: true}));
                        rangeEl.dispatchEvent(new Event('change', {bubbles: true}));
                    }
                });
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy_quoted_label_val', {
                        label: rowTitle || 'value',
                        val: rangeEl.value
                    }), ..._noEcho,
                    action: () => copyToClipboard(String(rangeEl.value))
                });
                items.push({
                    icon: '&#128269;', label: appFmt('menu.ctx_focus'), ..._noEcho, action: () => {
                        try {
                            rangeEl.focus();
                        } catch (_) {
                        }
                    }
                });
            }
            const extraBtns = settingsRow.querySelectorAll('.settings-control button:not(.settings-toggle)');
            for (const b of extraBtns) {
                const bl = b.textContent?.trim().replace(/\s+/g, ' ') || '';
                items.push({
                    icon: '&#9654;',
                    label: bl ? `${appFmt('menu.ctx_activate')}: ${bl}` : appFmt('menu.ctx_activate'),
                    action: () => b.click()
                });
            }
            if (rowTitle) {
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy_section_name'), ..._noEcho,
                    action: () => copyToClipboard(rowTitle)
                });
            }
            if (items.length === 0) {
                const flat = settingsRow.textContent.trim().replace(/\s+/g, ' ');
                if (flat) items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.ctx_copy_visible_text'), ..._noEcho,
                    action: () => copyToClipboard(flat.slice(0, 8000))
                });
            }
            showContextMenu(e, items);
            return;
        }

        // ── Settings container (empty area) ──
        const settingsContainer = e.target.closest('.settings-container');
        if (settingsContainer) {
            const items = [
                {icon: '&#8596;', label: appFmt('menu.reset_columns'), action: () => settingResetColumns()},
                {icon: '&#8644;', label: appFmt('menu.reset_tabs'), action: () => settingResetTabOrder()},
                {icon: '&#128465;', label: appFmt('menu.clear_history'), ...shortcutTip('clearPlayHistory'), action: () => settingClearAllHistory()},
                '---',
                {icon: '&#128206;', label: appFmt('menu.open_prefs_file'), action: () => openPrefsFile()},
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Directory breakdown rows ──
        const dirsRow = e.target.closest('#dirsList tr');
        if (dirsRow) {
            const dirPath = dirsRow.querySelector('td')?.textContent?.trim() || '';
            if (dirPath) {
                const items = [
                    {
                        icon: '&#128193;',
                        label: appFmt('menu.open_directory'), ..._noEcho,
                        action: () => openFolder(dirPath)
                    },
                    {
                        icon: '&#128203;',
                        label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'),
                        action: () => copyToClipboard(dirPath)
                    },
                ];
                showContextMenu(e, items);
                return;
            }
        }

        // ── Audio/DAW/Preset stats bars ──
        const audioStats = e.target.closest('.audio-stats');
        if (audioStats) {
            const statsText = audioStats.textContent.trim().replace(/\s+/g, ' ');
            const items = [
                {
                    icon: '&#128203;',
                    label: catalogFmt('menu.copy_stats'), ..._noEcho, ...shortcutTip('copyPath'),
                    action: () => copyToClipboard(statsText)
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── File browser breadcrumbs ──
        const crumb = e.target.closest('.file-crumb');
        if (crumb) {
            const crumbPath = crumb.dataset.fileNav || '';
            const items = [
                {
                    icon: '&#128193;',
                    label: appFmt('menu.open_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => typeof openFolder === 'function' && openFolder(crumbPath)
                },
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'),
                    action: () => copyToClipboard(crumbPath)
                },
                {
                    icon: '&#9733;',
                    label: appFmt('menu.bookmark_this_directory'),
                    action: () => typeof addFavDir === 'function' && addFavDir(crumbPath)
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── File browser rows ──
        const fileRow = e.target.closest('.file-row');
        if (fileRow && !e.target.closest('.fb-meta-panel')) {
            const path = fileRow.dataset.filePath;
            const isDir = fileRow.dataset.fileDir === 'true';
            const name = path.split('/').pop();
            const ext = name.split('.').pop().toLowerCase();
            const isAudio = !isDir && typeof AUDIO_EXTS !== 'undefined' && AUDIO_EXTS.includes(ext);
            const items = [];
            if (isAudio) {
                items.push({
                    icon: '&#9654;',
                    label: appFmt('menu.play'), ..._noEcho, ...shortcutTip('playPause'),
                    action: () => typeof previewAudio === 'function' && previewAudio(path)
                });
                items.push({
                    icon: '&#128269;', label: appFmt('menu.show_in_samples_tab'), ..._noEcho, action: async () => {
                        // If not in allAudioSamples, add it
                        if (typeof allAudioSamples !== 'undefined' && !findByPath(allAudioSamples, path)) {
                            try {
                                const meta = await window.vstUpdater.getAudioMetadata(path);
                                allAudioSamples.push({
                                    name: meta.fileName.replace(/\.[^.]+$/, ''),
                                    path: meta.fullPath,
                                    directory: meta.directory || path.replace(/\/[^/]+$/, ''),
                                    format: meta.format,
                                    size: meta.sizeBytes,
                                    sizeBytes: meta.sizeBytes,
                                    sizeFormatted: typeof formatAudioSize === 'function' ? formatAudioSize(meta.sizeBytes) : '',
                                    modified: meta.modified || '',
                                    duration: meta.duration || null,
                                    sampleRate: meta.sampleRate || null,
                                    channels: meta.channels || null,
                                    bitsPerSample: meta.bitsPerSample || null,
                                });
                            } catch (e) {
                                if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
                            }
                        }
                        switchTab('samples');
                        // Clear any existing search filter so the row is visible
                        const searchInput = document.getElementById('audioSearchInput');
                        if (searchInput && searchInput.value) {
                            searchInput.value = '';
                        }
                        if (typeof filterAudioSamples === 'function') filterAudioSamples();
                        // Scroll to and highlight the row
                        setTimeout(() => {
                            const row = document.querySelector(`#audioTableBody tr[data-audio-path="${CSS.escape(path)}"]`);
                            if (row) {
                                row.scrollIntoView({block: 'center', behavior: 'smooth'});
                                row.classList.add('row-playing');
                                setTimeout(() => row.classList.remove('row-playing'), 2000);
                            }
                        }, 100);
                    }
                });
                items.push({
                    icon: '&#128270;',
                    label: appFmt('menu.find_similar'),
                    ...shortcutTip('findSimilar'),
                    action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path)
                });
                items.push('---');
            }
            if (isDir) {
                items.push({
                    icon: '&#128193;',
                    label: appFmt('menu.open_directory'), ..._noEcho,
                    action: () => typeof loadDirectory === 'function' && loadDirectory(path)
                });
            }
            items.push({
                icon: '&#128193;', label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'), action: () => {
                    const dir = isDir ? path : path.replace(/\/[^/]+$/, '');
                    if (typeof openFolder === 'function') openFolder(dir);
                    else if (typeof openAudioFolder === 'function') openAudioFolder(path);
                }
            });
            items.push({
                icon: '&#128203;',
                label: appFmt('menu.copy_path'), ..._noEcho, ...shortcutTip('copyPath'),
                action: () => copyToClipboard(path)
            });
            items.push({
                icon: '&#128203;',
                label: appFmt('menu.copy_name'), ..._noEcho,
                action: () => copyToClipboard(name)
            });
            items.push('---');
            if (typeof isFavorite === 'function') {
                const fav = isFavorite(path);
                const favType = isDir ? 'folder' : 'file';
                items.push({
                    icon: fav ? '&#9734;' : '&#9733;',
                    label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                    action: () => {
                        fav ? removeFavorite(path) : addFavorite(favType, path, name);
                        if (typeof renderFileList === 'function') renderFileList();
                    }
                });
            }
            items.push({
                icon: '&#128221;', label: appFmt('menu.add_note_tags'), action: () => {
                    if (typeof showNoteEditor === 'function') showNoteEditor(path, name);
                }
            });
            if (isAudio) {
                items.push('---');
                const ap = prefs.getItem('autoplayNext') !== 'off';
                items.push({
                    icon: ap ? '&#9209;' : '&#9654;',
                    label: ap ? appFmt('menu.disable_autoplay_next') : appFmt('menu.enable_autoplay_next'), ..._noEcho,
                    action: () => {
                        prefs.setItem('autoplayNext', ap ? 'off' : 'on');
                        if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
                        showToast(ap ? toastFmt('toast.autoplay_next_disabled') : toastFmt('toast.autoplay_next_enabled'));
                    }
                });
                items.push('---');
                items.push(...ctxMenuVideoAudioRouteItems());
            }
            showContextMenu(e, items);
            return;
        }

        // ── Disk usage segments ──
        const diskSeg = e.target.closest('.disk-segment, .disk-legend-item');
        if (diskSeg) {
            const label = diskSeg.getAttribute('title') || diskSeg.textContent.trim();
            const items = [
                {icon: '&#128203;', label: appFmt('menu.copy'), ..._noEcho, action: () => copyToClipboard(label)},
            ];
            showContextMenu(e, items);
            return;
        }

        // ── EQ/Gain/Pan sliders ──
        const eqSlider = e.target.closest('.eq-slider, .volume-slider');
        if (eqSlider) {
            const items = [
                {
                    icon: '&#8634;', label: appFmt('menu.reset_eq_default'), action: () => {
                        if (typeof resetEq === 'function') resetEq();
                    }
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Expanded metadata panel ──
        const metaPanel = e.target.closest('.audio-meta-panel');
        if (metaPanel && !e.target.closest('.meta-waveform')) {
            const metaRow = metaPanel.closest('#audioMetaRow');
            const path = metaRow?.getAttribute('data-meta-path') || '';
            const metaValue = e.target.closest('.meta-item');
            const items = [];
            // Copy the specific value if clicking a meta-item
            if (metaValue) {
                const label = metaValue.querySelector('.meta-label')?.textContent || '';
                const val = metaValue.querySelector('.meta-value')?.textContent || '';
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy_field_label', {label}), ..._noEcho,
                    action: () => copyToClipboard(val)
                });
            }
            if (path) {
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy_file_path'), ..._noEcho,
                    action: () => copyToClipboard(path)
                });
                items.push({
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => typeof openAudioFolder === 'function' && openAudioFolder(path)
                });
                items.push('---');
                items.push({
                    icon: '&#9654;',
                    label: appFmt('menu.play'), ..._noEcho,
                    action: () => typeof previewAudio === 'function' && previewAudio(path)
                });
                if (typeof isFavorite === 'function') {
                    const fav = isFavorite(path);
                    const name = metaPanel.querySelector('.meta-value')?.textContent || '';
                    items.push({
                        icon: fav ? '&#9734;' : '&#9733;',
                        label: fav ? appFmt('menu.remove_from_favorites') : appFmt('menu.add_to_favorites'), ..._noEcho, ...shortcutTip('toggleFavorite'),
                        action: () => fav ? removeFavorite(path) : addFavorite('sample', path, name)
                    });
                }
                items.push({
                    icon: '&#128221;', label: appFmt('menu.add_note'), ...shortcutTip('addNote'), action: () => {
                        const name = metaPanel.querySelector('.meta-value')?.textContent || '';
                        typeof showNoteEditor === 'function' && showNoteEditor(path, name);
                    }
                });
                items.push({
                    icon: '&#128270;',
                    label: appFmt('menu.find_similar'),
                    ...shortcutTip('findSimilar'),
                    action: () => typeof findSimilarSamples === 'function' && findSimilarSamples(path)
                });
                items.push('---');
                items.push({
                    icon: '&#10005;', label: appFmt('menu.close_panel'), action: () => {
                        const mr = document.getElementById('audioMetaRow');
                        if (mr) mr.remove();
                        expandedMetaPath = null;
                    }
                });
            }
            if (items.length > 0) {
                showContextMenu(e, items);
                return;
            }
        }

        // ── Waveform ──
        const waveform = e.target.closest('.now-playing-waveform, .meta-waveform');
        if (waveform) {
            const items = [];
            if (typeof audioPlayerPath !== 'undefined' && audioPlayerPath) {
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy_file_path'), ..._noEcho, ...shortcutTip('copyPath'),
                    action: () => copyToClipboard(audioPlayerPath)
                });
                items.push({
                    icon: '&#128193;',
                    label: appFmt('menu.reveal_in_finder'), ..._noEcho, ...shortcutTip('revealFile'),
                    action: () => typeof openAudioFolder === 'function' && openAudioFolder(audioPlayerPath)
                });
            }
            if (items.length > 0) {
                showContextMenu(e, items);
                return;
            }
        }

        // ── Shortcut keys ──
        const shortcutKey = e.target.closest('.shortcut-key');
        if (shortcutKey) {
            const scId = shortcutKey.dataset.shortcutId;
            let currentBindingHint = '';
            if (scId && typeof getShortcuts === 'function' && typeof formatKey === 'function') {
                const sc = getShortcuts()[scId];
                if (sc) currentBindingHint = formatKey(sc);
            }
            const rebindItem = {icon: '&#9881;', label: appFmt('menu.rebind_shortcut'), action: () => shortcutKey.click()};
            if (currentBindingHint) rebindItem.shortcutHint = currentBindingHint;
            const items = [
                rebindItem,
                {
                    icon: '&#8634;',
                    label: appFmt('menu.reset_all_shortcuts'),
                    action: () => typeof resetShortcuts === 'function' && resetShortcuts()
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Color scheme buttons ──
        const schemeBtn = e.target.closest('.scheme-btn');
        if (schemeBtn) {
            const scheme = schemeBtn.dataset.scheme;
            const items = [
                {
                    icon: '&#127912;',
                    label: appFmt('menu.apply_scheme', {scheme: scheme || 'scheme'}), ..._noEcho,
                    action: () => schemeBtn.click()
                },
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_scheme_name'), ..._noEcho,
                    action: () => copyToClipboard(scheme || '')
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Progress bars ──
        const progressBar = e.target.closest('.audio-progress-bar, .global-progress, .progress-bar');
        if (progressBar) {
            const items = [
                {
                    icon: '&#9632;',
                    label: appFmt('menu.stop_all_scans'),
                    ...shortcutTip('stopAll'), action: () => typeof stopAll === 'function' && stopAll()
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // Smart playlists section
        const spSection = e.target.closest('.np-smart-playlists-section');
        if (spSection && !e.target.closest('.sp-item')) {
            const items = [
                {
                    icon: '&#127926;',
                    label: appFmt('menu.new_smart_playlist'),
                    ...shortcutTip('newSmartPlaylist'),
                    action: () => typeof showSmartPlaylistEditor === 'function' && showSmartPlaylistEditor(null)
                },
                '---',
            ];
            if (typeof getSmartPlaylistPresets === 'function') {
                for (const preset of getSmartPlaylistPresets()) {
                    items.push({
                        icon: '&#127925;',
                        label: appFmt('menu.add_smart_playlist_named', {name: preset.name}),
                        ...shortcutTip('newSmartPlaylist'),
                        action: () => {
                            if (typeof createSmartPlaylist === 'function') {
                                const pl = createSmartPlaylist(preset.name, preset.rules);
                                pl.matchMode = preset.matchMode;
                                if (typeof saveSmartPlaylists === 'function') saveSmartPlaylists();
                                showToast(toastFmt('toast.created_preset', {name: preset.name}));
                            }
                        }
                    });
                }
            }
            showContextMenu(e, items);
            return;
        }

        // ── Similar panel ──
        const simPanel = e.target.closest('.similar-panel');
        if (simPanel && !e.target.closest('[data-similar-path]')) {
            const items = [
                {
                    icon: '&#9866;',
                    label: appFmt('menu.minimize'),
                    action: () => typeof minimizeSimilarPanel === 'function' && minimizeSimilarPanel()
                },
                {
                    icon: '&#10005;',
                    label: appFmt('menu.close'),
                    action: () => typeof closeSimilarPanel === 'function' && closeSimilarPanel()
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Heatmap dashboard ──
        const hmDash = e.target.closest('#heatmapDashModal');
        if (hmDash) {
            const card = e.target.closest('.hm-card');
            const barRow = e.target.closest('.hm-bar-row');
            const items = [];
            if (barRow) {
                const label = barRow.querySelector('.hm-bar-label')?.textContent || '';
                const val = barRow.querySelector('.hm-bar-val')?.textContent || '';
                items.push({
                    icon: '&#128203;',
                    label: appFmt('menu.copy_quoted_label_val', {label, val}), ..._noEcho,
                    action: () => copyToClipboard(`${label}: ${val}`)
                });
            }
            if (card) {
                const title = card.querySelector('.hm-card-title')?.textContent || '';
                items.push({
                    icon: '&#128203;', label: appFmt('menu.copy_tabular_title', {title}), ..._noEcho, action: () => {
                        const rows = [...card.querySelectorAll('.hm-bar-row')].map(r => {
                            const l = r.querySelector('.hm-bar-label')?.textContent || '';
                            const v = r.querySelector('.hm-bar-val')?.textContent || '';
                            return `${l}\t${v}`;
                        }).join('\n');
                        copyToClipboard(rows || title);
                    }
                });
            }
            items.push('---');
            items.push({
                icon: '&#8634;', label: appFmt('menu.refresh_dashboard'), action: () => {
                    if (typeof showHeatmapDashboard === 'function') void showHeatmapDashboard();
                }
            });
            items.push({
                icon: '&#10005;', label: appFmt('menu.close_dashboard'), action: () => {
                    if (typeof closeHeatmapDash === 'function') closeHeatmapDash();
                }
            });
            showContextMenu(e, items);
            return;
        }

        // ── Walker tiles ──
        const walkerTile = e.target.closest('.walker-tile');
        if (walkerTile) {
            const body = walkerTile.querySelector('.walker-tile-body');
            const dirs = body ? [...body.querySelectorAll('.walker-dir')].map(d => d.textContent).join('\n') : '';
            const title = walkerTile.querySelector('.walker-tile-title, h4, h3')?.textContent?.trim()
                || catalogFmt('menu.context_walker_fallback');
            const items = [
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_all_paths'), ..._noEcho,
                    action: () => copyToClipboard(dirs),
                    disabled: !dirs
                },
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_tile_title'), ..._noEcho,
                    action: () => copyToClipboard(title)
                },
                '---',
                {
                    icon: '&#10005;', label: appFmt('menu.clear_tile'), action: () => {
                        if (body) body.innerHTML = '';
                        showToast(toastFmt('toast.tile_cleared', {title}));
                    }
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Visualizer tiles ── (handled in visualizer.js — single menu with mode-specific items)

        // ── Settings sections ──
        const settingsSection = e.target.closest('.settings-section');
        if (settingsSection) {
            const heading = settingsSection.querySelector('.settings-heading')?.textContent?.trim()
                || catalogFmt('menu.context_settings_section_fallback');
            const persistSectionPaneOrder = () => {
                const pane = settingsSection.closest('.settings-container');
                if (!pane || typeof prefs === 'undefined') return;
                const key = pane.classList.contains('audio-engine-tab') ? 'audioEngineSectionOrder' : 'settingsSectionOrder';
                const sections = [...pane.querySelectorAll('.settings-section[data-section]')].map(s => s.dataset.section);
                prefs.setItem(key, sections);
            };
            const items = [
                {
                    icon: '&#128203;',
                    label: appFmt('menu.copy_section_name'), ..._noEcho,
                    action: () => typeof copyToClipboard === 'function' && copyToClipboard(heading)
                },
                {
                    icon: '&#9650;', label: appFmt('menu.move_up'), action: () => {
                        const prev = settingsSection.previousElementSibling;
                        if (prev && prev.classList.contains('settings-section')) {
                            settingsSection.parentNode.insertBefore(settingsSection, prev);
                            persistSectionPaneOrder();
                            showToast(toastFmt('toast.moved_heading_up', {heading}));
                        }
                    }
                },
                {
                    icon: '&#9660;', label: appFmt('menu.move_down'), action: () => {
                        const next = settingsSection.nextElementSibling;
                        if (next && next.classList.contains('settings-section')) {
                            next.parentNode.insertBefore(next, settingsSection);
                            persistSectionPaneOrder();
                            showToast(toastFmt('toast.moved_heading_down', {heading}));
                        }
                    }
                },
                '---',
                {
                    icon: '&#128065;',
                    label: settingsSection.classList.contains('collapsed') ? appFmt('menu.section_expand') : appFmt('menu.section_collapse'),
                    action: () => {
                        settingsSection.classList.toggle('collapsed');
                        const body = settingsSection.querySelectorAll('.settings-row');
                        body.forEach(r => r.style.display = settingsSection.classList.contains('collapsed') ? 'none' : '');
                    }
                },
            ];
            showContextMenu(e, items);
            return;
        }

        // ── Universal fallback (empty tab panels, chrome gaps, tag bar, dock overlay, etc.) ──
        const shell = e.target.closest('.app, .audio-now-playing, #dockOverlay, .dock-zone-overlay');
        if (shell) {
            if (e.target.closest('#ctxMenu')) return;
            if (e.target.closest('.viz-tile')) return;
            if (e.target.closest('.sp-item')) return;
            e.preventDefault();
            showContextMenu(e, buildFallbackShellContextMenu(e));
            return;
        }

    } catch (err) {
        console.error('Context menu error:', err, err.stack);
        showToast(toastFmt('toast.context_menu_error', {err: err.message || err}), 4000, 'error');
    }
});
