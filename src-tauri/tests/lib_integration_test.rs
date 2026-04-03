//! Integration tests for all modules

#[test]
fn test_lib_format_size() {
    assert_eq!(app_lib::format_size(100), "100.0 B");
    assert_eq!(app_lib::format_size(1024), "1.0 KB");
    assert_eq!(app_lib::format_size(1048576), "1.0 MB");
    assert_eq!(app_lib::format_size(0), "0 B");
}

#[test]
fn test_lib_format_size_bounds() {
    assert!(app_lib::format_size(1024 * 1024 * 1024).contains("1.")); // 1GB minimum
    assert!(app_lib::format_size(u64::MAX).contains("TB"));
}
