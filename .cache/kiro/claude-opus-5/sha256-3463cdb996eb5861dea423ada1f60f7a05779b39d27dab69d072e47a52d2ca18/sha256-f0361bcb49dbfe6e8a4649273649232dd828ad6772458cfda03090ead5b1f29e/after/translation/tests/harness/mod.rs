//! Shared differential-test harness.
//!
//! Loads BOTH the C `libdriver.so` and the Rust `libdriver.so` with
//! `libloading` and calls every entry point through the dynamic-symbol table,
//! exactly as an external C consumer would. No Rust function is ever called
//! directly, so the `#[no_mangle]` export wrappers are under test too.

#![allow(dead_code, non_camel_case_types, non_snake_case)]

use std::ffi::{CStr, CString, c_char, c_int, c_long, c_uint, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::{Mutex, MutexGuard};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// C ABI types mirrored from the headers (used only to read results back).
// ---------------------------------------------------------------------------

pub const MAX_FQUEUE: usize = 256;

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

#[repr(C)]
pub struct FILE {
    _o: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct timespec {
    pub tv_sec: c_long,
    pub tv_nsec: c_long,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [i64; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

impl Default for tm {
    fn default() -> Self {
        // Same as `struct tm time = {0};`
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct file_queue {
    pub last_change: i64,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat,
}

impl Default for file_queue {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
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

// ---------------------------------------------------------------------------
// libc, used by the tests themselves to build FILE* etc. Not part of the
// translation under test.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;
    pub fn fclose(fp: *mut FILE) -> c_int;
    pub fn fflush(fp: *mut FILE) -> c_int;
    pub fn fseek(fp: *mut FILE, off: c_long, whence: c_int) -> c_int;
    pub fn ftell(fp: *mut FILE) -> c_long;
    pub fn feof(fp: *mut FILE) -> c_int;
    pub fn ferror(fp: *mut FILE) -> c_int;
    pub fn fileno(fp: *mut FILE) -> c_int;
    pub fn fwrite(buf: *const c_void, sz: usize, n: usize, fp: *mut FILE) -> usize;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    pub fn close(fd: c_int) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(a: c_int, b: c_int) -> c_int;
    pub fn free(p: *mut c_void);
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, opts: c_int) -> c_int;
    pub fn _exit(code: c_int) -> !;
    pub fn chdir(path: *const c_char) -> c_int;
    pub fn getpid() -> c_int;
    pub fn mkdir(path: *const c_char, mode: u32) -> c_int;
}

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

// ---------------------------------------------------------------------------
// The two libraries under comparison.
// ---------------------------------------------------------------------------

pub type FnDriver = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut alert_data;
pub type FnInitFileQueue = unsafe extern "C" fn(*mut file_queue, *const tm, c_int) -> c_int;
pub type FnReadFileMon = unsafe extern "C" fn(*mut file_queue, *const tm, c_uint) -> *mut alert_data;
pub type FnGetAlertData = unsafe extern "C" fn(c_int, *mut FILE) -> *mut alert_data;
pub type FnFreeAlertData = unsafe extern "C" fn(*mut alert_data);
pub type FnMerror = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);
pub type FnOsCalloc = unsafe extern "C" fn(usize, usize) -> *mut c_void;
pub type FnOsRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnOsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub driver: FnDriver,
    pub Init_FileQueue: FnInitFileQueue,
    pub Read_FileMon: FnReadFileMon,
    pub GetAlertData: FnGetAlertData,
    pub FreeAlertData: FnFreeAlertData,
    pub merror: FnMerror,
    pub os_calloc: FnOsCalloc,
    pub os_realloc: FnOsRealloc,
    pub os_strdup: FnOsStrdup,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
            Lib {
                name,
                driver: sym(&lib, b"driver\0"),
                Init_FileQueue: sym(&lib, b"Init_FileQueue\0"),
                Read_FileMon: sym(&lib, b"Read_FileMon\0"),
                GetAlertData: sym(&lib, b"GetAlertData\0"),
                FreeAlertData: sym(&lib, b"FreeAlertData\0"),
                merror: sym(&lib, b"merror\0"),
                os_calloc: sym(&lib, b"os_calloc\0"),
                os_realloc: sym(&lib, b"os_realloc\0"),
                os_strdup: sym(&lib, b"os_strdup\0"),
                path,
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation has a parent")
        .to_path_buf()
}

pub fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let root = workspace_root();
        let c_so = root.join("c_src/build/libdriver.so");
        assert!(
            c_so.exists(),
            "C shared library not built: {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_so.display()
        );

        // Prefer the profile cargo is currently using, fall back to the other.
        let mut candidates = vec![
            root.join("translation/target/release/libdriver.so"),
            root.join("translation/target/debug/libdriver.so"),
        ];
        if cfg!(debug_assertions) {
            candidates.reverse();
        }
        let rs_so = candidates
            .iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| {
                panic!(
                    "Rust shared library not built; run `cargo build --release` in translation/ (looked in {:?})",
                    candidates
                )
            })
            .clone();

        Pair {
            c: Lib::open("C", c_so),
            rs: Lib::open("Rust", rs_so),
        }
    })
}

