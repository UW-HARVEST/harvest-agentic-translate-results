//! Differential-test harness.
//!
//! Both libraries under test are loaded with `libloading::Library::new` and
//! driven **only** through their exported symbols (see `api.rs`, which is
//! generated from `c_src/include/png.h` + `c_src/include/pngpriv.h` and covers
//! all 384 exported symbols).  No Rust function is ever called directly, so the
//! `#[no_mangle]` export wrappers are exercised too.
#![allow(dead_code)]

pub mod api;
pub mod forked;
pub mod helpers;
pub mod sweep;
pub mod types;

#[allow(unused_imports)]
pub use api::{Api, API_NAMES};
#[allow(unused_imports)]
pub use forked::*;
#[allow(unused_imports)]
pub use helpers::*;
pub use types::*;

use core::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/* ------------------------------------------------------------------ */
/* library discovery + loading                                         */
/* ------------------------------------------------------------------ */

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` of the currently running test binary.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    crate_root().join("c_src/build/libpng.so")
}

fn rust_so_path() -> PathBuf {
    let p = target_profile_dir().join("liblibpng.so");
    if p.exists() {
        return p;
    }
    for prof in ["release", "debug"] {
        let q = crate_root().join("target").join(prof).join("liblibpng.so");
        if q.exists() {
            return q;
        }
    }
    p
}

fn shim_so_path() -> PathBuf {
    target_profile_dir().join("libpngharness.so")
}

fn build_shim() -> PathBuf {
    let out = shim_so_path();
    let src = crate_root().join("tests/common/shim.c");
    let fresh = std::fs::metadata(&out)
        .and_then(|a| a.modified())
        .ok()
        .zip(std::fs::metadata(&src).and_then(|a| a.modified()).ok())
        .map(|(o, s)| o >= s)
        .unwrap_or(false);
    if !fresh {
        let tmp = out.with_extension(format!("so.{}", std::process::id()));
        let st = std::process::Command::new("cc")
            .args(["-O1", "-fPIC", "-shared", "-o"])
            .arg(&tmp)
            .arg(&src)
            .status()
            .expect("cc (needed to build the test's setjmp shim)");
        assert!(st.success(), "failed to compile {}", src.display());
        let _ = std::fs::rename(&tmp, &out);
    }
    out
}

/// `cargo test` does not necessarily rebuild a `cdylib`-only lib target, so make
/// sure we are never silently testing a stale artefact.
fn assert_fresh(so: &Path) {
    let so_mtime = std::fs::metadata(so).and_then(|m| m.modified()).unwrap();
    let mut newest = so_mtime;
    let mut newest_path = so.to_path_buf();
    let mut stack = vec![crate_root().join("src")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                if m > newest {
                    newest = m;
                    newest_path = p;
                }
            }
        }
    }
    assert!(
        newest <= so_mtime,
        "{} is older than {} -- run `cargo build --release` first \
         (cargo test does not rebuild a cdylib-only lib target)",
        so.display(),
        newest_path.display()
    );
}

pub struct Shim {
    pub run: unsafe extern "C" fn(
        set_longjmp_fn: *mut c_void,
        png_ptr: *mut PngStruct,
        body: unsafe extern "C" fn(*mut c_void),
        arg: *mut c_void,
    ) -> c_int,
    pub last_error: unsafe extern "C" fn() -> *const c_char,
    pub error_fn: unsafe extern "C" fn(*mut PngStruct, *const c_char),
    pub jmp_buf_size: unsafe extern "C" fn() -> usize,
    _lib: &'static libloading::Library,
}

pub struct Libs {
    pub c: Api,
    pub rust: Api,
    pub shim: Shim,
}

/// SAFETY: every field is either a plain function pointer or a pointer into a
/// permanently mapped, read-only section of a dlopen'd library; both are safe to
/// read from any thread.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    let _ = root_pid();
    static ATEXIT: std::sync::Once = std::sync::Once::new();
    ATEXIT.call_once(|| unsafe {
        atexit(report_cases);
    });
    LIBS.get_or_init(|| unsafe {
        let cp = c_so_path();
        assert!(
            cp.exists(),
            "C reference library not built: {}\n\
             build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            cp.display()
        );
        let rp = rust_so_path();
        assert!(rp.exists(), "Rust cdylib not built: {}", rp.display());
        assert_fresh(&rp);

        // `c_src/CMakeLists.txt` links only zlib, so the reference `libpng.so`
        // leaves `floor`/`pow`/`log`/... undefined and relies on the *program*
        // pulling in libm.  A Rust test binary does not necessarily do that, so
        // publish libm into the global symbol scope first.  (Without this the C
        // library dies with `symbol lookup error: undefined symbol: floor` the
        // first time a gamma computation runs.)
        {
            use libloading::os::unix as u;
            let m = u::Library::open(Some("libm.so.6"), 0x2 /*RTLD_NOW*/ | 0x100 /*RTLD_GLOBAL*/)
                .expect("dlopen libm.so.6 (RTLD_GLOBAL)");
            std::mem::forget(m);
        }

        let cl: &'static libloading::Library =
            Box::leak(Box::new(libloading::Library::new(&cp).expect("dlopen C .so")));
        let rl: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(&rp).expect("dlopen Rust .so"),
        ));
        let sp = build_shim();
        let sl: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(&sp).expect("dlopen shim .so"),
        ));

        let shim = Shim {
            run: *sl.get(b"harness_run").unwrap(),
            last_error: *sl.get(b"harness_last_error").unwrap(),
            error_fn: *sl.get(b"harness_error_fn").unwrap(),
            jmp_buf_size: *sl.get(b"harness_jmp_buf_size").unwrap(),
            _lib: sl,
        };
        assert_eq!(
            (shim.jmp_buf_size)(),
            200,
            "glibc jmp_buf is expected to be 200 bytes on this target"
        );

        Libs {
            c: Api::load(cl, "C"),
            rust: Api::load(rl, "Rust"),
            shim,
        }
    })
}

/* ------------------------------------------------------------------ */
/* per-thread callback state                                           */
/* ------------------------------------------------------------------ */

/// State the libpng callbacks operate on.  It is reached through a thread-local
/// raw pointer rather than `png_get_io_ptr()` so that the very same callback
/// works for both libraries; a raw pointer (not `RefCell`) because libpng may
/// `longjmp` out of a callback, which would leave a `RefCell` borrowed forever.
pub struct Tls {
    /// Ordered log of everything the library reported / did.
    pub trace: Vec<String>,
    /// Bytes the library may read (`png_set_read_fn`).
    pub input: Vec<u8>,
    pub in_pos: usize,
    /// Bytes the library wrote (`png_set_write_fn`).
    pub output: Vec<u8>,
    pub flushes: u32,
    /// Rows handed to / expected from the library.
    pub rows: Vec<Vec<u8>>,
    /// Scratch used by user callbacks.
    pub counter: u64,
    /// Allocation bookkeeping for the custom allocator test.
    pub allocs: Vec<(usize, usize)>,
    pub alloc_serial: u64,
    /// Set to make the read callback report a short read.
    pub truncate_reads_at: Option<usize>,
}

impl Default for Tls {
    fn default() -> Self {
        Tls {
            trace: Vec::new(),
            input: Vec::new(),
            in_pos: 0,
            output: Vec::new(),
            flushes: 0,
            rows: Vec::new(),
            counter: 0,
            allocs: Vec::new(),
            alloc_serial: 0,
            truncate_reads_at: None,
        }
    }
}

thread_local! {
    static TLS: std::cell::Cell<*mut Tls> = const { std::cell::Cell::new(core::ptr::null_mut()) };
}

pub fn tls() -> &'static mut Tls {
    let p = TLS.with(|c| c.get());
    assert!(!p.is_null(), "no Tls installed on this thread");
    unsafe { &mut *p }
}

pub fn set_tls(p: *mut Tls) -> *mut Tls {
    TLS.with(|c| c.replace(p))
}

pub fn log(s: impl Into<String>) {
    tls().trace.push(s.into());
}

/* ------------------------------------------------------------------ */
/* libpng callbacks (shared by both libraries)                         */
/* ------------------------------------------------------------------ */

unsafe fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        "<null>".to_string()
    } else {
        std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

pub unsafe extern "C" fn warn_cb(_png: *mut PngStruct, msg: *const c_char) {
    let m = cstr(msg);
    observe(&m);
    log(format!("warning: {}", m));
}

pub unsafe extern "C" fn read_cb(png: *mut PngStruct, data: *mut u8, len: usize) {
    let t = tls();
    let avail = t.input.len().saturating_sub(t.in_pos);
    let mut n = len.min(avail);
    if let Some(limit) = t.truncate_reads_at {
        if t.in_pos + n > limit {
            n = limit.saturating_sub(t.in_pos);
        }
    }
    if n > 0 {
        core::ptr::copy_nonoverlapping(t.input.as_ptr().add(t.in_pos), data, n);
        t.in_pos += n;
    }
    if n < len {
        // Same behaviour as libpng's own png_default_read_data on a short read.
        log(format!("read_fn: short read ({} of {})", n, len));
        let api = CUR_API.with(|c| c.get());
        assert!(!api.is_null());
        ((*api).png_error)(png, b"Read Error\0".as_ptr() as *const c_char);
    }
}

pub unsafe extern "C" fn write_cb(_png: *mut PngStruct, data: *mut u8, len: usize) {
    let t = tls();
    t.output.extend_from_slice(core::slice::from_raw_parts(data, len));
}

pub unsafe extern "C" fn flush_cb(_png: *mut PngStruct) {
    tls().flushes += 1;
}

pub unsafe extern "C" fn read_status_cb(_png: *mut PngStruct, row: u32, pass: c_int) {
    log(format!("read_row_status row={} pass={}", row, pass));
}

pub unsafe extern "C" fn write_status_cb(_png: *mut PngStruct, row: u32, pass: c_int) {
    log(format!("write_row_status row={} pass={}", row, pass));
}

thread_local! {
    static CUR_API: std::cell::Cell<*const Api> =
        const { std::cell::Cell::new(core::ptr::null()) };
}

pub fn set_cur_api(p: *const Api) -> *const Api {
    CUR_API.with(|c| c.replace(p))
}

pub fn cur_api() -> &'static Api {
    let p = CUR_API.with(|c| c.get());
    assert!(!p.is_null());
    unsafe { &*p }
}

/* ------------------------------------------------------------------ */
/* running a scenario under the libpng error trap                      */
/* ------------------------------------------------------------------ */

unsafe extern "C" fn trampoline(arg: *mut c_void) {
    let f = arg as *mut &mut dyn FnMut();
    (*f)();
}

/// Result of `guarded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Guard {
    /// The body ran to completion.
    Ok,
    /// libpng raised a fatal error with this message.
    Error(String),
    /// `png_set_longjmp_fn` returned NULL.
    NoTrap,
}

