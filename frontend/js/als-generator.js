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
  // Wizard step navigation
  // ---------------------------------------------------------------------------

  function showStep(step) {
    document.querySelectorAll('.als-wizard-panel').forEach(p => p.classList.remove('active'));
    document.querySelectorAll('.als-step-btn').forEach(b => b.classList.remove('active'));
    const panel = document.getElementById('alsStep' + step);
    if (panel) panel.classList.add('active');
    const btn = document.querySelector(`.als-step-btn[data-step="${step}"]`);
    if (btn) btn.classList.add('active');

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
    const chaos = document.getElementById('alsChaos');
    const chaosVal = document.getElementById('alsChaosValue');
    if (chaos) {
      chaos.value = defaults.chaos;
      if (chaosVal) chaosVal.textContent = (defaults.chaos / 100).toFixed(2);
    }
  }

  // ---------------------------------------------------------------------------
  // Build config from wizard state
  // ---------------------------------------------------------------------------

  function buildConfig() {
    const el = (id) => document.getElementById(id);
    const chk = (id) => el(id)?.checked || false;
    const num = (id, def) => parseInt(el(id)?.value || def, 10);
    return {
      genre: el('alsGenre')?.value || 'techno',
      hardness: (parseInt(el('alsHardness')?.value || '30', 10)) / 100,
      chaos: (parseInt(el('alsChaos')?.value || '30', 10)) / 100,
      glitch_intensity: (parseInt(el('alsGlitchIntensity')?.value || '0', 10)) / 100,
      density: (parseInt(el('alsDensity')?.value || '0', 10)) / 100,
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

    summary.innerHTML = `
      <div style="display:grid;grid-template-columns:auto 1fr;gap:4px 12px;">
        <span style="color:var(--text-dim);">Genre:</span><span>${config.genre}</span>
        <span style="color:var(--text-dim);">Hardness:</span><span>${config.hardness.toFixed(2)}</span>
        <span style="color:var(--text-dim);">Chaos:</span><span>${config.chaos.toFixed(2)}</span>
        <span style="color:var(--text-dim);">Glitch:</span><span>${config.glitch_intensity.toFixed(2)}</span>
        <span style="color:var(--text-dim);">Density:</span><span>${config.density.toFixed(2)}</span>
        <span style="color:var(--text-dim);">BPM:</span><span>${config.bpm}</span>
        <span style="color:var(--text-dim);">Key:</span><span>${keyStr}</span>
        <span style="color:var(--text-dim);">Songs:</span><span>${config.num_songs}</span>
        <span style="color:var(--text-dim);">Tracks (incl. groups):</span><span>${totalTracks}</span>
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
          if (resultEl) {
            const r = payload.result;
            resultEl.style.display = 'block';
            resultEl.innerHTML = `
              <div style="color:var(--cyan);font-weight:600;margin-bottom:8px;">Project created</div>
              <div style="font-size:12px;color:var(--text);">
                <div>${r.projectName}</div>
                <div style="margin-top:4px;color:var(--text-dim);">${r.tracks} tracks, ${r.clips} clips, ${r.bars} bars @ ${r.bpm} BPM</div>
                <div style="margin-top:4px;color:var(--text-dim);word-break:break-all;">${r.path}</div>
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
        if (result) {
          result.style.display = 'block';
          result.innerHTML = `
            <div style="color:var(--cyan);font-weight:600;margin-bottom:8px;">Project created</div>
            <div style="font-size:12px;color:var(--text);">
              <div>${res.projectName}</div>
              <div style="margin-top:4px;color:var(--text-dim);">${res.tracks} tracks, ${res.clips} clips, ${res.bars} bars @ ${res.bpm} BPM</div>
              <div style="margin-top:4px;color:var(--text-dim);word-break:break-all;">${res.path}</div>
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
    { id: 'alsChaos', type: 'value' },
    { id: 'alsGlitchIntensity', type: 'value' },
    { id: 'alsDensity', type: 'value' },
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
      updateTrackCountLabels();
      updateEstimatedTracks();
      const hv = document.getElementById('alsHardnessValue');
      const h = document.getElementById('alsHardness');
      if (hv && h) hv.textContent = (parseInt(h.value, 10) / 100).toFixed(2);
      const cv = document.getElementById('alsChaosValue');
      const c = document.getElementById('alsChaos');
      if (cv && c) cv.textContent = (parseInt(c.value, 10) / 100).toFixed(2);
      const gv = document.getElementById('alsGlitchIntensityValue');
      const g = document.getElementById('alsGlitchIntensity');
      if (gv && g) gv.textContent = (parseInt(g.value, 10) / 100).toFixed(2);
      const dv = document.getElementById('alsDensityValue');
      const d = document.getElementById('alsDensity');
      if (dv && d) dv.textContent = (parseInt(d.value, 10) / 100).toFixed(2);
    } catch (_) {}
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
    showStep(1);
    checkAnalysisStatus();
    updateBlacklistCount();
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
    } catch (e) {
      console.error('Failed to clear blacklist:', e);
    }
  }

  document.addEventListener('click', (e) => {
    // Handle clear blacklist button
    if (e.target.id === 'alsClearBlacklist') {
      e.preventDefault();
      clearBlacklist();
      return;
    }
    // Handle select/deselect all tonal
    if (e.target.id === 'alsTonalSelectAll') {
      e.preventDefault();
      setAllTonal(true);
      return;
    }
    if (e.target.id === 'alsTonalDeselectAll') {
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
    }
  });

  // Character sliders + track count inputs — save prefs on every change
  document.addEventListener('input', (e) => {
    const id = e.target.id;
    if (id === 'alsHardness') {
      const val = document.getElementById('alsHardnessValue');
      if (val) val.textContent = (parseInt(e.target.value, 10) / 100).toFixed(2);
    }
    if (id === 'alsChaos') {
      const val = document.getElementById('alsChaosValue');
      if (val) val.textContent = (parseInt(e.target.value, 10) / 100).toFixed(2);
    }
    if (id === 'alsGlitchIntensity') {
      const val = document.getElementById('alsGlitchIntensityValue');
      if (val) val.textContent = (parseInt(e.target.value, 10) / 100).toFixed(2);
    }
    if (id === 'alsDensity') {
      const val = document.getElementById('alsDensityValue');
      if (val) val.textContent = (parseInt(e.target.value, 10) / 100).toFixed(2);
    }
    if (id === 'alsGenre') onGenreChange();
    if (id?.startsWith('alsCount')) {
      updateTrackCountLabels();
      updateEstimatedTracks();
    }
    if (id?.startsWith('als')) saveAlsPrefs();
  });

  document.addEventListener('change', (e) => {
    if (e.target.id === 'alsGenre') onGenreChange();
    if (e.target.id?.startsWith('als')) saveAlsPrefs();
  });

  // Expose load function for tab switch
  window.loadAlsGenerator = loadAlsGenerator;
})();
