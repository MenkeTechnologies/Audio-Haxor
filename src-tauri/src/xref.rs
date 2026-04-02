//! Cross-reference engine: extract plugin references from DAW project files.
//!
//! Parses Ableton Live (.als — gzip XML) and REAPER (.rpp — plaintext) project
//! files to discover which plugins each project uses. Returns deduplicated lists
//! of plugin names, manufacturers, and types.

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
    let result = s.split_whitespace()
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
    let vst2_re = Regex::new(r#"<VstPluginInfo[^>]*>[\s\S]*?</VstPluginInfo>"#).unwrap();
    let vst2_name_re = Regex::new(r#"<PlugName\s+Value="([^"]+)""#).unwrap();
    let vst2_mfg_re = Regex::new(r#"<Manufacturer\s+Value="([^"]+)""#).unwrap();

    for block in vst2_re.find_iter(&xml) {
        let text = block.as_str();
        let name = vst2_name_re
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let mfg = vst2_mfg_re
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
    let vst3_re = Regex::new(r#"<Vst3PluginInfo[^>]*>[\s\S]*?</Vst3PluginInfo>"#).unwrap();
    let vst3_name_re = Regex::new(r#"<Name\s+Value="([^"]+)""#).unwrap();
    let vst3_mfg_re = Regex::new(r#"<DeviceCreator\s+Value="([^"]+)""#).unwrap();

    for block in vst3_re.find_iter(&xml) {
        let text = block.as_str();
        let name = vst3_name_re
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let mfg = vst3_mfg_re
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
    let au_re = Regex::new(r#"<AuPluginInfo[^>]*>[\s\S]*?</AuPluginInfo>"#).unwrap();
    let au_name_re = Regex::new(r#"<Name\s+Value="([^"]+)""#).unwrap();
    let au_mfg_re = Regex::new(r#"<Manufacturer\s+Value="([^"]+)""#).unwrap();

    for block in au_re.find_iter(&xml) {
        let text = block.as_str();
        let name = au_name_re
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let mfg = au_mfg_re
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
    let re = Regex::new(r#"<(?:VST|AU|CLAP)\s+"(VST3?|AU|CLAP):\s*(.+?)\s*(?:\(([^)]+)\))?\s*""#)
        .unwrap();

    for cap in re.captures_iter(&text) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

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
}
