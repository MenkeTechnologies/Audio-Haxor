//! Pure-helper tests for `daw_scanner::is_valid_pro_tools_session_file`.
//!
//! The header-magic check is the gate the scanner uses before tagging a file
//! as a Pro Tools session. It opens the file, reads exactly the
//! `PRO_TOOLS_SESSION_MAGIC` bytes (17 bytes per src), and compares them.
//! These tests construct real on-disk fixtures (the only way to drive the
//! function — it takes `&Path` and opens via `fs::File`) and cover:
//!   - exact-magic accept
//!   - magic + trailing payload accept (only the prefix matters)
//!   - off-by-one truncated header reject
//!   - wrong byte at every position reject (mutation matrix)
//!   - empty file reject
//!   - non-existent path reject
//!
//! All fixtures live under `std::env::temp_dir()`/`ptools_test_<unique>/`
//! and are deleted in a `Drop` guard so failures don't leak.

use app_lib::daw_scanner::is_valid_pro_tools_session_file;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Per-source PRONOM fmt/1727 + LOC FDD fdd000639: shared BOF for `.ptx`/`.ptf`/`.pts`.
const PRO_TOOLS_SESSION_MAGIC: &[u8] = &[
    0x03, b'0', b'0', b'1', b'0', b'1', b'1', b'1', b'1', b'0', b'0', b'1', b'0', b'1', b'0', b'1',
    b'1',
];

/// Per-test fixture dir. Drop deletes it. Counter prevents collisions if
/// the test binary is rerun within the same second.
static FIXTURE_SEQ: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let n = FIXTURE_SEQ.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("ptools_test_{pid}_{n}_{label}"));
        fs::create_dir_all(&dir).expect("create fixture dir");
        Fixture { dir }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.dir.join(name);
        let mut f = fs::File::create(&p).expect("create fixture file");
        f.write_all(bytes).expect("write fixture");
        p
    }

    fn nonexistent(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn exact_magic_only_is_accepted() {
    let fx = Fixture::new("exact");
    let p = fx.write("Session.ptx", PRO_TOOLS_SESSION_MAGIC);
    assert!(is_valid_pro_tools_session_file(&p));
}

#[test]
fn magic_plus_trailing_payload_is_accepted() {
    let fx = Fixture::new("trailing");
    let mut bytes = PRO_TOOLS_SESSION_MAGIC.to_vec();
    bytes.extend_from_slice(b"...realistic payload follows for many KB...");
    let p = fx.write("Big.ptx", &bytes);
    assert!(is_valid_pro_tools_session_file(&p));
}

/// Same magic — `.ptf` is identical-header per src comment.
#[test]
fn magic_accepted_regardless_of_extension() {
    let fx = Fixture::new("ext_ptf");
    let p = fx.write("Old.ptf", PRO_TOOLS_SESSION_MAGIC);
    // The function checks bytes, not extension — extension filtering is the
    // caller's responsibility. Document that contract.
    assert!(is_valid_pro_tools_session_file(&p));
}

#[test]
fn truncated_header_one_byte_short_is_rejected() {
    let fx = Fixture::new("truncated");
    let short = &PRO_TOOLS_SESSION_MAGIC[..PRO_TOOLS_SESSION_MAGIC.len() - 1];
    let p = fx.write("Short.ptx", short);
    assert!(!is_valid_pro_tools_session_file(&p));
}

#[test]
fn empty_file_is_rejected() {
    let fx = Fixture::new("empty");
    let p = fx.write("Empty.ptx", b"");
    assert!(!is_valid_pro_tools_session_file(&p));
}

#[test]
fn nonexistent_path_is_rejected() {
    let fx = Fixture::new("noent");
    let p = fx.nonexistent("DoesNotExist.ptx");
    assert!(!is_valid_pro_tools_session_file(&p));
}

#[test]
fn wrong_first_byte_is_rejected() {
    let fx = Fixture::new("badfirst");
    let mut bytes = PRO_TOOLS_SESSION_MAGIC.to_vec();
    bytes[0] = 0xFF; // any value ≠ 0x03
    let p = fx.write("Mut.ptx", &bytes);
    assert!(!is_valid_pro_tools_session_file(&p));
}

#[test]
fn wrong_last_byte_is_rejected() {
    let fx = Fixture::new("badlast");
    let mut bytes = PRO_TOOLS_SESSION_MAGIC.to_vec();
    let last = bytes.len() - 1;
    bytes[last] = b'X'; // anything ≠ b'1'
    let p = fx.write("Mut.ptx", &bytes);
    assert!(!is_valid_pro_tools_session_file(&p));
}

/// Mutation matrix: flipping byte at any internal position breaks the check.
/// Documents that the comparison is byte-equality, not prefix-of or substring.
#[test]
fn every_internal_byte_position_is_load_bearing() {
    let fx = Fixture::new("mutmatrix");
    for i in 1..(PRO_TOOLS_SESSION_MAGIC.len() - 1) {
        let mut bytes = PRO_TOOLS_SESSION_MAGIC.to_vec();
        // XOR with 0xFF guarantees any-bit change at position i.
        bytes[i] ^= 0xFF;
        let p = fx.write(&format!("Mut{i}.ptx"), &bytes);
        assert!(
            !is_valid_pro_tools_session_file(&p),
            "mutating byte {i} should invalidate the magic, got accept"
        );
    }
}

/// A file containing only a near-match prefix and unrelated bytes after must
/// still be rejected — there is no scan, just a fixed-window equality check.
#[test]
fn near_miss_header_with_unrelated_continuation_is_rejected() {
    let fx = Fixture::new("nearmiss");
    let mut bytes = PRO_TOOLS_SESSION_MAGIC[..8].to_vec();
    bytes.extend_from_slice(b"GARBAGEDATA12345678");
    let p = fx.write("Near.ptx", &bytes);
    assert!(!is_valid_pro_tools_session_file(&p));
}
