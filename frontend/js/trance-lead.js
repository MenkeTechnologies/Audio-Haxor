/**
 * Trance Lead Generator pane — generates MIDI leads and finds matching samples.
 *
 * Lives inside the ALS Generator tab as a full-width section.
 * Uses IPC: generateMidiLead, generateTranceStarter, findTranceSamples.
 */
(function () {
  'use strict';

  const NOTES = ['C','C#','D','D#','E','F','F#','G','G#','A','A#','B'];

  // ── Minor key presets ──────────────────────────────────────────────
  // Numerals: lowercase = minor chord, UPPERCASE = major chord
  // Special: IV = borrowed major IV in minor, bII = Neapolitan, bVII = borrowed
  const MINOR_PRESETS = [
    { label: 'Trance Standard',       chords: ['i','III','VII','VI'],                   ref: 'Above & Beyond - Sun & Moon' },
    { label: 'Uplifting Staple',       chords: ['i','VI','III','VII'],                   ref: 'Armin - Shivers' },
    { label: 'Classic Uplifting',      chords: ['i','VII','VI','VII'],                   ref: 'Tiesto - Adagio for Strings' },
    { label: 'Emotional Minor',        chords: ['i','iv','VI','VII'],                    ref: 'Dash Berlin - Till The Sky Falls Down' },
    { label: 'Dark Uplifting',         chords: ['i','VI','iv','VII'],                    ref: 'Gareth Emery - Concrete Angel' },
    { label: 'Vocal Trance',           chords: ['i','III','VI','VII'],                   ref: 'Oceanlab - Satellite' },
    { label: 'Psy Crossover',          chords: ['i','VII','i','VII'],                    ref: 'Astrix - He.Art' },
    { label: 'Borrowed Major IV',      chords: ['i','IV','VI','VII'],                    ref: 'PvD - For an Angel' },
    { label: 'Anthemic Minor',         chords: ['i','v','VI','III'],                     ref: 'Andrew Rayel - My Reflection' },
    { label: 'Minor Plagal Loop',      chords: ['i','iv','i','VII'],                     ref: 'ATB - Ecstasy' },
    { label: 'Chromatic Bass',         chords: ['i','III','iv','IV'],                    ref: 'Armin - In and Out of Love' },
    { label: 'Dark Melodic',           chords: ['i','ii','VI','VII'],                    ref: 'RAM - RAMelia' },
    { label: 'Neapolitan Psy',         chords: ['i','bII','i','VII'],                    ref: 'Vini Vici - Great Spirit' },
    { label: 'Extended Uplifting',     chords: ['i','III','VII','VI','i','iv','VI','VII'], ref: 'Rank 1 - Airwave' },
    { label: 'Extended Emotional',     chords: ['i','VI','III','VII','iv','VI','VII','i'], ref: 'Chicane - Saltwater' },
    { label: 'i iv v III',             chords: ['i','iv','v','III'],                     ref: 'Classic trance' },
    { label: 'i v VI IV',              chords: ['i','v','VI','IV'],                      ref: 'Pop-trance crossover' },
  ];

  // ── Major key presets ────────────────────────────────────────────
  const MAJOR_PRESETS = [
    { label: 'Major Uplifting',        chords: ['I','V','vi','IV'],                      ref: 'ATB - 9pm' },
    { label: 'Euphoric Major',         chords: ['I','IV','V','vi'],                      ref: 'Dash Berlin - Waiting' },
    { label: 'Bright Trance',          chords: ['I','V','IV','V'],                       ref: 'A&B - Thing Called Love' },
    { label: 'Major Anthemic',         chords: ['I','vi','IV','V'],                      ref: 'Gareth Emery - Saving Light' },
    { label: 'Major Minor Borrow',     chords: ['I','V','vi','iii'],                     ref: 'Chicane - Offshore' },
    { label: 'Lydian Color',           chords: ['I','II','vi','IV'],                     ref: 'Oceanlab - On a Good Day' },
    { label: 'Extended Major',         chords: ['I','IV','vi','V','IV','I'],             ref: 'Tiesto - Elements of Life' },
    { label: 'I V IV vi',              chords: ['I','V','IV','vi'],                      ref: 'Classic euphoric' },
  ];

  // ── Roman numeral → semitone offset ──────────────────────────────
  // Minor: i=0, ii=2, III=3, iv=5, v=7, VI=8, VII=10
  // Specials: IV=5 (borrowed major), bII=1 (Neapolitan), bVII=10
  const ROMAN_MINOR = {
    'i':0, 'I':0, 'ii':2, 'II':2, 'bII':1,
    'iii':3, 'III':3, 'iv':5, 'IV':5,
    'v':7, 'V':7, 'vi':8, 'VI':8, 'bVII':10,
    'vii':10, 'VII':10,
  };
  // Major: I=0, ii=2, iii=4, IV=5, V=7, vi=9, vii=11
  // Specials: II=2 (Lydian borrowed)
  const ROMAN_MAJOR = {
    'i':0, 'I':0, 'ii':2, 'II':2,
    'iii':4, 'III':4, 'iv':5, 'IV':5,
    'v':7, 'V':7, 'vi':9, 'VI':9,
    'vii':11, 'VII':11,
  };

  function romanToChordNames(numerals, keyRoot, isMinor) {
    const table = isMinor ? ROMAN_MINOR : ROMAN_MAJOR;
    return numerals.map(r => {
      const offset = table[r];
      if (offset == null) return r;
      const pc = (keyRoot + offset) % 12;
      const isMinorChord = r === r.toLowerCase();
      return NOTES[pc] + (isMinorChord ? 'm' : '');
    });
  }

  const TL_PREF_FIELDS = [
    'tlKey', 'tlScale', 'tlLeadType', 'tlProgression', 'tlBarsPerChord',
    'tlBpm', 'tlVariations', 'tlSeed', 'tlPerLayer', 'tlOutputPath', 'tlLengthBars',
    'tlChromaticism',
  ];

  const LAYER_LABELS = {
    kick: 'Kick', mid_bass: 'Bass', pad: 'Pad', arp: 'Arp', pluck: 'Pluck',
    lead: 'Lead', vocal: 'Vocal', vocal_chop: 'Vocal Chop',
    vocal_atmosphere: 'Vocal Atmosphere', vocal_phrase: 'Vocal Phrase',
    fx_riser: 'Riser', fx_downer: 'Downlifter', fx_impact: 'Impact',
    fx_crash: 'Crash', atmos: 'Atmos',
  };

  // ── Preferences ──────────────────────────────────────────────────

  function savePrefs() {
    if (typeof window.vstUpdater?.prefsSet !== 'function') return;
    for (const id of TL_PREF_FIELDS) {
      const el = document.getElementById(id);
      if (!el) continue;
      const val = el.type === 'checkbox' ? el.checked : el.value;
      window.vstUpdater.prefsSet(id, String(val));
    }
  }

  function restorePrefs() {
    if (typeof window.vstUpdater?.prefsGetAll !== 'function') return;
    window.vstUpdater.prefsGetAll().then(prefs => {
      if (!prefs) return;
      for (const id of TL_PREF_FIELDS) {
        const val = prefs[id];
        if (val == null) continue;
        const el = document.getElementById(id);
        if (!el) continue;
        if (el.type === 'checkbox') el.checked = val === 'true';
        else el.value = val;
      }
      updateTotalBars();
      populatePresetDropdown();
    });
  }

  // ── Progression presets ──────────────────────────────────────────

  function populatePresetDropdown() {
    const sel = document.getElementById('tlPreset');
    if (!sel) return;
    const keyRoot = parseInt(document.getElementById('tlKey')?.value || '9', 10);
    const isMinor = (document.getElementById('tlScale')?.value || 'minor') === 'minor';
    const presets = isMinor ? MINOR_PRESETS : MAJOR_PRESETS;
    while (sel.options.length > 1) sel.remove(1);
    for (let i = 0; i < presets.length; i++) {
      const p = presets[i];
      const names = romanToChordNames(p.chords, keyRoot, isMinor);
      const opt = document.createElement('option');
      opt.value = String(i);
      opt.textContent = names.join(' ') + '  \u2014 ' + p.label;
      sel.appendChild(opt);
    }
  }

  function applyPreset() {
    const sel = document.getElementById('tlPreset');
    const input = document.getElementById('tlProgression');
    if (!sel || !input || sel.value === '') return;
    const idx = parseInt(sel.value, 10);
    const keyRoot = parseInt(document.getElementById('tlKey')?.value || '9', 10);
    const isMinor = (document.getElementById('tlScale')?.value || 'minor') === 'minor';
    const presets = isMinor ? MINOR_PRESETS : MAJOR_PRESETS;
    const preset = presets[idx];
    if (!preset) return;
    const names = romanToChordNames(preset.chords, keyRoot, isMinor);
    input.value = names.join(' ');
    updateTotalBars();
    savePrefs();
  }

  // ── Total bars display ───────────────────────────────────────────

  function getChordCount() {
    const str = (document.getElementById('tlProgression')?.value || '').trim();
    if (!str) return 0;
    return str.split(/[\s,]+/).filter(Boolean).length;
  }

  function updateTotalBars() {
    const el = document.getElementById('tlTotalBars');
    if (!el) return;
    const lengthEl = document.getElementById('tlLengthBars');
    const lengthVal = parseInt(lengthEl?.value || '0', 10);
    if (lengthVal > 0) {
      el.textContent = `= ${lengthVal} bars`;
      return;
    }
    const chords = getChordCount();
    const bpc = parseInt(document.getElementById('tlBarsPerChord')?.value || '2', 10);
    const total = chords * bpc;
    el.textContent = total > 0 ? `= ${total} bars` : '';
  }

  function updateChromaticismLabel() {
    const el = document.getElementById('tlChromaticismValue');
    const slider = document.getElementById('tlChromaticism');
    if (el && slider) el.textContent = slider.value + '%';
  }

  // ── Config builders ──────────────────────────────────────────────

  function buildMidiConfig() {
    const progStr = (document.getElementById('tlProgression')?.value || 'Am Dm Em C').trim();
    const progression = progStr.split(/[\s,]+/).filter(Boolean);
    const lengthVal = parseInt(document.getElementById('tlLengthBars')?.value || '0', 10);
    return {
      keyRoot: parseInt(document.getElementById('tlKey')?.value || '9', 10),
      minor: (document.getElementById('tlScale')?.value || 'minor') === 'minor',
      leadType: document.getElementById('tlLeadType')?.value || 'two_layer',
      chords: [],
      progression,
      bpm: parseInt(document.getElementById('tlBpm')?.value || '120', 10),
      barsPerChord: parseInt(document.getElementById('tlBarsPerChord')?.value || '2', 10),
      lengthBars: lengthVal > 0 ? lengthVal : null,
      chromaticism: parseInt(document.getElementById('tlChromaticism')?.value || '15', 10),
      seed: parseInt(document.getElementById('tlSeed')?.value || '42', 10),
      name: 'Trance Lead',
      variations: parseInt(document.getElementById('tlVariations')?.value || '5', 10),
    };
  }

  function buildStarterConfig() {
    return {
      keyRoot: parseInt(document.getElementById('tlKey')?.value || '9', 10),
      minor: (document.getElementById('tlScale')?.value || 'minor') === 'minor',
      perLayer: parseInt(document.getElementById('tlPerLayer')?.value || '10', 10),
      midiConfig: buildMidiConfig(),
    };
  }

  function getOutputDir() {
    return document.getElementById('tlOutputPath')?.value || '';
  }

  // ── Actions ──────────────────────────────────────────────────────

  function toast(msg, dur, type) {
    if (typeof showToast === 'function') showToast(msg, dur, type);
  }
  function fmt(key, vars) {
    return typeof toastFmt === 'function' ? toastFmt(key, vars) : key;
  }

  async function generateMidi() {
    const dir = getOutputDir();
    if (!dir) { await pickOutput(); return generateMidi(); }
    const config = buildMidiConfig();
    toast(fmt('toast.tl_generating'), 2000);
    try {
      const result = await window.vstUpdater.generateMidiLead(config, dir);
      const files = Array.isArray(result) ? result : [];
      showMidiResults(files);
      toast(fmt('toast.tl_midi_generated', {n: files.length}), 3000, 'success');
    } catch (e) {
      console.error('generateMidiLead failed:', e);
      toast(fmt('toast.tl_error', {err: e.message || e}), 4000, 'error');
    }
  }

  async function findSamples() {
    const config = {
      keyRoot: parseInt(document.getElementById('tlKey')?.value || '9', 10),
      minor: (document.getElementById('tlScale')?.value || 'minor') === 'minor',
      perLayer: parseInt(document.getElementById('tlPerLayer')?.value || '10', 10),
    };
    toast(fmt('toast.tl_finding_samples'), 2000);
    try {
      const result = await window.vstUpdater.findTranceSamples(config);
      showSampleResults(result);
      const total = (result.layers || []).reduce((s, l) => s + (l.samples || []).length, 0);
      toast(fmt('toast.tl_samples_found', {n: total}), 3000, 'success');
    } catch (e) {
      console.error('findTranceSamples failed:', e);
      toast(fmt('toast.tl_error', {err: e.message || e}), 4000, 'error');
    }
  }

  async function generateAll() {
    const dir = getOutputDir();
    if (!dir) { await pickOutput(); return generateAll(); }
    const config = buildStarterConfig();
    toast(fmt('toast.tl_generating_all'), 2000);
    try {
      const result = await window.vstUpdater.generateTranceStarter(config, dir);
      const midi = result.midiFiles || [];
      showMidiResults(midi);
      showSampleResults(result);
      const total = (result.layers || []).reduce((s, l) => s + (l.samples || []).length, 0);
      toast(fmt('toast.tl_starter_done', {midi: midi.length, samples: total}), 3000, 'success');
    } catch (e) {
      console.error('generateTranceStarter failed:', e);
      toast(fmt('toast.tl_error', {err: e.message || e}), 4000, 'error');
    }
  }

  async function generateKits() {
    const dir = getOutputDir();
    if (!dir) { await pickOutput(); return generateKits(); }
    const progStr = (document.getElementById('tlProgression')?.value || 'Am Dm Em C').trim();
    const progression = progStr.split(/[\s,]+/).filter(Boolean);
    const lengthVal = parseInt(document.getElementById('tlLengthBars')?.value || '0', 10);
    const config = {
      keyRoot: parseInt(document.getElementById('tlKey')?.value || '9', 10),
      minor: (document.getElementById('tlScale')?.value || 'minor') === 'minor',
      progression,
      chords: [],
      bpm: parseInt(document.getElementById('tlBpm')?.value || '120', 10),
      barsPerChord: parseInt(document.getElementById('tlBarsPerChord')?.value || '2', 10),
      lengthBars: lengthVal > 0 ? lengthVal : null,
      chromaticism: parseInt(document.getElementById('tlChromaticism')?.value || '15', 10),
      seed: parseInt(document.getElementById('tlSeed')?.value || '42', 10),
      numKits: parseInt(document.getElementById('tlVariations')?.value || '5', 10),
      layers: [],
    };
    toast(fmt('toast.tl_generating_kits', {n: config.numKits}), 2000);
    try {
      const kits = await window.vstUpdater.generateMidiKits(config, dir);
      const kitArr = Array.isArray(kits) ? kits : [];
      showKitResults(kitArr);
      const totalFiles = kitArr.reduce((s, k) => s + (k.files || []).length, 0);
      toast(fmt('toast.tl_kits_done', {kits: kitArr.length, files: totalFiles}), 3000, 'success');
    } catch (e) {
      console.error('generateMidiKits failed:', e);
      toast(fmt('toast.tl_error', {err: e.message || e}), 4000, 'error');
    }
  }

  async function pickOutput() {
    if (typeof window.__TAURI__?.dialog?.open !== 'function') return;
    const selected = await window.__TAURI__.dialog.open({ directory: true, title: 'Choose MIDI output folder' });
    if (selected) {
      const el = document.getElementById('tlOutputPath');
      if (el) el.value = selected;
      savePrefs();
    }
  }

  function randomizeSeed() {
    const el = document.getElementById('tlSeed');
    if (el) {
      el.value = Math.floor(Math.random() * 2147483647);
      savePrefs();
    }
  }

  // ── Rendering ────────────────────────────────────────────────────

  function showMidiResults(files) {
    const wrap = document.getElementById('tlResults');
    const container = document.getElementById('tlMidiResults');
    if (!wrap || !container) return;
    wrap.hidden = false;

    if (!files.length) {
      container.innerHTML = '';
      return;
    }

    container.innerHTML = files.map(f => {
      const name = (f.path || '').split('/').pop() || 'untitled.mid';
      const kb = f.size ? `${(f.size / 1024).toFixed(1)} KB` : '';
      return `<span class="tl-midi-file">&#127925; ${esc(name)} <span style="color:var(--text-dim)">${kb}</span></span>`;
    }).join('');
  }

  function showSampleResults(result) {
    const wrap = document.getElementById('tlResults');
    const keysEl = document.getElementById('tlCompatibleKeys');
    const layersEl = document.getElementById('tlLayerResults');
    if (!wrap || !keysEl || !layersEl) return;
    wrap.hidden = false;

    if (result.compatibleKeys && result.compatibleKeys.length) {
      keysEl.textContent = 'Compatible keys: ' + result.compatibleKeys.join(', ');
    }

    const layers = result.layers || [];
    if (!layers.length) {
      layersEl.innerHTML = '<div class="tl-no-samples">No sample analysis data found. Run Sample Analysis first.</div>';
      return;
    }

    layersEl.innerHTML = layers.map(layer => {
      const label = LAYER_LABELS[layer.layer] || layer.layer;
      const badgeClass = layer.keyMatched ? 'key-matched' : 'no-match';
      const badgeText = layer.isTonal
        ? (layer.keyMatched ? 'Key Matched' : 'No Key Match')
        : 'Atonal';

      const rows = (layer.samples || []).map(s => {
        const key = s.parsed_key || '';
        return `<div class="tl-sample-row" data-path="${esc(s.path)}" title="${esc(s.path)}">
          <span class="tl-sample-name">${esc(s.name)}</span>
          <span class="tl-sample-key">${esc(key)}</span>
        </div>`;
      }).join('');

      const noSamples = (layer.samples || []).length === 0
        ? '<div class="tl-no-samples">No samples found</div>'
        : '';

      return `<div class="tl-layer-card">
        <div class="tl-layer-head">
          <span class="tl-layer-name">${esc(label)}</span>
          <span class="tl-layer-badge ${badgeClass}">${badgeText}</span>
        </div>
        ${rows}${noSamples}
      </div>`;
    }).join('');
  }

  function showKitResults(kits) {
    const wrap = document.getElementById('tlResults');
    const container = document.getElementById('tlMidiResults');
    const layersEl = document.getElementById('tlLayerResults');
    if (!wrap || !container) return;
    wrap.hidden = false;
    if (layersEl) layersEl.innerHTML = '';
    document.getElementById('tlCompatibleKeys').textContent = '';

    if (!kits.length) { container.innerHTML = ''; return; }

    container.innerHTML = kits.map(kit => {
      const fileList = (kit.files || []).map(f => {
        const kb = f.size ? `${(f.size / 1024).toFixed(1)}KB` : '';
        return `<span class="tl-midi-file">&#127925; ${esc(f.layer)} <span style="color:var(--text-dim)">${kb}</span></span>`;
      }).join('');
      return `<div style="margin-bottom:12px;">
        <div style="font-size:13px;font-weight:600;color:var(--cyan);margin-bottom:4px;">${esc(kit.name)}</div>
        <div>${fileList}</div>
      </div>`;
    }).join('');
  }

  function esc(s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  }

  // ── Event delegation ─────────────────────────────────────────────

  document.addEventListener('click', e => {
    const action = e.target.closest('[data-action]')?.dataset.action;
    if (!action) return;
    switch (action) {
      case 'tlGenerateMidi': generateMidi(); break;
      case 'tlFindSamples': findSamples(); break;
      case 'tlGenerateAll': generateAll(); break;
      case 'tlGenerateKits': generateKits(); break;
      case 'tlPickOutput': pickOutput(); break;
      case 'tlRandomizeSeed': randomizeSeed(); break;
    }
  });

  document.addEventListener('input', e => {
    if (TL_PREF_FIELDS.includes(e.target.id)) savePrefs();
    if (['tlProgression', 'tlBarsPerChord', 'tlLengthBars'].includes(e.target.id)) updateTotalBars();
    if (e.target.id === 'tlChromaticism') updateChromaticismLabel();
  });
  document.addEventListener('change', e => {
    if (TL_PREF_FIELDS.includes(e.target.id)) savePrefs();
    if (e.target.id === 'tlPreset') applyPreset();
    if (['tlKey', 'tlScale'].includes(e.target.id)) {
      populatePresetDropdown(); // rebuild labels for new key
      if (document.getElementById('tlPreset')?.value) applyPreset();
    }
    if (['tlProgression', 'tlBarsPerChord', 'tlLengthBars'].includes(e.target.id)) updateTotalBars();
  });

  // ── Init ─────────────────────────────────────────────────────────

  function initTranceLead() {
    restorePrefs();
    populatePresetDropdown();
    updateTotalBars();
    updateChromaticismLabel();
  }

  window.initTranceLead = initTranceLead;
})();
