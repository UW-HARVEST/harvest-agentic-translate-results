//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` (`dlopen`)
//! and every call goes through the exported C ABI symbols — the Rust functions
//! are never called directly, so the `#[no_mangle]` wrappers are under test as
//! well.
//!
//! The library under test writes to the process' `stdout`/`stderr` through
//! glibc `stdio`, so "output" means: the return value **plus** the exact bytes
//! that landed on fd 1 and fd 2. Both are captured by temporarily `dup2`-ing
//! the two fds onto scratch files.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type CFile = libc::FILE;

pub type FgeFn = unsafe extern "C" fn(c_int) -> c_int;
pub type OwcFn = unsafe extern "C" fn(*const c_char) -> *mut CFile;
pub type DrvFn = unsafe extern "C" fn(c_int, *const c_char) -> c_int;

// glibc globals. `src/lib.rs` declares `stderr` the same way.
unsafe extern "C" {
    static stdout: *mut CFile;
    static stderr: *mut CFile;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub forward_goto_example: FgeFn,
    pub open_with_cleanup: OwcFn,
    pub driver: DrvFn,
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact
/// (integration tests do not link against it), so a plain `cargo test` after
/// editing `src/lib.rs` would silently diff against a *stale* `.so` and pass.
/// Refuse to run in that case instead of reporting a vacuous success.
fn assert_fresh(name: &str, so: &Path, sources: &[PathBuf], rebuild_hint: &str) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {so:?}: {e}"));
    for src in sources {
        let Ok(m) = std::fs::metadata(src).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            m <= so_mtime,
            "{name} shared object {so:?} is OLDER than its source {src:?}.\n\
             The differential tests would compare against a stale library.\n\
             Rebuild first: {rebuild_hint}"
        );
    }
}

fn collect_sources(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            // never descend into build output
            if p.file_name().is_some_and(|n| n == "build" || n == "target") {
                continue;
            }
            collect_sources(&p, exts, out);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.contains(&e))
        {
            out.push(p);
        }
    }
}

impl Api {
    fn load(name: &'static str, path: PathBuf) -> Api {
        assert!(
            path.exists(),
            "{name} shared object not found at {path:?}; build it first \
             (C: cmake --build c_src/build, Rust: cargo build)"
        );
        let mut sources = Vec::new();
        if name == "C" {
            collect_sources(&manifest_dir().join("c_src"), &["c", "h"], &mut sources);
            assert_fresh(
                name,
                &path,
                &sources,
                "cd c_src/build && cmake --build .   (or ./verify.sh)",
            );
        } else {
            collect_sources(&manifest_dir().join("src"), &["rs"], &mut sources);
            sources.push(manifest_dir().join("Cargo.toml"));
            assert_fresh(
                name,
                &path,
                &sources,
                "cargo build   (cargo test alone does NOT rebuild a cdylib!) — use ./verify.sh",
            );
        }
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({path:?}) failed: {e}"));
        unsafe {
            let forward_goto_example = *lib
                .get::<FgeFn>(b"forward_goto_example\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol forward_goto_example: {e}"));
            let open_with_cleanup = *lib
                .get::<OwcFn>(b"open_with_cleanup\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol open_with_cleanup: {e}"));
            let driver = *lib
                .get::<DrvFn>(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol driver: {e}"));
            Api {
                name,
                path,
                _lib: lib,
                forward_goto_example,
                open_with_cleanup,
                driver,
            }
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

/// `target/<profile>/libdriver.so`, derived from the test executable's own path
/// (`target/<profile>/deps/<test>-<hash>`).
pub fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let direct = profile.join("libdriver.so");
    if direct.exists() {
        return direct;
    }
    let in_deps = deps.join("libdriver.so");
    if in_deps.exists() {
        return in_deps;
    }
    panic!("libdriver.so not found in {profile:?} or {deps:?}; run `cargo build` first");
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| Api::load("C", c_so()))
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| Api::load("Rust", rust_so()))
}

// ---------------------------------------------------------------------------
// Scratch files
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_id() -> u64 {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// A per-process scratch directory inside `target/`.
pub fn tmp_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let base = std::option_env!("CARGO_TARGET_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest_dir().join("target/tmp"));
        // The pid alone is not unique enough: pids are recycled, and a stale
        // mode-000 fixture from an earlier run would make this run fail.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d = base.join(format!("difftest-{}-{nanos:x}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap_or_else(|e| panic!("mkdir {d:?}: {e}"));
        d
    })
}

