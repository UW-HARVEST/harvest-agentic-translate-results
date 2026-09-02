//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! Nothing in here calls the Rust translation directly. Both implementations
//! are reached only through `dlopen` + `dlsym` on their respective shared
//! objects, so the `#[no_mangle] extern "C"` export wrapper is under test too.
//!
//! The only observable behaviour of `void driver(int)` is the bytes it writes
//! to `stdout` via `printf`, so every assertion is a byte-for-byte comparison
//! of captured `stdout`.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits used for capturing whatever the loaded libraries write to fd 1.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn clearerr(stream: *mut c_void);
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    static mut stdout: *mut c_void;
}

const O_WRONLY: c_int = 1;

/// fd 1 is process-global, so captures must never overlap.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Run `f` with fd 1 redirected to a temp file; return everything written.
fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _guard = capture_lock();
    capture_stdout_locked(tag, f)
}

fn capture_stdout_locked<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        tag,
        n
    ));
    let file = File::create(&path).expect("create capture file");

    unsafe {
        // Flush anything already buffered so it lands on the real stdout.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        f();

        // The libraries use buffered stdio; force it out before restoring.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built with cmake on demand.
fn c_so_path() -> PathBuf {
    let root = manifest_dir().parent().expect("workspace root").to_path_buf();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let so = build.join("libdriver.so");
    if !so.exists() {
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build)
            .status()
            .expect("run cmake configure");
        assert!(cfg.success(), "cmake configure failed");
        let bld = Command::new("cmake")
            .arg("--build")
            .arg(".")
            .current_dir(&build)
            .status()
            .expect("run cmake build");
        assert!(bld.success(), "cmake build failed");
    }
    assert!(so.exists(), "missing C shared object at {}", so.display());
    so
}

/// The Rust `cdylib` under test.
///
/// `cargo test` does NOT build the `cdylib` artifact, so the library must be
/// built explicitly (`cargo build` / `cargo build --release`). `run_all.sh`
/// does that and points `DRIVER_RUST_SO` at the exact artifact to verify, so a
/// stale or wrong-profile `.so` can never be silently tested.
fn rust_so_path() -> PathBuf {
    let so = if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.exists(),
            "DRIVER_RUST_SO points at a missing file: {}",
            p.display()
        );
        p
    } else {
        // current_exe is <target>/<profile>/deps/<test>-<hash>; the cdylib is
        // two directories up. Fall back to the usual profile directories.
        let exe = std::env::current_exe().expect("current_exe");
        let mut candidates = Vec::new();
        if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
            candidates.push(profile_dir.join("libdriver.so"));
        }
        let target = manifest_dir().join("target");
        candidates.push(target.join("debug/libdriver.so"));
        candidates.push(target.join("release/libdriver.so"));
        candidates
            .iter()
            .find(|c| c.exists())
            .unwrap_or_else(|| {
                panic!(
                    "missing Rust cdylib; looked in: {candidates:?} \
                     (run `cargo build` first, or set DRIVER_RUST_SO)"
                )
            })
            .clone()
    };

    // Refuse to verify an artifact older than the source it should reflect.
    let src = manifest_dir().join("src/lib.rs");
    let so_mtime = std::fs::metadata(&so).and_then(|m| m.modified()).ok();
    let src_mtime = std::fs::metadata(&src).and_then(|m| m.modified()).ok();
    if let (Some(a), Some(b)) = (so_mtime, src_mtime) {
        assert!(
            a >= b,
            "stale Rust cdylib: {} is older than {} — rebuild before testing",
            so.display(),
            src.display()
        );
    }
    so
}

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C .so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust .so");
        Libs { c, rust }
    })
}

type DriverFn = unsafe extern "C" fn(c_int);
/// Same symbol, deliberately mistyped as taking a 64-bit argument, to probe
/// what the callee does with the upper half of the argument register.
type DriverFn64 = unsafe extern "C" fn(i64);

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("dlsym driver in C .so") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("dlsym driver in Rust .so") }
}

fn c_driver64() -> Symbol<'static, DriverFn64> {
    unsafe { libs().c.get(b"driver\0").expect("dlsym driver in C .so") }
}

fn rust_driver64() -> Symbol<'static, DriverFn64> {
    unsafe { libs().rust.get(b"driver\0").expect("dlsym driver in Rust .so") }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_D1FF_C0FF_EE01;

struct Rng(u64);

