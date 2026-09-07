//! Differential tests: load BOTH the C `libdriver.so` and the Rust
//! `libdriver.so` with `libloading` and compare their observable behaviour
//! through the FFI boundary.
//!
//! The only public symbol is `void driver(int x, int y)`, whose entire
//! observable effect is the bytes it writes to file descriptor 1. So every
//! test here captures raw fd 1 around the call and compares the captured
//! bytes byte-for-byte.
//!
//! Nothing is ever called directly in-process: both sides go through
//! `dlopen` + `dlsym`, so the `#[no_mangle] extern "C"` export wrapper is
//! exercised exactly as an external C caller would exercise it.

use std::ffi::c_int;
use std::ffi::c_void;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;

use libloading::Library;
use libloading::Symbol;

type DriverFn = unsafe extern "C" fn(c_int, c_int);

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// library loading
// ---------------------------------------------------------------------------

struct Libs {
    c: Library,
    rust: Library,
}

// SAFETY-ish: `Library` is Send+Sync already; the raw fn pointers we pull out
// are used only under `FD_LOCK`.
static LIBS: OnceLock<Libs> = OnceLock::new();

/// Serializes all fd-1 redirection, since cargo runs tests in parallel
/// threads inside one process and fd 1 is process-global.
static FD_LOCK: Mutex<()> = Mutex::new(());

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // target/<profile>/deps/<test-exe>  ->  target/<profile>/libdriver.so
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            let cand = profile_dir.join("libdriver.so");
            if cand.exists() {
                return cand;
            }
        }
    }
    for profile in ["release", "debug"] {
        let cand = manifest_dir().join("target").join(profile).join("libdriver.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found. Build it first:\n  \
         cd translation && cargo build --release\n\
         (or set RUST_DRIVER_SO=/path/to/libdriver.so)"
    );
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {c_path:?}; build it with \
             `cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`"
        );
        assert!(rust_path.exists(), "Rust shared library not found at {rust_path:?}");
        unsafe {
            Libs {
                c: Library::new(&c_path).expect("dlopen C libdriver.so"),
                rust: Library::new(&rust_path).expect("dlopen Rust libdriver.so"),
            }
        }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("dlsym driver in C .so") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("dlsym driver in Rust .so") }
}

// ---------------------------------------------------------------------------
// fd-1 capture
// ---------------------------------------------------------------------------

/// Runs `f` with file descriptor 1 redirected into a temp file and returns
/// every byte written to it (by C `stdio`, by `write(2)`, by anything).
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut tmp = tempfile();
    // Flush anything already pending on either runtime's stdout so it does
    // not land inside our capture window.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    f();

    // Flush the callee's C stdio buffers *while* fd 1 is still the temp file.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();

    assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
    unsafe { close(saved) };

    tmp.rewind().expect("rewind capture file");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read capture file");
    out
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture temp file");
    // Unlink immediately; the fd keeps it alive, so nothing is left behind.
    let _ = std::fs::remove_file(&path);
    file
}

// ---------------------------------------------------------------------------
// differential assertions
// ---------------------------------------------------------------------------

/// Calls `driver(x, y)` in the C `.so` and in the Rust `.so`, capturing each
/// separately, and asserts the two byte streams are identical.
fn assert_same(x: c_int, y: c_int) {
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cf = c_driver();
    let rf = rust_driver();
    let c_out = capture(|| unsafe { cf(x, y) });
    let r_out = capture(|| unsafe { rf(x, y) });
    if c_out != r_out {
        panic!(
            "divergence for driver({x}, {y}) [x=0x{:08x}, y=0x{:08x}]\n  C   : {:?} ({:?})\n  Rust: {:?} ({:?})",
            x as u32,
            y as u32,
            String::from_utf8_lossy(&c_out),
            c_out,
            String::from_utf8_lossy(&r_out),
            r_out
        );
    }
}

