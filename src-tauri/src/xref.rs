//! Cross-reference engine: extract plugin references from DAW project files.
//!
//! Parses 11 DAW formats: Ableton (.als), REAPER (.rpp), Bitwig (.bwproject),
//! Studio One (.song), DAWproject, FL Studio (.flp), Logic Pro (.logicx),
//! Cubase/Nuendo (.cpr), Pro Tools (.ptx/.ptf), and Reason (.reason).
//! Returns deduplicated lists of plugin names, manufacturers, and types.

use flate2::read::GzDecoder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PluginRef {
    pub name: String,
    #[serde(rename = "normalizedName")]
    pub normalized_name: String,
    pub manufacturer: String,
    #[serde(rename = "pluginType")]
    pub plugin_type: String, // "VST2", "VST3", "AU"
}

/// Regex to strip architecture/platform suffixes from plugin names.
static ARCH_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[\(\[](x64|x86_64|x86|arm64|aarch64|64[- ]?bit|32[- ]?bit|intel|apple silicon|universal|stereo|mono|vst3?|au|aax)[\)\]]$").unwrap()
});

// ── Ableton .als regexes (compiled once) ──

static ALS_VST2_BLOCK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<VstPluginInfo[^>]*>[\s\S]*?</VstPluginInfo>"#).unwrap());
static ALS_VST2_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<PlugName\s+Value="([^"]+)""#).unwrap());
static ALS_VST2_MFG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<Manufacturer\s+Value="([^"]+)""#).unwrap());
static ALS_VST3_BLOCK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<Vst3PluginInfo[^>]*>[\s\S]*?</Vst3PluginInfo>"#).unwrap());
static ALS_VST3_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<Name\s+Value="([^"]+)""#).unwrap());
static ALS_VST3_MFG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<DeviceCreator\s+Value="([^"]+)""#).unwrap());
static ALS_AU_BLOCK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<AuPluginInfo[^>]*>[\s\S]*?</AuPluginInfo>"#).unwrap());
static ALS_AU_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<Name\s+Value="([^"]+)""#).unwrap());
static ALS_AU_MFG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<Manufacturer\s+Value="([^"]+)""#).unwrap());

// ── REAPER .rpp regex (compiled once) ──

static RPP_PLUGIN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<(?:VST|AU|CLAP)\s+"(VST3?|AU|CLAP):\s*(.+?)\s*(?:\(([^)]+)\))?\s*""#).unwrap()
});

// ── Studio One (plugName only) / DAWproject XML regexes (compiled once) ──

static XML_PLUG_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"plugName="([^"]+)""#).unwrap());
static XML_DEVICE_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"deviceName="([^"]+)""#).unwrap());
static XML_PLUGIN_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<Plugin\s+name="([^"]+)""#).unwrap());

/// Normalize a plugin name for matching: lowercase, strip arch suffixes,
/// collapse whitespace, trim.
pub fn normalize_plugin_name(name: &str) -> String {
    let mut s = name.trim().to_string();
    // Strip trailing arch/platform suffixes repeatedly (e.g. "Serum (x64) (VST3)")
    loop {
        let before = s.len();
        s = ARCH_SUFFIX_RE.replace(&s, "").to_string();
        if s.len() == before {
            break;
        }
    }
    // Strip standalone trailing " x64", " x86" etc. without parens
    static BARE_SUFFIX_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\s+(x64|x86_64|x86|64bit|32bit)$").unwrap());
    s = BARE_SUFFIX_RE.replace(&s, "").to_string();
    // Collapse internal whitespace and lowercase
    let result = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    // If stripping removed everything, fall back to original lowercased name
    if result.is_empty() {
        name.trim().to_lowercase()
    } else {
        result
    }
}

/// Extract plugin references from a DAW project file.
/// Returns an empty vec for unsupported formats.
pub fn extract_plugins(project_path: &str) -> Vec<PluginRef> {
    let path = Path::new(project_path);
    let ext = path
        .extension()
        .or_else(|| {
            // Handle compound extensions like .rpp-bak
            let name = path.file_name()?.to_str()?;
            if name.ends_with(".rpp-bak") {
                Some(std::ffi::OsStr::new("rpp-bak"))
            } else {
                None
            }
        })
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut plugins = match ext.as_str() {
        "als" => parse_ableton(path),
        "rpp" | "rpp-bak" => parse_reaper(path),
        "bwproject" => parse_bitwig(path),
        "song" => parse_studio_one(path),
        "dawproject" => parse_dawproject(path),
        "flp" => parse_flp(path),
        "logicx" => parse_logic(path),
        "cpr" | "npr" => parse_cubase(path),
        "ptx" | "ptf" => parse_protools(path),
        "reason" => parse_reason(path),
        _ => vec![],
    };

    // Deduplicate by (normalized_name, plugin_type)
    let mut seen = HashSet::new();
    plugins.retain(|p| seen.insert((p.normalized_name.clone(), p.plugin_type.clone())));
    plugins.sort_by(|a, b| a.normalized_name.cmp(&b.normalized_name));
    plugins
}

/// Parse Ableton Live .als file (gzip-compressed XML).
///
/// Looks for:
/// - `<VstPluginInfo>` blocks with `<PlugName Value="..."/>` and `<Manufacturer Value="..."/>`
/// - `<Vst3PluginInfo>` blocks with `<Name Value="..."/>` and `<DeviceCreator Value="..."/>`
/// - `<AuPluginInfo>` blocks with `<Name Value="..."/>` and `<Manufacturer Value="..."/>`
fn parse_ableton(path: &Path) -> Vec<PluginRef> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut decoder = GzDecoder::new(&data[..]);
    let mut xml = String::new();
    if decoder.read_to_string(&mut xml).is_err() {
        return vec![];
    }

    let mut plugins = Vec::new();

    // VST2 plugins: <VstPluginInfo> ... <PlugName Value="X"/> ... <Manufacturer Value="Y"/>
    for block in ALS_VST2_BLOCK_RE.find_iter(&xml) {
        let text = block.as_str();
        let name = ALS_VST2_NAME_RE
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let mfg = ALS_VST2_MFG_RE
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        if !name.is_empty() {
            let normalized_name = normalize_plugin_name(&name);
            plugins.push(PluginRef {
                name,
                normalized_name,
                manufacturer: mfg,
                plugin_type: "VST2".into(),
            });
        }
    }

    // VST3 plugins: <Vst3PluginInfo> ... <Name Value="X"/> ... <DeviceCreator Value="Y"/>
    for block in ALS_VST3_BLOCK_RE.find_iter(&xml) {
        let text = block.as_str();
        let name = ALS_VST3_NAME_RE
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let mfg = ALS_VST3_MFG_RE
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        if !name.is_empty() {
            let normalized_name = normalize_plugin_name(&name);
            plugins.push(PluginRef {
                name,
                normalized_name,
                manufacturer: mfg,
                plugin_type: "VST3".into(),
            });
        }
    }

    // AU plugins: <AuPluginInfo> ... <Name Value="X"/> ... <Manufacturer Value="Y"/>
    for block in ALS_AU_BLOCK_RE.find_iter(&xml) {
        let text = block.as_str();
        let name = ALS_AU_NAME_RE
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let mfg = ALS_AU_MFG_RE
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        if !name.is_empty() {
            let normalized_name = normalize_plugin_name(&name);
            plugins.push(PluginRef {
                name,
                normalized_name,
                manufacturer: mfg,
                plugin_type: "AU".into(),
            });
        }
    }

    plugins
}

/// Parse REAPER .rpp file (plaintext).
///
/// Looks for lines like:
/// - `<VST "VST: Plugin Name (Manufacturer)" file.dll ...`
/// - `<VST "VST3: Plugin Name (Manufacturer)" file.vst3 ...`
/// - `<AU "AU: Plugin Name (Manufacturer)" ...`
/// - `<CLAP "CLAP: Plugin Name (Manufacturer)" ...`
fn parse_reaper(path: &Path) -> Vec<PluginRef> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    let mut plugins = Vec::new();

    // Match <VST "VST: Name (Mfg)" or <VST "VST3: Name (Mfg)" or <AU "AU: Name (Mfg)"
    for cap in RPP_PLUGIN_RE.captures_iter(&text) {
        let ptype = cap.get(1).map(|m| m.as_str()).unwrap_or("VST2");
        let name = cap
            .get(2)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let mfg = cap
            .get(3)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();

        if !name.is_empty() {
            let plugin_type = match ptype {
                "VST" => "VST2",
                "VST3" => "VST3",
                "AU" => "AU",
                "CLAP" => "CLAP",
                _ => "VST2",
            }
            .to_string();

            let normalized_name = normalize_plugin_name(&name);
            plugins.push(PluginRef {
                name,
                normalized_name,
                manufacturer: mfg,
                plugin_type,
            });
        }
    }

    plugins
}

