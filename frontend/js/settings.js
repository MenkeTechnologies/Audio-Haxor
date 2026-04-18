// ── Settings ──

/** Force CSS columns rebalance on the settings container.
 *  Dynamically-loaded panes (app info, caches, system info) change
 *  content height after the initial layout — CSS columns won't
 *  rebalance on their own. Toggling column-count triggers a reflow. */
function settingsReflow() {
    const c = document.querySelector('#tabSettings .settings-container');
    if (!c) return;
    requestAnimationFrame(() => {
        const cols = getComputedStyle(c).columnCount;
        c.style.columnCount = '1';
        void c.offsetHeight;
        c.style.columnCount = '';
    });
}

function _uiTheme(isLight) {
    if (typeof appFmt !== 'function') return isLight ? 'Light' : 'Dark';
    return isLight ? appFmt('ui.js.theme_light') : appFmt('ui.js.theme_dark');
}

function _uiToggle(on) {
    if (typeof appFmt !== 'function') return on ? 'On' : 'Off';
    return on ? appFmt('ui.js.toggle_on') : appFmt('ui.js.toggle_off');
}

// All CSS variable keys that color schemes control
const SCHEME_VAR_KEYS = [
    '--accent', '--accent-light', '--accent-glow',
    '--cyan', '--cyan-glow', '--cyan-dim',
    '--magenta', '--magenta-glow',
    '--green', '--green-bg',
    '--yellow', '--yellow-glow',
    '--orange', '--orange-bg',
    '--red',
    '--text', '--text-dim', '--text-muted',
    '--bg-primary', '--bg-secondary', '--bg-card', '--bg-hover',
    '--border', '--border-glow',
];
if (typeof window !== 'undefined') window.SCHEME_VAR_KEYS = SCHEME_VAR_KEYS;

const COLOR_SCHEMES = {
    cyberpunk: {
        label: 'Cyberpunk',
        desc: 'Hot pink + cyan neon (default)',
        vars: {
            '--accent': '#ff2a6d', '--accent-light': '#ff6b9d',
            '--accent-glow': 'rgba(255, 42, 109, 0.4)',
            '--cyan': '#05d9e8', '--cyan-glow': 'rgba(5, 217, 232, 0.4)',
            '--cyan-dim': 'rgba(5, 217, 232, 0.15)',
            '--magenta': '#d300c5', '--magenta-glow': 'rgba(211, 0, 197, 0.3)',
            '--green': '#39ff14', '--green-bg': 'rgba(57, 255, 20, 0.08)',
            '--yellow': '#f9f002', '--yellow-glow': 'rgba(249, 240, 2, 0.2)',
            '--orange': '#ff6b35', '--orange-bg': 'rgba(255, 107, 53, 0.1)',
            '--red': '#ff073a',
            '--text': '#e0f0ff', '--text-dim': '#7a8ba8', '--text-muted': '#3d4f6a',
            '--bg-primary': '#05050a', '--bg-secondary': '#0a0a14',
            '--bg-card': '#0d0d1a', '--bg-hover': '#12122a',
            '--border': '#1a1a3e', '--border-glow': '#2a1a4e',
        },
        lightVars: {
            '--accent': '#d6196e', '--accent-light': '#e84d8a',
            '--accent-glow': 'rgba(214, 25, 110, 0.15)',
            '--cyan': '#0891b2', '--cyan-glow': 'rgba(8, 145, 178, 0.2)',
            '--cyan-dim': 'rgba(8, 145, 178, 0.08)',
            '--magenta': '#a300a3', '--magenta-glow': 'rgba(163, 0, 163, 0.15)',
            '--green': '#15803d', '--green-bg': 'rgba(21, 128, 61, 0.08)',
            '--yellow': '#a16207', '--yellow-glow': 'rgba(161, 98, 7, 0.1)',
            '--orange': '#c2410c', '--orange-bg': 'rgba(194, 65, 12, 0.06)',
            '--red': '#dc2626',
            '--text': '#1e293b', '--text-dim': '#475569', '--text-muted': '#94a3b8',
            '--bg-primary': '#f0f2f5', '--bg-secondary': '#e4e7ec',
            '--bg-card': '#ffffff', '--bg-hover': '#f7f8fa',
            '--border': '#cbd5e1', '--border-glow': '#a5b4c8',
        }
    },
    midnight: {
        label: 'Midnight',
        desc: 'Deep blue + electric purple',
        vars: {
            '--accent': '#7c3aed', '--accent-light': '#a78bfa',
            '--accent-glow': 'rgba(124, 58, 237, 0.4)',
            '--cyan': '#38bdf8', '--cyan-glow': 'rgba(56, 189, 248, 0.4)',
            '--cyan-dim': 'rgba(56, 189, 248, 0.15)',
            '--magenta': '#6366f1', '--magenta-glow': 'rgba(99, 102, 241, 0.3)',
            '--green': '#34d399', '--green-bg': 'rgba(52, 211, 153, 0.08)',
            '--yellow': '#c084fc', '--yellow-glow': 'rgba(192, 132, 252, 0.2)',
            '--orange': '#818cf8', '--orange-bg': 'rgba(129, 140, 248, 0.1)',
            '--red': '#f472b6',
            '--text': '#e0e7ff', '--text-dim': '#94a3b8', '--text-muted': '#475569',
            '--bg-primary': '#050510', '--bg-secondary': '#0a0a1e',
            '--bg-card': '#0d0d28', '--bg-hover': '#141432',
            '--border': '#1e1e4a', '--border-glow': '#2e1e5a',
        },
        lightVars: {
            '--accent': '#6d28d9', '--accent-light': '#8b5cf6',
            '--accent-glow': 'rgba(109, 40, 217, 0.15)',
            '--cyan': '#0284c7', '--cyan-glow': 'rgba(2, 132, 199, 0.2)',
            '--cyan-dim': 'rgba(2, 132, 199, 0.08)',
            '--magenta': '#4f46e5', '--magenta-glow': 'rgba(79, 70, 229, 0.15)',
            '--green': '#059669', '--green-bg': 'rgba(5, 150, 105, 0.08)',
            '--yellow': '#7c3aed', '--yellow-glow': 'rgba(124, 58, 237, 0.1)',
            '--orange': '#6366f1', '--orange-bg': 'rgba(99, 102, 241, 0.06)',
            '--red': '#e11d48',
            '--text': '#1e1b4b', '--text-dim': '#4338ca', '--text-muted': '#a5b4fc',
            '--bg-primary': '#eef2ff', '--bg-secondary': '#e0e7ff',
            '--bg-card': '#ffffff', '--bg-hover': '#f5f3ff',
            '--border': '#c7d2fe', '--border-glow': '#a5b4fc',
        }
    },
    matrix: {
        label: 'Matrix',
        desc: 'Terminal green on black',
        vars: {
            '--accent': '#22c55e', '--accent-light': '#4ade80',
            '--accent-glow': 'rgba(34, 197, 94, 0.4)',
            '--cyan': '#39ff14', '--cyan-glow': 'rgba(57, 255, 20, 0.4)',
            '--cyan-dim': 'rgba(57, 255, 20, 0.15)',
            '--magenta': '#16a34a', '--magenta-glow': 'rgba(22, 163, 74, 0.3)',
            '--green': '#4ade80', '--green-bg': 'rgba(74, 222, 128, 0.08)',
            '--yellow': '#a3e635', '--yellow-glow': 'rgba(163, 230, 53, 0.2)',
            '--orange': '#86efac', '--orange-bg': 'rgba(134, 239, 172, 0.1)',
            '--red': '#ef4444',
            '--text': '#d1fae5', '--text-dim': '#6ee7b7', '--text-muted': '#365314',
            '--bg-primary': '#020a02', '--bg-secondary': '#061006',
            '--bg-card': '#081408', '--bg-hover': '#0e200e',
            '--border': '#1a3a1a', '--border-glow': '#1a4a1a',
        },
        lightVars: {
            '--accent': '#16a34a', '--accent-light': '#22c55e',
            '--accent-glow': 'rgba(22, 163, 74, 0.15)',
            '--cyan': '#15803d', '--cyan-glow': 'rgba(21, 128, 61, 0.2)',
            '--cyan-dim': 'rgba(21, 128, 61, 0.08)',
            '--magenta': '#166534', '--magenta-glow': 'rgba(22, 101, 52, 0.15)',
            '--green': '#22c55e', '--green-bg': 'rgba(34, 197, 94, 0.08)',
            '--yellow': '#65a30d', '--yellow-glow': 'rgba(101, 163, 13, 0.1)',
            '--orange': '#4ade80', '--orange-bg': 'rgba(74, 222, 128, 0.06)',
            '--red': '#dc2626',
            '--text': '#14532d', '--text-dim': '#166534', '--text-muted': '#86efac',
            '--bg-primary': '#f0fdf4', '--bg-secondary': '#dcfce7',
            '--bg-card': '#ffffff', '--bg-hover': '#f0fdf4',
            '--border': '#bbf7d0', '--border-glow': '#86efac',
        }
    },
    ember: {
        label: 'Ember',
        desc: 'Warm amber + orange tones',
        vars: {
            '--accent': '#f59e0b', '--accent-light': '#fbbf24',
            '--accent-glow': 'rgba(245, 158, 11, 0.4)',
            '--cyan': '#fb923c', '--cyan-glow': 'rgba(251, 146, 60, 0.4)',
            '--cyan-dim': 'rgba(251, 146, 60, 0.15)',
            '--magenta': '#ea580c', '--magenta-glow': 'rgba(234, 88, 12, 0.3)',
            '--green': '#84cc16', '--green-bg': 'rgba(132, 204, 22, 0.08)',
            '--yellow': '#fde047', '--yellow-glow': 'rgba(253, 224, 71, 0.2)',
            '--orange': '#f97316', '--orange-bg': 'rgba(249, 115, 22, 0.1)',
            '--red': '#dc2626',
            '--text': '#fef3c7', '--text-dim': '#d97706', '--text-muted': '#92400e',
            '--bg-primary': '#0a0502', '--bg-secondary': '#120a04',
            '--bg-card': '#1a0e06', '--bg-hover': '#24140a',
            '--border': '#3e2a1a', '--border-glow': '#4e3a1a',
        },
        lightVars: {
            '--accent': '#d97706', '--accent-light': '#f59e0b',
            '--accent-glow': 'rgba(217, 119, 6, 0.15)',
            '--cyan': '#ea580c', '--cyan-glow': 'rgba(234, 88, 12, 0.2)',
            '--cyan-dim': 'rgba(234, 88, 12, 0.08)',
            '--magenta': '#c2410c', '--magenta-glow': 'rgba(194, 65, 12, 0.15)',
            '--green': '#65a30d', '--green-bg': 'rgba(101, 163, 13, 0.08)',
            '--yellow': '#a16207', '--yellow-glow': 'rgba(161, 98, 7, 0.1)',
            '--orange': '#c2410c', '--orange-bg': 'rgba(194, 65, 12, 0.06)',
            '--red': '#dc2626',
            '--text': '#451a03', '--text-dim': '#92400e', '--text-muted': '#fbbf24',
            '--bg-primary': '#fffbeb', '--bg-secondary': '#fef3c7',
            '--bg-card': '#ffffff', '--bg-hover': '#fffbeb',
            '--border': '#fde68a', '--border-glow': '#fbbf24',
        }
    },
    arctic: {
        label: 'Arctic',
        desc: 'Cool whites + icy blue',
        vars: {
            '--accent': '#0ea5e9', '--accent-light': '#38bdf8',
            '--accent-glow': 'rgba(14, 165, 233, 0.4)',
            '--cyan': '#67e8f9', '--cyan-glow': 'rgba(103, 232, 249, 0.4)',
            '--cyan-dim': 'rgba(103, 232, 249, 0.15)',
            '--magenta': '#06b6d4', '--magenta-glow': 'rgba(6, 182, 212, 0.3)',
            '--green': '#2dd4bf', '--green-bg': 'rgba(45, 212, 191, 0.08)',
            '--yellow': '#a5f3fc', '--yellow-glow': 'rgba(165, 243, 252, 0.2)',
            '--orange': '#22d3ee', '--orange-bg': 'rgba(34, 211, 238, 0.1)',
            '--red': '#f43f5e',
            '--text': '#ecfeff', '--text-dim': '#a5f3fc', '--text-muted': '#155e75',
            '--bg-primary': '#020a0e', '--bg-secondary': '#041218',
            '--bg-card': '#061a22', '--bg-hover': '#0a2430',
            '--border': '#1a3a4e', '--border-glow': '#1a4a5e',
        },
        lightVars: {
            '--accent': '#0284c7', '--accent-light': '#0ea5e9',
            '--accent-glow': 'rgba(2, 132, 199, 0.15)',
            '--cyan': '#0891b2', '--cyan-glow': 'rgba(8, 145, 178, 0.2)',
            '--cyan-dim': 'rgba(8, 145, 178, 0.08)',
            '--magenta': '#0e7490', '--magenta-glow': 'rgba(14, 116, 144, 0.15)',
            '--green': '#0d9488', '--green-bg': 'rgba(13, 148, 136, 0.08)',
            '--yellow': '#155e75', '--yellow-glow': 'rgba(21, 94, 117, 0.1)',
            '--orange': '#06b6d4', '--orange-bg': 'rgba(6, 182, 212, 0.06)',
            '--red': '#e11d48',
            '--text': '#164e63', '--text-dim': '#0e7490', '--text-muted': '#a5f3fc',
            '--bg-primary': '#ecfeff', '--bg-secondary': '#cffafe',
            '--bg-card': '#ffffff', '--bg-hover': '#ecfeff',
            '--border': '#a5f3fc', '--border-glow': '#67e8f9',
        }
    },
};

// Default root CSS values (captured once to allow scheme reset)
const ROOT_DEFAULTS = {};
(function captureDefaults() {
    const style = getComputedStyle(document.documentElement);
    for (const key of SCHEME_VAR_KEYS) {
        ROOT_DEFAULTS[key] = style.getPropertyValue(key).trim();
    }
})();

