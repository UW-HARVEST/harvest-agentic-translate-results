//! Shared harness: loads BOTH the C `libdriver.so` and the Rust `libdriver.so`
//! through `libloading` and calls every function purely through its exported
//! C symbol, exactly as an external consumer would.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long, c_uint, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// C ABI types (mirrors of the public headers)
// ---------------------------------------------------------------------------

pub const MAX_FQUEUE: usize = 256;

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

#[repr(C)]
pub struct alert_data {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: *mut c_char,
    pub date: *mut c_char,
    pub location: *mut c_char,
    pub comment: *mut c_char,
    pub group: *mut c_char,
    pub srcip: *mut c_char,
    pub srcport: c_int,
    pub dstip: *mut c_char,
    pub dstport: c_int,
    pub user: *mut c_char,
    pub filename: *mut c_char,
}

#[repr(C)]
pub struct file_queue {
    pub last_change: libc::time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut libc::FILE,
    pub f_status: libc::stat,
}

// ---------------------------------------------------------------------------
// Owned snapshots used for byte-for-byte comparison
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AlertSnap {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: Option<Vec<u8>>,
    pub date: Option<Vec<u8>>,
    pub location: Option<Vec<u8>>,
    pub comment: Option<Vec<u8>>,
    pub group: Option<Vec<u8>>,
    pub srcip: Option<Vec<u8>>,
    pub srcport: c_int,
    pub dstip: Option<Vec<u8>>,
    pub dstport: c_int,
    pub user: Option<Vec<u8>>,
    pub filename: Option<Vec<u8>>,
}

unsafe fn snap_str(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes().to_vec())
    }
}

pub unsafe fn snap_alert(p: *const alert_data) -> Option<AlertSnap> {
    if p.is_null() {
        return None;
    }
    let a = &*p;
    Some(AlertSnap {
        rule: a.rule,
        level: a.level,
        alertid: snap_str(a.alertid),
        date: snap_str(a.date),
        location: snap_str(a.location),
        comment: snap_str(a.comment),
        group: snap_str(a.group),
        srcip: snap_str(a.srcip),
        srcport: a.srcport,
        dstip: snap_str(a.dstip),
        dstport: a.dstport,
        user: snap_str(a.user),
        filename: snap_str(a.filename),
    })
}

/// Everything observable about a `file_queue` after a call. `fp` is reduced to
/// a null/non-null flag (the pointer value itself is never equal), and the
/// access time of `f_status` is skipped because reading the file mutates it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct QueueSnap {
    pub last_change: i64,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [u8; 4],
    pub file_name: Vec<u8>,
    pub fp_is_null: bool,
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u64,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_size: i64,
    pub st_mtime: i64,
    pub st_ctime: i64,
}

pub unsafe fn snap_queue(q: *const file_queue) -> QueueSnap {
    let q = &*q;
    let mut mon = [0u8; 4];
    for i in 0..4 {
        mon[i] = q.mon[i] as u8;
    }
    QueueSnap {
        last_change: q.last_change as i64,
        year: q.year,
        day: q.day,
        flags: q.flags,
        mon,
        file_name: q.file_name.iter().map(|c| *c as u8).collect(),
        fp_is_null: q.fp.is_null(),
        st_dev: q.f_status.st_dev as u64,
        st_ino: q.f_status.st_ino as u64,
        st_mode: q.f_status.st_mode as u32,
        st_nlink: q.f_status.st_nlink as u64,
        st_uid: q.f_status.st_uid as u32,
        st_gid: q.f_status.st_gid as u32,
        st_size: q.f_status.st_size as i64,
        st_mtime: q.f_status.st_mtime as i64,
        st_ctime: q.f_status.st_ctime as i64,
    }
}

/// Observable stream state after a parse call.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StreamSnap {
    pub pos: c_long,
    pub eof: bool,
    pub err: bool,
}

