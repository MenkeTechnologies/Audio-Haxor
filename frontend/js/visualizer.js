// ── Audio Visualizer Tab ──
// 6 real-time displays: FFT, waveform, spectrogram, stereo, levels, bands.
// Grid (all) or single mode. Fullscreen. Trello drag to rearrange. Context menus.

let _vizMode = 'all';
let _vizRAF = null;
let _vizSpectrogramData = [];
let _vizLastFrame = 0;
const _VIZ_FPS = 30;
const _VIZ_INTERVAL = 1000 / _VIZ_FPS;
// Pre-allocated buffers (set once when analyser is available)
let _vizFreqData = null;
let _vizTimeData = null;
let _vizParams = {
  fftSmoothing: 0.8,
  fftLogScale: true,
  waveformColor: 'cyan',
  spectrogramSpeed: 1,
  levelsHold: true,
  bandsCount: 10,
};
let _vizPeakHold = -96;
let _vizPeakTimer = null;

// ── Mode switching ──
document.addEventListener('click', (e) => {
  const btn = e.target.closest('.viz-mode-btn');
  if (!btn || btn.dataset.action) return;
  const mode = btn.dataset.vizMode;
  if (!mode) return;

  document.querySelectorAll('.viz-mode-btn[data-viz-mode]').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  _vizMode = mode;
  _vizSpectrogramData = [];

  const grid = document.getElementById('vizGrid');
  if (!grid) return;
  if (mode === 'all') {
    grid.classList.remove('viz-single');
    grid.querySelectorAll('.viz-tile').forEach(t => t.classList.remove('viz-tile-active'));
  } else {
    grid.classList.add('viz-single');
    grid.querySelectorAll('.viz-tile').forEach(t => {
      t.classList.toggle('viz-tile-active', t.dataset.vizTile === mode);
    });
  }
  _resizeCanvases();
});

// ── Fullscreen ──
document.addEventListener('click', (e) => {
  if (e.target.closest('[data-action="vizFullscreen"]')) {
    const container = document.getElementById('vizContainer');
    if (!container) return;
    container.classList.toggle('viz-fullscreen');
    _resizeCanvases();
  }
});

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    const container = document.getElementById('vizContainer');
    if (container?.classList.contains('viz-fullscreen')) {
      container.classList.remove('viz-fullscreen');
      _resizeCanvases();
    }
  }
});

function _resizeCanvases() {
  requestAnimationFrame(() => {
    document.querySelectorAll('.viz-tile-canvas').forEach(canvas => {
      const rect = canvas.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        canvas.width = Math.floor(rect.width * (window.devicePixelRatio || 1));
        canvas.height = Math.floor(rect.height * (window.devicePixelRatio || 1));
      }
    });
  });
}

// ── Start/Stop ──
function startVisualizer() {
  _resizeCanvases();
  if (_vizRAF) return;
  const empty = document.getElementById('vizEmpty');
  if (empty) empty.style.display = 'none';
  _vizLoop();
}

function stopVisualizer() {
  if (_vizRAF) { cancelAnimationFrame(_vizRAF); _vizRAF = null; }
}

// ── Main render loop ──
function _vizLoop(timestamp) {
  _vizRAF = requestAnimationFrame(_vizLoop);

  // Throttle to target FPS
  if (timestamp - _vizLastFrame < _VIZ_INTERVAL) return;
  _vizLastFrame = timestamp;

  const tab = document.getElementById('tabVisualizer');
  if (!tab || !tab.classList.contains('active')) return;

  const analyser = typeof _analyser !== 'undefined' ? _analyser : null;
  const isPlaying = typeof audioPlayer !== 'undefined' && !audioPlayer.paused;
  const empty = document.getElementById('vizEmpty');
  if (empty) empty.style.display = (analyser && isPlaying) ? 'none' : '';

  if (!analyser || !isPlaying) return;

  // Ensure pre-allocated buffers match analyser
  if (!_vizFreqData || _vizFreqData.length !== analyser.frequencyBinCount) {
    _vizFreqData = new Uint8Array(analyser.frequencyBinCount);
    _vizTimeData = new Float32Array(analyser.fftSize);
  }
  // Smoothing only affects frequency data, not time domain — set for FFT/bands
  analyser.smoothingTimeConstant = _vizParams.fftSmoothing;

  if (_vizMode === 'all') {
    // Read data once, share across all tiles
    analyser.getByteFrequencyData(_vizFreqData);
    analyser.getFloatTimeDomainData(_vizTimeData);
    _drawTile('fft', analyser);
    _drawTile('waveform', analyser);
    _drawTile('spectrogram', analyser);
    _drawTile('stereo', analyser);
    _drawTile('levels', analyser);
    _drawTile('bands', analyser);
  } else {
    _drawTile(_vizMode, analyser);
  }
}

