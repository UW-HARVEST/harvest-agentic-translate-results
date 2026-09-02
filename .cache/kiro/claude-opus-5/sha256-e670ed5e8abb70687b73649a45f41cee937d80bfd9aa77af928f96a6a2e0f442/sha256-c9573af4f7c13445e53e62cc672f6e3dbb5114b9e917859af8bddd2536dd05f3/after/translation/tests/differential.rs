//! Differential tests: load BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compare their observable behaviour byte-for-byte.
//!
//! The Rust implementation is NEVER called directly — every call goes through
//! `dlopen`/`dlsym` on `target/{debug,release}/libdriver.so`, exactly as an
//! external C consumer would, so the `#[no_mangle] extern "C"` export wrapper
//! is exercised too.
//!
//! `driver` communicates only through `stdout` (`printf("%d", …)` followed by
//! `puts("")`), so "output" here means the raw bytes that reach file
//! descriptor 1. `capture()` redirects fd 1 to a temporary file around the
//! calls, flushes every `FILE` buffer, and reads the bytes back.
//!
//! Layout of this file:
//!   * harness            — .so loading, fd-1 capture, deterministic PRNG
//!   * Phase B            — one test per row of `CONFIGS.md`
//!   * Phase C            — one test per row of `ERRORS.md`
//!   * Phase D            — symbol-parity check driven from `nm -D`

use std::ffi::{c_int, c_void};
use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// harness: libc bits needed to capture the real fd 1
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, including the
    /// `stdout` `FILE` buffer that both libraries write through.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// fd redirection is process-global, so all captures must be serialised even
/// though libtest runs tests on multiple threads.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Run `f` with file descriptor 1 pointed at a fresh temporary file and return
/// everything that was written to it.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver-diff-{}-{}.out",
        std::process::id(),
        n
    ));

    let data = unsafe {
        // Don't let anything already sitting in a buffer leak into the
        // capture: Rust's own `stdout` LineWriter first, then every C `FILE`.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        {
            let file = fs::File::create(&path).expect("create temp capture file");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
            // fd 1 now holds its own reference to the file description.
        }

        f();

        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);

        fs::read(&path).expect("read temp capture file")
    };

    let _ = fs::remove_file(&path);
    drop(guard);
    data
}

// ---------------------------------------------------------------------------
// harness: locating and loading the two shared objects
// ---------------------------------------------------------------------------

struct Libs {
    c: Library,
    rust: Library,
    c_path: PathBuf,
    rust_path: PathBuf,
}

// Both handles are only ever used behind `CAPTURE_LOCK` or for `dlsym`, which
// is thread-safe; the libraries are leaked for the life of the process (never
// `dlclose`d), which is what a real consumer that keeps them loaded does.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().expect("workspace root").to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\ntried: {candidates:?}"
    );
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // Walk up from the test binary (…/target/<profile>/deps/<exe>) so the
    // profile currently under test is preferred.
    let mut from_exe: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            from_exe.push(deps.join("libdriver.so"));
            if let Some(profile) = deps.parent() {
                from_exe.push(profile.join("libdriver.so"));
            }
        }
    }
    let target = manifest_dir().join("target");
    from_exe.push(target.join("release/libdriver.so"));
    from_exe.push(target.join("debug/libdriver.so"));

    for c in &from_exe {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust shared library not found; build it with `cargo build --release`\ntried: {from_exe:?}"
    );
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));
            Libs { c, rust, c_path, rust_path }
        }
    })
}

type DriverFn = unsafe extern "C" fn(c_int, c_int);
/// Same symbol viewed with 64-bit parameters, to probe what the callee does
/// with the unspecified upper half of an `int` argument register.
type DriverFn64 = unsafe extern "C" fn(i64, i64);
/// Same symbol viewed with extra trailing arguments.
type DriverFn4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int);

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C .so exports `driver`") }
}
fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("Rust .so exports `driver`") }
}

// ---------------------------------------------------------------------------
// harness: deterministic PRNG (SplitMix64) — fixed seeds, reproducible
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform over all 2^32 bit patterns of `i32`.
    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    fn next_positive(&mut self) -> i32 {
        (self.next_u64() as u32 & 0x7FFF_FFFF) as i32
    }
    fn next_negative(&mut self) -> i32 {
        (self.next_u64() as u32 | 0x8000_0000) as i32
    }
}

