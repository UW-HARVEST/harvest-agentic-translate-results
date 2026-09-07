// Differential test harness: loads BOTH the C `libdriver.so` and the Rust
// `libdriver.so` through `libloading` and compares their observable behaviour
// through the FFI boundary. The Rust implementation is NEVER called directly —
// only via its exported `#[no_mangle]` symbol in the built `.so`, exactly as an
// external C consumer would.
//
// The library's only public symbol is `void driver(int, int, int)`, which
// returns `void`, so ALL observable behaviour is the bytes it writes to stdout.
// Comparison is therefore done by redirecting fd 1 to a temp file around each
// call and diffing the captured bytes.

use std::ffi::{c_int, c_void};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed to capture what the loaded libraries write to stdout.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, including the `stdout`
    /// FILE shared by this test binary, the C `.so` and the Rust `.so`.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Serializes whole test bodies: fd 1 is process-wide state and each test
/// `dlopen`s/`dlclose`s its own handles (which is what resets the libraries'
/// file-scope `static int y`).
fn serial_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Redirect fd 1 to a fresh temp file, run `f`, flush, restore, return bytes.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        n
    ));

    let file = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture temp file");

    let bytes = unsafe {
        // Do not let anything already buffered leak into the capture.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore fd 1 failed");
        close(saved);

        std::fs::read(&path).expect("read capture temp file")
    };

    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared libraries.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release cdylib (the artifact an external consumer links), fall
    // back to the debug cdylib that `cargo test` itself builds.
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}. Build it with: cargo build --release",
        base.display()
    );
}

type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

/// A loaded library plus its resolved `driver` symbol. Keeps the `Library`
/// alive alongside the function pointer.
struct Lib {
    _lib: libloading::Library,
    driver: DriverFn,
    which: &'static str,
}

impl Lib {
    fn load(path: &std::path::Path, which: &'static str) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            let sym: libloading::Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("`driver` not exported by {}: {e}", path.display()));
            let driver = *sym;
            Lib {
                _lib: lib,
                driver,
                which,
            }
        }
    }

    fn call(&self, x: i32, y: i32, z: i32) {
        unsafe { (self.driver)(x, y, z) }
    }
}

/// Freshly `dlopen` both libraries. Returned handles are dropped (`dlclose`d)
/// at end of scope, which is what makes each test see the pristine
/// `static int y = 123` initial state.
fn load_pair() -> (Lib, Lib) {
    let c = Lib::load(&c_so_path(), "C");
    let r = Lib::load(&rust_so_path(), "Rust");
    (c, r)
}

// ---------------------------------------------------------------------------
// Comparison helpers.
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return Some(i);
        }
    }
    if a.len() == b.len() { None } else { Some(n) }
}

/// Core differential assertion for a single call: both libraries are called
/// with the same arguments on freshly loaded handles' current state, and their
/// stdout bytes must be identical.
fn assert_same_call(row: &str, c: &Lib, r: &Lib, x: i32, y: i32, z: i32) -> Vec<u8> {
    let c_out = capture(|| c.call(x, y, z));
    let r_out = capture(|| r.call(x, y, z));
    if c_out != r_out {
        panic!(
            "[{row}] divergence for driver({x}, {y}, {z})\n  C   ({}): \"{}\"\n  Rust({}): \"{}\"\n  first differing byte index: {:?}",
            c.which,
            show(&c_out),
            r.which,
            show(&r_out),
            first_diff(&c_out, &r_out)
        );
    }
    c_out
}