/// Run `body` with a libpng error trap armed on `png_ptr`.
pub unsafe fn guarded(api: &Api, png_ptr: *mut PngStruct, body: &mut dyn FnMut()) -> Guard {
    let sh = &libs().shim;
    let mut b: &mut dyn FnMut() = body;
    let arg = &mut b as *mut &mut dyn FnMut() as *mut c_void;
    let r = (sh.run)(
        api.png_set_longjmp_fn as *mut c_void,
        png_ptr,
        trampoline,
        arg,
    );
    match r {
        0 => Guard::Ok,
        1 => {
            let m = cstr((sh.last_error)());
            observe(&m);
            Guard::Error(m)
        }
        _ => Guard::NoTrap,
    }
}

/// Every distinct diagnostic message either library has produced in this process.
///
/// `tests/errors.rs` diffs this against `tests/error_sites.txt` (generated from
/// the C sources by `tools/gen_error_sites.py`) to report which of libpng's
/// rejection sites the error-path tests have actually reached.
pub static OBSERVED: std::sync::Mutex<Option<std::collections::BTreeSet<String>>> =
    std::sync::Mutex::new(None);

/// Set in a `fork()`ed child (see `forked.rs`).  A child must not touch the
/// process-global `OBSERVED` mutex: if another thread happened to hold it at the
/// moment of the fork, that lock is held forever in the child's copy of memory
/// and the child would deadlock.  A child's observations are useless for coverage
/// anyway -- they live in a separate address space.
pub static IN_FORKED_CHILD: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn observe(msg: &str) {
    if IN_FORKED_CHILD.load(std::sync::atomic::Ordering::Relaxed) {
        // Lock-free path: a forked child may not take a process-global mutex, but
        // its observations are valuable (the out-of-memory scenarios live there),
        // so append them straight to this process's file.
        append_observed(msg);
        return;
    }
    // try_lock, never lock: this runs from inside libpng callbacks and must never
    // be able to block.
    if let Ok(mut g) = OBSERVED.try_lock() {
        if g.get_or_insert_with(Default::default).insert(msg.to_string()) {
            drop(g);
            append_observed(msg);
        }
    }
}

