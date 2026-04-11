//! Integration smoke tests for `file_watcher::FileWatcherState` helpers.

use app_lib::file_watcher::{FileWatcherState, get_watched_dirs, is_watching, stop_watching};

#[test]
fn file_watcher_new_is_not_watching() {
    let state = FileWatcherState::new();
    assert!(!is_watching(&state));
    assert!(get_watched_dirs(&state).is_empty());
}

#[test]
fn stop_watching_on_fresh_state_is_safe() {
    let state = FileWatcherState::new();
    stop_watching(&state);
    assert!(!is_watching(&state));
    assert!(get_watched_dirs(&state).is_empty());
}

#[test]
fn stop_watching_is_idempotent() {
    let state = FileWatcherState::new();
    stop_watching(&state);
    stop_watching(&state);
    assert!(!is_watching(&state));
}