/// Same, but for a whole batch of calls inside a single capture window, so
/// that concatenation / buffering / flushing behaviour is compared too.
fn assert_same_batch(inputs: &[(c_int, c_int)]) {
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cf = c_driver();
    let rf = rust_driver();
    let c_out = capture(|| {
        for &(x, y) in inputs {
            unsafe { cf(x, y) };
        }
    });
    let r_out = capture(|| {
        for &(x, y) in inputs {
            unsafe { rf(x, y) };
        }
    });
    assert!(
        c_out == r_out,
        "batch divergence over {} calls (first input {:?})\n  C   : {:?}\n  Rust: {:?}",
        inputs.len(),
        inputs.first(),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64), fixed seed => reproducible
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
    fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// Uniform in `[lo, hi]` (inclusive), works across the full i32 range.
    fn range_i32(&mut self, lo: i64, hi: i64) -> c_int {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        (lo + (self.next_u64() % span) as i64) as c_int
    }
}

const IMIN: c_int = c_int::MIN;
const IMAX: c_int = c_int::MAX;

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// CONFIGS row 1: the minimal call, `(0, 0)`.
#[test]
fn cfg01_minimal_zero_zero() {
    assert_same(0, 0);
}

/// CONFIGS row 2: both operands small and non-negative (0..=9 cross product).
#[test]
fn cfg02_small_nonnegative_cross() {
    for x in 0..=9 {
        for y in 0..=9 {
            assert_same(x, y);
        }
    }
}

/// CONFIGS row 3: both operands small and non-positive.
#[test]
fn cfg03_small_negative_cross() {
    for x in -9..=0 {
        for y in -9..=0 {
            assert_same(x, y);
        }
    }
}

/// CONFIGS row 4: mixed signs, small magnitudes (-9..=9 cross product).
#[test]
fn cfg04_small_mixed_sign_cross() {
    for x in -9..=9 {
        for y in -9..=9 {
            assert_same(x, y);
        }
    }
}

/// CONFIGS row 5: the branch where `result = x | ~y` is non-negative,
/// i.e. `x >= 0 && y < 0`, so `printf` emits no sign.
#[test]
fn cfg05_nonnegative_result_branch_random() {
    let mut rng = Rng::new(0x5EED_0005);
    for _ in 0..3000 {
        let x = rng.range_i32(0, IMAX as i64);
        let y = rng.range_i32(IMIN as i64, -1);
        assert!((x | !y) >= 0, "row 5 precondition");
        assert_same(x, y);
    }
}

/// CONFIGS row 6: the three sub-cases where `result` is negative.
#[test]
fn cfg06_negative_result_branch_random() {
    let mut rng = Rng::new(0x5EED_0006);
    for _ in 0..1500 {
        // x < 0, y < 0
        let x = rng.range_i32(IMIN as i64, -1);
        let y = rng.range_i32(IMIN as i64, -1);
        assert!((x | !y) < 0);
        assert_same(x, y);
    }
    for _ in 0..1500 {
        // x < 0, y >= 0
        let x = rng.range_i32(IMIN as i64, -1);
        let y = rng.range_i32(0, IMAX as i64);
        assert!((x | !y) < 0);
        assert_same(x, y);
    }
    for _ in 0..1500 {
        // x >= 0, y >= 0
        let x = rng.range_i32(0, IMAX as i64);
        let y = rng.range_i32(0, IMAX as i64);
        assert!((x | !y) < 0);
        assert_same(x, y);
    }
}

/// CONFIGS row 7: every decimal width 1..=10 of a non-negative result.
/// `y = INT_MIN` gives `~y = INT_MAX`, so `x | ~y == INT_MAX`; instead use
/// `y = -1 - r` so that `~y == r` and `x = 0`, giving exactly `result == r`.
#[test]
fn cfg07_every_nonnegative_width() {
    let widths: [c_int; 11] = [0, 9, 99, 999, 9999, 99999, 999999, 9999999, 99999999, 999999999, IMAX];
    for &r in &widths {
        // ~y == r  =>  y == !r
        let y = !r;
        assert_eq!(0 | !y, r);
        assert_same(0, y);
        // and via the x side: x == r, y == 0 gives x | ~0 == -1, so use y = !r again
        assert_same(r, !r);
    }
    // also the smallest positive widths individually
    for r in [1, 10, 100, 1000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000, 1_000_000_000] {
        assert_same(0, !r);
    }
}

