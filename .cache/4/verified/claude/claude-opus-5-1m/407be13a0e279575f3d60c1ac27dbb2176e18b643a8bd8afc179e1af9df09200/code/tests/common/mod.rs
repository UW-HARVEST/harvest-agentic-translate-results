// Shared differential-test harness.
//
// Both the C shared library and the Rust shared library are loaded with
// `libloading` (i.e. `dlopen`/`dlsym`) and driven purely through their exported
// C ABI symbols. Rust functions are NEVER called directly from the test crate,
// so the `#[no_mangle]` export wrappers are exercised too.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits used for stdout capture
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn time(tloc: *mut i64) -> i64;
}

/// `time_t` on Linux/x86-64.
pub type TimeT = i64;

// ---------------------------------------------------------------------------
// exported ABI, as raw function pointers
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct Api {
    pub classify_mode: unsafe extern "C" fn(*const c_char) -> c_int,
    pub apply_multiplier: unsafe extern "C" fn(c_int, c_int) -> c_int,
    pub convert_time_factor: unsafe extern "C" fn(c_double) -> c_int,
    pub convert_negative_overflow: unsafe extern "C" fn(c_double) -> c_int,
    pub get_modified_time: unsafe extern "C" fn(c_int, c_int) -> TimeT,
    pub hash_time_value: unsafe extern "C" fn(TimeT) -> c_int,
    pub modeselect: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

/// Every symbol the C `.so` exports (see SYMBOLS.md). Both libraries must
/// provide all of them.
pub const EXPORTED_SYMBOLS: &[&str] = &[
    "apply_multiplier",
    "classify_mode",
    "convert_negative_overflow",
    "convert_time_factor",
    "get_modified_time",
    "hash_time_value",
    "modeselect",
];

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name));
        unsafe { *s.into_raw() }
    }};
}

fn open_lib(path: &PathBuf) -> &'static Library {
    Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
    }))
}

fn load(path: &PathBuf) -> Api {
    let lib = open_lib(path);
    Api {
        classify_mode: sym!(lib, "classify_mode", unsafe extern "C" fn(*const c_char) -> c_int),
        apply_multiplier: sym!(
            lib,
            "apply_multiplier",
            unsafe extern "C" fn(c_int, c_int) -> c_int
        ),
        convert_time_factor: sym!(
            lib,
            "convert_time_factor",
            unsafe extern "C" fn(c_double) -> c_int
        ),
        convert_negative_overflow: sym!(
            lib,
            "convert_negative_overflow",
            unsafe extern "C" fn(c_double) -> c_int
        ),
        get_modified_time: sym!(
            lib,
            "get_modified_time",
            unsafe extern "C" fn(c_int, c_int) -> TimeT
        ),
        hash_time_value: sym!(lib, "hash_time_value", unsafe extern "C" fn(TimeT) -> c_int),
        modeselect: sym!(
            lib,
            "modeselect",
            unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int
        ),
    }
}

// ---------------------------------------------------------------------------
// locating the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C reference `.so`. `build.rs` rebuilds it on every source change, so the
/// path baked in here is never stale. `C_SO` overrides it.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let p = PathBuf::from(env!("C_SO_PATH"));
    assert!(p.exists(), "C shared library missing: {}", p.display());
    p
}

/// The Rust `.so` (unoptimised). `build.rs` rebuilds it on every change of
/// `src/lib.rs`; `cargo test` on its own does NOT refresh
/// `target/<profile>/libmodeselect_lib.so`, which would silently test a stale
/// artifact. `RUST_SO` overrides it.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let p = PathBuf::from(env!("RUST_SO_PATH"));
    assert!(p.exists(), "Rust shared library missing: {}", p.display());
    p
}

/// The Rust `.so` built with `-C opt-level=3`.
pub fn rust_so_opt_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_OPT") {
        return PathBuf::from(p);
    }
    let p = PathBuf::from(env!("RUST_SO_OPT_PATH"));
    assert!(p.exists(), "optimised Rust shared library missing: {}", p.display());
    p
}