// ---------------------------------------------------------------------------
// harness: the differential assertion
// ---------------------------------------------------------------------------

/// Reference model of the C: `printf("%d", x | ~y); puts("");`
fn model(pairs: &[(i32, i32)]) -> Vec<u8> {
    let mut out = Vec::new();
    for &(x, y) in pairs {
        out.extend_from_slice(format!("{}\n", x | !y).as_bytes());
    }
    out
}

/// Drive `pairs` through the C `.so` and the Rust `.so` and require the
/// captured `stdout` bytes to be identical.
fn assert_same(row: &str, pairs: &[(i32, i32)]) {
    assert!(!pairs.is_empty(), "{row}: empty input set");

    let c = c_driver();
    let c_out = capture(|| unsafe {
        for &(x, y) in pairs {
            c(x, y);
        }
    });

    let r = rust_driver();
    let r_out = capture(|| unsafe {
        for &(x, y) in pairs {
            r(x, y);
        }
    });

    if c_out != r_out {
        // Narrow the report down to the first differing call.
        let c_lines: Vec<&[u8]> = c_out.split(|b| *b == b'\n').collect();
        let r_lines: Vec<&[u8]> = r_out.split(|b| *b == b'\n').collect();
        for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
            if cl != rl {
                let (x, y) = pairs.get(i).copied().unwrap_or((0, 0));
                panic!(
                    "{row}: divergence at call #{i} driver({x}, {y}):\n  C   : {:?}\n  Rust: {:?}",
                    String::from_utf8_lossy(cl),
                    String::from_utf8_lossy(rl)
                );
            }
        }
        panic!(
            "{row}: output length differs: C {} bytes / {} lines, Rust {} bytes / {} lines",
            c_out.len(),
            c_lines.len(),
            r_out.len(),
            r_lines.len()
        );
    }

    // Harness sanity: confirm the captured stream is the real thing and not
    // (say) two identically-empty captures. The C remains ground truth — if
    // this ever fired, the model would be what is wrong.
    assert_eq!(
        c_out,
        model(pairs),
        "{row}: harness/model mismatch against the C output (C is ground truth)"
    );
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// Row 1: `x = 0`, `y = 0`.
fn cfg_row01_zero_zero() {
    assert_same("cfg row 1", &[(0, 0)]);
}

/// Row 2: the unique pair whose result is `0`.
fn cfg_row02_result_is_zero() {
    assert_same("cfg row 2", &[(0, -1)]);
}

/// Row 3: `x = -1` saturates the OR for any `y`.
fn cfg_row03_x_minus_one_random_y() {
    let mut rng = Rng::new(0x0301);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (-1, rng.next_i32())).collect();
    assert_same("cfg row 3", &pairs);
}

/// Row 4: `y = 0` makes `~y = -1`, saturating the OR for any `x`.
fn cfg_row04_random_x_y_zero() {
    let mut rng = Rng::new(0x0401);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (rng.next_i32(), 0)).collect();
    assert_same("cfg row 4", &pairs);
}

/// Row 5: `x = 0` with negative `y` — non-negative results.
fn cfg_row05_x_zero_negative_y() {
    let mut rng = Rng::new(0x0501);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (0, rng.next_negative())).collect();
    assert_same("cfg row 5", &pairs);
}

/// Row 6: `x = 0` with non-negative `y` — negative results.
fn cfg_row06_x_zero_nonnegative_y() {
    let mut rng = Rng::new(0x0601);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (0, rng.next_positive())).collect();
    assert_same("cfg row 6", &pairs);
}

const BOUNDARIES: [i32; 5] = [i32::MIN, -1, 0, 1, i32::MAX];

/// Row 7: the full 5x5 boundary cross-product.
fn cfg_row07_boundary_cross_product() {
    let mut pairs = Vec::new();
    for &x in &BOUNDARIES {
        for &y in &BOUNDARIES {
            pairs.push((x, y));
        }
    }
    assert_eq!(pairs.len(), 25);
    assert_same("cfg row 7", &pairs);
}