/// Batched differential assertion: runs the whole input list against C in one
/// capture and against Rust in one capture, then compares the two complete
/// transcripts. This is stricter than per-call comparison because any
/// divergence in the persisted `static y` shows up in later calls too. On
/// mismatch it re-runs per input to name the exact offending arguments.
fn assert_same_batch(row: &str, inputs: &[(i32, i32, i32)]) {
    let (c, r) = load_pair();
    let c_out = capture(|| {
        for &(x, y, z) in inputs {
            c.call(x, y, z);
        }
    });
    let r_out = capture(|| {
        for &(x, y, z) in inputs {
            r.call(x, y, z);
        }
    });

    if c_out == r_out {
        return;
    }

    // Localize the divergence on fresh handles.
    let (c2, r2) = load_pair();
    for &(x, y, z) in inputs {
        assert_same_call(row, &c2, &r2, x, y, z);
    }
    panic!(
        "[{row}] batch transcripts diverged over {} calls (index {:?}) although every single call matched in isolation — the persisted `static y` state must differ.\n  C len {} / Rust len {}",
        inputs.len(),
        first_diff(&c_out, &r_out),
        c_out.len(),
        r_out.len()
    );
}

/// Assert both libraries produce exactly the tabulated bytes from ERRORS.md,
/// i.e. the same error *sentinel*, not merely "both failed somehow".
fn assert_same_and_expected(row: &str, x: i32, y: i32, z: i32, expected: &str) {
    let (c, r) = load_pair();
    let out = assert_same_call(row, &c, &r, x, y, z);
    assert_eq!(
        out,
        expected.as_bytes(),
        "[{row}] driver({x}, {y}, {z}): both libraries agreed on \"{}\" but that is not the expected C behaviour \"{}\"",
        show(&out),
        expected.escape_debug()
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64), fixed seed for reproducibility.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new() -> Rng {
        Rng(0x2545F491_4F6CDD1D)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Any `i32` except `avoid` — used to generate "invalid" arguments.
    fn i32_not(&mut self, avoid: i32) -> i32 {
        loop {
            let v = self.i32();
            if v != avoid {
                return v;
            }
        }
    }
    /// Small value near the sentinels, so guards and the success path all fire.
    fn small(&mut self) -> i32 {
        (self.next_u64() % 5) as i32
    }
}

const REPS: usize = 200;

// ===========================================================================
// Phase B — valid-path differential tests, one test per CONFIGS.md row.
// ===========================================================================

// CONFIGS.md row 1: x==1, y==2, z==3 — the success path.
#[test]
fn cfg_row1_all_valid_success() {
    let _g = serial_guard();
    assert_same_and_expected("CONFIGS row 1", 1, 2, 3, "Ok!\nResult: 0\n");
    // Repeat on the same handles: the success path must stay stable.
    assert_same_batch("CONFIGS row 1", &[(1, 2, 3); 32]);
}

// CONFIGS.md row 2: x==1, y==2, z!=3 (randomized).
#[test]
fn cfg_row2_valid_x_valid_y_invalid_z() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..REPS).map(|_| (1, 2, rng.i32_not(3))).collect();
    assert_same_batch("CONFIGS row 2", &inputs);
}

// CONFIGS.md row 3: x==1, y!=2 (randomized), z==3.
#[test]
fn cfg_row3_valid_x_invalid_y_valid_z() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..REPS).map(|_| (1, rng.i32_not(2), 3)).collect();
    assert_same_batch("CONFIGS row 3", &inputs);
}

// CONFIGS.md row 4: x==1, y!=2, z!=3 (both randomized).
#[test]
fn cfg_row4_valid_x_invalid_y_invalid_z() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..REPS)
        .map(|_| (1, rng.i32_not(2), rng.i32_not(3)))
        .collect();
    assert_same_batch("CONFIGS row 4", &inputs);
}

// CONFIGS.md row 5: x!=1 (randomized), y==2, z==3.
#[test]
fn cfg_row5_invalid_x_valid_y_valid_z() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..REPS).map(|_| (rng.i32_not(1), 2, 3)).collect();
    assert_same_batch("CONFIGS row 5", &inputs);
}