function _drawTile(mode, analyser) {
  const canvas = document.querySelector(`.viz-tile-canvas[data-viz="${mode}"]`);
  if (!canvas || canvas.width === 0) return;
  const ctx = canvas.getContext('2d');
  const w = canvas.width, h = canvas.height;

  switch (mode) {
    case 'fft': _drawFFT(ctx, w, h, analyser); break;
    case 'waveform': _drawWaveform(ctx, w, h, analyser); break;
    case 'spectrogram': _drawSpectrogram(ctx, w, h, analyser); break;
    case 'stereo': _drawStereo(ctx, w, h, analyser); break;
    case 'levels': _drawLevels(ctx, w, h, analyser); break;
    case 'bands': _drawBands(ctx, w, h, analyser); break;
  }
}

// ── FFT Spectrum ──
function _drawFFT(ctx, w, h, analyser) {
  const bufLen = analyser.frequencyBinCount;
  if (_vizMode !== 'all') { if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen); analyser.getByteFrequencyData(_vizFreqData); }
  const data = _vizFreqData;
  ctx.clearRect(0, 0, w, h);

  if (_vizParams.fftLogScale) {
    // Log-frequency display
    const minF = 20, maxF = 20000;
    const logMin = Math.log10(minF), logMax = Math.log10(maxF);
    const sr = 44100;
    for (let x = 0; x < w; x++) {
      const logF = logMin + (x / w) * (logMax - logMin);
      const freq = Math.pow(10, logF);
      const bin = Math.round(freq / (sr / 2) * bufLen);
      if (bin >= bufLen) continue;
      const barH = (data[bin] / 255) * h;
      const t = x / w;
      ctx.fillStyle = `rgb(${Math.floor(5 + t * 206)},${Math.floor(217 - t * 167)},${Math.floor(232 - t * 35)})`;
      ctx.fillRect(x, h - barH, 1, barH);
    }
    // Frequency grid
    ctx.fillStyle = 'rgba(122,139,168,0.4)';
    ctx.font = `${Math.max(8, h / 40)}px sans-serif`;
    ctx.textAlign = 'center';
    [50, 100, 200, 500, 1000, 2000, 5000, 10000].forEach(f => {
      const x = ((Math.log10(f) - logMin) / (logMax - logMin)) * w;
      ctx.fillText(f >= 1000 ? (f / 1000) + 'k' : f + '', x, h - 2);
    });
  } else {
    const barW = w / bufLen;
    for (let i = 0; i < bufLen; i++) {
      const barH = (data[i] / 255) * h;
      const t = i / bufLen;
      ctx.fillStyle = `rgb(${Math.floor(5 + t * 206)},${Math.floor(217 - t * 167)},${Math.floor(232 - t * 35)})`;
      ctx.fillRect(i * barW, h - barH, barW - 0.5, barH);
    }
  }
}

// ── Waveform / Oscilloscope ──
function _drawWaveform(ctx, w, h, analyser) {
  const bufLen = analyser.fftSize;
  if (_vizMode !== 'all') { if (!_vizTimeData || _vizTimeData.length !== bufLen) _vizTimeData = new Float32Array(bufLen); analyser.getFloatTimeDomainData(_vizTimeData); }
  const data = _vizTimeData;
  ctx.clearRect(0, 0, w, h);

  const color = _vizParams.waveformColor === 'magenta' ? 'rgba(211,0,197,0.8)' :
    _vizParams.waveformColor === 'green' ? 'rgba(57,255,20,0.8)' : 'rgba(5,217,232,0.8)';
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  const sliceW = w / bufLen;
  for (let i = 0; i < bufLen; i++) {
    const x = i * sliceW;
    const y = (0.5 - data[i] * 0.5) * h;
    if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
  }
  ctx.stroke();

  // Center line + grid
  ctx.strokeStyle = 'rgba(122,139,168,0.12)';
  ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(0, h / 2); ctx.lineTo(w, h / 2); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(0, h / 4); ctx.lineTo(w, h / 4); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(0, h * 3 / 4); ctx.lineTo(w, h * 3 / 4); ctx.stroke();
}