/// CONFIGS row 8: negative results of every decimal width, including the
/// 11-byte worst case `-2147483648`.
#[test]
fn cfg08_every_negative_width() {
    let negatives: [c_int; 11] = [
        -1,
        -9,
        -99,
        -999,
        -9999,
        -99999,
        -999999,
        -9_999_999,
        -99_999_999,
        -999_999_999,
        IMIN,
    ];
    for &r in &negatives {
        let y = !r; // ~y == r, x == 0 => result == r
        assert_eq!(0 | !y, r);
        assert_same(0, y);
    }
    // INT_MIN as a result also via x = INT_MIN, y = INT_MAX
    assert_eq!(IMIN | !IMAX, IMIN);
    assert_same(IMIN, IMAX);
    // -1 via the documented (0,0) path
    assert_same(0, 0);
}

const BOUNDARY_OPS: [c_int; 13] = [
    IMIN,
    IMIN + 1,
    -65537,
    -256,
    -2,
    -1,
    0,
    1,
    2,
    255,
    65536,
    IMAX - 1,
    IMAX,
];

/// CONFIGS row 9: cross product of boundary operand magnitudes.
#[test]
fn cfg09_boundary_operand_cross() {
    for &x in &BOUNDARY_OPS {
        for &y in &BOUNDARY_OPS {
            assert_same(x, y);
        }
    }
}

const BIT_PATTERNS: [u32; 8] = [
    0x0000_0000,
    0xFFFF_FFFF,
    0x8000_0000,
    0x7FFF_FFFF,
    0x5555_5555,
    0xAAAA_AAAA,
    0x0000_0001,
    0xFFFF_FFFE,
];

/// CONFIGS row 10: cross product of interesting bit-pattern shapes.
#[test]
fn cfg10_bit_pattern_cross() {
    for &xb in &BIT_PATTERNS {
        for &yb in &BIT_PATTERNS {
            assert_same(xb as c_int, yb as c_int);
        }
    }
}

/// CONFIGS row 11: single bit set in `x`, single bit clear in `y`.
#[test]
fn cfg11_single_bit_cross() {
    for i in 0..32u32 {
        for j in 0..32u32 {
            let x = (1u32 << i) as c_int;
            let y = !(1u32 << j) as c_int;
            assert_same(x, y);
        }
    }
}

/// CONFIGS row 12: fully randomized 32-bit inputs (large sample).
#[test]
fn cfg12_fully_random() {
    let mut rng = Rng::new(0x5EED_0012);
    // Batched to keep the number of fd redirections reasonable while still
    // covering many thousands of distinct inputs.
    let mut batch = Vec::with_capacity(500);
    for _ in 0..40 {
        batch.clear();
        for _ in 0..500 {
            batch.push((rng.next_i32(), rng.next_i32()));
        }
        assert_same_batch(&batch);
    }
}

/// CONFIGS row 13: many consecutive calls in one capture window — the
/// concatenated byte stream (and hence newline placement / buffering) must
/// match exactly.
#[test]
fn cfg13_consecutive_calls_single_window() {
    let mut rng = Rng::new(0x5EED_0013);
    let inputs: Vec<(c_int, c_int)> = (0..500).map(|_| (rng.next_i32(), rng.next_i32())).collect();
    assert_same_batch(&inputs);

    // Sanity-check the *shape* of the stream: one line per call.
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cf = c_driver();
    let out = capture(|| {
        for &(x, y) in &inputs {
            unsafe { cf(x, y) };
        }
    });
    assert_eq!(
        out.iter().filter(|&&b| b == b'\n').count(),
        inputs.len(),
        "C emits exactly one newline per call"
    );
    assert_eq!(*out.last().unwrap(), b'\n', "stream ends with a newline");
}

