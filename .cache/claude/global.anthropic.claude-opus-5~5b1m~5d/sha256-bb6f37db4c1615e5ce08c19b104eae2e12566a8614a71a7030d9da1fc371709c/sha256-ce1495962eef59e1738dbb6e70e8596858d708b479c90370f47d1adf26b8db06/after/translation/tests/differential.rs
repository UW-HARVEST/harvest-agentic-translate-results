//! Differential tests: C `.so` vs Rust `.so`, both loaded through `libloading`.
//!
//! Nothing in this file calls the Rust implementation directly — the Rust code is
//! reached only through `dlopen`/`dlsym` on `target/{debug,release}/libdriver.so`,
//! exactly as an external C consumer would, so the `#[no_mangle] extern "C"`
//! export wrapper is under test too.
//!
//! `driver`'s only observable effect is the bytes it writes to `stdout` via
//! `printf`, so each call is wrapped in an fd-1 redirection into a temp file and
//! the captured bytes are compared byte-for-byte.

use std::ffi::c_void;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed for stdout capture (linked by the test binary itself).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    /// `fflush(NULL)` flushes every open output stream, which avoids needing the
    /// `stdout` FILE* symbol. Both `.so`s share the process's single glibc
    /// `stdout`, so this flushes whichever of them just ran.
    fn fflush(stream: *mut c_void) -> i32;
}

type DriverFn = unsafe extern "C" fn(i32);

// ---------------------------------------------------------------------------
// Locating the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// All Rust `.so` artifacts that exist (debug and/or release) — CONFIGS.md row 15.
/// The artifact for the profile the tests were built under always exists; the
/// other is included when it has been built.
fn rust_so_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();

    // target/<profile>/deps/<test-bin> -> target/<profile>/libdriver.so
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
            let p = profile_dir.join("libdriver.so");
            if p.exists() {
                out.push(p);
            }
        }
    }
    for profile in ["debug", "release"] {
        let p = manifest_dir().join("target").join(profile).join("libdriver.so");
        if p.exists() && !out.iter().any(|q| q == &p) {
            out.push(p);
        }
    }

    assert!(
        !out.is_empty(),
        "no Rust libdriver.so found under target/; run `cargo build` and `cargo build --release`"
    );
    out
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so only one capture may be in flight at a time even
/// though the test harness runs tests on several threads.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// fd-1 capture is only sound if libtest is not concurrently writing its own
/// progress output ("test foo ... ok") to fd 1 from another thread. `.cargo/config.toml`
/// pins `RUST_TEST_THREADS=1`; fail loudly rather than silently mis-compare if
/// the suite is ever run without it.
fn require_single_threaded() {
    static CHECKED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    CHECKED.get_or_init(|| {
        let v = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
        assert_eq!(
            v, "1",
            "these differential tests redirect the process-global fd 1 and must run \
             single-threaded. Run them via `cargo test` from the crate root (which applies \
             .cargo/config.toml's RUST_TEST_THREADS=1) or pass `--test-threads=1` with \
             RUST_TEST_THREADS=1 set."
        );
    });
}

/// Runs `f` with fd 1 redirected into a temp file and returns the exact bytes
/// written. Used identically for the C and the Rust library so the comparison is
/// symmetric.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    require_single_threaded();
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Flush anything already buffered so it is not attributed to `f`.
        fflush(std::ptr::null_mut());

        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "driver_capture_{}_{:?}.txt",
            std::process::id(),
            std::thread::current().id()
        ));
        let file = std::fs::File::create(&path).expect("create capture file");

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 into fd 1 failed");

        f();

        // Push the library's buffered bytes out to the redirected fd.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
        drop(file);

        let mut buf = Vec::new();
        std::fs::File::open(&path)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        let _ = std::fs::remove_file(&path);
        buf
    }
}

// ---------------------------------------------------------------------------
// The differential harness
// ---------------------------------------------------------------------------

struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: DriverFn,
    rust: DriverFn,
    rust_path: PathBuf,
}

