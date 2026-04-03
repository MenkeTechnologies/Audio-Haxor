//! `format_size` — explicit (bytes, label) pairs (labels match `app_lib::format_size` / IEC 1024^n tiers).

#[test]
fn handcrafted_format_size_zero() {
    assert_eq!(app_lib::format_size(0), "0 B");
}

macro_rules! format_size_cases {
    ($($name:ident: $bytes:expr => $want:literal)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(app_lib::format_size($bytes), $want, "bytes={}", $bytes);
            }
        )*
    };
}

format_size_cases! {
    fmt_b1: 1 => "1.0 B"
    fmt_b2: 2 => "2.0 B"
    fmt_b10: 10 => "10.0 B"
    fmt_b100: 100 => "100.0 B"
    fmt_b512: 512 => "512.0 B"
    fmt_b1023: 1023 => "1023.0 B"
    fmt_kb1: 1024 => "1.0 KB"
    fmt_kb1025: 1025 => "1.0 KB"
    fmt_kb1536: 1536 => "1.5 KB"
    fmt_kb2047: 2047 => "2.0 KB"
    fmt_kb2048: 2048 => "2.0 KB"
    fmt_kb10k: 10240 => "10.0 KB"
    fmt_kb100k: 102400 => "100.0 KB"
    fmt_kb512k: 524288 => "512.0 KB"
    fmt_mb_almost: 1048575 => "1024.0 KB"
    fmt_mb1: 1048576 => "1.0 MB"
    fmt_mb1p: 1048577 => "1.0 MB"
    fmt_mb100: 104857600 => "100.0 MB"
    fmt_gb1: 1073741824 => "1.0 GB"
    fmt_gb2: 2147483648u64 => "2.0 GB"
    fmt_tb_almost: 1099511627775u64 => "1024.0 GB"
    fmt_tb1: 1099511627776u64 => "1.0 TB"
    fmt_tb10: 10995116277760u64 => "10.0 TB"
}
