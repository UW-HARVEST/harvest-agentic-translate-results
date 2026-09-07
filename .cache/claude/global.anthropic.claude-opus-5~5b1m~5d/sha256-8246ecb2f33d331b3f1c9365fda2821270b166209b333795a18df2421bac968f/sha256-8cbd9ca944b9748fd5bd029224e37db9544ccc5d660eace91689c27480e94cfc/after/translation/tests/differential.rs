//! Differential tests: the C `libdriver.so` vs. the Rust `libdriver.so`.
//!
//! BOTH libraries are loaded with `libloading` and every call goes through the
//! exported `driver` symbol of the respective shared object. The Rust
//! translation is never called directly as a Rust function, so the
//! `#[no_mangle] extern "C"` wrapper is exercised too.
//!
//! `driver` returns `void` and its only observable effect is what libc `printf`
//! writes to file descriptor 1, so each call is made with fd 1 redirected to a
//! temporary file and the resulting bytes are compared exactly.

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits used only by the test harness (never the code under test)
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(c_int);

struct Libs {
    c: Library,
    r: Library,
}

// fd 1 is process-global; serialize every capture.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the profile this test binary itself was built with, but fall back
    // to whichever cdylib actually exists.
    let base = manifest_dir().join("target");
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for p in preferred {
        let cand = base.join(p).join("libdriver.so");
        if cand.exists() {
            return cand;
        }
    }
    base.join("debug/libdriver.so")
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "C shared library not found at {cp:?} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rp.exists(),
            "Rust shared library not found at {rp:?} — build it with `cargo build` / `cargo build --release`"
        );
        unsafe {
            Libs {
                c: Library::new(&cp).expect("load C .so"),
                r: Library::new(&rp).expect("load Rust .so"),
            }
        }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C .so exports `driver`") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().r.get(b"driver\0").expect("Rust .so exports `driver`") }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Restores fd 1 even if the closure under capture panics.
struct FdGuard(c_int);

impl Drop for FdGuard {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.0, 1);
            close(self.0);
        }
    }
}

fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}.txt",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let file = File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    // Make sure nothing already buffered lands in the capture file.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    {
        let _restore = FdGuard(saved);
        assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 failed");
        f();
        // `_restore` flushes the libc stdout buffer that `printf` wrote into and
        // puts the original fd 1 back.
    }
    drop(file);

    let data = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    data
}