/// Write `content` to a fresh file and return its path as a `CString`.
pub fn fixture(tag: &str, content: &[u8]) -> CString {
    let p = tmp_dir().join(format!("fix-{}-{}", tag, next_id()));
    std::fs::write(&p, content).unwrap_or_else(|e| panic!("write {p:?}: {e}"));
    path_cstring(&p)
}

/// Write `content` to a fresh file whose *name* contains raw non-UTF-8 bytes.
pub fn fixture_raw_name(name_bytes: &[u8], content: &[u8]) -> CString {
    use std::os::unix::ffi::OsStrExt;
    let mut full = tmp_dir().as_os_str().as_bytes().to_vec();
    full.push(b'/');
    full.extend_from_slice(name_bytes);
    full.extend_from_slice(format!("-{}", next_id()).as_bytes());
    let p = PathBuf::from(std::ffi::OsStr::from_bytes(&full));
    std::fs::write(&p, content).unwrap_or_else(|e| panic!("write {p:?}: {e}"));
    path_cstring(&p)
}

pub fn path_cstring(p: &Path) -> CString {
    use std::os::unix::ffi::OsStrExt;
    CString::new(p.as_os_str().as_bytes()).expect("path has interior NUL")
}

/// A path that is guaranteed not to exist.
pub fn missing_path() -> CString {
    let p = tmp_dir().join(format!("does-not-exist-{}", next_id()));
    let _ = std::fs::remove_file(&p);
    path_cstring(&p)
}

/// A fresh directory (usable as a `fopen` target: opens, then read fails).
pub fn dir_path() -> CString {
    let p = tmp_dir().join(format!("a-directory-{}", next_id()));
    std::fs::create_dir_all(&p).unwrap();
    path_cstring(&p)
}

/// An existing file with mode 000 (`fopen` fails with EACCES).
pub fn unreadable_path() -> CString {
    use std::os::unix::fs::PermissionsExt;
    let p = tmp_dir().join(format!("unreadable-{}", next_id()));
    // a leftover mode-000 file would make the write below fail
    let _ = std::fs::remove_file(&p);
    std::fs::write(&p, b"secret\n").unwrap_or_else(|e| panic!("write {p:?}: {e}"));
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();
    path_cstring(&p)
}

// ---------------------------------------------------------------------------
// fd-level output capture
// ---------------------------------------------------------------------------

/// Captured observable output of one call.
#[derive(Clone, PartialEq, Eq)]
pub struct Cap {
    pub out: Vec<u8>,
    pub err: Vec<u8>,
    /// Set when stdout and stderr were pointed at the *same* fd; then `out`
    /// holds the interleaved stream and `err` is empty.
    pub merged: bool,
}

impl std::fmt::Debug for Cap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.merged {
            write!(f, "merged={:?}", Show(&self.out))
        } else {
            write!(f, "stdout={:?} stderr={:?}", Show(&self.out), Show(&self.err))
        }
    }
}

/// Byte-exact but readable rendering of captured output.
pub struct Show<'a>(pub &'a [u8]);

impl std::fmt::Debug for Show<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for &b in self.0 {
            match b {
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                b'"' => write!(f, "\\\"")?,
                b'\\' => write!(f, "\\\\")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        write!(f, "\" ({} bytes)", self.0.len())
    }
}

static IO_LOCK: Mutex<()> = Mutex::new(());

fn io_lock() -> MutexGuard<'static, ()> {
    IO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Pin glibc's `stdout` buffering mode once, so that every capture — for the C
/// library and for the Rust library alike — sees exactly the same buffering
/// state and therefore the same stdout/stderr interleaving.
fn init_stdio() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        libc::setvbuf(stdout, std::ptr::null_mut(), libc::_IOFBF, 4096);
        libc::setvbuf(stderr, std::ptr::null_mut(), libc::_IONBF, 0);
    });
}

/// Capturing works by `dup2`-ing fd 1 and fd 2, which is process-wide. libtest's
/// own progress output ("test foo ... ok") is written straight to fd 1 by the
/// runner thread, so if tests ran concurrently that text would land inside a
/// captured stream and be diffed as if the library had produced it. The suite
/// therefore requires serial execution.
fn require_serial_execution() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let args: Vec<String> = std::env::args().collect();
        let flag_ok = args.windows(2).any(|w| {
            (w[0] == "--test-threads" && w[1] == "1")
                || (w[0].starts_with("--test-threads=") && w[0].ends_with("=1"))
        }) || args
            .iter()
            .any(|a| a == "--test-threads=1" || a == "--test-thread=1");
        let env_ok = std::env::var("RUST_TEST_THREADS").ok().as_deref() == Some("1");
        assert!(
            flag_ok || env_ok,
            "this differential suite captures fd 1 / fd 2 process-wide and must \
             run serially.\nRe-run with:  cargo test -- --test-threads=1   \
             (or set RUST_TEST_THREADS=1); ./verify.sh does this for you."
        );
    });
}

