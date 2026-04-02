// ── Settings ──

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

// Dynamically build scheme buttons from COLOR_SCHEMES
(function buildSchemeButtons() {
  const grid = document.getElementById('schemeGrid');
  const dotKeys = ['--accent', '--cyan', '--magenta', '--green', '--yellow', '--orange', '--red', '--text'];
  for (const [key, scheme] of Object.entries(COLOR_SCHEMES)) {
    const dots = dotKeys.map(k => `<span class="scheme-dot" style="background: ${scheme.vars[k]};"></span>`).join('');
    grid.insertAdjacentHTML('beforeend',
      `<button class="scheme-btn" data-action="settingColorScheme" data-scheme="${key}" title="Apply ${scheme.label} color scheme">` +
        `<div class="scheme-btn-name">${scheme.label}</div>` +
        `<div class="scheme-btn-desc">${scheme.desc}</div>` +
        `<div class="scheme-btn-preview">${dots}</div>` +
      `</button>`
    );
  }
})();

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
  presets.push({ name, vars });
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
    return `<button class="custom-preset-chip${active}" data-action="loadCustomPreset" data-idx="${i}" title="Load custom preset: ${escapeHtml(p.name || 'Preset ' + (i+1))}">
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
}

function settingToggleCrt() {
  const current = prefs.getItem('crtEffects') !== 'off';
  const next = !current;
  prefs.setItem('crtEffects', next ? 'on' : 'off');
  applyCrtSetting(next);
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

function settingResetColumns() {
  prefs.removeItem('columnWidths');
  // Re-init tables if they exist
  const audioTable = document.getElementById('audioTable');
  const dawTable = document.getElementById('dawTable');
  if (audioTable) {
    audioTable.querySelectorAll('thead th').forEach(th => { th.style.width = ''; });
  }
  if (dawTable) {
    dawTable.querySelectorAll('thead th').forEach(th => { th.style.width = ''; });
  }
}

async function settingClearAllHistory() {
  if (!await confirmAction('Clear all plugin, audio, DAW, and preset scan history? This cannot be undone.', 'Clear History')) return;
  showGlobalProgress();
  try {
    await Promise.all([
      window.vstUpdater.clearHistory(),
      window.vstUpdater.clearAudioHistory(),
      window.vstUpdater.clearDawHistory(),
      window.vstUpdater.clearPresetHistory(),
    ]);
    showToast('All scan history cleared');
  } catch (e) {
    showToast(`Failed to clear history — ${e.message || e}`, 4000, 'error');
  } finally { hideGlobalProgress(); }
}

async function resetAllScans() {
  if (!await confirmAction('Reset everything? This will clear all scan results, history, and KVR cache. The app will return to its initial state.\n\nThis cannot be undone.', 'Reset All Scans')) return;
  showGlobalProgress();
  try {
    // Stop any running scans
    await Promise.all([
      window.vstUpdater.stopScan().catch(() => {}),
      window.vstUpdater.stopAudioScan().catch(() => {}),
      window.vstUpdater.stopDawScan().catch(() => {}),
      window.vstUpdater.stopPresetScan().catch(() => {}),
    ]);
    // Clear backend history + KVR cache
    await Promise.all([
      window.vstUpdater.clearHistory(),
      window.vstUpdater.clearAudioHistory(),
      window.vstUpdater.clearDawHistory(),
      window.vstUpdater.clearPresetHistory(),
      window.vstUpdater.updateKvrCache([]),
    ]);
    // Clear in-memory data
    allPlugins = [];
    pluginsWithUpdates = [];
    if (typeof allAudioSamples !== 'undefined') allAudioSamples = [];
    if (typeof filteredAudioSamples !== 'undefined') filteredAudioSamples = [];
    if (typeof allDawProjects !== 'undefined') allDawProjects = [];
    if (typeof allPresets !== 'undefined') allPresets = [];
    if (typeof recentlyPlayed !== 'undefined') { recentlyPlayed = []; saveRecentlyPlayed(); }
    // Clear xref cache
    if (typeof _xrefCache !== 'undefined') { for (const k in _xrefCache) delete _xrefCache[k]; }
    // Reset plugin UI
    document.getElementById('pluginList').innerHTML = '<div class="state-message" id="emptyState"><div class="state-icon">&#128268;</div><h2>Audio Plugin Scanner</h2><p>Click <strong>"Scan Plugins"</strong> to discover all VST2, VST3, and Audio Unit plugins on your system.</p></div>';
    document.getElementById('totalCount').textContent = '0';
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
    showToast('All scans reset to fresh state');
  } catch (e) {
    showToast(`Reset failed — ${e.message || e}`, 4000, 'error');
  } finally { hideGlobalProgress(); }
}

async function settingClearKvrCache() {
  if (!await confirmAction('Clear all cached KVR version lookups? Next update check will re-fetch everything.', 'Clear KVR Cache')) return;
  showGlobalProgress();
  try {
    await window.vstUpdater.updateKvrCache([]);
    showToast('KVR cache cleared');
  } catch (e) {
    showToast(`Failed to clear KVR cache — ${e.message || e}`, 4000, 'error');
  } finally { hideGlobalProgress(); }
}

function settingToggleAutoScan() {
  const current = prefs.getItem('autoScan') === 'on';
  prefs.setItem('autoScan', current ? 'off' : 'on');
  refreshSettingsUI();
}

function settingToggleAutoUpdate() {
  const current = prefs.getItem('autoUpdate') === 'on';
  prefs.setItem('autoUpdate', current ? 'off' : 'on');
  refreshSettingsUI();
}

function settingToggleSingleClickPlay() {
  const current = prefs.getItem('singleClickPlay') === 'on';
  prefs.setItem('singleClickPlay', current ? 'off' : 'on');
  refreshSettingsUI();
}

function settingToggleAutoplayNext() {
  const current = prefs.getItem('autoplayNext');
  prefs.setItem('autoplayNext', current === 'off' ? 'on' : 'off');
  refreshSettingsUI();
}

function settingToggleShowPlayerOnStartup() {
  const current = prefs.getItem('showPlayerOnStartup') === 'on';
  prefs.setItem('showPlayerOnStartup', current ? 'off' : 'on');
  refreshSettingsUI();
}

function settingToggleExpandOnClick() {
  const current = prefs.getItem('expandOnClick');
  prefs.setItem('expandOnClick', current === 'off' ? 'on' : 'off');
  refreshSettingsUI();
}

function settingToggleIncludeBackups() {
  const current = prefs.getItem('includeAbletonBackups') === 'on';
  prefs.setItem('includeAbletonBackups', current ? 'off' : 'on');
  refreshSettingsUI();
}

function settingUpdatePageSize(val) {
  document.getElementById('settingPageSizeValue').textContent = val;
  prefs.setItem('pageSize', val);
  AUDIO_PAGE_SIZE = parseInt(val, 10);
  DAW_PAGE_SIZE = parseInt(val, 10);
}

function settingUpdateFlushInterval(val) {
  document.getElementById('settingFlushIntervalValue').textContent = val;
  prefs.setItem('flushInterval', val);
}

function settingUpdateThreadMultiplier(val) {
  document.getElementById('settingThreadMultiplierValue').textContent = val + 'x';
  prefs.setItem('threadMultiplier', val);
  showToast('Thread multiplier set to ' + val + 'x — restart to apply');
}

function settingUpdateChannelBuffer(val) {
  document.getElementById('settingChannelBufferValue').textContent = val;
  prefs.setItem('channelBuffer', val);
  showToast('Channel buffer set to ' + val + ' — restart to apply');
}

function settingUpdateBatchSize(val) {
  document.getElementById('settingBatchSizeValue').textContent = val;
  prefs.setItem('batchSize', val);
  showToast('Batch size set to ' + val + ' — restart to apply');
}

function settingUpdateFdLimit(val) {
  document.getElementById('settingFdLimitValue').textContent = val;
  prefs.setItem('fdLimit', val);
  showToast('FD limit set to ' + val + ' — restart to apply');
}

function settingSaveSelect(key, value) {
  prefs.setItem(key, value);
}

function showSavedMsg(id) {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = 'Saved';
  el.classList.add('visible');
  setTimeout(() => el.classList.remove('visible'), 2000);
}

async function browseDir(targetId) {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi || !dialogApi.open) {
    showToast('Dialog API not available', 3000, 'error');
    return;
  }
  const selected = await dialogApi.open({ directory: true, multiple: false, title: 'Select folder to scan' });
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
  showToast(`Added: ${selected}`);
}

function saveCustomDirs() {
  const val = document.getElementById('settingCustomDirs').value.trim();
  prefs.setItem('customDirs', val);
  showSavedMsg('savedMsgCustomDirs');
}

function saveAudioScanDirs() {
  const val = document.getElementById('settingAudioScanDirs').value.trim();
  prefs.setItem('audioScanDirs', val);
  showSavedMsg('savedMsgAudioScanDirs');
}

function saveDawScanDirs() {
  const val = document.getElementById('settingDawScanDirs').value.trim();
  prefs.setItem('dawScanDirs', val);
  showSavedMsg('savedMsgDawScanDirs');
}

function savePresetScanDirs() {
  const val = document.getElementById('settingPresetScanDirs').value.trim();
  prefs.setItem('presetScanDirs', val);
  showSavedMsg('savedMsgPresetScanDirs');
}

function openPrefsFile() {
  window.vstUpdater.openPrefsFile().catch(e => showToast(`Failed to open prefs file — ${e.message || e}`, 4000, 'error'));
}

function getSettingValue(key, defaultVal) {
  return prefs.getItem(key) || defaultVal;
}

function refreshSettingsUI() {
  // Theme
  const theme = document.documentElement.getAttribute('data-theme') || 'dark';
  const themeBtn = document.getElementById('settingTheme');
  const themeLabel = document.getElementById('settingThemeLabel');
  themeBtn.classList.toggle('active', theme === 'light');
  themeLabel.textContent = theme === 'light' ? 'Light' : 'Dark';

  // CRT
  const crtOn = prefs.getItem('crtEffects') !== 'off';
  const crtBtn = document.getElementById('settingCrt');
  const crtLabel = document.getElementById('settingCrtLabel');
  crtBtn.classList.toggle('active', crtOn);
  crtLabel.textContent = crtOn ? 'On' : 'Off';

  // Color scheme
  const currentScheme = prefs.getItem('colorScheme') || 'cyberpunk';
  document.querySelectorAll('.scheme-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.scheme === currentScheme);
  });

  // Auto-scan
  const autoScan = prefs.getItem('autoScan') === 'on';
  const autoScanBtn = document.getElementById('settingAutoScan');
  const autoScanLabel = document.getElementById('settingAutoScanLabel');
  if (autoScanBtn) {
    autoScanBtn.classList.toggle('active', autoScan);
    autoScanLabel.textContent = autoScan ? 'On' : 'Off';
  }

  // Auto-update
  const autoUpdate = prefs.getItem('autoUpdate') === 'on';
  const autoUpdateBtn = document.getElementById('settingAutoUpdate');
  const autoUpdateLabel = document.getElementById('settingAutoUpdateLabel');
  if (autoUpdateBtn) {
    autoUpdateBtn.classList.toggle('active', autoUpdate);
    autoUpdateLabel.textContent = autoUpdate ? 'On' : 'Off';
  }

  // Single-click play
  const singleClick = prefs.getItem('singleClickPlay') === 'on';
  const singleClickBtn = document.getElementById('settingSingleClickPlay');
  const singleClickLabel = document.getElementById('settingSingleClickPlayLabel');
  if (singleClickBtn) {
    singleClickBtn.classList.toggle('active', singleClick);
    singleClickLabel.textContent = singleClick ? 'On' : 'Off';
  }

  // Expand on click
  const expandOnClick = prefs.getItem('expandOnClick') !== 'off';
  const expandBtn = document.getElementById('settingExpandOnClick');
  const expandLabel = document.getElementById('settingExpandOnClickLabel');
  if (expandBtn) {
    expandBtn.classList.toggle('active', expandOnClick);
    expandLabel.textContent = expandOnClick ? 'On' : 'Off';
  }

  // Show player on startup
  const showPlayer = prefs.getItem('showPlayerOnStartup') === 'on';
  const showPlayerBtn = document.getElementById('settingShowPlayerOnStartup');
  const showPlayerLabel = document.getElementById('settingShowPlayerOnStartupLabel');
  if (showPlayerBtn) {
    showPlayerBtn.classList.toggle('active', showPlayer);
    showPlayerLabel.textContent = showPlayer ? 'On' : 'Off';
  }

  // Autoplay next
  const autoplay = prefs.getItem('autoplayNext') !== 'off';
  const autoplayBtn = document.getElementById('settingAutoplayNext');
  const autoplayLabel = document.getElementById('settingAutoplayNextLabel');
  if (autoplayBtn) {
    autoplayBtn.classList.toggle('active', autoplay);
    autoplayLabel.textContent = autoplay ? 'On' : 'Off';
  }

  // Include Ableton backups
  const includeBackups = prefs.getItem('includeAbletonBackups') === 'on';
  const backupsBtn = document.getElementById('settingIncludeBackups');
  const backupsLabel = document.getElementById('settingIncludeBackupsLabel');
  if (backupsBtn) {
    backupsBtn.classList.toggle('active', includeBackups);
    backupsLabel.textContent = includeBackups ? 'On' : 'Off';
  }

  // Page size
  const pageSize = getSettingValue('pageSize', '500');
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

  // System perf info — get real stats from Rust backend
  const perfInfo = document.getElementById('settingPerfInfo');
  if (perfInfo) {
    window.vstUpdater.getProcessStats().then(stats => {
      const cpus = stats.numCpus || navigator.hardwareConcurrency || '?';
      const scanThreads = parseInt(threadMult) * parseInt(cpus);
      const fmtMem = (bytes) => {
        if (!bytes || bytes === 0) return '0 B';
        const units = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
      };
      const fmtUptime = (secs) => {
        if (!secs) return '0s';
        const h = Math.floor(secs / 3600);
        const m = Math.floor((secs % 3600) / 60);
        const s = secs % 60;
        return (h ? h + 'h ' : '') + (m ? m + 'm ' : '') + s + 's';
      };
      const sc = stats.scanner || {};
      const cfg = stats.config || {};
      const df = stats.dataFiles || {};
      const pluginCount = typeof allPlugins !== 'undefined' ? allPlugins.length : 0;
      const sampleCount = typeof allAudioSamples !== 'undefined' ? allAudioSamples.length : 0;
      const dawCount = typeof allDawProjects !== 'undefined' ? allDawProjects.length : 0;
      const presetCount = typeof allPresets !== 'undefined' ? allPresets.length : 0;
      const dot = (on) => on ? '<span style="color:var(--green);">&#9679;</span>' : '<span style="color:var(--text-dim);">&#9675;</span>';
      perfInfo.innerHTML = [
        `<b>CPU:</b> ${cpus} cores | ${(stats.cpuPercent || 0).toFixed(1)}% usage`,
        `<b>Memory:</b> RSS ${fmtMem(stats.rssBytes)} | Virtual ${fmtMem(stats.virtualBytes)}`,
        `<b>Process:</b> PID ${stats.pid} | ${stats.threads} threads | ${stats.openFds} FDs | uptime ${fmtUptime(stats.uptimeSecs)}`,
        `<b>Thread Pools:</b> global rayon ${stats.rayonThreads} | per-scanner ${cfg.perScannerThreads || '?'} | multiplier ${cfg.threadMultiplier || '?'}x | total pool ${cfg.globalPoolSize || '?'} | stack ${cfg.stackSize || '?'}`,
        `<b>Plugin Scanner:</b> channel buf ${cfg.channelBuffer || '?'} (${cfg.pluginChannelMin || '?'}-${cfg.pluginChannelMax || '?'}) | batch ${cfg.batchSize || '?'}`,
        `<b>Walker Scanners:</b> channel buf ${cfg.walkerChannelBuffer || '?'} | batch ${cfg.walkerBatchSize || '?'} | depth limit ${cfg.depthLimit || '?'} | flush ${cfg.flushInterval || '?'}ms | page ${cfg.pageSize || '?'}`,
        `<b>Scanner State:</b> ${dot(sc.pluginScanning)} Plugins ${dot(sc.audioScanning)} Samples ${dot(sc.dawScanning)} DAW ${dot(sc.presetScanning)} Presets ${dot(sc.updateChecking)} Updates`,
        `<b>Scan Results:</b> ${pluginCount} plugins | ${sampleCount} samples | ${dawCount} DAW projects | ${presetCount} presets`,
        `<b>Data Files:</b> prefs ${fmtMem(df.preferencesBytes)} | KVR cache ${fmtMem(df.kvrCacheBytes)} | history ${fmtMem((df.scanHistoryBytes||0) + (df.audioHistoryBytes||0) + (df.dawHistoryBytes||0) + (df.presetHistoryBytes||0))}`,
        `<b>Data Dir:</b> <code style="font-size:10px;word-break:break-all;">${escapeHtml(stats.dataDir || '?')}</code>`,
      ].join('<br>');
    }).catch((err) => {
      const cpus = navigator.hardwareConcurrency || '?';
      perfInfo.textContent = `${cpus} cores | error: ${err}`;
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

  const dawSort = getSettingValue('dawSort', 'name');
  const dawSortEl = document.getElementById('settingDawSort');
  if (dawSortEl) dawSortEl.value = dawSort;

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

  // Version
  const ver = document.getElementById('appVersion')?.textContent || '';
  const settingsVer = document.getElementById('settingsVersion');
  if (settingsVer) settingsVer.textContent = ver;

  // Prefs file path
  const prefsPathEl = document.getElementById('prefsFilePath');
  if (prefsPathEl && !prefsPathEl.textContent) {
    window.vstUpdater.getPrefsPath().then(p => { prefsPathEl.textContent = p; }).catch(() => {});
  }
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
  const scheme = prefs.getItem('colorScheme');
  if (scheme && scheme.startsWith('custom')) {
    const customVars = prefs.getObject('customSchemeVars', {});
    if (Object.keys(customVars).length > 0) {
      applySchemeVars(customVars);
    }
  } else if (scheme && scheme !== 'cyberpunk') {
    applyColorScheme(scheme);
  }
  const pageSize = parseInt(prefs.getItem('pageSize') || '500', 10);
  AUDIO_PAGE_SIZE = pageSize;
  DAW_PAGE_SIZE = pageSize;
}
// restoreSettings is called from loadLastScan after prefs.load()

// ── fzf Parameter Settings ──
function renderFzfSettings() {
  const grid = document.getElementById('fzfSettingsGrid');
  if (!grid) return;
  const params = [
    { key: 'SCORE_MATCH', label: 'Match Score', desc: 'Points per matched character', min: 1, max: 50 },
    { key: 'SCORE_GAP_START', label: 'Gap Start Penalty', desc: 'Penalty for starting a gap', min: -20, max: 0 },
    { key: 'SCORE_GAP_EXTENSION', label: 'Gap Extension', desc: 'Penalty per additional gap character', min: -10, max: 0 },
    { key: 'BONUS_BOUNDARY', label: 'Word Boundary Bonus', desc: 'Bonus for matching at word boundaries', min: 0, max: 30 },
    { key: 'BONUS_NON_WORD', label: 'Non-Word Bonus', desc: 'Bonus for non-word character transitions', min: 0, max: 30 },
    { key: 'BONUS_CAMEL', label: 'CamelCase Bonus', desc: 'Bonus for camelCase transitions', min: 0, max: 30 },
    { key: 'BONUS_CONSECUTIVE', label: 'Consecutive Bonus', desc: 'Bonus for consecutive matches', min: 0, max: 20 },
    { key: 'BONUS_FIRST_CHAR_MULT', label: 'First Char Multiplier', desc: 'Multiplier for first character bonus', min: 1, max: 5 },
  ];
  grid.innerHTML = params.map(p => {
    const val = window[p.key] ?? FZF_DEFAULTS[p.key];
    return `<div class="settings-row" style="padding:6px 8px;margin-bottom:2px;">
      <div class="settings-label" style="min-width:0;">
        <span class="settings-title" style="font-size:11px;">${p.label}</span>
        <span class="settings-desc" style="font-size:9px;">${p.desc}</span>
      </div>
      <div class="settings-control" style="display:flex;align-items:center;gap:6px;">
        <input type="number" class="settings-input" data-fzf-param="${p.key}" value="${val}" min="${p.min}" max="${p.max}" step="1" style="width:60px;font-size:11px;padding:3px 6px;" title="${p.desc} (default: ${FZF_DEFAULTS[p.key]})">
      </div>
    </div>`;
  }).join('');
  if (typeof initDragReorder === 'function') {
    initDragReorder(grid, '.settings-row', 'fzfParamOrder', {
      getKey: (el) => el.querySelector('[data-fzf-param]')?.dataset.fzfParam || '',
      handleSelector: '.settings-label',
    });
  }
}

document.addEventListener('input', (e) => {
  const input = e.target.closest('[data-fzf-param]');
  if (!input) return;
  const key = input.dataset.fzfParam;
  const val = parseFloat(input.value);
  if (isNaN(val)) return;
  window[key] = val;
  // Update the module-level variable
  switch (key) {
    case 'SCORE_MATCH': SCORE_MATCH = val; break;
    case 'SCORE_GAP_START': SCORE_GAP_START = val; break;
    case 'SCORE_GAP_EXTENSION': SCORE_GAP_EXTENSION = val; break;
    case 'BONUS_BOUNDARY': BONUS_BOUNDARY = val; break;
    case 'BONUS_NON_WORD': BONUS_NON_WORD = val; break;
    case 'BONUS_CAMEL': BONUS_CAMEL = val; break;
    case 'BONUS_CONSECUTIVE': BONUS_CONSECUTIVE = val; break;
    case 'BONUS_FIRST_CHAR_MULT': BONUS_FIRST_CHAR_MULT = val; break;
  }
  saveFzfParams();
});

// ── Settings Section Drag Reorder (Trello-style) ──
function initSettingsSectionDrag() {
  const container = document.querySelector('.settings-container');
  if (!container) return;
  let dragged = null;
  let ghost = null;
  let placeholder = null;
  let startY = 0;
  let startX = 0;
  let offsetY = 0;
  let offsetX = 0;
  let isDragging = false;

  container.addEventListener('mousedown', (e) => {
    const heading = e.target.closest('.settings-heading');
    if (!heading || e.button !== 0) return;
    const section = heading.closest('.settings-section');
    if (!section) return;
    e.preventDefault();
    dragged = section;
    const rect = section.getBoundingClientRect();
    startY = e.clientY;
    startX = e.clientX;
    offsetY = e.clientY - rect.top;
    offsetX = e.clientX - rect.left;
    isDragging = false;
  });

  document.addEventListener('mousemove', (e) => {
    if (!dragged) return;
    if (!isDragging && Math.abs(e.clientY - startY) > 8) {
      isDragging = true;
      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'grabbing';

      // Create placeholder
      const rect = dragged.getBoundingClientRect();
      placeholder = document.createElement('div');
      placeholder.className = 'section-placeholder';
      placeholder.style.height = rect.height + 'px';
      dragged.parentNode.insertBefore(placeholder, dragged);

      // Create floating ghost
      ghost = dragged.cloneNode(true);
      ghost.className = 'settings-section section-ghost';
      ghost.style.width = rect.width + 'px';
      ghost.style.left = (e.clientX - offsetX) + 'px';
      ghost.style.top = (e.clientY - offsetY) + 'px';
      document.body.appendChild(ghost);

      // Hide original
      dragged.style.display = 'none';
    }
    if (!isDragging || !ghost) return;

    // Move ghost with cursor
    ghost.style.left = (e.clientX - offsetX) + 'px';
    ghost.style.top = (e.clientY - offsetY) + 'px';

    // Find drop target — temporarily hide ghost to get element underneath
    ghost.style.pointerEvents = 'none';
    const el = document.elementFromPoint(e.clientX, e.clientY);
    ghost.style.pointerEvents = '';
    const target = el?.closest('.settings-section');

    container.querySelectorAll('.settings-section').forEach(s => s.classList.remove('section-drag-over'));
    if (target && target !== dragged && target !== placeholder) {
      // Move placeholder to show where the section will land
      const targetRect = target.getBoundingClientRect();
      const midY = targetRect.top + targetRect.height / 2;
      if (e.clientY < midY) {
        container.insertBefore(placeholder, target);
      } else {
        container.insertBefore(placeholder, target.nextSibling);
      }
      target.classList.add('section-drag-over');
    }
  });

  document.addEventListener('mouseup', (e) => {
    if (!dragged) return;
    if (isDragging) {
      container.querySelectorAll('.settings-section').forEach(s => s.classList.remove('section-drag-over'));
      document.body.style.userSelect = '';
      document.body.style.cursor = '';

      // Move dragged section to placeholder position
      if (placeholder && placeholder.parentNode) {
        placeholder.parentNode.insertBefore(dragged, placeholder);
        placeholder.remove();
      }
      dragged.style.display = '';
      if (ghost) { ghost.remove(); ghost = null; }
      placeholder = null;
      saveSettingsSectionOrder();

      const suppress = (ev) => { ev.stopPropagation(); ev.preventDefault(); };
      container.addEventListener('click', suppress, { capture: true, once: true });
    }
    dragged = null;
    isDragging = false;
    document.body.style.userSelect = '';
    document.body.style.cursor = '';
  });

  restoreSettingsSectionOrder();

  // Make individual settings rows draggable within each section
  if (typeof initDragReorder === 'function') {
    container.querySelectorAll('.settings-section[data-section]').forEach(section => {
      initDragReorder(section, '.settings-row', 'settingsRows_' + section.dataset.section, {
        getKey: (el) => el.querySelector('.settings-title')?.textContent?.trim() || '',
        handleSelector: '.settings-label',
      });
    });
  }
}

function saveSettingsSectionOrder() {
  const sections = [...document.querySelectorAll('.settings-section[data-section]')].map(s => s.dataset.section);
  prefs.setItem('settingsSectionOrder', JSON.stringify(sections));
}

function restoreSettingsSectionOrder() {
  const saved = prefs.getItem('settingsSectionOrder');
  if (!saved) return;
  try {
    const order = JSON.parse(saved);
    if (!Array.isArray(order)) return;
    const container = document.querySelector('.settings-container');
    const sectionMap = {};
    container.querySelectorAll('.settings-section[data-section]').forEach(s => {
      sectionMap[s.dataset.section] = s;
    });
    for (const key of order) {
      if (sectionMap[key]) container.appendChild(sectionMap[key]);
    }
    // Append any sections not in saved order
    container.querySelectorAll('.settings-section[data-section]').forEach(s => {
      if (!order.includes(s.dataset.section)) container.appendChild(s);
    });
  } catch {}
}

function resetSettingsSectionOrder() {
  prefs.removeItem('settingsSectionOrder');
  const container = document.querySelector('.settings-container');
  const defaultOrder = ['appearance', 'colorscheme', 'scanning', 'performance', 'sorting', 'data', 'about'];
  const sectionMap = {};
  container.querySelectorAll('.settings-section[data-section]').forEach(s => {
    sectionMap[s.dataset.section] = s;
  });
  for (const key of defaultOrder) {
    if (sectionMap[key]) container.appendChild(sectionMap[key]);
  }
  showToast('Settings layout reset');
}