// CONFIGS.md row 6: x!=1, y==2, z!=3.
#[test]
fn cfg_row6_invalid_x_valid_y_invalid_z() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..REPS)
        .map(|_| (rng.i32_not(1), 2, rng.i32_not(3)))
        .collect();
    assert_same_batch("CONFIGS row 6", &inputs);
}

// CONFIGS.md row 7: x!=1, y!=2, z==3.
#[test]
fn cfg_row7_invalid_x_invalid_y_valid_z() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..REPS)
        .map(|_| (rng.i32_not(1), rng.i32_not(2), 3))
        .collect();
    assert_same_batch("CONFIGS row 7", &inputs);
}

// CONFIGS.md row 8: all three invalid, randomized.
#[test]
fn cfg_row8_all_invalid() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..REPS)
        .map(|_| (rng.i32_not(1), rng.i32_not(2), rng.i32_not(3)))
        .collect();
    assert_same_batch("CONFIGS row 8", &inputs);
}

// CONFIGS.md row 9: exhaustive 11^3 boundary cross-product.
#[test]
fn cfg_row9_boundary_cross_product() {
    let _g = serial_guard();
    const VALS: [i32; 11] = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut inputs = Vec::with_capacity(VALS.len().pow(3));
    for &x in &VALS {
        for &y in &VALS {
            for &z in &VALS {
                inputs.push((x, y, z));
            }
        }
    }
    assert_eq!(inputs.len(), 1331);
    assert_same_batch("CONFIGS row 9", &inputs);
}

// CONFIGS.md row 10: unconstrained randomized full-i32 cross-product.
#[test]
fn cfg_row10_random_full_range() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..4000)
        .map(|_| (rng.i32(), rng.i32(), rng.i32()))
        .collect();
    assert_same_batch("CONFIGS row 10", &inputs);
}

// CONFIGS.md row 11: randomized near the sentinels so every guard and the
// success path fire many times.
#[test]
fn cfg_row11_random_near_sentinels() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..2000)
        .map(|_| (rng.small(), rng.small(), rng.small()))
        .collect();
    // Sanity: this distribution really does hit the success path.
    assert!(inputs.iter().any(|&(x, y, z)| (x, y, z) == (1, 2, 3)));
    assert_same_batch("CONFIGS row 11", &inputs);
}

// CONFIGS.md row 12: shape "one call" on freshly loaded handles.
#[test]
fn cfg_row12_single_first_ever_call() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    for _ in 0..32 {
        let (x, y, z) = (rng.small(), rng.small(), rng.small());
        // Fresh dlopen per iteration: the initial `static int y = 123` is in
        // place for each of these first-ever calls.
        let (c, r) = load_pair();
        assert_same_call("CONFIGS row 12", &c, &r, x, y, z);
    }
}

// CONFIGS.md row 13: shape "two calls" — latched `y` must not leak between
// calls, in both orders.
#[test]
fn cfg_row13_two_call_sequences() {
    let _g = serial_guard();
    // failure then success, and success then failure, plus the y-latching pairs
    // that would expose a stale static.
    let pairs: [[(i32, i32, i32); 2]; 8] = [
        [(0, 0, 0), (1, 2, 3)],
        [(1, 2, 3), (0, 0, 0)],
        [(1, 9, 3), (1, 2, 3)],
        [(1, 2, 3), (1, 9, 3)],
        [(1, 2, 9), (1, 2, 3)],
        [(9, 2, 3), (1, 2, 3)],
        [(1, 2, 3), (1, 2, 3)],
        [(i32::MIN, i32::MAX, 0), (1, 2, 3)],
    ];
    for seq in pairs {
        assert_same_batch("CONFIGS row 13", &seq);
    }
}

// CONFIGS.md row 14: shape "many calls" — one long transcript on one pair of
// handles, compared as a whole.
#[test]
fn cfg_row14_long_stateful_sequence() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..3000)
        .map(|_| {
            // Mix of near-sentinel and wild values within one sequence.
            if rng.next_u64() % 2 == 0 {
                (rng.small(), rng.small(), rng.small())
            } else {
                (rng.i32(), rng.i32(), rng.i32())
            }
        })
        .collect();
    assert_same_batch("CONFIGS row 14", &inputs);
}

