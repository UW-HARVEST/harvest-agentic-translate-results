//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! The Rust implementation is NEVER called directly as a Rust function. Both
//! sides are reached only through `dlopen` + `dlsym` on their shared objects,
//! so the `#[no_mangle] extern "C"` export wrapper is part of what is tested.
//!
//! `driver`'s only observable effect is bytes written to the process `stdout`
//! by libc `printf`, so every comparison is a byte-for-byte comparison of
//! captured stdout. Capture is done at the file-descriptor level (`dup`/`dup2`),
//! which is what actually observes libc-buffered writes coming out of a shared
//! object; Rust's `println!` capture in the test harness would not see them.
//!
//! Phase mapping:
//!   * `cfg_NN_*` tests  -> rows 1..20 of CONFIGS.md   (Phase B, valid paths)
//!   * `err_*` tests     -> the boundary obligations of ERRORS.md (Phase C)
//!   * `sym_*` tests     -> symbol parity assertions of SYMBOLS.md (Phase D)

use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bindings used by the harness itself (not by the code under test).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn printf(fmt: *const c_char, ...) -> c_int;
    static mut stdout: *mut c_void;
}

const IOFBF: c_int = 0; // fully buffered
const IOLBF: c_int = 1; // line buffered
const IONBF: c_int = 2; // unbuffered

/// Serializes every test that touches fd 1 or the `stdout` FILE*.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects.
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = crate_root().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Every Rust `.so` we can find. Both the debug and the release cdylib are
/// tested when present: the debug profile keeps `overflow-checks` on and
/// `panic = "unwind"`, so it catches arithmetic that only *looks* like the C
/// because the release profile silently wrapped it.
fn rust_so_paths() -> Vec<PathBuf> {
    let root = crate_root();
    let mut v = Vec::new();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libdriver.so");
        if p.exists() {
            v.push(p);
        }
    }
    assert!(
        !v.is_empty(),
        "no Rust libdriver.so found under {}/target/{{release,debug}} -- run `cargo build --release` first",
        root.display()
    );
    v
}

type DriverFn = unsafe extern "C" fn(c_int);

struct Impls {
    c: Library,
    rust: Vec<(String, Library)>,
}

fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let c = unsafe { Library::new(c_so_path()) }.expect("dlopen C .so");
        let rust = rust_so_paths()
            .into_iter()
            .map(|p| {
                let name = format!(
                    "rust/{}",
                    p.parent().unwrap().file_name().unwrap().to_string_lossy()
                );
                let lib = unsafe { Library::new(&p) }.expect("dlopen Rust .so");
                (name, lib)
            })
            .collect();
        Impls { c, rust }
    })
}

fn sym<'l>(lib: &'l Library, name: &str) -> Symbol<'l, DriverFn> {
    unsafe { lib.get(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("dlsym {name:?} failed: {e}"))
}

// ---------------------------------------------------------------------------
// fd-level stdout capture.
// ---------------------------------------------------------------------------

fn tmp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    // Row labels are free-form prose and may contain '/', '*', spaces, etc.
    // Reduce them to a filename-safe slug so the capture file always lands
    // directly in the temp dir.
    let slug: String = tag
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(48)
        .collect();
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed),
        slug
    ))
}

/// Runs `body` with fd 1 redirected to a fresh temp file and returns every byte
/// that ended up there. `mode` selects the glibc buffering mode applied to the
/// `stdout` FILE* for the duration of `body`.
fn capture_with_mode<F: FnOnce()>(tag: &str, mode: c_int, body: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = tmp_path(tag);
    let file = std::fs::File::create(&path).expect("create temp capture file");
    let file_fd = {
        use std::os::fd::AsRawFd;
        file.as_raw_fd()
    };

    let out = unsafe {
        // Drain anything the harness itself left pending before we steal fd 1.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file_fd, 1) >= 0, "dup2 onto fd 1 failed");
        setvbuf(stdout, std::ptr::null_mut(), mode, 0);

        body();

        // Force out whatever the library left in the stdio buffer. This is the
        // same flush libc performs at process exit.
        fflush(std::ptr::null_mut());
        // Put the real stdout back and return it to default full buffering.
        assert!(dup2(saved, 1) >= 0, "dup2 restoring fd 1 failed");
        close(saved);
        setvbuf(stdout, std::ptr::null_mut(), IOFBF, 0);

        drop(file);
        std::fs::read(&path).expect("read temp capture file")
    };
    let _ = std::fs::remove_file(&path);
    out
}

