const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('vstUpdater', {
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
  openUpdateUrl: (url) => ipcRenderer.invoke('open-update-url', url),
  openPluginFolder: (path) => ipcRenderer.invoke('open-plugin-folder', path),
  // History
  getScans: () => ipcRenderer.invoke('history-get-scans'),
  getScanDetail: (id) => ipcRenderer.invoke('history-get-detail', id),
  deleteScan: (id) => ipcRenderer.invoke('history-delete', id),
  clearHistory: () => ipcRenderer.invoke('history-clear'),
  diffScans: (oldId, newId) => ipcRenderer.invoke('history-diff', oldId, newId),
  getLatestScan: () => ipcRenderer.invoke('history-latest'),
});
