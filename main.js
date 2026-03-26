const { app, BrowserWindow, ipcMain, protocol } = require('electron');
const path = require('path');
const { Worker } = require('worker_threads');
const history = require('./history');

let mainWindow;
let scanWorker = null;
let updateWorker = null;
let audioScanWorker = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
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

protocol.registerSchemesAsPrivileged([
  { scheme: 'audio-preview', privileges: { bypassCSP: true, stream: true, supportFetchAPI: true } },
]);

app.whenReady().then(() => {
  protocol.registerFileProtocol('audio-preview', (request, callback) => {
    const filePath = decodeURIComponent(request.url.replace('audio-preview://', ''));
    callback({ path: filePath });
  });
  createWindow();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow();
});

// IPC handlers
ipcMain.handle('get-version', () => app.getVersion());

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

ipcMain.handle('kvr-cache-get', async () => {
  return history.getKvrCache();
});

ipcMain.handle('kvr-cache-update', async (_event, entries) => {
  history.updateKvrCache(entries);
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

ipcMain.handle('resolve-kvr', async (_event, directUrl, pluginName) => {
  const https = require('https');

  // Bogus landing pages KVR redirects to when a product doesn't exist
  const KVR_INVALID_PAGES = [
    '/plugins/the-newest-plugins',
    '/plugins/newest',
    '/plugins',
  ];

  function fetchWithFinalUrl(url, maxRedirects = 5) {
    return new Promise((resolve, reject) => {
      https.get(url, { timeout: 8000, headers: {
        'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
      }}, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location && maxRedirects > 0) {
          let redirectUrl = res.headers.location;
          if (redirectUrl.startsWith('/')) redirectUrl = 'https://www.kvraudio.com' + redirectUrl;

          // Check if redirect target is a known invalid page
          const redirectPath = redirectUrl.replace(/https?:\/\/[^/]+/, '').replace(/[?#].*/, '');
          if (KVR_INVALID_PAGES.some(p => redirectPath.startsWith(p))) {
            res.resume();
            return resolve({ html: '', finalUrl: redirectUrl, valid: false });
          }

          res.resume();
          return fetchWithFinalUrl(redirectUrl, maxRedirects - 1).then(resolve, reject);
        }
        let body = '';
        res.on('data', (c) => body += c);
        res.on('end', () => {
          // Also check final URL in case of JS/meta redirects embedded in HTML
          const finalPath = url.replace(/https?:\/\/[^/]+/, '').replace(/[?#].*/, '');
          const isInvalid = KVR_INVALID_PAGES.some(p => finalPath.startsWith(p));
          resolve({ html: body, finalUrl: url, valid: !isInvalid && res.statusCode < 400 });
        });
        res.on('error', reject);
      }).on('error', reject).on('timeout', function() { this.destroy(); reject(new Error('timeout')); });
    });
  }

  function fetchHtml(url) {
    return fetchWithFinalUrl(url).then(r => r.html);
  }

  const platformKeywords = {
    darwin: ['mac', 'macos', 'osx', 'os x', 'apple'],
    win32: ['win', 'windows', 'pc'],
    linux: ['linux', 'ubuntu', 'debian'],
  }[process.platform] || [];

  function extractDownloadUrl(html) {
    // Find download/get links
    const linkPattern = /href="(https?:\/\/[^"]*(?:download|get|buy|release)[^"]*)"/gi;
    const allLinks = [];
    let m;
    while ((m = linkPattern.exec(html)) !== null) {
      allLinks.push(m[1]);
    }

    // Prefer platform-specific link
    for (const link of allLinks) {
      const lower = link.toLowerCase();
      if (platformKeywords.some(kw => lower.includes(kw))) {
        return link;
      }
    }

    // Check for platform text near download links
    for (const kw of platformKeywords) {
      const ctxPattern = new RegExp(
        `(?:${kw})[^<]{0,80}?href="(https?:\\/\\/[^"]*(?:download|get)[^"]*)"` +
        `|href="(https?:\\/\\/[^"]*(?:download|get)[^"]*)"[^<]{0,80}?(?:${kw})`, 'gi'
      );
      const ctxMatch = ctxPattern.exec(html);
      if (ctxMatch) return ctxMatch[1] || ctxMatch[2];
    }

    // Any download link
    return allLinks.length > 0 ? allLinks[0] : null;
  }

  async function scrapeProductPage(productUrl) {
    try {
      const html = await fetchHtml(productUrl);
      const downloadUrl = extractDownloadUrl(html);
      return { productUrl, downloadUrl };
    } catch {
      return { productUrl, downloadUrl: null };
    }
  }

  // Try direct URL first -- follow redirects and check we didn't land on a generic page
  try {
    const response = await fetchWithFinalUrl(directUrl);
    if (response.valid) {
      const downloadUrl = extractDownloadUrl(response.html);
      return { productUrl: response.finalUrl, downloadUrl };
    }
  } catch {}

  // Fallback: search KVR by plugin name
  try {
    const searchUrl = `https://www.kvraudio.com/plugins/search?q=${encodeURIComponent(pluginName)}`;
    const html = await fetchHtml(searchUrl);

    // Collect unique product links from search results
    const pattern = /href="(\/product\/[^"]+)"/gi;
    const productLinks = [];
    const seen = new Set();
    let match;
    while ((match = pattern.exec(html)) !== null) {
      const href = match[1];
      if (!seen.has(href)) {
        seen.add(href);
        productLinks.push('https://www.kvraudio.com' + href);
      }
    }

    // Check the plugin name appears in the product URL slug (basic relevance check)
    const nameSlug = pluginName.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
    const nameWords = pluginName.toLowerCase().replace(/[^a-z0-9]+/g, ' ').trim().split(/\s+/);

    for (const foundUrl of productLinks.slice(0, 5)) {
      const urlSlug = foundUrl.split('/product/')[1] || '';
      // Check if the URL contains the plugin name or most of its words
      const matchingWords = nameWords.filter(w => w.length > 1 && urlSlug.includes(w));
      if (urlSlug.includes(nameSlug) || matchingWords.length >= Math.ceil(nameWords.length * 0.5)) {
        const result = await scrapeProductPage(foundUrl);
        return result;
      }
    }

    // If no relevant match, return the first result anyway
    if (productLinks.length > 0) {
      const result = await scrapeProductPage(productLinks[0]);
      return result;
    }
  } catch {}

  // Last resort
  return {
    productUrl: `https://www.kvraudio.com/plugins/search?q=${encodeURIComponent(pluginName)}`,
    downloadUrl: null,
  };
});

// Audio sample scanner
ipcMain.handle('scan-audio-samples', async () => {
  return new Promise((resolve, reject) => {
    const allSamples = [];

    audioScanWorker = new Worker(path.join(__dirname, 'audio-scanner-worker.js'));

    audioScanWorker.on('message', (msg) => {
      if (msg.type === 'status') {
        mainWindow.webContents.send('audio-scan-progress', {
          phase: 'status',
          message: msg.message,
        });
      } else if (msg.type === 'batch') {
        allSamples.push(...msg.samples);
        mainWindow.webContents.send('audio-scan-progress', {
          phase: 'scanning',
          samples: msg.samples,
          found: msg.found,
        });
      } else if (msg.type === 'done') {
        audioScanWorker = null;
        allSamples.sort((a, b) => a.name.localeCompare(b.name));
        resolve({ samples: allSamples });
      }
    });

    audioScanWorker.on('error', (err) => { audioScanWorker = null; reject(err); });
    audioScanWorker.on('exit', (code) => {
      audioScanWorker = null;
      if (code !== 0) reject(new Error('stopped'));
    });
  });
});

ipcMain.handle('stop-audio-scan', async () => {
  if (audioScanWorker) {
    await audioScanWorker.terminate();
    audioScanWorker = null;
  }
});

ipcMain.handle('open-audio-folder', async (_event, filePath) => {
  const { shell } = require('electron');
  shell.showItemInFolder(filePath);
});

ipcMain.handle('get-audio-file-url', async (_event, filePath) => {
  return 'audio-preview://' + encodeURIComponent(filePath);
});

ipcMain.handle('get-audio-metadata', async (_event, filePath) => {
  const fs = require('fs');
  const pathMod = require('path');

  try {
    const stat = fs.statSync(filePath);
    const ext = pathMod.extname(filePath).toLowerCase();
    const meta = {
      fullPath: filePath,
      fileName: pathMod.basename(filePath),
      directory: pathMod.dirname(filePath),
      format: ext.slice(1).toUpperCase(),
      sizeBytes: stat.size,
      created: stat.birthtime.toISOString(),
      modified: stat.mtime.toISOString(),
      accessed: stat.atime.toISOString(),
      permissions: '0' + (stat.mode & 0o777).toString(8),
    };

    // Read WAV header for sample rate, bit depth, channels, duration
    if (ext === '.wav') {
      try {
        const fd = fs.openSync(filePath, 'r');
        const header = Buffer.alloc(44);
        fs.readSync(fd, header, 0, 44, 0);
        fs.closeSync(fd);

        if (header.toString('ascii', 0, 4) === 'RIFF' && header.toString('ascii', 8, 12) === 'WAVE') {
          meta.channels = header.readUInt16LE(22);
          meta.sampleRate = header.readUInt32LE(24);
          meta.byteRate = header.readUInt32LE(28);
          meta.bitsPerSample = header.readUInt16LE(34);
          const dataSize = header.readUInt32LE(40);
          if (meta.byteRate > 0) {
            meta.duration = dataSize / meta.byteRate;
          }
        }
      } catch {}
    }

    // Read AIFF header
    if (ext === '.aiff' || ext === '.aif') {
      try {
        const fd = fs.openSync(filePath, 'r');
        const buf = Buffer.alloc(512);
        const bytesRead = fs.readSync(fd, buf, 0, 512, 0);
        fs.closeSync(fd);

        if (buf.toString('ascii', 0, 4) === 'FORM' && buf.toString('ascii', 8, 12) === 'AIFF') {
          // Find COMM chunk
          let offset = 12;
          while (offset + 8 < bytesRead) {
            const chunkId = buf.toString('ascii', offset, offset + 4);
            const chunkSize = buf.readUInt32BE(offset + 4);
            if (chunkId === 'COMM') {
              meta.channels = buf.readUInt16BE(offset + 8);
              const numFrames = buf.readUInt32BE(offset + 10);
              meta.bitsPerSample = buf.readUInt16BE(offset + 14);
              // 80-bit extended float for sample rate
              const exponent = buf.readUInt16BE(offset + 16);
              const mantissa = buf.readUInt32BE(offset + 18);
              const exp = exponent - 16383 - 31;
              meta.sampleRate = Math.round(mantissa * Math.pow(2, exp));
              if (meta.sampleRate > 0) {
                meta.duration = numFrames / meta.sampleRate;
              }
              break;
            }
            offset += 8 + chunkSize;
            if (chunkSize % 2 !== 0) offset++; // pad byte
          }
        }
      } catch {}
    }

    // Read FLAC streaminfo
    if (ext === '.flac') {
      try {
        const fd = fs.openSync(filePath, 'r');
        const buf = Buffer.alloc(42);
        fs.readSync(fd, buf, 0, 42, 0);
        fs.closeSync(fd);

        if (buf.toString('ascii', 0, 4) === 'fLaC') {
          meta.sampleRate = (buf[18] << 12) | (buf[19] << 4) | (buf[20] >> 4);
          meta.channels = ((buf[20] >> 1) & 0x07) + 1;
          meta.bitsPerSample = ((buf[20] & 1) << 4) | (buf[21] >> 4) + 1;
          const totalSamples = ((buf[21] & 0x0F) * Math.pow(2, 32)) +
            (buf[22] << 24 | buf[23] << 16 | buf[24] << 8 | buf[25]);
          if (meta.sampleRate > 0 && totalSamples > 0) {
            meta.duration = totalSamples / meta.sampleRate;
          }
        }
      } catch {}
    }

    return meta;
  } catch (err) {
    return { error: err.message };
  }
});

// Audio history IPC handlers
ipcMain.handle('audio-history-save', async (_event, samples) => {
  return history.saveAudioScan(samples);
});

ipcMain.handle('audio-history-get-scans', async () => {
  return history.getAudioScans();
});

ipcMain.handle('audio-history-get-detail', async (_event, id) => {
  return history.getAudioScanDetail(id);
});

ipcMain.handle('audio-history-delete', async (_event, id) => {
  history.deleteAudioScan(id);
});

ipcMain.handle('audio-history-clear', async () => {
  history.clearAudioHistory();
});

ipcMain.handle('audio-history-latest', async () => {
  return history.getLatestAudioScan();
});

ipcMain.handle('audio-history-diff', async (_event, oldId, newId) => {
  return history.diffAudioScans(oldId, newId);
});

ipcMain.handle('open-update-url', async (_event, url) => {
  const { shell } = require('electron');
  shell.openExternal(url);
});

ipcMain.handle('open-plugin-folder', async (_event, pluginPath) => {
  const { shell } = require('electron');
  shell.showItemInFolder(pluginPath);
});