fn capture<F: FnOnce()>(tag: &str, body: F) -> Vec<u8> {
    capture_with_mode(tag, IOFBF, body)
}

// ---------------------------------------------------------------------------
// Comparison helpers.
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Reports the first divergence in a way that names the offending input.
fn assert_same(row: &str, which: &str, inputs: &[i32], c_out: &[u8], r_out: &[u8]) {
    if c_out == r_out {
        return;
    }
    let c_lines: Vec<&[u8]> = c_out.split(|b| *b == b'\n').collect();
    let r_lines: Vec<&[u8]> = r_out.split(|b| *b == b'\n').collect();
    for i in 0..c_lines.len().max(r_lines.len()) {
        let cl = c_lines.get(i).copied().unwrap_or(b"<missing>");
        let rl = r_lines.get(i).copied().unwrap_or(b"<missing>");
        if cl != rl {
            let input = inputs
                .get(i)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "<beyond input list>".into());
            panic!(
                "{row}: {which} diverged from C at output line {i} (input x = {input}):\n  \
                 C    = {:?}\n  Rust = {:?}\n  (total lines: C {} / Rust {})",
                show(cl),
                show(rl),
                c_lines.len(),
                r_lines.len()
            );
        }
    }
    panic!(
        "{row}: {which} diverged from C but line-splitting matched.\n  C    = {}\n  Rust = {}",
        show(c_out),
        show(r_out)
    );
}

/// The core Phase-B driver: feed the identical `inputs` to the C `.so` and to
/// every Rust `.so`, in the identical buffering mode, and demand byte-identical
/// captured stdout.
fn check_inputs_mode(row: &str, mode: c_int, inputs: &[i32]) {
    let im = impls();
    let c_out = {
        let f = sym(&im.c, "driver");
        capture_with_mode(row, mode, || {
            for &x in inputs {
                unsafe { f(x) };
            }
        })
    };
    for (name, lib) in &im.rust {
        let f = sym(lib, "driver");
        let r_out = capture_with_mode(row, mode, || {
            for &x in inputs {
                unsafe { f(x) };
            }
        });
        assert_same(row, name, inputs, &c_out, &r_out);
    }
    // A non-empty input list must produce output; a silent no-op on both sides
    // would otherwise "match" vacuously.
    if !inputs.is_empty() {
        assert!(
            !c_out.is_empty(),
            "{row}: C produced no output for {} inputs -- capture is broken, \
             so a match would be meaningless",
            inputs.len()
        );
    }
}