/// Parse Bitwig Studio .bwproject file (binary with embedded strings).
///
/// Bitwig files have a `BtWg` magic header followed by binary-serialized
/// project data. Plugin references are stored as DLL/VST3/component paths
/// in plain text within the binary. We extract them via string scanning.
/// Parse Studio One .song file (ZIP containing song.xml + Devices/*.xml).
fn parse_studio_one(path: &Path) -> Vec<PluginRef> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    let mut all_xml = String::new();
    // Read all XML files in the archive
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .filter(|n| n.ends_with(".xml"))
        .collect();
    for name in &names {
        if let Ok(mut entry) = archive.by_name(name) {
            let mut s = String::new();
            if entry.read_to_string(&mut s).is_ok() {
                all_xml.push_str(&s);
                all_xml.push('\n');
            }
        }
    }
    if all_xml.is_empty() {
        return vec![];
    }
    // Only `plugName=` denotes a plugin instance. Studio One XML also has many
    // `label="..."` (tracks, buses, UI) and `deviceName="..."` is often the
    // vendor/model alongside `plugName` — matching those inflated xref counts badly.
    extract_plugins_from_xml(&all_xml, &[(&XML_PLUG_NAME_RE, "", "VST")])
}

/// Parse .dawproject file (ZIP containing project.xml — open standard).
fn parse_dawproject(path: &Path) -> Vec<PluginRef> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    let xml = match archive.by_name("project.xml") {
        Ok(mut entry) => {
            let mut s = String::new();
            entry.read_to_string(&mut s).ok();
            s
        }
        Err(_) => return vec![],
    };
    extract_plugins_from_xml(
        &xml,
        &[
            (&XML_PLUGIN_NAME_RE, "", "VST"),
            (&XML_DEVICE_NAME_RE, "", "VST"),
        ],
    )
}

/// Parse FL Studio .flp file (binary chunk format).
/// Uses binary string extraction + UTF-16LE scanning for plugin paths.
fn parse_flp(path: &Path) -> Vec<PluginRef> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let mut plugins = extract_plugins_from_binary(&data);
    // FL Studio stores many strings as UTF-16LE — scan for plugin paths in UTF-16
    plugins.extend(extract_plugins_utf16le(&data));
    plugins
}

/// Extract plugin references from UTF-16LE encoded strings in binary data.
/// FL Studio and some other DAWs use UTF-16LE for internal strings.
fn extract_plugins_utf16le(data: &[u8]) -> Vec<PluginRef> {
    let mut plugins = Vec::new();
    if data.len() < 2 {
        return plugins;
    }
    // Scan for runs of valid UTF-16LE characters
    let mut start = 0;
    while start + 1 < data.len() {
        let lo = data[start];
        let hi = data[start + 1];
        // Check if this looks like a printable ASCII char in UTF-16LE (lo=printable, hi=0)
        if hi == 0 && (0x20..=0x7E).contains(&lo) {
            let run_start = start;
            let mut end = start;
            while end + 1 < data.len() && data[end + 1] == 0 && (0x20..=0x7E).contains(&data[end]) {
                end += 2;
            }
            let char_count = (end - run_start) / 2;
            if char_count >= 6 {
                let u16s: Vec<u16> = data[run_start..end]
                    .chunks(2)
                    .map(|c| u16::from_le_bytes([c[0], c.get(1).copied().unwrap_or(0)]))
                    .collect();
                let s = String::from_utf16_lossy(&u16s);
                if let Some(p) = extract_plugin_from_string(&s) {
                    plugins.push(p);
                }
            }
            start = end;
        } else {
            start += 1;
        }
    }
    plugins
}

/// Parse Logic Pro .logicx package (contains binary plists with plugin info).
fn parse_logic(path: &Path) -> Vec<PluginRef> {
    let candidates = [
        path.join("Alternatives/000/ProjectData"),
        path.join("ProjectData"),
    ];
    let mut all_plugins = Vec::new();

    for plist_path in &candidates {
        if let Ok(data) = fs::read(plist_path) {
            // Try plist parsing
            if let Ok(val) = plist::from_bytes::<plist::Value>(&data) {
                extract_plugins_from_plist(&val, &mut all_plugins);
            }
            // Binary string extraction for file paths (.component, .vst3, etc.)
            all_plugins.extend(extract_plugins_from_binary(&data));
            // Extract AU identifiers
            all_plugins.extend(extract_au_identifiers(&data));
            // Extract known Logic plugin names by scanning for standalone strings
            all_plugins.extend(extract_logic_plugin_names(&data));
        }
    }

    if all_plugins.is_empty() {
        all_plugins = extract_plugins_from_dir(path);
    }

    all_plugins
}

