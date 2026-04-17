// ALS Generator wizard — 4-step UI for creating Ableton Live Set files
// Communicates with Rust backend via window.vstUpdater.*

(function () {
  'use strict';

  const ALS_SLIDER_MAX = 50;

  let _alsLoaded = false;
  let _analysisListenerAttached = false;
  let _generationListenerAttached = false;
  let _alsGenerating = false;

  // Genre defaults
  const GENRE_DEFAULTS = {
    techno:  { bpm: 132, hardness: 30, chaos: 30 },
    schranz: { bpm: 155, hardness: 80, chaos: 50 },
    trance:  { bpm: 140, hardness: 20, chaos: 20 },
  };

  // ---------------------------------------------------------------------------
  // Section visibility (no longer wizard panels - all visible)
  // ---------------------------------------------------------------------------

  function showStep(step) {
    // Legacy function - all sections now visible at once
    // Just trigger previews/summary load if needed
    if (step === 3) loadPreviews();
    if (step === 4) updateSummary();
  }

  // ---------------------------------------------------------------------------
  // Character slider labels
  // ---------------------------------------------------------------------------

  const CHAR_LABELS = [
    [0,  'clean'],
    [25, 'warm'],
    [50, 'balanced'],
    [75, 'aggressive'],
    [100, 'extreme'],
  ];

  function charLabel(val) {
    for (let i = CHAR_LABELS.length - 1; i >= 0; i--) {
      if (val >= CHAR_LABELS[i][0]) return CHAR_LABELS[i][1];
    }
    return 'balanced';
  }

  function updateCharLabels() {
    // No longer used - character sliders removed
  }

  // All per-type track count slider IDs
  const TYPE_COUNT_IDS = [
    'alsCountKick', 'alsCountClap', 'alsCountSnare', 'alsCountHat', 'alsCountPerc', 'alsCountRide', 'alsCountFill',
    'alsCountBass', 'alsCountSub',
    'alsCountLead', 'alsCountSynth', 'alsCountPad', 'alsCountArp',
    'alsCountRiser', 'alsCountDownlifter', 'alsCountCrash', 'alsCountImpact', 'alsCountHit', 'alsCountSweepUp', 'alsCountSweepDown',
    'alsCountSnareRoll', 'alsCountReverse', 'alsCountSubDrop', 'alsCountBoomKick', 'alsCountAtmos', 'alsCountGlitch', 'alsCountScatter',
    'alsCountVox',
  ];

  function updateTrackCountLabels() {
    for (const id of TYPE_COUNT_IDS) {
      const slider = document.getElementById(id);
      const label = document.getElementById(id + 'Label');
      if (slider && label) label.textContent = slider.value;
    }
  }

  function updateEstimatedTracks() {
    let total = 0;
    for (const id of TYPE_COUNT_IDS) {
      const el = document.getElementById(id);
      if (el) total += parseInt(el.value, 10) || 0;
    }
    const est = document.getElementById('alsEstimatedTracks');
    if (est) est.textContent = total + 5; // +5 for group tracks (DRUMS, BASS, BASS FX, MELODICS, FX)
  }

  // ---------------------------------------------------------------------------
  // Genre change handler
  // ---------------------------------------------------------------------------

  function onGenreChange() {
    const genre = document.getElementById('alsGenre');
    if (!genre) return;
    const defaults = GENRE_DEFAULTS[genre.value];
    if (!defaults) return;
    const bpm = document.getElementById('alsBpm');
    const hardness = document.getElementById('alsHardness');
    const hardnessVal = document.getElementById('alsHardnessValue');
    if (bpm) bpm.value = defaults.bpm;
    if (hardness) {
      hardness.value = defaults.hardness;
      if (hardnessVal) hardnessVal.textContent = (defaults.hardness / 100).toFixed(2);
    }
    // `defaults.chaos` used to seed the global Chaos slider; that slider was
    // retired when per-param overrides moved to the timeline. Section widths on
    // the timeline also depend on genre, so request a repaint.
    if (typeof window.renderAlsSectionOverridesTimeline === 'function') {
      window.renderAlsSectionOverridesTimeline();
    }
  }

  // ---------------------------------------------------------------------------
  // Build config from wizard state
  // ---------------------------------------------------------------------------

  function buildConfig() {
    const el = (id) => document.getElementById(id);
    const chk = (id) => el(id)?.checked || false;
    const num = (id, def) => parseInt(el(id)?.value || def, 10);
    // chaos / glitch_intensity / density / variation / parallelism: the five
    // global sliders have been retired — values now live exclusively in the
    // Section Overrides timeline (per-section). Sections without an override
    // fall back to the Rust serde defaults: chaos=0.3, glitch=0.0, density=0.0,
    // variation=0.0, parallelism=0.4 (see ProjectConfig in als_project.rs).
    return {
      genre: el('alsGenre')?.value || 'techno',
      hardness: (parseInt(el('alsHardness')?.value || '30', 10)) / 100,
      bpm: parseInt(el('alsBpm')?.value || '130', 10),
      root_note: el('alsAtonal')?.checked ? null : (el('alsRootNote')?.value || 'A'),
      mode: el('alsAtonal')?.checked ? null : (el('alsMode')?.value || 'Aeolian'),
      atonal: el('alsAtonal')?.checked || false,
      keywords: [],
      element_keywords: {},
      // Per-type track counts
      track_counts: {
        kick: num('alsCountKick', '1'),
        clap: num('alsCountClap', '1'),
        snare: num('alsCountSnare', '1'),
        hat: num('alsCountHat', '2'),
        perc: num('alsCountPerc', '2'),
        ride: num('alsCountRide', '1'),
        fill: num('alsCountFill', '4'),
        bass: num('alsCountBass', '1'),
        sub: num('alsCountSub', '1'),
        lead: num('alsCountLead', '1'),
        synth: num('alsCountSynth', '3'),
        pad: num('alsCountPad', '2'),
        arp: num('alsCountArp', '2'),
        riser: num('alsCountRiser', '3'),
        downlifter: num('alsCountDownlifter', '1'),
        crash: num('alsCountCrash', '2'),
        impact: num('alsCountImpact', '2'),
        hit: num('alsCountHit', '2'),
        sweep_up: num('alsCountSweepUp', '4'),
        sweep_down: num('alsCountSweepDown', '4'),
        snare_roll: num('alsCountSnareRoll', '1'),
        reverse: num('alsCountReverse', '2'),
        sub_drop: num('alsCountSubDrop', '2'),
        boom_kick: num('alsCountBoomKick', '2'),
        atmos: num('alsCountAtmos', '2'),
        glitch: num('alsCountGlitch', '2'),
        scatter: num('alsCountScatter', '4'),
        vox: num('alsCountVox', '1'),
      },
      // Legacy category counts (for backwards compat)
      tracks: {
        drums:  { count: 7, character: 0.5 },
        bass:   { count: 2, character: 0.5 },
        leads:  { count: 4, character: 0.5 },
        pads:   { count: 2, character: 0.5 },
        fx:     { count: 10, character: 0.5 },
        vocals: { count: 1, character: 0.5 },
      },
      output_path: el('alsOutputPath')?.value || '',
      project_name: el('alsProjectName')?.value?.trim() || null,
      num_songs: parseInt(el('alsNumSongs')?.value || '1', 10),
      type_atonal: {
        kick: !chk('alsTonalKick'),
        clap: !chk('alsTonalClap'),
        snare: !chk('alsTonalSnare'),
        hat: !chk('alsTonalHat'),
        perc: !chk('alsTonalPerc'),
        ride: !chk('alsTonalRide'),
        fill: !chk('alsTonalFill'),
        bass: !chk('alsTonalBass'),
        sub: !chk('alsTonalSub'),
        lead: !chk('alsTonalLead'),
        synth: !chk('alsTonalSynth'),
        pad: !chk('alsTonalPad'),
        arp: !chk('alsTonalArp'),
        riser: !chk('alsTonalRiser'),
        downlifter: !chk('alsTonalDownlifter'),
        crash: !chk('alsTonalCrash'),
        impact: !chk('alsTonalImpact'),
        hit: !chk('alsTonalHit'),
        sweep_up: !chk('alsTonalSweepUp'),
        sweep_down: !chk('alsTonalSweepDown'),
        snare_roll: !chk('alsTonalSnareRoll'),
        reverse: !chk('alsTonalReverse'),
        sub_drop: !chk('alsTonalSubDrop'),
        boom_kick: !chk('alsTonalBoomKick'),
        atmos: !chk('alsTonalAtmos'),
        glitch: !chk('alsTonalGlitch'),
        scatter: !chk('alsTonalScatter'),
        vox: !chk('alsTonalVox'),
      },
      // Per-section overrides for chaos/glitch/density/variation/parallelism/scatter
      // (each missing key = "use global scalar"). Sourced from the timeline editor
      // in als-timeline.js; shape matches Rust `SectionOverridesConfig`.
      section_overrides: typeof window.alsSectionOverridesForIpc === 'function'
        ? window.alsSectionOverridesForIpc()
        : { chaos: {}, glitch: {}, density: {}, variation: {}, parallelism: {}, scatter: {} },
      // Per-section bar lengths chosen by the user (draggable in the timeline
      // canvas). Falls back to genre defaults when the user hasn't touched
      // anything. Shape matches Rust `SectionLengths`.
      section_lengths: typeof window.alsSectionLengthsForIpc === 'function'
        ? window.alsSectionLengthsForIpc()
        : { intro: 32, build: 32, breakdown: 32, drop1: 32, drop2: 32, fadedown: 32, outro: 32 },
      // Seed — empty string or missing = let the backend pick a fresh random
      // one. We send as a STRING so u64 seeds above Number.MAX_SAFE_INTEGER
      // round-trip without precision loss; the Rust side deserializes either
      // form (see `deserialize_seed` in als_project.rs).
      seed: (el('alsSeed')?.value || '').trim() || null,
    };
  }

  // ---------------------------------------------------------------------------
  // Preview samples (Step 3)
  // ---------------------------------------------------------------------------

  async function loadPreviews() {
    const list = document.getElementById('alsPreviewList');
    if (!list || typeof window.vstUpdater?.alsQuerySamples !== 'function') return;
    list.innerHTML = '<p style="color:var(--text-dim);">Loading samples...</p>';

    const config = buildConfig();
    const categories = ['kick', 'sub_bass', 'mid_bass', 'lead', 'pad'];
    const labels = ['Kick', 'Sub Bass', 'Mid Bass', 'Main Lead', 'Main Pad'];

    let html = '';
    for (let i = 0; i < categories.length; i++) {
      try {
        const samples = await window.vstUpdater.alsQuerySamples(categories[i], config, 3);
        const sample = samples?.[0];
        const name = sample?.name || '(no sample found)';
        html += `<div style="display:flex;align-items:center;gap:8px;padding:8px 0;border-bottom:1px solid var(--border);">
          <span style="width:80px;color:var(--cyan);font-size:12px;font-weight:600;">${labels[i]}</span>
          <span style="flex:1;font-size:12px;color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${name}</span>
          <button class="btn btn-secondary" style="font-size:11px;padding:2px 8px;" data-action="alsShuffleSample" data-category="${categories[i]}" data-idx="${i}">Shuffle</button>
        </div>`;
      } catch (e) {
        html += `<div style="padding:8px 0;color:var(--text-dim);font-size:12px;">${labels[i]}: error loading</div>`;
      }
    }
    list.innerHTML = html || '<p style="color:var(--text-dim);">No samples available. Run sample analysis first.</p>';
  }

  // ---------------------------------------------------------------------------
  // Summary (Step 4)
  // ---------------------------------------------------------------------------

  function updateSummary() {
    const config = buildConfig();
    const summary = document.getElementById('alsSummary');
    if (!summary) return;

    const keyStr = `${config.root_note || 'A'} ${config.mode || 'Aeolian'}`;
    const tc = config.track_counts;
    const totalTracks = 5 + // group tracks (DRUMS, BASS, BASS FX, MELODICS, FX)
      tc.kick + tc.clap + tc.snare + tc.hat + tc.perc + tc.ride + tc.fill +
      tc.bass + tc.sub +
      tc.lead + tc.synth + tc.pad + tc.arp +
      tc.riser + tc.downlifter + tc.crash + tc.impact + tc.hit + tc.sweep_up + tc.sweep_down +
      tc.snare_roll + tc.reverse + tc.sub_drop + tc.boom_kick + tc.atmos + tc.glitch + tc.scatter +
      tc.vox;

    // Section override counts (per-param) replace the old scalar rows — the
    // 6 global sliders were retired when the Section Overrides timeline shipped.
    const so = config.section_overrides || {};
    const oCount = (p) => (so[p] ? Object.keys(so[p]).length : 0);
    const overridesLine = ['chaos', 'glitch', 'density', 'variation', 'parallelism', 'scatter']
      .map((p) => `${p[0].toUpperCase() + p.slice(1)}:${oCount(p)}`).join(' · ');

    summary.innerHTML = `
      <div style="display:grid;grid-template-columns:auto 1fr;gap:4px 12px;">
        <span style="color:var(--text-dim);">Genre:</span><span>${config.genre}</span>
        <span style="color:var(--text-dim);">Hardness:</span><span>${config.hardness.toFixed(2)}</span>
        <span style="color:var(--text-dim);">BPM:</span><span>${config.bpm}</span>
        <span style="color:var(--text-dim);">Key:</span><span>${keyStr}</span>
        <span style="color:var(--text-dim);">Songs:</span><span>${config.num_songs}</span>
        <span style="color:var(--text-dim);">Tracks (incl. groups):</span><span>${totalTracks}</span>
        <span style="color:var(--text-dim);">Section overrides:</span><span>${overridesLine}</span>
      </div>`;

    // Set default output path
    const outputEl = document.getElementById('alsOutputPath');
    if (outputEl && !outputEl.value) {
      outputEl.value = '~/Desktop';
    }
  }

  // ---------------------------------------------------------------------------
  // Generate
  // ---------------------------------------------------------------------------

  function resetGenerateUI() {
    const btn = document.getElementById('alsGenerateBtn');
    const cancelBtn = document.getElementById('alsCancelBtn');
    const spinner = document.getElementById('alsProgressSpinner');
    const filenameEl = document.getElementById('alsOutputFilename');
    const barWrap = document.getElementById('alsProgressBarWrap');
    const bar = document.getElementById('alsProgressBar');
    if (btn) { btn.disabled = false; btn.style.display = ''; }
    if (cancelBtn) cancelBtn.style.display = 'none';
    if (spinner) spinner.style.display = 'none';
    if (filenameEl) { filenameEl.style.display = 'none'; filenameEl.textContent = ''; }
    if (barWrap) barWrap.style.display = 'none';
    if (bar) bar.style.width = '0%';
    _alsGenerating = false;
  }

  async function cancelGeneration() {
    if (typeof window.vstUpdater?.cancelAlsGeneration === 'function') {
      await window.vstUpdater.cancelAlsGeneration();
    }
    const progressText = document.getElementById('alsProgressText');
    if (progressText) progressText.textContent = 'Cancelled';
    resetGenerateUI();
  }

  async function generateAls() {
    const btn = document.getElementById('alsGenerateBtn');
    const progress = document.getElementById('alsProgressWrap');
    const progressText = document.getElementById('alsProgressText');
    const result = document.getElementById('alsResult');

    if (!btn || typeof window.vstUpdater?.generateAlsProject !== 'function') return;
    if (_alsGenerating) return;
    _alsGenerating = true;

    const cancelBtn = document.getElementById('alsCancelBtn');
    btn.disabled = true;
    btn.style.display = 'none';
    if (cancelBtn) cancelBtn.style.display = '';
    if (progress) progress.style.display = 'block';
    if (result) result.style.display = 'none';
    if (progressText) progressText.textContent = 'Generating...';
    const spinner = document.getElementById('alsProgressSpinner');
    if (spinner) spinner.style.display = '';

    // Listen for progress
    if (!_generationListenerAttached && typeof window.vstUpdater.onAlsGenerationProgress === 'function') {
      _generationListenerAttached = true;
      console.log('[ALS] Progress listener attached');
      window.vstUpdater.onAlsGenerationProgress((payload) => {
        console.log('[ALS] Progress event:', payload);
        const spinner = document.getElementById('alsProgressSpinner');
        const filenameEl = document.getElementById('alsOutputFilename');
        const pText = document.getElementById('alsProgressText');
        const resultEl = document.getElementById('alsResult');
        console.log('[ALS] DOM elements:', { spinner: !!spinner, filenameEl: !!filenameEl, pText: !!pText, resultEl: !!resultEl });
        if (payload.phase === 'progress') {
          // Check if this is the "Building X.als" message
          if (payload.message.startsWith('Building ') && payload.message.endsWith('.als')) {
            if (filenameEl) {
              filenameEl.textContent = payload.message;
              filenameEl.style.display = '';
            }
          } else if (payload.message.startsWith('SAMPLE_PROGRESS:') || payload.message.startsWith('TRACK_PROGRESS:')) {
            // Parse progress: TYPE_PROGRESS:elapsed:total
            const parts = payload.message.split(':');
            const progressType = parts[0];
            const elapsed = parseInt(parts[1], 10);
            const total = parseInt(parts[2], 10);
            console.log('[ALS]', progressType, elapsed, '/', total);
            const barWrap = document.getElementById('alsProgressBarWrap');
            const bar = document.getElementById('alsProgressBar');
            const countEl = document.getElementById('alsProgressCount');
            const labelEl = document.getElementById('alsProgressLabel');
            if (barWrap) barWrap.style.display = '';
            if (labelEl) labelEl.textContent = progressType === 'SAMPLE_PROGRESS' ? 'Searching samples...' : 'Building tracks...';
            if (bar && total > 0) bar.style.width = `${(elapsed / total) * 100}%`;
            if (countEl) countEl.textContent = `${elapsed} / ${total}`;
          } else {
            console.log('[ALS] Updating progress text:', payload.message);
            if (pText) pText.textContent = payload.message;
            if (typeof showToast === 'function') {
              const isError = payload.message.startsWith('ERROR:');
              showToast(payload.message, isError ? 4000 : 2000, isError ? 'error' : undefined);
            }
          }
        } else if (payload.phase === 'completed' && payload.result) {
          if (pText) pText.textContent = 'Done!';
          if (spinner) spinner.style.display = 'none';
          applySeedFromResult(payload.result);
          if (resultEl) {
            const r = payload.result;
            resultEl.style.display = 'block';
            resultEl.innerHTML = `
              <div style="color:var(--cyan);font-weight:600;margin-bottom:8px;">Project created</div>
              <div style="font-size:12px;color:var(--text);">
                <div>${r.projectName}</div>
                <div style="margin-top:4px;color:var(--text-dim);">${r.tracks} tracks, ${r.clips} clips, ${r.bars} bars @ ${r.bpm} BPM</div>
                <div style="margin-top:4px;color:var(--text-dim);word-break:break-all;">${r.path}</div>
                ${r.seed ? `<div style="margin-top:4px;color:var(--text-dim);font-family:ui-monospace,SFMono-Regular,Menlo,monospace;">Seed: ${r.seed}</div>` : ''}
                ${r.warnings?.length ? '<div style="margin-top:4px;color:var(--accent);">Warnings: ' + r.warnings.join(', ') + '</div>' : ''}
                <button class="btn btn-primary" style="margin-top:8px;" data-action="alsOpenProject" data-path="${r.path}">Open in Ableton Live</button>
              </div>`;
            if (typeof showToast === 'function') showToast(`ALS project created: ${r.projectName}`);
          }
          resetGenerateUI();
        } else if (payload.phase === 'error') {
          if (pText) pText.textContent = 'Error: ' + payload.message;
          if (spinner) spinner.style.display = 'none';
          resetGenerateUI();
        }
      });
    }

    try {
      const config = buildConfig();
      if (progressText) progressText.textContent = 'Selecting samples and building arrangement...';
      const res = await window.vstUpdater.generateAlsProject(config);
      // Handle result directly in case event was missed
      if (res && !_alsGenerating) return; // event already handled it
      if (res) {
        if (progressText) progressText.textContent = 'Done!';
        if (spinner) spinner.style.display = 'none';
        applySeedFromResult(res);
        if (result) {
          result.style.display = 'block';
          result.innerHTML = `
            <div style="color:var(--cyan);font-weight:600;margin-bottom:8px;">Project created</div>
            <div style="font-size:12px;color:var(--text);">
              <div>${res.projectName}</div>
              <div style="margin-top:4px;color:var(--text-dim);">${res.tracks} tracks, ${res.clips} clips, ${res.bars} bars @ ${res.bpm} BPM</div>
              <div style="margin-top:4px;color:var(--text-dim);word-break:break-all;">${res.path}</div>
              ${res.seed ? `<div style="margin-top:4px;color:var(--text-dim);font-family:ui-monospace,SFMono-Regular,Menlo,monospace;">Seed: ${res.seed}</div>` : ''}
              ${res.warnings?.length ? '<div style="margin-top:4px;color:var(--accent);">Warnings: ' + res.warnings.join(', ') + '</div>' : ''}
              <button class="btn btn-primary" style="margin-top:8px;" data-action="alsOpenProject" data-path="${res.path}">Open in Ableton Live</button>
            </div>`;
          if (typeof showToast === 'function') showToast(`ALS project created: ${res.projectName}`);
          updateBlacklistCount(); // Refresh blacklist count after generation
        }
        _alsGenerating = false;
        btn.disabled = false;
        btn.textContent = 'Generate ALS';
      }
    } catch (e) {
      if (progressText) progressText.textContent = 'Error: ' + e;
      resetGenerateUI();
    }
  }

  // ---------------------------------------------------------------------------
  // Sample Analysis
  // ---------------------------------------------------------------------------

  // Status bar badge — same pattern as BPM/LUFS analysis badge
  function showBadge(detailKey, vars) {
    window.__statusBarSampleAnalysisJob = true;
    const badge = document.getElementById('bgSampleAnalysisBadge');
    if (badge) {
      badge.textContent = typeof formatBgJobBadgeLine === 'function'
        ? formatBgJobBadgeLine('sampleAnalysis', detailKey, vars)
        : detailKey;
    }
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
  }

  function hideBadge() {
    window.__statusBarSampleAnalysisJob = false;
    const badge = document.getElementById('bgSampleAnalysisBadge');
    if (badge) badge.textContent = '';
    if (typeof syncAppStatusBarVisibility === 'function') syncAppStatusBarVisibility();
  }

  function updateAnalysisUI(phase, payload) {
    const status = document.getElementById('alsAnalysisStatus');
    const startBtn = document.getElementById('alsAnalysisBtn');
    const stopBtn = document.getElementById('alsStopAnalysisBtn');

    if (phase === 'analyzing') {
      const pct = payload.total > 0 ? Math.round((payload.analyzed / payload.total) * 100) : 0;
      if (status) status.textContent = `${payload.analyzed} / ${payload.total} (${pct}%)`;
      showBadge('ui.stats.sample_analysis_progress', { n: payload.analyzed, total: payload.total });
    } else if (phase === 'completed' || phase === 'stopped') {
      if (status) status.textContent = `${payload.analyzed} / ${payload.total} — ${phase}`;
      if (startBtn) startBtn.style.display = '';
      if (stopBtn) stopBtn.style.display = 'none';
      hideBadge();
    } else if (phase === 'error') {
      if (status) status.textContent = 'Error: ' + (payload.message || 'unknown');
      hideBadge();
    } else if (phase === 'started') {
      if (status) status.textContent = 'Starting...';
      showBadge('ui.stats.sample_analysis_progress', { n: payload.analyzed || 0, total: payload.total || 0 });
    }
  }

  async function checkAnalysisStatus() {
    const status = document.getElementById('alsAnalysisStatus');
    const startBtn = document.getElementById('alsAnalysisBtn');
    if (!status || typeof window.vstUpdater?.sampleAnalysisStats !== 'function') return;
    try {
      console.log('[ALS] Checking analysis stats...');
      const stats = await window.vstUpdater.sampleAnalysisStats();
      console.log('[ALS] Stats:', stats);
      status.textContent = `${stats.analyzed} analyzed / ${stats.total} total`;
      if (startBtn && stats.unanalyzed > 0) startBtn.style.display = '';
      if (startBtn && stats.unanalyzed === 0) startBtn.style.display = 'none';
    } catch (e) {
      console.error('[ALS] Stats check failed:', e);
      status.textContent = 'unavailable';
    }
  }

  async function startAnalysis() {
    const startBtn = document.getElementById('alsAnalysisBtn');
    const stopBtn = document.getElementById('alsStopAnalysisBtn');
    if (typeof window.vstUpdater?.sampleAnalysisStart !== 'function') return;

    if (startBtn) startBtn.style.display = 'none';
    if (stopBtn) stopBtn.style.display = '';
    showBadge('ui.stats.sample_analysis_progress', { n: 0, total: 0 });

    if (!_analysisListenerAttached && typeof window.vstUpdater.onSampleAnalysisProgress === 'function') {
      _analysisListenerAttached = true;
      window.vstUpdater.onSampleAnalysisProgress((payload) => {
        updateAnalysisUI(payload.phase, payload);
      });
    }

    try {
      console.log('[ALS] Starting sample analysis...');
      const result = await window.vstUpdater.sampleAnalysisStart();
      console.log('[ALS] Analysis started:', result);
    } catch (e) {
      console.error('[ALS] Analysis start failed:', e);
      updateAnalysisUI('error', { message: String(e) });
      if (startBtn) startBtn.style.display = '';
      if (stopBtn) stopBtn.style.display = 'none';
    }
  }

  async function stopAnalysis() {
    if (typeof window.vstUpdater?.sampleAnalysisStop === 'function') {
      await window.vstUpdater.sampleAnalysisStop();
    }
  }

  // ---------------------------------------------------------------------------
  // Output folder picker
  // ---------------------------------------------------------------------------

  async function pickOutputFolder() {
    if (typeof window.__TAURI__?.dialog?.open !== 'function') return;
    const selected = await window.__TAURI__.dialog.open({ directory: true, title: 'Choose output folder' });
    if (selected) {
      const el = document.getElementById('alsOutputPath');
      if (el) { el.value = selected; saveAlsPrefs(); }
    }
  }

  // ---------------------------------------------------------------------------
  // Preferences persistence
  // ---------------------------------------------------------------------------

  const ALS_PREF_FIELDS = [
    { id: 'alsGenre', type: 'value' },
    { id: 'alsHardness', type: 'value' },
    // chaos/glitch/density/variation/parallelism persist through the timeline
    // (als-timeline.js → prefs key `alsSectionOverrides`), not through here.
    { id: 'alsBpm', type: 'value' },
    { id: 'alsRootNote', type: 'value' },
    { id: 'alsMode', type: 'value' },
    { id: 'alsAtonal', type: 'checked' },
    { id: 'alsOutputPath', type: 'value' },
    { id: 'alsProjectName', type: 'value' },
    { id: 'alsNumSongs', type: 'value' },
    // Per-type track counts
    { id: 'alsCountKick', type: 'value' },
    { id: 'alsCountClap', type: 'value' },
    { id: 'alsCountSnare', type: 'value' },
    { id: 'alsCountHat', type: 'value' },
    { id: 'alsCountPerc', type: 'value' },
    { id: 'alsCountRide', type: 'value' },
    { id: 'alsCountFill', type: 'value' },
    { id: 'alsCountBass', type: 'value' },
    { id: 'alsCountSub', type: 'value' },
    { id: 'alsCountLead', type: 'value' },
    { id: 'alsCountSynth', type: 'value' },
    { id: 'alsCountPad', type: 'value' },
    { id: 'alsCountArp', type: 'value' },
    { id: 'alsCountRiser', type: 'value' },
    { id: 'alsCountDownlifter', type: 'value' },
    { id: 'alsCountCrash', type: 'value' },
    { id: 'alsCountImpact', type: 'value' },
    { id: 'alsCountHit', type: 'value' },
    { id: 'alsCountSweepUp', type: 'value' },
    { id: 'alsCountSweepDown', type: 'value' },
    { id: 'alsCountSnareRoll', type: 'value' },
    { id: 'alsCountReverse', type: 'value' },
    { id: 'alsCountSubDrop', type: 'value' },
    { id: 'alsCountBoomKick', type: 'value' },
    { id: 'alsCountAtmos', type: 'value' },
    { id: 'alsCountGlitch', type: 'value' },
    { id: 'alsCountScatter', type: 'value' },
    { id: 'alsCountVox', type: 'value' },
    // Per-type tonal toggles
    { id: 'alsTonalKick', type: 'checked' },
    { id: 'alsTonalClap', type: 'checked' },
    { id: 'alsTonalSnare', type: 'checked' },
    { id: 'alsTonalHat', type: 'checked' },
    { id: 'alsTonalPerc', type: 'checked' },
    { id: 'alsTonalRide', type: 'checked' },
    { id: 'alsTonalFill', type: 'checked' },
    { id: 'alsTonalBass', type: 'checked' },
    { id: 'alsTonalSub', type: 'checked' },
    { id: 'alsTonalLead', type: 'checked' },
    { id: 'alsTonalSynth', type: 'checked' },
    { id: 'alsTonalPad', type: 'checked' },
    { id: 'alsTonalArp', type: 'checked' },
    { id: 'alsTonalRiser', type: 'checked' },
    { id: 'alsTonalDownlifter', type: 'checked' },
    { id: 'alsTonalCrash', type: 'checked' },
    { id: 'alsTonalImpact', type: 'checked' },
    { id: 'alsTonalHit', type: 'checked' },
    { id: 'alsTonalSweepUp', type: 'checked' },
    { id: 'alsTonalSweepDown', type: 'checked' },
    { id: 'alsTonalSnareRoll', type: 'checked' },
    { id: 'alsTonalReverse', type: 'checked' },
    { id: 'alsTonalSubDrop', type: 'checked' },
    { id: 'alsTonalBoomKick', type: 'checked' },
    { id: 'alsTonalAtmos', type: 'checked' },
    { id: 'alsTonalGlitch', type: 'checked' },
    { id: 'alsTonalScatter', type: 'checked' },
    { id: 'alsTonalVox', type: 'checked' },
    // Seed: saved as a string (full u64 precision). The lock checkbox gates
    // whether the seed is restored on next load — see `restoreAlsPrefs`.
    { id: 'alsSeed', type: 'value' },
    { id: 'alsSeedLocked', type: 'checked' },
    // Per-section overrides now live in the canvas timeline (als-timeline.js);
    // that module persists to prefs under `alsSectionOverrides`.
  ];

  function saveAlsPrefs() {
    if (typeof prefs === 'undefined') return;
    const data = {};
    for (const f of ALS_PREF_FIELDS) {
      const el = document.getElementById(f.id);
      if (!el) continue;
      data[f.id] = f.type === 'checked' ? el.checked : el.value;
    }
    prefs.setItem('alsGeneratorPrefs', JSON.stringify(data));
  }

  function restoreAlsPrefs() {
    if (typeof prefs === 'undefined') return;
    const raw = prefs.getItem('alsGeneratorPrefs');
    if (!raw) return;
    try {
      const data = JSON.parse(raw);
      for (const f of ALS_PREF_FIELDS) {
        if (!(f.id in data)) continue;
        const el = document.getElementById(f.id);
        if (!el) continue;
        if (f.type === 'checked') el.checked = !!data[f.id];
        else el.value = data[f.id];
      }
      // Seed: only restore the value if the user locked it. Otherwise drop
      // the stored seed on startup so a fresh random one is drawn — avoids
      // the footgun where a previous session's seed silently sticks around
      // across restarts and every generation produces the same arrangement.
      const seedEl = document.getElementById('alsSeed');
      const lockEl = document.getElementById('alsSeedLocked');
      if (seedEl && lockEl && !lockEl.checked) seedEl.value = '';
      updateTrackCountLabels();
      updateEstimatedTracks();
      const hv = document.getElementById('alsHardnessValue');
      const h = document.getElementById('alsHardness');
      if (hv && h) hv.textContent = (parseInt(h.value, 10) / 100).toFixed(2);
    } catch (_) {}
  }

  // After a successful generation, copy the backend-resolved seed into the
  // seed input so the user can see what was used and optionally lock it for
  // a "regenerate with same seed" run. `result.seed` is a string (the Rust
  // side serializes u64 as string to avoid JSON Number precision loss), so
  // we assign as-is. Also triggers `saveAlsPrefs` so the restored-on-reload
  // state stays coherent.
  function applySeedFromResult(result) {
    if (!result || result.seed == null) return;
    const el = document.getElementById('alsSeed');
    if (!el) return;
    el.value = String(result.seed);
    saveAlsPrefs();
  }

  // Generate a random decimal seed in the JS safe-integer range (0 .. 2^53-1)
  // and drop it into the seed input. Rust's `StdRng::seed_from_u64` treats
  // any u64 equally, so capping to 53 bits costs no real entropy. Capping
  // here (instead of using full u64) means the string round-trips losslessly
  // through JS `Number` in any future tool that reads it back.
  function randomizeAlsSeed() {
    const el = document.getElementById('alsSeed');
    if (!el) return;
    // Number.MAX_SAFE_INTEGER = 2^53 - 1 = 9007199254740991
    const seed = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
    el.value = String(seed);
    saveAlsPrefs();
    if (typeof updateSummary === 'function') updateSummary();
  }

  // ---------------------------------------------------------------------------
  // Tab load
  // ---------------------------------------------------------------------------

  function loadAlsGenerator() {
    if (_alsLoaded) return;
    _alsLoaded = true;
    // Apply slider max from constant
    for (const id of TYPE_COUNT_IDS) {
      const el = document.getElementById(id);
      if (el) el.max = ALS_SLIDER_MAX;
    }
    restoreAlsPrefs();
    updateTrackCountLabels();
    updateEstimatedTracks();
    // Section overrides timeline (canvas) — must init after its canvas is in the DOM.
    if (typeof window.initAlsSectionOverridesTimeline === 'function') {
      window.initAlsSectionOverridesTimeline();
    }
    loadPreviews();
    updateSummary();
    checkAnalysisStatus();
    updateBlacklistCount();
    updateWhitelistCount();
  }

  // ---------------------------------------------------------------------------
  // Event delegation
  // ---------------------------------------------------------------------------

  // All tonal checkbox IDs
  const TONAL_IDS = [
    'alsTonalKick', 'alsTonalClap', 'alsTonalSnare', 'alsTonalHat', 'alsTonalPerc', 'alsTonalRide', 'alsTonalFill',
    'alsTonalBass', 'alsTonalSub',
    'alsTonalLead', 'alsTonalSynth', 'alsTonalPad', 'alsTonalArp',
    'alsTonalRiser', 'alsTonalDownlifter', 'alsTonalCrash', 'alsTonalImpact', 'alsTonalHit', 'alsTonalSweepUp', 'alsTonalSweepDown',
    'alsTonalSnareRoll', 'alsTonalReverse', 'alsTonalSubDrop', 'alsTonalBoomKick', 'alsTonalAtmos', 'alsTonalGlitch', 'alsTonalScatter',
    'alsTonalVox',
  ];

  function setAllTonal(checked) {
    for (const id of TONAL_IDS) {
      const el = document.getElementById(id);
      if (el) el.checked = checked;
    }
    saveAlsPrefs();
  }

  // Update blacklist count display
  async function updateBlacklistCount() {
    const countEl = document.getElementById('alsBlacklistCount');
    if (!countEl || typeof window.vstUpdater?.getAlsBlacklistCount !== 'function') return;
    try {
      const count = await window.vstUpdater.getAlsBlacklistCount();
      countEl.textContent = count;
    } catch (e) {
      console.error('Failed to get blacklist count:', e);
    }
  }

  // Clear the sample blacklist
  async function clearBlacklist() {
    if (typeof window.vstUpdater?.clearAlsSampleBlacklist !== 'function') return;
    try {
      const result = await window.vstUpdater.clearAlsSampleBlacklist();
      console.log(`Cleared ${result.cleared} samples from blacklist`);
      await updateBlacklistCount();
      if (typeof showToast === 'function') {
        showToast(toastFmt('toast.blacklist_cleared', { count: result.cleared }), 2000);
      }
    } catch (e) {
      console.error('Failed to clear blacklist:', e);
      if (typeof showToast === 'function') showToast(toastFmt('toast.blacklist_clear_failed'), 3000, 'error');
    }
  }

  // ---------------------------------------------------------------------------
  // Blacklist Modal CRUD
  // ---------------------------------------------------------------------------

  let _blacklistEntries = [];

  async function openBlacklistModal() {
    const modal = document.getElementById('blacklistModal');
    if (!modal) return;
    modal.classList.add('modal-visible');
    await refreshBlacklistModal();
  }

  function closeBlacklistModal() {
    const modal = document.getElementById('blacklistModal');
    if (modal) modal.classList.remove('modal-visible');
  }

  async function refreshBlacklistModal() {
    const entriesEl = document.getElementById('blacklistEntries');
    const totalEl = document.getElementById('blacklistTotalCount');
    const filterEl = document.getElementById('blacklistFilterCount');
    const searchEl = document.getElementById('blacklistSearchInput');
    
    if (!entriesEl || typeof window.vstUpdater?.getAlsBlacklistEntries !== 'function') return;
    
    try {
      _blacklistEntries = await window.vstUpdater.getAlsBlacklistEntries();
      renderBlacklistEntries(searchEl?.value || '');
      if (totalEl) totalEl.textContent = `${_blacklistEntries.length} entries`;
    } catch (e) {
      entriesEl.innerHTML = `<p style="color:var(--red);">Error: ${e}</p>`;
    }
  }

  function renderBlacklistEntries(filter = '') {
    const entriesEl = document.getElementById('blacklistEntries');
    const filterEl = document.getElementById('blacklistFilterCount');
    if (!entriesEl) return;

    // Use fzf matching if available, otherwise fallback to substring
    let filtered;
    if (filter && typeof fzfMatch === 'function') {
      filtered = _blacklistEntries
        .map(entry => {
          const match = fzfMatch(filter, entry);
          return match ? { entry, score: match.score, indices: match.indices } : null;
        })
        .filter(Boolean)
        .sort((a, b) => b.score - a.score);
    } else if (filter) {
      const filterLower = filter.toLowerCase();
      filtered = _blacklistEntries
        .filter(e => e.toLowerCase().includes(filterLower))
        .map(entry => ({ entry, indices: [] }));
    } else {
      filtered = _blacklistEntries.map(entry => ({ entry, indices: [] }));
    }

    if (filterEl) {
      filterEl.textContent = filter ? `${filtered.length} / ${_blacklistEntries.length}` : '';
    }

    if (filtered.length === 0) {
      entriesEl.innerHTML = `<p style="color:var(--text-dim);">${_blacklistEntries.length === 0 ? 'Blacklist is empty' : 'No matches'}</p>`;
      return;
    }

    entriesEl.innerHTML = '';
    for (const { entry, indices } of filtered) {
      const row = document.createElement('div');
      row.style.cssText = 'display:flex;align-items:center;gap:8px;padding:4px 0;border-bottom:1px solid var(--border);';
      
      const textSpan = document.createElement('span');
      textSpan.style.cssText = 'flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
      textSpan.title = entry;
      textSpan.appendChild(highlightFzfMatch(entry, indices));
      
      const removeBtn = document.createElement('button');
      removeBtn.className = 'blacklist-remove-btn';
      removeBtn.dataset.entry = entry;
      removeBtn.style.cssText = 'background:transparent;border:1px solid var(--red);color:var(--red);padding:2px 6px;font-size:10px;cursor:pointer;';
      removeBtn.textContent = '×';
      
      row.appendChild(textSpan);
      row.appendChild(removeBtn);
      entriesEl.appendChild(row);
    }
  }

  // Highlight matched characters in fzf style
  function highlightFzfMatch(text, indices) {
    if (!indices || !indices.length) return document.createTextNode(text);
    const frag = document.createDocumentFragment();
    const idxSet = new Set(indices);
    let run = '';
    let inMatch = false;
    for (let i = 0; i < text.length; i++) {
      const m = idxSet.has(i);
      if (m !== inMatch) {
        if (run) {
          if (inMatch) {
            const sp = document.createElement('span');
            sp.style.cssText = 'color:var(--cyan);font-weight:bold;';
            sp.textContent = run;
            frag.appendChild(sp);
          } else {
            frag.appendChild(document.createTextNode(run));
          }
        }
        run = '';
        inMatch = m;
      }
      run += text[i];
    }
    if (run) {
      if (inMatch) {
        const sp = document.createElement('span');
        sp.style.cssText = 'color:var(--cyan);font-weight:bold;';
        sp.textContent = run;
        frag.appendChild(sp);
      } else {
        frag.appendChild(document.createTextNode(run));
      }
    }
    return frag;
  }

  async function addToBlacklist(path) {
    if (!path?.trim() || typeof window.vstUpdater?.addToAlsBlacklist !== 'function') return;
    try {
      await window.vstUpdater.addToAlsBlacklist(path.trim());
      await refreshBlacklistModal();
      await updateBlacklistCount();
    } catch (e) {
      console.error('Failed to add to blacklist:', e);
    }
  }

  async function removeFromBlacklist(entry) {
    if (!entry || typeof window.vstUpdater?.removeFromAlsBlacklist !== 'function') return;
    try {
      await window.vstUpdater.removeFromAlsBlacklist(entry);
      await refreshBlacklistModal();
      await updateBlacklistCount();
    } catch (e) {
      console.error('Failed to remove from blacklist:', e);
    }
  }

  // Blacklist modal event handlers — use `.closest()` so clicks on child text nodes
  // or inner spans still resolve to the target button / overlay.
  document.addEventListener('click', (e) => {
    // Open modal
    if (e.target.closest('#alsViewBlacklist')) {
      e.preventDefault();
      openBlacklistModal();
      return;
    }
    // Close modal (× button OR overlay background)
    if (e.target.closest('#blacklistModalClose')) {
      closeBlacklistModal();
      return;
    }
    if (e.target.id === 'blacklistModal') {
      closeBlacklistModal();
      return;
    }
    // Add entry
    if (e.target.closest('#blacklistAddBtn')) {
      const input = document.getElementById('blacklistAddInput');
      if (input?.value?.trim()) {
        addToBlacklist(input.value);
        input.value = '';
      }
      return;
    }
    // Remove entry
    const removeBtn = e.target.closest('.blacklist-remove-btn');
    if (removeBtn) {
      const entry = removeBtn.dataset.entry;
      if (entry) removeFromBlacklist(entry);
      return;
    }
    // Clear all from modal
    if (e.target.closest('#blacklistClearAllBtn')) {
      clearBlacklist().then(() => refreshBlacklistModal());
      return;
    }
  });

  // Search input
  document.addEventListener('input', (e) => {
    if (e.target.id === 'blacklistSearchInput') {
      renderBlacklistEntries(e.target.value);
    }
  });

  // Add on Enter key
  document.addEventListener('keydown', (e) => {
    if (e.target.id === 'blacklistAddInput' && e.key === 'Enter') {
      e.preventDefault();
      const input = e.target;
      if (input.value) {
        addToBlacklist(input.value);
        input.value = '';
      }
    }
    if (e.target.id === 'whitelistAddInput' && e.key === 'Enter') {
      e.preventDefault();
      const input = e.target;
      if (input.value) {
        addToWhitelist(input.value);
        input.value = '';
      }
    }
    // Close modals on Escape
    if (e.key === 'Escape') {
      const blacklistModal = document.getElementById('blacklistModal');
      if (blacklistModal && blacklistModal.classList.contains('modal-visible')) {
        closeBlacklistModal();
      }
      const whitelistModal = document.getElementById('whitelistModal');
      if (whitelistModal && whitelistModal.classList.contains('modal-visible')) {
        closeWhitelistModal();
      }
    }
  });

  // ---------------------------------------------------------------------------
  // Directory Whitelist Modal CRUD
  // ---------------------------------------------------------------------------

  let _whitelistEntries = [];

  async function updateWhitelistCount() {
    const countEl = document.getElementById('alsWhitelistCount');
    if (!countEl || typeof window.vstUpdater?.getAlsWhitelistCount !== 'function') return;
    try {
      const count = await window.vstUpdater.getAlsWhitelistCount();
      countEl.textContent = count;
    } catch (e) {
      console.error('Failed to get whitelist count:', e);
    }
  }

  async function openWhitelistModal() {
    const modal = document.getElementById('whitelistModal');
    if (!modal) return;
    modal.classList.add('modal-visible');
    await refreshWhitelistModal();
  }

  function closeWhitelistModal() {
    const modal = document.getElementById('whitelistModal');
    if (modal) modal.classList.remove('modal-visible');
  }

  async function refreshWhitelistModal() {
    const entriesEl = document.getElementById('whitelistEntries');
    const totalEl = document.getElementById('whitelistTotalCount');
    const searchEl = document.getElementById('whitelistSearchInput');
    
    if (!entriesEl || typeof window.vstUpdater?.getAlsWhitelistEntries !== 'function') return;
    
    try {
      _whitelistEntries = await window.vstUpdater.getAlsWhitelistEntries();
      renderWhitelistEntries(searchEl?.value || '');
      if (totalEl) {
        totalEl.textContent = _whitelistEntries.length === 0 
          ? '0 directories (all allowed)' 
          : `${_whitelistEntries.length} directories`;
      }
    } catch (e) {
      entriesEl.innerHTML = `<p style="color:var(--red);">Error: ${e}</p>`;
    }
  }

  function renderWhitelistEntries(filter = '') {
    const entriesEl = document.getElementById('whitelistEntries');
    const filterEl = document.getElementById('whitelistFilterCount');
    if (!entriesEl) return;

    // Use fzf matching if available
    let filtered;
    if (filter && typeof fzfMatch === 'function') {
      filtered = _whitelistEntries
        .map(entry => {
          const match = fzfMatch(filter, entry);
          return match ? { entry, score: match.score, indices: match.indices } : null;
        })
        .filter(Boolean)
        .sort((a, b) => b.score - a.score);
    } else if (filter) {
      const filterLower = filter.toLowerCase();
      filtered = _whitelistEntries
        .filter(e => e.toLowerCase().includes(filterLower))
        .map(entry => ({ entry, indices: [] }));
    } else {
      filtered = _whitelistEntries.map(entry => ({ entry, indices: [] }));
    }

    if (filterEl) {
      filterEl.textContent = filter ? `${filtered.length} / ${_whitelistEntries.length}` : '';
    }

    if (filtered.length === 0) {
      entriesEl.innerHTML = `<p style="color:var(--text-dim);">${_whitelistEntries.length === 0 ? 'No directories set (all samples allowed)' : 'No matches'}</p>`;
      return;
    }

    entriesEl.innerHTML = '';
    for (const { entry, indices } of filtered) {
      const row = document.createElement('div');
      row.style.cssText = 'display:flex;align-items:center;gap:8px;padding:4px 0;border-bottom:1px solid var(--border);';
      
      const textSpan = document.createElement('span');
      textSpan.style.cssText = 'flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
      textSpan.title = entry;
      textSpan.appendChild(highlightFzfMatch(entry, indices));
      
      const removeBtn = document.createElement('button');
      removeBtn.className = 'whitelist-remove-btn';
      removeBtn.dataset.entry = entry;
      removeBtn.style.cssText = 'background:transparent;border:1px solid var(--red);color:var(--red);padding:2px 6px;font-size:10px;cursor:pointer;';
      removeBtn.textContent = '×';
      
      row.appendChild(textSpan);
      row.appendChild(removeBtn);
      entriesEl.appendChild(row);
    }
  }

  async function addToWhitelist(path) {
    if (!path?.trim() || typeof window.vstUpdater?.addToAlsWhitelist !== 'function') return;
    try {
      await window.vstUpdater.addToAlsWhitelist(path.trim());
      await refreshWhitelistModal();
      await updateWhitelistCount();
    } catch (e) {
      console.error('Failed to add to whitelist:', e);
    }
  }

  async function removeFromWhitelist(entry) {
    if (!entry || typeof window.vstUpdater?.removeFromAlsWhitelist !== 'function') return;
    try {
      await window.vstUpdater.removeFromAlsWhitelist(entry);
      await refreshWhitelistModal();
      await updateWhitelistCount();
    } catch (e) {
      console.error('Failed to remove from whitelist:', e);
    }
  }

  async function clearWhitelist() {
    if (typeof window.vstUpdater?.clearAlsWhitelist !== 'function') return;
    try {
      const prevCount = parseInt(document.getElementById('alsWhitelistCount')?.textContent || '0', 10);
      await window.vstUpdater.clearAlsWhitelist();
      await updateWhitelistCount();
      if (typeof showToast === 'function') {
        showToast(toastFmt('toast.whitelist_cleared', { count: prevCount }), 2000);
      }
    } catch (e) {
      console.error('Failed to clear whitelist:', e);
      if (typeof showToast === 'function') showToast(toastFmt('toast.whitelist_clear_failed'), 3000, 'error');
    }
  }

  // Whitelist modal event handlers — `.closest()` for robustness.
  document.addEventListener('click', (e) => {
    // Open modal
    if (e.target.closest('#alsViewWhitelist')) {
      e.preventDefault();
      openWhitelistModal();
      return;
    }
    // Close modal
    if (e.target.closest('#whitelistModalClose')) {
      closeWhitelistModal();
      return;
    }
    if (e.target.id === 'whitelistModal') {
      closeWhitelistModal();
      return;
    }
    // Add entry
    if (e.target.closest('#whitelistAddBtn')) {
      const input = document.getElementById('whitelistAddInput');
      if (input?.value?.trim()) {
        addToWhitelist(input.value);
        input.value = '';
      }
      return;
    }
    // Browse for folder
    if (e.target.closest('#whitelistBrowseBtn')) {
      if (typeof window.__TAURI__?.dialog?.open === 'function') {
        window.__TAURI__.dialog.open({ directory: true, title: 'Choose sample source directory' }).then(path => {
          if (path) addToWhitelist(path);
        });
      }
      return;
    }
    // Remove entry
    const removeBtn = e.target.closest('.whitelist-remove-btn');
    if (removeBtn) {
      const entry = removeBtn.dataset.entry;
      if (entry) removeFromWhitelist(entry);
      return;
    }
    // Clear all from modal
    if (e.target.closest('#whitelistClearAllBtn')) {
      clearWhitelist().then(() => refreshWhitelistModal());
      return;
    }
    // Clear whitelist from main UI
    if (e.target.closest('#alsClearWhitelist')) {
      e.preventDefault();
      clearWhitelist();
      return;
    }
  });

  // Whitelist search input
  document.addEventListener('input', (e) => {
    if (e.target.id === 'whitelistSearchInput') {
      renderWhitelistEntries(e.target.value);
    }
  });

  document.addEventListener('click', (e) => {
    // Handle clear blacklist button
    if (e.target.closest('#alsClearBlacklist')) {
      e.preventDefault();
      clearBlacklist();
      return;
    }
    // Handle select/deselect all tonal
    if (e.target.closest('#alsTonalSelectAll')) {
      e.preventDefault();
      setAllTonal(true);
      return;
    }
    if (e.target.closest('#alsTonalDeselectAll')) {
      e.preventDefault();
      setAllTonal(false);
      return;
    }

    const action = e.target.closest('[data-action]')?.dataset?.action;
    if (!action) return;

    switch (action) {
      case 'alsWizardStep': {
        const step = e.target.closest('[data-step]')?.dataset?.step;
        if (step) showStep(parseInt(step, 10));
        break;
      }
      case 'alsGenerate':
        generateAls();
        break;
      case 'alsStartAnalysis':
        startAnalysis();
        break;
      case 'alsStopAnalysis':
        stopAnalysis();
        break;
      case 'alsPickOutput':
        pickOutputFolder();
        break;
      case 'alsCancelGenerate':
        cancelGeneration();
        break;
      case 'alsRandomizeSeed':
        randomizeAlsSeed();
        break;
    }
  });

  // Hardness slider + track count inputs — save prefs on every change.
  // Chaos/Glitch/Density/Variation/Parallelism are now handled by the timeline
  // canvas (als-timeline.js), so no input listeners here for those IDs.
  document.addEventListener('input', (e) => {
    const id = e.target.id;
    if (id === 'alsHardness') {
      const val = document.getElementById('alsHardnessValue');
      if (val) val.textContent = (parseInt(e.target.value, 10) / 100).toFixed(2);
    }
    if (id === 'alsGenre') onGenreChange();
    if (id?.startsWith('alsCount')) {
      updateTrackCountLabels();
      updateEstimatedTracks();
    }
    if (id?.startsWith('als')) {
      saveAlsPrefs();
      updateSummary();
    }
  });

  document.addEventListener('change', (e) => {
    if (e.target.id === 'alsGenre') onGenreChange();
    if (e.target.id?.startsWith('als')) {
      saveAlsPrefs();
      updateSummary();
    }
  });

  // Expose load function for tab switch
  window.loadAlsGenerator = loadAlsGenerator;
})();