// ---------------------------------------------------------------------------
// Comparable snapshots of the C data structures.
// ---------------------------------------------------------------------------

/// Owned, comparable copy of an `alert_data` (or its absence).
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum AlertSnap {
    Null,
    Some {
        rule: c_uint,
        level: c_uint,
        srcport: c_int,
        dstport: c_int,
        alertid: Option<Vec<u8>>,
        date: Option<Vec<u8>>,
        location: Option<Vec<u8>>,
        comment: Option<Vec<u8>>,
        group: Option<Vec<u8>>,
        srcip: Option<Vec<u8>>,
        dstip: Option<Vec<u8>>,
        user: Option<Vec<u8>>,
        filename: Option<Vec<u8>>,
    },
}

unsafe fn cstr_opt(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_bytes().to_vec())
    }
}

/// Snapshot an `alert_data*` and then release it through the SAME library that
/// produced it (`FreeAlertData`), so the free path is exercised as well.
pub unsafe fn snap_and_free(lib: &Lib, p: *mut alert_data) -> AlertSnap {
    unsafe {
        if p.is_null() {
            return AlertSnap::Null;
        }
        let a = &*p;
        let snap = AlertSnap::Some {
            rule: a.rule,
            level: a.level,
            srcport: a.srcport,
            dstport: a.dstport,
            alertid: cstr_opt(a.alertid),
            date: cstr_opt(a.date),
            location: cstr_opt(a.location),
            comment: cstr_opt(a.comment),
            group: cstr_opt(a.group),
            srcip: cstr_opt(a.srcip),
            dstip: cstr_opt(a.dstip),
            user: cstr_opt(a.user),
            filename: cstr_opt(a.filename),
        };
        (lib.FreeAlertData)(p);
        snap
    }
}

/// Comparable copy of a `file_queue`. The raw `FILE*` value is intentionally
/// excluded (it is a heap address) but whether it is NULL is recorded.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FqSnap {
    pub last_change: i64,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [u8; 4],
    pub file_name: Vec<u8>,
    /// Raw bytes of `file_name[0..=MAX_FQUEUE]` — catches stray writes past
    /// the NUL that a `Vec<u8>` C-string view would hide.
    pub file_name_raw: Vec<u8>,
    pub fp_is_null: bool,
    pub st_size: i64,
    pub st_mode: u32,
    pub st_mtime: c_long,
    pub st_ino: u64,
    /// `ftell(fp)` when `fp` is non-NULL.
    pub offset: Option<c_long>,
    pub eof: Option<c_int>,
}

impl FqSnap {
    /// Zero out the identity/timestamp fields that necessarily differ when the
    /// two libraries are run against two separately created copies of the same
    /// file (inode number and mtime are properties of the file, not of the
    /// code under test). Everything else stays compared.
    pub fn masked(mut self) -> FqSnap {
        self.st_ino = 0;
        self.st_mtime = 0;
        self.last_change = 0;
        self
    }
}

pub unsafe fn snap_fq(fq: &file_queue) -> FqSnap {
    unsafe {
        let raw: Vec<u8> = fq
            .file_name
            .iter()
            .map(|&c| c as u8)
            .collect::<Vec<u8>>();
        let name = raw
            .iter()
            .position(|&b| b == 0)
            .map(|n| raw[..n].to_vec())
            .unwrap_or_else(|| raw.clone());
        FqSnap {
            last_change: fq.last_change,
            year: fq.year,
            day: fq.day,
            flags: fq.flags,
            mon: [
                fq.mon[0] as u8,
                fq.mon[1] as u8,
                fq.mon[2] as u8,
                fq.mon[3] as u8,
            ],
            file_name: name,
            file_name_raw: raw,
            fp_is_null: fq.fp.is_null(),
            st_size: fq.f_status.st_size,
            st_mode: fq.f_status.st_mode,
            st_mtime: fq.f_status.st_mtim.tv_sec,
            st_ino: fq.f_status.st_ino,
            offset: if fq.fp.is_null() {
                None
            } else {
                Some(ftell(fq.fp))
            },
            eof: if fq.fp.is_null() {
                None
            } else {
                Some(if feof(fq.fp) != 0 { 1 } else { 0 })
            },
        }
    }
}