/// Run `driver(x)` in both libraries and require byte-identical stdout.
#[track_caller]
fn check(x: c_int) {
    let cf = c_driver();
    let rf = rust_driver();
    let c_out = capture(|| unsafe { cf(x) });
    let r_out = capture(|| unsafe { rf(x) });
    assert_eq!(
        c_out,
        r_out,
        "divergence for x={x}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert!(!c_out.is_empty(), "C produced no output for x={x}");
}

#[track_caller]
fn check_all(xs: &[c_int]) {
    for &x in xs {
        check(x);
    }
}

/// Many calls inside a single capture region (no intervening fflush), so the
/// libc stdout buffer accumulates several results before being compared.
#[track_caller]
fn check_batched(xs: &[c_int]) {
    let cf = c_driver();
    let rf = rust_driver();
    let c_out = capture(|| {
        for &x in xs {
            unsafe { cf(x) }
        }
    });
    let r_out = capture(|| {
        for &x in xs {
            unsafe { rf(x) }
        }
    });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "batched divergence for {} inputs starting at {:?}",
        xs.len(),
        xs.first()
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
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
    fn i32_any(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in the inclusive range `[lo, hi]`.
    fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

const N_RANDOM: usize = 200;

fn random_in(seed: u64, lo: i32, hi: i32, n: usize) -> Vec<i32> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rng.in_range(lo, hi)).collect()
}

// Boundaries derived from the C arithmetic `y = 2*x; y += 300`.
// Largest x with 2*x+300 <= INT_MAX. Note 2*x+300 is always even, so it can
// never equal INT_MAX (odd); the maximum attainable result is INT_MAX-1.
const LAST_OK: i32 = 1_073_741_673; // 2*x+300 == 2147483646 == INT_MAX-1
const HALF_MAX: i32 = i32::MAX / 2; // 1_073_741_823
const HALF_MIN: i32 = i32::MIN / 2; // -1_073_741_824

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

fn cfg01_zero() {
    check(0);
    // Sanity: pin the C output itself so we know what we are comparing against.
    let cf = c_driver();
    assert_eq!(capture(|| unsafe { cf(0) }), b"300\n");
}

fn cfg02_small_positive() {
    check_all(&(1..=1000).collect::<Vec<i32>>());
}

fn cfg03_small_negative() {
    check_all(&(-1000..=-1).collect::<Vec<i32>>());
}

fn cfg04_root_prints_zero() {
    check(-150);
    let cf = c_driver();
    assert_eq!(capture(|| unsafe { cf(-150) }), b"0\n");
}

fn cfg05_negative_x_positive_result() {
    check_all(&(-149..=-1).collect::<Vec<i32>>());
}

fn cfg06_negative_x_negative_result() {
    check_all(&(-1000..=-151).collect::<Vec<i32>>());
}

fn cfg07_mid_positive_no_overflow() {
    check_all(&random_in(0xC0FFEE, 1001, LAST_OK - 1, N_RANDOM));
}

fn cfg08_mid_negative_no_overflow() {
    check_all(&random_in(0xBEEF01, -LAST_OK, -1001, N_RANDOM));
}

fn cfg09_last_non_overflowing_inputs() {
    check_all(&[LAST_OK - 1, LAST_OK]);
    let cf = c_driver();
    assert_eq!(capture(|| unsafe { cf(LAST_OK) }), b"2147483646\n");
    // One step past: 2*x+300 overflows by exactly 2 -> wraps to INT_MIN+1.
    assert_eq!(capture(|| unsafe { cf(LAST_OK + 1) }), b"-2147483648\n");
}

fn cfg10_add_overflows_only() {
    // 2*x fits, but 2*x+300 overflows.
    let mut xs: Vec<i32> = (LAST_OK + 1..=LAST_OK + 40).collect();
    xs.extend(HALF_MAX - 5..=HALF_MAX);
    xs.extend(random_in(0x10DEAD, LAST_OK + 1, HALF_MAX, N_RANDOM));
    check_all(&xs);
}

fn cfg11_mul_overflows_positive() {
    let mut xs = vec![HALF_MAX + 1, HALF_MAX + 2, i32::MAX - 1, i32::MAX];
    xs.extend(random_in(0x11DEAD, HALF_MAX + 1, i32::MAX, N_RANDOM));
    check_all(&xs);
}

fn cfg12_negative_limit_inside() {
    let mut xs = vec![HALF_MIN, HALF_MIN + 1, -LAST_OK - 1];
    xs.extend(random_in(0x12DEAD, HALF_MIN, -LAST_OK - 1, N_RANDOM));
    check_all(&xs);
}

fn cfg13_mul_overflows_negative() {
    let mut xs = vec![HALF_MIN - 1, HALF_MIN - 2, i32::MIN + 1, i32::MIN];
    xs.extend(random_in(0x13DEAD, i32::MIN, HALF_MIN - 1, N_RANDOM));
    check_all(&xs);
}

fn cfg14_full_range_random() {
    let mut rng = Rng::new(0xA5A5_5A5A);
    let xs: Vec<i32> = (0..2000).map(|_| rng.i32_any()).collect();
    check_batched(&xs);
    // Also compare a subset call-by-call (independent captures).
    check_all(&xs[..100]);
}

fn cfg15_result_width_sweep() {
    // Sweep printed widths 1..=10 digits, both signs, via exact x values.
    let mut xs = Vec::new();
    let mut w: i64 = 1;
    for _ in 0..10 {
        for target in [w, -w, w * 9, -(w * 9)] {
            // y = 2*x + 300 == target  =>  x = (target - 300) / 2 when even
            let t = target - 300;
            if t % 2 == 0 {
                let x = t / 2;
                if x >= i32::MIN as i64 && x <= i32::MAX as i64 {
                    xs.push(x as i32);
                }
            }
            let t2 = target - 1 - 300;
            if t2 % 2 == 0 {
                let x = t2 / 2;
                if x >= i32::MIN as i64 && x <= i32::MAX as i64 {
                    xs.push(x as i32);
                }
            }
        }
        w *= 10;
    }
    xs.sort_unstable();
    xs.dedup();
    check_all(&xs);
}

fn cfg16_repeated_and_interleaved_calls() {
    let cf = c_driver();
    let rf = rust_driver();

    // Same value many times.
    for _ in 0..20 {
        check(12345);
    }

    // Interleaved C/Rust sequence with varying values: the concatenated output
    // of a C-only run must equal the concatenated output of a Rust-only run,
    // and an interleaved run must equal both halves.
    let xs: Vec<i32> = random_in(0x1616, i32::MIN, i32::MAX, 50);
    let c_out = capture(|| {
        for &x in &xs {
            unsafe { cf(x) }
        }
    });
    let r_out = capture(|| {
        for &x in &xs {
            unsafe { rf(x) }
        }
    });
    assert_eq!(c_out, r_out);

    let interleaved = capture(|| {
        for &x in &xs {
            unsafe { cf(x) };
            unsafe { rf(x) };
        }
    });
    // Each value appears twice, identically.
    let lines: Vec<&[u8]> = interleaved.split(|&b| b == b'\n').collect();
    for pair in lines.chunks(2) {
        if pair.len() == 2 {
            assert_eq!(pair[0], pair[1], "interleaved C/Rust pair differs");
        }
    }
}

fn cfg17_buffered_accumulation() {
    // Enough calls to exceed a 4 KiB stdio buffer while redirected.
    let xs: Vec<i32> = random_in(0x1717, i32::MIN, i32::MAX, 1000);
    check_batched(&xs);
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

fn errors_no_rejection_path_is_reachable() {
    // Row 1: the C source contains no return/assert/range check, so no input is
    // rejected. Demonstrated by requiring non-empty, identical output for the
    // most extreme arguments constructible.
    for x in [0, 1, -1, i32::MAX, i32::MIN, -150] {
        let cf = c_driver();
        let out = capture(|| unsafe { cf(x) });
        assert!(out.ends_with(b"\n"), "C rejected x={x}?");
        check(x);
    }
}

fn errors_boundary_and_extremal_inputs() {
    // ERRORS.md rows 2..=10
    check_all(&[
        0,             // row 2
        i32::MAX,      // row 3
        i32::MIN,      // row 4
        HALF_MAX,      // row 5
        LAST_OK,       // row 6
        LAST_OK + 1,   // row 7
        HALF_MIN,      // row 8
        HALF_MIN - 1,  // row 9
        -150,          // row 10
    ]);
}

fn errors_full_int_bit_patterns() {
    // Row 11: `int` has no invalid variant; every bit pattern, including the
    // ones an out-of-range enum value would produce, is a legal argument.
    let patterns: Vec<i32> = [
        0x0000_0000u32,
        0x0000_0001,
        0x0000_00FF,
        0x0000_7FFF,
        0x0000_8000,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFF,
        0xFFFF_FF9C, // -100
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0xAAAA_AAAA,
        0x5555_5555,
    ]
    .iter()
    .map(|&u| u as i32)
    .collect();
    check_all(&patterns);
    check_batched(&patterns);
}

fn errors_repeated_calls_have_no_hidden_state() {
    // Row 12: no init required, no state carried between calls. The same value
    // must print the same bytes on the 1st and the 500th call, in both libs.
    let cf = c_driver();
    let rf = rust_driver();
    let first_c = capture(|| unsafe { cf(7) });
    let first_r = capture(|| unsafe { rf(7) });
    let noise: Vec<i32> = random_in(0xF00D, i32::MIN, i32::MAX, 500);
    capture(|| {
        for &x in &noise {
            unsafe { cf(x) };
            unsafe { rf(x) };
        }
    });
    let last_c = capture(|| unsafe { cf(7) });
    let last_r = capture(|| unsafe { rf(7) });
    assert_eq!(first_c, last_c);
    assert_eq!(first_r, last_r);
    assert_eq!(first_c, first_r);
}

// ===========================================================================
// Phase D — symbol parity through the FFI boundary
// ===========================================================================

fn symbols_rust_so_exports_every_c_symbol() {
    // Every symbol the C .so exports must be resolvable in the Rust .so.
    let l = libs();
    for name in [b"driver\0".as_slice()] {
        unsafe {
            l.c.get::<DriverFn>(name)
                .unwrap_or_else(|e| panic!("C .so missing {name:?}: {e}"));
            l.r.get::<DriverFn>(name)
                .unwrap_or_else(|e| panic!("Rust .so missing {name:?}: {e}"));
        }
    }
}


/// ERRORS.md rows 2..=10: assert the EXACT documented bytes, from BOTH .so's.
/// (Stronger than "C == Rust": pins the ground-truth output too.)
fn errors_expected_exact_bytes() {
    let table: &[(c_int, &str)] = &[
        (0, "300\n"),                       // row 2
        (i32::MAX, "298\n"),                // row 3
        (i32::MIN, "300\n"),                // row 4
        (HALF_MAX, "-2147483350\n"),        // row 5
        (LAST_OK, "2147483646\n"),          // row 6
        (LAST_OK + 1, "-2147483648\n"),     // row 7
        (HALF_MIN, "-2147483348\n"),        // row 8
        (HALF_MIN - 1, "-2147483350\n"),    // row 9
        (-150, "0\n"),                      // row 10
    ];
    let cf = c_driver();
    let rf = rust_driver();
    for &(x, want) in table {
        let c_out = capture(|| unsafe { cf(x) });
        let r_out = capture(|| unsafe { rf(x) });
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            want,
            "C ground truth changed for x={x}"
        );
        assert_eq!(
            String::from_utf8_lossy(&r_out),
            want,
            "Rust diverges from documented C output for x={x}"
        );
    }
}

/// Wide sweep: strided walk over the entire i32 range plus dense windows around
/// every boundary. Streams in chunks so no huge Vec is materialised.
/// Set `DRIVER_SOAK=0` to skip, or `DRIVER_SOAK=<stride>` (default 512).
fn soak_wide_sweep() {
    let stride: i64 = match std::env::var("DRIVER_SOAK").as_deref() {
        Ok("0") => {
            eprintln!("(soak skipped)");
            return;
        }
        Ok(v) => v.parse().unwrap_or(512),
        Err(_) => 512,
    };
    assert!(stride >= 1);

    const CHUNK: usize = 8192;
    let mut buf: Vec<i32> = Vec::with_capacity(CHUNK);
    let mut total: u64 = 0;

    let flush = |buf: &mut Vec<i32>| {
        if !buf.is_empty() {
            check_batched(buf);
            buf.clear();
        }
    };

    // Strided walk over the whole i32 range (or the sub-range given by
    // DRIVER_SOAK_LO / DRIVER_SOAK_HI, used to shard an exhaustive run).
    let lo: i64 = std::env::var("DRIVER_SOAK_LO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(i32::MIN as i64);
    let hi: i64 = std::env::var("DRIVER_SOAK_HI")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(i32::MAX as i64);
    assert!(lo >= i32::MIN as i64 && hi <= i32::MAX as i64 && lo <= hi);
    let mut x = lo;
    while x <= hi {
        buf.push(x as i32);
        total += 1;
        if buf.len() == CHUNK {
            flush(&mut buf);
        }
        x += stride;
    }
    flush(&mut buf);

    // Dense windows (every single value) around every boundary the arithmetic
    // distinguishes.
    for centre in [
        0i64,
        -150,
        LAST_OK as i64,
        HALF_MAX as i64,
        HALF_MIN as i64,
        i32::MAX as i64,
        i32::MIN as i64,
        -(LAST_OK as i64),
    ] {
        for d in -2048..=2048i64 {
            let v = centre + d;
            if (i32::MIN as i64..=i32::MAX as i64).contains(&v) {
                buf.push(v as i32);
                total += 1;
                if buf.len() == CHUNK {
                    flush(&mut buf);
                }
            }
        }
        flush(&mut buf);
    }
    eprintln!("(soak: {total} values compared, stride {stride}) ");
}

// ===========================================================================
// Sequential harness (harness = false)
//
// These tests redirect the process-wide fd 1, so they cannot run concurrently
// with libtest's own progress printing; hence a custom, strictly sequential
// runner.
// ===========================================================================

fn all_tests() -> Vec<(&'static str, fn())> {
    vec![
        // Phase B — CONFIGS.md rows 1..=17
        ("cfg01_zero", cfg01_zero as fn()),
        ("cfg02_small_positive", cfg02_small_positive),
        ("cfg03_small_negative", cfg03_small_negative),
        ("cfg04_root_prints_zero", cfg04_root_prints_zero),
        ("cfg05_negative_x_positive_result", cfg05_negative_x_positive_result),
        ("cfg06_negative_x_negative_result", cfg06_negative_x_negative_result),
        ("cfg07_mid_positive_no_overflow", cfg07_mid_positive_no_overflow),
        ("cfg08_mid_negative_no_overflow", cfg08_mid_negative_no_overflow),
        ("cfg09_last_non_overflowing_inputs", cfg09_last_non_overflowing_inputs),
        ("cfg10_add_overflows_only", cfg10_add_overflows_only),
        ("cfg11_mul_overflows_positive", cfg11_mul_overflows_positive),
        ("cfg12_negative_limit_inside", cfg12_negative_limit_inside),
        ("cfg13_mul_overflows_negative", cfg13_mul_overflows_negative),
        ("cfg14_full_range_random", cfg14_full_range_random),
        ("cfg15_result_width_sweep", cfg15_result_width_sweep),
        ("cfg16_repeated_and_interleaved_calls", cfg16_repeated_and_interleaved_calls),
        ("cfg17_buffered_accumulation", cfg17_buffered_accumulation),
        // Phase C — ERRORS.md rows
        ("errors_no_rejection_path_is_reachable", errors_no_rejection_path_is_reachable),
        ("errors_boundary_and_extremal_inputs", errors_boundary_and_extremal_inputs),
        ("errors_full_int_bit_patterns", errors_full_int_bit_patterns),
        ("errors_repeated_calls_have_no_hidden_state", errors_repeated_calls_have_no_hidden_state),
        // Phase D
        ("errors_expected_exact_bytes", errors_expected_exact_bytes),
        ("symbols_rust_so_exports_every_c_symbol", symbols_rust_so_exports_every_c_symbol),
        // Wide sweep (last: slowest)
        ("soak_wide_sweep", soak_wide_sweep),
    ]
}

fn main() {
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));

    eprintln!("C   .so: {}", c_so_path().display());
    eprintln!("Rust.so: {}", rust_so_path().display());

    let mut passed = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    let mut skipped = 0usize;

    for (name, f) in all_tests() {
        if let Some(fl) = &filter {
            if !name.contains(fl.as_str()) {
                skipped += 1;
                continue;
            }
        }
        print!("test {name} ... ");
        let _ = std::io::stdout().flush();
        match std::panic::catch_unwind(f) {
            Ok(()) => {
                println!("ok");
                passed += 1;
            }
            Err(_) => {
                println!("FAILED");
                failed.push(name);
            }
        }
        let _ = std::io::stdout().flush();
    }

    println!();
    if failed.is_empty() {
        println!("test result: ok. {passed} passed; 0 failed; {skipped} filtered out");
    } else {
        println!("failures:");
        for n in &failed {
            println!("    {n}");
        }
        println!(
            "test result: FAILED. {passed} passed; {} failed; {skipped} filtered out",
            failed.len()
        );
        std::process::exit(1);
    }
}
