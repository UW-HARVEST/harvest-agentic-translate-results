// Differential tests: load BOTH the C `.so` and the Rust `.so` with `libloading`
// and compare their behavior through the FFI boundary.
//
// The Rust functions are NEVER called directly -- every call goes through a
// `libloading` symbol lookup on `libdriver.so`, exactly as an external C caller
// would, so the `#[no_mangle] extern "C"` export wrapper is under test too.
//
// The public surface is `void driver(int x)`, whose entire observable effect is
// the line it writes to stdout via libc `printf`. So "outputs match
// byte-for-byte" means: the bytes that appear on file descriptor 1 are
// identical. Capture is done at the fd level (dup/dup2) rather than with Rust's
// `print!` capture, because the libraries write through libc's buffered
// `stdout`, which Rust's test harness does not intercept.

use std::ffi::c_int;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

type DriverFn = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    // `cargo test` builds the cdylib into target/<profile>/. Accept either
    // profile, preferring whichever matches the current test build, so the test
    // works under `cargo test` and `cargo test --release`.
    let base = manifest_dir().join("target");
    let candidates = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in candidates {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust shared library libdriver.so not found under {base:?}.\nBuild it with: cargo build --release"
    );
}

struct Libs {
    c: Library,
    rust: Library,
}

// Both libraries stay loaded for the whole process: they share one libc
// `stdout`, which is itself part of what we are verifying (CONFIGS row 14).
fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("failed to dlopen the C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("failed to dlopen the Rust libdriver.so");
        Libs { c, rust }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0") }.expect("symbol `driver` missing from the C .so")
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0") }
        .expect("symbol `driver` missing from the Rust .so -- is the #[no_mangle] export present?")
}

// ---------------------------------------------------------------------------
// stdout capture at the file-descriptor level
// ---------------------------------------------------------------------------

// fd 1 redirection is process-global, and cargo runs tests on parallel threads,
// so every capture must be serialized.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Redirect fd 1 to a temp file, run `f`, flush libc's streams, restore fd 1,
/// and return the exact bytes written.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Flush anything already buffered (ours and libc's) so it is not captured.
    std::io::stdout().flush().ok();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = dir.join(format!(
        "driver-diff-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("failed to create capture temp file");

    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    f();

    // Flush the library's buffered stdout *while fd 1 is still redirected*.
    unsafe { libc::fflush(std::ptr::null_mut()) };
    assert!(unsafe { libc::dup2(saved, 1) } >= 0, "restoring fd 1 failed");
    unsafe { libc::close(saved) };

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek of capture file failed");
    file.read_to_end(&mut out).expect("read of capture file failed");
    let _ = std::fs::remove_file(&path);
    out
}

/// Bytes printed by the C `.so` for a whole batch of inputs, in order.
fn c_output(inputs: &[i32]) -> Vec<u8> {
    let f = c_driver();
    capture_stdout(|| {
        for &x in inputs {
            unsafe { f(x) };
        }
    })
}

/// Bytes printed by the Rust `.so` for a whole batch of inputs, in order.
fn rust_output(inputs: &[i32]) -> Vec<u8> {
    let f = rust_driver();
    capture_stdout(|| {
        for &x in inputs {
            unsafe { f(x) };
        }
    })
}

/// The core differential assertion: C and Rust must emit identical bytes for
/// the given inputs. On divergence, report the first differing line together
/// with the input that produced it.
#[track_caller]
fn assert_same(row: &str, inputs: &[i32]) {
    let c = c_output(inputs);
    let r = rust_output(inputs);
    if c == r {
        return;
    }

    let cs = String::from_utf8_lossy(&c);
    let rs = String::from_utf8_lossy(&r);
    let cl: Vec<&str> = cs.lines().collect();
    let rl: Vec<&str> = rs.lines().collect();
    for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
        if a != b {
            panic!(
                "[{row}] divergence at output line {i} (input x = {}): C printed {a:?}, Rust printed {b:?}",
                inputs.get(i).copied().unwrap_or_default()
            );
        }
    }
    panic!(
        "[{row}] output length differs: C produced {} lines ({} bytes), Rust produced {} lines ({} bytes) for {} inputs",
        cl.len(),
        c.len(),
        rl.len(),
        r.len(),
        inputs.len()
    );
}

