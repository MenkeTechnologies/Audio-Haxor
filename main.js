const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const { Worker } = require('worker_threads');
const history = require('./history');

let mainWindow;
let scanWorker = null;
let updateWorker = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1100,
    height: 750,
    minWidth: 800,
    minHeight: 600,
    backgroundColor: '#05050a',
    titleBarStyle: 'hiddenInset',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  mainWindow.loadFile('index.html');
}

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow();
});

// IPC handlers
ipcMain.handle('scan-plugins', async () => {
  return new Promise((resolve, reject) => {
    const allPlugins = [];
    let directories = [];

    scanWorker = new Worker(path.join(__dirname, 'scanner-worker.js'));

    scanWorker.on('message', (msg) => {
      if (msg.type === 'total') {
        directories = msg.directories;
        mainWindow.webContents.send('scan-progress', {
          phase: 'start',
          total: msg.total,
          processed: 0,
        });
      } else if (msg.type === 'batch') {
        allPlugins.push(...msg.plugins);
        mainWindow.webContents.send('scan-progress', {
          phase: 'scanning',
          plugins: msg.plugins,
          processed: msg.processed,
          total: msg.total,
        });
      } else if (msg.type === 'done') {
        scanWorker = null;
        allPlugins.sort((a, b) => a.name.localeCompare(b.name));
        const snapshot = history.saveScan(allPlugins, directories);
        resolve({ plugins: allPlugins, directories, snapshotId: snapshot.id });
      }
    });

    scanWorker.on('error', (err) => { scanWorker = null; reject(err); });
    scanWorker.on('exit', (code) => {
      scanWorker = null;
      if (code !== 0) reject(new Error(`stopped`));
    });
  });
});

ipcMain.handle('stop-scan', async () => {
  if (scanWorker) {
    await scanWorker.terminate();
    scanWorker = null;
  }
});

// History IPC handlers
ipcMain.handle('history-get-scans', async () => {
  return history.getScans();
});

ipcMain.handle('history-get-detail', async (_event, id) => {
  return history.getScanDetail(id);
});

ipcMain.handle('history-delete', async (_event, id) => {
  history.deleteScan(id);
});

ipcMain.handle('history-clear', async () => {
  history.clearHistory();
});

ipcMain.handle('history-diff', async (_event, oldId, newId) => {
  return history.diffScans(oldId, newId);
});

ipcMain.handle('history-latest', async () => {
  return history.getLatestScan();
});

ipcMain.handle('check-updates', async (_event, plugins) => {
  return new Promise((resolve, reject) => {
    updateWorker = new Worker(path.join(__dirname, 'update-worker.js'), {
      workerData: { plugins },
    });

    updateWorker.on('message', (msg) => {
      if (msg.type === 'start') {
        mainWindow.webContents.send('update-progress', {
          phase: 'start',
          total: msg.total,
          processed: 0,
        });
      } else if (msg.type === 'batch') {
        mainWindow.webContents.send('update-progress', {
          phase: 'checking',
          plugins: msg.plugins,
          processed: msg.processed,
          total: msg.total,
        });
      } else if (msg.type === 'done') {
        updateWorker = null;
        resolve(msg.plugins);
      } else if (msg.type === 'error') {
        updateWorker = null;
        reject(new Error(msg.message));
      }
    });

    updateWorker.on('error', (err) => { updateWorker = null; reject(err); });
    updateWorker.on('exit', (code) => {
      updateWorker = null;
      if (code !== 0) reject(new Error(`stopped`));
    });
  });
});

ipcMain.handle('stop-updates', async () => {
  if (updateWorker) {
    await updateWorker.terminate();
    updateWorker = null;
  }
});

ipcMain.handle('open-update-url', async (_event, url) => {
  const { shell } = require('electron');
  shell.openExternal(url);
});

ipcMain.handle('open-plugin-folder', async (_event, pluginPath) => {
  const { shell } = require('electron');
  shell.showItemInFolder(pluginPath);
});