// CONFIGS.md row 15: the `%d` conversion at every producible value, and the
// exact line counts of the success vs fail paths.
#[test]
fn cfg_row15_result_formatting_and_line_counts() {
    let _g = serial_guard();
    let cases = [
        ((1, 2, 3), 0usize, 2usize), // Ok! + Result:  -> 2 lines, code 0
        ((0, 2, 3), 1, 3),           // Error + Operation failed + Result: -> 3
        ((1, 0, 3), 2, 3),
        ((1, 2, 0), 3, 3),
    ];
    for ((x, y, z), code, lines) in cases {
        let (c, r) = load_pair();
        let out = assert_same_call("CONFIGS row 15", &c, &r, x, y, z);
        let text = String::from_utf8(out).expect("driver output is ASCII");
        assert_eq!(
            text.lines().count(),
            lines,
            "driver({x}, {y}, {z}) line count; got \"{}\"",
            text.escape_debug()
        );
        assert!(
            text.ends_with(&format!("Result: {code}\n")),
            "driver({x}, {y}, {z}) should report Result: {code}; got \"{}\"",
            text.escape_debug()
        );
    }
}

// CONFIGS.md row 16: interleave C and Rust calls on the SAME redirected stdout
// stream. The C compiler rewrote the conversion-free `printf`s into `puts`
// while the Rust build calls `printf`; interleaving proves both go through the
// same libc stdio buffer and emit identical bytes in identical stream order.
#[test]
fn cfg_row16_interleaved_on_one_stream() {
    let _g = serial_guard();
    let mut rng = Rng::new();
    let inputs: Vec<_> = (0..256)
        .map(|_| (rng.small(), rng.small(), rng.small()))
        .collect();

    let (c, r) = load_pair();
    // Reference: the same call made twice against C only.
    let cc = capture(|| {
        for &(x, y, z) in &inputs {
            c.call(x, y, z);
            c.call(x, y, z);
        }
    });
    // Under test: C then Rust, alternating, in one stream.
    let cr = capture(|| {
        for &(x, y, z) in &inputs {
            c.call(x, y, z);
            r.call(x, y, z);
        }
    });
    assert_eq!(
        cr,
        cc,
        "[CONFIGS row 16] interleaved C/Rust transcript differs from C/C reference at byte {:?}",
        first_diff(&cr, &cc)
    );
    assert!(!cc.is_empty());
}

// ===========================================================================
// Phase C — error-path differential tests, one test per ERRORS.md row.
// Each asserts BOTH libraries reject identically AND with the exact sentinel
// (`Result: N`) the C source produces.
// ===========================================================================

// ERRORS.md row 1: x != 1.
#[test]
fn err_row1_x_not_1() {
    let _g = serial_guard();
    const EXPECT: &str = "Error: x != 1\nOperation failed\nResult: 1\n";
    assert_same_and_expected("ERRORS row 1", 0, 2, 3, EXPECT);
    assert_same_and_expected("ERRORS row 1", 2, 2, 3, EXPECT);

    // Randomized: any x != 1 must give exactly this, regardless of y and z.
    let mut rng = Rng::new();
    let (c, r) = load_pair();
    for _ in 0..REPS {
        let (x, y, z) = (rng.i32_not(1), rng.i32(), rng.i32());
        let out = assert_same_call("ERRORS row 1", &c, &r, x, y, z);
        assert_eq!(
            out,
            EXPECT.as_bytes(),
            "driver({x}, {y}, {z}) should take the x-guard"
        );
    }
}