// ── Scrolling Spectrogram ──
function _drawSpectrogram(ctx, w, h, analyser) {
  const bufLen = analyser.frequencyBinCount;
  if (_vizMode !== 'all') { if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen); analyser.getByteFrequencyData(_vizFreqData); }
  const data = _vizFreqData;

  _vizSpectrogramData.push(Array.from(data));
  const maxCols = Math.floor(w / _vizParams.spectrogramSpeed);
  if (_vizSpectrogramData.length > maxCols) _vizSpectrogramData.splice(0, _vizSpectrogramData.length - maxCols);

  ctx.clearRect(0, 0, w, h);
  const cols = _vizSpectrogramData.length;
  const colW = w / maxCols;
  for (let col = 0; col < cols; col++) {
    const cd = _vizSpectrogramData[col];
    const x = (maxCols - cols + col) * colW;
    for (let bin = 0; bin < bufLen; bin++) {
      const mag = cd[bin] / 255;
      if (mag < 0.015) continue;
      const y = h - (bin / bufLen) * h;
      const binH = Math.max(1, Math.ceil(h / bufLen));
      const r = Math.floor(mag * 211 + (1 - mag) * 5);
      const g = Math.floor(mag * mag * 50);
      const b = Math.floor(mag * 197 + (1 - mag) * 20);
      ctx.fillStyle = `rgba(${r},${g},${b},${mag * 0.9 + 0.1})`;
      ctx.fillRect(x, y - binH, Math.max(1, colW), binH);
    }
  }
}

// ── Stereo Field ──
// Pre-allocated stereo buffers
let _vizLeftData = null;
let _vizRightData = null;

function _drawStereo(ctx, w, h, analyser) {
  const aL = window._analyserL;
  const aR = window._analyserR;
  if (!aL || !aR) return;

  const bufLen = aL.fftSize;
  if (!_vizLeftData || _vizLeftData.length !== bufLen) {
    _vizLeftData = new Float32Array(bufLen);
    _vizRightData = new Float32Array(bufLen);
  }
  aL.getFloatTimeDomainData(_vizLeftData);
  aR.getFloatTimeDomainData(_vizRightData);

  ctx.clearRect(0, 0, w, h);
  const cx = w / 2, cy = h / 2;
  const scale = Math.min(cx, cy) * 0.8;

  // Grid
  ctx.strokeStyle = 'rgba(122,139,168,0.12)';
  ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(cx, 0); ctx.lineTo(cx, h); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(0, cy); ctx.lineTo(w, cy); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(cx - scale, cy - scale); ctx.lineTo(cx + scale, cy + scale); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(cx + scale, cy - scale); ctx.lineTo(cx - scale, cy + scale); ctx.stroke();

  // Plot true L vs R
  ctx.fillStyle = 'rgba(5,217,232,0.35)';
  for (let i = 0; i < bufLen; i++) {
    const l = _vizLeftData[i];
    const r = _vizRightData[i];
    const mid = (l + r) * 0.5;
    const side = (l - r) * 0.5;
    ctx.fillRect(cx + side * scale - 0.5, cy - mid * scale - 0.5, 1.5, 1.5);
  }

  ctx.fillStyle = 'rgba(122,139,168,0.4)';
  ctx.font = `${Math.max(9, h / 30)}px sans-serif`;
  ctx.textAlign = 'center';
  ctx.fillText('L', 12, cy + 4);
  ctx.fillText('R', w - 12, cy + 4);
  ctx.fillText('MONO', cx, 14);
}