unsafe fn open_trunc(p: &Path) -> c_int {
    let cs = path_cstring(p);
    let fd = unsafe {
        libc::open(
            cs.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_uint,
        )
    };
    assert!(fd >= 0, "open({p:?}) failed: {}", std::io::Error::last_os_error());
    fd
}

fn cap_impl<T>(merged: bool, f: impl FnOnce() -> T) -> (T, Cap) {
    let _g = io_lock();
    require_serial_execution();
    init_stdio();
    let id = next_id();
    let op = tmp_dir().join(format!("cap-{id}.out"));
    let ep = tmp_dir().join(format!("cap-{id}.err"));

    // Push out anything libtest has buffered in Rust's own line-buffered
    // stdout/stderr (e.g. the partial "test foo ... " line) *before* fd 1 and
    // fd 2 are pointed elsewhere.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }

    let (r, out, err) = unsafe {
        libc::fflush(stdout);
        libc::fflush(stderr);

        let saved_out = libc::dup(1);
        let saved_err = libc::dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");

        let fo = open_trunc(&op);
        // `dup` shares the file *description* (and hence the offset), which is
        // what makes the interleaving of the two streams observable.
        let fe = if merged { libc::dup(fo) } else { open_trunc(&ep) };
        assert!(fe >= 0, "dup/open failed");

        assert_eq!(libc::dup2(fo, 1), 1, "dup2 stdout");
        assert_eq!(libc::dup2(fe, 2), 2, "dup2 stderr");

        // A panic inside `f` must not leave fd 1 / fd 2 dangling on the scratch
        // files, or every later test in this binary would lose its output.
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        libc::fflush(stdout);
        libc::fflush(stderr);

        libc::dup2(saved_out, 1);
        libc::dup2(saved_err, 2);
        libc::close(saved_out);
        libc::close(saved_err);
        libc::close(fo);
        libc::close(fe);

        let out = std::fs::read(&op).unwrap_or_default();
        let err = if merged {
            Vec::new()
        } else {
            std::fs::read(&ep).unwrap_or_default()
        };
        (r, out, err)
    };

    let _ = std::fs::remove_file(&op);
    let _ = std::fs::remove_file(&ep);
    match r {
        Ok(v) => (v, Cap { out, err, merged }),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Run `f` with fd 1 and fd 2 captured separately.
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Cap) {
    cap_impl(false, f)
}

/// Run `f` with fd 1 and fd 2 pointing at the *same* file, so the relative
/// ordering of buffered stdout and unbuffered stderr writes is observable.
pub fn capture_merged<T>(f: impl FnOnce() -> T) -> (T, Cap) {
    cap_impl(true, f)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        lo + self.below((hi - lo) as u64 + 1) as i64
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Printable ASCII (no NUL, no newline) of length `len`.
    pub fn ascii_line(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let c = 0x20u8 + (self.below(0x5f) as u8); // 0x20..=0x7e
                if c == b'\n' { b'X' } else { c }
            })
            .collect()
    }
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte()).collect()
    }
}

// ---------------------------------------------------------------------------
// Differential comparison helpers
// ---------------------------------------------------------------------------

fn assert_same<T: PartialEq + std::fmt::Debug>(
    what: &str,
    ctx: &str,
    c: (T, Cap),
    r: (T, Cap),
) {
    let (cret, ccap) = c;
    let (rret, rcap) = r;
    if cret != rret || ccap != rcap {
        panic!(
            "DIVERGENCE in {what} [{ctx}]\n  C    ret={cret:?} {ccap:?}\n  Rust ret={rret:?} {rcap:?}"
        );
    }
}

/// Differential call of `forward_goto_example`.
pub fn diff_fge(x: c_int) {
    let c = capture(|| unsafe { (c_api().forward_goto_example)(x) });
    let r = capture(|| unsafe { (rust_api().forward_goto_example)(x) });
    assert_same("forward_goto_example", &format!("x={x}"), c, r);
}

