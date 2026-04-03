//! `kvr::extract_version` — HTML snippets with expected semver captures (or `None` when filtered).

macro_rules! extract_version_cases {
    ($($name:ident: $html:expr => $want:expr)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(
                    app_lib::kvr::extract_version($html).as_deref(),
                    $want,
                    "html snippet mismatch"
                );
            }
        )*
    };
}

extract_version_cases! {
    ex_v_001: r#"<div>Version: 3.5.2</div>"# => Some("3.5.2")
    ex_v_002: r#"<dt>Latest Version</dt><dd>2.1.0</dd>"# => Some("2.1.0")
    ex_v_003: r#"{"softwareVersion": "1.4.7"}"# => Some("1.4.7")
    ex_v_004: r#"<span>Version: 1.2.3.4</span>"# => Some("1.2.3.4")
    ex_v_005: "current version v2.3.1" => Some("2.3.1")
    ex_v_006: "latest release 4.0.2 available" => Some("4.0.2")
    ex_v_007: r#"<span>Version</span><dd>11.22.33</dd>"# => Some("11.22.33")
    ex_v_008: r#"Release notes: latest v2.4.0 is now available"# => Some("2.4.0")
    ex_v_009: r#"<div>Version 8.9.10 build</div>"# => Some("8.9.10")
    ex_v_010: r#"Version: 0.0.1-alpha"# => Some("0.0.1")
    ex_v_011: r#"<label>Version</label><span>12.0</span>"# => Some("12.0")
    ex_v_012: r#"softwareVersion">9.8.7</span>"# => Some("9.8.7")
    ex_v_013: r#"Version</th><td>55.66.77</td>"# => Some("55.66.77")
    ex_v_014: r#"<div>Version: 3.3.3</div>"# => Some("3.3.3")
    ex_v_015: r#"version v100.200.300 here"# => Some("100.200.300")
    ex_v_016: r#"VERSION 1.11.111 released"# => Some("1.11.111")
    ex_v_017: r#"<p>Version: 7.7.7.7</p>"# => Some("7.7.7.7")
    ex_v_018: r#"release 0.1.2 stable"# => Some("0.1.2")
    ex_v_019: r#"Version&nbsp;15.3.9"# => Some("15.3.9")
    ex_v_020: r#"release 2.0.0-final"# => Some("2.0.0")
    ex_v_021: r#"softwareVersion: 4.5.6"# => Some("4.5.6")
    ex_v_022: r#"Version 10.1 (build)"# => Some("10.1")
    ex_v_023: r#"latest v88.99.0"# => Some("88.99.0")
    ex_v_024: r#"SoftwareVersion: 1.0.0"# => Some("1.0.0")
    ex_v_025: r#"currentVersion">33.44.55<"# => Some("33.44.55")
}

extract_version_cases! {
    ex_n_001: r#"<div>Version: 2024.01.15</div>"# => None
    ex_n_002: r#"<div>No version info here</div>"# => None
    ex_n_003: r#""# => None
    ex_n_004: r#"<div>only text</div>"# => None
    ex_n_005: r#"<div>year 2025.12</div>"# => None
    ex_n_006: r#"2020.05.05 release"# => None
    ex_n_007: r#"date 2024.11.3"# => None
    ex_n_008: r#"Version TBD"# => None
    ex_n_009: r#"<html></html>"# => None
    ex_n_010: r#"NaN.NaN.NaN"# => None
}