/// Deterministic LCG (fixed seed => reproducible runs).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi` (inclusive), correct for the full i32 range.
    fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        let r = ((self.next_u32() as u64) << 32 | self.next_u32() as u64) % span;
        (lo as i64 + r as i64) as i32
    }
}

// ===========================================================================
// Phase A / Phase D -- symbol parity, checked through the loaded objects
// ===========================================================================

#[test]
fn phase_a_both_sos_export_driver() {
    // Resolving both symbols is the load-bearing assertion: it proves the
    // Rust `.so` really exports `driver` under that exact unmangled name.
    let _c = c_driver();
    let _r = rust_driver();
}

#[test]
fn phase_d_symbol_parity_via_nm() {
    let defined = |p: &PathBuf| -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(p)
            .output();
        let Ok(out) = out else { return Vec::new() };
        if !out.status.success() {
            return Vec::new();
        }
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(str::to_string))
            .collect();
        v.sort();
        v
    };

    let c = defined(&c_so_path());
    if c.is_empty() {
        eprintln!("skipping: `nm` unavailable");
        return;
    }
    let r = defined(&rust_so_path());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
    assert_eq!(c, vec!["driver".to_string()], "unexpected C export set: {c:?}");
}

#[test]
fn phase_d_features_surface_is_only_default() {
    // Documents (and enforces) the Phase D claim that there is exactly one
    // build configuration: Cargo.toml declares no [features] table.
    let toml = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    let has_features = toml
        .lines()
        .any(|l| l.trim_start().starts_with("[features]"));
    assert!(
        !has_features,
        "Cargo.toml gained a [features] table -- Phases B and C must now be re-run for every feature combination"
    );
}

// ===========================================================================
// Phase B -- valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

#[test]
fn phase_b_row01_zero() {
    assert_same("row 1: x = 0", &[0]);
}

#[test]
fn phase_b_row02_small_positive_exhaustive() {
    let inputs: Vec<i32> = (1..=1000).collect();
    assert_same("row 2: x in 1..=1000 exhaustive", &inputs);
}

#[test]
fn phase_b_row03_small_negative_exhaustive() {
    let inputs: Vec<i32> = (-1000..=-1).collect();
    assert_same("row 3: x in -1000..=-1 exhaustive", &inputs);
}

#[test]
fn phase_b_row04_result_sign_flip() {
    // 2*x + 300 == 0 at x == -150; -149 and -151 straddle it.
    assert_same("row 4: sign flip of the result", &[-151, -150, -149]);
}

#[test]
fn phase_b_row05_mid_positive_random() {
    let mut rng = Rng::new(0x5EED_0005);
    let inputs: Vec<i32> = (0..5000).map(|_| rng.i32_in(1001, 1 << 29)).collect();
    assert_same("row 5: random positive, no overflow", &inputs);
}

#[test]
fn phase_b_row06_mid_negative_random() {
    let mut rng = Rng::new(0x5EED_0006);
    let inputs: Vec<i32> = (0..5000).map(|_| rng.i32_in(-(1 << 29), -1001)).collect();
    assert_same("row 6: random negative, no overflow", &inputs);
}

#[test]
fn phase_b_row07_add_only_overflow_exhaustive() {
    // 2*x fits, but 2*x + 300 does not: 2*x in [i32::MAX-299, i32::MAX-1].
    let inputs: Vec<i32> = (1073741524..=1073741823).collect();
    assert_same("row 7: `+= 300` overflows, multiply does not", &inputs);
}

