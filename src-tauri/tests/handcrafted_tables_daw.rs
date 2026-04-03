//! DAW scanner pure helpers: `ext_matches`, `is_package_ext`, `daw_name_for_format`.

use std::path::Path;

macro_rules! ext_matches_case {
    ($($name:ident: $path:expr => $want:expr)*) => {
        $(
            #[test]
            fn $name() {
                let got = app_lib::daw_scanner::ext_matches(Path::new($path));
                assert_eq!(got.as_deref(), $want, "path={}", $path);
            }
        )*
    };
}

ext_matches_case! {
    ext_als: "/p/Song.als" => Some("ALS")
    ext_rpp: "/p/Proj.rpp" => Some("RPP")
    ext_rpp_bak: "/p/Proj.rpp-bak" => Some("RPP-BAK")
    ext_bw: "/p/B.bwproject" => Some("BWPROJECT")
    ext_song: "/p/S.song" => Some("SONG")
    ext_dawproject: "/p/X.dawproject" => Some("DAWPROJECT")
    ext_flp: "/p/F.flp" => Some("FLP")
    ext_logicx: "/p/L.logicx" => Some("LOGICX")
    ext_cpr: "/p/C.cpr" => Some("CPR")
    ext_ptx: "/p/P.ptx" => Some("PTX")
    ext_reason: "/p/R.reason" => Some("REASON")
    ext_none: "/p/x.txt" => None
}

macro_rules! package_ext_case {
    ($($name:ident: $path:expr => $want:expr)*) => {
        $(
            #[test]
            fn $name() {
                let got = app_lib::daw_scanner::is_package_ext(Path::new($path));
                assert_eq!(got, $want, "path={}", $path);
            }
        )*
    };
}

package_ext_case! {
    pkg_logicx: "/Music/P.logicx" => true
    pkg_band: "/Music/G.band" => true
    pkg_als: "/p/x.als" => false
}

macro_rules! daw_name_case {
    ($($name:ident: $fmt:expr => $want:literal)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(app_lib::daw_scanner::daw_name_for_format($fmt), $want);
            }
        )*
    };
}

daw_name_case! {
    dn_als: "ALS" => "Ableton Live"
    dn_rpp: "RPP" => "REAPER"
    dn_rpp_bak: "RPP-BAK" => "REAPER"
    dn_bw: "BWPROJECT" => "Bitwig Studio"
    dn_song: "SONG" => "Studio One"
    dn_dawproject: "DAWPROJECT" => "DAWproject"
    dn_flp: "FLP" => "FL Studio"
    dn_logic: "LOGICX" => "Logic Pro"
    dn_cpr: "CPR" => "Cubase"
    dn_npr: "NPR" => "Nuendo"
    dn_ptx: "PTX" => "Pro Tools"
    dn_ptf: "PTF" => "Pro Tools"
    dn_reason: "REASON" => "Reason"
    dn_aup: "AUP" => "Audacity"
    dn_band: "BAND" => "GarageBand"
    dn_ardour: "ARDOUR" => "Ardour"
    dn_bad: "NOPE" => "Unknown"
}
