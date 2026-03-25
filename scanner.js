const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Common VST plugin directories per platform
function getVSTDirectories() {
  const platform = process.platform;
  const home = process.env.HOME || process.env.USERPROFILE;
  const dirs = [];

  if (platform === 'darwin') {
    // macOS VST paths
    dirs.push(
      '/Library/Audio/Plug-Ins/VST',
      '/Library/Audio/Plug-Ins/VST3',
      '/Library/Audio/Plug-Ins/Components', // AU plugins
      path.join(home, 'Library/Audio/Plug-Ins/VST'),
      path.join(home, 'Library/Audio/Plug-Ins/VST3'),
      path.join(home, 'Library/Audio/Plug-Ins/Components'),
    );
  } else if (platform === 'win32') {
    // Windows VST paths
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
    // Linux VST paths
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
    try {
      return fs.existsSync(d);
    } catch {
      return false;
    }
  });
}

function getPluginType(ext) {
  const map = {
    '.vst': 'VST2',
    '.vst3': 'VST3',
    '.component': 'AU',
    '.dll': 'VST2',
  };
  return map[ext] || 'Unknown';
}

function getPluginInfo(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  const name = path.basename(filePath, ext);
  const stat = fs.statSync(filePath);

  let version = null;
  let manufacturer = null;
  let bundleId = null;
  let manufacturerUrl = null;

  // Try to extract version info from Info.plist (macOS bundles)
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
          bundleId = mfgResult;
          // Extract manufacturer from bundle ID like "com.manufacturer.plugin"
          const parts = mfgResult.split('.');
          if (parts.length >= 2) {
            manufacturer = parts[1].charAt(0).toUpperCase() + parts[1].slice(1);
            // Derive manufacturer URL from bundle ID
            // e.g. "com.native-instruments.foo" -> "https://native-instruments.com"
            // e.g. "com.fabfilter.Pro-Q" -> "https://fabfilter.com"
            const domain = parts[1].toLowerCase();
            if (domain && domain !== 'apple' && domain.length > 1) {
              manufacturerUrl = `https://www.${domain}.com`;
            }
          }
        }
      } catch {}

      // Try to read the URL from the plist directly (some plugins have it)
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

function scanDirectory(dir) {
  const plugins = [];
  const validExtensions = ['.vst', '.vst3', '.component', '.dll'];

  try {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      const ext = path.extname(entry.name).toLowerCase();

      if (validExtensions.includes(ext)) {
        try {
          plugins.push(getPluginInfo(fullPath));
        } catch {}
      }
    }
  } catch {}

  return plugins;
}

function scanVSTPlugins() {
  const directories = getVSTDirectories();
  const allPlugins = [];
  const seen = new Set();

  for (const dir of directories) {
    const plugins = scanDirectory(dir);
    for (const plugin of plugins) {
      if (!seen.has(plugin.path)) {
        seen.add(plugin.path);
        allPlugins.push(plugin);
      }
    }
  }

  // Sort by name
  allPlugins.sort((a, b) => a.name.localeCompare(b.name));
  return { plugins: allPlugins, directories };
}

module.exports = { scanVSTPlugins };