/// State of a `FILE*` after a call: position + eof/error indicators.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct StreamSnap {
    pub offset: c_long,
    pub eof: bool,
    pub err: bool,
}

pub unsafe fn snap_stream(fp: *mut FILE) -> StreamSnap {
    unsafe {
        StreamSnap {
            offset: ftell(fp),
            eof: feof(fp) != 0,
            err: ferror(fp) != 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Temp-file / stderr plumbing.
// ---------------------------------------------------------------------------

/// A private scratch directory, removed on drop.
pub struct Scratch {
    pub dir: PathBuf,
}

impl Scratch {
    pub fn new(tag: &str) -> Scratch {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cdiff-{}-{}-{}",
            tag,
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        Scratch { dir }
    }

    pub fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.dir.join(name);
        std::fs::write(&p, bytes).expect("write scratch file");
        p
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn cs(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

pub fn cbytes(b: &[u8]) -> CString {
    CString::new(b).expect("no interior NUL")
}

/// `fopen(path, mode)`, panicking on failure.
pub unsafe fn open_ro(path: &std::path::Path) -> *mut FILE {
    unsafe {
        let p = cs(path.to_str().unwrap());
        let m = cs("r");
        let fp = fopen(p.as_ptr(), m.as_ptr());
        assert!(!fp.is_null(), "fopen({}) failed", path.display());
        fp
    }
}

/// A read end of a pipe wrapped in a `FILE*`. Non-seekable, which is what the
/// `fseek` error branches need.
pub struct PipeFile {
    pub fp: *mut FILE,
    pub write_fd: c_int,
}

impl PipeFile {
    pub unsafe fn new(contents: &[u8]) -> PipeFile {
        unsafe {
            let mut fds = [0 as c_int; 2];
            assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
            // Pipe capacity is 64 KiB by default; all test payloads are far
            // smaller, so a blocking write cannot deadlock here.
            assert!(contents.len() < 32 * 1024);
            let n = write(fds[1], contents.as_ptr() as *const c_void, contents.len());
            assert_eq!(n, contents.len() as isize);
            close(fds[1]);
            let m = cs("r");
            let fp = fdopen(fds[0], m.as_ptr());
            assert!(!fp.is_null());
            PipeFile {
                fp,
                write_fd: fds[1],
            }
        }
    }
}

impl Drop for PipeFile {
    fn drop(&mut self) {
        unsafe {
            fclose(self.fp);
        }
    }
}

/// Run `f` with the process' `stderr` (fd 2, and the C `stderr` stream)
/// redirected into a temp file; return everything written.
static ERR_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the process-global stderr lock. `capture_stderr` redirects fd 2, so
/// any test that *writes* to stderr (the `perror` branches in `GetAlertData`)
/// must hold this too, or its output lands in someone else's capture.
pub fn stderr_lock() -> MutexGuard<'static, ()> {
    ERR_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub unsafe fn capture_stderr<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    // fd 2 is process-global; serialize every capture.
    let _g = stderr_lock();
    unsafe {
        let tmp = std::env::temp_dir().join(format!("cdiff-err-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_file(&tmp);
        let path = cs(tmp.to_str().unwrap());
        let mode = cs("w+");
        let sink = fopen(path.as_ptr(), mode.as_ptr());
        assert!(!sink.is_null());

        let saved = dup(2);
        assert!(saved >= 0);
        assert!(dup2(fileno(sink), 2) >= 0);

        let out = f();

        // Both libraries write through their own `stderr` FILE object, which
        // is the shared glibc one; flush it before restoring fd 2.
        fflush(std::ptr::null_mut());
        let bytes = std::fs::read(&tmp).unwrap_or_default();

        dup2(saved, 2);
        close(saved);
        fclose(sink);
        let _ = std::fs::remove_file(&tmp);
        (out, bytes)
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so property tests are reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    pub fn choice<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A printable token of `len` bytes drawn from a set that includes the
    /// characters the parser is sensitive to.
    pub fn token(&mut self, len: usize) -> Vec<u8> {
        const A: &[u8] = b"abcXYZ0189._-:'/ \t";
        (0..len).map(|_| *self.choice(A)).collect()
    }
    /// A token guaranteed free of `\n` and `\0` but otherwise arbitrary bytes.
    pub fn raw_token(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let mut b = (self.next_u64() & 0xFF) as u8;
                if b == 0 || b == b'\n' {
                    b = b'?';
                }
                b
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Assertion helper that names the diverging entry point.
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! assert_same {
    ($what:expr, $c:expr, $r:expr) => {{
        let c = $c;
        let r = $r;
        assert!(
            c == r,
            "DIVERGENCE in {}\n  C   : {:?}\n  Rust: {:?}",
            $what,
            c,
            r
        );
    }};
}

// ---------------------------------------------------------------------------
// Process-global CWD lock.
//
// `Init_FileQueue`/`driver` open the *relative* path "alerts.log", so any test
// that drives them must chdir. chdir is process-wide, hence this mutex.
// ---------------------------------------------------------------------------


static CWD_LOCK: Mutex<()> = Mutex::new(());

pub struct CwdGuard {
    _g: MutexGuard<'static, ()>,
    old: PathBuf,
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.old);
    }
}

/// Enter `dir` as the process CWD, excluding every other test that needs it.
pub fn enter_dir(dir: &std::path::Path) -> CwdGuard {
    let g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let old = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir).expect("chdir");
    CwdGuard { _g: g, old }
}

// ---------------------------------------------------------------------------
// Alert-text corpus builders.
// ---------------------------------------------------------------------------

/// One synthetic alert record.
#[derive(Clone, Debug, Default)]
pub struct AlertSpec {
    pub id: String,
    /// The token right after the id, e.g. `mail` or `-`.
    pub kind: String,
    pub group: String,
    pub date: String,
    pub location: String,
    pub rule: Option<String>,
    pub srcip: Option<String>,
    pub srcport: Option<String>,
    pub dstip: Option<String>,
    pub dstport: Option<String>,
    pub user: Option<String>,
    pub body: Vec<String>,
}

impl AlertSpec {
    /// A minimal-but-complete alert: header, date/location, `Rule:` line.
    pub fn minimal() -> AlertSpec {
        AlertSpec {
            id: "1500000000.1234".into(),
            kind: "mail".into(),
            group: "syslog,errors".into(),
            date: "2006 Apr 13 16:15:17".into(),
            location: "/var/log/auth.log".into(),
            rule: Some("Rule: 1002 (level 6) -> 'Unknown problem somewhere'".into()),
            ..Default::default()
        }
    }

    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "** Alert {}: {} - {}\n",
            self.id, self.kind, self.group
        ));
        s.push_str(&format!("{} {}\n", self.date, self.location));
        for line in [
            self.rule.as_ref(),
            self.srcip.as_ref(),
            self.srcport.as_ref(),
            self.dstip.as_ref(),
            self.dstport.as_ref(),
            self.user.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            s.push_str(line);
            s.push('\n');
        }
        for line in &self.body {
            s.push_str(line);
            s.push('\n');
        }
        s
    }
}

pub fn render_all(specs: &[AlertSpec]) -> String {
    specs.iter().map(|a| a.render()).collect()
}

/// Drain a stream with repeated `GetAlertData` calls until it yields NULL,
/// snapshotting the stream state after every call. Bounded to avoid a runaway
/// loop if the two implementations disagree about progress.
pub unsafe fn drain(lib: &Lib, flag: c_int, fp: *mut FILE) -> Vec<(AlertSnap, StreamSnap)> {
    unsafe {
        let mut out = Vec::new();
        for _ in 0..512 {
            let p = (lib.GetAlertData)(flag, fp);
            let snap = snap_and_free(lib, p);
            let st = snap_stream(fp);
            let done = snap == AlertSnap::Null;
            out.push((snap, st));
            if done {
                break;
            }
        }
        out
    }
}

/// Differential `GetAlertData` on identical file contents: run the full drain
/// against the C lib and against the Rust lib and compare.
pub fn diff_get_alert_data(tag: &str, contents: &[u8], flag: c_int) {
    let p = libs();
    // Some inputs make `GetAlertData` call `perror`; keep that off other
    // tests' captured stderr.
    let _err = stderr_lock();
    let sc = Scratch::new("gad");
    let path = sc.write("alerts.log", contents);
    unsafe {
        let fc = open_ro(&path);
        let c = drain(&p.c, flag, fc);
        fclose(fc);

        let fr = open_ro(&path);
        let r = drain(&p.rs, flag, fr);
        fclose(fr);

        assert!(
            c == r,
            "DIVERGENCE in GetAlertData [{tag}] flag={flag:#x}\n  input={:?}\n  C   : {:#?}\n  Rust: {:#?}",
            String::from_utf8_lossy(contents),
            c,
            r
        );
    }
}

impl Rng {
    /// `token` with a length itself drawn from `0..n`.
    pub fn token_upto(&mut self, n: usize) -> Vec<u8> {
        let len = if n == 0 { 0 } else { self.below(n) };
        self.token(len)
    }
    /// `raw_token` with a length itself drawn from `0..n`.
    pub fn raw_token_upto(&mut self, n: usize) -> Vec<u8> {
        let len = if n == 0 { 0 } else { self.below(n) };
        self.raw_token(len)
    }
}

// ---------------------------------------------------------------------------
// Sub-process execution, for the `exit(EXIT_FAILURE)` paths in shared.h.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    pub fn mkfifo(path: *const c_char, mode: u32) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

pub const O_RDWR: c_int = 2;
pub const O_NONBLOCK: c_int = 0o4000;

/// Outcome of running a closure in a forked child.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ChildOutcome {
    /// `_exit`/`exit` with this status.
    Exited(c_int),
    /// Killed by this signal.
    Signalled(c_int),
    /// The closure returned instead of exiting.
    Returned,
}

/// Run `f` in a forked child with stderr captured; report how the child ended
/// and what it wrote. Used for `os_calloc`/`os_realloc`/`os_strdup`, which call
/// `exit(EXIT_FAILURE)` on failure and so cannot be tested in-process.
pub unsafe fn run_in_child(f: impl FnOnce()) -> (ChildOutcome, Vec<u8>) {
    unsafe {
        static FORK_LOCK: Mutex<()> = Mutex::new(());
        let _g = FORK_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let tmp = std::env::temp_dir().join(format!("cdiff-child-{}", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let path = cs(tmp.to_str().unwrap());
        let mode = cs("w+");

        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: redirect stderr into the temp file, run, and mark a
            // "returned without exiting" outcome with status 123.
            let sink = fopen(path.as_ptr(), mode.as_ptr());
            if !sink.is_null() {
                dup2(fileno(sink), 2);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(123);
        }

        let mut status: c_int = 0;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        let bytes = std::fs::read(&tmp).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp);

        let outcome = if status & 0x7f == 0 {
            let code = (status >> 8) & 0xff;
            if code == 123 {
                ChildOutcome::Returned
            } else {
                ChildOutcome::Exited(code)
            }
        } else {
            ChildOutcome::Signalled(status & 0x7f)
        };
        (outcome, bytes)
    }
}

/// A FIFO wrapped so that `fopen(path, "r")` succeeds immediately (a writer is
/// held open) but `fseek` fails with `ESPIPE`. This is how the `Handle_Queue`
/// `fseek`-error branch is reached through the public API.
pub struct Fifo {
    pub path: PathBuf,
    writer: c_int,
}

impl Fifo {
    pub fn new(path: PathBuf) -> Fifo {
        let _ = std::fs::remove_file(&path);
        let p = cs(path.to_str().unwrap());
        unsafe {
            assert_eq!(mkfifo(p.as_ptr(), 0o600), 0, "mkfifo failed");
            // O_RDWR on a FIFO never blocks on Linux and keeps a writer open.
            let w = open(p.as_ptr(), O_RDWR | O_NONBLOCK);
            assert!(w >= 0, "open fifo failed");
            Fifo { path, writer: w }
        }
    }
}

impl Drop for Fifo {
    fn drop(&mut self) {
        unsafe {
            close(self.writer);
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// A `FILE*` whose underlying descriptor has been closed behind its back, so
/// `fstat(fileno(fp))` fails with `EBADF`.
pub struct DeadFile {
    pub fp: *mut FILE,
}

impl DeadFile {
    pub unsafe fn new() -> DeadFile {
        unsafe {
            let mut fds = [0 as c_int; 2];
            assert_eq!(pipe(fds.as_mut_ptr()), 0);
            close(fds[1]);
            // Relocate to a high descriptor number: glibc hands out the LOWEST
            // free fd, so after we close this one a later `fopen` (e.g. the
            // stderr capture) would otherwise reuse the number and `fstat`
            // would succeed again.
            static NEXT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(900);
            let high = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            assert!(dup2(fds[0], high) == high, "dup2 to fd {high} failed");
            close(fds[0]);
            let m = cs("r");
            let fp = fdopen(high, m.as_ptr());
            assert!(!fp.is_null());
            // Yank the descriptor out from under the stream.
            close(high);
            DeadFile { fp }
        }
    }
}