/// Extract Logic Pro plugin names from binary data.
/// Logic stores plugin names as standalone readable strings in the ProjectData binary.
fn extract_logic_plugin_names(data: &[u8]) -> Vec<PluginRef> {
    let mut plugins = Vec::new();
    // Known third-party plugins and Logic stock effects to look for
    let stock_effects = [
        "Channel EQ",
        "Compressor",
        "Adaptive Limiter",
        "Multipressor",
        "Space Designer",
        "Tape Delay",
        "Stereo Delay",
        "ChromaVerb",
        "Exciter",
        "Overdrive",
        "AutoFilter",
        "Direction Mixer",
        "Gain",
        "Stereo Spread",
        "Limiter",
        "Noise Gate",
        "DeEsser",
        "Tremolo",
        "Phaser",
        "Flanger",
        "Chorus",
        "Ringshifter",
        "Pitch Correction",
        "Pitch Shifter",
        "Vocal Transformer",
    ];
    // Extract all readable strings and check for known plugin names
    let mut current = Vec::new();
    let mut found_names = std::collections::HashSet::new();
    for &byte in data {
        if (0x20..=0x7E).contains(&byte) {
            current.push(byte);
        } else {
            if current.len() >= 3 && current.len() <= 64 {
                let s = String::from_utf8_lossy(&current).to_string();
                // Skip common non-plugin strings
                if !s.contains('/')
                    && !s.contains('\\')
                    && !s.starts_with("com.")
                    && !s.starts_with("kD")
                    && !s.starts_with("0x")
                    && !s.starts_with("Aco")
                    && !s.starts_with("Output ")
                    && !s.starts_with("Input ")
                    && !s.starts_with("Automatic-")
                    && !s.contains("KeyLab")
                    && !s.ends_with(".pst")
                    && !s.ends_with(".aif")
                    && !s.ends_with(".wav")
                    && !s.ends_with(".cst")
                    && !s.ends_with(".exs")
                    && !found_names.contains(&s)
                {
                    let is_stock = stock_effects.contains(&s.as_str());
                    let known_third_party = [
                        "Sylenth1",
                        "Spire",
                        "Serum",
                        "Massive",
                        "Kontakt",
                        "Omnisphere",
                        "Nexus",
                        "Diva",
                        "Hive",
                        "Vital",
                        "Phase Plant",
                        "Pro-Q",
                        "Pro-L",
                        "Pro-R",
                        "Pro-C",
                        "Pro-G",
                        "Pro-MB",
                        "Ozone",
                        "Neutron",
                        "Trash",
                        "VocalSynth",
                        "Iris",
                        "Valhalla",
                        "FabFilter",
                        "iZotope",
                        "Waves",
                        "Soundtoys",
                        "LFOTool",
                        "CamelCrusher",
                        "OTT",
                        "Sausage Fattener",
                        "Saturn",
                        "Volcano",
                        "Timeless",
                        "Decapitator",
                        "EchoBoy",
                        "Radiator",
                        "Devil-Loc",
                        "PanMan",
                        "FilterFreak",
                        "PhaseMistress",
                        "RC-20",
                        "Kickstart",
                        "Cableguys",
                        "Portal",
                        "Output",
                        "Arturia",
                        "u-he",
                        "Xfer",
                        "Native Instruments",
                        "Spectrasonics",
                        "Alchemy",
                        "ES2",
                        "EXS24",
                        "Retro Synth",
                        "Drum Kit Designer",
                    ];
                    let is_known = known_third_party
                        .iter()
                        .any(|&kp| s.starts_with(kp) || s == kp);

                    if is_stock || is_known {
                        // Trim trailing non-alphanumeric junk (binary artifacts)
                        let s = s
                            .trim_end_matches(|c: char| {
                                !c.is_alphanumeric() && c != ')' && c != ']'
                            })
                            .to_string();
                        if s.len() < 2 {
                            current.clear();
                            continue;
                        }
                        found_names.insert(s.clone());
                        let normalized = normalize_plugin_name(&s);
                        if !normalized.is_empty() {
                            plugins.push(PluginRef {
                                name: s,
                                normalized_name: normalized,
                                manufacturer: String::new(),
                                plugin_type: if is_stock {
                                    "AU (Stock)".into()
                                } else {
                                    "AU".into()
                                },
                            });
                        }
                    }
                }
            }
            current.clear();
        }
    }
    plugins
}

/// Extract Audio Unit identifiers from binary data.
/// Logic stores AU plugins as 4-char codes like "aufx", "aumu", "aumf" followed by subtype and manufacturer.
fn extract_au_identifiers(data: &[u8]) -> Vec<PluginRef> {
    let mut plugins = Vec::new();
    let mut current = Vec::new();
    // Look for readable strings that could be AU plugin names
    for &byte in data {
        if (0x20..=0x7E).contains(&byte) {
            current.push(byte);
        } else {
            if current.len() >= 4 {
                let s = String::from_utf8_lossy(&current).to_string();
                // Match common AU plugin name patterns
                // Logic stores plugin names as readable strings near AU type codes
                if !s.contains('/')
                    && !s.contains('\\')
                    && !s.contains("com.apple")
                    && s.len() >= 4
                    && s.len() <= 64
                    && (s.ends_with(".component")
                        || s.contains("AUPlugin")
                        || s.contains("AudioUnit"))
                {
                    let name = s.trim_end_matches(".component").trim();
                    if name.len() >= 3 {
                        let normalized = normalize_plugin_name(name);
                        if !normalized.is_empty() {
                            plugins.push(PluginRef {
                                name: name.to_string(),
                                normalized_name: normalized,
                                manufacturer: String::new(),
                                plugin_type: "AU".into(),
                            });
                        }
                    }
                }
            }
            current.clear();
        }
    }
    plugins
}

/// Parse Cubase/Nuendo .cpr file (binary — string extraction + Plugin Name markers).
fn parse_cubase(path: &Path) -> Vec<PluginRef> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let mut plugins = extract_plugins_from_binary(&data);
    // Cubase stores plugin names after "Plugin Name" markers
    plugins.extend(extract_named_plugins(&data, b"Plugin Name"));
    plugins
}

/// Parse Pro Tools .ptx/.ptf file.
/// Note: .ptf files (Pro Tools 7-10) are XOR-encrypted and require decryption.
/// .ptx files (Pro Tools 10+) use a different format.
/// Both are attempted via string extraction; encrypted files will yield 0 results.
fn parse_protools(path: &Path) -> Vec<PluginRef> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let mut plugins = extract_plugins_from_binary(&data);
    // Pro Tools also stores plugin names near specific markers
    plugins.extend(extract_named_plugins(&data, b"PlugIn Name"));
    plugins.extend(extract_named_plugins(&data, b"Insert Name"));
    plugins
}

/// Parse Reason .reason file (binary — string extraction).
fn parse_reason(path: &Path) -> Vec<PluginRef> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    extract_plugins_from_binary(&data)
}

// ── Shared extraction helpers ──

/// Extract plugin names from XML using pre-compiled regex patterns.
fn extract_plugins_from_xml(xml: &str, patterns: &[(&Regex, &str, &str)]) -> Vec<PluginRef> {
    let mut plugins = Vec::new();
    for &(re, manufacturer_default, type_default) in patterns {
        for cap in re.captures_iter(xml) {
            if let Some(name) = cap.get(1) {
                let n = name.as_str().trim();
                if n.is_empty() || n.len() < 2 {
                    continue;
                }
                let normalized = normalize_plugin_name(n);
                if normalized.is_empty() {
                    continue;
                }
                plugins.push(PluginRef {
                    name: n.to_string(),
                    normalized_name: normalized,
                    manufacturer: manufacturer_default.to_string(),
                    plugin_type: type_default.to_string(),
                });
            }
        }
    }
    plugins
}

/// Extract plugin references from a binary file via string scanning.
/// Looks for paths ending in .dll, .vst3, .component, .clap, .aaxplugin
fn extract_plugins_from_binary(data: &[u8]) -> Vec<PluginRef> {
    let mut plugins = Vec::new();
    let mut current = Vec::new();
    for &byte in data {
        if (0x20..=0x7E).contains(&byte) {
            current.push(byte);
        } else {
            if current.len() >= 6 {
                let s = String::from_utf8_lossy(&current).to_string();
                if let Some(p) = extract_plugin_from_string(&s) {
                    plugins.push(p);
                }
            }
            current.clear();
        }
    }
    if current.len() >= 6 {
        let s = String::from_utf8_lossy(&current).to_string();
        if let Some(p) = extract_plugin_from_string(&s) {
            plugins.push(p);
        }
    }
    plugins
}

/// Extract plugins from all files in a directory (for .logicx packages).
fn extract_plugins_from_dir(dir: &Path) -> Vec<PluginRef> {
    let mut plugins = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(data) = fs::read(&p) {
                    plugins.extend(extract_plugins_from_binary(&data));
                }
            } else if p.is_dir() && plugins.len() < 500 {
                plugins.extend(extract_plugins_from_dir(&p));
            }
        }
    }
    plugins
}

/// Extract plugin names from a Logic Pro plist structure.
fn extract_plugins_from_plist(val: &plist::Value, plugins: &mut Vec<PluginRef>) {
    match val {
        plist::Value::Dictionary(dict) => {
            // Look for plugin name keys
            for key in ["pluginName", "PluginName", "name", "Name", "plugName"] {
                if let Some(plist::Value::String(name)) = dict.get(key) {
                    let n = name.trim();
                    if n.len() >= 2 {
                        let normalized = normalize_plugin_name(n);
                        if !normalized.is_empty() {
                            plugins.push(PluginRef {
                                name: n.to_string(),
                                normalized_name: normalized,
                                manufacturer: dict
                                    .get("manufacturer")
                                    .and_then(|v| v.as_string())
                                    .unwrap_or("")
                                    .to_string(),
                                plugin_type: dict
                                    .get("pluginType")
                                    .and_then(|v| v.as_string())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| "AU".into()),
                            });
                        }
                    }
                }
            }
            for (_, v) in dict.iter() {
                extract_plugins_from_plist(v, plugins);
            }
        }
        plist::Value::Array(arr) => {
            for v in arr {
                extract_plugins_from_plist(v, plugins);
            }
        }
        _ => {}
    }
}