/// CONFIGS row 14: interleaved ordering (C-then-Rust and Rust-then-C) inside
/// one capture window each, to rule out order-dependent stdio state.
#[test]
fn cfg14_interleaved_order() {
    let mut rng = Rng::new(0x5EED_0014);
    let inputs: Vec<(c_int, c_int)> = (0..200).map(|_| (rng.next_i32(), rng.next_i32())).collect();

    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cf = c_driver();
    let rf = rust_driver();

    // C first, then Rust: the second half must equal the first half.
    let out = capture(|| {
        for &(x, y) in &inputs {
            unsafe { cf(x, y) };
        }
        for &(x, y) in &inputs {
            unsafe { rf(x, y) };
        }
    });
    let text = String::from_utf8(out).expect("ascii output");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), inputs.len() * 2);
    assert_eq!(
        &lines[..inputs.len()],
        &lines[inputs.len()..],
        "C-then-Rust halves differ"
    );

    // Rust first, then C, and per-call alternation.
    let out2 = capture(|| {
        for &(x, y) in &inputs {
            unsafe { rf(x, y) };
            unsafe { cf(x, y) };
        }
    });
    let text2 = String::from_utf8(out2).expect("ascii output");
    let lines2: Vec<&str> = text2.lines().collect();
    assert_eq!(lines2.len(), inputs.len() * 2);
    for pair in lines2.chunks(2) {
        assert_eq!(pair[0], pair[1], "alternating Rust/C outputs differ");
    }
}

/// CONFIGS row 15: for each possible output byte length, many randomized
/// inputs that produce exactly that length.
#[test]
fn cfg15_per_output_length_sweep() {
    let mut rng = Rng::new(0x5EED_0015);
    // len includes the trailing newline: 1 digit + '\n' = 2 .. 11 chars + '\n' = 12
    for target_len in 2..=12usize {
        let mut found = 0;
        let mut attempts = 0u64;
        let mut batch = Vec::new();
        while found < 200 && attempts < 4_000_000 {
            attempts += 1;
            let (x, y) = (rng.next_i32(), rng.next_i32());
            let r = x | !y;
            if format!("{r}\n").len() == target_len {
                batch.push((x, y));
                found += 1;
            }
        }
        // Some lengths (very short outputs) are astronomically rare under a
        // uniform sample; fall back to constructing them exactly.
        if found == 0 {
            let digits = target_len - 1;
            let base: i64 = 10i64.pow((digits - 1) as u32);
            let r = if digits == 1 { 5 } else { base as i64 + 7 };
            let r = r.min(IMAX as i64) as c_int;
            batch.push((0, !r));
        }
        assert_same_batch(&batch);
    }
}

/// CONFIGS row 16: repeat determinism — calling twice yields two identical
/// copies in both libraries.
#[test]
fn cfg16_repeat_determinism() {
    let mut rng = Rng::new(0x5EED_0016);
    for _ in 0..50 {
        let (x, y) = (rng.next_i32(), rng.next_i32());
        let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let cf = c_driver();
        let rf = rust_driver();
        let c_out = capture(|| {
            unsafe { cf(x, y) };
            unsafe { cf(x, y) };
        });
        let r_out = capture(|| {
            unsafe { rf(x, y) };
            unsafe { rf(x, y) };
        });
        assert_eq!(c_out, r_out, "repeat divergence for ({x}, {y})");
        let half = c_out.len() / 2;
        assert_eq!(&c_out[..half], &c_out[half..], "C output not self-identical");
    }
}

// ===========================================================================
// Phase C — ERRORS.md generic boundary rows (the error table itself is empty:
// the C code contains no rejection, error return, assert, or null check)
// ===========================================================================

/// G1: zero / identity values.
#[test]
fn err_g1_zero_and_identity() {
    for &(x, y) in &[(0, 0), (0, -1), (-1, 0), (-1, -1)] {
        assert_same(x, y);
    }
}

/// G2: extremal `int` values — a value "one past" these is not representable
/// in the ABI, so these *are* the range boundaries.
#[test]
fn err_g2_extremal_cross_product() {
    let extremes: [c_int; 9] = [IMIN, IMIN + 1, -2, -1, 0, 1, 2, IMAX - 1, IMAX];
    for &x in &extremes {
        for &y in &extremes {
            assert_same(x, y);
        }
    }
}