// ── Level Meters ──
function _drawLevels(ctx, w, h, analyser) {
  const bufLen = analyser.fftSize;
  if (_vizMode !== 'all') { if (!_vizTimeData || _vizTimeData.length !== bufLen) _vizTimeData = new Float32Array(bufLen); analyser.getFloatTimeDomainData(_vizTimeData); }
  const data = _vizTimeData;

  let sumSq = 0, peak = 0;
  for (let i = 0; i < bufLen; i++) {
    sumSq += data[i] * data[i];
    const abs = Math.abs(data[i]);
    if (abs > peak) peak = abs;
  }
  const rms = Math.sqrt(sumSq / bufLen);
  const rmsDb = rms > 0 ? 20 * Math.log10(rms) : -96;
  const peakDb = peak > 0 ? 20 * Math.log10(peak) : -96;

  // Peak hold
  if (_vizParams.levelsHold) {
    if (peakDb > _vizPeakHold) { _vizPeakHold = peakDb; clearTimeout(_vizPeakTimer); _vizPeakTimer = setTimeout(() => { _vizPeakHold = -96; }, 2000); }
  }

  ctx.clearRect(0, 0, w, h);
  const meterW = Math.min(80, w / 4);
  const meterH = h - 50;
  const startY = 25;

  const grad = ctx.createLinearGradient(0, startY + meterH, 0, startY);
  grad.addColorStop(0, 'rgba(57,255,20,0.8)');
  grad.addColorStop(0.6, 'rgba(249,240,2,0.8)');
  grad.addColorStop(0.85, 'rgba(255,107,53,0.8)');
  grad.addColorStop(1, 'rgba(255,7,58,0.9)');

  const drawMeter = (x, db, label) => {
    const pct = Math.max(0, Math.min(1, (db + 60) / 60));
    const barH = pct * meterH;
    ctx.fillStyle = 'rgba(10,10,20,0.5)';
    ctx.fillRect(x, startY, meterW, meterH);
    ctx.fillStyle = grad;
    ctx.fillRect(x, startY + meterH - barH, meterW, barH);
    ctx.fillStyle = 'rgba(224,240,255,0.8)';
    ctx.font = `${Math.max(10, h / 30)}px Orbitron, sans-serif`;
    ctx.textAlign = 'center';
    ctx.fillText(label, x + meterW / 2, startY - 6);
    ctx.font = `${Math.max(9, h / 35)}px Share Tech Mono, monospace`;
    ctx.fillText(db.toFixed(1) + ' dB', x + meterW / 2, startY + meterH + 16);
  };

  drawMeter(w / 2 - meterW - 15, rmsDb, 'RMS');
  drawMeter(w / 2 + 15, peakDb, 'PEAK');

  // Peak hold indicator
  if (_vizParams.levelsHold && _vizPeakHold > -96) {
    const holdPct = Math.max(0, Math.min(1, (_vizPeakHold + 60) / 60));
    const holdY = startY + meterH - holdPct * meterH;
    ctx.strokeStyle = 'rgba(255,7,58,0.9)';
    ctx.lineWidth = 2;
    ctx.beginPath(); ctx.moveTo(w / 2 + 15, holdY); ctx.lineTo(w / 2 + 15 + meterW, holdY); ctx.stroke();
  }

  // dB scale
  ctx.fillStyle = 'rgba(122,139,168,0.3)';
  ctx.font = '8px sans-serif';
  ctx.textAlign = 'right';
  for (let db = 0; db >= -60; db -= 6) {
    const y = startY + meterH * (1 - (db + 60) / 60);
    ctx.fillText(db + '', w / 2 - meterW - 20, y + 3);
  }
}

// ── Octave Bands ──
function _drawBands(ctx, w, h, analyser) {
  const bufLen = analyser.frequencyBinCount;
  if (_vizMode !== 'all') { if (!_vizFreqData || _vizFreqData.length !== bufLen) _vizFreqData = new Uint8Array(bufLen); analyser.getByteFrequencyData(_vizFreqData); }
  const data = _vizFreqData;

  const sr = 44100;
  const binFreq = sr / analyser.fftSize;
  const bands = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
  const labels = ['31', '63', '125', '250', '500', '1k', '2k', '4k', '8k', '16k'];

  ctx.clearRect(0, 0, w, h);
  const bandW = (w - 30) / bands.length;
  const maxH = h - 35;

  for (let i = 0; i < bands.length; i++) {
    const cf = bands[i];
    const lo = Math.floor((cf / Math.sqrt(2)) / binFreq);
    const hi = Math.ceil((cf * Math.sqrt(2)) / binFreq);
    let sum = 0, cnt = 0;
    for (let k = Math.max(0, lo); k <= Math.min(hi, bufLen - 1); k++) { sum += data[k]; cnt++; }
    const avg = cnt > 0 ? sum / cnt : 0;
    const barH = (avg / 255) * maxH;
    const x = 15 + i * bandW;
    const t = i / bands.length;
    ctx.fillStyle = `rgba(${Math.floor(5 + t * 206)},${Math.floor(217 - t * 167)},${Math.floor(232 - t * 35)},0.8)`;
    ctx.fillRect(x + 2, h - 20 - barH, bandW - 4, barH);

    ctx.fillStyle = 'rgba(224,240,255,0.6)';
    ctx.font = `${Math.max(8, h / 35)}px sans-serif`;
    ctx.textAlign = 'center';
    ctx.fillText(labels[i], x + bandW / 2, h - 5);
  }
}