#[test]
fn phase_b_row08_multiply_overflow_boundary_positive() {
    let mut rng = Rng::new(0x5EED_0008);
    let mut inputs = vec![1073741822, 1073741823, 1073741824, 1073741825];
    inputs.extend((0..5000).map(|_| rng.i32_in(1 << 30, i32::MAX)));
    assert_same("row 8: multiply-overflow boundary, positive side", &inputs);
}

#[test]
fn phase_b_row09_multiply_overflow_boundary_negative() {
    let mut rng = Rng::new(0x5EED_0009);
    let mut inputs = vec![-1073741823, -1073741824, -1073741825, -1073741826];
    inputs.extend((0..5000).map(|_| rng.i32_in(i32::MIN, -(1 << 30))));
    assert_same("row 9: multiply-overflow boundary, negative side", &inputs);
}

#[test]
fn phase_b_row10_extremes() {
    assert_same(
        "row 10: extremes",
        &[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 1, -1, 0],
    );
}

#[test]
fn phase_b_row11_digit_width_sweep() {
    let mut inputs = Vec::new();
    // y = 2*x + 300, so x = (y - 300) / 2. Pick y straddling every power of ten
    // (both signs) to exercise every printed field width of "%d".
    for k in 0..10u32 {
        let p = 10i64.pow(k);
        for y in [p - 1, p, p + 1, -(p - 1), -p, -(p + 1)] {
            let t = y - 300;
            if t % 2 == 0 {
                let x = t / 2;
                if x >= i32::MIN as i64 && x <= i32::MAX as i64 {
                    inputs.push(x as i32);
                }
            }
        }
    }
    inputs.extend([-150, -149, -151, 0]);
    assert_same("row 11: printed digit-width sweep", &inputs);
}

#[test]
fn phase_b_row12_full_range_random() {
    let mut rng = Rng::new(0x5EED_0012);
    let inputs: Vec<i32> = (0..20000).map(|_| rng.i32_any()).collect();
    assert_same("row 12: 20000 uniform random i32", &inputs);
}