// ERRORS.md row 2: x == 1 && y != 2.
#[test]
fn err_row2_y_not_2() {
    let _g = serial_guard();
    const EXPECT: &str = "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
    assert_same_and_expected("ERRORS row 2", 1, 1, 3, EXPECT);
    assert_same_and_expected("ERRORS row 2", 1, 3, 3, EXPECT);

    let mut rng = Rng::new();
    let (c, r) = load_pair();
    for _ in 0..REPS {
        let (y, z) = (rng.i32_not(2), rng.i32());
        let out = assert_same_call("ERRORS row 2", &c, &r, 1, y, z);
        assert_eq!(
            out,
            EXPECT.as_bytes(),
            "driver(1, {y}, {z}) should take the y-guard"
        );
    }
}

// ERRORS.md row 3: x == 1 && y == 2 && z != 3.
#[test]
fn err_row3_z_not_3() {
    let _g = serial_guard();
    const EXPECT: &str = "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n";
    assert_same_and_expected("ERRORS row 3", 1, 2, 2, EXPECT);
    assert_same_and_expected("ERRORS row 3", 1, 2, 4, EXPECT);

    let mut rng = Rng::new();
    let (c, r) = load_pair();
    for _ in 0..REPS {
        let z = rng.i32_not(3);
        let out = assert_same_call("ERRORS row 3", &c, &r, 1, 2, z);
        assert_eq!(
            out,
            EXPECT.as_bytes(),
            "driver(1, 2, {z}) should take the z-guard"
        );
    }
}

// ERRORS.md row 4: masking — x invalid AND y invalid AND z invalid prints only
// the x message (the `fail:` label is shared, so never more than 3 lines).
#[test]
fn err_row4_masking_order() {
    let _g = serial_guard();
    const EXPECT: &str = "Error: x != 1\nOperation failed\nResult: 1\n";
    let mut rng = Rng::new();
    let (c, r) = load_pair();
    for _ in 0..REPS {
        let (x, y, z) = (rng.i32_not(1), rng.i32_not(2), rng.i32_not(3));
        let out = assert_same_call("ERRORS row 4", &c, &r, x, y, z);
        assert_eq!(out, EXPECT.as_bytes());
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 3, "shared fail: label prints 3 lines");
        assert!(!text.contains("y != 2"));
        assert!(!text.contains("z != 3"));
    }
}

// ERRORS.md row 5: y guard masks the z guard.
#[test]
fn err_row5_masking_y_before_z() {
    let _g = serial_guard();
    const EXPECT: &str = "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
    let mut rng = Rng::new();
    let (c, r) = load_pair();
    for _ in 0..REPS {
        let (y, z) = (rng.i32_not(2), rng.i32_not(3));
        let out = assert_same_call("ERRORS row 5", &c, &r, 1, y, z);
        assert_eq!(out, EXPECT.as_bytes());
        assert!(!String::from_utf8(out).unwrap().contains("z != 3"));
    }
}

// ERRORS.md row 6: x one step past / far past its single valid value.
#[test]
fn err_row6_x_boundaries() {
    let _g = serial_guard();
    const EXPECT: &str = "Error: x != 1\nOperation failed\nResult: 1\n";
    for x in [0, 2, -1, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        assert_same_and_expected("ERRORS row 6", x, 2, 3, EXPECT);
    }
}

// ERRORS.md row 7: y one step past / far past its single valid value.
#[test]
fn err_row7_y_boundaries() {
    let _g = serial_guard();
    const EXPECT: &str = "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n";
    for y in [1, 3, 0, -1, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        assert_same_and_expected("ERRORS row 7", 1, y, 3, EXPECT);
    }
}

// ERRORS.md row 8: z one step past / far past its single valid value.
#[test]
fn err_row8_z_boundaries() {
    let _g = serial_guard();
    const EXPECT: &str = "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n";
    for z in [2, 4, 0, -1, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        assert_same_and_expected("ERRORS row 8", 1, 2, z, EXPECT);
    }
}