/// Row 8: widest negative output (`-2147483648`, 11 bytes).
fn cfg_row08_widest_negative_output() {
    assert_same("cfg row 8", &[(i32::MIN, i32::MAX)]);
}

/// Row 9: widest non-negative output (`2147483647`, 10 bytes).
fn cfg_row09_widest_positive_output() {
    assert_same("cfg row 9", &[(i32::MAX, i32::MIN)]);
}

/// Row 10: positive `x`, negative `y` — the non-negative-result class.
fn cfg_row10_positive_x_negative_y() {
    let mut rng = Rng::new(0x1001);
    let pairs: Vec<(i32, i32)> = (0..1024)
        .map(|_| (rng.next_positive(), rng.next_negative()))
        .collect();
    assert_same("cfg row 10", &pairs);
}

/// Row 11: negative `x`, any `y`.
fn cfg_row11_negative_x_any_y() {
    let mut rng = Rng::new(0x1101);
    let pairs: Vec<(i32, i32)> = (0..1024)
        .map(|_| (rng.next_negative(), rng.next_i32()))
        .collect();
    assert_same("cfg row 11", &pairs);
}

/// Row 12: `y = INT_MAX` so `~y = INT_MIN` — sign bit forced on.
fn cfg_row12_y_int_max_random_x() {
    let mut rng = Rng::new(0x1201);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (rng.next_i32(), i32::MAX)).collect();
    assert_same("cfg row 12", &pairs);
}

/// Row 13: `y = INT_MIN` so `~y = INT_MAX` — all value bits forced on.
fn cfg_row13_y_int_min_random_x() {
    let mut rng = Rng::new(0x1301);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (rng.next_i32(), i32::MIN)).collect();
    assert_same("cfg row 13", &pairs);
}

/// Row 14: `x = INT_MAX`, any `y`.
fn cfg_row14_x_int_max_random_y() {
    let mut rng = Rng::new(0x1401);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (i32::MAX, rng.next_i32())).collect();
    assert_same("cfg row 14", &pairs);
}

/// Row 15: `x = INT_MIN`, any `y`.
fn cfg_row15_x_int_min_random_y() {
    let mut rng = Rng::new(0x1501);
    let pairs: Vec<(i32, i32)> = (0..512).map(|_| (i32::MIN, rng.next_i32())).collect();
    assert_same("cfg row 15", &pairs);
}

/// Targets `t` are reached two independent ways:
///   * `driver(t, -1)`  because `~(-1) == 0`
///   * `driver(0, !t)`  because `~!t == t`
fn pairs_for_targets(targets: &[i32]) -> Vec<(i32, i32)> {
    let mut pairs = Vec::with_capacity(targets.len() * 2);
    for &t in targets {
        pairs.push((t, -1));
        pairs.push((0, !t));
    }
    pairs
}

/// Row 16: non-negative-result decimal-width sweep, 1 to 10 output bytes.
fn cfg_row16_nonnegative_digit_width_sweep() {
    let mut targets: Vec<i32> = (0..=10).collect();
    let mut p: i64 = 1;
    for _ in 0..10 {
        for cand in [p - 1, p, p + 1] {
            if (0..=i32::MAX as i64).contains(&cand) {
                targets.push(cand as i32);
            }
        }
        p *= 10;
    }
    targets.push(i32::MAX);
    targets.sort_unstable();
    targets.dedup();
    assert_same("cfg row 16", &pairs_for_targets(&targets));
}

/// Row 17: negative-result decimal-width sweep, 2 to 11 output bytes.
fn cfg_row17_negative_digit_width_sweep() {
    let mut targets: Vec<i32> = (-10..=-1).collect();
    let mut p: i64 = 1;
    for _ in 0..10 {
        for cand in [-(p - 1), -p, -(p + 1)] {
            if (i32::MIN as i64..0).contains(&cand) {
                targets.push(cand as i32);
            }
        }
        p *= 10;
    }
    targets.push(i32::MIN);
    targets.push(i32::MIN + 1);
    targets.sort_unstable();
    targets.dedup();
    assert_same("cfg row 17", &pairs_for_targets(&targets));
}

