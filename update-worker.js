const { parentPort, workerData } = require('worker_threads');
const https = require('https');
const http = require('http');

const plugins = workerData.plugins;

// Fetch a URL, following redirects (up to 5)
function fetch(url, maxRedirects = 5) {
  return new Promise((resolve, reject) => {
    const mod = url.startsWith('https') ? https : http;
    const req = mod.get(url, {
      headers: {
        'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
        'Accept-Language': 'en-US,en;q=0.5',
      },
      timeout: 15000,
    }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location && maxRedirects > 0) {
        let redirectUrl = res.headers.location;
        if (redirectUrl.startsWith('/')) {
          const u = new URL(url);
          redirectUrl = u.origin + redirectUrl;
        }
        res.resume();
        return fetch(redirectUrl, maxRedirects - 1).then(resolve, reject);
      }
      let body = '';
      res.on('data', (chunk) => body += chunk);
      res.on('end', () => resolve(body));
      res.on('error', reject);
    });
    req.on('error', reject);
    req.on('timeout', () => { req.destroy(); reject(new Error('timeout')); });
  });
}

// Normalize a version string to comparable parts
function parseVersion(ver) {
  if (!ver || ver === 'Unknown') return [0, 0, 0];
  return ver.split('.').map(n => parseInt(n, 10) || 0);
}

function compareVersions(a, b) {
  const pa = parseVersion(a);
  const pb = parseVersion(b);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const diff = (pa[i] || 0) - (pb[i] || 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

// ── KVR Audio Search ──
// Search KVR's product database and scrape the product page for version info.

// Step 1: Search KVR for the plugin and find the product page URL
async function searchKVR(pluginName, manufacturer) {
  const mfg = manufacturer && manufacturer !== 'Unknown' ? manufacturer : '';
  const query = `${mfg} ${pluginName}`.trim();
  const url = `https://www.kvraudio.com/plugins/search?q=${encodeURIComponent(query)}`;

  try {
    const html = await fetch(url);

    // KVR search results contain product links like:
    // <a href="/product/fabfilter-pro-q-3-by-fabfilter" ...>
    // or /plugins/... URLs
    const productLinks = [];

    // Match product page links
    const linkPattern = /href="(\/product\/[^"]+)"/gi;
    let match;
    while ((match = linkPattern.exec(html)) !== null) {
      productLinks.push('https://www.kvraudio.com' + match[1]);
    }

    // Also try /plugins/ style links
    const pluginLinkPattern = /href="(\/plugins\/[^"]+)"/gi;
    while ((match = pluginLinkPattern.exec(html)) !== null) {
      // Skip search/category links
      if (!match[1].includes('/search') && !match[1].includes('/category')) {
        productLinks.push('https://www.kvraudio.com' + match[1]);
      }
    }

    // Deduplicate
    return [...new Set(productLinks)];
  } catch {
    return [];
  }
}