/// The artifact `cargo build` produces, if it happens to be present.
pub fn cargo_rust_so_path() -> Option<PathBuf> {
    const NAME: &str = "libmodeselect_lib.so";
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            for cand in [deps.join(NAME), deps.parent().map(|p| p.join(NAME))?] {
                if cand.exists() {
                    return Some(cand);
                }
            }
        }
    }
    for prof in ["debug", "release"] {
        let c = manifest_dir().join("target").join(prof).join(NAME);
        if c.exists() {
            return Some(c);
        }
    }
    None
}

static C_API: OnceLock<Api> = OnceLock::new();
static R_API: OnceLock<Api> = OnceLock::new();
static R_OPT_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> Api {
    *C_API.get_or_init(|| load(&c_so_path()))
}

pub fn rust_api() -> Api {
    *R_API.get_or_init(|| load(&rust_so_path()))
}

/// The `-C opt-level=3` Rust build, used to prove the translation does not
/// depend on the optimisation level.
pub fn rust_opt_api() -> Api {
    *R_OPT_API.get_or_init(|| load(&rust_so_opt_path()))
}

/// `(c, rust)` — both loaded, ready to be driven side by side.
pub fn both() -> (Api, Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// stdout capture (fd 1 redirection, so both libraries' `printf` is captured)
// ---------------------------------------------------------------------------

/// One process-wide scratch file + lock. fd 1 is process-wide, so captures must
/// be serialised even when libtest runs tests in parallel.
fn cap_file() -> &'static Mutex<std::fs::File> {
    static F: OnceLock<Mutex<std::fs::File>> = OnceLock::new();
    F.get_or_init(|| {
        let mut path = std::env::temp_dir();
        path.push(format!("difftest-stdout-{}.bin", std::process::id()));
        let f = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("temp capture file");
        let _ = std::fs::remove_file(&path); // unlink, keep the fd
        Mutex::new(f)
    })
}

/// Run `f` with fd 1 redirected into a scratch file and return the bytes it
/// wrote. `printf` in both shared objects goes through the process' single
/// glibc `stdout`, so `fflush(NULL)` before/after makes the capture exact.
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::fd::AsRawFd;

    let mut file = cap_file().lock().unwrap_or_else(|e| e.into_inner());

    // Push out anything libtest / Rust has buffered (e.g. the partial
    // "test <name> ... " progress line) so it cannot land in our capture.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }
    file.seek(SeekFrom::Start(0)).unwrap();
    file.set_len(0).unwrap();

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let out = f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }

    file.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    (out, buf)
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5A5A_5A5A_DEAD_BEEF;

pub struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}