/// Try to extract a plugin reference from a single string (path or name).
/// Handles both exact suffix match and embedded paths (e.g. "Serum.dll8" in FLP chunks).
fn extract_plugin_from_string(s: &str) -> Option<PluginRef> {
    let exts = [
        (".dll", "VST2"),
        (".vst3", "VST3"),
        (".component", "AU"),
        (".clap", "CLAP"),
        (".aaxplugin", "AAX"),
    ];
    for (ext, ptype) in &exts {
        // Find the extension anywhere in the string (not just at the end)
        if let Some(pos) = s.find(ext) {
            // Extract the substring up to and including the extension
            let path_part = &s[..pos + ext.len()];
            let name = path_part
                .rsplit(['\\', '/'])
                .next()?
                .trim_end_matches(ext)
                .trim();
            if name.is_empty() || name.len() < 2 {
                continue;
            }
            if name.contains("VstPlugins")
                || name.contains("Program Files")
                || name.contains("CommonFiles")
            {
                continue;
            }
            let normalized = normalize_plugin_name(name);
            if normalized.is_empty() {
                continue;
            }
            return Some(PluginRef {
                name: name.to_string(),
                normalized_name: normalized,
                manufacturer: String::new(),
                plugin_type: ptype.to_string(),
            });
        }
    }
    None
}

/// Extract plugin names that follow a marker string in binary data.
/// Used by Cubase (.cpr) where plugins appear as "Plugin Name" followed by the name.
fn extract_named_plugins(data: &[u8], marker: &[u8]) -> Vec<PluginRef> {
    let mut plugins = Vec::new();
    let builtin = [
        "Standard Panner",
        "Stereo Combined Panner",
        "Mono",
        "Stereo",
        "No Bus",
    ];
    let mut pos = 0;
    while pos + marker.len() < data.len() {
        if let Some(idx) = data[pos..].windows(marker.len()).position(|w| w == marker) {
            let after = pos + idx + marker.len();
            // Skip non-printable bytes to find the next readable string
            let mut start = after;
            while start < data.len() && (data[start] < 0x20 || data[start] > 0x7E) {
                start += 1;
            }
            if start < data.len() {
                let mut end = start;
                while end < data.len() && data[end] >= 0x20 && data[end] <= 0x7E {
                    end += 1;
                }
                if end - start >= 3 && end - start <= 100 {
                    let name = String::from_utf8_lossy(&data[start..end]).to_string();
                    if !builtin.contains(&name.as_str())
                        && !name.starts_with("VST")
                        && !name.contains("Plugin")
                    {
                        let normalized = normalize_plugin_name(&name);
                        if !normalized.is_empty() {
                            plugins.push(PluginRef {
                                name: name.clone(),
                                normalized_name: normalized,
                                manufacturer: String::new(),
                                plugin_type: "VST".into(),
                            });
                        }
                    }
                }
            }
            pos = after + 1;
        } else {
            break;
        }
    }
    plugins
}