impl Rng {
    fn new(stream: u64) -> Self {
        Rng(SEED ^ stream.wrapping_mul(0xA0761D6478BD642F))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn i32_full(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

// ---------------------------------------------------------------------------
// The differential comparison itself.
// ---------------------------------------------------------------------------

/// Feed `values` to both libraries (batched, one capture each) and require the
/// resulting byte streams to be identical. On mismatch, re-run value by value
/// to name the first diverging input.
#[track_caller]
fn assert_same(row: &str, values: &[i32]) {
    assert!(!values.is_empty(), "{row}: no inputs generated");

    let (c_out, rust_out) = {
        let _guard = capture_lock();
        let c_fn = c_driver();
        let r_fn = rust_driver();
        let c_out = capture_stdout_locked("c", || {
            for &v in values {
                unsafe { c_fn(v) };
            }
        });
        let rust_out = capture_stdout_locked("rs", || {
            for &v in values {
                unsafe { r_fn(v) };
            }
        });
        (c_out, rust_out)
    };

    if c_out == rust_out {
        return;
    }

    // Bisect to the first offending input for a useful failure message.
    for &v in values {
        let (c1, r1) = {
            let _guard = capture_lock();
            let c_fn = c_driver();
            let r_fn = rust_driver();
            let c1 = capture_stdout_locked("c1", || unsafe { c_fn(v) });
            let r1 = capture_stdout_locked("r1", || unsafe { r_fn(v) });
            (c1, r1)
        };
        assert_eq!(
            String::from_utf8_lossy(&c1),
            String::from_utf8_lossy(&r1),
            "{row}: divergence for driver({v})"
        );
    }
    panic!(
        "{row}: batched streams differ although every single call matched \
         (C {} bytes, Rust {} bytes)",
        c_out.len(),
        rust_out.len()
    );
}

fn randomized(row: &str, stream: u64, n: usize, lo: i32, hi: i32) {
    let mut rng = Rng::new(stream);
    let values: Vec<i32> = (0..n).map(|_| rng.i32_in(lo, hi)).collect();
    assert_same(row, &values);
}

const N: usize = 512;

// ===========================================================================
// Phase B — valid-path differential tests, one per CONFIGS.md row.
// ===========================================================================

#[test]
fn config_c1_small_positive() {
    randomized("C1", 1, N, 1, 1000);
}

#[test]
fn config_c2_small_negative() {
    randomized("C2", 2, N, -1000, -1);
}

#[test]
fn config_c3_zero() {
    assert_same("C3", &[0]);
}

#[test]
fn config_c4_result_sign_transition() {
    // 2x+300 == 0 at x == -150; ±1 at -149 / -151.
    assert_same("C4", &[-148, -149, -150, -151, -152]);
}

#[test]
fn config_c5_digit_width_sweep() {
    let mut values: Vec<i32> = Vec::new();
    // Hand-picked so that 2x+300 lands on each printed width, both signs,
    // including the widest negative (-2147483648) and INT_MAX's wrap (298).
    for &y in &[
        0i32,
        1,
        -1,
        9,
        -9,
        10,
        -10,
        99,
        -99,
        100,
        -100,
        999,
        -999,
        1000,
        -1000,
        9999,
        -9999,
        10000,
        -10000,
        99999,
        -99999,
        100000,
        -100000,
        999999,
        -999999,
        1000000,
        -1000000,
        9999999,
        -9999999,
        10000000,
        -10000000,
        99999999,
        -99999999,
        100000000,
        -100000000,
        999999999,
        -999999999,
        1000000000,
        -1000000000,
        2147483647,
        -2147483648,
    ] {
        // invert y = 2x+300 where possible; otherwise use nearest reachable x.
        let target = (y as i64) - 300;
        let x = if target % 2 == 0 {
            (target / 2) as i32
        } else {
            ((target - 1) / 2) as i32
        };
        values.push(x);
        values.push(x.wrapping_add(1));
    }
    // Randomized values within each decimal width band as well.
    let mut rng = Rng::new(5);
    for k in 0..10u32 {
        let lo = 10i64.pow(k);
        let hi = (10i64.pow(k + 1) - 1).min(i32::MAX as i64);
        if lo > hi {
            continue;
        }
        for _ in 0..32 {
            let y = rng.i32_in(lo as i32, hi as i32);
            values.push(((y as i64 - 300) / 2) as i32);
            values.push(((-(y as i64) - 300) / 2) as i32);
        }
    }
    assert_same("C5", &values);
}

#[test]
fn config_c6_mid_positive_no_overflow() {
    randomized("C6", 6, N, 1001, 1_073_741_673);
}

#[test]
fn config_c7_add_only_overflow_band() {
    randomized("C7", 7, N, 1_073_741_674, 1_073_741_823);
}

#[test]
fn config_c8_multiply_overflow_positive() {
    randomized("C8", 8, N, 1_073_741_824, i32::MAX);
}

#[test]
fn config_c9_multiply_overflow_negative() {
    randomized("C9", 9, N, i32::MIN, -1_073_741_825);
}

#[test]
fn config_c10_mid_negative_no_overflow() {
    randomized("C10", 10, N, -1_073_741_824, -1001);
}

#[test]
fn config_c11_full_range_uniform() {
    let mut rng = Rng::new(11);
    let values: Vec<i32> = (0..4096).map(|_| rng.i32_full()).collect();
    assert_same("C11", &values);
}

#[test]
fn config_c12_powers_of_two_and_neighbours() {
    let mut values: Vec<i32> = Vec::new();
    for k in 0..32u32 {
        let p = 1i64 << k;
        for delta in [-1i64, 0, 1] {
            for sign in [1i64, -1] {
                let v = sign * p + delta;
                if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                    values.push(v as i32);
                }
            }
        }
    }
    values.push(0);
    values.push(i32::MIN);
    values.push(i32::MAX);
    assert_same("C12", &values);
}

#[test]
fn config_c13_high_bits_of_argument_register() {
    // A caller may pass a 64-bit word; the C callee reads %edi only. The Rust
    // export must ignore the upper half identically.
    let mut rng = Rng::new(13);
    let mut args: Vec<i64> = Vec::new();
    for _ in 0..N {
        let low = rng.i32_full() as u32 as u64;
        let high = rng.next_u64() << 32;
        args.push((high | low) as i64);
    }
    // Plus explicit shapes: all-ones upper half, and "enum" values far out of
    // any valid variant range.
    for low in [0u32, 1, 0xFFFF_FFFF, 0x8000_0000, 300, 0xDEAD_BEEF] {
        args.push(((0xFFFF_FFFFu64 << 32) | low as u64) as i64);
        args.push(((0x0000_0001u64 << 32) | low as u64) as i64);
    }

    let (c_out, rust_out) = {
        let _guard = capture_lock();
        let c_fn = c_driver64();
        let r_fn = rust_driver64();
        let c_out = capture_stdout_locked("c64", || {
            for &a in &args {
                unsafe { c_fn(a) };
            }
        });
        let rust_out = capture_stdout_locked("r64", || {
            for &a in &args {
                unsafe { r_fn(a) };
            }
        });
        (c_out, rust_out)
    };
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
        "C13: divergence when the argument register has non-zero upper bits"
    );
}

#[test]
fn config_c14_long_sequence_same_handle() {
    // Many calls through one dlopen handle: no residual state may accumulate
    // and stdio buffering must behave the same for both.
    let mut rng = Rng::new(14);
    let values: Vec<i32> = (0..8192).map(|_| rng.i32_full()).collect();
    assert_same("C14", &values);
}

#[test]
fn config_c15_interleaved_c_and_rust() {
    let mut rng = Rng::new(15);
    let values: Vec<i32> = (0..N).map(|_| rng.i32_full()).collect();

    let (c_twice, interleaved) = {
        let _guard = capture_lock();
        let c_fn = c_driver();
        let r_fn = rust_driver();
        let c_twice = capture_stdout_locked("cc", || {
            for &v in &values {
                unsafe {
                    c_fn(v);
                    c_fn(v);
                }
            }
        });
        let interleaved = capture_stdout_locked("cr", || {
            for &v in &values {
                unsafe {
                    c_fn(v);
                    r_fn(v);
                }
            }
        });
        (c_twice, interleaved)
    };
    assert_eq!(
        String::from_utf8_lossy(&c_twice),
        String::from_utf8_lossy(&interleaved),
        "C15: interleaved C/Rust output differs from C-only output"
    );
}

// ===========================================================================
// Phase C — error-path differential tests, one per ERRORS.md row.
// ===========================================================================

#[test]
fn error_e1_multiply_overflow_positive() {
    // 2*x overflows for x >= 2^30; C wraps (lea on 32-bit reg), no rejection.
    let mut values = vec![1_073_741_824, 1_073_741_825, 2_000_000_000, i32::MAX];
    let mut rng = Rng::new(101);
    for _ in 0..N {
        values.push(rng.i32_in(1_073_741_824, i32::MAX));
    }
    assert_same("E1", &values);

    // And pin the documented value so a silently-changed convention is caught.
    let out = capture_stdout("e1", || unsafe { c_driver()(1_073_741_824) });
    assert_eq!(out, b"-2147483348\n", "E1: unexpected C reference output");
}

#[test]
fn error_e2_multiply_overflow_negative() {
    let mut values = vec![-1_073_741_825, -1_073_741_826, -2_000_000_000, i32::MIN];
    let mut rng = Rng::new(102);
    for _ in 0..N {
        values.push(rng.i32_in(i32::MIN, -1_073_741_825));
    }
    assert_same("E2", &values);

    let out = capture_stdout("e2", || unsafe { c_driver()(-1_073_741_825) });
    assert_eq!(out, b"-2147483350\n", "E2: unexpected C reference output");
}

#[test]
fn error_e3_add_only_overflow() {
    let mut values = vec![1_073_741_674, 1_073_741_675, 1_073_741_823];
    let mut rng = Rng::new(103);
    for _ in 0..N {
        values.push(rng.i32_in(1_073_741_674, 1_073_741_823));
    }
    assert_same("E3", &values);

    let out = capture_stdout("e3", || unsafe { c_driver()(1_073_741_674) });
    assert_eq!(out, b"-2147483648\n", "E3: unexpected C reference output");
}

#[test]
fn error_e4_int_type_boundaries() {
    assert_same("E4", &[i32::MAX, i32::MIN]);

    let hi = capture_stdout("e4hi", || unsafe { c_driver()(i32::MAX) });
    assert_eq!(hi, b"298\n", "E4: unexpected C output for INT_MAX");
    let lo = capture_stdout("e4lo", || unsafe { c_driver()(i32::MIN) });
    assert_eq!(lo, b"300\n", "E4: unexpected C output for INT_MIN");
}

#[test]
fn error_e5_one_step_past_thresholds() {
    let thresholds = [
        1_073_741_673i32,
        1_073_741_674, // add-only overflow starts
        1_073_741_823,
        1_073_741_824, // multiply overflow (positive) starts
        -1_073_741_824,
        -1_073_741_825, // multiply overflow (negative) starts
        -150,
        -151, // sign flip of the result
    ];
    let mut values: Vec<i32> = Vec::new();
    for &t in &thresholds {
        for d in -2i64..=2 {
            let v = t as i64 + d;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                values.push(v as i32);
            }
        }
    }
    assert_same("E5", &values);
}