impl Rng {
    pub fn new() -> Self {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(s ^ SEED)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    /// uniform in `0..n`
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A finite `f64` drawn from the whole magnitude range.
    pub fn finite_f64(&mut self) -> f64 {
        loop {
            let v = f64::from_bits(self.next_u64());
            if v.is_finite() {
                return v;
            }
        }
    }
    /// A finite `f64` in `(-10^exp, 10^exp)`.
    pub fn scaled_f64(&mut self, exp: i32) -> f64 {
        let m = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        let s = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        s * m * 10f64.powi(exp)
    }
}

// ---------------------------------------------------------------------------
// differential assertions
// ---------------------------------------------------------------------------

/// `time() >> 29` — the only clock-derived quantity the library uses. It
/// changes roughly once every 17 years, so it is effectively constant, but the
/// tests still guard against straddling a tick.
pub fn coarse_now() -> i64 {
    (unsafe { time(std::ptr::null_mut()) }) >> 29
}

/// Compare the return value of the same call made against both libraries.
/// Used for the six functions that perform no I/O (their silence is asserted
/// separately, see `silent_functions_produce_no_output`).
pub fn assert_same<T, F>(ctx: &str, call: F)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(&Api) -> T,
{
    let (c, r) = both();
    assert_pair(ctx, &c, &r, call)
}

/// `assert_same` between an arbitrary pair of loaded libraries.
pub fn assert_pair<T, F>(ctx: &str, c: &Api, r: &Api, mut call: F)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(&Api) -> T,
{
    for attempt in 0..3 {
        let t0 = coarse_now();
        let cv = call(c);
        let rv = call(r);
        let t1 = coarse_now();
        if t0 != t1 && attempt < 2 {
            continue; // coarse-clock boundary straddled; retry
        }
        assert_eq!(cv, rv, "return value mismatch for {ctx}");
        return;
    }
    unreachable!()
}

/// Compare `(return value, stdout bytes)` of the same call made against both
/// libraries and return the agreed-upon pair.
///
/// A disagreement is retried a few times before it is reported: fd 1 is a
/// process-wide resource, so if libtest happens to be running tests in parallel
/// its own progress output can land inside a capture window. Contamination is
/// non-deterministic, a genuine divergence is not.
pub fn same_io<T, F>(ctx: &str, call: F) -> (T, Vec<u8>)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(&Api) -> T,
{
    let (c, r) = both();
    same_io_pair(ctx, &c, &r, call)
}

/// `same_io` between an arbitrary pair of loaded libraries.
pub fn same_io_pair<T, F>(ctx: &str, c: &Api, r: &Api, mut call: F) -> (T, Vec<u8>)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(&Api) -> T,
{
    let mut last: Option<(T, T, Vec<u8>, Vec<u8>)> = None;
    for _ in 0..6 {
        let t0 = coarse_now();
        let (cv, cout) = capture_stdout(|| call(c));
        let (rv, rout) = capture_stdout(|| call(r));
        let t1 = coarse_now();
        if t0 == t1 && cv == rv && cout == rout {
            return (cv, cout);
        }
        last = Some((cv, rv, cout, rout));
    }
    let (cv, rv, cout, rout) = last.unwrap();
    assert_eq!(cv, rv, "return value mismatch for {ctx}");
    assert_eq!(
        cout,
        rout,
        "stdout mismatch for {ctx}\n C   : {}\n RUST: {}",
        show(&cout),
        show(&rout)
    );
    panic!("unstable result for {ctx} (clock boundary or fd-1 contamination)");
}

/// `same_io`, discarding the agreed value.
pub fn assert_same_io<T, F>(ctx: &str, call: F)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut(&Api) -> T,
{
    let _ = same_io(ctx, call);
}

// ---------------------------------------------------------------------------
// subprocess helper for the two fatal (SIGSEGV) rows of ERRORS.md
// ---------------------------------------------------------------------------

/// Re-exec this test binary and run the `#[ignore]`d child test `name`.
/// Returns `(signal, exit_code)`.
pub fn run_child(name: &str, envs: &[(&str, String)]) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = std::process::Command::new(exe);
    cmd.args([name, "--exact", "--ignored", "--test-threads=1", "--nocapture"]);
    cmd.env("DIFFTEST_CHILD", "1");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    // Propagate explicit library overrides so the child looks in the same place.
    if let Ok(v) = std::env::var("C_SO") {
        cmd.env("C_SO", v);
    }
    if let Ok(v) = std::env::var("RUST_SO") {
        cmd.env("RUST_SO", v);
    }
    let st = cmd.status().expect("spawn child test");
    (st.signal(), st.code())
}

/// Skip the body unless we were spawned by `run_child` (guards the crashing
/// child tests from being run directly by a `--include-ignored` sweep).
pub fn is_child() -> bool {
    std::env::var_os("DIFFTEST_CHILD").is_some()
}

// ---------------------------------------------------------------------------
// full-surface randomized battery, usable against any pair of libraries
// ---------------------------------------------------------------------------

