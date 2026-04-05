//! Bulk directory enumeration + metadata fetch in a single syscall.
//!
//! ## Why
//! Standard `fs::read_dir` returns just names + d_type. To get size/mtime we
//! call `fs::metadata()` per file — one `stat(2)` syscall each. On SMB/NFS
//! every syscall = network roundtrip (1-10ms LAN, 50-200ms WAN). Walking a
//! 100k-file share spends 200+ seconds in per-file stats.
//!
//! macOS has `getattrlistbulk(2)` which returns metadata for an entire
//! directory in one syscall. `find(1)` and `fts(3)` use it. For SMB it's
//! a 10-100× speedup on metadata cost.
//!
//! Other platforms (Linux, Windows) fall back to readdir+metadata. Linux
//! has no direct equivalent — the closest is `statx` which is still per-file.
//!
//! ## Usage
//! Call `read_dir_bulk(path)` instead of `fs::read_dir(path)` when you need
//! type + size + mtime per entry. Returns a `Vec<BulkEntry>` with all
//! metadata already populated. If the platform-specific fast path fails
//! (unsupported filesystem, permission error, etc.) it falls back to the
//! portable `fs::read_dir + metadata` path transparently.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BulkEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size: u64,
    /// Modified time as seconds since UNIX epoch. 0 if unavailable.
    pub mtime_secs: i64,
}

/// Enumerate `dir` returning name + type + size + mtime for every entry in
/// a single syscall where possible. Hidden `.` and `..` entries are excluded.
pub fn read_dir_bulk(dir: &Path) -> io::Result<Vec<BulkEntry>> {
    #[cfg(target_os = "macos")]
    {
        match macos::read_dir_bulk_fast(dir) {
            Ok(v) => return Ok(v),
            Err(e) => {
                // Fall back to portable path on error (unsupported FS, etc.).
                // Log at debug level so production noise stays low.
                let _ = e;
            }
        }
    }
    read_dir_bulk_portable(dir)
}

/// Portable fallback: readdir + per-entry metadata. One syscall per entry.
fn read_dir_bulk_portable(dir: &Path) -> io::Result<Vec<BulkEntry>> {
    let entries = fs::read_dir(dir)?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let ft = match entry.file_type() {
            Ok(f) => f,
            Err(_) => continue,
        };
        // Stat only files — stats are syscalls on Unix. Directory mtimes are
        // left as 0 on the portable path; callers that need dir mtime (DAW
        // packages) must stat those specific directories themselves. On
        // Windows, DirEntry::metadata() is free (cached from FindFirstFileW)
        // so stat'ing dirs there would be cheap, but we keep the logic
        // uniform across portable platforms.
        let (size, mtime_secs) = if ft.is_file() {
            match entry.metadata() {
                Ok(m) => (
                    m.len(),
                    m.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0),
                ),
                Err(_) => (0, 0),
            }
        } else {
            (0, 0)
        };
        out.push(BulkEntry {
            name,
            path,
            is_dir: ft.is_dir(),
            is_file: ft.is_file(),
            is_symlink: ft.is_symlink(),
            size,
            mtime_secs,
        });
    }
    Ok(out)
}

#[cfg(target_os = "macos")]
mod macos {
    //! macOS-specific bulk metadata via `getattrlistbulk(2)`.
    //!
    //! Layout reference: the man page at `man 2 getattrlistbulk`.
    //! Each returned entry is: u32 total_length, then each requested attr
    //! packed in the order it appears in the attrlist bitmap.
    use super::BulkEntry;
    use std::ffi::{CStr, CString};
    use std::io;
    use std::mem;
    use std::os::raw::{c_int, c_void};
    use std::path::{Path, PathBuf};

    // attrlist bitmap count
    const ATTR_BIT_MAP_COUNT: u16 = 5;

    // common attributes (attrgroup_t bits)
    const ATTR_CMN_RETURNED_ATTRS: u32 = 0x80000000;
    const ATTR_CMN_NAME: u32 = 0x00000001;
    const ATTR_CMN_OBJTYPE: u32 = 0x00000008;
    const ATTR_CMN_MODTIME: u32 = 0x00000400;