#[test]
fn error_e6_garbage_high_bits_and_out_of_range_enum() {
    // Out-of-range enum values: a C enum parameter accepts any int, so pass
    // values with no plausible variant, both as clean ints and as 64-bit words
    // with junk in the upper half.
    let odd = [
        -1i32,
        i32::MIN,
        i32::MAX,
        0x7FFF_FFFE,
        -12345,
        999_999_999,
        0x5555_5555,
        -0x5555_5555,
    ];
    assert_same("E6-int", &odd);

    let mut args: Vec<i64> = Vec::new();
    for &v in &odd {
        args.push(((0xDEAD_BEEFu64 << 32) | v as u32 as u64) as i64);
        args.push(((0xFFFF_FFFFu64 << 32) | v as u32 as u64) as i64);
    }
    let (c_out, rust_out) = {
        let _guard = capture_lock();
        let c_fn = c_driver64();
        let r_fn = rust_driver64();
        let c_out = capture_stdout_locked("e6c", || {
            for &a in &args {
                unsafe { c_fn(a) };
            }
        });
        let rust_out = capture_stdout_locked("e6r", || {
            for &a in &args {
                unsafe { r_fn(a) };
            }
        });
        (c_out, rust_out)
    };
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
        "E6: divergence for out-of-range / junk-high-bit arguments"
    );
}