/// Row 18: single-bit `x` against single-hole `~y`, all 32x32 bit positions.
fn cfg_row18_single_bit_walk() {
    let mut pairs = Vec::with_capacity(1024);
    for i in 0..32u32 {
        let x = (1u32 << i) as i32;
        for j in 0..32u32 {
            let y = !(1u32 << j) as i32;
            pairs.push((x, y));
        }
    }
    assert_eq!(pairs.len(), 1024);
    assert_same("cfg row 18", &pairs);
}

/// Row 19: the general case — uniform random over all bit patterns.
fn cfg_row19_uniform_random_batch() {
    let mut rng = Rng::new(0x1901_2345_6789_ABCD);
    let pairs: Vec<(i32, i32)> = (0..20_000)
        .map(|_| (rng.next_i32(), rng.next_i32()))
        .collect();
    assert_same("cfg row 19", &pairs);
}

/// Row 20: call-count axis, *empty* — loading the libraries and making zero
/// calls must write nothing to `stdout`.
fn cfg_row20_zero_calls_write_nothing() {
    let c = c_driver();
    let r = rust_driver();
    let c_out = capture(|| {
        let _ = &c;
    });
    let r_out = capture(|| {
        let _ = &r;
    });
    assert_eq!(c_out, r_out, "cfg row 20: zero-call output differs");
    assert!(
        c_out.is_empty(),
        "cfg row 20: expected no output, C wrote {:?}",
        String::from_utf8_lossy(&c_out)
    );
}

/// Row 21: call-count axis, *one* — exactly `<digits>\n`, no extra bytes.
fn cfg_row21_exactly_one_call_framing() {
    let pairs = [(1234, -1)];
    assert_same("cfg row 21", &pairs);

    let c = c_driver();
    let c_out = capture(|| unsafe { c(1234, -1) });
    let r = rust_driver();
    let r_out = capture(|| unsafe { r(1234, -1) });
    assert_eq!(c_out, r_out);
    assert_eq!(
        c_out, b"1234\n",
        "cfg row 21: unexpected framing from the C: {:?}",
        String::from_utf8_lossy(&c_out)
    );
    assert_eq!(c_out.len(), 5, "cfg row 21: trailing/extra bytes present");
}

/// Row 22: call-count axis, *many* — a long unflushed sequence, comparing the
/// whole concatenated buffered stream.
fn cfg_row22_many_calls_buffered_stream() {
    let mut rng = Rng::new(0x2201);
    let pairs: Vec<(i32, i32)> = (0..5_000)
        .map(|_| (rng.next_i32(), rng.next_i32()))
        .collect();
    assert_same("cfg row 22", &pairs);
}

/// Row 23: exhaustive sweep of the signed low byte in both parameters.
fn cfg_row23_exhaustive_low_byte_sweep() {
    let mut pairs = Vec::with_capacity(65_536);
    for x in -128i32..=127 {
        for y in -128i32..=127 {
            pairs.push((x, y));
        }
    }
    assert_eq!(pairs.len(), 65_536);
    assert_same("cfg row 23", &pairs);
}

// ===========================================================================
// Phase C — ERRORS.md rows
//
// The C library has an empty rejection surface (see ERRORS.md for the grep
// that establishes this): no returns, no error codes, no asserts, no range or
// null checks, no pointer/length/enum parameters. Each row therefore asserts
// that the Rust *also* does not reject, and produces identical bytes, for the
// exact boundary condition named in the table.
// ===========================================================================

/// Row 1: `x = INT_MIN`, the smallest representable first argument.
fn err_row01_x_int_min() {
    let mut rng = Rng::new(0xE001);
    let mut pairs = vec![(i32::MIN, 0), (i32::MIN, -1), (i32::MIN, 1)];
    pairs.extend((0..256).map(|_| (i32::MIN, rng.next_i32())));
    assert_same("err row 1", &pairs);
}