/// G3: operands adjacent to signed-overflow territory, and inputs whose
/// result is exactly `INT_MIN` (the value with no positive counterpart).
#[test]
fn err_g3_overflow_adjacent() {
    for &(x, y) in &[
        (IMIN, IMAX),
        (IMAX, IMIN),
        (IMIN, IMIN),
        (IMAX, IMAX),
        (IMIN, 2_147_483_646),
        (0, IMAX),
        (IMAX, 0),
        (0, IMIN),
        (IMIN, 0),
    ] {
        assert_same(x, y);
    }
    // results pinned to INT_MIN and INT_MAX
    assert_eq!(IMIN | !IMAX, IMIN);
    assert_eq!(0 | !IMIN, IMAX);
    assert_same(IMIN, IMAX);
    assert_same(0, IMIN);
}

/// G4: raw 32-bit bit patterns reinterpreted as `int` — the analogue of
/// "out-of-range enum value" for an API whose parameters are plain `int`:
/// every bit pattern is a legal input the C accepts, including ones a
/// well-behaved caller would never produce.
#[test]
fn err_g4_raw_bit_patterns() {
    let mut rng = Rng::new(0xBAD_C0DE);
    let mut batch = Vec::with_capacity(500);
    for _ in 0..40 {
        batch.clear();
        for _ in 0..500 {
            let xb = rng.next_u64() as u32;
            let yb = rng.next_u64() as u32;
            batch.push((xb as c_int, yb as c_int));
        }
        assert_same_batch(&batch);
    }
}

/// G5: dirty upper half of the 64-bit argument registers. A C `int`
/// parameter only occupies the low 32 bits; both implementations must ignore
/// the garbage above it identically. We force this by calling through a
/// pointer typed with 64-bit parameters.
#[test]
fn err_g5_register_upper_half() {
    type DriverWide = unsafe extern "C" fn(i64, i64);
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cf: Symbol<DriverWide> = unsafe { libs().c.get(b"driver\0").unwrap() };
    let rf: Symbol<DriverWide> = unsafe { libs().rust.get(b"driver\0").unwrap() };

    let mut rng = Rng::new(0x5EED_00A5);
    let cases: Vec<(i64, i64)> = (0..200)
        .map(|_| {
            let x = rng.next_i32();
            let y = rng.next_i32();
            let dirty_x = ((0xDEAD_BEEFu64 << 32) | (x as u32 as u64)) as i64;
            let dirty_y = ((0xFEED_FACEu64 << 32) | (y as u32 as u64)) as i64;
            (dirty_x, dirty_y)
        })
        .collect();

    let c_out = capture(|| {
        for &(x, y) in &cases {
            unsafe { cf(x, y) };
        }
    });
    let r_out = capture(|| {
        for &(x, y) in &cases {
            unsafe { rf(x, y) };
        }
    });
    assert_eq!(
        c_out, r_out,
        "divergence when the upper 32 bits of the argument registers are dirty"
    );
}

/// G6: the API has no pointer parameters, so there is no null-pointer input
/// to construct. Documented and asserted against the header text.
#[test]
fn err_g6_no_pointer_params() {
    let header = std::fs::read_to_string(
        manifest_dir()
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("read driver.h");
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains("driver(") )
        .collect();
    assert_eq!(decls.len(), 1, "exactly one declaration in the header: {decls:?}");
    assert!(
        !decls[0].contains('*'),
        "no pointer parameters, so no null-pointer error path exists: {:?}",
        decls[0]
    );
    // The only remaining "invalid input" class is arbitrary int bit patterns,
    // covered by err_g4 / err_g5.
}

/// Also assert the two `.so`s export the same symbol name set (Phase D, kept
/// here so it runs on every `cargo test`).
#[test]
fn symbol_parity_driver_exported_by_both() {
    let _c: Symbol<DriverFn> = unsafe { libs().c.get(b"driver\0").expect("C exports driver") };
    let _r: Symbol<DriverFn> = unsafe { libs().rust.get(b"driver\0").expect("Rust exports driver") };
}
