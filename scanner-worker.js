const { parentPort } = require('worker_threads');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

function getVSTDirectories() {
  const platform = process.platform;
  const home = process.env.HOME || process.env.USERPROFILE;
  const dirs = [];

  if (platform === 'darwin') {
    dirs.push(
      '/Library/Audio/Plug-Ins/VST',
      '/Library/Audio/Plug-Ins/VST3',
      '/Library/Audio/Plug-Ins/Components',
      path.join(home, 'Library/Audio/Plug-Ins/VST'),
      path.join(home, 'Library/Audio/Plug-Ins/VST3'),
      path.join(home, 'Library/Audio/Plug-Ins/Components'),
    );
  } else if (platform === 'win32') {
    const programFiles = process.env['ProgramFiles'] || 'C:\\Program Files';
    const programFilesX86 = process.env['ProgramFiles(x86)'] || 'C:\\Program Files (x86)';
    dirs.push(
      path.join(programFiles, 'Common Files', 'VST3'),
      path.join(programFiles, 'VSTPlugins'),
      path.join(programFiles, 'Steinberg', 'VSTPlugins'),
      path.join(programFilesX86, 'Common Files', 'VST3'),
      path.join(programFilesX86, 'VSTPlugins'),
      path.join(programFilesX86, 'Steinberg', 'VSTPlugins'),
    );
  } else {
    dirs.push(
      '/usr/lib/vst',
      '/usr/lib/vst3',
      '/usr/local/lib/vst',
      '/usr/local/lib/vst3',
      path.join(home, '.vst'),
      path.join(home, '.vst3'),
    );
  }

  return dirs.filter((d) => {
    try { return fs.existsSync(d); } catch { return false; }
  });
}

function getPluginType(ext) {
  const map = { '.vst': 'VST2', '.vst3': 'VST3', '.component': 'AU', '.dll': 'VST2' };
  return map[ext] || 'Unknown';
}

function getDirectorySize(dirPath) {
  let size = 0;
  try {
    const entries = fs.readdirSync(dirPath, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name);
      if (entry.isDirectory()) {
        size += getDirectorySize(fullPath);
      } else {
        size += fs.statSync(fullPath).size;
      }
    }
  } catch {}
  return size;
}

function formatSize(bytes) {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
}

function getPluginInfo(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  const name = path.basename(filePath, ext);
  const stat = fs.statSync(filePath);

  let version = null;
  let manufacturer = null;
  let manufacturerUrl = null;

  if (process.platform === 'darwin' && (ext === '.vst' || ext === '.vst3' || ext === '.component')) {
    const plistPath = path.join(filePath, 'Contents', 'Info.plist');
    if (fs.existsSync(plistPath)) {
      try {
        const versionResult = execSync(
          `/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "${plistPath}" 2>/dev/null`,
          { encoding: 'utf8', timeout: 3000 },
        ).trim();
        if (versionResult) version = versionResult;
      } catch {}

      try {
        const mfgResult = execSync(
          `/usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" "${plistPath}" 2>/dev/null`,
          { encoding: 'utf8', timeout: 3000 },
        ).trim();
        if (mfgResult) {
          const parts = mfgResult.split('.');
          if (parts.length >= 2) {
            manufacturer = parts[1].charAt(0).toUpperCase() + parts[1].slice(1);
            const domain = parts[1].toLowerCase();
            if (domain && domain !== 'apple' && domain.length > 1) {
              manufacturerUrl = `https://www.${domain}.com`;
            }
          }
        }
      } catch {}

      if (!manufacturerUrl) {
        try {
          const urlResult = execSync(
            `/usr/libexec/PlistBuddy -c "Print :NSHumanReadableCopyright" "${plistPath}" 2>/dev/null`,
            { encoding: 'utf8', timeout: 3000 },
          ).trim();
          if (urlResult) {
            const urlMatch = urlResult.match(/https?:\/\/[^\s)"',]+/);
            if (urlMatch) manufacturerUrl = urlMatch[0];
          }
        } catch {}
      }
    }
  }

  return {
    name,
    path: filePath,
    type: getPluginType(ext),
    version: version || 'Unknown',
    manufacturer: manufacturer || 'Unknown',
    manufacturerUrl: manufacturerUrl || null,
    size: formatSize(stat.size || getDirectorySize(filePath)),
    modified: stat.mtime.toISOString().split('T')[0],
  };
}

// Collect all plugin file paths first, then process them with progress
const directories = getVSTDirectories();
const validExtensions = ['.vst', '.vst3', '.component', '.dll'];

// Phase 1: discover all plugin paths
const pluginPaths = [];
for (const dir of directories) {
  try {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const ext = path.extname(entry.name).toLowerCase();
      if (validExtensions.includes(ext)) {
        pluginPaths.push(path.join(dir, entry.name));
      }
    }
  } catch {}
}

parentPort.postMessage({ type: 'total', total: pluginPaths.length, directories });

// Phase 2: process each plugin and stream results back
const seen = new Set();
let processed = 0;
const BATCH_SIZE = 10;
let batch = [];

for (const pluginPath of pluginPaths) {
  if (seen.has(pluginPath)) {
    processed++;
    continue;
  }
  seen.add(pluginPath);

  try {
    const info = getPluginInfo(pluginPath);
    batch.push(info);
  } catch {}

  processed++;

  // Send batch of plugins + progress
  if (batch.length >= BATCH_SIZE || processed === pluginPaths.length) {
    parentPort.postMessage({
      type: 'batch',
      plugins: batch,
      processed,
      total: pluginPaths.length,
    });
    batch = [];
  }
}

parentPort.postMessage({ type: 'done' });