pub unsafe fn snap_stream(fp: *mut libc::FILE) -> StreamSnap {
    StreamSnap {
        pos: libc::ftell(fp),
        eof: libc::feof(fp) != 0,
        err: libc::ferror(fp) != 0,
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type FnGetAlertData = unsafe extern "C" fn(c_int, *mut libc::FILE) -> *mut alert_data;
type FnFreeAlertData = unsafe extern "C" fn(*mut alert_data);
type FnInitFileQueue = unsafe extern "C" fn(*mut file_queue, *const libc::tm, c_int) -> c_int;
type FnReadFileMon =
    unsafe extern "C" fn(*mut file_queue, *const libc::tm, c_uint) -> *mut alert_data;
type FnDriver = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut alert_data;
type FnMerror = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);
type FnOsCalloc = unsafe extern "C" fn(usize, usize) -> *mut c_void;
type FnOsRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnOsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;

/// One loaded implementation. Every entry point is resolved by its exported
/// symbol name, so the `#[no_mangle]` wrappers are what actually gets tested.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub GetAlertData: FnGetAlertData,
    pub FreeAlertData: FnFreeAlertData,
    pub Init_FileQueue: FnInitFileQueue,
    pub Read_FileMon: FnReadFileMon,
    pub driver: FnDriver,
    pub merror: FnMerror,
    pub os_calloc: FnOsCalloc,
    pub os_realloc: FnOsRealloc,
    pub os_strdup: FnOsStrdup,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to load {} ({}): {e}", name, path.display()));
        macro_rules! sym {
            ($t:ty, $n:expr) => {{
                let s: Symbol<$t> = lib
                    .get($n)
                    .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {e}", name, $n));
                *s
            }};
        }
        let me = Impl {
            name,
            GetAlertData: sym!(FnGetAlertData, b"GetAlertData\0"),
            FreeAlertData: sym!(FnFreeAlertData, b"FreeAlertData\0"),
            Init_FileQueue: sym!(FnInitFileQueue, b"Init_FileQueue\0"),
            Read_FileMon: sym!(FnReadFileMon, b"Read_FileMon\0"),
            driver: sym!(FnDriver, b"driver\0"),
            merror: sym!(FnMerror, b"merror\0"),
            os_calloc: sym!(FnOsCalloc, b"os_calloc\0"),
            os_realloc: sym!(FnOsRealloc, b"os_realloc\0"),
            os_strdup: sym!(FnOsStrdup, b"os_strdup\0"),
            _lib: lib,
        };
        me
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built: {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Paths of the two shared objects under comparison (C first, Rust second).
/// Both are guaranteed to exist once this returns.
pub fn so_paths() -> (PathBuf, PathBuf) {
    pair(); // force the nested cdylib build
    (c_so_path(), rust_so_path())
}

/// Builds (or rebuilds) the `cdylib` and returns its path.
///
/// `cargo test` does not build the `cdylib` artifact of the crate under test, so
/// the harness drives a nested `cargo build --lib` into a dedicated target
/// directory. Using a separate `CARGO_TARGET_DIR` avoids any lock contention
/// with the outer `cargo test` invocation, and rebuilding unconditionally means
/// the loaded `.so` always matches the current sources.
fn rust_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = manifest.join("target").join("test-cdylib");
    let profile_dir = if cfg!(debug_assertions) { "debug" } else { "release" };

    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.current_dir(&manifest)
        .arg("build")
        .arg("--lib")
        .env("CARGO_TARGET_DIR", &target)
        .env_remove("RUSTFLAGS");
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }
    for f in enabled_features() {
        cmd.arg("--features").arg(f);
    }
    if ALL_FEATURES.is_empty() {
        // nothing to toggle
    } else {
        cmd.arg("--no-default-features");
    }

    let out = cmd.output().expect("failed to spawn cargo build --lib");
    assert!(
        out.status.success(),
        "nested `cargo build --lib` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let p = target.join(profile_dir).join("libdriver.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {} after nested build",
        p.display()
    );
    p
}

/// Every feature declared in `Cargo.toml`. The crate currently declares none,
/// so there is exactly one build configuration; the list is kept so that adding
/// a feature automatically propagates to the nested build.
pub const ALL_FEATURES: &[&str] = &[];

fn enabled_features() -> Vec<&'static str> {
    // Mirror of ALL_FEATURES gated on `cfg(feature = ...)`; empty today.
    Vec::new()
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Impl::load("C", &c_so_path()),
            rs: Impl::load("Rust", &rust_so_path()),
        }
    })
}

/// Serialises tests that touch process-global state (cwd files, stderr fd).
pub fn global_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Takes the global lock, tolerating poisoning from an earlier failed test so
/// that one failure does not cascade into unrelated ones.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    match global_lock().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Small utilities
// ---------------------------------------------------------------------------

pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "c2rust-verify-{}-{}-{:?}",
            tag,
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("create temp dir");
        TempDir(p)
    }

    pub fn file(&self, name: &str, contents: &[u8]) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, contents).expect("write temp file");
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn cstring(s: &[u8]) -> CString {
    CString::new(s).expect("no interior NUL")
}

/// `fopen(path, mode)`, panicking on failure.
pub unsafe fn fopen(path: &PathBuf, mode: &[u8]) -> *mut libc::FILE {
    let p = cstring(path.to_str().expect("utf8 path").as_bytes());
    let m = cstring(mode);
    let fp = libc::fopen(p.as_ptr(), m.as_ptr());
    assert!(!fp.is_null(), "fopen failed for {}", path.display());
    fp
}
