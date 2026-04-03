#[test]
fn test_history_radix_string_bases() {
    assert_eq!(app_lib::history::radix_string(255, 16), "ff");
    assert_eq!(app_lib::history::radix_string(10, 10), "10");
}