/// Parse Bitwig .bwproject file (binary — reuses shared string extraction).
fn parse_bitwig(path: &Path) -> Vec<PluginRef> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    extract_plugins_from_binary(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn test_normalize_plugin_name_whitespace_only() {
        assert_eq!(normalize_plugin_name("   "), "");
    }

    #[test]
    fn test_plugin_ref_json_roundtrip() {
        let p = PluginRef {
            name: "Serum".into(),
            normalized_name: "serum".into(),
            manufacturer: "Xfer Records".into(),
            plugin_type: "VST3".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: PluginRef = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn test_extract_empty_for_unsupported() {
        let result = extract_plugins("/some/file.flp");
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_nonexistent_file() {
        let result = extract_plugins("/nonexistent/project.als");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_ableton_vst2() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Ableton>
  <LiveSet>
    <Tracks>
      <MidiTrack>
        <DeviceChain>
          <Devices>
            <PluginDevice>
              <PluginDesc>
                <VstPluginInfo>
                  <PlugName Value="Serum" />
                  <Manufacturer Value="Xfer Records" />
                </VstPluginInfo>
              </PluginDesc>
            </PluginDevice>
          </Devices>
        </DeviceChain>
      </MidiTrack>
    </Tracks>
  </LiveSet>
</Ableton>"#;

        let tmp = std::env::temp_dir().join("test_xref_als_vst2.als");
        let f = fs::File::create(&tmp).unwrap();
        let mut enc = GzEncoder::new(f, Compression::default());
        enc.write_all(xml.as_bytes()).unwrap();
        enc.finish().unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Serum");
        assert_eq!(result[0].manufacturer, "Xfer Records");
        assert_eq!(result[0].plugin_type, "VST2");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_ableton_vst3() {
        let xml = r#"<Ableton>
  <Vst3PluginInfo>
    <Name Value="Pro-Q 3" />
    <DeviceCreator Value="FabFilter" />
  </Vst3PluginInfo>
</Ableton>"#;

        let tmp = std::env::temp_dir().join("test_xref_als_vst3.als");
        let f = fs::File::create(&tmp).unwrap();
        let mut enc = GzEncoder::new(f, Compression::default());
        enc.write_all(xml.as_bytes()).unwrap();
        enc.finish().unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Pro-Q 3");
        assert_eq!(result[0].manufacturer, "FabFilter");
        assert_eq!(result[0].plugin_type, "VST3");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_ableton_au() {
        let xml = r#"<Ableton>
  <AuPluginInfo>
    <Name Value="AUReverb2" />
    <Manufacturer Value="Apple" />
  </AuPluginInfo>
</Ableton>"#;

        let tmp = std::env::temp_dir().join("test_xref_als_au.als");
        let f = fs::File::create(&tmp).unwrap();
        let mut enc = GzEncoder::new(f, Compression::default());
        enc.write_all(xml.as_bytes()).unwrap();
        enc.finish().unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "AUReverb2");
        assert_eq!(result[0].plugin_type, "AU");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_ableton_multiple_deduped() {
        let xml = r#"<Ableton>
  <VstPluginInfo><PlugName Value="Serum" /><Manufacturer Value="Xfer" /></VstPluginInfo>
  <VstPluginInfo><PlugName Value="Serum" /><Manufacturer Value="Xfer" /></VstPluginInfo>
  <Vst3PluginInfo><Name Value="Pro-Q 3" /><DeviceCreator Value="FabFilter" /></Vst3PluginInfo>
</Ableton>"#;

        let tmp = std::env::temp_dir().join("test_xref_als_multi.als");
        let f = fs::File::create(&tmp).unwrap();
        let mut enc = GzEncoder::new(f, Compression::default());
        enc.write_all(xml.as_bytes()).unwrap();
        enc.finish().unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 2); // Serum deduped
        assert!(result.iter().any(|p| p.name == "Serum"));
        assert!(result.iter().any(|p| p.name == "Pro-Q 3"));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_reaper_vst2() {
        let rpp = r#"<REAPER_PROJECT 0.1 "7.0"
  <TRACK
    <FXCHAIN
      <VST "VST: Serum (Xfer Records)" Serum_x64.dll 0 "" 1397572658
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_vst2.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Serum");
        assert_eq!(result[0].manufacturer, "Xfer Records");
        assert_eq!(result[0].plugin_type, "VST2");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_reaper_vst3() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <VST "VST3: Pro-Q 3 (FabFilter)" "{ABCDEF}" 0
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_vst3.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Pro-Q 3");
        assert_eq!(result[0].manufacturer, "FabFilter");
        assert_eq!(result[0].plugin_type, "VST3");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_reaper_mixed() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <VST "VST: Serum (Xfer Records)" Serum.dll 0 "" 123
      >
      <VST "VST3: Ozone 11 (iZotope, Inc.)" Ozone.vst3 0 "" 456
      >
      <AU "AU: AUHighShelfFilter (Apple)" "" 0 "" 789
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_mixed.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 3);
        assert!(result
            .iter()
            .any(|p| p.name == "Serum" && p.plugin_type == "VST2"));
        assert!(result
            .iter()
            .any(|p| p.name == "Ozone 11" && p.plugin_type == "VST3"));
        assert!(result
            .iter()
            .any(|p| p.name == "AUHighShelfFilter" && p.plugin_type == "AU"));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_reaper_no_manufacturer() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <VST "VST: ReaComp" reacomp.dll 0
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_nomfg.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "ReaComp");
        assert_eq!(result[0].manufacturer, "");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_reaper_deduplicates() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <VST "VST: Serum (Xfer Records)" Serum.dll 0 "" 123
      >
    >
  >
  <TRACK
    <FXCHAIN
      <VST "VST: Serum (Xfer Records)" Serum.dll 0 "" 123
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_dedup.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_reaper_empty_fx_chain() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_empty.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert!(result.is_empty());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_ableton_not_gzip() {
        let tmp = std::env::temp_dir().join("test_xref_als_bad.als");
        fs::write(&tmp, b"not gzip data").unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert!(result.is_empty());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_ableton_empty_xml() {
        // Valid gzip but no plugin blocks at all
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Ableton>
  <LiveSet>
    <Tracks>
      <AudioTrack>
        <DeviceChain>
          <Devices />
        </DeviceChain>
      </AudioTrack>
    </Tracks>
  </LiveSet>
</Ableton>"#;

        let tmp = std::env::temp_dir().join("test_xref_als_empty_xml.als");
        let f = fs::File::create(&tmp).unwrap();
        let mut enc = GzEncoder::new(f, Compression::default());
        enc.write_all(xml.as_bytes()).unwrap();
        enc.finish().unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert!(
            result.is_empty(),
            "No plugin blocks should yield empty result"
        );

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_reaper_clap() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <CLAP "CLAP: Surge XT (Surge Synth Team)" com.surge-synth-team.surge-xt 0
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_clap.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Surge XT");
        assert_eq!(result[0].manufacturer, "Surge Synth Team");
        assert_eq!(result[0].plugin_type, "CLAP");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_extract_rpp_bak_extension() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <VST "VST: Vital (Matt Tytel)" Vital.dll 0 "" 999
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref.rpp-bak");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1, ".rpp-bak should be treated as REAPER");
        assert_eq!(result[0].name, "Vital");
        assert_eq!(result[0].plugin_type, "VST2");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_normalize_plugin_name_basic() {
        assert_eq!(normalize_plugin_name("Serum"), "serum");
        assert_eq!(normalize_plugin_name("Pro-Q 3"), "pro-q 3");
        assert_eq!(normalize_plugin_name("  Diva  "), "diva");
    }

    #[test]
    fn test_normalize_plugin_name_mixed_case_collapses_to_lowercase() {
        assert_eq!(
            normalize_plugin_name("FabFilter Pro-Q 3"),
            "fabfilter pro-q 3"
        );
    }

    #[test]
    fn test_normalize_strips_arch_suffixes() {
        assert_eq!(normalize_plugin_name("Serum (x64)"), "serum");
        assert_eq!(normalize_plugin_name("Kontakt (x86_64)"), "kontakt");
        assert_eq!(normalize_plugin_name("Massive (64-bit)"), "massive");
        assert_eq!(normalize_plugin_name("Sylenth1 (32-bit)"), "sylenth1");
        assert_eq!(normalize_plugin_name("Reaktor (ARM64)"), "reaktor");
        assert_eq!(
            normalize_plugin_name("Omnisphere (Universal)"),
            "omnisphere"
        );
        assert_eq!(normalize_plugin_name("Pigments [x64]"), "pigments");
        assert_eq!(normalize_plugin_name("Vital (Stereo)"), "vital");
    }

    #[test]
    fn test_normalize_strips_bare_arch_suffix() {
        assert_eq!(normalize_plugin_name("Serum x64"), "serum");
        assert_eq!(normalize_plugin_name("Kontakt x86_64"), "kontakt");
        assert_eq!(normalize_plugin_name("Massive x86"), "massive");
    }

    #[test]
    fn test_normalize_strips_multiple_suffixes() {
        assert_eq!(normalize_plugin_name("Serum (x64) (VST3)"), "serum");
        assert_eq!(normalize_plugin_name("Kontakt (Stereo) (x64)"), "kontakt");
    }

    #[test]
    fn test_normalize_preserves_inner_parens() {
        assert_eq!(normalize_plugin_name("EQ (3-band)"), "eq (3-band)");
        assert_eq!(
            normalize_plugin_name("Compressor (Legacy)"),
            "compressor (legacy)"
        );
    }

    #[test]
    fn test_normalize_collapses_whitespace() {
        assert_eq!(normalize_plugin_name("Pro   Q  3"), "pro q 3");
    }

    #[test]
    fn test_normalize_all_suffix_fallback() {
        // If stripping arch suffixes removes everything, fall back to original
        assert_eq!(normalize_plugin_name("(x64)"), "(x64)");
        assert_eq!(normalize_plugin_name("(x64) (VST3)"), "(x64) (vst3)");
        // Empty/whitespace input
        assert_eq!(normalize_plugin_name(""), "");
        assert_eq!(normalize_plugin_name("   "), "");
    }

    #[test]
    fn test_normalize_strips_au_vst_aax_brackets() {
        assert_eq!(normalize_plugin_name("Pro-Q 3 (AU)"), "pro-q 3");
        assert_eq!(normalize_plugin_name("Serum (VST3)"), "serum");
        assert_eq!(normalize_plugin_name("Tune (AAX)"), "tune");
    }

    #[test]
    fn test_normalize_intel_bracket() {
        assert_eq!(normalize_plugin_name("Legacy (Intel)"), "legacy");
    }

    #[test]
    fn test_normalize_strips_apple_silicon_bracket() {
        assert_eq!(
            normalize_plugin_name("Melodyne (Apple Silicon)"),
            "melodyne"
        );
    }

    #[test]
    fn test_normalize_equivalent_after_strip() {
        assert_eq!(
            normalize_plugin_name("Massive (x64)"),
            normalize_plugin_name("Massive x64")
        );
    }

    #[test]
    fn test_extract_plugin_from_string_embedded_windows_path() {
        let s = r"C:\VSTPlugins\Xfer\Serum.vst3";
        let p = extract_plugin_from_string(s).expect("vst3 in path");
        assert_eq!(p.name, "Serum");
        assert_eq!(p.plugin_type, "VST3");
        assert_eq!(p.normalized_name, "serum");
    }

    #[test]
    fn test_extract_plugin_from_string_skips_vstplugins_substring_in_stem() {
        // `name` is the file stem; embedded "VstPlugins" skips generic installer noise
        let s = r"C:\Plugins\SerumVstPluginsBundle.vst3";
        assert!(extract_plugin_from_string(s).is_none());
    }

    #[test]
    fn test_extract_plugin_from_string_short_stem_skipped() {
        let s = r"C:\p\X.vst3";
        assert!(extract_plugin_from_string(s).is_none());
    }

    #[test]
    fn test_extract_plugin_from_string_clap_suffix() {
        let s = "/lib/CLAP/Pigments.clap";
        let p = extract_plugin_from_string(s).unwrap();
        assert_eq!(p.name, "Pigments");
        assert_eq!(p.plugin_type, "CLAP");
    }

    #[test]
    fn test_extract_au_identifiers_component_bundle_name() {
        let mut data = b"MyDelay.component".to_vec();
        data.push(0);
        let refs = extract_au_identifiers(&data);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "MyDelay");
        assert_eq!(refs[0].plugin_type, "AU");
    }

    #[test]
    fn test_extract_plugins_from_xml_regex_capture() {
        // Avoid attributes like deviceName= — substring `name="` would match inside it first
        let xml = r#"<Plugin name="Serum" />"#;
        let re = Regex::new(r#"name="([^"]+)""#).unwrap();
        let patterns: &[(&Regex, &str, &str)] = &[(&re, "Xfer Records", "VST3")];
        let refs = extract_plugins_from_xml(xml, patterns);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "Serum");
        assert_eq!(refs[0].manufacturer, "Xfer Records");
        assert_eq!(refs[0].plugin_type, "VST3");
    }

    #[test]
    fn test_extract_plugins_from_xml_skips_single_char_name() {
        let xml = r#"<x name="X" />"#;
        let re = Regex::new(r#"name="([^"]+)""#).unwrap();
        let patterns: &[(&Regex, &str, &str)] = &[(&re, "Co", "VST3")];
        assert!(
            extract_plugins_from_xml(xml, patterns).is_empty(),
            "stem length < 2 must be skipped"
        );
    }

    #[test]
    fn test_dedup_case_insensitive() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <VST "VST: Serum (Xfer Records)" Serum.dll 0 "" 123
      >
      <VST "VST: SERUM (Xfer Records)" Serum.dll 0 "" 456
      >
      <VST "VST: serum (Xfer)" Serum.dll 0 "" 789
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_case_dedup.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1, "case variants should dedup to one");
        assert_eq!(result[0].normalized_name, "serum");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_dedup_arch_suffix_variants() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <VST "VST: Serum (Xfer)" Serum.dll 0 "" 1
      >
      <VST "VST: Serum x64 (Xfer)" Serum_x64.dll 0 "" 2
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_arch_dedup.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 1, "arch suffix variants should dedup");
        assert_eq!(result[0].normalized_name, "serum");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_results_sorted_by_name() {
        let rpp = r#"<REAPER_PROJECT
  <TRACK
    <FXCHAIN
      <VST "VST: Zebra2 (u-he)" z.dll 0 "" 1
      >
      <VST "VST: Diva (u-he)" d.dll 0 "" 2
      >
      <VST "VST: Ace (u-he)" a.dll 0 "" 3
      >
    >
  >
>"#;
        let tmp = std::env::temp_dir().join("test_xref_rpp_sorted.rpp");
        fs::write(&tmp, rpp).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "Ace");
        assert_eq!(result[1].name, "Diva");
        assert_eq!(result[2].name, "Zebra2");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_extract_bitwig_plugin_dll() {
        let p = extract_plugin_from_string(r"C:\Program Files\Steinberg\VstPlugins\Serum.dll");
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(p.name, "Serum");
        assert_eq!(p.plugin_type, "VST2");
    }

    #[test]
    fn test_extract_bitwig_plugin_vst3() {
        let p = extract_plugin_from_string("/Library/Audio/Plug-Ins/VST3/FabFilter Pro-Q 3.vst3");
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(p.name, "FabFilter Pro-Q 3");
        assert_eq!(p.plugin_type, "VST3");
    }

    #[test]
    fn test_extract_bitwig_plugin_au() {
        let p = extract_plugin_from_string("/Library/Audio/Plug-Ins/Components/Massive.component");
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(p.name, "Massive");
        assert_eq!(p.plugin_type, "AU");
    }

    #[test]
    fn test_extract_bitwig_plugin_clap() {
        let p = extract_plugin_from_string("/Library/Audio/Plug-Ins/CLAP/Vital.clap");
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(p.name, "Vital");
        assert_eq!(p.plugin_type, "CLAP");
    }

    #[test]
    fn test_extract_bitwig_plugin_rejects_dir() {
        // VstPlugins directory path should not be extracted as a plugin
        assert!(extract_plugin_from_string(r"C:\Program Files\Steinberg\VstPlugins").is_none());
    }

    #[test]
    fn test_extract_bitwig_plugin_strips_path() {
        let p = extract_plugin_from_string(r"MeldaProduction\Modulation\MFlanger.dll");
        assert!(p.is_some());
        assert_eq!(p.unwrap().name, "MFlanger");
    }

    #[test]
    fn test_parse_bitwig_synthetic() {
        // Create a fake bwproject with embedded plugin strings
        let tmp = std::env::temp_dir().join("test_bitwig.bwproject");
        let mut data = b"BtWg0003".to_vec();
        data.extend_from_slice(&[0u8; 100]); // padding
        data.extend_from_slice(b"C:\\VstPlugins\\Serum.dll");
        data.extend_from_slice(&[0u8; 20]);
        data.extend_from_slice(b"/Library/Audio/Plug-Ins/VST3/Pro-Q 3.vst3");
        data.extend_from_slice(&[0u8; 20]);
        data.extend_from_slice(b"/Library/Audio/Plug-Ins/Components/Kontakt.component");
        data.extend_from_slice(&[0u8; 20]);
        fs::write(&tmp, &data).unwrap();

        let result = extract_plugins(tmp.to_str().unwrap());
        assert!(
            result.len() >= 3,
            "should find at least 3 plugins, got {}",
            result.len()
        );
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.contains(&"Serum"),
            "should find Serum, got {:?}",
            names
        );
        assert!(
            names.contains(&"Pro-Q 3"),
            "should find Pro-Q 3, got {:?}",
            names
        );
        assert!(
            names.contains(&"Kontakt"),
            "should find Kontakt, got {:?}",
            names
        );

        let _ = fs::remove_file(&tmp);
    }

    // ── Shared extraction tests ──

    #[test]
    fn test_binary_extraction_all_plugin_types() {
        let mut data = vec![0u8; 100];
        for (path, expected_type) in [
            ("C:\\VSTPlugins\\Massive.dll", "VST2"),
            ("/Library/Audio/Plug-Ins/VST3/Serum.vst3", "VST3"),
            ("/Library/Audio/Plug-Ins/Components/Kontakt.component", "AU"),
            ("/Library/Audio/Plug-Ins/CLAP/Vital.clap", "CLAP"),
            ("C:\\AAX\\Pro-Q 3.aaxplugin", "AAX"),
        ] {
            data.extend_from_slice(path.as_bytes());
            data.extend_from_slice(&[0; 50]);
            let _ = expected_type;
        }
        let result = extract_plugins_from_binary(&data);
        let types: Vec<&str> = result.iter().map(|p| p.plugin_type.as_str()).collect();
        assert!(types.contains(&"VST2"), "missing VST2");
        assert!(types.contains(&"VST3"), "missing VST3");
        assert!(types.contains(&"AU"), "missing AU");
        assert!(types.contains(&"CLAP"), "missing CLAP");
        assert!(types.contains(&"AAX"), "missing AAX");
    }

    #[test]
    fn test_extract_plugin_with_trailing_junk() {
        // FLP-style: plugin path followed by chunk byte
        let p = extract_plugin_from_string("F:\\VSTPlugins\\Serum_x64.dll8");
        assert!(p.is_some(), "should extract despite trailing '8'");
        assert_eq!(p.unwrap().name, "Serum_x64");
    }

    #[test]
    fn test_extract_plugin_embedded_in_longer_string() {
        let p = extract_plugin_from_string("some_prefix/Sylenth1.dll/some_suffix");
        assert!(p.is_some());
        assert_eq!(p.unwrap().name, "Sylenth1");
    }

    // ── FLP tests ──

    #[test]
    fn test_flp_ascii_extraction() {
        let mut data = vec![0u8; 100];
        data.extend_from_slice(b"C:\\Program Files\\VSTPlugins\\Sylenth1.dll");
        data.extend_from_slice(&[0; 50]);
        data.extend_from_slice(b"C:\\VST3\\FabFilter Pro-Q 3.vst3");
        data.extend_from_slice(&[0; 50]);
        let result = extract_plugins_from_binary(&data);
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Sylenth1"), "missing Sylenth1: {:?}", names);
        assert!(
            names.contains(&"FabFilter Pro-Q 3"),
            "missing Pro-Q 3: {:?}",
            names
        );
    }

    #[test]
    fn test_flp_utf16le_extraction() {
        let mut data = vec![0u8; 50];
        // "Serum_x64.dll" as UTF-16LE
        for c in "Serum_x64.dll".chars() {
            data.push(c as u8);
            data.push(0);
        }
        data.extend_from_slice(&[0; 50]);
        let result = extract_plugins_utf16le(&data);
        assert!(!result.is_empty(), "UTF-16LE extraction failed");
        assert_eq!(result[0].name, "Serum_x64");
    }

    #[test]
    fn test_flp_combined_ascii_and_utf16() {
        let tmp = std::env::temp_dir().join("test_xref_flp_combined.flp");
        let mut data = vec![0u8; 100];
        data.extend_from_slice(b"C:\\Plugins\\OTT.dll");
        data.extend_from_slice(&[0; 30]);
        for c in "F:\\VSTPlugins\\Massive.dll".chars() {
            data.push(c as u8);
            data.push(0);
        }
        data.extend_from_slice(&[0; 50]);
        fs::write(&tmp, &data).unwrap();
        let result = parse_flp(&tmp);
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("OTT")),
            "missing OTT: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n.contains("Massive")),
            "missing Massive: {:?}",
            names
        );
        let _ = fs::remove_file(&tmp);
    }

    // ── Cubase tests ──

    #[test]
    fn test_cubase_plugin_name_markers() {
        let mut data = vec![0u8; 50];
        data.extend_from_slice(b"Plugin Name");
        data.push(0);
        data.extend_from_slice(b"Spire-1.5");
        data.extend_from_slice(&[0; 30]);
        data.extend_from_slice(b"Plugin Name");
        data.push(0);
        data.extend_from_slice(b"LFOTool");
        data.extend_from_slice(&[0; 30]);
        // Should skip builtin
        data.extend_from_slice(b"Plugin Name");
        data.push(0);
        data.extend_from_slice(b"Standard Panner");
        data.extend_from_slice(&[0; 30]);
        let result = extract_named_plugins(&data, b"Plugin Name");
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Spire-1.5"), "missing Spire: {:?}", names);
        assert!(names.contains(&"LFOTool"), "missing LFOTool: {:?}", names);
        assert!(
            !names.contains(&"Standard Panner"),
            "should filter Standard Panner"
        );
    }

    #[test]
    fn test_cubase_binary_paths_plus_markers() {
        let mut data = vec![0u8; 50];
        data.extend_from_slice(b"C:\\VST3\\Serum.vst3");
        data.extend_from_slice(&[0; 30]);
        data.extend_from_slice(b"Plugin Name");
        data.push(0);
        data.extend_from_slice(b"LFOTool");
        data.extend_from_slice(&[0; 30]);
        let tmp = std::env::temp_dir().join("test_xref_cubase.cpr");
        fs::write(&tmp, &data).unwrap();
        let result = extract_plugins(tmp.to_str().unwrap());
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(
            names.contains(&"Serum"),
            "missing Serum from binary: {:?}",
            names
        );
        assert!(
            names.contains(&"LFOTool"),
            "missing LFOTool from marker: {:?}",
            names
        );
        let _ = fs::remove_file(&tmp);
    }

    // ── Logic tests ──

    #[test]
    fn test_logic_known_plugins_extraction() {
        let mut data = vec![0u8; 50];
        // Embed known plugin names as standalone strings
        for name in ["Sylenth1", "Channel EQ", "Compressor", "Alchemy", "Hive"] {
            data.extend_from_slice(name.as_bytes());
            data.extend_from_slice(&[0; 10]);
        }
        let result = extract_logic_plugin_names(&data);
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Sylenth1"), "missing Sylenth1: {:?}", names);
        assert!(
            names.contains(&"Channel EQ"),
            "missing Channel EQ: {:?}",
            names
        );
        assert!(
            names.contains(&"Compressor"),
            "missing Compressor: {:?}",
            names
        );
        assert!(names.contains(&"Alchemy"), "missing Alchemy: {:?}", names);
        assert!(names.contains(&"Hive"), "missing Hive: {:?}", names);
    }

    #[test]
    fn test_logic_filters_false_positives() {
        let mut data = vec![0u8; 50];
        for name in [
            "Output 1",
            "Output 5-6H",
            "Automatic-Generic Audio 12",
            "com.apple.foo",
        ] {
            data.extend_from_slice(name.as_bytes());
            data.extend_from_slice(&[0; 10]);
        }
        let result = extract_logic_plugin_names(&data);
        assert!(
            result.is_empty(),
            "should filter all false positives, got: {:?}",
            result.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_logic_stock_vs_thirdparty_type() {
        let mut data = vec![0u8; 50];
        data.extend_from_slice(b"Channel EQ");
        data.extend_from_slice(&[0; 10]);
        data.extend_from_slice(b"Sylenth1");
        data.extend_from_slice(&[0; 10]);
        let result = extract_logic_plugin_names(&data);
        let stock = result.iter().find(|p| p.name == "Channel EQ").unwrap();
        let third = result.iter().find(|p| p.name == "Sylenth1").unwrap();
        assert_eq!(stock.plugin_type, "AU (Stock)");
        assert_eq!(third.plugin_type, "AU");
    }

    #[test]
    fn test_logic_component_path_extraction() {
        let mut data = vec![0u8; 50];
        data.extend_from_slice(b"/Library/Audio/Plug-Ins/Components/FabFilter Pro-Q 3.component");
        data.extend_from_slice(&[0; 50]);
        let result = extract_plugins_from_binary(&data);
        assert!(!result.is_empty());
        assert_eq!(result[0].name, "FabFilter Pro-Q 3");
        assert_eq!(result[0].plugin_type, "AU");
    }

    // ── Studio One tests ──

    #[test]
    fn test_studio_one_zip_xml() {
        use std::io::Write;
        let tmp = std::env::temp_dir().join("test_xref_s1.song");
        let file = fs::File::create(&tmp).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file::<_, ()>("Song/song.xml", Default::default())
            .unwrap();
        zip.write_all(b"<Song><MediaTrack name=\"Bass\"/></Song>")
            .unwrap();
        zip.start_file::<_, ()>("Devices/audiomixer.xml", Default::default())
            .unwrap();
        zip.write_all(
            b"<AudioMixer><Insert plugName=\"Pro-Q 3\" deviceName=\"FabFilter\"/></AudioMixer>",
        )
        .unwrap();
        zip.finish().unwrap();
        let result = extract_plugins(tmp.to_str().unwrap());
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Pro-Q 3"), "missing Pro-Q 3: {:?}", names);
        assert!(
            !names.iter().any(|n| *n == "FabFilter"),
            "deviceName is vendor/model, not a second plugin: {:?}",
            names
        );
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_studio_one_ignores_generic_label_attributes() {
        use std::io::Write;
        let tmp = std::env::temp_dir().join("test_xref_s1_labels.song");
        let file = fs::File::create(&tmp).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file::<_, ()>("Song/song.xml", Default::default())
            .unwrap();
        zip.write_all(
            b"<Song><MediaTrack label=\"Lead Vocals\"/><Channel label=\"Main\"/>\
              <Insert plugName=\"True-Plugin\"/></Song>",
        )
        .unwrap();
        zip.finish().unwrap();
        let result = extract_plugins(tmp.to_str().unwrap());
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["True-Plugin"]);
        let _ = fs::remove_file(&tmp);
    }

    // ── DAWproject tests ──

    #[test]
    fn test_dawproject_zip_xml() {
        use std::io::Write;
        let tmp = std::env::temp_dir().join("test_xref.dawproject");
        let file = fs::File::create(&tmp).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file::<_, ()>("project.xml", Default::default())
            .unwrap();
        zip.write_all(b"<Project><Plugin name=\"Serum\" deviceName=\"Xfer Records\"/><Plugin name=\"Diva\" deviceName=\"u-he\"/></Project>").unwrap();
        zip.finish().unwrap();
        let result = extract_plugins(tmp.to_str().unwrap());
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Serum"), "missing Serum: {:?}", names);
        assert!(names.contains(&"Diva"), "missing Diva: {:?}", names);
        let _ = fs::remove_file(&tmp);
    }

    // ── Pro Tools tests ──

    #[test]
    fn test_protools_binary_extraction() {
        // PTX with embedded .aaxplugin paths
        let mut data = vec![0u8; 100];
        data.extend_from_slice(
            b"/Library/Application Support/Avid/Audio/Plug-Ins/EQ III.aaxplugin",
        );
        data.extend_from_slice(&[0; 50]);
        let result = extract_plugins_from_binary(&data);
        assert!(!result.is_empty(), "should find AAX plugin");
        assert_eq!(result[0].name, "EQ III");
        assert_eq!(result[0].plugin_type, "AAX");
    }

    #[test]
    fn test_protools_named_markers() {
        let mut data = vec![0u8; 50];
        data.extend_from_slice(b"PlugIn Name");
        data.push(0);
        data.extend_from_slice(b"Channel Strip");
        data.extend_from_slice(&[0; 30]);
        data.extend_from_slice(b"Insert Name");
        data.push(0);
        data.extend_from_slice(b"D-Verb");
        data.extend_from_slice(&[0; 30]);
        let result = extract_named_plugins(&data, b"PlugIn Name");
        let result2 = extract_named_plugins(&data, b"Insert Name");
        assert!(
            !result.is_empty() || !result2.is_empty(),
            "should find named plugins"
        );
    }

    // ── Reason tests ──

    #[test]
    fn test_reason_binary_extraction() {
        let mut data = vec![0u8; 100];
        data.extend_from_slice(b"C:\\VST\\Massive.dll");
        data.extend_from_slice(&[0; 50]);
        data.extend_from_slice(b"/Library/Audio/Plug-Ins/VST3/Serum.vst3");
        data.extend_from_slice(&[0; 50]);
        let tmp = std::env::temp_dir().join("test_xref.reason");
        fs::write(&tmp, &data).unwrap();
        let result = extract_plugins(tmp.to_str().unwrap());
        let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Massive"), "missing Massive: {:?}", names);
        assert!(names.contains(&"Serum"), "missing Serum: {:?}", names);
        let _ = fs::remove_file(&tmp);
    }

    // ── Bitwig tests ──

    #[test]
    fn test_bitwig_all_plugin_types() {
        let tmp = std::env::temp_dir().join("test_xref_bw_types.bwproject");
        let mut data = vec![0u8; 100];
        data.extend_from_slice(b"/VST3/Serum.vst3");
        data.extend_from_slice(&[0; 20]);
        data.extend_from_slice(b"C:\\Plugins\\Massive.dll");
        data.extend_from_slice(&[0; 20]);
        data.extend_from_slice(b"/Components/Kontakt.component");
        data.extend_from_slice(&[0; 20]);
        data.extend_from_slice(b"/CLAP/Vital.clap");
        data.extend_from_slice(&[0; 20]);
        fs::write(&tmp, &data).unwrap();
        let result = extract_plugins(tmp.to_str().unwrap());
        assert!(
            result.len() >= 4,
            "should find 4+ plugins, got {}",
            result.len()
        );
        let _ = fs::remove_file(&tmp);
    }

    // ── Cross-format dedup test ──

    #[test]
    fn test_dedup_across_extraction_methods() {
        // Same plugin found via both binary and UTF-16LE should dedup
        let mut data = vec![0u8; 50];
        data.extend_from_slice(b"C:\\Plugins\\Serum.dll");
        data.extend_from_slice(&[0; 30]);
        for c in "C:\\Plugins\\Serum.dll".chars() {
            data.push(c as u8);
            data.push(0);
        }
        data.extend_from_slice(&[0; 50]);
        let tmp = std::env::temp_dir().join("test_xref_dedup.flp");
        fs::write(&tmp, &data).unwrap();
        let result = extract_plugins(tmp.to_str().unwrap());
        let serum_count = result
            .iter()
            .filter(|p| p.normalized_name == "serum")
            .count();
        assert_eq!(
            serum_count, 1,
            "duplicate Serum should be deduped, got {}",
            serum_count
        );
        let _ = fs::remove_file(&tmp);
    }

    /// `.rpp-bak` must route through the REAPER parser (compound extension).
    #[test]
    fn test_extract_plugins_reaper_backup_extension() {
        let tmp = std::env::temp_dir().join("test_xref_reaper_bak.rpp-bak");
        let content = r#"<VST "VST3: Serum (Xfer)" C:\VST3\Serum.vst3"#;
        fs::write(&tmp, content).unwrap();
        let result = extract_plugins(tmp.to_str().unwrap());
        assert!(
            result.iter().any(|p| p.name == "Serum"),
            "expected Serum from REAPER backup path, got {:?}",
            result.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_extract_plugins_no_extension_returns_empty() {
        assert!(extract_plugins("/tmp/no-extension").is_empty());
    }

    #[test]
    fn test_normalize_plugin_name_idempotent_on_clean_name() {
        let once = normalize_plugin_name("FabFilter Pro-Q 3");
        let twice = normalize_plugin_name(&once);
        assert_eq!(once, twice);
    }

    // ── Real file tests (ignored, run manually) ──

    #[test]
    #[ignore]
    fn test_real_flp() {
        let path = "/Users/wizard/mnt/production/MusicProduction/Samples/Producer loops/2021/prototypesamples_RAGE - PROJECT/RAGE PROJECT/_RAGE.flp";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let result = extract_plugins(path);
        println!("FLP: {} plugins", result.len());
        for p in &result {
            println!("  {} ({})", p.name, p.plugin_type);
        }
        assert!(result.len() >= 5, "Real FLP should have 5+ plugins");
    }

    #[test]
    #[ignore]
    fn test_real_cubase() {
        let path = "/Users/wizard/mnt/production/MusicProduction/Samples/Producer loops/2021/OST Audio - Trance Collection/Collection/Powerful Trance For Spire/Templates/Cubase/0_1 By OST_Audio/0_1 By OST_Audio.cpr";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let result = extract_plugins(path);
        println!("Cubase: {} plugins", result.len());
        for p in &result {
            println!("  {} ({})", p.name, p.plugin_type);
        }
        assert!(result.len() >= 2, "Real Cubase should have 2+ plugins");
    }

    #[test]
    #[ignore]
    fn test_real_logic() {
        let path = "/Users/wizard/mnt/production/MusicProduction/Samples/mettaglyde/Alex Di Stefano Logic Pro Tech-Trance Template Vol One/Alex Di Stefano Logic Pro Tech-Trance Template Vol One.logicx";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let result = extract_plugins(path);
        println!("Logic: {} plugins", result.len());
        for p in &result {
            println!("  {} ({})", p.name, p.plugin_type);
        }
        assert!(result.len() >= 5, "Real Logic should have 5+ plugins");
    }
}