// ERRORS.md row 9: the "out-of-range enum" analogue. The public API takes plain
// `int`s with exactly one accepted value each, so any other bit pattern is an
// out-of-range input crossing the FFI boundary; C accepts any `int` and the
// Rust `extern "C" fn` must handle the identical set.
#[test]
fn err_row9_out_of_range_ints() {
    let _g = serial_guard();
    let garbage: [i32; 12] = [
        0x8000_0000u32 as i32,
        0x7FFF_FFFF,
        0xFFFF_FFFFu32 as i32,
        0xDEAD_BEEFu32 as i32,
        0xCAFE_BABEu32 as i32,
        0x8000_0001u32 as i32,
        -0x8000_0000i64 as i32,
        0x0000_0100,
        0x00FF_FF00,
        -3,
        1 << 30,
        -(1 << 30),
    ];
    // Every garbage value in every argument position, crossed with the valid
    // sentinels in the other two positions.
    let mut inputs = Vec::new();
    for &g in &garbage {
        inputs.push((g, 2, 3));
        inputs.push((1, g, 3));
        inputs.push((1, 2, g));
        inputs.push((g, g, g));
        for &h in &garbage {
            inputs.push((1, h, g));
            inputs.push((h, 2, g));
        }
    }
    assert_same_batch("ERRORS row 9", &inputs);
}

// ERRORS.md row 10: the `static int y = 123` initializer is never observable,
// because `driver` assigns `y = local_y` before `multi_stage` reads it. The
// first-ever call to a freshly loaded library with (1,2,3) must succeed.
#[test]
fn err_row10_initial_y_never_observable() {
    let _g = serial_guard();
    for _ in 0..8 {
        let (c, r) = load_pair();
        let out = assert_same_call("ERRORS row 10", &c, &r, 1, 2, 3);
        assert_eq!(
            out,
            b"Ok!\nResult: 0\n",
            "initial static y leaked into the first call"
        );
    }
    // And the initializer value 123 is not special as an input either.
    assert_same_and_expected(
        "ERRORS row 10",
        1,
        123,
        3,
        "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n",
    );
}

// ERRORS.md row 11: every rejection path returns normally (no abort, no panic
// across the FFI boundary), including under repetition.
#[test]
fn err_row11_rejection_paths_return_normally() {
    let _g = serial_guard();
    let (c, r) = load_pair();
    let mut rng = Rng::new();
    let mut n = 0usize;
    let out = capture(|| {
        for _ in 0..500 {
            let (x, y, z) = (rng.small(), rng.small(), rng.small());
            c.call(x, y, z);
            r.call(x, y, z);
            n += 1;
        }
    });
    // Reaching here at all proves both libraries returned normally 1000 times.
    assert_eq!(n, 500);
    assert!(!out.is_empty());
    // Even output count: each call produced the same block from both libs.
    let lines = String::from_utf8(out).unwrap().lines().count();
    assert_eq!(lines % 2, 0, "C and Rust produced different line counts");
}

// ===========================================================================
// Phase D — symbol parity asserted from inside the test suite.
// ===========================================================================

// Both `.so` files must export `driver`, and the C-internal `static` symbols
// must be absent from BOTH (exporting them would be a parity failure too).
#[test]
fn symbols_parity_via_dlsym() {
    let _g = serial_guard();
    let (c, r) = load_pair();
    // `driver` resolved during load_pair(); assert the internals stay internal.
    for hidden in [b"multi_stage\0".as_ref(), b"y\0".as_ref()] {
        let name = String::from_utf8_lossy(&hidden[..hidden.len() - 1]).to_string();
        let in_c = unsafe {
            c._lib
                .get::<*const c_void>(hidden)
                .is_ok()
        };
        let in_r = unsafe {
            r._lib
                .get::<*const c_void>(hidden)
                .is_ok()
        };
        assert_eq!(
            in_c, in_r,
            "`{name}` visibility differs: C exports={in_c}, Rust exports={in_r}"
        );
        assert!(!in_c, "`{name}` is `static` in C and must not be exported");
    }
}