impl Pair {
    fn load(rust_path: &Path) -> Pair {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("dlopen C libdriver.so");
            let rust_lib = Library::new(rust_path).expect("dlopen Rust libdriver.so");
            let c: Symbol<DriverFn> = c_lib.get(b"driver\0").expect("dlsym driver in C .so");
            let rust: Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("dlsym driver in Rust .so (is #[no_mangle] export present?)");
            let (c, rust) = (*c, *rust);
            Pair { _c_lib: c_lib, _rust_lib: rust_lib, c, rust, rust_path: rust_path.to_path_buf() }
        }
    }

    /// One differential call: assert C and Rust emit identical bytes for `x`.
    fn check(&self, row: &str, x: i32) {
        let c_out = capture_stdout(|| unsafe { (self.c)(x) });
        let rust_out = capture_stdout(|| unsafe { (self.rust)(x) });
        assert_eq!(
            c_out,
            rust_out,
            "[{}] divergence for driver({x}) using {}\n  C   : {:?}\n  Rust: {:?}",
            row,
            self.rust_path.display(),
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out),
        );
        // The output must be non-empty for every input: `driver` always prints.
        assert!(!c_out.is_empty(), "[{row}] C produced no output for driver({x})");
    }

    /// Differential check of a whole *sequence* of calls without an intervening
    /// flush (CONFIGS.md row 14): compares the accumulated stream.
    fn check_sequence(&self, row: &str, xs: &[i32]) {
        let c_out = capture_stdout(|| {
            for &x in xs {
                unsafe { (self.c)(x) }
            }
        });
        let rust_out = capture_stdout(|| {
            for &x in xs {
                unsafe { (self.rust)(x) }
            }
        });
        assert_eq!(
            c_out,
            rust_out,
            "[{}] sequence divergence over {} calls using {}\n  C   : {:?}\n  Rust: {:?}",
            row,
            xs.len(),
            self.rust_path.display(),
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out),
        );
    }
}