/// Where every test binary records the diagnostics it saw, so that
/// `tools/error_coverage.py` can compute the coverage of the *whole* suite (each
/// integration test is its own process, so an in-process set only ever sees a
/// fraction of libpng's error surface).
pub fn observed_dir() -> PathBuf {
    let d = crate_root().join("target/observed");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// The pid of the *test process*, captured before any fork, so that a forked
/// child appends to the same file as its parent instead of creating a new one.
static ROOT_PID: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

pub fn root_pid() -> u32 {
    *ROOT_PID.get_or_init(std::process::id)
}

fn append_observed(msg: &str) {
    use std::io::Write;
    let p = observed_dir().join(format!("{}.txt", root_pid()));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = f.write_all(msg.replace('\n', " ").as_bytes());
        let _ = f.write_all(b"\n");
    }
}

/// The union of everything every test binary of this run has observed.
pub fn observed_all() -> std::collections::BTreeSet<String> {
    let mut set = observed();
    if let Ok(rd) = std::fs::read_dir(observed_dir()) {
        for e in rd.flatten() {
            if let Ok(t) = std::fs::read_to_string(e.path()) {
                for l in t.lines() {
                    set.insert(l.to_string());
                }
            }
        }
    }
    set
}

pub fn observed() -> std::collections::BTreeSet<String> {
    OBSERVED
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_default()
}