#[test]
fn error_e7_printf_failure_ignored() {
    // Redirect fd 1 to /dev/full so the eventual write fails with ENOSPC.
    // Both libraries must ignore printf's return value, return normally, and
    // report the failure identically (same fflush result / same errno).
    let _guard = capture_lock();

    let run = |f: &dyn Fn()| -> (c_int, c_int) {
        unsafe {
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0);
            let devfull = open(c"/dev/full".as_ptr(), O_WRONLY);
            assert!(devfull >= 0, "open /dev/full failed");
            assert!(dup2(devfull, 1) >= 0);

            f(); // must not abort, must return normally

            let flush_rc = fflush(stdout);
            let err = *__errno_location();

            clearerr(stdout);
            assert!(dup2(saved, 1) >= 0);
            close(saved);
            close(devfull);
            (flush_rc, err)
        }
    };

    let c_fn = c_driver();
    let r_fn = rust_driver();
    let (c_rc, c_err) = run(&|| {
        unsafe { c_fn(7) };
    });
    let (r_rc, r_err) = run(&|| {
        unsafe { r_fn(7) };
    });

    // Reaching this line at all proves both calls returned normally rather
    // than aborting on the failed write.
    assert_eq!(c_rc, r_rc, "E7: fflush status differs (C {c_rc} vs Rust {r_rc})");
    assert_eq!(
        c_err, r_err,
        "E7: errno after failed write differs (C {c_err} vs Rust {r_err})"
    );
    // Sanity: the write really did fail, i.e. the test is meaningful.
    assert_eq!(c_rc, -1, "E7: expected fflush to report failure on /dev/full");
}

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
}

// ===========================================================================
// Phase D — symbol parity enforced as a test.
// ===========================================================================

fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn phase_d_symbol_parity() {
    let c = defined_dynamic_symbols(&c_so_path());
    let rust = defined_dynamic_symbols(&rust_so_path());
    assert!(
        c.contains(&"driver".to_string()),
        "C .so unexpectedly does not export `driver`: {c:?}"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !rust.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
}