// Rebuilt on load and when locale changes (refreshSettingsUI)
function rebuildSchemeButtons() {
    const grid = document.getElementById('schemeGrid');
    if (!grid) return;
    const dotKeys = ['--accent', '--cyan', '--magenta', '--green', '--yellow', '--orange', '--red', '--text'];
    const fmt = catalogFmt;
    const meta = {
        cyberpunk: {labelKey: 'ui.scheme.cyberpunk.label', descKey: 'ui.scheme.cyberpunk.desc'},
        midnight: {labelKey: 'ui.scheme.midnight.label', descKey: 'ui.scheme.midnight.desc'},
        matrix: {labelKey: 'ui.scheme.matrix.label', descKey: 'ui.scheme.matrix.desc'},
        ember: {labelKey: 'ui.scheme.ember.label', descKey: 'ui.scheme.ember.desc'},
        arctic: {labelKey: 'ui.scheme.arctic.label', descKey: 'ui.scheme.arctic.desc'},
    };
    grid.innerHTML = '';
    for (const [key, scheme] of Object.entries(COLOR_SCHEMES)) {
        const m = meta[key];
        const label = m ? fmt(m.labelKey) : key;
        const desc = m ? fmt(m.descKey) : '';
        const title = fmt('ui.scheme.apply_title', {name: label});
        const dots = dotKeys.map(k => `<span class="scheme-dot" style="background: ${scheme.vars[k]};"></span>`).join('');
        grid.insertAdjacentHTML('beforeend',
            `<button class="scheme-btn" data-action="settingColorScheme" data-scheme="${key}" title="${escapeHtml(title)}">` +
            `<div class="scheme-btn-name">${escapeHtml(label)}</div>` +
            `<div class="scheme-btn-desc">${escapeHtml(desc)}</div>` +
            `<div class="scheme-btn-preview">${dots}</div>` +
            `</button>`
        );
    }
}

function applyColorScheme(name) {
    const scheme = COLOR_SCHEMES[name];
    if (!scheme) return;
    prefs.setItem('colorScheme', name);
    prefs.removeItem('customSchemeVars');
    const isLight = document.documentElement.getAttribute('data-theme') === 'light';
    const vars = isLight && scheme.lightVars ? scheme.lightVars : scheme.vars;
    const root = document.documentElement.style;
    for (const key of SCHEME_VAR_KEYS) {
        root.removeProperty(key);
    }
    for (const [k, v] of Object.entries(vars)) {
        root.setProperty(k, v);
    }
    refreshSettingsUI();
    if (typeof window !== 'undefined' && typeof window.invalidateNativeDragIconCache === 'function') {
        window.invalidateNativeDragIconCache();
    }
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
}

function settingColorScheme(name) {
    applyColorScheme(name);
}

// ── Custom Color Schemes ──

function hexToRgba(hex, alpha) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function readCustomColorsFromPickers() {
    const vars = {};
    document.querySelectorAll('.custom-color-input').forEach(input => {
        vars[input.dataset.var] = input.value;
    });
    // Auto-generate rgba glow/dim variants from hex pickers
    if (vars['--accent']) vars['--accent-glow'] = hexToRgba(vars['--accent'], 0.4);
    if (vars['--cyan']) {
        vars['--cyan-glow'] = hexToRgba(vars['--cyan'], 0.4);
        vars['--cyan-dim'] = hexToRgba(vars['--cyan'], 0.15);
    }
    if (vars['--magenta']) vars['--magenta-glow'] = hexToRgba(vars['--magenta'], 0.3);
    if (vars['--yellow']) vars['--yellow-glow'] = hexToRgba(vars['--yellow'], 0.2);
    if (vars['--green']) vars['--green-bg'] = hexToRgba(vars['--green'], 0.08);
    if (vars['--orange']) vars['--orange-bg'] = hexToRgba(vars['--orange'], 0.1);
    return vars;
}

function applyCustomVars(vars) {
    const root = document.documentElement.style;
    for (const [k, v] of Object.entries(vars)) {
        root.setProperty(k, v);
    }
}

function applySchemeVars(vars) {
    const root = document.documentElement.style;
    const isLight = document.documentElement.getAttribute('data-theme') === 'light';
    const lightKeep = new Set(['--bg-primary', '--bg-secondary', '--bg-card', '--bg-hover',
        '--text', '--text-dim', '--text-muted', '--border', '--border-glow']);
    // Always remove ALL inline vars first so CSS selectors can take effect
    for (const key of SCHEME_VAR_KEYS) {
        root.removeProperty(key);
    }
    // In light mode, filter out bg/text/border — let [data-theme="light"] CSS handle those
    const filtered = isLight
        ? Object.fromEntries(Object.entries(vars).filter(([k]) => !lightKeep.has(k)))
        : vars;
    applyCustomVars(filtered);
    if (typeof window !== 'undefined' && typeof window.invalidateNativeDragIconCache === 'function') {
        window.invalidateNativeDragIconCache();
    }
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
}

function applyCustomScheme() {
    const vars = readCustomColorsFromPickers();
    prefs.setItem('colorScheme', 'custom');
    prefs.setItem('customSchemeVars', vars);
    applySchemeVars(vars);
    document.querySelectorAll('.scheme-btn').forEach(b => b.classList.remove('active'));
    refreshCustomPresetUI();
}

function showSavePreset() {
    const row = document.getElementById('savePresetRow');
    const input = document.getElementById('savePresetName');
    const presets = prefs.getObject('customSchemePresets', []);
    input.value = 'Custom ' + (presets.length + 1);
    row.style.display = 'flex';
    input.focus();
    input.select();
    input.onkeydown = (e) => {
        if (e.key === 'Enter') confirmSavePreset();
        if (e.key === 'Escape') cancelSavePreset();
    };
}

function cancelSavePreset() {
    document.getElementById('savePresetRow').style.display = 'none';
}

function confirmSavePreset() {
    const input = document.getElementById('savePresetName');
    const name = input.value.trim();
    if (!name) return;
    const vars = readCustomColorsFromPickers();
    const presets = prefs.getObject('customSchemePresets', []);
    presets.push({name, vars});
    prefs.setItem('customSchemePresets', presets);
    prefs.setItem('colorScheme', 'custom-' + (presets.length - 1));
    prefs.setItem('customSchemeVars', vars);
    applySchemeVars(vars);
    document.querySelectorAll('.scheme-btn').forEach(b => b.classList.remove('active'));
    document.getElementById('savePresetRow').style.display = 'none';
    refreshCustomPresetUI();
}

function loadCustomPreset(idx) {
    const presets = prefs.getObject('customSchemePresets', []);
    const preset = presets[idx];
    if (!preset) return;
    for (const input of document.querySelectorAll('.custom-color-input')) {
        const v = input.dataset.var;
        if (preset.vars[v]) input.value = preset.vars[v];
    }
    prefs.setItem('colorScheme', 'custom-' + idx);
    prefs.setItem('customSchemeVars', preset.vars);
    applySchemeVars(preset.vars);
    document.querySelectorAll('.scheme-btn').forEach(b => b.classList.remove('active'));
    refreshCustomPresetUI();
}

async function deleteCustomSchemes() {
    if (!await confirmAction('Delete all saved custom color schemes?', 'Delete Schemes')) return;
    prefs.removeItem('customSchemePresets');
    refreshCustomPresetUI();
}

function refreshCustomPresetUI() {
    const container = document.getElementById('customSchemeSaved');
    const deleteBtn = document.getElementById('btnDeleteCustom');
    const presets = prefs.getObject('customSchemePresets', []);
    const currentScheme = prefs.getItem('colorScheme') || 'cyberpunk';

    if (presets.length === 0) {
        container.innerHTML = '';
        deleteBtn.style.display = 'none';
        return;
    }

    deleteBtn.style.display = '';
    container.innerHTML = presets.map((p, i) => {
        const active = currentScheme === 'custom-' + i ? ' active' : '';
        const accent = p.vars['--accent'] || '#ff2a6d';
        const cyan = p.vars['--cyan'] || '#05d9e8';
        const magenta = p.vars['--magenta'] || '#d300c5';
        return `<button class="custom-preset-chip${active}" data-action="loadCustomPreset" data-idx="${i}" title="Load custom preset: ${escapeHtml(p.name || 'Preset ' + (i + 1))}">
      <span class="custom-preset-chip-dots">
        <span class="custom-preset-chip-dot" style="background:${accent}"></span>
        <span class="custom-preset-chip-dot" style="background:${cyan}"></span>
        <span class="custom-preset-chip-dot" style="background:${magenta}"></span>
      </span>
      ${escapeHtml(p.name)}
    </button>`;
    }).join('');
    if (typeof initDragReorder === 'function') {
        initDragReorder(container, '.custom-preset-chip', 'presetChipOrder', {
            direction: 'horizontal',
            getKey: (el) => el.textContent.trim(),
            onReorder: () => {
                // Reorder the presets array to match
                const chips = [...container.querySelectorAll('.custom-preset-chip')];
                const oldPresets = prefs.getObject('customSchemePresets', []);
                const newPresets = [];
                for (const chip of chips) {
                    const idx = parseInt(chip.dataset.idx);
                    if (oldPresets[idx]) newPresets.push(oldPresets[idx]);
                }
                prefs.setItem('customSchemePresets', newPresets);
            },
        });
    }
}

function settingToggleTheme() {
    const html = document.documentElement;
    const current = html.getAttribute('data-theme');
    const next = current === 'light' ? 'dark' : 'light';
    html.setAttribute('data-theme', next);
    prefs.setItem('theme', next);
    showToast(toastFmt('toast.theme_set', {next}));
    const scheme = prefs.getItem('colorScheme') || 'cyberpunk';
    if (scheme.startsWith('custom')) {
        const customVars = prefs.getObject('customSchemeVars', {});
        if (Object.keys(customVars).length > 0) {
            applySchemeVars(customVars);
        } else {
            for (const key of SCHEME_VAR_KEYS) html.style.removeProperty(key);
        }
    } else {
        applyColorScheme(scheme);
    }
    refreshSettingsUI();
    /* Tray popover HUD: push `ui_theme` on the same `TrayNowPlayingPayload` path as playback (belt-and-suspenders with `tray-popover-ui-theme`). */
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
}

