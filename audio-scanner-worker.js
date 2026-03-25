const { parentPort } = require('worker_threads');
const { Worker } = require('worker_threads');
const fs = require('fs');
const path = require('path');
const os = require('os');

function getRoots() {
  const home = process.env.HOME || process.env.USERPROFILE;
  const platform = process.platform;

  if (platform === 'darwin') {
    const roots = [home, '/Library/Audio', '/Applications'];
    try {
      const vols = fs.readdirSync('/Volumes', { withFileTypes: true });
      for (const v of vols) {
        if (v.isDirectory() || v.isSymbolicLink()) {
          roots.push(path.join('/Volumes', v.name));
        }
      }
    } catch {}
    return [...new Set(roots)];
  } else if (platform === 'win32') {
    const roots = [
      home,
      process.env['ProgramFiles'] || 'C:\\Program Files',
      process.env['ProgramFiles(x86)'] || 'C:\\Program Files (x86)',
    ];
    for (let c = 67; c <= 90; c++) {
      const drive = String.fromCharCode(c) + ':\\';
      try { if (fs.existsSync(drive)) roots.push(drive); } catch {}
    }
    return [...new Set(roots)];
  } else {
    return [home, '/usr/share/sounds', '/usr/local/share/sounds'];
  }
}

const CHILD_WORKER_CODE = `
const { parentPort, workerData } = require('worker_threads');
const fs = require('fs');
const path = require('path');

const AUDIO_EXTENSIONS = new Set(['.wav', '.mp3', '.aiff', '.aif', '.flac', '.ogg', '.m4a', '.wma', '.aac', '.opus', '.rex', '.rx2', '.sf2', '.sfz']);
const SKIP_DIRS = new Set(['node_modules', '.git', '.Trash', '$RECYCLE.BIN', 'System Volume Information', '.cache', '__pycache__']);

function formatSize(bytes) {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
}

const BATCH_SIZE = 50;
let batch = [];
let found = 0;
const visited = new Set();

function walk(dir, depth) {
  if (depth > 30) return;

  let realDir;
  try { realDir = fs.realpathSync(dir); } catch { return; }
  if (visited.has(realDir)) return;
  visited.add(realDir);

  let entries;
  try { entries = fs.readdirSync(dir, { withFileTypes: true }); } catch { return; }

  for (const entry of entries) {
    const name = entry.name;
    if (name.startsWith('.') || SKIP_DIRS.has(name)) continue;

    const fullPath = path.join(dir, name);

    if (entry.isDirectory()) {
      walk(fullPath, depth + 1);
    } else if (entry.isFile()) {
      const ext = path.extname(name).toLowerCase();
      if (AUDIO_EXTENSIONS.has(ext)) {
        try {
          const stat = fs.statSync(fullPath);
          batch.push({
            name: path.basename(name, ext),
            path: fullPath,
            directory: dir,
            format: ext.slice(1).toUpperCase(),
            size: stat.size,
            sizeFormatted: formatSize(stat.size),
            modified: stat.mtime.toISOString().split('T')[0],
          });
          found++;

          if (batch.length >= BATCH_SIZE) {
            parentPort.postMessage({ type: 'batch', samples: batch, found });
            batch = [];
          }
        } catch {}
      }
    }
  }
}

for (const root of workerData.roots) {
  try { if (fs.existsSync(root)) walk(root, 0); } catch {}
}

if (batch.length > 0) {
  parentPort.postMessage({ type: 'batch', samples: batch, found });
}

parentPort.postMessage({ type: 'done', found });
`;

// Main: split roots across parallel workers
parentPort.postMessage({ type: 'status', message: 'Walking filesystem for audio files...' });

const roots = getRoots().filter(r => {
  try { return fs.existsSync(r); } catch { return false; }
});

// Use up to numCpus workers, distributing roots round-robin
const numWorkers = Math.min(roots.length, Math.max(2, os.cpus().length));
const rootBuckets = Array.from({ length: numWorkers }, () => []);
roots.forEach((r, i) => rootBuckets[i % numWorkers].push(r));

let totalFound = 0;
let finishedWorkers = 0;

for (const bucket of rootBuckets) {
  if (bucket.length === 0) {
    finishedWorkers++;
    continue;
  }

  const w = new Worker(CHILD_WORKER_CODE, {
    eval: true,
    workerData: { roots: bucket },
  });

  w.on('message', (msg) => {
    if (msg.type === 'batch') {
      totalFound += msg.samples.length;
      parentPort.postMessage({ type: 'batch', samples: msg.samples, found: totalFound });
    } else if (msg.type === 'done') {
      finishedWorkers++;
      if (finishedWorkers === numWorkers) {
        parentPort.postMessage({ type: 'done', total: totalFound });
      }
    }
  });

  w.on('error', () => {
    finishedWorkers++;
    if (finishedWorkers === numWorkers) {
      parentPort.postMessage({ type: 'done', total: totalFound });
    }
  });
}

// Edge case: no roots at all
if (roots.length === 0) {
  parentPort.postMessage({ type: 'done', total: 0 });
}
