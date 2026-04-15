// ALS Generator wizard — 4-step UI for creating Ableton Live Set files
// Communicates with Rust backend via window.ipc.*

(function () {
  'use strict';

  let _alsLoaded = false;
  let _analysisListenerAttached = false;
  let _generationListenerAttached = false;

  // Genre defaults
  const GENRE_DEFAULTS = {
    techno:  { bpm: 132, hardness: 30 },
    schranz: { bpm: 155, hardness: 80 },
    trance:  { bpm: 140, hardness: 20 },
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
    const pairs = [
      ['alsDrumChar', 'alsDrumCharLabel'],
      ['alsBassChar', 'alsBassCharLabel'],
      ['alsLeadChar', 'alsLeadCharLabel'],
      ['alsPadChar',  'alsPadCharLabel'],
      ['alsFxChar',   'alsFxCharLabel'],
      ['alsVoxChar',  'alsVoxCharLabel'],
    ];
    for (const [sliderId, labelId] of pairs) {
      const slider = document.getElementById(sliderId);
      const label = document.getElementById(labelId);
      if (slider && label) label.textContent = charLabel(parseInt(slider.value, 10));
    }
  }

  function updateEstimatedTracks() {
    const ids = ['alsDrumCount', 'alsBassCount', 'alsLeadCount', 'alsPadCount', 'alsFxCount', 'alsVoxCount'];
    let total = 0;
    for (const id of ids) {
      const el = document.getElementById(id);
      if (el) total += parseInt(el.value, 10) || 0;
    }
    // Add atmos (always 1) + fills/risers/impacts estimate
    total += 5;
    const est = document.getElementById('alsEstimatedTracks');
    if (est) est.textContent = '~' + total;
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
  }

  // ---------------------------------------------------------------------------
  // Build config from wizard state
  // ---------------------------------------------------------------------------

  function buildConfig() {
    const el = (id) => document.getElementById(id);
    return {
      genre: el('alsGenre')?.value || 'techno',
      hardness: (parseInt(el('alsHardness')?.value || '30', 10)) / 100,
      bpm: parseInt(el('alsBpm')?.value || '130', 10),
      root_note: el('alsAtonal')?.checked ? null : (el('alsRootNote')?.value || 'A'),
      mode: el('alsAtonal')?.checked ? null : (el('alsMode')?.value || 'Aeolian'),
      atonal: el('alsAtonal')?.checked || false,
      keywords: [],
      element_keywords: {},
      tracks: {
        drums:  { count: parseInt(el('alsDrumCount')?.value || '3', 10), character: (parseInt(el('alsDrumChar')?.value || '50', 10)) / 100 },
        bass:   { count: parseInt(el('alsBassCount')?.value || '2', 10), character: (parseInt(el('alsBassChar')?.value || '50', 10)) / 100 },
        leads:  { count: parseInt(el('alsLeadCount')?.value || '2', 10), character: (parseInt(el('alsLeadChar')?.value || '50', 10)) / 100 },
        pads:   { count: parseInt(el('alsPadCount')?.value || '2', 10),  character: (parseInt(el('alsPadChar')?.value || '50', 10)) / 100 },
        fx:     { count: parseInt(el('alsFxCount')?.value || '6', 10),   character: (parseInt(el('alsFxChar')?.value || '50', 10)) / 100 },
        vocals: { count: parseInt(el('alsVoxCount')?.value || '0', 10),  character: (parseInt(el('alsVoxChar')?.value || '50', 10)) / 100 },
      },
      output_path: el('alsOutputPath')?.value || '',
      project_name: null,
      num_songs: parseInt(el('alsNumSongs')?.value || '1', 10),
    };
  }

  // ---------------------------------------------------------------------------
  // Preview samples (Step 3)
  // ---------------------------------------------------------------------------

  async function loadPreviews() {
    const list = document.getElementById('alsPreviewList');
    if (!list || typeof window.ipc?.alsQuerySamples !== 'function') return;
    list.innerHTML = '<p style="color:var(--text-dim);">Loading samples...</p>';

    const config = buildConfig();
    const categories = ['kick', 'sub_bass', 'mid_bass', 'lead', 'pad'];
    const labels = ['Kick', 'Sub Bass', 'Mid Bass', 'Main Lead', 'Main Pad'];

    let html = '';
    for (let i = 0; i < categories.length; i++) {
      try {
        const samples = await window.ipc.alsQuerySamples(categories[i], config, 3);
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

    const keyStr = config.atonal ? 'Atonal' : `${config.root_note} ${config.mode}`;
    const totalTracks = config.tracks.drums.count + config.tracks.bass.count +
      config.tracks.leads.count + config.tracks.pads.count +
      config.tracks.fx.count + config.tracks.vocals.count + 5;

    summary.innerHTML = `
      <div style="display:grid;grid-template-columns:auto 1fr;gap:4px 12px;">
        <span style="color:var(--text-dim);">Genre:</span><span>${config.genre}</span>
        <span style="color:var(--text-dim);">Hardness:</span><span>${config.hardness.toFixed(2)}</span>
        <span style="color:var(--text-dim);">BPM:</span><span>${config.bpm}</span>
        <span style="color:var(--text-dim);">Key:</span><span>${keyStr}</span>
        <span style="color:var(--text-dim);">Songs:</span><span>${config.num_songs}</span>
        <span style="color:var(--text-dim);">Tracks:</span><span>~${totalTracks}</span>
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

  async function generateAls() {
    const btn = document.getElementById('alsGenerateBtn');
    const progress = document.getElementById('alsProgressWrap');
    const progressBar = document.getElementById('alsProgressBar');
    const progressText = document.getElementById('alsProgressText');
    const result = document.getElementById('alsResult');

    if (!btn || typeof window.ipc?.generateAlsProject !== 'function') return;

    btn.disabled = true;
    btn.textContent = 'Generating...';
    if (progress) progress.style.display = 'block';
    if (result) result.style.display = 'none';
    if (progressBar) progressBar.style.width = '0%';
    if (progressText) progressText.textContent = 'Starting...';

    // Listen for progress
    if (!_generationListenerAttached && typeof window.ipc.onAlsGenerationProgress === 'function') {
      _generationListenerAttached = true;
      window.ipc.onAlsGenerationProgress((payload) => {
        if (payload.phase === 'completed' && payload.result) {
          if (progressBar) progressBar.style.width = '100%';
          if (progressText) progressText.textContent = 'Done!';
          if (result) {
            const r = payload.result;
            result.style.display = 'block';
            result.innerHTML = `
              <div style="color:var(--cyan);font-weight:600;margin-bottom:8px;">Project created</div>
              <div style="font-size:12px;color:var(--text);">
                <div>${r.projectName}</div>
                <div style="margin-top:4px;color:var(--text-dim);">${r.tracks} tracks, ${r.clips} clips, ${r.bars} bars @ ${r.bpm} BPM</div>
                <div style="margin-top:4px;color:var(--text-dim);word-break:break-all;">${r.path}</div>
                ${r.warnings?.length ? '<div style="margin-top:4px;color:var(--accent);">Warnings: ' + r.warnings.join(', ') + '</div>' : ''}
              </div>`;
          }
          btn.disabled = false;
          btn.textContent = 'Generate ALS';
        } else if (payload.phase === 'error') {
          if (progressText) progressText.textContent = 'Error: ' + payload.message;
          btn.disabled = false;
          btn.textContent = 'Generate ALS';
        }
      });
    }

    try {
      const config = buildConfig();
      if (progressBar) progressBar.style.width = '50%';
      if (progressText) progressText.textContent = 'Selecting samples and building arrangement...';
      await window.ipc.generateAlsProject(config);
    } catch (e) {
      if (progressText) progressText.textContent = 'Error: ' + e;
      btn.disabled = false;
      btn.textContent = 'Generate ALS';
    }
  }

  // ---------------------------------------------------------------------------
  // Sample Analysis
  // ---------------------------------------------------------------------------

  async function checkAnalysisStatus() {
    const status = document.getElementById('alsAnalysisStatus');
    const startBtn = document.getElementById('alsAnalysisBtn');
    if (!status || typeof window.ipc?.sampleAnalysisStats !== 'function') return;
    try {
      const stats = await window.ipc.sampleAnalysisStats();
      status.textContent = `${stats.analyzed} analyzed / ${stats.total} total`;
      if (startBtn && stats.unanalyzed > 0) startBtn.style.display = '';
    } catch (e) {
      status.textContent = 'unavailable';
    }
  }

  async function startAnalysis() {
    const startBtn = document.getElementById('alsAnalysisBtn');
    const stopBtn = document.getElementById('alsStopAnalysisBtn');
    const status = document.getElementById('alsAnalysisStatus');
    if (typeof window.ipc?.sampleAnalysisStart !== 'function') return;

    if (startBtn) startBtn.style.display = 'none';
    if (stopBtn) stopBtn.style.display = '';

    if (!_analysisListenerAttached && typeof window.ipc.onSampleAnalysisProgress === 'function') {
      _analysisListenerAttached = true;
      window.ipc.onSampleAnalysisProgress((payload) => {
        if (status) {
          if (payload.phase === 'analyzing') {
            status.textContent = `${payload.analyzed} / ${payload.total} (${payload.keys_detected || 0} keys detected)`;
          } else if (payload.phase === 'completed' || payload.phase === 'stopped') {
            status.textContent = `${payload.analyzed} / ${payload.total} — ${payload.phase}`;
            if (startBtn) startBtn.style.display = '';
            if (stopBtn) stopBtn.style.display = 'none';
          } else if (payload.phase === 'error') {
            status.textContent = 'Error: ' + payload.message;
          }
        }
      });
    }

    try {
      await window.ipc.sampleAnalysisStart();
    } catch (e) {
      if (status) status.textContent = 'Error: ' + e;
      if (startBtn) startBtn.style.display = '';
      if (stopBtn) stopBtn.style.display = 'none';
    }
  }

  async function stopAnalysis() {
    if (typeof window.ipc?.sampleAnalysisStop === 'function') {
      await window.ipc.sampleAnalysisStop();
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
      if (el) el.value = selected;
    }
  }

  // ---------------------------------------------------------------------------
  // Tab load
  // ---------------------------------------------------------------------------

  function loadAlsGenerator() {
    if (_alsLoaded) return;
    _alsLoaded = true;
    showStep(1);
    checkAnalysisStatus();
  }

  // ---------------------------------------------------------------------------
  // Event delegation
  // ---------------------------------------------------------------------------

  document.addEventListener('click', (e) => {
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
    }
  });

  // Character sliders + track count inputs
  document.addEventListener('input', (e) => {
    const id = e.target.id;
    if (id === 'alsHardness') {
      const val = document.getElementById('alsHardnessValue');
      if (val) val.textContent = (parseInt(e.target.value, 10) / 100).toFixed(2);
    }
    if (id === 'alsGenre') onGenreChange();
    if (id?.endsWith('Char')) updateCharLabels();
    if (id?.endsWith('Count')) updateEstimatedTracks();
  });

  document.addEventListener('change', (e) => {
    if (e.target.id === 'alsGenre') onGenreChange();
  });

  // Expose load function for tab switch
  window.loadAlsGenerator = loadAlsGenerator;
})();