/* ------------------------------------------------------------------ */
/* the differential driver                                             */
/* ------------------------------------------------------------------ */

/// Everything a scenario observed: an ordered event log plus the produced bytes.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub trace: Vec<String>,
    pub output: Vec<u8>,
}

impl Outcome {
    pub fn push(&mut self, s: impl Into<String>) {
        self.trace.push(s.into());
    }
}

/// Run `f` once against the C library and once against the Rust library, each
/// with a fresh `Tls`, and return the two outcomes.
pub fn run_both<F>(mut f: F) -> (Outcome, Outcome)
where
    F: FnMut(&Api) -> Outcome,
{
    let l = libs();
    let mut out = Vec::new();
    for api in [&l.c, &l.rust] {
        let mut state = Box::new(Tls::default());
        let prev = set_tls(&mut *state as *mut Tls);
        let prev_api = set_cur_api(api as *const Api);
        let mut o = f(api);
        // The event log the callbacks appended goes first, then whatever the
        // scenario itself recorded, so that both sides are ordered identically.
        let mut trace = std::mem::take(&mut state.trace);
        trace.append(&mut o.trace);
        o.trace = trace;
        set_cur_api(prev_api);
        set_tls(prev);
        out.push(o);
    }
    let b = out.pop().unwrap();
    let a = out.pop().unwrap();
    (a, b)
}

fn hexdump(b: &[u8], from: usize) -> String {
    let start = from.saturating_sub(16) & !15;
    let end = (from + 48).min(b.len());
    let mut s = String::new();
    let mut i = start;
    while i < end {
        s.push_str(&format!("{:08x} ", i));
        for j in i..(i + 16).min(end) {
            s.push_str(&format!("{:02x} ", b[j]));
        }
        s.push('\n');
        i += 16;
    }
    s
}

/// How many differential comparisons this test binary has performed.
pub static CASES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// Record the comparison counts when the test process exits, so that
// `tools/count_cases.py` can report the total for the whole suite.  Registered
// from `libs()`, i.e. exactly once per test process, before any fork.
extern "C" {
    fn atexit(f: extern "C" fn()) -> c_int;
}

