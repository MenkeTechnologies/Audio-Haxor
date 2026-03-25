const fs = require('fs');
const path = require('path');

let HISTORY_FILE;
try {
  const electron = require('electron');
  const app = electron.app || (electron.remote && electron.remote.app);
  if (app && typeof app.getPath === 'function') {
    HISTORY_FILE = path.join(app.getPath('userData'), 'scan-history.json');
  } else {
    throw new Error('no app');
  }
} catch {
  // Fallback for testing outside Electron
  HISTORY_FILE = path.join(__dirname, 'scan-history.json');
}

let KVR_CACHE_FILE;
try {
  const electron2 = require('electron');
  const app2 = electron2.app || (electron2.remote && electron2.remote.app);
  if (app2 && typeof app2.getPath === 'function') {
    KVR_CACHE_FILE = path.join(app2.getPath('userData'), 'kvr-cache.json');
  } else {
    throw new Error('no app');
  }
} catch {
  KVR_CACHE_FILE = path.join(__dirname, 'kvr-cache.json');
}

function setHistoryFile(filePath) {
  HISTORY_FILE = filePath;
}

function setKvrCacheFile(filePath) {
  KVR_CACHE_FILE = filePath;
}

function loadKvrCache() {
  try {
    if (fs.existsSync(KVR_CACHE_FILE)) {
      return JSON.parse(fs.readFileSync(KVR_CACHE_FILE, 'utf8'));
    }
  } catch {}
  return {};
}

function saveKvrCache(cache) {
  fs.writeFileSync(KVR_CACHE_FILE, JSON.stringify(cache, null, 2), 'utf8');
}

function updateKvrCache(entries) {
  const cache = loadKvrCache();
  for (const entry of entries) {
    cache[entry.key] = {
      kvrUrl: entry.kvrUrl,
      updateUrl: entry.updateUrl || null,
      latestVersion: entry.latestVersion || null,
      hasUpdate: entry.hasUpdate || false,
      source: entry.source || 'kvr',
      timestamp: new Date().toISOString(),
    };
  }
  saveKvrCache(cache);
}

function getKvrCache() {
  return loadKvrCache();
}

function clearKvrCache() {
  saveKvrCache({});
}

let AUDIO_HISTORY_FILE;
try {
  const electron3 = require('electron');
  const app3 = electron3.app || (electron3.remote && electron3.remote.app);
  if (app3 && typeof app3.getPath === 'function') {
    AUDIO_HISTORY_FILE = path.join(app3.getPath('userData'), 'audio-scan-history.json');
  } else {
    throw new Error('no app');
  }
} catch {
  AUDIO_HISTORY_FILE = path.join(__dirname, 'audio-scan-history.json');
}

function setAudioHistoryFile(filePath) {
  AUDIO_HISTORY_FILE = filePath;
}

function loadHistory() {
  try {
    if (fs.existsSync(HISTORY_FILE)) {
      return JSON.parse(fs.readFileSync(HISTORY_FILE, 'utf8'));
    }
  } catch {}
  return { scans: [] };
}

function saveHistory(history) {
  fs.writeFileSync(HISTORY_FILE, JSON.stringify(history, null, 2), 'utf8');
}

function saveScan(plugins, directories) {
  const history = loadHistory();
  const snapshot = {
    id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
    timestamp: new Date().toISOString(),
    pluginCount: plugins.length,
    plugins,
    directories,
  };
  history.scans.unshift(snapshot);
  // Keep last 50 scans
  if (history.scans.length > 50) {
    history.scans = history.scans.slice(0, 50);
  }
  saveHistory(history);
  return snapshot;
}

function getScans() {
  const history = loadHistory();
  // Return summaries without full plugin lists to keep IPC payload small
  return history.scans.map((s) => ({
    id: s.id,
    timestamp: s.timestamp,
    pluginCount: s.pluginCount,
  }));
}

function getScanDetail(id) {
  const history = loadHistory();
  return history.scans.find((s) => s.id === id) || null;
}

function deleteScan(id) {
  const history = loadHistory();
  history.scans = history.scans.filter((s) => s.id !== id);
  saveHistory(history);
}

function clearHistory() {
  saveHistory({ scans: [] });
}