/// Row 2: `y = INT_MIN`, the smallest representable second argument.
fn err_row02_y_int_min() {
    let mut rng = Rng::new(0xE002);
    let mut pairs = vec![(0, i32::MIN), (-1, i32::MIN), (1, i32::MIN)];
    pairs.extend((0..256).map(|_| (rng.next_i32(), i32::MIN)));
    assert_same("err row 2", &pairs);
}

/// Row 3: `x = INT_MAX`, the largest representable first argument.
fn err_row03_x_int_max() {
    let mut rng = Rng::new(0xE003);
    let mut pairs = vec![(i32::MAX, 0), (i32::MAX, -1), (i32::MAX, 1)];
    pairs.extend((0..256).map(|_| (i32::MAX, rng.next_i32())));
    assert_same("err row 3", &pairs);
}

/// Row 4: `y = INT_MAX`, the largest representable second argument.
fn err_row04_y_int_max() {
    let mut rng = Rng::new(0xE004);
    let mut pairs = vec![(0, i32::MAX), (-1, i32::MAX), (1, i32::MAX)];
    pairs.extend((0..256).map(|_| (rng.next_i32(), i32::MAX)));
    assert_same("err row 4", &pairs);
}

/// Row 5: the all-zero argument case.
fn err_row05_both_zero() {
    assert_same("err row 5", &[(0, 0)]);
}

/// Row 6: the only argument pair yielding `0`.
fn err_row06_result_zero() {
    assert_same("err row 6", &[(0, -1)]);
}

/// Row 7: values one step past `int` range handed across the FFI boundary.
///
/// The SysV AMD64 ABI passes an `int` parameter in the low 32 bits of a
/// register and leaves the upper half unspecified. A caller that (wrongly)
/// declares the parameter 64-bit is therefore a real input both libraries must
/// handle identically: each must ignore the high half.
fn err_row07_out_of_int_range_ffi() {
    let wide: [i64; 12] = [
        i32::MAX as i64 + 1,
        i32::MIN as i64 - 1,
        0x1_0000_0000,
        0x1_0000_0001,
        -0x1_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        i64::MIN,
        -1,
        0xFFFF_FFFF,
        0xDEAD_BEEF_0000_0000u64 as i64,
        0xDEAD_BEEF_7FFF_FFFFu64 as i64,
        0x0000_0001_8000_0000,
    ];

    let c: Symbol<DriverFn64> = unsafe { libs().c.get(b"driver\0").unwrap() };
    let r: Symbol<DriverFn64> = unsafe { libs().rust.get(b"driver\0").unwrap() };

    let mut cases: Vec<(i64, i64)> = Vec::new();
    for &x in &wide {
        for &y in &wide {
            cases.push((x, y));
        }
    }

    let c_out = capture(|| unsafe {
        for &(x, y) in &cases {
            c(x, y);
        }
    });
    let r_out = capture(|| unsafe {
        for &(x, y) in &cases {
            r(x, y);
        }
    });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "err row 7: 64-bit-argument truncation differs between C and Rust"
    );

    // And it must agree with the properly-truncated 32-bit call.
    let narrow: Vec<(i32, i32)> = cases.iter().map(|&(x, y)| (x as i32, y as i32)).collect();
    assert_eq!(
        c_out,
        model(&narrow),
        "err row 7: C did not simply truncate the arguments (C is ground truth)"
    );
}

/// Row 8: extra trailing arguments across the FFI boundary are ignored.
fn err_row08_extra_ffi_arguments() {
    let c: Symbol<DriverFn4> = unsafe { libs().c.get(b"driver\0").unwrap() };
    let r: Symbol<DriverFn4> = unsafe { libs().rust.get(b"driver\0").unwrap() };

    let cases: [(c_int, c_int, c_int, c_int); 5] = [
        (0, 0, 0, 0),
        (5, -7, i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX, -1, 1),
        (-1, -1, 12345, -12345),
        (1, 2, 3, 4),
    ];

    let c_out = capture(|| unsafe {
        for &(a, b, cc, d) in &cases {
            c(a, b, cc, d);
        }
    });
    let r_out = capture(|| unsafe {
        for &(a, b, cc, d) in &cases {
            r(a, b, cc, d);
        }
    });
    assert_eq!(c_out, r_out, "err row 8: extra-argument call diverges");

    let two: Vec<(i32, i32)> = cases.iter().map(|&(a, b, _, _)| (a, b)).collect();
    assert_eq!(
        c_out,
        model(&two),
        "err row 8: extra arguments were not ignored by the C (C is ground truth)"
    );
}