// Step 2: Scrape a KVR product page for version number and download URL
function extractKVRVersion(html) {
  // KVR product pages show version in patterns like:
  // "Version: 1.2.3" or "v1.2.3" or "Latest Version</dt><dd>1.2.3"
  // Often in a specs/details section

  const patterns = [
    // "Version: 1.2.3" or "Version 1.2.3"
    /Version\s*[:]\s*(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)/i,
    // "Latest Version</dt><dd>1.2.3" (definition list)
    /(?:Latest\s+)?Version<\/(?:dt|th|span|div|label)>\s*<(?:dd|td|span|div)[^>]*>\s*(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)/i,
    // Version in meta or structured data
    /softwareVersion["\s:>]+(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)/i,
    // "v1.2.3" near download/release context
    /(?:current|latest|release|version)[^<]{0,40}?v?(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)/i,
    // Version number right after "Version" text with possible HTML tags between
    /Version\s*(?:<[^>]*>\s*)*(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)/i,
  ];

  for (const pattern of patterns) {
    const match = html.match(pattern);
    if (match) {
      const ver = match[1];
      // Filter out dates and garbage
      if (!ver.match(/^20[0-2]\d\./) && !ver.match(/^\d{4}\./)) {
        return ver;
      }
    }
  }

  return null;
}

// Detect the current platform name for matching download links
const platformNames = {
  darwin: ['mac', 'macos', 'osx', 'os x', 'apple'],
  win32: ['win', 'windows', 'pc'],
  linux: ['linux', 'ubuntu', 'debian'],
};
const currentPlatform = process.platform;
const platformKeywords = platformNames[currentPlatform] || [];

// Extract the KVR product page URL and platform-specific download link
function extractKVRDownloadUrl(html, productUrl) {
  // Look for all download/get links on the page
  const linkPattern = /href="(https?:\/\/[^"]*(?:download|get|buy|release)[^"]*)"/gi;
  const allLinks = [];
  let match;
  while ((match = linkPattern.exec(html)) !== null) {
    allLinks.push(match[1]);
  }

  // Try to find a platform-specific download link
  for (const link of allLinks) {
    const lower = link.toLowerCase();
    if (platformKeywords.some(kw => lower.includes(kw))) {
      return { downloadUrl: link, hasPlatformDownload: true };
    }
  }

  // Check for platform text near download links in the HTML
  // e.g. "Mac" or "Windows" near an <a> tag
  for (const kw of platformKeywords) {
    const contextPattern = new RegExp(
      `(?:${kw})[^<]{0,80}?href="(https?:\/\/[^"]*(?:download|get)[^"]*)"` +
      `|href="(https?:\/\/[^"]*(?:download|get)[^"]*)"[^<]{0,80}?(?:${kw})`,
      'gi'
    );
    const contextMatch = contextPattern.exec(html);
    if (contextMatch) {
      return { downloadUrl: contextMatch[1] || contextMatch[2], hasPlatformDownload: true };
    }
  }

  // Fallback: any download link
  if (allLinks.length > 0) {
    return { downloadUrl: allLinks[0], hasPlatformDownload: false };
  }

  return { downloadUrl: productUrl, hasPlatformDownload: false };
}

// ── Main lookup function ──
async function findLatestVersion(plugin) {
  const name = plugin.name;
  const mfg = plugin.manufacturer !== 'Unknown' ? plugin.manufacturer : '';

  // Try KVR Audio first
  try {
    const productUrls = await searchKVR(name, mfg);

    // Check up to 3 product pages for a version match
    for (const productUrl of productUrls.slice(0, 3)) {
      try {
        const html = await fetch(productUrl);

        // Verify this page is actually about our plugin
        // (check if the plugin name appears on the page)
        const cleanName = name.replace(/[^a-zA-Z0-9]/g, '').toLowerCase();
        const pageText = html.replace(/<[^>]+>/g, '').toLowerCase();
        const nameInPage = pageText.includes(cleanName) ||
          pageText.includes(name.toLowerCase());

        if (!nameInPage) continue;

        const version = extractKVRVersion(html);
        if (version) {
          const { downloadUrl, hasPlatformDownload } = extractKVRDownloadUrl(html, productUrl);
          return {
            latestVersion: version,
            hasUpdate: compareVersions(version, plugin.version) > 0,
            source: 'kvr',
            updateUrl: downloadUrl,
            kvrUrl: productUrl,
            hasPlatformDownload,
          };
        }
      } catch {}
    }
  } catch {}

  // Fallback: search KVR via DuckDuckGo site-restricted search
  try {
    const query = `site:kvraudio.com ${mfg} ${name} VST version`.trim();
    const ddgUrl = `https://html.duckduckgo.com/html/?q=${encodeURIComponent(query)}`;
    const ddgHtml = await fetch(ddgUrl);

    const kvrLinks = [];
    const linkPattern = /href="[^"]*?(https?:\/\/(?:www\.)?kvraudio\.com\/product\/[^"&]+)/gi;
    let match;
    while ((match = linkPattern.exec(ddgHtml)) !== null) {
      kvrLinks.push(match[1]);
    }

    for (const kvrUrl of [...new Set(kvrLinks)].slice(0, 2)) {
      try {
        const html = await fetch(kvrUrl);
        const version = extractKVRVersion(html);
        if (version) {
          const { downloadUrl, hasPlatformDownload } = extractKVRDownloadUrl(html, kvrUrl);
          return {
            latestVersion: version,
            hasUpdate: compareVersions(version, plugin.version) > 0,
            source: 'kvr-ddg',
            updateUrl: downloadUrl,
            kvrUrl,
            hasPlatformDownload,
          };
        }
      } catch {}
    }
  } catch {}

  return null;
}

// ── Rate-limited concurrent processing ──
async function processPlugins() {
  const CONCURRENCY = 2;
  const DELAY_MS = 1000;
  let processed = 0;
  const total = plugins.length;
  const results = new Map();

  // Deduplicate: group by (manufacturer + name)
  const searchGroups = new Map();
  for (const plugin of plugins) {
    const key = `${plugin.manufacturer}|||${plugin.name}`.toLowerCase();
    if (!searchGroups.has(key)) {
      searchGroups.set(key, { plugin, siblings: [] });
    }
    searchGroups.get(key).siblings.push(plugin);
  }

  const groups = [...searchGroups.values()];
  let groupIdx = 0;

  async function processGroup(group) {
    const result = await findLatestVersion(group.plugin);

    const updatedPlugins = [];
    for (const sibling of group.siblings) {
      const currentVersion = sibling.version || 'Unknown';
      let pluginResult;

      if (result) {
        const hasUpdate = compareVersions(result.latestVersion, currentVersion) > 0;
        pluginResult = {
          ...sibling,
          currentVersion,
          latestVersion: result.latestVersion,
          hasUpdate: hasUpdate && currentVersion !== 'Unknown',
          updateUrl: result.updateUrl || result.kvrUrl || null,
          kvrUrl: result.kvrUrl || null,
          hasPlatformDownload: result.hasPlatformDownload || false,
          source: result.source,
        };
      } else {
        pluginResult = {
          ...sibling,
          currentVersion,
          latestVersion: currentVersion,
          hasUpdate: false,
          updateUrl: null,
          kvrUrl: null,
          hasPlatformDownload: false,
          source: 'not-found',
        };
      }

      results.set(sibling.path, pluginResult);
      updatedPlugins.push(pluginResult);
      processed++;
    }

    parentPort.postMessage({
      type: 'batch',
      plugins: updatedPlugins,
      processed,
      total,
    });
  }

  while (groupIdx < groups.length) {
    const batch = groups.slice(groupIdx, groupIdx + CONCURRENCY);
    groupIdx += CONCURRENCY;

    await Promise.all(batch.map(g => processGroup(g)));

    if (groupIdx < groups.length) {
      await new Promise(r => setTimeout(r, DELAY_MS));
    }
  }

  const finalPlugins = plugins.map(p => results.get(p.path) || {
    ...p,
    currentVersion: p.version || 'Unknown',
    latestVersion: p.version || 'Unknown',
    hasUpdate: false,
    updateUrl: null,
    kvrUrl: null,
    source: 'not-found',
  });

  parentPort.postMessage({ type: 'done', plugins: finalPlugins });
}

parentPort.postMessage({ type: 'start', total: plugins.length });
processPlugins().catch(err => {
  parentPort.postMessage({ type: 'error', message: err.message });
});