/// Runs `body` once per available Rust artifact (debug and release) — row 15.
fn for_each_artifact<F: FnMut(&Pair)>(mut body: F) {
    for p in rust_so_paths() {
        let pair = Pair::load(&p);
        body(&pair);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible runs, no extra dependency).
// ---------------------------------------------------------------------------
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi` (inclusive), safe for the full i32 range.
    fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

const SEED: u64 = 0xC0FFEE_1234_5678;

// ===========================================================================
// Phase B — valid-path differential tests, one test per CONFIGS.md row group
// ===========================================================================

#[test]
fn row01_zero() {
    for_each_artifact(|p| p.check("row 1: x = 0", 0));
}

#[test]
fn row02_small_positive() {
    for_each_artifact(|p| {
        let mut rng = Rng::new(SEED);
        p.check("row 2: small positive", 1);
        p.check("row 2: small positive", 1000);
        for _ in 0..500 {
            p.check("row 2: small positive", rng.in_range(1, 1_000));
        }
    });
}

#[test]
fn row03_small_negative() {
    for_each_artifact(|p| {
        let mut rng = Rng::new(SEED ^ 3);
        p.check("row 3: small negative", -1);
        p.check("row 3: small negative", -1000);
        for _ in 0..500 {
            p.check("row 3: small negative", rng.in_range(-1_000, -1));
        }
    });
}

#[test]
fn row04_result_exactly_zero() {
    for_each_artifact(|p| {
        p.check("row 4: result exactly zero", -150);
        // Confirm the shared expectation about the C output too.
        let out = capture_stdout(|| unsafe { (p.c)(-150) });
        assert_eq!(out, b"0\n", "C driver(-150) should print \"0\\n\"");
    });
}

#[test]
fn row05_zero_crossing_sweep() {
    for_each_artifact(|p| {
        for x in -160..=-140 {
            p.check("row 5: zero-crossing sweep", x);
        }
    });
}

#[test]
fn row06_positive_no_overflow() {
    for_each_artifact(|p| {
        let mut rng = Rng::new(SEED ^ 6);
        for _ in 0..1_000 {
            p.check("row 6: positive, no overflow", rng.in_range(0, i32::MAX / 2));
        }
    });
}

#[test]
fn row07_negative_no_overflow() {
    for_each_artifact(|p| {
        let mut rng = Rng::new(SEED ^ 7);
        for _ in 0..1_000 {
            p.check("row 7: negative, no overflow", rng.in_range(i32::MIN / 2, -1));
        }
    });
}

#[test]
fn row08_multiply_overflow_positive() {
    for_each_artifact(|p| {
        let mut rng = Rng::new(SEED ^ 8);
        for _ in 0..1_000 {
            p.check(
                "row 8: 2*x overflows (positive x)",
                rng.in_range(i32::MAX / 2 + 1, i32::MAX),
            );
        }
    });
}

#[test]
fn row09_multiply_overflow_negative() {
    for_each_artifact(|p| {
        let mut rng = Rng::new(SEED ^ 9);
        for _ in 0..1_000 {
            p.check(
                "row 9: 2*x overflows (negative x)",
                rng.in_range(i32::MIN, i32::MIN / 2 - 1),
            );
        }
    });
}

#[test]
fn row10_add_only_overflow_band() {
    // (INT_MAX - 300) / 2 == 1_073_741_673 is the last x where `y += 300` fits.
    for_each_artifact(|p| {
        for x in 1_073_741_670..=1_073_741_680 {
            p.check("row 10: only the += 300 overflows", x);
        }
        // and the mirrored band at the negative end
        for x in -1_073_741_680..=-1_073_741_670 {
            p.check("row 10: negative band", x);
        }
    });
}

#[test]
fn row11_extreme_boundary_constants() {
    let xs = [
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        1,
        -1,
        2,
        -2,
        -149,
        -151,
        -150,
        0,
        1_073_741_673,
        1_073_741_674,
    ];
    for_each_artifact(|p| {
        for &x in &xs {
            p.check("row 11: extreme boundary constants", x);
        }
    });
}

#[test]
fn row12_output_digit_widths() {
    // x values chosen so that 2*x + 300 has 1..10 digits, both signs.
    let mut xs = Vec::new();
    for (target, _digits) in [
        (0i64, 1),
        (7, 1),
        (42, 2),
        (999, 3),
        (1_000, 4),
        (12_345, 5),
        (100_000, 6),
        (9_999_999, 7),
        (10_000_000, 8),
        (999_999_999, 9),
        (1_000_000_000, 10),
        (2_147_483_647, 10),
        (-1, 1),
        (-42, 2),
        (-999, 3),
        (-123_456, 6),
        (-1_000_000_000, 10),
        (-2_147_483_648, 10),
    ] {
        // Solve 2*x + 300 == target where the parity allows; otherwise use
        // target-1 so we still land on the same digit width.
        let t = if (target - 300) % 2 == 0 { target } else { target - 1 };
        let x = (t - 300) / 2;
        xs.push(x as i32);
        // Also push the raw wrapped value so widths after wraparound are covered.
        xs.push(target as i32);
    }
    for_each_artifact(|p| {
        for &x in &xs {
            p.check("row 12: output digit widths", x);
        }
    });
}

#[test]
fn row13_randomized_full_range() {
    for_each_artifact(|p| {
        let mut rng = Rng::new(SEED ^ 13);
        for _ in 0..20_000 {
            p.check("row 13: full-range random", rng.next_i32());
        }
    });
}

#[test]
fn row14_repeated_calls_shared_stream() {
    let mut rng = Rng::new(SEED ^ 14);
    let xs: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    for_each_artifact(|p| {
        p.check_sequence("row 14: 256 calls, one flush", &xs);
        // Interleaved order must not matter either.
        p.check_sequence("row 14: boundary sequence", &[i32::MIN, 0, i32::MAX, -150, 1, -1]);
    });
}

#[test]
fn row15_both_artifacts_present_and_agree() {
    let paths = rust_so_paths();
    // Every artifact found must agree with C on a shared spot-check set.
    for p in &paths {
        let pair = Pair::load(p);
        for x in [0, 1, -1, i32::MAX, i32::MIN, -150, 1_073_741_674] {
            pair.check("row 15: per-artifact agreement", x);
        }
    }
    eprintln!("row 15: verified Rust artifacts: {paths:?}");
}

// ===========================================================================
// Phase C — error-path / boundary differential tests (ERRORS.md)
// ===========================================================================

/// ERRORS.md rows G5–G10. The C code has *no* rejection sites, so the whole
/// error surface reduces to the generic boundary classes: the extremes of the
/// domain, the two overflow boundaries, and the zero/sign-flip point. Rows
/// G1–G4 (null pointers, lengths, out-of-range enums) do not exist in this
/// ABI — `driver`'s only parameter is a by-value `int`, and every one of its
/// 2^32 bit patterns is a valid input, which this test asserts by construction.
#[test]
fn boundary_and_error_surface() {
    let cases: &[(&str, i32)] = &[
        ("G5  zero value", 0),
        ("G6  INT_MAX (2*x overflows)", i32::MAX),
        ("G6  INT_MAX-1", i32::MAX - 1),
        ("G7  INT_MIN (2*x overflows negatively)", i32::MIN),
        ("G7  INT_MIN+1", i32::MIN + 1),
        ("G8  last x where += 300 fits", 1_073_741_673),
        ("G8  first x where += 300 overflows", 1_073_741_674),
        ("G9  INT_MAX/2", i32::MAX / 2),
        ("G9  INT_MAX/2 + 1", i32::MAX / 2 + 1),
        ("G9  INT_MIN/2", i32::MIN / 2),
        ("G9  INT_MIN/2 - 1", i32::MIN / 2 - 1),
        ("G10 result exactly zero", -150),
        ("G10 result +2", -149),
        ("G10 result -2", -151),
    ];
    for_each_artifact(|p| {
        for (row, x) in cases {
            p.check(row, *x);
        }
    });
}

/// G11: no input is rejected and none aborts. Sweep a large, structured set of
/// bit patterns (all single-bit values, all single-bit complements, and the
/// neighbourhood of every power of two) through both libraries. If the Rust side
/// panicked or aborted on any of them the test process would die, and if it
/// diverged the byte comparison would fail.
#[test]
fn total_over_domain_no_rejection() {
    let mut xs: Vec<i32> = Vec::new();
    for bit in 0..32u32 {
        let v = 1i64 << bit;
        xs.push(v as i32);
        xs.push(!(v as i32));
        xs.push((v as i32).wrapping_neg());
        xs.push((v as i32).wrapping_sub(1));
        xs.push((v as i32).wrapping_add(1));
    }
    xs.extend_from_slice(&[0, -1, i32::MAX, i32::MIN]);
    for_each_artifact(|p| {
        for &x in &xs {
            p.check("G11 total over domain", x);
        }
        // Also as one uninterrupted stream.
        p.check_sequence("G11 total over domain (stream)", &xs);
    });
}

/// Sanity check that the harness itself is trustworthy: the capture mechanism
/// must observe the exact bytes `printf("%d\n", ...)` produces, and it must be
/// able to detect a difference (guards against a vacuously-passing comparison).
#[test]
fn harness_self_check() {
    for_each_artifact(|p| {
        let a = capture_stdout(|| unsafe { (p.c)(0) });
        assert_eq!(a, b"300\n", "capture harness did not observe C output for driver(0)");
        let b = capture_stdout(|| unsafe { (p.rust)(0) });
        assert_eq!(b, b"300\n", "capture harness did not observe Rust output for driver(0)");
        // Different inputs must give different captures, else capture is stale.
        let c = capture_stdout(|| unsafe { (p.c)(1) });
        assert_ne!(a, c, "capture harness appears to return stale data");
    });
}

/// Symbol-parity assertion executed from inside the test suite (Phase D):
/// every symbol the C `.so` exports must be resolvable in the Rust `.so`.
#[test]
fn symbol_parity_driver_resolvable() {
    for path in rust_so_paths() {
        unsafe {
            let lib = Library::new(&path).expect("dlopen Rust .so");
            let sym: Result<Symbol<DriverFn>, _> = lib.get(b"driver\0");
            assert!(
                sym.is_ok(),
                "Rust .so {} does not export `driver`, which the C .so exports",
                path.display()
            );
        }
    }
}