/// Row 9: "out-of-range enum" style `int`s — bit patterns with no meaningful
/// interpretation. A C enum parameter accepts any `int`, so these must be
/// accepted, not rejected, and must behave identically on both sides.
fn err_row09_out_of_range_enum_like_ints() {
    let odd: [i32; 16] = [
        -1,                       // every bit set
        i32::MIN,                 // lone sign bit
        i32::MAX,                 // every value bit set
        0x5555_5555,
        0x5555_5555u32 as i32 | i32::MIN,
        0x3333_3333,
        0x0F0F_0F0F,
        0x00FF_00FF,
        0x0000_FFFF,
        0xFFFF_0000u32 as i32,
        0xAAAA_AAAAu32 as i32,
        0xDEAD_BEEFu32 as i32,
        0xCAFE_BABEu32 as i32,
        1 << 31 >> 31,            // -1 via shift
        0x7FFF_FFFE,
        -0x7FFF_FFFF,
    ];
    let mut pairs = Vec::with_capacity(odd.len() * odd.len());
    for &x in &odd {
        for &y in &odd {
            pairs.push((x, y));
        }
    }
    assert_same("err row 9", &pairs);
}

/// Row 10: there is no init/teardown API, so "uninitialised use" and
/// "use after teardown" are simply repeated calls; none may be rejected.
fn err_row10_repeated_invocation_no_init() {
    let mut rng = Rng::new(0xE010);
    let pairs: Vec<(i32, i32)> = (0..2_000)
        .map(|_| (rng.next_i32(), rng.next_i32()))
        .collect();
    // Twice through, no re-initialisation of any kind in between.
    assert_same("err row 10 (pass 1)", &pairs);
    assert_same("err row 10 (pass 2)", &pairs);
}

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
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
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .filter(|s| !s.is_empty())
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`
/// under the exact same name. The diff must be empty.
fn phase_d_symbol_parity() {
    let l = libs();
    let c_syms = defined_dynamic_symbols(&l.c_path);
    let r_syms = defined_dynamic_symbols(&l.rust_path);

    assert!(
        c_syms.contains(&"driver".to_string()),
        "sanity: C .so should export `driver`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C ({}): {c_syms:?}\nRust ({}): {r_syms:?}",
        missing.len(),
        l.c_path.display(),
        l.rust_path.display()
    );
}

/// The Rust `.so` must not depend on any undefined symbol outside the C
/// runtime — i.e. nothing that a plain C consumer would fail to resolve.
fn phase_d_no_unresolvable_undefined_symbols() {
    let l = libs();
    let out = std::process::Command::new("nm")
        .args(["-D", "-u", l.rust_path.to_str().unwrap()])
        .output()
        .expect("run nm -u");
    assert!(out.status.success(), "nm -u failed");

    // Anything that is not a libc / libgcc(unwind) / loader symbol.
    let text = String::from_utf8_lossy(&out.stdout);
    let suspicious: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        .filter(|s| {
            let base = s.split('@').next().unwrap_or(s);
            // Rust-mangled or crate-internal leftovers would look like these.
            base.starts_with("_ZN") || base.starts_with("_RN")
        })
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so has unresolved Rust-mangled symbols: {suspicious:?}"
    );

    // Both objects must load and resolve `driver` — already proven by the
    // successful dlopen + dlsym in `libs()`, restated here explicitly.
    let _ = c_driver();
    let _ = rust_driver();
}

// ===========================================================================
// Custom harness
//
// `harness = false`: libtest is not used, because libtest writes its progress
// to file descriptor 1 — the exact channel these tests capture. All progress
// here goes to stderr instead, so the captured stream contains nothing but the
// bytes the libraries under test produced.
//
// Usage: `cargo test --release` runs everything; `cargo test --release --
// <substring>` runs the matching subset.
// ===========================================================================