/// Exercise every exported entry point of `x` and `y` with randomised inputs
/// and assert byte-identical results (return values and, for `modeselect`,
/// stdout). `n` scales the number of iterations per entry point.
pub fn diff_battery(label: &str, x: &Api, y: &Api, n: usize, seed: u64) {
    use std::ffi::CString;
    let mut rng = Rng::with_seed(seed);

    // classify_mode: the four literals plus random junk
    for lit in ["standard", "enhanced", "turbo", "extreme", "", "STANDARD", "turb"] {
        let cs = CString::new(lit).unwrap();
        let p = cs.as_ptr();
        assert_pair(&format!("{label} classify_mode({lit})"), x, y, |a: &Api| unsafe {
            (a.classify_mode)(p)
        });
    }
    for _ in 0..n {
        let len = (rng.below(24) + 1) as usize;
        let v: Vec<u8> = (0..len).map(|_| (rng.below(255) + 1) as u8).collect();
        let cs = CString::new(v).unwrap();
        let p = cs.as_ptr();
        assert_pair(&format!("{label} classify_mode(random)"), x, y, |a: &Api| unsafe {
            (a.classify_mode)(p)
        });
    }

    // apply_multiplier over the whole int x int space plus the valid window
    for lvl in [-1i32, 0, 1, 2, 3, 4, 5, i32::MIN, i32::MAX] {
        for base in [0xA0, 0, 1, -1, i32::MAX, i32::MIN] {
            assert_pair(
                &format!("{label} apply_multiplier({base},{lvl})"),
                x,
                y,
                |a: &Api| unsafe { (a.apply_multiplier)(base, lvl) },
            );
        }
    }
    for _ in 0..n {
        let base = rng.next_i32();
        let lvl = if rng.next_u64() & 1 == 0 {
            (rng.below(9) as i32) - 2
        } else {
            rng.next_i32()
        };
        assert_pair(
            &format!("{label} apply_multiplier({base},{lvl})"),
            x,
            y,
            |a: &Api| unsafe { (a.apply_multiplier)(base, lvl) },
        );
    }

    // the two double -> int converters
    let mut doubles: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        2147483648.0 / 1e12,
        -2147483648.0 / 1e12,
        2147483648.0 / 1e15,
        -2147483648.0 / 1e15,
        1.0,
        -1.0,
    ];
    for _ in 0..n {
        doubles.push(rng.finite_f64());
        doubles.push(rng.scaled_f64(-3));
        doubles.push(rng.scaled_f64(-6));
        doubles.push(f64::from_bits(rng.next_u64()));
    }
    for v in doubles {
        assert_pair(
            &format!("{label} convert_time_factor({v:e})"),
            x,
            y,
            |a: &Api| unsafe { (a.convert_time_factor)(v) },
        );
        assert_pair(
            &format!("{label} convert_negative_overflow({v:e})"),
            x,
            y,
            |a: &Api| unsafe { (a.convert_negative_overflow)(v) },
        );
    }

    // get_modified_time / hash_time_value
    for (d, h) in [
        (0i32, 0i32),
        (1, 1),
        (-1, -1),
        (100000, 0),
        (0, 1000000),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (24855, 596523),
    ] {
        assert_pair(
            &format!("{label} get_modified_time({d},{h})"),
            x,
            y,
            |a: &Api| unsafe { (a.get_modified_time)(d, h) },
        );
    }
    for _ in 0..n {
        let d = rng.next_i32();
        let h = rng.next_i32();
        assert_pair(
            &format!("{label} get_modified_time({d},{h})"),
            x,
            y,
            |a: &Api| unsafe { (a.get_modified_time)(d, h) },
        );
        let t = rng.next_i64();
        assert_pair(&format!("{label} hash_time_value({t})"), x, y, |a: &Api| unsafe {
            (a.hash_time_value)(t)
        });
    }
    for t in [0i64, -1, 1, i64::MIN, i64::MAX, 0x8080_8080_8080_8080u64 as i64] {
        assert_pair(&format!("{label} hash_time_value({t})"), x, y, |a: &Api| unsafe {
            (a.hash_time_value)(t)
        });
    }

    // modeselect: the full mode x complexity grid plus randomised arguments
    for idx in 0..4i32 {
        for lvl in 0..5i32 {
            let _ = same_io_pair(
                &format!("{label} modeselect({idx},0,{lvl},0)"),
                x,
                y,
                |a: &Api| unsafe { (a.modeselect)(idx, 0, lvl, 0) },
            );
        }
    }
    for _ in 0..n.min(150) {
        let m = (rng.next_u32() & 0x7FFF_FFFF) as i32; // keep m % 4 >= 0
        let t = rng.next_i32();
        let cx = rng.next_i32();
        let s = rng.next_i32();
        let _ = same_io_pair(
            &format!("{label} modeselect({m},{t},{cx},{s})"),
            x,
            y,
            |a: &Api| unsafe { (a.modeselect)(m, t, cx, s) },
        );
    }
    for m in [i32::MIN, -4, -8, 0, 1, 2, 3, 4, i32::MAX - 3, i32::MAX] {
        for cx in [i32::MIN, -1, 0, 1, i32::MAX] {
            let _ = same_io_pair(
                &format!("{label} modeselect({m},0,{cx},0)"),
                x,
                y,
                |a: &Api| unsafe { (a.modeselect)(m, 0, cx, 0) },
            );
        }
    }
}