/// Differential call of `driver`. `None` means a NULL `filename` pointer.
pub fn diff_driver(num: c_int, filename: Option<&CStr>) {
    let p = filename.map(|f| f.as_ptr()).unwrap_or(std::ptr::null());
    let c = capture(|| unsafe { (c_api().driver)(num, p) });
    let r = capture(|| unsafe { (rust_api().driver)(num, p) });
    let ctx = format!(
        "num={num} filename={}",
        filename.map(|f| format!("{:?}", f)).unwrap_or("NULL".into())
    );
    assert_same("driver", &ctx, c, r);
}

/// Everything observable about the `FILE*` handed back by `open_with_cleanup`.
#[derive(Debug, PartialEq, Eq)]
pub struct StreamState {
    pub is_null: bool,
    pub ferror: c_int,
    pub feof: c_int,
    pub ftell: i64,
    /// `getc` at the current position (`-1` == EOF).
    pub next_char: c_int,
    pub feof_after_getc: c_int,
    pub fclose_ret: c_int,
}

fn probe_owc(api: &Api, p: *const c_char) -> StreamState {
    unsafe {
        let fp = (api.open_with_cleanup)(p);
        if fp.is_null() {
            return StreamState {
                is_null: true,
                ferror: 0,
                feof: 0,
                ftell: -1,
                next_char: 0,
                feof_after_getc: 0,
                fclose_ret: 0,
            };
        }
        let ferror = libc::ferror(fp);
        let feof = libc::feof(fp);
        let ftell = libc::ftell(fp) as i64;
        let next_char = libc::fgetc(fp);
        let feof_after_getc = libc::feof(fp);
        let fclose_ret = libc::fclose(fp);
        StreamState {
            is_null: false,
            ferror,
            feof,
            ftell,
            next_char,
            feof_after_getc,
            fclose_ret,
        }
    }
}

/// Differential call of `open_with_cleanup`, comparing the captured output, the
/// NULL-ness of the result and — on success — the state of the returned stream.
pub fn diff_owc(filename: Option<&CStr>) {
    let p = filename.map(|f| f.as_ptr()).unwrap_or(std::ptr::null());
    let c = capture(|| probe_owc(c_api(), p));
    let r = capture(|| probe_owc(rust_api(), p));
    let ctx = format!(
        "filename={}",
        filename.map(|f| format!("{:?}", f)).unwrap_or("NULL".into())
    );
    assert_same("open_with_cleanup", &ctx, c, r);
}

/// Same as [`diff_owc`] but the file content is described in the failure
/// message, which matters for generated fixtures.
pub fn diff_owc_content(tag: &str, content: &[u8]) {
    let f = fixture(tag, content);
    let p = f.as_ptr();
    let c = capture(|| probe_owc(c_api(), p));
    let r = capture(|| probe_owc(rust_api(), p));
    let ctx = format!("{tag}: content={:?}", Show(content));
    assert_same("open_with_cleanup", &ctx, c, r);
    let _ = std::fs::remove_file(cstr_to_path(&f));
}

pub fn diff_driver_content(tag: &str, num: c_int, content: &[u8]) {
    let f = fixture(tag, content);
    let p = f.as_ptr();
    let c = capture(|| unsafe { (c_api().driver)(num, p) });
    let r = capture(|| unsafe { (rust_api().driver)(num, p) });
    let ctx = format!("{tag}: num={num} content={:?}", Show(content));
    assert_same("driver", &ctx, c, r);
    let _ = std::fs::remove_file(cstr_to_path(&f));
}

pub fn cstr_to_path(c: &CStr) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(c.to_bytes()))
}

/// Run an arbitrary batch of calls against one library inside a *single*
/// capture; used for the "many calls in one process / cumulative buffer state"
/// rows. The batch is run once per library and the whole captured stream is
/// compared byte for byte.
pub fn diff_batch<R, F>(what: &str, merged: bool, mut batch: F)
where
    R: PartialEq + std::fmt::Debug,
    F: FnMut(&Api) -> R,
{
    let c = if merged {
        capture_merged(|| batch(c_api()))
    } else {
        capture(|| batch(c_api()))
    };
    let r = if merged {
        capture_merged(|| batch(rust_api()))
    } else {
        capture(|| batch(rust_api()))
    };
    assert_same(what, if merged { "batch/merged" } else { "batch" }, c, r);
}

/// Small helper used by batches: call `open_with_cleanup` and close whatever
/// comes back, returning the observable state.
pub fn owc_and_close(api: &Api, p: *const c_char) -> StreamState {
    probe_owc(api, p)
}

pub fn as_void(p: *const c_char) -> *const c_void {
    p as *const c_void
}