#[test]
fn phase_b_row13_call_multiplicity_and_interleaving() {
    // (a) Many successive calls into the same object: no state may leak.
    let repeated = vec![7i32; 500];
    assert_same("row 13a: 500 repeats of the same input", &repeated);

    // (b) Per-input interleaving of C and Rust in one process, rather than
    // batch-vs-batch, so a divergence cannot be masked by ordering.
    let mut rng = Rng::new(0x5EED_0013);
    let cf = c_driver();
    let rf = rust_driver();
    for _ in 0..300 {
        let x = rng.i32_any();
        let c = capture_stdout(|| unsafe { cf(x) });
        let r = capture_stdout(|| unsafe { rf(x) });
        assert_eq!(
            c,
            r,
            "row 13b: interleaved divergence for x = {x}: C {:?} vs Rust {:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
        // Each call must emit exactly one newline-terminated line.
        assert_eq!(c.iter().filter(|&&b| b == b'\n').count(), 1, "x = {x}: not exactly one line");
        assert_eq!(*c.last().unwrap(), b'\n', "x = {x}: output not newline-terminated");
    }
}

#[test]
fn phase_b_row14_shared_stdout_between_objects() {
    // Both objects are loaded in this process and share one libc stdout.
    // Alternate between them inside a *single* capture: the interleaved stream
    // must be exactly the same as the C library producing every line itself.
    let mut rng = Rng::new(0x5EED_0014);
    let inputs: Vec<i32> = (0..400).map(|_| rng.i32_any()).collect();

    let cf = c_driver();
    let rf = rust_driver();
    let mixed = capture_stdout(|| {
        for (i, &x) in inputs.iter().enumerate() {
            if i % 2 == 0 {
                unsafe { cf(x) };
            } else {
                unsafe { rf(x) };
            }
        }
    });
    let all_c = c_output(&inputs);
    assert_eq!(
        mixed, all_c,
        "row 14: C/Rust alternating on the shared stdout diverged from all-C output"
    );
}

// ===========================================================================
// Phase C -- error / boundary path differential tests
//
// ERRORS.md establishes that the C library has NO rejection path: no return
// value, no status code, no pointer to validate, and not one conditional. The
// generic C-API boundaries are covered here regardless, asserting identical
// bytes and that neither implementation traps.
// ===========================================================================

#[test]
fn phase_c_boundaries() {
    // E1: zero. E2/E3: INT_MAX / INT_MIN (multiply overflows). E4/E5: one step
    // either side of the multiply-overflow threshold on both sides.
    // E6: add-only overflow. E7: arbitrary int bit patterns (no enum exists to
    // be out of range, so every bit pattern is the hostile input).
    let cases: &[(&str, i32)] = &[
        ("E1 zero", 0),
        ("E2 INT_MAX", i32::MAX),
        ("E3 INT_MIN", i32::MIN),
        ("E4 INT_MAX/2", i32::MAX / 2),
        ("E4 INT_MAX/2 + 1", i32::MAX / 2 + 1),
        ("E5 INT_MIN/2", i32::MIN / 2),
        ("E5 INT_MIN/2 - 1", i32::MIN / 2 - 1),
        ("E6 add-only overflow low", 1073741524),
        ("E6 add-only overflow high", 1073741823),
        ("E7 all bits set", -1),
        ("E7 sign bit only", i32::MIN),
        ("E7 0x7FFFFFFE", 0x7FFF_FFFE),
        ("E7 0xAAAAAAAA", 0xAAAA_AAAAu32 as i32),
        ("E7 0x55555555", 0x5555_5555),
        ("E7 0xFFFF0000", 0xFFFF_0000u32 as i32),
        ("E7 0x0000FFFF", 0x0000_FFFF),
        ("E7 0x80000001", 0x8000_0001u32 as i32),
    ];

    for (name, x) in cases {
        // Each boundary is checked in isolation so a failure names the case.
        assert_same(name, std::slice::from_ref(x));
    }

    // Same set, in one batch, to also pin the aggregate byte stream.
    let all: Vec<i32> = cases.iter().map(|&(_, x)| x).collect();
    assert_same("E1-E7 aggregate", &all);
}

#[test]
fn phase_c_no_side_channel_or_trap_on_overflow() {
    // Signed overflow is UB in ISO C, but the built C `.so` is ground truth:
    // assert the Rust matches whatever the C actually prints, and that a
    // released-profile Rust build (panic = "abort") does not abort here.
    let overflowing: Vec<i32> = (0..2000)
        .map(|i| i32::MAX - i)
        .chain((0..2000).map(|i| i32::MIN + i))
        .collect();
    assert_same("Phase C: dense overflow sweep near both extremes", &overflowing);
}

#[test]
fn phase_c_interleaved_calls() {
    // E8: repeated and interleaved calls -- one line per call, same order, no
    // state carried between calls in either implementation.
    let inputs: Vec<i32> = vec![0, -150, i32::MAX, i32::MIN, 42, -42, 1073741824, -1073741825];

    for _round in 0..25 {
        assert_same("E8: repeated rounds over the same inputs", &inputs);
    }

    let out = c_output(&inputs);
    assert_eq!(
        out.iter().filter(|&&b| b == b'\n').count(),
        inputs.len(),
        "E8: expected exactly one line per call"
    );
}

/// Heavy near-exhaustive sweep: every 4093rd value across the whole `i32`
/// range (~1.05M inputs). Not run by default (slow, ~10 MB of captured
/// output); run with `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn phase_b_heavy_stride_sweep() {
    const STRIDE: i64 = 4093;
    let inputs: Vec<i32> = (i32::MIN as i64..=i32::MAX as i64)
        .step_by(STRIDE as usize)
        .map(|v| v as i32)
        .collect();
    assert_same("heavy: full-range stride-4093 sweep", &inputs);
}