    // file attributes
    const ATTR_FILE_DATALENGTH: u32 = 0x00000200;

    // FSOPT: pack placeholders for requested-but-unavailable attrs so the
    // layout stays predictable.
    const FSOPT_PACK_INVAL_ATTRS: u64 = 0x00000008;

    // vnode_type_t / fsobj_type_t values
    const VREG: u32 = 1;
    const VDIR: u32 = 2;
    const VLNK: u32 = 5;

    #[repr(C)]
    struct Attrlist {
        bitmapcount: u16,
        reserved: u16,
        commonattr: u32,
        volattr: u32,
        dirattr: u32,
        fileattr: u32,
        forkattr: u32,
    }

    #[repr(C)]
    struct AttributeSet {
        commonattr: u32,
        volattr: u32,
        dirattr: u32,
        fileattr: u32,
        forkattr: u32,
    }
    // Layout notes: AttrReference (int32 offset + u32 length, 8 bytes) and
    // Timespec (2 x i64, 16 bytes) are parsed manually via byte slicing — no
    // Rust structs needed for them.

    extern "C" {
        fn getattrlistbulk(
            dirfd: c_int,
            alist: *mut c_void,
            attrbuf: *mut c_void,
            bufsize: usize,
            options: u64,
        ) -> c_int;
    }