// ── Context menus for visualizer tiles ──
document.addEventListener('contextmenu', (e) => {
  const tile = e.target.closest('.viz-tile');
  if (!tile) return;
  e.preventDefault();
  const mode = tile.dataset.vizTile;
  const items = [
    { icon: '&#9974;', label: 'View Fullscreen', action: () => { _vizMode = mode; document.querySelector(`.viz-mode-btn[data-viz-mode="${mode}"]`)?.click(); document.querySelector('[data-action="vizFullscreen"]')?.click(); } },
    { icon: '&#9650;', label: 'Show Only This', action: () => document.querySelector(`.viz-mode-btn[data-viz-mode="${mode}"]`)?.click() },
    { icon: '&#9632;', label: 'Show All', action: () => document.querySelector('.viz-mode-btn[data-viz-mode="all"]')?.click() },
    '---',
  ];

  if (mode === 'fft') {
    items.push({ icon: _vizParams.fftLogScale ? '&#10003;' : '&#9634;', label: 'Log Frequency Scale', action: () => { _vizParams.fftLogScale = !_vizParams.fftLogScale; } });
  }
  if (mode === 'waveform') {
    items.push({ icon: '&#127912;', label: 'Color: Cyan', action: () => { _vizParams.waveformColor = 'cyan'; } });
    items.push({ icon: '&#127912;', label: 'Color: Magenta', action: () => { _vizParams.waveformColor = 'magenta'; } });
    items.push({ icon: '&#127912;', label: 'Color: Green', action: () => { _vizParams.waveformColor = 'green'; } });
  }
  if (mode === 'levels') {
    items.push({ icon: _vizParams.levelsHold ? '&#10003;' : '&#9634;', label: 'Peak Hold', action: () => { _vizParams.levelsHold = !_vizParams.levelsHold; _vizPeakHold = -96; } });
  }
  if (mode === 'spectrogram') {
    items.push({ icon: '&#9654;', label: 'Speed: Normal', action: () => { _vizParams.spectrogramSpeed = 1; _vizSpectrogramData = []; } });
    items.push({ icon: '&#9654;&#9654;', label: 'Speed: Fast (2x)', action: () => { _vizParams.spectrogramSpeed = 2; _vizSpectrogramData = []; } });
    items.push({ icon: '&#9654;&#9654;&#9654;', label: 'Speed: Slow (0.5x)', action: () => { _vizParams.spectrogramSpeed = 0.5; _vizSpectrogramData = []; } });
  }

  if (typeof showContextMenu === 'function') showContextMenu(e, items);
});

// ── Trello drag to rearrange tiles ──
document.addEventListener('DOMContentLoaded', () => {
  const grid = document.getElementById('vizGrid');
  if (grid && typeof initDragReorder === 'function') {
    initDragReorder(grid, '.viz-tile', 'vizTileOrder', {
      getKey: (el) => el.dataset.vizTile || '',
      handleSelector: '.viz-tile-label',
    });
  }
});

// ── Auto start/stop ──
document.addEventListener('click', (e) => {
  const tab = e.target.closest('[data-action="switchTab"]');
  if (tab && tab.dataset.tab === 'visualizer') startVisualizer();
  if (e.target.closest('[data-action="toggleAudioPlayback"], [data-action="previewAudio"], [data-action="playRecent"]')) {
    setTimeout(() => {
      const vizTab = document.getElementById('tabVisualizer');
      if (vizTab && vizTab.classList.contains('active')) startVisualizer();
    }, 100);
  }
});

// Resize canvases when window resizes
window.addEventListener('resize', _resizeCanvases);