extern "C" fn report_cases() {
    if IN_FORKED_CHILD.load(std::sync::atomic::Ordering::Relaxed) {
        return; // a child's counts belong to its parent
    }
    use std::io::Write;
    let n = CASES.load(std::sync::atomic::Ordering::Relaxed);
    let f = forked::FORKED_CASES.load(std::sync::atomic::Ordering::Relaxed);
    if n == 0 && f == 0 {
        return;
    }
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let p = observed_dir().join(format!("cases-{}.txt", root_pid()));
    if let Ok(mut fh) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = writeln!(fh, "{}\t{}\t{}", exe, n, f);
    }
}

/// Assert the two libraries behaved identically for the scenario `f`.
#[track_caller]
pub fn assert_same<F>(case: &str, f: F)
where
    F: FnMut(&Api) -> Outcome,
{
    let (c, r) = run_both(f);
    CASES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    assert!(
        !c.trace.is_empty() || !c.output.is_empty(),
        "case `{}`: the scenario recorded nothing at all -- the test is not \
         actually exercising the library",
        case
    );
    if c.trace != r.trace {
        let n = c
            .trace
            .iter()
            .zip(r.trace.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.trace.len().min(r.trace.len()));
        let ctx = n.saturating_sub(4);
        let mut msg = format!(
            "case `{}`: event traces differ at index {} (C has {} events, Rust has {})\n",
            case,
            n,
            c.trace.len(),
            r.trace.len()
        );
        for i in ctx..(n + 6).min(c.trace.len().max(r.trace.len())) {
            let a = c.trace.get(i).map(String::as_str).unwrap_or("<none>");
            let b = r.trace.get(i).map(String::as_str).unwrap_or("<none>");
            msg += &format!(
                "  [{}] {} C: {}\n       {} R: {}\n",
                i,
                if a == b { ' ' } else { '!' },
                a,
                if a == b { ' ' } else { '!' },
                b
            );
        }
        panic!("{}", msg);
    }
    if c.output != r.output {
        let n = c
            .output
            .iter()
            .zip(r.output.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.output.len().min(r.output.len()));
        panic!(
            "case `{}`: output bytes differ at offset {} (C {} bytes, Rust {} bytes)\nC:\n{}\nRust:\n{}",
            case,
            n,
            c.output.len(),
            r.output.len(),
            hexdump(&c.output, n),
            hexdump(&r.output, n)
        );
    }
}

/* ------------------------------------------------------------------ */
/* deterministic RNG (no external crates)                              */
/* ------------------------------------------------------------------ */

/// SplitMix64 — fixed seed, so every failure is reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9e3779b97f4a7c15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi > lo);
        lo + (self.next_u64() % (hi - lo) as u64) as i64
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    pub fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        v[self.below(v.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/* ------------------------------------------------------------------ */
/* small helpers                                                       */
/* ------------------------------------------------------------------ */

pub fn cs(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).unwrap()
}

pub const VER: *const c_char = b"1.6.59.git\0".as_ptr() as *const c_char;

/// Create a read struct with the harness error/warning handlers installed.
pub unsafe fn new_read(api: &Api) -> (*mut PngStruct, *mut PngInfo) {
    let sh = &libs().shim;
    let png = (api.png_create_read_struct)(VER, core::ptr::null_mut(), Some(sh.error_fn), Some(warn_cb));
    assert!(!png.is_null(), "{}: png_create_read_struct failed", api.which);
    let info = (api.png_create_info_struct)(png);
    assert!(!info.is_null());
    (png, info)
}

pub unsafe fn new_write(api: &Api) -> (*mut PngStruct, *mut PngInfo) {
    let sh = &libs().shim;
    let png =
        (api.png_create_write_struct)(VER, core::ptr::null_mut(), Some(sh.error_fn), Some(warn_cb));
    assert!(!png.is_null(), "{}: png_create_write_struct failed", api.which);
    let info = (api.png_create_info_struct)(png);
    assert!(!info.is_null());
    (png, info)
}

pub unsafe fn destroy_read(api: &Api, png: *mut PngStruct, info: *mut PngInfo) {
    let mut p = png;
    let mut i = info;
    (api.png_destroy_read_struct)(&mut p, &mut i, core::ptr::null_mut());
}

pub unsafe fn destroy_write(api: &Api, png: *mut PngStruct, info: *mut PngInfo) {
    let mut p = png;
    let mut i = info;
    (api.png_destroy_write_struct)(&mut p, &mut i);
}
