const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('vstUpdater', {
  getVersion: () => ipcRenderer.invoke('get-version'),
  scanPlugins: () => ipcRenderer.invoke('scan-plugins'),
  stopScan: () => ipcRenderer.invoke('stop-scan'),
  onScanProgress: (callback) => {
    const handler = (_event, data) => callback(data);
    ipcRenderer.on('scan-progress', handler);
    return () => ipcRenderer.removeListener('scan-progress', handler);
  },
  checkUpdates: (plugins) => ipcRenderer.invoke('check-updates', plugins),
  stopUpdates: () => ipcRenderer.invoke('stop-updates'),
  onUpdateProgress: (callback) => {
    const handler = (_event, data) => callback(data);
    ipcRenderer.on('update-progress', handler);
    return () => ipcRenderer.removeListener('update-progress', handler);
  },
  resolveKvr: (directUrl, pluginName) => ipcRenderer.invoke('resolve-kvr', directUrl, pluginName),
  openUpdateUrl: (url) => ipcRenderer.invoke('open-update-url', url),
  openPluginFolder: (path) => ipcRenderer.invoke('open-plugin-folder', path),
  // History
  getScans: () => ipcRenderer.invoke('history-get-scans'),
  getScanDetail: (id) => ipcRenderer.invoke('history-get-detail', id),
  deleteScan: (id) => ipcRenderer.invoke('history-delete', id),
  clearHistory: () => ipcRenderer.invoke('history-clear'),
  diffScans: (oldId, newId) => ipcRenderer.invoke('history-diff', oldId, newId),
  getLatestScan: () => ipcRenderer.invoke('history-latest'),
  // Audio samples
  scanAudioSamples: () => ipcRenderer.invoke('scan-audio-samples'),
  stopAudioScan: () => ipcRenderer.invoke('stop-audio-scan'),
  onAudioScanProgress: (callback) => {
    const handler = (_event, data) => callback(data);
    ipcRenderer.on('audio-scan-progress', handler);
    return () => ipcRenderer.removeListener('audio-scan-progress', handler);
  },
  openAudioFolder: (path) => ipcRenderer.invoke('open-audio-folder', path),
  getAudioMetadata: (filePath) => ipcRenderer.invoke('get-audio-metadata', filePath),
  getAudioFileUrl: (filePath) => ipcRenderer.invoke('get-audio-file-url', filePath),
  // Audio history
  saveAudioScan: (samples) => ipcRenderer.invoke('audio-history-save', samples),
  getAudioScans: () => ipcRenderer.invoke('audio-history-get-scans'),
  getAudioScanDetail: (id) => ipcRenderer.invoke('audio-history-get-detail', id),
  deleteAudioScan: (id) => ipcRenderer.invoke('audio-history-delete', id),
  clearAudioHistory: () => ipcRenderer.invoke('audio-history-clear'),
  getLatestAudioScan: () => ipcRenderer.invoke('audio-history-latest'),
  diffAudioScans: (oldId, newId) => ipcRenderer.invoke('audio-history-diff', oldId, newId),
  // KVR cache
  getKvrCache: () => ipcRenderer.invoke('kvr-cache-get'),
  updateKvrCache: (entries) => ipcRenderer.invoke('kvr-cache-update', entries),
});