fn check_inputs(row: &str, inputs: &[i32]) {
    check_inputs_mode(row, IOFBF, inputs);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed, reproducible.
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
    fn i32_any(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]`, computed in i64 so the range itself cannot overflow.
    fn i32_in(&mut self, lo: i64, hi: i64) -> i32 {
        let span = (hi - lo + 1) as u64;
        (lo + (self.next_u64() % span) as i64) as i32
    }
}

const I32_MAX: i64 = i32::MAX as i64;
const I32_MIN: i64 = i32::MIN as i64;

// ===========================================================================
// PHASE B -- CONFIGS.md rows 1..20
// ===========================================================================

/// Row 1 -- `x == 0` (the "zero length" analogue), fully buffered.
#[test]
fn cfg_01_zero() {
    check_inputs("CONFIGS row 1 (x == 0)", &[0]);
}

/// Row 2 -- exhaustive small positive domain.
#[test]
fn cfg_02_small_positive_exhaustive() {
    let inputs: Vec<i32> = (1..=1000).collect();
    check_inputs("CONFIGS row 2 (x in 1..=1000)", &inputs);
}

/// Row 3 -- exhaustive small negative domain.
#[test]
fn cfg_03_small_negative_exhaustive() {
    let inputs: Vec<i32> = (-1000..=-1).collect();
    check_inputs("CONFIGS row 3 (x in -1000..=-1)", &inputs);
}

/// Row 4 -- `x == -150`, the exact preimage of `y == 0`; hits glibc `%d`'s
/// zero special case, which is a distinct code path from any other value.
#[test]
fn cfg_04_y_exactly_zero() {
    check_inputs("CONFIGS row 4 (x == -150 -> y == 0)", &[-150]);
}

/// Row 5 -- `y` crossing zero, i.e. the `-` sign appearing/disappearing.
#[test]
fn cfg_05_sign_flip_neighbourhood() {
    check_inputs(
        "CONFIGS row 5 (sign flip of y)",
        &[-153, -152, -151, -150, -149, -148, -147],
    );
}

/// Row 6 -- every decimal-digit-count boundary of the printed `y`, both signs.
/// For each target `y` we solve `x = (y - 300) / 2` and also probe `y +- 1`, so
/// the 1->2->...->10 digit transitions of the conversion are all crossed.
#[test]
fn cfg_06_digit_count_boundaries() {
    let mut inputs = Vec::new();
    let mut targets: Vec<i64> = vec![0];
    let mut p: i64 = 1;
    for _ in 0..10 {
        for base in [p, p * 9] {
            for d in [-1i64, 0, 1] {
                targets.push(base + d);
                targets.push(-base + d);
            }
        }
        p *= 10;
    }
    targets.push(I32_MAX);
    targets.push(I32_MAX - 1);
    targets.push(I32_MIN);
    targets.push(I32_MIN + 1);
    for y in targets {
        // Recover an x that lands on (or adjacent to) this y; clamp into range.
        let x = (y - 300) / 2;
        if (I32_MIN..=I32_MAX).contains(&x) {
            inputs.push(x as i32);
        }
    }
    inputs.sort_unstable();
    inputs.dedup();
    check_inputs("CONFIGS row 6 (digit-count boundaries of y)", &inputs);
}

/// Row 7 -- the `2*x` signed-overflow boundary in both directions.
#[test]
fn cfg_07_mul_overflow_boundary() {
    let half_max = (i32::MAX / 2) as i64; // 1073741823
    let half_min = (i32::MIN / 2) as i64; // -1073741824
    let mut inputs = Vec::new();
    for base in [half_max, half_min] {
        for d in -3i64..=3 {
            let v = base + d;
            if (I32_MIN..=I32_MAX).contains(&v) {
                inputs.push(v as i32);
            }
        }
    }
    check_inputs("CONFIGS row 7 (2*x overflow boundary)", &inputs);
}

/// Row 8 -- the `y += 300` overflow boundary: `2*x + 300` crossing `INT_MAX`.
#[test]
fn cfg_08_add_overflow_boundary() {
    let mut inputs = Vec::new();
    // 2*x + 300 == INT_MAX  =>  x == (INT_MAX - 300) / 2
    let center = (I32_MAX - 300) / 2;
    for d in -8i64..=8 {
        let v = center + d;
        if (I32_MIN..=I32_MAX).contains(&v) {
            inputs.push(v as i32);
        }
    }
    // and the mirror end, 2*x + 300 == INT_MIN
    let center_lo = (I32_MIN - 300) / 2;
    for d in -8i64..=8 {
        let v = center_lo + d;
        if (I32_MIN..=I32_MAX).contains(&v) {
            inputs.push(v as i32);
        }
    }
    inputs.sort_unstable();
    inputs.dedup();
    check_inputs("CONFIGS row 8 (y += 300 overflow boundary)", &inputs);
}

/// Row 9 -- the domain extremes.
#[test]
fn cfg_09_domain_extremes() {
    check_inputs(
        "CONFIGS row 9 (INT_MIN / INT_MAX extremes)",
        &[
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 2,
            i32::MAX - 2,
            i32::MAX - 1,
            i32::MAX,
        ],
    );
}

/// Row 10 -- randomized sweep of the full `i32` domain, fixed seed.
#[test]
fn cfg_10_random_full_domain() {
    let mut rng = Rng::new(0xD1CE_5EED_0000_000A);
    let inputs: Vec<i32> = (0..20_000).map(|_| rng.i32_any()).collect();
    check_inputs("CONFIGS row 10 (random full i32 domain, 20000 draws)", &inputs);
}

/// Row 11 -- randomized band straddling the positive `2*x` overflow point.
#[test]
fn cfg_11_random_near_positive_overflow() {
    let mut rng = Rng::new(0xD1CE_5EED_0000_000B);
    let c = (i32::MAX / 2) as i64;
    let inputs: Vec<i32> = (0..8_000)
        .map(|_| rng.i32_in((c - 4096).max(I32_MIN), (c + 4096).min(I32_MAX)))
        .collect();
    check_inputs("CONFIGS row 11 (random near +2*x overflow, 8000 draws)", &inputs);
}

/// Row 12 -- randomized band straddling the negative `2*x` overflow point.
#[test]
fn cfg_12_random_near_negative_overflow() {
    let mut rng = Rng::new(0xD1CE_5EED_0000_000C);
    let c = (i32::MIN / 2) as i64;
    let inputs: Vec<i32> = (0..8_000)
        .map(|_| rng.i32_in((c - 4096).max(I32_MIN), (c + 4096).min(I32_MAX)))
        .collect();
    check_inputs("CONFIGS row 12 (random near -2*x overflow, 8000 draws)", &inputs);
}

/// Row 13 -- randomized narrow band around zero.
#[test]
fn cfg_13_random_near_zero() {
    let mut rng = Rng::new(0xD1CE_5EED_0000_000D);
    let inputs: Vec<i32> = (0..8_000).map(|_| rng.i32_in(-4096, 4096)).collect();
    check_inputs("CONFIGS row 13 (random near zero, 8000 draws)", &inputs);
}

/// Row 14 -- 5000 randomized calls accumulated into ONE stdout flush. Checks
/// byte ordering across an entire buffered run rather than per single call.
#[test]
fn cfg_14_many_calls_one_flush() {
    let mut rng = Rng::new(0xD1CE_5EED_0000_000E);
    let inputs: Vec<i32> = (0..5_000).map(|_| rng.i32_any()).collect();
    check_inputs_mode("CONFIGS row 14 (5000 calls, one flush)", IOFBF, &inputs);
}

/// Row 15 -- stdout line buffered.
#[test]
fn cfg_15_line_buffered() {
    let mut rng = Rng::new(0xD1CE_5EED_0000_000F);
    let inputs: Vec<i32> = (0..2_000).map(|_| rng.i32_any()).collect();
    check_inputs_mode("CONFIGS row 15 (line buffered)", IOLBF, &inputs);
}

/// Row 16 -- stdout unbuffered.
#[test]
fn cfg_16_unbuffered() {
    let mut rng = Rng::new(0xD1CE_5EED_0000_0010);
    let inputs: Vec<i32> = (0..2_000).map(|_| rng.i32_any()).collect();
    check_inputs_mode("CONFIGS row 16 (unbuffered)", IONBF, &inputs);
}

/// Row 17 -- the caller's own `printf` interleaved between library calls. If the
/// library wrote through a private buffer (e.g. Rust's `std::io::stdout`) instead
/// of the process stdio stream, the byte order here would differ from C's even
/// though each individual call produced the right digits.
#[test]
fn cfg_17_interleaved_with_caller_printf() {
    let im = impls();
    let inputs = [7i32, -150, i32::MAX, i32::MIN, 0];
    let body = |f: &Symbol<'_, DriverFn>| {
        let tag = CString::new("caller-%d\n").unwrap();
        for (i, &x) in inputs.iter().enumerate() {
            unsafe {
                printf(tag.as_ptr(), i as c_int);
                f(x);
            }
        }
    };
    let c_out = {
        let f = sym(&im.c, "driver");
        capture("row17c", || body(&f))
    };
    for (name, lib) in &im.rust {
        let f = sym(lib, "driver");
        let r_out = capture("row17r", || body(&f));
        assert_same(
            "CONFIGS row 17 (interleaved caller printf)",
            name,
            &[],
            &c_out,
            &r_out,
        );
    }
    // The interleaving must actually be present, or the test proves nothing.
    let text = String::from_utf8_lossy(&c_out);
    assert!(
        text.contains("caller-0\n314\n"),
        "row 17: expected caller output interleaved with library output, got {}",
        show(&c_out)
    );
}

/// Row 18 -- a raw unbuffered `write(2)` from the caller interleaved with the
/// buffered library output. Detects a library that bypasses stdio entirely (its
/// bytes would land before the caller's buffered ones instead of after).
#[test]
fn cfg_18_interleaved_with_raw_write() {
    let im = impls();
    let inputs = [1i32, 2, 3];
    let body = |f: &Symbol<'_, DriverFn>| {
        let buffered = CString::new("buffered\n").unwrap();
        for &x in inputs.iter() {
            unsafe {
                printf(buffered.as_ptr());
                f(x);
                let raw = b"RAW\n";
                write(1, raw.as_ptr() as *const c_void, raw.len());
            }
        }
    };
    let c_out = {
        let f = sym(&im.c, "driver");
        capture("row18c", || body(&f))
    };
    for (name, lib) in &im.rust {
        let f = sym(lib, "driver");
        let r_out = capture("row18r", || body(&f));
        assert_same(
            "CONFIGS row 18 (raw write vs buffered library output)",
            name,
            &[],
            &c_out,
            &r_out,
        );
    }
    assert!(
        c_out.windows(4).any(|w| w == b"RAW\n"),
        "row 18: raw write did not reach the capture file"
    );
}

/// Row 19 -- statelessness: the same inputs replayed C, Rust, C again must give
/// three identical transcripts. A hidden accumulator on either side (a `static`
/// in C, a `static mut`/`OnceLock` in Rust) would show up as drift on replay.
#[test]
fn cfg_19_stateless_replay() {
    let im = impls();
    let mut rng = Rng::new(0xD1CE_5EED_0000_0013);
    let inputs: Vec<i32> = (0..2_000).map(|_| rng.i32_any()).collect();
    let run = |f: &Symbol<'_, DriverFn>| {
        capture("row19", || {
            for &x in &inputs {
                unsafe { f(x) };
            }
        })
    };
    let c_f = sym(&im.c, "driver");
    let c_first = run(&c_f);
    for (name, lib) in &im.rust {
        let f = sym(lib, "driver");
        let r1 = run(&f);
        assert_same("CONFIGS row 19 (replay, pass 1)", name, &inputs, &c_first, &r1);
        let r2 = run(&f);
        assert_same("CONFIGS row 19 (replay, pass 2)", name, &inputs, &c_first, &r2);
    }
    let c_second = run(&c_f);
    assert_eq!(
        c_first, c_second,
        "CONFIGS row 19: the C library itself is not stateless across replays"
    );
}

/// Row 20 -- strided exhaustive-style sweep of the whole domain: covers every
/// magnitude decade and both signs uniformly with a prime-ish stride, so no
/// alignment of the stride with a power of two can hide a value-dependent bug.
#[test]
fn cfg_20_strided_full_domain_sweep() {
    const STRIDE: i64 = 3_999_971;
    let mut inputs = Vec::new();
    let mut v = I32_MIN;
    while v <= I32_MAX {
        inputs.push(v as i32);
        v += STRIDE;
    }
    inputs.push(i32::MAX);
    assert!(
        inputs.len() > 1000,
        "stride sweep produced only {} points",
        inputs.len()
    );
    check_inputs("CONFIGS row 20 (strided full-domain sweep)", &inputs);
}

// ===========================================================================
// PHASE C -- ERRORS.md
//
// The error-surface table is EMPTY: `void driver(int)` has no pointer
// parameter, no enum parameter, no return value, no branch, and no rejection
// path (see ERRORS.md for the mechanical grep that establishes this). What
// remains is the generic-boundary obligation, rows B1..B6 of ERRORS.md. B1
// (null pointer) and B5 (out-of-range enum) are structurally impossible for
// this signature and are asserted as such below rather than silently dropped.
// ===========================================================================

/// ERRORS.md B1 + B5 -- assert the *reason* these boundaries are inapplicable,
/// so the exemption is checked rather than assumed: the exported symbol must be
/// the one-`int`-argument function the header declares, meaning there is no
/// pointer to null and no enum to feed an out-of-range discriminant.
#[test]
fn err_no_pointer_or_enum_parameter_exists() {
    let header = std::fs::read_to_string(crate_root().join("../c_src/include/driver.h"))
        .expect("read driver.h");
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains("driver") && l.contains('(') && l.trim_end().ends_with(';'))
        .collect();
    assert_eq!(
        decls.len(),
        1,
        "driver.h declares {} functions, so ERRORS.md B1/B5 must be re-derived: {:?}",
        decls.len(),
        decls
    );
    let d = decls[0].replace(' ', "");
    assert_eq!(
        d, "voiddriver(intx);",
        "the public prototype changed; ERRORS.md B1/B5 (no pointer, no enum) no longer follow"
    );
    assert!(!decls[0].contains('*'), "a pointer parameter appeared: B1 now applies");
    assert!(!header.contains("enum"), "an enum appeared in the header: B5 now applies");
}

/// ERRORS.md B2 -- the zero value, the only "zero length" this API has.
#[test]
fn err_boundary_zero() {
    check_inputs("ERRORS B2 (zero value)", &[0]);
}

/// ERRORS.md B3 -- extremal magnitudes, the "oversized length" analogue.
#[test]
fn err_boundary_extremes() {
    check_inputs("ERRORS B3 (extremal magnitudes)", &[i32::MIN, i32::MAX]);
}

/// ERRORS.md B4 -- one step past the range where `2*x` is representable. In ISO
/// C this is UB; in the library as compiled it wraps, and the Rust must wrap
/// identically. `INT_MAX/2` is the last `x` whose double fits; `INT_MAX/2 + 1`
/// is one step past it.
#[test]
fn err_boundary_overflow_2x() {
    let hi = i32::MAX / 2;
    let lo = i32::MIN / 2;
    check_inputs(
        "ERRORS B4 (one step past representable 2*x)",
        &[lo - 1, lo, lo + 1, hi - 1, hi, hi + 1],
    );
}

/// ERRORS.md B4 -- one step past the range where `2*x + 300` is representable.
#[test]
fn err_boundary_overflow_plus300() {
    let center = ((i32::MAX as i64 - 300) / 2) as i32;
    let inputs: Vec<i32> = (-2..=2).map(|d| center + d).collect();
    check_inputs("ERRORS B4 (one step past representable 2*x + 300)", &inputs);
}

/// ERRORS.md B6 -- the library never checks `printf`'s return value, so a failed
/// write must be silently ignored. `/dev/full` makes every write fail with
/// ENOSPC. Both implementations must survive it, emit nothing, and leave the
/// process usable afterwards -- identically.
#[test]
fn err_printf_failure_ignored() {
    let dev_full = Path::new("/dev/full");
    if !dev_full.exists() {
        eprintln!("skipping err_printf_failure_ignored: /dev/full is unavailable");
        return;
    }
    let im = impls();
    let inputs = [0i32, 42, -150, i32::MIN, i32::MAX];

    // Returns (bytes that nonetheless escaped, fflush result) for one impl.
    let run = |f: &Symbol<'_, DriverFn>| -> (Vec<u8>, c_int) {
        let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let sink = std::fs::OpenOptions::new()
            .write(true)
            .open(dev_full)
            .expect("open /dev/full");
        let sink_fd = {
            use std::os::fd::AsRawFd;
            sink.as_raw_fd()
        };
        unsafe {
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0);
            assert!(dup2(sink_fd, 1) >= 0);
            // Unbuffered so each call's write hits /dev/full immediately and
            // the failure is observed inside the library call, not at flush.
            setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
            for &x in &inputs {
                f(x);
            }
            let flush_rc = fflush(stdout);
            assert!(dup2(saved, 1) >= 0);
            close(saved);
            setvbuf(stdout, std::ptr::null_mut(), IOFBF, 0);
            drop(sink);
            (Vec::new(), flush_rc)
        }
    };

    let c_f = sym(&im.c, "driver");
    let (c_bytes, c_flush) = run(&c_f);
    for (name, lib) in &im.rust {
        let f = sym(lib, "driver");
        let (r_bytes, r_flush) = run(&f);
        assert_eq!(
            c_bytes, r_bytes,
            "ERRORS B6: {name} escaped different bytes than C when stdout was full"
        );
        assert_eq!(
            c_flush, r_flush,
            "ERRORS B6: {name} left stdout in a different error state than C \
             (fflush returned {r_flush} vs C's {c_flush})"
        );
    }

    // Both survived; stdout must still work for everyone afterwards.
    let after = capture("after-full", || unsafe { c_f(1) });
    assert_eq!(after, b"302\n", "stdout unusable after the /dev/full test");
}

/// ERRORS.md, generic: calling the exported symbol many times in a row after a
/// write failure, and after buffering-mode churn, must not diverge. This pins
/// down that neither side latched an error flag the other did not.
#[test]
fn err_no_latched_state_after_failure() {
    let mut rng = Rng::new(0xD1CE_5EED_0000_00C6);
    let inputs: Vec<i32> = (0..500).map(|_| rng.i32_any()).collect();
    check_inputs_mode("ERRORS (no latched state, IONBF)", IONBF, &inputs);
    check_inputs_mode("ERRORS (no latched state, IOFBF)", IOFBF, &inputs);
    check_inputs_mode("ERRORS (no latched state, IOLBF)", IOLBF, &inputs);
}

// ===========================================================================
// PHASE D -- symbol parity, asserted in-test so it cannot drift
// ===========================================================================

fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` defines must also be defined by the Rust `.so`,
/// with the exact same name. The diff must be empty.
#[test]
fn sym_parity_c_subset_of_rust() {
    let c_syms = defined_dynamic_symbols(&c_so_path());
    assert!(
        c_syms.contains(&"driver".to_string()),
        "the C .so does not export `driver`; SYMBOLS.md must be re-derived. Got {c_syms:?}"
    );
    for so in rust_so_paths() {
        let r_syms = defined_dynamic_symbols(&so);
        let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "{} is missing {} symbol(s) exported by the C .so: {:?}\n\
             (C exports {:?}, Rust exports {:?})",
            so.display(),
            missing.len(),
            missing,
            c_syms,
            r_syms
        );
    }
}

/// The exported symbol must be reachable by `dlsym` under its exact C name from
/// both objects -- i.e. `#[no_mangle]` really took effect and the name was not
/// decorated.
#[test]
fn sym_dlsym_exact_name() {
    let im = impls();
    let _ = sym(&im.c, "driver");
    for (name, lib) in &im.rust {
        let f: Symbol<'_, DriverFn> = unsafe { lib.get(b"driver") }
            .unwrap_or_else(|e| panic!("{name}: dlsym(\"driver\") failed -- \
                 the #[no_mangle] export wrapper is missing or renamed: {e}"));
        let out = capture("dlsym", || unsafe { f(0) });
        assert_eq!(out, b"300\n", "{name}: dlsym'd `driver` produced {}", show(&out));
    }
    // A name that must NOT exist, guarding against a wildcard/aliasing export.
    for (name, lib) in &im.rust {
        let bogus: Result<Symbol<'_, DriverFn>, _> = unsafe { lib.get(b"driver_rs") };
        assert!(
            bogus.is_err(),
            "{name} exports an unexpected alias `driver_rs` that the C .so does not"
        );
    }
}