type Case = (&'static str, fn());

fn cases() -> Vec<Case> {
    vec![
        // Phase B — CONFIGS.md
        ("cfg_row01_zero_zero", cfg_row01_zero_zero as fn()),
        ("cfg_row02_result_is_zero", cfg_row02_result_is_zero),
        ("cfg_row03_x_minus_one_random_y", cfg_row03_x_minus_one_random_y),
        ("cfg_row04_random_x_y_zero", cfg_row04_random_x_y_zero),
        ("cfg_row05_x_zero_negative_y", cfg_row05_x_zero_negative_y),
        ("cfg_row06_x_zero_nonnegative_y", cfg_row06_x_zero_nonnegative_y),
        ("cfg_row07_boundary_cross_product", cfg_row07_boundary_cross_product),
        ("cfg_row08_widest_negative_output", cfg_row08_widest_negative_output),
        ("cfg_row09_widest_positive_output", cfg_row09_widest_positive_output),
        ("cfg_row10_positive_x_negative_y", cfg_row10_positive_x_negative_y),
        ("cfg_row11_negative_x_any_y", cfg_row11_negative_x_any_y),
        ("cfg_row12_y_int_max_random_x", cfg_row12_y_int_max_random_x),
        ("cfg_row13_y_int_min_random_x", cfg_row13_y_int_min_random_x),
        ("cfg_row14_x_int_max_random_y", cfg_row14_x_int_max_random_y),
        ("cfg_row15_x_int_min_random_y", cfg_row15_x_int_min_random_y),
        ("cfg_row16_nonnegative_digit_width_sweep", cfg_row16_nonnegative_digit_width_sweep),
        ("cfg_row17_negative_digit_width_sweep", cfg_row17_negative_digit_width_sweep),
        ("cfg_row18_single_bit_walk", cfg_row18_single_bit_walk),
        ("cfg_row19_uniform_random_batch", cfg_row19_uniform_random_batch),
        ("cfg_row20_zero_calls_write_nothing", cfg_row20_zero_calls_write_nothing),
        ("cfg_row21_exactly_one_call_framing", cfg_row21_exactly_one_call_framing),
        ("cfg_row22_many_calls_buffered_stream", cfg_row22_many_calls_buffered_stream),
        ("cfg_row23_exhaustive_low_byte_sweep", cfg_row23_exhaustive_low_byte_sweep),
        // Phase C — ERRORS.md
        ("err_row01_x_int_min", err_row01_x_int_min),
        ("err_row02_y_int_min", err_row02_y_int_min),
        ("err_row03_x_int_max", err_row03_x_int_max),
        ("err_row04_y_int_max", err_row04_y_int_max),
        ("err_row05_both_zero", err_row05_both_zero),
        ("err_row06_result_zero", err_row06_result_zero),
        ("err_row07_out_of_int_range_ffi", err_row07_out_of_int_range_ffi),
        ("err_row08_extra_ffi_arguments", err_row08_extra_ffi_arguments),
        ("err_row09_out_of_range_enum_like_ints", err_row09_out_of_range_enum_like_ints),
        ("err_row10_repeated_invocation_no_init", err_row10_repeated_invocation_no_init),
        // Phase D — symbol parity
        ("phase_d_symbol_parity", phase_d_symbol_parity),
        ("phase_d_no_unresolvable_undefined_symbols", phase_d_no_unresolvable_undefined_symbols),
    ]
}

fn main() {
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();

    let all = cases();
    let selected: Vec<&Case> = all
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str())))
        .collect();

    eprintln!("\nrunning {} differential cases", selected.len());
    eprintln!("  C    .so: {}", c_so_path().display());
    eprintln!("  Rust .so: {}", rust_so_path().display());

    let mut failed: Vec<&str> = Vec::new();
    for (name, f) in &selected {
        eprint!("test {name} ... ");
        let _ = std::io::stderr().flush();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    eprintln!(
        "\nresult: {}. {} passed; {} failed",
        if failed.is_empty() { "ok" } else { "FAILED" },
        selected.len() - failed.len(),
        failed.len()
    );
    if !failed.is_empty() {
        eprintln!("failures:");
        for f in &failed {
            eprintln!("    {f}");
        }
        std::process::exit(1);
    }
}
