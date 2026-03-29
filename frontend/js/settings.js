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
      `<button class="scheme-btn" data-action="settingColorScheme" data-scheme="${key}">` +
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
  // In light mode, only save the preference — don't apply inline vars
  // that would override [data-theme="light"] CSS
  const isLight = document.documentElement.getAttribute('data-theme') === 'light';
  if (!isLight) {
    const root = document.documentElement.style;
    for (const [k, v] of Object.entries(ROOT_DEFAULTS)) {
      root.setProperty(k, v);
    }
    for (const [k, v] of Object.entries(scheme.vars)) {
      root.setProperty(k, v);
    }
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

function applySchemeVarsIfDark(vars) {
  if (document.documentElement.getAttribute('data-theme') === 'light') return;
  const root = document.documentElement.style;
  for (const [k, v] of Object.entries(ROOT_DEFAULTS)) {
    root.setProperty(k, v);
  }
  applyCustomVars(vars);
}

function applyCustomScheme() {
  const vars = readCustomColorsFromPickers();
  prefs.setItem('colorScheme', 'custom');
  prefs.setItem('customSchemeVars', JSON.stringify(vars));
  applySchemeVarsIfDark(vars);
  document.querySelectorAll('.scheme-btn').forEach(b => b.classList.remove('active'));
  refreshCustomPresetUI();
}

function saveCustomScheme() {
  const vars = readCustomColorsFromPickers();
  const presets = JSON.parse(prefs.getItem('customSchemePresets') || '[]');
  const name = 'Custom ' + (presets.length + 1);
  presets.push({ name, vars });
  prefs.setItem('customSchemePresets', JSON.stringify(presets));
  prefs.setItem('colorScheme', 'custom-' + (presets.length - 1));
  prefs.setItem('customSchemeVars', JSON.stringify(vars));
  applySchemeVarsIfDark(vars);
  document.querySelectorAll('.scheme-btn').forEach(b => b.classList.remove('active'));
  refreshCustomPresetUI();
}

function loadCustomPreset(idx) {
  const presets = JSON.parse(prefs.getItem('customSchemePresets') || '[]');
  const preset = presets[idx];
  if (!preset) return;
  for (const input of document.querySelectorAll('.custom-color-input')) {
    const v = input.dataset.var;
    if (preset.vars[v]) input.value = preset.vars[v];
  }
  prefs.setItem('colorScheme', 'custom-' + idx);
  prefs.setItem('customSchemeVars', JSON.stringify(preset.vars));
  applySchemeVarsIfDark(preset.vars);
  document.querySelectorAll('.scheme-btn').forEach(b => b.classList.remove('active'));
  refreshCustomPresetUI();
}

function deleteCustomSchemes() {
  if (!confirm('Delete all saved custom color schemes?')) return;
  prefs.removeItem('customSchemePresets');
  refreshCustomPresetUI();
}

function refreshCustomPresetUI() {
  const container = document.getElementById('customSchemeSaved');
  const deleteBtn = document.getElementById('btnDeleteCustom');
  const presets = JSON.parse(prefs.getItem('customSchemePresets') || '[]');
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
    return `<button class="custom-preset-chip${active}" data-action="loadCustomPreset" data-idx="${i}">
      <span class="custom-preset-chip-dots">
        <span class="custom-preset-chip-dot" style="background:${accent}"></span>
        <span class="custom-preset-chip-dot" style="background:${cyan}"></span>
        <span class="custom-preset-chip-dot" style="background:${magenta}"></span>
      </span>
      ${escapeHtml(p.name)}
    </button>`;
  }).join('');
}

function settingToggleTheme() {
  const html = document.documentElement;
  const current = html.getAttribute('data-theme');
  const next = current === 'light' ? 'dark' : 'light';
  html.setAttribute('data-theme', next);
  prefs.setItem('theme', next);
  if (next === 'light') {
    // Clear all inline scheme vars so [data-theme="light"] CSS takes over
    for (const key of SCHEME_VAR_KEYS) {
      html.style.removeProperty(key);
    }
  } else {
    // Reapply color scheme for dark mode
    const scheme = prefs.getItem('colorScheme') || 'cyberpunk';
    if (scheme.startsWith('custom')) {
      const customVars = JSON.parse(prefs.getItem('customSchemeVars') || '{}');
      if (Object.keys(customVars).length > 0) applyCustomVars(customVars);
    } else {
      applyColorScheme(scheme);
    }
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
  if (!confirm('Clear all plugin, audio, and DAW scan history? This cannot be undone.')) return;
  await Promise.all([
    window.vstUpdater.clearHistory(),
    window.vstUpdater.clearAudioHistory(),
    window.vstUpdater.clearDawHistory(),
  ]);
}

async function settingClearKvrCache() {
  if (!confirm('Clear all cached KVR version lookups? Next update check will re-fetch everything.')) return;
  await window.vstUpdater.updateKvrCache([]);
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

function settingSaveSelect(key, value) {
  prefs.setItem(key, value);
}

function settingSaveCustomDirs() {
  const val = document.getElementById('settingCustomDirs').value.trim();
  prefs.setItem('customDirs', val);
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

  // Page size
  const pageSize = getSettingValue('pageSize', '500');
  const pageSizeEl = document.getElementById('settingPageSize');
  const pageSizeValEl = document.getElementById('settingPageSizeValue');
  if (pageSizeEl) {
    pageSizeEl.value = pageSize;
    pageSizeValEl.textContent = pageSize;
  }

  // Flush interval
  const flush = getSettingValue('flushInterval', '300');
  const flushEl = document.getElementById('settingFlushInterval');
  const flushValEl = document.getElementById('settingFlushIntervalValue');
  if (flushEl) {
    flushEl.value = flush;
    flushValEl.textContent = flush;
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

  // Custom scheme presets
  refreshCustomPresetUI();

  // Sync color pickers to current scheme (preset or custom)
  const customVars = JSON.parse(prefs.getItem('customSchemeVars') || '{}');
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
  // Only apply color scheme inline styles in dark mode;
  // light mode uses its own CSS vars via [data-theme="light"]
  if (saved !== 'light') {
    const scheme = prefs.getItem('colorScheme');
    if (scheme && scheme.startsWith('custom')) {
      const customVars = JSON.parse(prefs.getItem('customSchemeVars') || '{}');
      if (Object.keys(customVars).length > 0) {
        applyCustomVars(customVars);
      }
    } else if (scheme && scheme !== 'cyberpunk') {
      applyColorScheme(scheme);
    }
  }
  const pageSize = parseInt(prefs.getItem('pageSize') || '500', 10);
  AUDIO_PAGE_SIZE = pageSize;
  DAW_PAGE_SIZE = pageSize;
}
// restoreSettings is called from loadLastScan after prefs.load()
