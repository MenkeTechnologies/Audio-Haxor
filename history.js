const fs = require('fs');
const path = require('path');
const { app } = require('electron');

const HISTORY_FILE = path.join(app.getPath('userData'), 'scan-history.json');

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

module.exports = { saveScan, getScans, getScanDetail, deleteScan, clearHistory, diffScans, getLatestScan };
