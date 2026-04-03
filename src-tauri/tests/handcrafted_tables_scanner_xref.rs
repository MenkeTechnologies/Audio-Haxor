//! `scanner::get_plugin_type` and `xref::normalize_plugin_name` — explicit tables.

macro_rules! plugin_type_cases {
    ($($name:ident: $ext:expr => $want:literal)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(app_lib::scanner::get_plugin_type($ext), $want, "ext={}", $ext);
            }
        )*
    };
}

plugin_type_cases! {
    pt_vst: ".vst" => "VST2"
    pt_vst3: ".vst3" => "VST3"
    pt_au: ".component" => "AU"
    pt_dll: ".dll" => "VST2"
    pt_clap: ".clap" => "Unknown"
    pt_aax: ".aaxplugin" => "Unknown"
    pt_exe: ".exe" => "Unknown"
    pt_txt: ".txt" => "Unknown"
    pt_upper: ".VST3" => "Unknown"
}

macro_rules! normalize_cases {
    ($($name:ident: $s:expr => $want:literal)*) => {
        $(
            #[test]
            fn $name() {
                let got = app_lib::xref::normalize_plugin_name($s);
                assert_eq!(got, $want, "input={:?}", $s);
            }
        )*
    };
}

normalize_cases! {
    norm_empty: "" => ""
    norm_spaces: "   " => ""
    norm_simple: "FabFilter" => "fabfilter"
    norm_internal_space: "Pro Q 3" => "pro q 3"
    norm_x64_paren: "Plugin (x64)" => "plugin"
    norm_vst3_paren: "Serum (VST3)" => "serum"
    norm_double_paren: "Serum (x64) (VST3)" => "serum"
    norm_bracket_aax: "EQ [AAX]" => "eq"
    norm_unicode: "插件A" => "插件a"
}