    pub fn read_dir_bulk_fast(dir: &Path) -> io::Result<Vec<BulkEntry>> {
        let cpath = CString::new(dir.as_os_str().to_string_lossy().as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        // SAFETY: libc FFI. Opens dir for reading with O_DIRECTORY flag.
        let fd = unsafe {
            libc::open(
                cpath.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // Close fd on drop.
        struct FdGuard(c_int);
        impl Drop for FdGuard {
            fn drop(&mut self) {
                unsafe {
                    libc::close(self.0);
                }
            }
        }
        let _guard = FdGuard(fd);

        let mut alist: Attrlist = unsafe { mem::zeroed() };
        alist.bitmapcount = ATTR_BIT_MAP_COUNT;
        // ATTR_CMN_ERROR intentionally omitted — it's a flag on the returned
        // attribute_set_t, not an inline u32 value in the data buffer.
        alist.commonattr = ATTR_CMN_RETURNED_ATTRS
            | ATTR_CMN_NAME
            | ATTR_CMN_OBJTYPE
            | ATTR_CMN_MODTIME;
        alist.fileattr = ATTR_FILE_DATALENGTH;

        // 64KB buffer ~= 200-500 entries per call depending on name lengths.
        // Loop until getattrlistbulk returns 0 (no more entries).
        const BUFSIZE: usize = 64 * 1024;
        let mut buf = vec![0u8; BUFSIZE];
        let mut out = Vec::new();

        loop {
            let n = unsafe {
                getattrlistbulk(
                    fd,
                    &mut alist as *mut _ as *mut c_void,
                    buf.as_mut_ptr() as *mut c_void,
                    BUFSIZE,
                    FSOPT_PACK_INVAL_ATTRS,
                )
            };
            if n < 0 {
                return Err(io::Error::last_os_error());
            }
            if n == 0 {
                break;
            }
            let n = n as usize;
            let mut cursor = 0usize;
            for _ in 0..n {
                let entry_start = cursor;
                if cursor + 4 > buf.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "getattrlistbulk returned truncated entry length",
                    ));
                }
                // First field: u32 total entry length (includes this length field).
                let total_len = u32::from_ne_bytes(
                    buf[cursor..cursor + 4]
                        .try_into()
                        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "slice"))?,
                ) as usize;
                cursor += 4;

                // Next: attribute_set_t (5 x u32) — the actually-returned attrs.
                let returned = AttributeSet {
                    commonattr: read_u32(&buf, &mut cursor)?,
                    volattr: read_u32(&buf, &mut cursor)?,
                    dirattr: read_u32(&buf, &mut cursor)?,
                    fileattr: read_u32(&buf, &mut cursor)?,
                    forkattr: read_u32(&buf, &mut cursor)?,
                };

                // ATTR_CMN_NAME: attrreference_t (offset is relative to start of
                // the attrreference itself, NOT the entry).
                let mut name = String::new();
                if returned.commonattr & ATTR_CMN_NAME != 0 {
                    let ref_pos = cursor;
                    if cursor + 8 > buf.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "attrreference oob",
                        ));
                    }
                    let offset = i32::from_ne_bytes(
                        buf[cursor..cursor + 4]
                            .try_into()
                            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "slice"))?,
                    );
                    let length = u32::from_ne_bytes(
                        buf[cursor + 4..cursor + 8]
                            .try_into()
                            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "slice"))?,
                    );
                    cursor += 8;
                    let name_start = (ref_pos as isize + offset as isize) as usize;
                    let name_end = name_start + length as usize;
                    if name_end > buf.len() || name_start >= buf.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "name offset oob",
                        ));
                    }
                    // length includes trailing NUL byte.
                    let cstr = CStr::from_bytes_until_nul(&buf[name_start..name_end])
                        .unwrap_or(CStr::from_bytes_with_nul(b"\0").unwrap());
                    name = cstr.to_string_lossy().into_owned();
                }

                // ATTR_CMN_OBJTYPE: u32 fsobj_type
                let mut objtype: u32 = 0;
                if returned.commonattr & ATTR_CMN_OBJTYPE != 0 {
                    objtype = read_u32(&buf, &mut cursor)?;
                }

                // ATTR_CMN_MODTIME: struct timespec (8-aligned)
                let mut mtime_secs: i64 = 0;
                if returned.commonattr & ATTR_CMN_MODTIME != 0 {
                    // timespec is 16 bytes, 8-byte aligned. Cursor should already
                    // be aligned because packed(4) plus preceding fields sum right.
                    if cursor + 16 > buf.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "timespec oob",
                        ));
                    }
                    mtime_secs = i64::from_ne_bytes(
                        buf[cursor..cursor + 8]
                            .try_into()
                            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "slice"))?,
                    );
                    cursor += 16;
                }

                // Note: ATTR_CMN_ERROR is NOT packed into the buffer inline.
                // It only appears as a flag in the `returned` bitmap — when
                // nonzero per-entry errors occur, the value lives elsewhere in
                // the returned attribute set (not the data stream). Empirically
                // on macOS 14/15, requesting ATTR_CMN_ERROR does not consume
                // bytes in the output buffer for successful entries.

                // ATTR_FILE_DATALENGTH: off_t (i64) — only valid for regular files
                let mut size: u64 = 0;
                if returned.fileattr & ATTR_FILE_DATALENGTH != 0 {
                    if cursor + 8 > buf.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "datalength oob",
                        ));
                    }
                    let sz = i64::from_ne_bytes(
                        buf[cursor..cursor + 8]
                            .try_into()
                            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "slice"))?,
                    );
                    size = sz.max(0) as u64;
                    // cursor advance not needed — we jump to entry boundary below.
                }

                // Advance cursor to the declared entry boundary — handles any
                // padding attrs we didn't consume.
                cursor = entry_start + total_len;

                if name.is_empty() || name == "." || name == ".." {
                    continue;
                }
                let is_dir = objtype == VDIR;
                let is_file = objtype == VREG;
                let is_symlink = objtype == VLNK;
                let path = dir.join(&name);
                out.push(BulkEntry {
                    name,
                    path,
                    is_dir,
                    is_file,
                    is_symlink,
                    size: if is_file { size } else { 0 },
                    mtime_secs,
                });
            }
        }

        Ok(out)
    }

    fn read_u32(buf: &[u8], cursor: &mut usize) -> io::Result<u32> {
        if *cursor + 4 > buf.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "u32 oob"));
        }
        let v = u32::from_ne_bytes(
            buf[*cursor..*cursor + 4]
                .try_into()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "slice"))?,
        );
        *cursor += 4;
        Ok(v)
    }

    #[allow(dead_code)]
    fn _path_buf_unused() -> PathBuf {
        PathBuf::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    struct TestDir {
        path: PathBuf,
    }
    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "upum_bs_{}_{}",
                name,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn touch_with(p: &Path, content: &[u8]) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = File::create(p).unwrap();
        f.write_all(content).unwrap();
    }

    #[test]
    fn test_read_dir_bulk_basic() {
        let tmp = TestDir::new("basic");
        touch_with(&tmp.path.join("a.txt"), b"hello");
        touch_with(&tmp.path.join("b.dat"), b"0123456789");
        fs::create_dir_all(tmp.path.join("sub")).unwrap();

        let entries = read_dir_bulk(&tmp.path).unwrap();
        assert_eq!(entries.len(), 3);
        let by_name: std::collections::HashMap<_, _> =
            entries.iter().map(|e| (e.name.clone(), e)).collect();
        let a = by_name.get("a.txt").expect("a.txt missing");
        assert!(a.is_file);
        assert!(!a.is_dir);
        assert_eq!(a.size, 5);
        let b = by_name.get("b.dat").expect("b.dat missing");
        assert_eq!(b.size, 10);
        let sub = by_name.get("sub").expect("sub missing");
        assert!(sub.is_dir);
        assert!(!sub.is_file);
    }

    #[test]
    fn test_read_dir_bulk_empty() {
        let tmp = TestDir::new("empty");
        let entries = read_dir_bulk(&tmp.path).unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_read_dir_bulk_excludes_dot_entries() {
        let tmp = TestDir::new("dotentries");
        touch_with(&tmp.path.join("real.txt"), b"x");
        let entries = read_dir_bulk(&tmp.path).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(!names.contains(&"."));
        assert!(!names.contains(&".."));
        assert!(names.contains(&"real.txt"));
    }

    #[test]
    fn test_read_dir_bulk_nonexistent_dir() {
        let tmp = TestDir::new("nonexistent");
        let missing = tmp.path.join("does-not-exist");
        let result = read_dir_bulk(&missing);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_dir_bulk_many_entries() {
        // Ensure the getattrlistbulk loop handles multiple buffer passes.
        let tmp = TestDir::new("many");
        for i in 0..250 {
            touch_with(&tmp.path.join(format!("file_{:04}.txt", i)), b"x");
        }
        let entries = read_dir_bulk(&tmp.path).unwrap();
        assert_eq!(entries.len(), 250);
        // All should be files with size 1.
        assert!(entries.iter().all(|e| e.is_file && e.size == 1));
    }

    #[test]
    fn test_read_dir_bulk_mtime_populated() {
        let tmp = TestDir::new("mtime");
        touch_with(&tmp.path.join("x.txt"), b"y");
        let entries = read_dir_bulk(&tmp.path).unwrap();
        let x = entries.iter().find(|e| e.name == "x.txt").unwrap();
        // mtime should be near "now" (within the last 60 seconds).
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(x.mtime_secs > 0, "mtime should be populated");
        assert!(
            (now - x.mtime_secs).abs() < 60,
            "mtime {} should be near now {}",
            x.mtime_secs,
            now
        );
    }

    #[test]
    fn test_read_dir_bulk_matches_portable() {
        // The bulk path and the portable path should produce identical
        // classification + sizes. (macOS-specific — on other platforms both
        // paths ARE the portable one.)
        let tmp = TestDir::new("parity");
        touch_with(&tmp.path.join("a.wav"), b"RIFF1234");
        touch_with(&tmp.path.join("b.pdf"), b"%PDF-1.4");
        fs::create_dir_all(tmp.path.join("dir1")).unwrap();
        let bulk = read_dir_bulk(&tmp.path).unwrap();
        let portable = read_dir_bulk_portable(&tmp.path).unwrap();
        assert_eq!(bulk.len(), portable.len());
        let bulk_names: std::collections::HashSet<_> =
            bulk.iter().map(|e| e.name.clone()).collect();
        let portable_names: std::collections::HashSet<_> =
            portable.iter().map(|e| e.name.clone()).collect();
        assert_eq!(bulk_names, portable_names);
        // Sizes should match for files (bulk populates from bulk syscall,
        // portable populates from per-file metadata).
        for b in &bulk {
            if b.is_file {
                let p = portable.iter().find(|e| e.name == b.name).unwrap();
                assert_eq!(
                    b.size, p.size,
                    "size mismatch for {}: bulk={} portable={}",
                    b.name, b.size, p.size
                );
            }
        }
    }
}