function settingToggleCrt() {
    const current = prefs.getItem('crtEffects') !== 'off';
    const next = !current;
    prefs.setItem('crtEffects', next ? 'on' : 'off');
    applyCrtSetting(next);
    showToast(toastFmt('toast.crt_effects', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function applyCrtSetting(on) {
    document.querySelectorAll('.crt-scanline, .crt-scanline-v').forEach(el => {
        el.style.display = on ? '' : 'none';
    });
    const app = document.querySelector('.app');
    if (app) {
        app.classList.toggle('no-crt', !on);
    }
}

function settingToggleNeonGlow() {
    const current = prefs.getItem('neonGlow') !== 'off';
    const next = !current;
    prefs.setItem('neonGlow', next ? 'on' : 'off');
    applyNeonGlowSetting(next);
    showToast(toastFmt('toast.neon_glow', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function applyNeonGlowSetting(on) {
    document.body.classList.toggle('no-neon-glow', !on);
}

function formatCacheSize(bytes) {
    const n = Number(bytes);
    if (!Number.isFinite(n) || n <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB'];
    const i = Math.min(Math.floor(Math.log(n) / Math.log(1024)), units.length - 1);
    return (n / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0) + ' ' + units[i];
}

/** Last successful `db_cache_stats` rows — repainted synchronously when available (app startup + Settings tab open) so the table is not blank while SQLite runs COUNT/dbstat work. */
let _lastDbCacheStatsRows = null;
/** Bumps on each `renderCacheStats` call so a slow in-flight `dbCacheStats` cannot repaint stale counts after a clear. */
let _cacheStatsFetchGen = 0;

function invalidateDbCacheStatsSnapshot() {
    _lastDbCacheStatsRows = null;
}

/** Builds the Database Caches table markup from `db_cache_stats` rows (current locale). */
function buildCacheStatsTableHtml(rows) {
    const _cf = catalogFmt;
    return `<table style="width:100%;border-collapse:collapse;font-family:'Share Tech Mono',monospace;">
      <thead><tr style="color:var(--cyan);font-size:10px;text-transform:uppercase;letter-spacing:1px;">
        <th style="text-align:left;padding:4px 8px;">${_cf('ui.settings.cache_table_cache')}</th>
        <th style="text-align:right;padding:4px 8px;">${_cf('ui.settings.cache_table_items')}</th>
        <th style="text-align:right;padding:4px 8px;">${_cf('ui.settings.cache_table_size')}</th>
        <th style="text-align:center;padding:4px 8px;width:60px;"></th>
      </tr></thead>
      <tbody>${rows.map(s => {
        let countStr = '';
        if (s.count > 0) {
            if (s.total > 0 && s.key !== 'database' && !s.key.includes('_scans')) {
                countStr = s.count.toLocaleString() + ` / ${s.total.toLocaleString()}`;
            } else if (s.key.includes('_scans')) {
                countStr = `${s.count.toLocaleString()} (${_cf('ui.settings.cache_scans_suffix', {n: s.total.toLocaleString()})})`;
            } else {
                countStr = s.count.toLocaleString();
            }
        } else {
            countStr = s.key === 'database' ? '' : '0';
        }
        const sizeStr = formatCacheSize(s.sizeBytes ?? s.size_bytes);
        const canClear = s.key !== 'database' && !s.key.includes('_scans');
        const canBuild = s.count === 0 && (s.key === 'xref' || s.key === 'fingerprint');
        let action = '';
        if (canBuild) {
            action = `<button class="btn btn-secondary" data-action="buildCacheTable" data-cache="${s.key}" style="font-size:9px;padding:2px 6px;">${_cf('ui.settings.cache_build')}</button>`;
        } else if (canClear && s.count > 0) {
            action = `<button class="btn btn-secondary" data-action="clearCacheTable" data-cache="${s.key}" style="font-size:9px;padding:2px 6px;">${_cf('ui.settings.cache_clear')}</button>`;
        }
        const rowLabel = cacheStatRowLabel(s.key, s.label, _cf);
        const safeLabel = typeof escapeHtml === 'function' ? escapeHtml(rowLabel) : rowLabel;
        return `<tr style="border-bottom:1px solid rgba(26,26,62,0.2);">
          <td style="padding:4px 8px;color:var(--text);">${safeLabel}</td>
          <td style="padding:4px 8px;text-align:right;color:var(--text-muted);">${countStr}</td>
          <td style="padding:4px 8px;text-align:right;color:${(Number(s.sizeBytes ?? s.size_bytes) || 0) > 10 * 1024 * 1024 ? 'var(--yellow)' : 'var(--text-muted)'};">${sizeStr}</td>
          <td style="padding:4px 8px;text-align:center;">${action}</td>
        </tr>`;
    }).join('')}</tbody>
    </table>`;
}

/** Maps `db_cache_stats` row `key` → `appFmt` catalog key (Database Caches table). */
const CACHE_STAT_I18N_KEYS = {
    bpm: 'ui.settings.cache_row_bpm',
    key: 'ui.settings.cache_row_key',
    lufs: 'ui.settings.cache_row_lufs',
    waveform: 'ui.settings.cache_row_waveform',
    spectrogram: 'ui.settings.cache_row_spectrogram',
    xref: 'ui.settings.cache_row_xref',
    fingerprint: 'ui.settings.cache_row_fingerprint',
    kvr: 'ui.settings.cache_row_kvr',
    plugin_scans: 'ui.settings.cache_row_plugin_scans',
    audio_scans: 'ui.settings.cache_row_audio_scans',
    daw_scans: 'ui.settings.cache_row_daw_scans',
    preset_scans: 'ui.settings.cache_row_preset_scans',
    midi_scans: 'ui.settings.cache_row_midi_scans',
    pdf_scans: 'ui.settings.cache_row_pdf_scans',
    database: 'ui.settings.cache_row_database',
};

function cacheStatRowLabel(statKey, fallbackLabel, _cf) {
    const k = CACHE_STAT_I18N_KEYS[statKey];
    if (k && typeof _cf === 'function') return _cf(k);
    return fallbackLabel;
}

/** Fetches Settings → Database Caches table from `db_cache_stats`. Replays last snapshot synchronously when available (stale-while-revalidate). Invoked on app load (`app.js`) and when opening Settings. */
async function renderCacheStats() {
    const grid = document.getElementById('cacheStatsGrid');
    if (!grid) return;
    const gen = ++_cacheStatsFetchGen;
    if (Array.isArray(_lastDbCacheStatsRows) && _lastDbCacheStatsRows.length > 0) {
        grid.innerHTML = buildCacheStatsTableHtml(_lastDbCacheStatsRows);
    }
    try {
        const stats = await window.vstUpdater.dbCacheStats();
        if (gen !== _cacheStatsFetchGen) return;
        const rows = Array.isArray(stats) ? stats : [];
        _lastDbCacheStatsRows = rows;
        grid.innerHTML = buildCacheStatsTableHtml(rows);
        settingsReflow();
    } catch (e) {
        if (gen !== _cacheStatsFetchGen) return;
        const msg = catalogFmt('ui.settings.cache_load_failed', {err: e.message || String(e)});
        grid.innerHTML = `<span style="color:var(--red);font-size:11px;">${typeof escapeHtml === 'function' ? escapeHtml(msg) : msg}</span>`;
    }
}

async function exportSettingsPdf() {
    const shortcuts = typeof getShortcuts === 'function' ? getShortcuts() : {};
    const allPrefs = prefs._cache || {};

    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi) return;
    const savePath = await dialogApi.save({
        title: catalogFmt('menu.export_settings_keybindings'),
        defaultPath: 'audio-haxor-settings.pdf',
        filters: [
            {name: catalogFmt('ui.file_filter.pdf'), extensions: ['pdf']},
            {name: catalogFmt('ui.file_filter.text'), extensions: ['txt']},
        ],
    });
    if (!savePath) return;

    if (savePath.endsWith('.pdf')) {
        // PDF export via Rust
        const headers = [catalogFmt('ui.export.col_setting_shortcut'), catalogFmt('ui.export.col_value')];
        const rows = [];
        rows.push(['── KEYBOARD SHORTCUTS ──', '']);
        for (const [id, sc] of Object.entries(shortcuts)) {
            rows.push([sc.label, sc.mod ? `Cmd+${sc.key}` : sc.key]);
        }
        rows.push(['── PREFERENCES ──', '']);
        for (const [k, v] of Object.entries(allPrefs)) {
            if (typeof v === 'object') continue;
            rows.push([k, String(v)]);
        }
        try {
            await window.vstUpdater.exportPdf(catalogFmt('ui.export.pdf_title_settings'), headers, rows, savePath);
            showToast(toastFmt('toast.settings_pdf_exported'));
        } catch (e) {
            showToast(toastFmt('toast.pdf_export_failed', {err: e.message || e}), 4000, 'error');
        }
    } else {
        // Text export
        let text = 'AUDIO_HAXOR — Settings & Keybindings\n' + '='.repeat(50) + '\n\n';
        text += `Generated: ${new Date().toLocaleString()}\n\n── KEYBOARD SHORTCUTS ──\n\n`;
        for (const [id, sc] of Object.entries(shortcuts)) {
            text += `  ${sc.label.padEnd(35)} ${sc.mod ? 'Cmd+' + sc.key : sc.key}\n`;
        }
        text += '\n── PREFERENCES ──\n\n';
        for (const [k, v] of Object.entries(allPrefs)) {
            if (typeof v === 'object') continue;
            text += `  ${k.padEnd(35)} ${v}\n`;
        }
        await window.__TAURI__.core.invoke('write_text_file', {filePath: savePath, contents: text});
        showToast(toastFmt('toast.settings_exported'));
    }
}

async function exportLogPdf() {
    try {
        const log = await window.vstUpdater.readLog();
        if (!log || !log.trim()) {
            showToast(toastFmt('toast.log_empty'), 3000, 'warning');
            return;
        }

        const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
        if (!dialogApi) return;
        const savePath = await dialogApi.save({
            title: catalogFmt('menu.export_app_log'),
            defaultPath: 'audio-haxor-log.pdf',
            filters: [
                {name: catalogFmt('ui.file_filter.pdf'), extensions: ['pdf']},
                {name: catalogFmt('ui.file_filter.text'), extensions: ['txt']},
            ],
        });
        if (!savePath) return;

        if (savePath.endsWith('.pdf')) {
            const headers = [catalogFmt('ui.export.col_timestamp'), catalogFmt('ui.export.col_message')];
            const rows = log.split('\n').filter(Boolean).map(line => {
                const match = line.match(/^\[(.+?)\] (.+)$/);
                return match ? [match[1], match[2]] : ['', line];
            });
            try {
                await window.vstUpdater.exportPdf(catalogFmt('ui.export.pdf_title_error_log'), headers, rows, savePath);
                showToast(toastFmt('toast.log_pdf_exported'));
            } catch (e) {
                showToast(toastFmt('toast.pdf_export_failed', {err: e.message || e}), 4000, 'error');
            }
        } else {
            await window.__TAURI__.core.invoke('write_text_file', {filePath: savePath, contents: log});
            showToast(toastFmt('toast.log_exported'));
        }
    } catch (e) {
        showToast(toastFmt('toast.could_not_read_log', {err: e?.message || e}), 4000, 'error');
    }
}

async function renderCacheFilesList() {
    const container = document.getElementById('cacheFilesList');
    if (!container) return;
    try {
        const files = await window.vstUpdater.listDataFiles();
        if (files.length === 0) {
            const msg = catalogFmt('ui.settings.cache_files_empty');
            const esc = typeof escapeHtml === 'function' ? escapeHtml : (s) => String(s);
            container.innerHTML = `<div class="state-message"><div class="state-icon">&#128193;</div><h2>${esc(msg)}</h2></div>`;
            return;
        }
        const totalSize = files.reduce((s, f) => s + (f.size || 0), 0);
        const sz = typeof formatAudioSize === 'function' ? formatAudioSize(totalSize) : Math.round(totalSize / 1024) + ' KB';
        const summary = catalogFmt('ui.settings.cache_files_summary', {n: files.length, size: sz});
        const _h = catalogFmt;
        container.innerHTML = `<div style="margin-bottom:6px;color:var(--text-muted);font-size:10px;">${typeof escapeHtml === 'function' ? escapeHtml(summary) : summary}</div>` +
            `<table style="width:100%;border-collapse:collapse;font-size:10px;">
        <tr style="color:var(--text-dim);border-bottom:1px solid var(--border);">
          <th style="text-align:left;padding:3px 6px;">${_h('ui.settings.cache_files_col_file')}</th>
          <th style="text-align:right;padding:3px 6px;">${_h('ui.settings.cache_files_col_size')}</th>
          <th style="text-align:left;padding:3px 6px;">${_h('ui.settings.cache_files_col_modified')}</th>
          <th style="padding:3px 6px;"></th>
        </tr>
        ${files.map(f => `<tr style="border-bottom:1px solid rgba(26,26,62,0.2);" title="${escapeHtml(f.path)}">
          <td style="padding:3px 6px;color:var(--text);cursor:pointer;" data-action="revealDataFile" data-path="${escapeHtml(f.path)}">${escapeHtml(f.name)}</td>
          <td style="padding:3px 6px;text-align:right;color:var(--cyan);font-family:Orbitron,sans-serif;">${f.sizeFormatted}</td>
          <td style="padding:3px 6px;color:var(--text-muted);">${f.modified}</td>
          <td style="padding:3px 6px;"><button class="btn-small btn-stop" data-action="deleteDataFile" data-name="${escapeHtml(f.name)}" title="${escapeHtml(catalogFmt('ui.settings.cache_delete_file_tooltip', {name: f.name}))}" style="padding:1px 6px;font-size:9px;">&#10005;</button></td>
        </tr>`).join('')}
      </table>`;
    } catch (e) {
        const errLine = catalogFmt('ui.settings.cache_files_error', {err: e.message || String(e)});
        container.innerHTML = `<div style="color:var(--red);padding:8px;">${typeof escapeHtml === 'function' ? escapeHtml(errLine) : errLine}</div>`;
    }
}

async function settingClearAllDatabases() {
    // Wipe every scan-history DB table (plugins, audio, DAW, presets, MIDI, PDF).
    // Caches (BPM/Key/LUFS/Waveform/Spectrogram/Xref/Fingerprint/KVR) are NOT
    // touched here — those are handled by settingClearAnalysisCache.
    const msg = catalogFmt('confirm.clear_all_scan_databases');
    const title = catalogFmt('ui.btn.clear_all_databases');
    const ok = typeof confirmAction === 'function'
        ? await confirmAction(msg, title)
        : window.confirm(msg);
    if (!ok) return;
    const clears = [
        ['menu.tab_plugins', () => window.vstUpdater.clearHistory()],
        ['menu.tab_samples', () => window.vstUpdater.clearAudioHistory()],
        ['menu.tab_daw', () => window.vstUpdater.clearDawHistory()],
        ['menu.tab_presets', () => window.vstUpdater.clearPresetHistory()],
        ['menu.tab_midi', () => window.vstUpdater.clearMidiHistory()],
        ['menu.tab_pdf', () => window.vstUpdater.clearPdfHistory()],
    ];
    for (const [labelKey, fn] of clears) {
        try {
            await fn();
        } catch (e) {
            const label = catalogFmt(labelKey);
            const errLine = catalogFmt('toast.failed_clear_scan_db_history', {label, err: e.message || e});
            if (typeof showToast === 'function') showToast(errLine, 4000, 'error');
        }
    }
    if (typeof showToast === 'function') {
        showToast(catalogFmt('toast.all_scan_databases_cleared'));
    }
    if (typeof invalidateDbCacheStatsSnapshot === 'function') invalidateDbCacheStatsSnapshot();
    if (typeof renderCacheStats === 'function') renderCacheStats();
}

async function settingClearAnalysisCache() {
    // Delete separate cache files
    const files = ['bpm-cache.json', 'key-cache.json', 'lufs-cache.json', 'waveform-cache.json', 'spectrogram-cache.json', 'xref-cache.json'];
    for (const f of files) {
        try {
            await window.vstUpdater.writeCacheFile(f, {});
        } catch (e) {
            if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
        }
    }
    // Also remove old cache keys from prefs if they exist
    prefs.removeItem('bpmCache');
    prefs.removeItem('keyCache');
    prefs.removeItem('lufsCache');
    prefs.removeItem('waveformCache');
    prefs.removeItem('spectrogramCache');
    // Clear in-memory caches
    if (typeof _bpmCache !== 'undefined') {
        _bpmCache = {};
        _keyCache = {};
        _lufsCache = {};
    }
    if (typeof _waveformCache !== 'undefined') {
        _waveformCache = {};
        _spectrogramCache = {};
    }
    showToast(toastFmt('toast.analysis_cache_cleared_long'));
    invalidateDbCacheStatsSnapshot();
    void renderCacheStats();
}

function settingResetAllUI() {
    // All layout/ordering/sizing prefs keys
    const uiKeys = [
        'tabOrder', 'settingsSectionOrder', 'audioEngineSectionOrder', 'columnWidths',
        'playerSectionOrder', 'playerDock', 'playerWidth', 'playerHeight', 'playerExpanded', 'playerPaneHidden',
        'headerStatsOrder', 'statsBarOrder',
        'pluginStatsOrder', 'audioStatsOrder', 'dawStatsOrder', 'presetStatsOrder', 'midiStatsOrder', 'pdfStatsOrder',
        'audioColumnOrder', 'dawColumnOrder', 'presetColumnOrder',
        'favItemOrder', 'fileFavOrder', 'noteCardOrder', 'tagCardOrder', 'presetChipOrder',
        'hmCardOrder', 'fzfParamOrder', 'shortcutOrder', 'vizTileOrder',
        'scanBtnsParent', 'dashBtnParent',
        'similarDock', 'similarWidth', 'similarHeight',
        'expandOnClick',
    ];
    // Modal geometry keys
    const allKeys = Object.keys(prefs._cache || {});
    for (const key of allKeys) {
        if (key.startsWith('modal_') || key.startsWith('settingsRows_') || key.endsWith('BtnOrder')) {
            uiKeys.push(key);
        }
    }
    for (const key of uiKeys) {
        prefs.removeItem(key);
    }
    showToast(toastFmt('toast.all_ui_factory_reset'));
    setTimeout(() => location.reload(), 1000);
}

function settingResetColumns() {
    prefs.removeItem('columnWidths');
    // Re-init tables if they exist
    const audioTable = document.getElementById('audioTable');
    const dawTable = document.getElementById('dawTable');
    if (audioTable) {
        audioTable.querySelectorAll('thead th').forEach(th => {
            th.style.width = '';
        });
    }
    if (dawTable) {
        dawTable.querySelectorAll('thead th').forEach(th => {
            th.style.width = '';
        });
    }
}

async function settingClearAllHistory() {
    if (!await confirmAction(
        catalogFmt('confirm.clear_all_history_settings'),
        catalogFmt('ui.history.confirm_clear_title'),
    )) return;
    showGlobalProgress();
    try {
        await Promise.all([
            window.vstUpdater.clearHistory(),
            window.vstUpdater.clearAudioHistory(),
            window.vstUpdater.clearDawHistory(),
            window.vstUpdater.clearPresetHistory(),
            window.vstUpdater.clearPdfHistory(),
            window.vstUpdater.clearMidiHistory(),
        ]);
        showToast(toastFmt('toast.all_scan_history_cleared'));
    } catch (e) {
        showToast(toastFmt('toast.failed_clear_history', {err: e.message || e}), 4000, 'error');
    } finally {
        hideGlobalProgress();
    }
}

async function resetAllScans() {
    if (!await confirmAction('Reset everything? This will clear all scan results, history, and KVR cache. The app will return to its initial state.\n\nThis cannot be undone.', 'Reset All Scans')) return;
    showGlobalProgress();
    try {
        // Stop any running scans
        await Promise.all([
            window.vstUpdater.stopScan().catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            }),
            window.vstUpdater.stopAudioScan().catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            }),
            window.vstUpdater.stopDawScan().catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            }),
            window.vstUpdater.stopPresetScan().catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            }),
            window.vstUpdater.stopPdfScan().catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            }),
            window.vstUpdater.stopMidiScan().catch(e => {
                if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
            }),
        ]);
        // Clear backend history + KVR cache
        await Promise.all([
            window.vstUpdater.clearHistory(),
            window.vstUpdater.clearAudioHistory(),
            window.vstUpdater.clearDawHistory(),
            window.vstUpdater.clearPresetHistory(),
            window.vstUpdater.clearPdfHistory(),
            window.vstUpdater.clearMidiHistory(),
            window.vstUpdater.updateKvrCache([]),
        ]);
        // Clear in-memory data
        allPlugins = [];
        pluginsWithUpdates = [];
        if (typeof allAudioSamples !== 'undefined') allAudioSamples = [];
        if (typeof filteredAudioSamples !== 'undefined') filteredAudioSamples = [];
        if (typeof allDawProjects !== 'undefined') allDawProjects = [];
        if (typeof allPresets !== 'undefined') allPresets = [];
        if (typeof recentlyPlayed !== 'undefined') {
            recentlyPlayed = [];
            saveRecentlyPlayed();
        }
        // Clear xref cache
        if (typeof _xrefCache !== 'undefined') {
            for (const k in _xrefCache) delete _xrefCache[k];
        }
        // Reset plugin UI
        document.getElementById('pluginList').innerHTML = '<div class="state-message" id="emptyState"><div class="state-icon">&#128268;</div><h2>Audio Plugin Scanner</h2><p>Click <strong>"Scan Plugins"</strong> to discover all VST2, VST3, CLAP, and Audio Unit plugins on your system.</p></div>';
        if (typeof applyInventoryCounts === 'function') {
            const z = {plugins: 0, samples: 0, daw: 0, presets: 0, pdf: 0, midi: 0};
            window.__inventoryCounts = z;
            applyInventoryCounts(z);
        } else {
            document.getElementById('totalCount').textContent = '0';
        }
        window.__audioScanPendingFound = 0;
        window.__dawScanPendingFound = 0;
        window.__presetScanPendingFound = 0;
        window.__pdfScanPendingFound = 0;
        window.__midiScanPendingFound = 0;
        document.getElementById('dirsSection').style.display = 'none';
        document.getElementById('btnCheckUpdates').disabled = true;
        // Reset audio UI
        const audioWrap = document.getElementById('audioTableWrap');
        if (audioWrap) audioWrap.innerHTML = '<div class="state-message" id="audioEmptyState"><div class="state-icon">&#127925;</div><h2>Audio Sample Index</h2><p>Click <strong>"Scan Samples"</strong> to find all audio files.</p></div>';
        const audioStats = document.getElementById('audioStats');
        if (audioStats) audioStats.style.display = 'none';
        // Reset DAW UI
        const dawWrap = document.getElementById('dawTableWrap');
        if (dawWrap) dawWrap.innerHTML = '<div class="state-message" id="dawEmptyState"><div class="state-icon">&#127911;</div><h2>DAW Project Scanner</h2><p>Click <strong>"Scan DAW Projects"</strong> to find project files.</p></div>';
        const dawStats = document.getElementById('dawStats');
        if (dawStats) dawStats.style.display = 'none';
        // Reset preset UI
        const presetWrap = document.getElementById('presetTableWrap');
        if (presetWrap) presetWrap.innerHTML = '<div class="state-message" id="presetEmptyState"><div class="state-icon">&#127924;</div><h2>Preset Scanner</h2><p>Click <strong>"Scan Presets"</strong> to find preset files.</p></div>';
        const presetStats = document.getElementById('presetStats');
        if (presetStats) presetStats.style.display = 'none';
        // Reset history
        if (typeof loadHistory === 'function') loadHistory();
        showToast(toastFmt('toast.all_scans_reset_fresh'));
    } catch (e) {
        showToast(toastFmt('toast.reset_failed', {err: e.message || e}), 4000, 'error');
    } finally {
        hideGlobalProgress();
    }
}