function diffScans(oldId, newId) {
  const history = loadHistory();
  const oldScan = history.scans.find((s) => s.id === oldId);
  const newScan = history.scans.find((s) => s.id === newId);
  if (!oldScan || !newScan) return null;

  const oldPaths = new Set(oldScan.plugins.map((p) => p.path));
  const newPaths = new Set(newScan.plugins.map((p) => p.path));
  const oldByPath = Object.fromEntries(oldScan.plugins.map((p) => [p.path, p]));
  const newByPath = Object.fromEntries(newScan.plugins.map((p) => [p.path, p]));

  const added = newScan.plugins.filter((p) => !oldPaths.has(p.path));
  const removed = oldScan.plugins.filter((p) => !newPaths.has(p.path));
  const versionChanged = newScan.plugins.filter((p) => {
    const old = oldByPath[p.path];
    return old && old.version !== p.version && p.version !== 'Unknown' && old.version !== 'Unknown';
  }).map((p) => ({
    ...p,
    previousVersion: oldByPath[p.path].version,
  }));

  return {
    oldScan: { id: oldScan.id, timestamp: oldScan.timestamp, pluginCount: oldScan.pluginCount },
    newScan: { id: newScan.id, timestamp: newScan.timestamp, pluginCount: newScan.pluginCount },
    added,
    removed,
    versionChanged,
  };
}

function getLatestScan() {
  const history = loadHistory();
  return history.scans.length > 0 ? history.scans[0] : null;
}

// ── Audio scan history ──

function loadAudioHistory() {
  try {
    if (fs.existsSync(AUDIO_HISTORY_FILE)) {
      return JSON.parse(fs.readFileSync(AUDIO_HISTORY_FILE, 'utf8'));
    }
  } catch {}
  return { scans: [] };
}

function saveAudioHistory(history) {
  fs.writeFileSync(AUDIO_HISTORY_FILE, JSON.stringify(history, null, 2), 'utf8');
}

function saveAudioScan(samples) {
  const history = loadAudioHistory();
  // Compute format stats
  const formatCounts = {};
  let totalBytes = 0;
  for (const s of samples) {
    formatCounts[s.format] = (formatCounts[s.format] || 0) + 1;
    totalBytes += s.size || 0;
  }
  const snapshot = {
    id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
    timestamp: new Date().toISOString(),
    sampleCount: samples.length,
    totalBytes,
    formatCounts,
    samples,
  };
  history.scans.unshift(snapshot);
  if (history.scans.length > 50) {
    history.scans = history.scans.slice(0, 50);
  }
  saveAudioHistory(history);
  return snapshot;
}

function getAudioScans() {
  const history = loadAudioHistory();
  return history.scans.map((s) => ({
    id: s.id,
    timestamp: s.timestamp,
    sampleCount: s.sampleCount,
    totalBytes: s.totalBytes,
    formatCounts: s.formatCounts,
  }));
}

function getAudioScanDetail(id) {
  const history = loadAudioHistory();
  return history.scans.find((s) => s.id === id) || null;
}

function deleteAudioScan(id) {
  const history = loadAudioHistory();
  history.scans = history.scans.filter((s) => s.id !== id);
  saveAudioHistory(history);
}

function clearAudioHistory() {
  saveAudioHistory({ scans: [] });
}

function getLatestAudioScan() {
  const history = loadAudioHistory();
  return history.scans.length > 0 ? history.scans[0] : null;
}

function diffAudioScans(oldId, newId) {
  const history = loadAudioHistory();
  const oldScan = history.scans.find((s) => s.id === oldId);
  const newScan = history.scans.find((s) => s.id === newId);
  if (!oldScan || !newScan) return null;

  const oldPaths = new Set(oldScan.samples.map((s) => s.path));
  const newPaths = new Set(newScan.samples.map((s) => s.path));

  const added = newScan.samples.filter((s) => !oldPaths.has(s.path));
  const removed = oldScan.samples.filter((s) => !newPaths.has(s.path));

  return {
    oldScan: { id: oldScan.id, timestamp: oldScan.timestamp, sampleCount: oldScan.sampleCount },
    newScan: { id: newScan.id, timestamp: newScan.timestamp, sampleCount: newScan.sampleCount },
    added,
    removed,
  };
}

module.exports = { saveScan, getScans, getScanDetail, deleteScan, clearHistory, diffScans, getLatestScan, setHistoryFile, setKvrCacheFile, updateKvrCache, getKvrCache, clearKvrCache, saveAudioScan, getAudioScans, getAudioScanDetail, deleteAudioScan, clearAudioHistory, getLatestAudioScan, diffAudioScans, setAudioHistoryFile };