async function settingClearKvrCache() {
    if (!await confirmAction('Clear all cached KVR version lookups? Next update check will re-fetch everything.', 'Clear KVR Cache')) return;
    showGlobalProgress();
    try {
        await window.vstUpdater.updateKvrCache([]);
        showToast(toastFmt('toast.kvr_cache_cleared_palette'));
        invalidateDbCacheStatsSnapshot();
        void renderCacheStats();
    } catch (e) {
        showToast(toastFmt('toast.failed_clear_kvr', {err: e.message || e}), 4000, 'error');
    } finally {
        hideGlobalProgress();
    }
}

function settingToggleAutoAnalysis() {
    const current = prefs.getItem('autoAnalysis') === 'on';
    const next = !current;
    prefs.setItem('autoAnalysis', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_analysis_launch', {state: next ? 'on' : 'off'}));
    if (next && typeof startBackgroundAnalysis === 'function') void startBackgroundAnalysis();
    refreshSettingsUI();
}

function settingToggleAutoContentDupScan() {
    const current = prefs.getItem('autoContentDupScan') === 'on';
    const next = !current;
    prefs.setItem('autoContentDupScan', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_content_dup_launch', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleAutoFingerprintCache() {
    const current = prefs.getItem('autoFingerprintCache') === 'on';
    const next = !current;
    prefs.setItem('autoFingerprintCache', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_fingerprint_cache_launch', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleAutoPdfScanOnStartup() {
    const current = prefs.getItem('autoPdfScanOnStartup') === 'on';
    const next = !current;
    prefs.setItem('autoPdfScanOnStartup', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_pdf_scan_launch', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleAutoPdfMetadataOnStartup() {
    const current = prefs.getItem('autoPdfMetadataOnStartup') === 'on';
    const next = !current;
    prefs.setItem('autoPdfMetadataOnStartup', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_pdf_metadata_startup', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleAutoCheckUpdatesOnStartup() {
    const current = prefs.getItem('autoCheckUpdatesOnStartup') === 'on';
    const next = !current;
    prefs.setItem('autoCheckUpdatesOnStartup', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_kvr_check_startup', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingTogglePdfMetadataAutoExtract() {
    const current = prefs.getItem('pdfMetadataAutoExtract') !== 'off';
    const next = !current;
    prefs.setItem('pdfMetadataAutoExtract', next ? 'on' : 'off');
    showToast(toastFmt('toast.pdf_metadata_auto_extract', {state: next ? 'on' : 'off'}));
    if (!next && typeof abortPdfMetadataExtraction === 'function') void abortPdfMetadataExtraction();
    if (next && typeof loadPdfPagesForVisible === 'function') void loadPdfPagesForVisible();
    refreshSettingsUI();
}

function settingToggleAutoScan() {
    const current = prefs.getItem('autoScan') === 'on';
    const next = !current;
    prefs.setItem('autoScan', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_scan_launch', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleIncrementalDirectoryScan() {
    const current = prefs.getItem('incrementalDirectoryScan') !== 'off';
    const next = !current;
    prefs.setItem('incrementalDirectoryScan', next ? 'on' : 'off');
    showToast(toastFmt('toast.incremental_directory_scan', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleFolderWatch() {
    const current = prefs.getItem('folderWatch') === 'on';
    const next = !current;
    prefs.setItem('folderWatch', next ? 'on' : 'off');
    if (next) {
        /* One toast from `startFolderWatch` lists paths so “Watching 2” is explainable */
        startFolderWatch({quiet: false});
    } else {
        window.vstUpdater.stopFileWatcher().catch(() => showToast(toastFmt('toast.failed_stop_watcher'), 4000, 'error'));
        showToast(toastFmt('toast.folder_watch_off'));
    }
    refreshSettingsUI();
}

/** Trim trailing slashes so `/a` and `/a/` dedupe; keeps POSIX `/` root as `/`. */
function normalizeFolderWatchPath(p) {
    if (p == null || typeof p !== 'string') return '';
    let s = p.trim();
    while (s.length > 1 && (s.endsWith('/') || s.endsWith('\\'))) {
        s = s.slice(0, -1);
    }
    return s;
}

/** Roots for the file watcher: explicit `folderWatchDirs` if any lines set; else plugin + library scan prefs. Uniqued after normalization. */
function getFolderWatchScanDirsUnique() {
    const raw = [];
    const explicit = prefs.getItem('folderWatchDirs');
    if (explicit && explicit.trim()) {
        raw.push(...explicit.split('\n').map(d => d.trim()).filter(Boolean));
    } else {
        for (const key of ['customDirs', 'audioScanDirs', 'dawScanDirs', 'presetScanDirs', 'midiScanDirs', 'pdfScanDirs', 'videoScanDirs']) {
            const val = prefs.getItem(key);
            if (val) raw.push(...val.split('\n').map(d => d.trim()).filter(Boolean));
        }
    }
    const seen = new Set();
    const out = [];
    for (const p of raw) {
        const n = normalizeFolderWatchPath(p);
        if (!n || seen.has(n)) continue;
        seen.add(n);
        out.push(n);
    }
    return out;
}

function showFolderWatchStartedToast(paths) {
    const n = paths.length;
    const head = toastFmt('toast.watching_n_dirs', {n});
    const body = paths.join('\n');
    const ms = Math.min(14_000, 3800 + n * 550);
    showToast(`${head}\n${body}`, ms, '', 'toast-mono-paths');
}

/** @param {{quiet?: boolean}} [opts] — `quiet` skips success toast (e.g. app launch). */
function startFolderWatch(opts = {}) {
    const quiet = opts.quiet === true;
    const unique = getFolderWatchScanDirsUnique();
    if (unique.length === 0) {
        showToast(toastFmt('toast.no_scan_dirs'), 3000, 'error');
        if (prefs.getItem('folderWatch') === 'on') {
            prefs.setItem('folderWatch', 'off');
            if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
        }
        return;
    }
    window.vstUpdater.startFileWatcher(unique).then(() => {
        if (!quiet) showFolderWatchStartedToast(unique);
    }).catch(e => showToast(toastFmt('toast.watch_failed', {err: e}), 4000, 'error'));
}

function settingToggleAutoUpdate() {
    const current = prefs.getItem('autoUpdate') === 'on';
    const next = !current;
    prefs.setItem('autoUpdate', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_check_updates', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleSingleClickPlay() {
    const sc = prefs.getItem('singleClickPlay');
    const current = sc !== 'off' && sc !== 'false';
    const next = !current;
    prefs.setItem('singleClickPlay', next ? 'on' : 'off');
    showToast(toastFmt('toast.single_click_play', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleAutoPlaySampleOnSelect() {
    const ap = prefs.getItem('autoPlaySampleOnSelect');
    const current = ap !== 'off' && ap !== 'false';
    const next = !current;
    prefs.setItem('autoPlaySampleOnSelect', next ? 'on' : 'off');
    showToast(toastFmt('toast.auto_play_sample_on_select', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingToggleAutoplayNext() {
    const current = prefs.getItem('autoplayNext');
    const next = current === 'off' ? 'on' : 'off';
    prefs.setItem('autoplayNext', next);
    showToast(toastFmt('toast.autoplay_next_state', {next}));
    refreshSettingsUI();
}

/** `autoplayNextSource`: `player` or `samples`. */
function settingSetAutoplayNextSource(val) {
    const v = val === 'player' ? 'player' : 'samples';
    prefs.setItem('autoplayNextSource', v);
    const srcLabel =
        typeof appFmt === 'function'
            ? appFmt(v === 'player' ? 'ui.autoplay_next_source.player' : 'ui.autoplay_next_source.samples')
            : v;
    showToast(toastFmt('toast.autoplay_next_source', {source: srcLabel}));
    refreshSettingsUI();
}

/** `trayTransportSource`: `player` or `samples` — which list the tray popover's prev/next buttons walk.
 *  Independent of `autoplayNextSource` (which controls EOF autoplay), so the user can have the main
 *  window auto-advance through the Samples table while using the tray popover to step through the
 *  recently-played player history, or vice versa. Default `samples`. */
function settingSetTrayTransportSource(val) {
    const v = val === 'player' ? 'player' : 'samples';
    prefs.setItem('trayTransportSource', v);
    const srcLabel =
        typeof appFmt === 'function'
            ? appFmt(v === 'player' ? 'ui.autoplay_next_source.player' : 'ui.autoplay_next_source.samples')
            : v;
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.tray_transport_source', {source: srcLabel}));
    }
    if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
}

/** `videoAudioRoute`: `engine` (default) or `html5` — see `videoPlaybackUsesEngineAudio` in `video.js`. */
function settingSetVideoAudioRoute(val) {
    const v = val === 'html5' ? 'html5' : 'engine';
    prefs.setItem('videoAudioRoute', v);
    const routeLabel =
        typeof appFmt === 'function'
            ? appFmt(v === 'html5' ? 'ui.opt.video_audio_route_html5' : 'ui.opt.video_audio_route_engine')
            : v;
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.video_audio_route', {route: routeLabel}));
    }
    if (typeof refreshSettingsUI === 'function') refreshSettingsUI();
}

function settingToggleExpandOnClick() {
    const current = prefs.getItem('expandOnClick');
    const next = current === 'off' ? 'on' : 'off';
    prefs.setItem('expandOnClick', next);
    showToast(toastFmt('toast.expand_on_click', {next}));
    refreshSettingsUI();
}

function settingToggleIncludeBackups() {
    const current = prefs.getItem('includeAbletonBackups') === 'on';
    const next = !current;
    prefs.setItem('includeAbletonBackups', next ? 'on' : 'off');
    showToast(toastFmt('toast.include_ableton_backups', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingTogglePruneOldScans() {
    const po = prefs.getItem('pruneOldScans');
    const current = po === 'on' || po === 'true';
    const next = !current;
    prefs.setItem('pruneOldScans', next ? 'on' : 'off');
    showToast(toastFmt('toast.prune_old_scans', {state: next ? 'on' : 'off'}));
    refreshSettingsUI();
}

function settingUpdatePruneOldScansKeep(val) {
    const n = Math.min(100, Math.max(1, parseInt(String(val), 10) || 3));
    prefs.setItem('pruneOldScansKeep', String(n));
    const valEl = document.getElementById('settingPruneOldScansKeepValue');
    if (valEl) valEl.textContent = String(n);
    showToast(toastFmt('toast.prune_old_scans_keep_set', {val: n}));
}

function settingUpdatePageSize(val) {
    const pageSizeValEl = document.getElementById('settingPageSizeValue');
    if (pageSizeValEl) pageSizeValEl.textContent = val;
    const pageSizeEl = document.getElementById('settingPageSize');
    if (pageSizeEl) pageSizeEl.value = val;
    prefs.setItem('pageSize', val);
    const n = parseInt(val, 10);
    if (!Number.isNaN(n)) {
        AUDIO_PAGE_SIZE = n;
        DAW_PAGE_SIZE = n;
        PRESET_PAGE_SIZE = n;
        MIDI_PAGE_SIZE = n;
        PDF_PAGE_SIZE = n;
        VIDEO_PAGE_SIZE = n;
    }
}

/** Step table page size (Performance). Safe when Settings tab is not in the DOM (e.g. Cmd+K). */
function paletteNudgeTablePageSize(delta) {
    const cur = parseInt(String(prefs.getItem('pageSize') || '200'), 10);
    const base = Number.isNaN(cur) ? 200 : cur;
    const next = Math.min(2000, Math.max(100, base + delta));
    settingUpdatePageSize(String(next));
    if (typeof showToast === 'function') {
        showToast(toastFmt('toast.table_page_size_nudged', {val: next}));
    }
}

/** Step prune retention count (Performance). */
function paletteNudgePruneKeep(delta) {
    const cur = parseInt(String(prefs.getItem('pruneOldScansKeep') || '3'), 10);
    const base = Number.isNaN(cur) ? 3 : cur;
    const next = Math.min(100, Math.max(1, base + delta));
    settingUpdatePruneOldScansKeep(String(next));
}

/** Cycle quiet → normal → verbose (logging). */
function paletteCycleLogVerbosity() {
    const order = ['quiet', 'normal', 'verbose'];
    const cur = prefs.getItem('logVerbosity') || 'normal';
    let idx = order.indexOf(cur);
    if (idx < 0) idx = 1;
    const next = order[(idx + 1) % order.length];
    prefs.setItem('logVerbosity', next);
    const sel = document.getElementById('settingLogVerbosity');
    if (sel) sel.value = next;
    if (typeof showToast === 'function') showToast(toastFmt('toast.log_verbosity_saved'));
}

function settingUpdateFlushInterval(val) {
    document.getElementById('settingFlushIntervalValue').textContent = val;
    prefs.setItem('flushInterval', val);
}

function settingUpdateTooltipHoverDelay(val) {
    const n = Math.min(5000, Math.max(0, parseInt(String(val), 10) || 0));
    const valEl = document.getElementById('settingTooltipHoverDelayValue');
    const rangeEl = document.getElementById('settingTooltipHoverDelay');
    if (valEl) valEl.textContent = `${n} ms`;
    if (rangeEl) rangeEl.value = String(n);
    prefs.setItem('tooltipHoverDelayMs', String(n));
}

function settingUpdateThreadMultiplier(val) {
    document.getElementById('settingThreadMultiplierValue').textContent = val + 'x';
    prefs.setItem('threadMultiplier', val);
    showToast(toastFmt('toast.thread_multiplier_set', {val}));
}

function settingUpdateBatchAnalysisThreads(val) {
    const n = Math.min(16, Math.max(1, parseInt(String(val), 10) || 4));
    const rangeEl = document.getElementById('settingBatchAnalysisThreads');
    const valEl = document.getElementById('settingBatchAnalysisThreadsValue');
    if (rangeEl) rangeEl.value = String(n);
    if (valEl) valEl.textContent = String(n);
    prefs.setItem('batchAnalysisThreads', String(n));
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.batch_analysis_threads_set', {val: n}));
    }
}

function settingUpdateContentDupHashThreads(val) {
    const n = Math.min(32, Math.max(1, parseInt(String(val), 10) || 8));
    const rangeEl = document.getElementById('settingContentDupHashThreads');
    const valEl = document.getElementById('settingContentDupHashThreadsValue');
    if (rangeEl) rangeEl.value = String(n);
    if (valEl) valEl.textContent = String(n);
    prefs.setItem('contentDupHashThreads', String(n));
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.content_dup_hash_threads_set', {val: n}));
    }
}

function settingUpdateSqliteReadPoolExtra(val) {
    const n = parseInt(val, 10);
    const raw = (n === 0 || Number.isNaN(n)) ? 'auto' : String(n);
    prefs.setItem('sqliteReadPoolExtra', raw);
    const valEl = document.getElementById('settingSqliteReadPoolExtraValue');
    if (valEl) {
        valEl.textContent = (n === 0 || Number.isNaN(n))
            ? (typeof catalogFmt === 'function' ? catalogFmt('ui.perf.sqlite_read_pool_auto_label') : 'Auto')
            : String(n);
    }
    const display = (n === 0 || Number.isNaN(n))
        ? (typeof catalogFmt === 'function' ? catalogFmt('ui.perf.sqlite_read_pool_auto_label') : 'Auto')
        : String(n);
    showToast(toastFmt('toast.sqlite_read_pool_extra_set', {val: display}));
}

function settingUpdateBgJobThrottle(val) {
    const n = Math.min(3, Math.max(0, parseInt(String(val), 10) || 3));
    const selectEl = document.getElementById('settingBgJobThrottle');
    if (selectEl) selectEl.value = String(n);
    prefs.setItem('bgJobThrottle', String(n));
    // Apply immediately to Rust
    if (typeof window.vstUpdater?.setBgJobThrottle === 'function') {
        window.vstUpdater.setBgJobThrottle(n).catch(() => {});
    }
    const labels = ['Off', 'Light', 'Medium', 'Full Pause'];
    if (typeof showToast === 'function' && typeof toastFmt === 'function') {
        showToast(toastFmt('toast.bg_job_throttle_set', {val: labels[n] || 'Full Pause'}));
    }
}

function settingUpdateChannelBuffer(val) {
    document.getElementById('settingChannelBufferValue').textContent = val;
    prefs.setItem('channelBuffer', val);
    showToast(toastFmt('toast.channel_buffer_set', {val}));
}

function settingUpdateBatchSize(val) {
    document.getElementById('settingBatchSizeValue').textContent = val;
    prefs.setItem('batchSize', val);
    showToast(toastFmt('toast.batch_size_set', {val}));
}

function settingUpdateFdLimit(val) {
    document.getElementById('settingFdLimitValue').textContent = val;
    prefs.setItem('fdLimit', val);
    showToast(toastFmt('toast.fd_limit_set', {val}));
}

function settingUpdateVizFps(val) {
    document.getElementById('settingVizFpsValue').textContent = val;
    prefs.setItem('vizFps', val);
    if (typeof _VIZ_FPS_SINGLE !== 'undefined') _VIZ_FPS_SINGLE = parseInt(val);
    if (typeof _VIZ_FPS_ALL !== 'undefined') _VIZ_FPS_ALL = Math.max(10, parseInt(val) - 10);
}

function settingUpdateWfCacheMax(val) {
    document.getElementById('settingWfCacheMaxValue').textContent = val;
    prefs.setItem('wfCacheMax', val);
    if (typeof _WF_CACHE_MAX !== 'undefined') _WF_CACHE_MAX = parseInt(val);
}

function settingUpdateAnalysisPause(val) {
    document.getElementById('settingAnalysisPauseValue').textContent = val;
    prefs.setItem('analysisPause', val);
}

function settingUpdateMaxRecent(val) {
    document.getElementById('settingMaxRecentValue').textContent = val;
    prefs.setItem('maxRecent', val);
    if (typeof MAX_RECENT !== 'undefined') MAX_RECENT = parseInt(val);
}

async function settingToggleFileWatcher() {
    const current = prefs.getItem('fileWatcher') === 'on';
    const next = !current;
    prefs.setItem('fileWatcher', next ? 'on' : 'off');
    if (next) {
        const unique = getFolderWatchScanDirsUnique();
        if (unique.length === 0) {
            showToast(toastFmt('toast.no_scan_dirs'), 3000, 'error');
            prefs.setItem('fileWatcher', 'off');
            refreshSettingsUI();
            return;
        }
        try {
            await window.vstUpdater.startFileWatcher(unique);
            showFolderWatchStartedToast(unique);
        } catch (e) {
            showToast(toastFmt('toast.watcher_failed', {err: e}), 4000, 'error');
            prefs.setItem('fileWatcher', 'off');
        }
    } else {
        try {
            await window.vstUpdater.stopFileWatcher();
        } catch (e) {
            if (typeof showToast === 'function' && e) showToast(String(e), 4000, 'error');
        }
        showToast(toastFmt('toast.file_watcher_stopped'));
    }
    refreshSettingsUI();
}

function settingSaveSelect(key, value) {
    prefs.setItem(key, value);
}

function showSavedMsg(id) {
    const el = document.getElementById(id);
    if (!el) return;
    el.textContent = catalogFmt('ui.settings.saved');
    el.classList.add('visible');
    setTimeout(() => el.classList.remove('visible'), 2000);
}

async function browseDir(targetId) {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi || !dialogApi.open) {
        showToast(toastFmt('toast.dialog_api_unavailable'), 3000, 'error');
        return;
    }
    const selected = await dialogApi.open({
        directory: true,
        multiple: false,
        title: catalogFmt('ui.settings.dialog_select_folder'),
    });
    if (!selected) return;
    const textarea = document.getElementById(targetId);
    if (!textarea) return;
    const existing = textarea.value.trim();
    // Append if not already present
    const lines = existing ? existing.split('\n').map(s => s.trim()).filter(Boolean) : [];
    if (!lines.includes(selected)) {
        lines.push(selected);
        textarea.value = lines.join('\n');
    }
    showToast(toastFmt('toast.added_selected', {selected}));
}

async function browseSnapshotExportDir() {
    const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
    if (!dialogApi || !dialogApi.open) {
        showToast(toastFmt('toast.dialog_api_unavailable'), 3000, 'error');
        return;
    }
    const input = document.getElementById('settingSnapshotExportDir');
    let cur = input && input.value.trim();
    if (!cur && typeof window.vstUpdater?.ensureSnapshotExportDir === 'function') {
        try {
            cur = await window.vstUpdater.ensureSnapshotExportDir();
        } catch {
            cur = '';
        }
    }
    const selected = await dialogApi.open({
        directory: true,
        multiple: false,
        title: catalogFmt('ui.settings.dialog_select_folder'),
        defaultPath: cur || undefined,
    });
    if (!selected) return;
    if (input) input.value = selected;
    prefs.setItem('snapshotExportDir', selected);
    showToast(toastFmt('toast.snapshot_export_dir_saved'));
}

function saveSnapshotExportDir() {
    const input = document.getElementById('settingSnapshotExportDir');
    if (!input) return;
    prefs.setItem('snapshotExportDir', input.value.trim());
    showToast(toastFmt('toast.snapshot_export_dir_saved'));
}

function clearSnapshotExportDir() {
    const input = document.getElementById('settingSnapshotExportDir');
    if (input) input.value = '';
    prefs.removeItem('snapshotExportDir');
    showToast(toastFmt('toast.snapshot_export_dir_cleared'));
}

function saveCustomDirs() {
    const val = document.getElementById('settingCustomDirs').value.trim();
    prefs.setItem('customDirs', val);
    showSavedMsg('savedMsgCustomDirs');
    showToast(toastFmt('toast.plugin_scan_dirs_saved'));
}

function saveAudioScanDirs() {
    const val = document.getElementById('settingAudioScanDirs').value.trim();
    prefs.setItem('audioScanDirs', val);
    showSavedMsg('savedMsgAudioScanDirs');
    showToast(toastFmt('toast.audio_scan_dirs_saved'));
}

function saveDawScanDirs() {
    const val = document.getElementById('settingDawScanDirs').value.trim();
    prefs.setItem('dawScanDirs', val);
    showSavedMsg('savedMsgDawScanDirs');
    showToast(toastFmt('toast.daw_scan_dirs_saved'));
}

function savePresetScanDirs() {
    const val = document.getElementById('settingPresetScanDirs').value.trim();
    prefs.setItem('presetScanDirs', val);
    showSavedMsg('savedMsgPresetScanDirs');
    showToast(toastFmt('toast.preset_scan_dirs_saved'));
}

function saveMidiScanDirs() {
    const val = document.getElementById('settingMidiScanDirs').value.trim();
    prefs.setItem('midiScanDirs', val);
    showSavedMsg('savedMsgMidiScanDirs');
    if (typeof showToast === 'function') showToast(catalogFmt('toast.midi_scan_dirs_saved'));
}

function savePdfScanDirs() {
    const val = document.getElementById('settingPdfScanDirs').value.trim();
    prefs.setItem('pdfScanDirs', val);
    showSavedMsg('savedMsgPdfScanDirs');
    showToast(toastFmt('toast.pdf_scan_dirs_saved'));
}

function saveVideoScanDirs() {
    const val = document.getElementById('settingVideoScanDirs').value.trim();
    prefs.setItem('videoScanDirs', val);
    showSavedMsg('savedMsgVideoScanDirs');
    showToast(toastFmt('toast.video_scan_dirs_saved'));
}

function saveFolderWatchDirs() {
    const el = document.getElementById('settingFolderWatchDirs');
    const val = el ? el.value.trim() : '';
    prefs.setItem('folderWatchDirs', val);
    showSavedMsg('savedMsgFolderWatchDirs');
    showToast(toastFmt('toast.folder_watch_dirs_saved'));
    if (prefs.getItem('folderWatch') === 'on' && typeof startFolderWatch === 'function') {
        startFolderWatch({quiet: true});
    }
}

function openPrefsFile() {
    window.vstUpdater.openPrefsFile().catch(e => showToast(toastFmt('toast.failed_open_prefs_file', {err: e.message || e}), 4000, 'error'));
}

function getSettingValue(key, defaultVal) {
    return prefs.getItem(key) || defaultVal;
}

/** Settings → About: version line (uses `ui.settings.about_version_line` + `ui.logo.author_line`). */
function updateSettingsAboutVersionLine() {
    const line = document.getElementById('settingsAboutVersionLine');
    if (!line) return;
    const ver = document.getElementById('appVersion')?.textContent?.trim() || '';
    const author = catalogFmt('ui.logo.author_line');
    line.textContent = catalogFmt('ui.settings.about_version_line', {version: ver, author});
}

window.updateSettingsAboutVersionLine = updateSettingsAboutVersionLine;
window.invalidateDbCacheStatsSnapshot = invalidateDbCacheStatsSnapshot;

function refreshSettingsUI() {
    // Theme
    const theme = document.documentElement.getAttribute('data-theme') || 'dark';
    const themeBtn = document.getElementById('settingTheme');
    const themeLabel = document.getElementById('settingThemeLabel');
    themeBtn.classList.toggle('active', theme === 'light');
    themeLabel.textContent = _uiTheme(theme === 'light');

    // CRT
    const crtOn = prefs.getItem('crtEffects') !== 'off';
    const crtBtn = document.getElementById('settingCrt');
    const crtLabel = document.getElementById('settingCrtLabel');
    crtBtn.classList.toggle('active', crtOn);
    crtLabel.textContent = _uiToggle(crtOn);

    // Neon glow
    const neonOn = prefs.getItem('neonGlow') !== 'off';
    const neonBtn = document.getElementById('settingNeonGlow');
    const neonLabel = document.getElementById('settingNeonGlowLabel');
    if (neonBtn) {
        neonBtn.classList.toggle('active', neonOn);
    }
    if (neonLabel) {
        neonLabel.textContent = _uiToggle(neonOn);
    }

    // Tag bar
    const tagBarOn = prefs.getItem('tagBarVisible') !== 'off';
    const tagBarBtn = document.getElementById('settingTagBar');
    const tagBarLabel = document.getElementById('settingTagBarLabel');
    if (tagBarBtn) {
        tagBarBtn.classList.toggle('active', tagBarOn);
    }
    if (tagBarLabel) {
        tagBarLabel.textContent = _uiToggle(tagBarOn);
    }
    const tagPosEl = document.getElementById('settingTagBarPosition');
    if (tagPosEl) tagPosEl.value = prefs.getItem('tagBarPosition') || 'top';

    const ttDelayRaw = parseInt(String(prefs.getItem('tooltipHoverDelayMs') || '600'), 10);
    const ttN = Number.isNaN(ttDelayRaw) ? 600 : Math.min(5000, Math.max(0, ttDelayRaw));
    const ttRangeEl = document.getElementById('settingTooltipHoverDelay');
    const ttValEl = document.getElementById('settingTooltipHoverDelayValue');
    if (ttRangeEl) ttRangeEl.value = String(ttN);
    if (ttValEl) ttValEl.textContent = `${ttN} ms`;

    // Color scheme — rebuild labels for locale, then highlight active
    const currentScheme = prefs.getItem('colorScheme') || 'cyberpunk';
    rebuildSchemeButtons();
    document.querySelectorAll('.scheme-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.scheme === currentScheme);
    });

    // Auto-scan
    const autoScan = prefs.getItem('autoScan') === 'on';
    const autoScanBtn = document.getElementById('settingAutoScan');
    const autoScanLabel = document.getElementById('settingAutoScanLabel');
    if (autoScanBtn) {
        autoScanBtn.classList.toggle('active', autoScan);
        autoScanLabel.textContent = _uiToggle(autoScan);
    }

    // Auto-analysis
    const autoAnalysis = prefs.getItem('autoAnalysis') === 'on';
    const autoAnalysisBtn = document.getElementById('settingAutoAnalysis');
    const autoAnalysisLabel = document.getElementById('settingAutoAnalysisLabel');
    if (autoAnalysisBtn) {
        autoAnalysisBtn.classList.toggle('active', autoAnalysis);
        autoAnalysisLabel.textContent = _uiToggle(autoAnalysis);
    }

    const autoContentDup = prefs.getItem('autoContentDupScan') === 'on';
    const autoContentDupBtn = document.getElementById('settingAutoContentDupScan');
    const autoContentDupLabel = document.getElementById('settingAutoContentDupScanLabel');
    if (autoContentDupBtn) {
        autoContentDupBtn.classList.toggle('active', autoContentDup);
        if (autoContentDupLabel) autoContentDupLabel.textContent = _uiToggle(autoContentDup);
    }

    const autoFp = prefs.getItem('autoFingerprintCache') === 'on';
    const autoFpBtn = document.getElementById('settingAutoFingerprintCache');
    const autoFpLabel = document.getElementById('settingAutoFingerprintCacheLabel');
    if (autoFpBtn) {
        autoFpBtn.classList.toggle('active', autoFp);
        if (autoFpLabel) autoFpLabel.textContent = _uiToggle(autoFp);
    }

    const autoPdfScan = prefs.getItem('autoPdfScanOnStartup') === 'on';
    const autoPdfScanBtn = document.getElementById('settingAutoPdfScanOnStartup');
    const autoPdfScanLabel = document.getElementById('settingAutoPdfScanOnStartupLabel');
    if (autoPdfScanBtn) {
        autoPdfScanBtn.classList.toggle('active', autoPdfScan);
        if (autoPdfScanLabel) autoPdfScanLabel.textContent = _uiToggle(autoPdfScan);
    }

    const autoPdfMetaStartup = prefs.getItem('autoPdfMetadataOnStartup') === 'on';
    const autoPdfMetaStartupBtn = document.getElementById('settingAutoPdfMetadataOnStartup');
    const autoPdfMetaStartupLabel = document.getElementById('settingAutoPdfMetadataOnStartupLabel');
    if (autoPdfMetaStartupBtn) {
        autoPdfMetaStartupBtn.classList.toggle('active', autoPdfMetaStartup);
        if (autoPdfMetaStartupLabel) autoPdfMetaStartupLabel.textContent = _uiToggle(autoPdfMetaStartup);
    }

    const autoKvrStartup = prefs.getItem('autoCheckUpdatesOnStartup') === 'on';
    const autoKvrStartupBtn = document.getElementById('settingAutoCheckUpdatesOnStartup');
    const autoKvrStartupLabel = document.getElementById('settingAutoCheckUpdatesOnStartupLabel');
    if (autoKvrStartupBtn) {
        autoKvrStartupBtn.classList.toggle('active', autoKvrStartup);
        if (autoKvrStartupLabel) autoKvrStartupLabel.textContent = _uiToggle(autoKvrStartup);
    }

    // Background PDF metadata (page counts) while PDF tab is active
    const pdfMetaAuto = prefs.getItem('pdfMetadataAutoExtract') !== 'off';
    const pdfMetaAutoBtn = document.getElementById('settingPdfMetadataAutoExtract');
    const pdfMetaAutoLabel = document.getElementById('settingPdfMetadataAutoExtractLabel');
    if (pdfMetaAutoBtn) {
        pdfMetaAutoBtn.classList.toggle('active', pdfMetaAuto);
    }
    if (pdfMetaAutoLabel) {
        pdfMetaAutoLabel.textContent = _uiToggle(pdfMetaAuto);
    }

    // Folder watch
    const folderWatch = prefs.getItem('folderWatch') === 'on';
    const fwBtn = document.getElementById('settingFolderWatch');
    const fwLabel = document.getElementById('settingFolderWatchLabel');
    if (fwBtn) {
        fwBtn.classList.toggle('active', folderWatch);
    }
    if (fwLabel) {
        fwLabel.textContent = _uiToggle(folderWatch);
    }

    // Incremental unified scan (default on — matches config.default.toml)
    const incrementalScan = prefs.getItem('incrementalDirectoryScan') !== 'off';
    const incBtn = document.getElementById('settingIncrementalDirectoryScan');
    const incLabel = document.getElementById('settingIncrementalDirectoryScanLabel');
    if (incBtn) {
        incBtn.classList.toggle('active', incrementalScan);
    }
    if (incLabel) {
        incLabel.textContent = _uiToggle(incrementalScan);
    }

    // Auto-update
    const autoUpdate = prefs.getItem('autoUpdate') === 'on';
    const autoUpdateBtn = document.getElementById('settingAutoUpdate');
    const autoUpdateLabel = document.getElementById('settingAutoUpdateLabel');
    if (autoUpdateBtn) {
        autoUpdateBtn.classList.toggle('active', autoUpdate);
        autoUpdateLabel.textContent = _uiToggle(autoUpdate);
    }

    // Single-click play (treat null as on — matches merged defaults and pre-load race)
    const scPref = prefs.getItem('singleClickPlay');
    const singleClick = scPref !== 'off' && scPref !== 'false';
    const singleClickBtn = document.getElementById('settingSingleClickPlay');
    const singleClickLabel = document.getElementById('settingSingleClickPlayLabel');
    if (singleClickBtn) {
        singleClickBtn.classList.toggle('active', singleClick);
        singleClickLabel.textContent = _uiToggle(singleClick);
    }

    const apPref = prefs.getItem('autoPlaySampleOnSelect');
    const autoPlayOnSelect = apPref !== 'off' && apPref !== 'false';
    const autoPlayOnSelectBtn = document.getElementById('settingAutoPlaySampleOnSelect');
    const autoPlayOnSelectLabel = document.getElementById('settingAutoPlaySampleOnSelectLabel');
    if (autoPlayOnSelectBtn) {
        autoPlayOnSelectBtn.classList.toggle('active', autoPlayOnSelect);
        autoPlayOnSelectLabel.textContent = _uiToggle(autoPlayOnSelect);
    }

    // Expand on click
    const expandOnClick = prefs.getItem('expandOnClick') !== 'off';
    const expandBtn = document.getElementById('settingExpandOnClick');
    const expandLabel = document.getElementById('settingExpandOnClickLabel');
    if (expandBtn) {
        expandBtn.classList.toggle('active', expandOnClick);
        expandLabel.textContent = _uiToggle(expandOnClick);
    }

    // Autoplay next (Settings + Command Palette)
    const autoplay = prefs.getItem('autoplayNext') !== 'off';
    const autoplayBtn = document.getElementById('settingAutoplayNext');
    const autoplayLabel = document.getElementById('settingAutoplayNextLabel');
    if (autoplayBtn) {
        autoplayBtn.classList.toggle('active', autoplay);
        autoplayLabel.textContent = _uiToggle(autoplay);
    }
    const apSrc =
        typeof getAutoplayNextSource === 'function'
            ? getAutoplayNextSource()
            : prefs.getItem('autoplayNextSource') === 'player'
                ? 'player'
                : 'samples';
    const settingApSrc = document.getElementById('settingAutoplayNextSource');
    if (settingApSrc) settingApSrc.value = apSrc;

    /* Tray popover prev/next source — independent of EOF autoplay source above. */
    const trayTransportSrc = prefs.getItem('trayTransportSource') === 'player' ? 'player' : 'samples';
    const settingTrayTransportSrc = document.getElementById('settingTrayTransportSource');
    if (settingTrayTransportSrc) settingTrayTransportSrc.value = trayTransportSrc;

    const videoAudioRt = prefs.getItem('videoAudioRoute') === 'html5' ? 'html5' : 'engine';
    const settingVideoAudioRt = document.getElementById('settingVideoAudioRoute');
    if (settingVideoAudioRt) settingVideoAudioRt.value = videoAudioRt;

    // Include Ableton backups
    const includeBackups = prefs.getItem('includeAbletonBackups') === 'on';
    const backupsBtn = document.getElementById('settingIncludeBackups');
    const backupsLabel = document.getElementById('settingIncludeBackupsLabel');
    if (backupsBtn) {
        backupsBtn.classList.toggle('active', includeBackups);
        backupsLabel.textContent = _uiToggle(includeBackups);
    }

    // Blacklist — prepopulate with defaults if empty
    const blacklistEl = document.getElementById('settingBlacklist');
    if (blacklistEl) {
        const saved = prefs.getItem('blacklistDirs');
        if (saved) {
            blacklistEl.value = saved;
        } else {
            blacklistEl.value = '#recycle\n@eaDir\n.Spotlight-V100\n$RECYCLE.BIN\nSystem Volume Information\nnode_modules\n.git\n.Trash\n__pycache__\n.cache';
        }
    }

    // Page size
    const pageSize = getSettingValue('pageSize', '200');
    const pageSizeEl = document.getElementById('settingPageSize');
    const pageSizeValEl = document.getElementById('settingPageSizeValue');
    if (pageSizeEl) {
        pageSizeEl.value = pageSize;
        pageSizeValEl.textContent = pageSize;
    }

    // Flush interval
    const flush = getSettingValue('flushInterval', '100');
    const flushEl = document.getElementById('settingFlushInterval');
    const flushValEl = document.getElementById('settingFlushIntervalValue');
    if (flushEl) {
        flushEl.value = flush;
        flushValEl.textContent = flush;
    }

    // Thread multiplier
    const threadMult = getSettingValue('threadMultiplier', '4');
    const threadMultEl = document.getElementById('settingThreadMultiplier');
    const threadMultValEl = document.getElementById('settingThreadMultiplierValue');
    if (threadMultEl) {
        threadMultEl.value = threadMult;
        threadMultValEl.textContent = threadMult + 'x';
    }

    const batchThrRaw = getSettingValue('batchAnalysisThreads', '4');
    let batchThrNum = parseInt(String(batchThrRaw), 10);
    if (Number.isNaN(batchThrNum)) batchThrNum = 4;
    batchThrNum = Math.min(16, Math.max(1, batchThrNum));
    const batchThrEl = document.getElementById('settingBatchAnalysisThreads');
    const batchThrValEl = document.getElementById('settingBatchAnalysisThreadsValue');
    if (batchThrEl) {
        batchThrEl.value = String(batchThrNum);
        if (batchThrValEl) batchThrValEl.textContent = String(batchThrNum);
    }

    const dupHashRaw = getSettingValue('contentDupHashThreads', '8');
    let dupHashNum = parseInt(String(dupHashRaw), 10);
    if (Number.isNaN(dupHashNum)) dupHashNum = 8;
    dupHashNum = Math.min(32, Math.max(1, dupHashNum));
    const dupHashEl = document.getElementById('settingContentDupHashThreads');
    const dupHashValEl = document.getElementById('settingContentDupHashThreadsValue');
    if (dupHashEl) {
        dupHashEl.value = String(dupHashNum);
        if (dupHashValEl) dupHashValEl.textContent = String(dupHashNum);
    }

    // Background job throttle (0=off, 1=light, 2=medium, 3=full pause)
    const bgThrottleRaw = getSettingValue('bgJobThrottle', '3');
    let bgThrottleNum = parseInt(String(bgThrottleRaw), 10);
    if (Number.isNaN(bgThrottleNum)) bgThrottleNum = 3;
    bgThrottleNum = Math.min(3, Math.max(0, bgThrottleNum));
    const bgThrottleEl = document.getElementById('settingBgJobThrottle');
    if (bgThrottleEl) {
        bgThrottleEl.value = String(bgThrottleNum);
    }
    // Apply to Rust on startup
    if (typeof window.vstUpdater?.setBgJobThrottle === 'function') {
        window.vstUpdater.setBgJobThrottle(bgThrottleNum).catch(() => {});
    }

    // SQLite read pool (0 = auto)
    const sqlitePoolRaw = getSettingValue('sqliteReadPoolExtra', 'auto');
    const sqlitePoolNum = (sqlitePoolRaw === 'auto' || sqlitePoolRaw === '' || sqlitePoolRaw == null)
        ? 0
        : parseInt(String(sqlitePoolRaw), 10);
    const sqlitePoolSlider = Number.isNaN(sqlitePoolNum) || sqlitePoolRaw === 'auto' ? 0 : Math.min(32, Math.max(0, sqlitePoolNum));
    const sqlitePoolEl = document.getElementById('settingSqliteReadPoolExtra');
    const sqlitePoolValEl = document.getElementById('settingSqliteReadPoolExtraValue');
    if (sqlitePoolEl) {
        sqlitePoolEl.value = String(sqlitePoolSlider);
        if (sqlitePoolValEl) {
            sqlitePoolValEl.textContent = sqlitePoolSlider === 0
                ? (typeof catalogFmt === 'function' ? catalogFmt('ui.perf.sqlite_read_pool_auto_label') : 'Auto')
                : String(sqlitePoolSlider);
        }
    }

    // Prune old scans (default off — explicit "on"/"true" enables)
    const pruneRaw = getSettingValue('pruneOldScans', 'off');
    const pruneOn = pruneRaw === 'on' || pruneRaw === 'true';
    const pruneBtn = document.getElementById('settingPruneOldScans');
    const pruneLabel = document.getElementById('settingPruneOldScansLabel');
    if (pruneBtn) {
        pruneBtn.classList.toggle('active', pruneOn);
        if (pruneLabel) pruneLabel.textContent = _uiToggle(pruneOn);
    }
    const pruneKeepRaw = getSettingValue('pruneOldScansKeep', '3');
    let pruneKeepNum = parseInt(String(pruneKeepRaw), 10);
    if (Number.isNaN(pruneKeepNum)) pruneKeepNum = 3;
    pruneKeepNum = Math.min(100, Math.max(1, pruneKeepNum));
    const pruneKeepEl = document.getElementById('settingPruneOldScansKeep');
    const pruneKeepValEl = document.getElementById('settingPruneOldScansKeepValue');
    if (pruneKeepEl) {
        pruneKeepEl.value = String(pruneKeepNum);
        if (pruneKeepValEl) pruneKeepValEl.textContent = String(pruneKeepNum);
    }

    // Channel buffer
    const chanBuf = getSettingValue('channelBuffer', '512');
    const chanBufEl = document.getElementById('settingChannelBuffer');
    const chanBufValEl = document.getElementById('settingChannelBufferValue');
    if (chanBufEl) {
        chanBufEl.value = chanBuf;
        chanBufValEl.textContent = chanBuf;
    }

    // Batch size
    const batchSz = getSettingValue('batchSize', '100');
    const batchSzEl = document.getElementById('settingBatchSize');
    const batchSzValEl = document.getElementById('settingBatchSizeValue');
    if (batchSzEl) {
        batchSzEl.value = batchSz;
        batchSzValEl.textContent = batchSz;
    }

    // FD limit
    const fdLimit = getSettingValue('fdLimit', '10240');
    const fdEl = document.getElementById('settingFdLimit');
    const fdValEl = document.getElementById('settingFdLimitValue');
    if (fdEl) {
        fdEl.value = fdLimit;
        fdValEl.textContent = fdLimit;
    }

    // Visualizer FPS
    const vizFps = getSettingValue('vizFps', '30');
    const vizFpsEl = document.getElementById('settingVizFps');
    const vizFpsValEl = document.getElementById('settingVizFpsValue');
    if (vizFpsEl) {
        vizFpsEl.value = vizFps;
        vizFpsValEl.textContent = vizFps;
    }

    const snapDirEl = document.getElementById('settingSnapshotExportDir');
    if (snapDirEl) snapDirEl.value = prefs.getItem('snapshotExportDir') || '';

    // Waveform cache max
    const wfMax = getSettingValue('wfCacheMax', '500');
    const wfMaxEl = document.getElementById('settingWfCacheMax');
    const wfMaxValEl = document.getElementById('settingWfCacheMaxValue');
    if (wfMaxEl) {
        wfMaxEl.value = wfMax;
        wfMaxValEl.textContent = wfMax;
    }

    // Analysis pause
    const aPause = getSettingValue('analysisPause', '3');
    const aPauseEl = document.getElementById('settingAnalysisPause');
    const aPauseValEl = document.getElementById('settingAnalysisPauseValue');
    if (aPauseEl) {
        aPauseEl.value = aPause;
        aPauseValEl.textContent = aPause;
    }

    // Max recently played
    const maxRec = getSettingValue('maxRecent', '50');
    const maxRecEl = document.getElementById('settingMaxRecent');
    const maxRecValEl = document.getElementById('settingMaxRecentValue');
    if (maxRecEl) {
        maxRecEl.value = maxRec;
        maxRecValEl.textContent = maxRec;
    }

    // File watcher
    const fwOn2 = prefs.getItem('fileWatcher') === 'on';
    const fwBtn2 = document.getElementById('settingFileWatcher');
    const fwLabel2 = document.getElementById('settingFileWatcherLabel');
    if (fwBtn2) {
        fwBtn2.classList.toggle('active', fwOn2);
        if (fwLabel2) fwLabel2.textContent = _uiToggle(fwOn2);
    }

    // System perf info — get real stats from Rust backend
    const perfInfo = document.getElementById('settingPerfInfo');
    if (perfInfo) {
        const f = catalogFmt;
        window.vstUpdater.getProcessStats().then(stats => {
            const cpus = stats.numCpus || navigator.hardwareConcurrency || '?';
            const fmtMem = (bytes) => {
                if (typeof formatBytes === 'function') return formatBytes(bytes);
                if (!bytes || bytes === 0) return '0 B';
                const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
                const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
                return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
            };
            const fmtUptimeSecs = (secs) => {
                if (typeof formatUptime === 'function') return formatUptime(secs);
                if (!secs) return '0s';
                const h = Math.floor(secs / 3600);
                const m = Math.floor((secs % 3600) / 60);
                const s = secs % 60;
                return (h ? h + 'h ' : '') + (m ? m + 'm ' : '') + s + 's';
            };
            const sc = stats.scanner || {};
            const cfg = stats.config || {};
            const df = stats.dataFiles || {};
            const db = stats.database || {};
            const tc = db.tables || {};
            // Library-first counts from SQLite (same as Database → library row). In-memory tab arrays
            // are capped (~100k) for UI performance and may be empty before a tab is opened — do not use
            // their .length here.
            const pluginCount = Number(tc.plugins_library ?? tc.plugins ?? (typeof allPlugins !== 'undefined' ? allPlugins.length : 0));
            const sampleCount = Number(tc.audio_samples_library ?? tc.audio_samples ?? (typeof allAudioSamples !== 'undefined' ? allAudioSamples.length : 0));
            const dawCount = Number(tc.daw_projects_library ?? tc.daw_projects ?? (typeof allDawProjects !== 'undefined' ? allDawProjects.length : 0));
            const presetCount = Number(tc.presets_library ?? tc.presets ?? (typeof allPresets !== 'undefined' ? allPresets.length : 0));
            const midiCount = Number(tc.midi_files_library ?? tc.midi_files ?? (typeof allMidiFiles !== 'undefined' ? allMidiFiles.length : 0));
            const pdfCount = Number(tc.pdfs_library ?? tc.pdfs ?? (typeof allPdfs !== 'undefined' ? allPdfs.length : 0));
            const dot = (on) => on ? '<span style="color:var(--green);">&#9679;</span>' : '<span style="color:var(--text-dim);">&#9675;</span>';
            const section = (titleKey, lines) => {
                const title = f(titleKey);
                return `<div style="margin-bottom:6px;"><span style="color:var(--cyan);font-weight:700;font-size:10px;text-transform:uppercase;letter-spacing:1px;">${escapeHtml(title)}</span><br>${lines.join('<br>')}</div>`;
            };
            perfInfo.innerHTML = [
                section('ui.perf.section_system', [
                    f('ui.perf.line_os_host', {
                        os: escapeHtml(stats.os || '?'),
                        arch: escapeHtml(stats.arch || '?'),
                        hostname: escapeHtml(stats.hostname || '?'),
                    }),
                    f('ui.perf.line_cpu', {cpus: String(cpus), cpu_percent: (stats.cpuPercent || 0).toFixed(1)}),
                    f('ui.perf.line_disk', {free: fmtMem(stats.diskFreeBytes), total: fmtMem(stats.diskTotalBytes)}),
                ]),
                section('ui.perf.section_process', [
                    f('ui.perf.line_pid_version', {pid: String(stats.pid), version: stats.appVersion || '?'}),
                    f('ui.perf.line_memory', {rss: fmtMem(stats.rssBytes), virt: fmtMem(stats.virtualBytes)}),
                    f('ui.perf.line_threads', {threads: String(stats.threads), rayon: String(stats.rayonThreads)}),
                    f('ui.perf.line_fds', {
                        open: String(stats.openFds),
                        soft: String(stats.fdSoftLimit),
                        hard: String(stats.fdHardLimit),
                    }),
                    f('ui.perf.line_uptime', {uptime: fmtUptimeSecs(stats.uptimeSecs)}),
                ]),
                section('ui.perf.section_thread_pools', [
                    f('ui.perf.line_rayon_global', {
                        n: String(stats.rayonThreads),
                        mult: String(cfg.threadMultiplier || '?')
                    }),
                    f('ui.perf.line_per_scanner', {
                        n: String(cfg.perScannerThreads || '?'),
                        stack: String(cfg.stackSize || '?')
                    }),
                    f('ui.perf.line_plugin_channel', {
                        buf: String(cfg.channelBuffer || '?'),
                        pmin: String(cfg.pluginChannelMin || '?'),
                        pmax: String(cfg.pluginChannelMax || '?'),
                        batch: String(cfg.batchSize || '?'),
                    }),
                    f('ui.perf.line_walker_channel', {
                        buf: String(cfg.walkerChannelBuffer || '?'),
                        batch: String(cfg.walkerBatchSize || '?'),
                        depth: String(cfg.depthLimit || '?'),
                        flush: String(cfg.flushInterval || '?'),
                    }),
                ]),
                section('ui.perf.section_scanner_state', [
                    `${dot(sc.pluginScanning)} ${f('ui.perf.scan_plugins')}  ${dot(sc.audioScanning)} ${f('ui.perf.scan_samples')}  ${dot(sc.dawScanning)} ${f('ui.perf.scan_daw')}  ${dot(sc.presetScanning)} ${f('ui.perf.scan_presets')}  ${dot(sc.midiScanning)} ${f('ui.perf.scan_midi')}  ${dot(sc.pdfScanning)} ${f('ui.perf.scan_pdf')}  ${dot(sc.videoScanning)} ${f('ui.perf.scan_video')}  ${dot(sc.updateChecking)} ${f('ui.perf.scan_updates')}`,
                ]),
                section('ui.perf.section_scan_results', [
                    f('ui.perf.line_scan_results', {
                        plugin_count: pluginCount.toLocaleString(),
                        sample_count: sampleCount.toLocaleString(),
                        daw_count: dawCount.toLocaleString(),
                        preset_count: presetCount.toLocaleString(),
                        midi_count: midiCount.toLocaleString(),
                        pdf_count: pdfCount.toLocaleString(),
                    }),
                ]),
                section('ui.perf.section_database', [
                    f('ui.perf.line_db_size', {
                        db_size: fmtMem(db.sizeBytes || 0),
                        prefs_size: fmtMem((df.preferencesBytes || 0))
                    }),
                    f('ui.perf.line_db_read_pool', {
                        total: String(db.sqliteReadPoolTotal ?? 0),
                        extra: String(db.sqliteReadPoolExtra ?? 0),
                        pref: escapeHtml(String(db.sqliteReadPoolExtraPref ?? 'auto')),
                    }),
                    f('ui.perf.line_db_tables', {
                        samples: (tc.audio_samples || 0).toLocaleString(),
                        plugins: (tc.plugins || 0).toLocaleString(),
                        daw: (tc.daw_projects || 0).toLocaleString(),
                        presets: (tc.presets || 0).toLocaleString(),
                        midi: (tc.midi_files || 0).toLocaleString(),
                        pdfs: (tc.pdfs || 0).toLocaleString(),
                    }),
                    f('ui.perf.line_db_library', {
                        samples: (tc.audio_samples_library ?? tc.audio_samples ?? 0).toLocaleString(),
                        plugins: (tc.plugins_library ?? tc.plugins ?? 0).toLocaleString(),
                        daw: (tc.daw_projects_library ?? tc.daw_projects ?? 0).toLocaleString(),
                        presets: (tc.presets_library ?? tc.presets ?? 0).toLocaleString(),
                        midi: (tc.midi_files_library ?? tc.midi_files ?? 0).toLocaleString(),
                        pdfs: (tc.pdfs_library ?? tc.pdfs ?? 0).toLocaleString(),
                    }),
                    f('ui.perf.line_db_caches', {
                        kvr: (tc.kvr_cache || 0).toLocaleString(),
                        waveforms: (tc.waveform_cache || 0).toLocaleString(),
                        spectrograms: (tc.spectrogram_cache || 0).toLocaleString(),
                        xref: (tc.xref_cache || 0).toLocaleString(),
                        fp: (tc.fingerprint_cache || 0).toLocaleString(),
                    }),
                    f('ui.perf.line_db_scans', {
                        plugin_scans: String(tc.plugin_scans || 0),
                        audio_scans: String(tc.audio_scans || 0),
                        daw_scans: String(tc.daw_scans || 0),
                        preset_scans: String(tc.preset_scans || 0),
                        midi_scans: String(tc.midi_scans || 0),
                        pdf_scans: String(tc.pdf_scans || 0),
                    }),
                ]),
                section('ui.perf.section_storage', [
                    f('ui.perf.line_data_dir', {
                        path: `<code style="font-size:10px;word-break:break-all;">${escapeHtml(stats.dataDir || '?')}</code>`,
                    }),
                ]),
            ].join('');
            // App Info pane
            const appInfo = document.getElementById('settingAppInfo');
            if (appInfo) {
                const app = stats.app || {};
                const tag = (items) => (items || []).map(i => `<span style="display:inline-block;background:var(--bg-card);border:1px solid var(--border);border-radius:2px;padding:1px 6px;margin:1px 2px;font-size:10px;">${escapeHtml(String(i))}</span>`).join('');
                appInfo.innerHTML = [
                    section('ui.perf.section_build', [
                        f('ui.perf.line_build_version', {
                            version: stats.appVersion || '?',
                            tauri: stats.tauriVersion || '?'
                        }),
                        f('ui.perf.line_build_target', {
                            target: stats.rustcTarget || '?',
                            profile: stats.buildProfile || '?'
                        }),
                        f('ui.perf.line_build_ui', {ui_fw: app.uiFramework || '?', storage: app.storageBackend || '?'}),
                        f('ui.perf.line_build_search', {search: app.searchEngine || '?'}),
                    ]),
                    section('ui.perf.section_audio_formats', [tag(app.audioFormats)]),
                    section('ui.perf.section_plugin_formats', [tag(app.pluginFormats)]),
                    section('ui.perf.section_app_daw_projects', [
                        f('ui.perf.daw_format_count', {n: String((app.dawFormats || []).length)}),
                        tag(app.dawFormats),
                    ]),
                    section('ui.perf.section_preset_formats', [tag(app.presetFormats)]),
                    section('ui.perf.section_midi_formats', [tag(app.midiFormats)]),
                    section('ui.perf.section_pdf_formats', [tag(app.pdfFormats)]),
                    section('ui.perf.section_plugin_extraction', [
                        f('ui.perf.xref_format_count', {n: String((app.xrefFormats || []).length)}),
                        tag(app.xrefFormats),
                    ]),
                    section('ui.perf.section_analysis_engines', [tag(app.analysisEngines)]),
                    section('ui.perf.section_visualizers', [tag(app.visualizers)]),
                    section('ui.perf.section_export_formats', [tag(app.exportFormats)]),
                ].join('');
            }
            // Show panes now that content is loaded, then rebalance columns
            const sysPane = perfInfo.closest('.settings-section');
            const appPane = appInfo?.closest('.settings-section');
            if (sysPane) sysPane.style.display = '';
            if (appPane) appPane.style.display = '';
            settingsReflow();
        }).catch((err) => {
            const c = navigator.hardwareConcurrency || '?';
            perfInfo.textContent = f('ui.perf.load_error', {cpus: String(c), err: String(err)});
        });
    }

    // Selects
    const typeFilter = getSettingValue('defaultTypeFilter', 'all');
    const typeFilterEl = document.getElementById('settingDefaultTypeFilter');
    if (typeFilterEl) typeFilterEl.value = typeFilter;

    const pluginSort = getSettingValue('pluginSort', 'name-asc');
    const pluginSortEl = document.getElementById('settingPluginSort');
    if (pluginSortEl) pluginSortEl.value = pluginSort;

    const audioSort = getSettingValue('audioSort', 'name');
    const audioSortEl = document.getElementById('settingAudioSort');
    if (audioSortEl) audioSortEl.value = audioSort;

    // Custom dirs
    const customDirs = prefs.getItem('customDirs') || '';
    const customDirsEl = document.getElementById('settingCustomDirs');
    if (customDirsEl) customDirsEl.value = customDirs;

    const audioScanDirs = prefs.getItem('audioScanDirs') || '';
    const audioScanDirsEl = document.getElementById('settingAudioScanDirs');
    if (audioScanDirsEl) audioScanDirsEl.value = audioScanDirs;

    const dawScanDirs = prefs.getItem('dawScanDirs') || '';
    const dawScanDirsEl = document.getElementById('settingDawScanDirs');
    if (dawScanDirsEl) dawScanDirsEl.value = dawScanDirs;

    const presetScanDirs = prefs.getItem('presetScanDirs') || '';
    const presetScanDirsEl = document.getElementById('settingPresetScanDirs');
    if (presetScanDirsEl) presetScanDirsEl.value = presetScanDirs;
    const midiScanDirs = prefs.getItem('midiScanDirs') || '';
    const midiScanDirsEl = document.getElementById('settingMidiScanDirs');
    if (midiScanDirsEl) midiScanDirsEl.value = midiScanDirs;
    const pdfScanDirs = prefs.getItem('pdfScanDirs') || '';
    const pdfScanDirsEl = document.getElementById('settingPdfScanDirs');
    if (pdfScanDirsEl) pdfScanDirsEl.value = pdfScanDirs;
    const videoScanDirs = prefs.getItem('videoScanDirs') || '';
    const videoScanDirsEl = document.getElementById('settingVideoScanDirs');
    if (videoScanDirsEl) videoScanDirsEl.value = videoScanDirs;

    const folderWatchDirs = prefs.getItem('folderWatchDirs') || '';
    const folderWatchDirsEl = document.getElementById('settingFolderWatchDirs');
    if (folderWatchDirsEl) folderWatchDirsEl.value = folderWatchDirs;

    const dawSort = getSettingValue('dawSort', 'name');
    const dawSortEl = document.getElementById('settingDawSort');
    if (dawSortEl) dawSortEl.value = dawSort;

    const presetSort = getSettingValue('presetSort', 'name');
    const presetSortEl = document.getElementById('settingPresetSort');
    if (presetSortEl) presetSortEl.value = presetSort;

    const midiSort = getSettingValue('midiSort', 'name');
    const midiSortEl = document.getElementById('settingMidiSort');
    if (midiSortEl) midiSortEl.value = midiSort;

    const pdfSort = getSettingValue('pdfSort', 'name');
    const pdfSortEl = document.getElementById('settingPdfSort');
    if (pdfSortEl) pdfSortEl.value = pdfSort;

    const logVerbosity = getSettingValue('logVerbosity', 'normal');
    const logVerbosityEl = document.getElementById('settingLogVerbosity');
    if (logVerbosityEl) logVerbosityEl.value = logVerbosity;

    // Custom scheme presets
    refreshCustomPresetUI();

    // Sync color pickers to current scheme (preset or custom)
    const customVars = prefs.getObject('customSchemeVars', {});
    const schemeObj = COLOR_SCHEMES[currentScheme];
    document.querySelectorAll('.custom-color-input').forEach(input => {
        const v = input.dataset.var;
        if (Object.keys(customVars).length > 0 && customVars[v] && customVars[v].startsWith('#')) {
            input.value = customVars[v];
        } else if (schemeObj && schemeObj.vars[v] && schemeObj.vars[v].startsWith('#')) {
            input.value = schemeObj.vars[v];
        }
    });

    // About: version line (i18n)
    if (typeof updateSettingsAboutVersionLine === 'function') updateSettingsAboutVersionLine();

    // Prefs file path
    const prefsPathEl = document.getElementById('prefsFilePath');
    if (prefsPathEl && !prefsPathEl.textContent) {
        window.vstUpdater.getPrefsPath().then(p => {
            prefsPathEl.textContent = p;
        }).catch(e => {
            if (typeof showToast === 'function') showToast(String(e), 4000, 'error');
        });
    }

    if (typeof renderFzfSettings === 'function') renderFzfSettings();
    settingsReflow();
}

// Restore settings on load
function restoreSettings() {
    const saved = prefs.getItem('theme');
    if (saved === 'light') {
        document.documentElement.setAttribute('data-theme', 'light');
    }
    const crt = prefs.getItem('crtEffects');
    if (crt === 'off') {
        applyCrtSetting(false);
    }
    if (prefs.getItem('neonGlow') === 'off') {
        applyNeonGlowSetting(false);
    }
    const scheme = prefs.getItem('colorScheme');
    if (scheme && scheme.startsWith('custom')) {
        const customVars = prefs.getObject('customSchemeVars', {});
        if (Object.keys(customVars).length > 0) {
            applySchemeVars(customVars);
        }
    } else if (scheme && scheme !== 'cyberpunk') {
        applyColorScheme(scheme);
    }
    if (typeof syncTrayNowPlayingFromPlayback === 'function') syncTrayNowPlayingFromPlayback();
    const pageSize = parseInt(prefs.getItem('pageSize') || '200', 10);
    AUDIO_PAGE_SIZE = pageSize;
    DAW_PAGE_SIZE = pageSize;
    PRESET_PAGE_SIZE = pageSize;
    MIDI_PAGE_SIZE = pageSize;
    PDF_PAGE_SIZE = pageSize;
    VIDEO_PAGE_SIZE = pageSize;

    const uiLoc = prefs.getItem('uiLocale');
    const localeSel = document.getElementById('settingUiLocale');
    if (localeSel && Array.isArray(window.SUPPORTED_UI_LOCALES) && window.SUPPORTED_UI_LOCALES.includes(uiLoc)) {
        localeSel.value = uiLoc;
    }

    // Restore tag bar position
    const tagPos = prefs.getItem('tagBarPosition');
    if (tagPos === 'bottom') {
        const bar = document.getElementById('globalTagBar');
        const lastTab = [...document.querySelectorAll('.tab-content')].pop();
        if (bar && lastTab) lastTab.parentNode.insertBefore(bar, lastTab.nextSibling);
    }

    const ttDelay = parseInt(String(prefs.getItem('tooltipHoverDelayMs') || '600'), 10);
    const ttN = Number.isNaN(ttDelay) ? 600 : Math.min(5000, Math.max(0, ttDelay));
    const ttEl = document.getElementById('settingTooltipHoverDelay');
    const ttValEl = document.getElementById('settingTooltipHoverDelayValue');
    if (ttEl) ttEl.value = String(ttN);
    if (ttValEl) ttValEl.textContent = `${ttN} ms`;
}

// restoreSettings is called from loadLastScan after prefs.load()

// ── fzf Parameter Settings ──
function renderFzfSettings() {
    const grid = document.getElementById('fzfSettingsGrid');
    if (!grid) return;
    const fmt = catalogFmt;
    const params = [
        {key: 'SCORE_MATCH', labelKey: 'ui.fzf.score_match.label', descKey: 'ui.fzf.score_match.desc', min: 1, max: 50},
        {
            key: 'SCORE_GAP_START',
            labelKey: 'ui.fzf.gap_start.label',
            descKey: 'ui.fzf.gap_start.desc',
            min: -20,
            max: 0
        },
        {
            key: 'SCORE_GAP_EXTENSION',
            labelKey: 'ui.fzf.gap_extension.label',
            descKey: 'ui.fzf.gap_extension.desc',
            min: -10,
            max: 0
        },
        {key: 'BONUS_BOUNDARY', labelKey: 'ui.fzf.boundary.label', descKey: 'ui.fzf.boundary.desc', min: 0, max: 30},
        {key: 'BONUS_NON_WORD', labelKey: 'ui.fzf.non_word.label', descKey: 'ui.fzf.non_word.desc', min: 0, max: 30},
        {key: 'BONUS_CAMEL', labelKey: 'ui.fzf.camel.label', descKey: 'ui.fzf.camel.desc', min: 0, max: 30},
        {
            key: 'BONUS_CONSECUTIVE',
            labelKey: 'ui.fzf.consecutive.label',
            descKey: 'ui.fzf.consecutive.desc',
            min: 0,
            max: 20
        },
        {
            key: 'BONUS_FIRST_CHAR_MULT',
            labelKey: 'ui.fzf.first_char_mult.label',
            descKey: 'ui.fzf.first_char_mult.desc',
            min: 1,
            max: 5
        },
    ];
    grid.innerHTML = params.map(p => {
        const val = window[p.key] ?? FZF_DEFAULTS[p.key];
        const label = fmt(p.labelKey);
        const desc = fmt(p.descKey);
        const titleAttr = `${desc} (default: ${FZF_DEFAULTS[p.key]})`;
        return `<div class="settings-row" style="padding:6px 8px;margin-bottom:2px;">
      <div class="settings-label" style="min-width:0;">
        <span class="settings-title" style="font-size:11px;">${escapeHtml(label)}</span>
        <span class="settings-desc" style="font-size:9px;">${escapeHtml(desc)}</span>
      </div>
      <div class="settings-control" style="display:flex;align-items:center;gap:6px;">
        <input type="number" class="settings-input" data-fzf-param="${p.key}" value="${val}" min="${p.min}" max="${p.max}" step="1" style="width:60px;font-size:11px;padding:3px 6px;" title="${escapeHtml(titleAttr)}">
      </div>
    </div>`;
    }).join('');
    if (typeof initDragReorder === 'function') {
        initDragReorder(grid, '.settings-row', 'fzfParamOrder', {
            getKey: (el) => el.querySelector('[data-fzf-param]')?.dataset.fzfParam || '',
        });
    }
}

// Blacklist saved via Save button (data-action="saveBlacklist")

document.addEventListener('input', (e) => {
    const input = e.target.closest('[data-fzf-param]');
    if (!input) return;
    const key = input.dataset.fzfParam;
    const val = parseFloat(input.value);
    if (isNaN(val)) return;
    window[key] = val;
    // Update the module-level variable
    switch (key) {
        case 'SCORE_MATCH':
            SCORE_MATCH = val;
            break;
        case 'SCORE_GAP_START':
            SCORE_GAP_START = val;
            break;
        case 'SCORE_GAP_EXTENSION':
            SCORE_GAP_EXTENSION = val;
            break;
        case 'BONUS_BOUNDARY':
            BONUS_BOUNDARY = val;
            break;
        case 'BONUS_NON_WORD':
            BONUS_NON_WORD = val;
            break;
        case 'BONUS_CAMEL':
            BONUS_CAMEL = val;
            break;
        case 'BONUS_CONSECUTIVE':
            BONUS_CONSECUTIVE = val;
            break;
        case 'BONUS_FIRST_CHAR_MULT':
            BONUS_FIRST_CHAR_MULT = val;
            break;
    }
    saveFzfParams();
});

// ── Settings Section Drag Reorder (Trello-style) ──
// Whole-pane section order: `initDragReorder` on `#tabSettings .settings-container` in drag-reorder.js (prefs `settingsSectionOrder`).
// Row order within each section: below (prefs `settingsRows_<data-section>`).
function initSettingsSectionDrag() {
    // Settings uses CSS columns masonry (see index.html); legacy hook kept for callers that still invoke it.
    window.balanceSettingsColumns = function () {
    };

    const noDragSections = new Set(['colorscheme', 'shortcuts', 'fuzzy-search', 'system-info', 'app-info', 'caches']);
    if (typeof initDragReorder !== 'function') return;

    function initRowsInPane(containerSelector) {
        const pane = document.querySelector(containerSelector);
        if (!pane) return;
        pane.querySelectorAll('.settings-section[data-section]').forEach(section => {
            if (noDragSections.has(section.dataset.section)) return;
            initDragReorder(section, '.settings-row', 'settingsRows_' + section.dataset.section, {
                getKey: (el) => el.querySelector('.settings-title')?.textContent?.trim() || '',
            });
        });
    }

    initRowsInPane('#tabSettings .settings-container');
    initRowsInPane('#tabAudioEngine .settings-container.audio-engine-tab');
}

function resetSettingsSectionOrder() {
    prefs.removeItem('settingsSectionOrder');
    const container = document.querySelector('#tabSettings .settings-container');
    if (!container) return;
    const defaultOrder = ['appearance', 'scanning', 'performance', 'exclusions', 'sorting', 'fuzzy-search', 'about', 'system-info', 'colorscheme', 'app-info', 'playback', 'scan-behavior', 'visualizer-settings', 'data', 'files', 'storage', 'caches', 'danger-zone', 'shortcuts'];
    const sectionMap = {};
    container.querySelectorAll('.settings-section[data-section]').forEach(s => {
        sectionMap[s.dataset.section] = s;
    });
    const searchBar = container.querySelector('.settings-search-bar');
    if (searchBar) {
        let ref = searchBar;
        for (const key of defaultOrder) {
            if (sectionMap[key]) {
                ref.insertAdjacentElement('afterend', sectionMap[key]);
                ref = sectionMap[key];
            }
        }
    } else {
        for (const key of defaultOrder) {
            if (sectionMap[key]) container.appendChild(sectionMap[key]);
        }
    }
    showToast(toastFmt('toast.settings_layout_reset_short'));
}

// Export for menu-action handler in ipc.js
window.settingToggleTheme = settingToggleTheme;
window.settingToggleCrt = settingToggleCrt;
window.settingResetColumns = settingResetColumns;
window.settingClearAllDatabases = settingClearAllDatabases;
window.settingClearAllHistory = settingClearAllHistory;
window.resetAllScans = resetAllScans;
window.settingClearKvrCache = settingClearKvrCache;
