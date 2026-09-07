//! Differential tests: load BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compare their observable behavior byte-for-byte.
//!
//! The Rust implementation is NEVER called directly — it is always reached
//! through the exported `driver` symbol of `target/release/libdriver.so`, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! Both libraries write to `stdout` through C `stdio`, so the harness redirects
//! file descriptor 1 to a temporary file around each call batch and reads the
//! captured bytes back.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(f32);

struct Libs {
    c: Library,
    rust: Library,
}

// The libraries stay loaded for the whole test-binary lifetime.
fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let c_path = format!("{manifest}/../c_src/build/libdriver.so");
        let rust_path = format!("{manifest}/target/release/libdriver.so");
        for p in [&c_path, &rust_path] {
            assert!(
                std::path::Path::new(p).exists(),
                "shared library not found: {p}\n\
                 build the C lib with:  cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
                 build the Rust lib with:  cd translation && cargo build --release"
            );
        }
        unsafe {
            Libs {
                c: Library::new(&c_path).expect("failed to dlopen the C library"),
                rust: Library::new(&rust_path).expect("failed to dlopen the Rust library"),
            }
        }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C .so does not export `driver`") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe {
        libs()
            .rust
            .get(b"driver\0")
            .expect("Rust .so does not export `driver`")
    }
}

// ---------------------------------------------------------------------------
// stdout capture (fd-level, so it catches C `printf` from both .so files)
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Give C `stdio`'s `stdout` a statically owned, fully-buffered buffer once, so
/// that `printf` inside the forked child never needs to `malloc`.
fn init_stdio() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        const CAP: usize = 1 << 16;
        let buf: &'static mut [u8] = Box::leak(vec![0u8; CAP].into_boxed_slice());
        unsafe {
            libc::setvbuf(
                libc_stdout(),
                buf.as_mut_ptr() as *mut libc::c_char,
                libc::_IOFBF,
                CAP,
            );
        }
    });
}

/// glibc's `stdout` global.
fn libc_stdout() -> *mut libc::FILE {
    extern "C" {
        static mut stdout: *mut libc::FILE;
    }
    unsafe { std::ptr::read(std::ptr::addr_of!(stdout)) }
}

/// Runs `f` in a forked child whose fd 1 points at a temporary file, and returns
/// everything the child wrote.
///
/// Forking (rather than `dup2`-ing this process' fd 1) is what makes the capture
/// immune to the libtest harness' own concurrent progress output — the child has
/// a private fd 1, so nothing else in the process can land in the capture.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    init_stdio();

    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!(
        "{dir}/driver_capture_{}_{}.bin",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    let c_path = CString::new(path.clone()).unwrap();

    unsafe {
        // Nothing of ours may be pending in the C stdio buffers across the fork.
        libc::fflush(std::ptr::null_mut());

        let pid = libc::fork();
        assert!(pid >= 0, "fork() failed");

        if pid == 0 {
            // ---- child ----
            let fd = libc::open(
                c_path.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                0o600 as libc::c_int,
            );
            if fd < 0 {
                libc::_exit(91);
            }
            if libc::dup2(fd, 1) < 0 {
                libc::_exit(92);
            }
            f();
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }

        // ---- parent ----
        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        assert!(
            libc::WIFEXITED(status),
            "capture child did not exit normally (status {status:#x})"
        );
        assert_eq!(
            libc::WEXITSTATUS(status),
            0,
            "capture child exited with code {}",
            libc::WEXITSTATUS(status)
        );
    }

    let bytes = std::fs::read(&path).expect("failed to read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0, 1).
    fn next_unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
}

// ---------------------------------------------------------------------------
// Core differential helper
// ---------------------------------------------------------------------------

/// Calls `driver` in the C `.so` for every input, then in the Rust `.so` for
/// every input, and asserts the two captured stdout byte streams are identical.
#[track_caller]
fn assert_same(label: &str, inputs: &[f32]) {
    let c = c_driver();
    let r = rust_driver();

    let c_out = capture_stdout(|| {
        for &x in inputs {
            unsafe { c(x) }
        }
    });
    let r_out = capture_stdout(|| {
        for &x in inputs {
            unsafe { r(x) }
        }
    });

    if c_out != r_out {
        // Locate the first differing line to make the failure readable.
        let cl: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();
        for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
            if a != b {
                panic!(
                    "[{label}] divergence at output line {i} (input {:?} = bits {:#010x}):\n  \
                     C   : {}\n  Rust: {}",
                    inputs.get(i),
                    inputs.get(i).map(|v| v.to_bits()).unwrap_or(0),
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b),
                );
            }
        }
        panic!(
            "[{label}] output length mismatch: C {} bytes, Rust {} bytes",
            c_out.len(),
            r_out.len()
        );
    }

    // Sanity: exactly 9 bytes per call ("%02x" * 4 + "\n").
    assert_eq!(
        c_out.len(),
        inputs.len() * 9,
        "[{label}] unexpected C output size"
    );
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

// Row 1
#[test]
fn cfg01_positive_normals() {
    let mut rng = Rng::new(SEED ^ 1);
    let mut v = Vec::with_capacity(10_000);
    while v.len() < 10_000 {
        // exponent 1..=254 (normal), random mantissa, sign 0
        let exp = 1 + (rng.next_u32() % 254);
        let mant = rng.next_u32() & 0x007f_ffff;
        v.push(f32::from_bits((exp << 23) | mant));
    }
    assert_same("positive normals", &v);
}

// Row 2
#[test]
fn cfg02_negative_normals() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut v = Vec::with_capacity(10_000);
    while v.len() < 10_000 {
        let exp = 1 + (rng.next_u32() % 254);
        let mant = rng.next_u32() & 0x007f_ffff;
        v.push(f32::from_bits(0x8000_0000 | (exp << 23) | mant));
    }
    assert_same("negative normals", &v);
}

// Row 3
#[test]
fn cfg03_unit_interval() {
    let mut rng = Rng::new(SEED ^ 3);
    let v: Vec<f32> = (0..10_000)
        .map(|_| {
            let u = rng.next_unit();
            if rng.next_u32() & 1 == 0 {
                u
            } else {
                -u
            }
        })
        .collect();
    assert_same("[-1,1] randoms", &v);
}

// Row 4
#[test]
fn cfg04_large_magnitudes() {
    let mut rng = Rng::new(SEED ^ 4);
    let v: Vec<f32> = (0..10_000)
        .map(|_| {
            let u = rng.next_unit();
            let s = if rng.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
            s * u * f32::MAX
        })
        .collect();
    assert_same("large magnitudes", &v);
}

// Row 5
#[test]
fn cfg05_positive_subnormals() {
    let mut rng = Rng::new(SEED ^ 5);
    let v: Vec<f32> = (0..5_000)
        .map(|_| f32::from_bits(1 + (rng.next_u32() % 0x007f_ffff)))
        .collect();
    assert_same("positive subnormals", &v);
}

// Row 6
#[test]
fn cfg06_negative_subnormals() {
    let mut rng = Rng::new(SEED ^ 6);
    let v: Vec<f32> = (0..5_000)
        .map(|_| f32::from_bits(0x8000_0000 | (1 + (rng.next_u32() % 0x007f_ffff))))
        .collect();
    assert_same("negative subnormals", &v);
}

// Row 7 / E1
#[test]
fn cfg07_signed_zeros() {
    assert_same("signed zeros", &[0.0f32, -0.0f32, 0.0f32, -0.0f32]);
}

// Row 8
#[test]
fn cfg08_integral_values() {
    let mut rng = Rng::new(SEED ^ 8);
    let v: Vec<f32> = (0..10_000).map(|_| rng.next_u32() as i32 as f32).collect();
    assert_same("integral values", &v);
}

// Row 9 / E4
#[test]
fn cfg09_infinities() {
    assert_same(
        "infinities",
        &[f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY],
    );
}

// Row 10
#[test]
fn cfg10_quiet_nans() {
    let mut rng = Rng::new(SEED ^ 10);
    let v: Vec<f32> = (0..5_000)
        .map(|_| {
            let sign = (rng.next_u32() & 1) << 31;
            let payload = rng.next_u32() & 0x003f_ffff; // quiet bit stays set
            f32::from_bits(sign | 0x7fc0_0000 | payload)
        })
        .collect();
    assert_same("quiet NaNs", &v);
}

// Row 11
#[test]
fn cfg11_signalling_nans() {
    let mut rng = Rng::new(SEED ^ 11);
    let v: Vec<f32> = (0..5_000)
        .map(|_| {
            let sign = (rng.next_u32() & 1) << 31;
            // exponent all ones, quiet bit CLEAR, non-zero payload => sNaN
            let payload = 1 + (rng.next_u32() % 0x003f_ffff);
            f32::from_bits(sign | 0x7f80_0000 | payload)
        })
        .collect();
    assert_same("signalling NaNs", &v);
}

// Row 12
#[test]
fn cfg12_ieee_boundary_constants() {
    let v = vec![
        f32::MIN_POSITIVE,                 // smallest normal, 0x00800000
        f32::from_bits(0x007f_ffff),       // largest subnormal
        f32::from_bits(0x0000_0001),       // FLT_TRUE_MIN
        f32::MAX,                          // 0x7f7fffff
        f32::MIN,                          // -FLT_MAX
        f32::EPSILON,                      // 0x34000000
        1.0,
        -1.0,
        2.0,
        0.5,
        -0.5,
        f32::from_bits(0x7f80_0000),       // one step past FLT_MAX => +inf
        f32::from_bits(0xff80_0000),       // -inf
    ];
    assert_same("IEEE boundary constants", &v);
}

// Row 13 / E6
#[test]
fn cfg13_all_byte_values_all_positions() {
    let mut v = Vec::with_capacity(1024);
    for pos in 0..4u32 {
        for b in 0..256u32 {
            v.push(f32::from_bits(b << (8 * pos)));
        }
    }
    assert_same("all byte values in all 4 positions", &v);
}

// Row 14 / E7
#[test]
fn cfg14_random_full_u32_space() {
    let mut rng = Rng::new(SEED ^ 14);
    let v: Vec<f32> = (0..200_000)
        .map(|_| f32::from_bits(rng.next_u32()))
        .collect();
    assert_same("200k random u32 bit patterns", &v);
}

// Row 15
#[test]
fn cfg15_single_call_exact_bytes() {
    let c = c_driver();
    let r = rust_driver();
    for bits in [0x0000_0000u32, 0x3f80_0000, 0xdead_beef, 0x7fc0_0000] {
        let x = f32::from_bits(bits);
        let co = capture_stdout(|| unsafe { c(x) });
        let ro = capture_stdout(|| unsafe { r(x) });
        assert_eq!(co, ro, "single-call divergence for bits {bits:#010x}");
        assert_eq!(co.len(), 9, "expected 9 bytes for bits {bits:#010x}");
        // Independently confirm the expected native-endian hex encoding.
        let expected: String = x
            .to_ne_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            + "\n";
        assert_eq!(
            String::from_utf8_lossy(&co),
            expected,
            "C output does not match the native-endian hex encoding"
        );
    }
}

// Row 16
#[test]
fn cfg16_many_calls_one_capture() {
    let mut rng = Rng::new(SEED ^ 16);
    let v: Vec<f32> = (0..1_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    assert_same("1000 calls in one capture", &v);
}

// Row 17 / E8
#[test]
fn cfg17_interleaved_calls() {
    let c = c_driver();
    let r = rust_driver();
    let mut rng = Rng::new(SEED ^ 17);
    let v: Vec<f32> = (0..1_000).map(|_| f32::from_bits(rng.next_u32())).collect();

    let out = capture_stdout(|| {
        for &x in &v {
            unsafe {
                c(x);
                r(x);
            }
        }
    });
    let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').collect();
    // trailing empty element after the final '\n'
    assert_eq!(lines.len(), v.len() * 2 + 1, "unexpected line count");
    for i in 0..v.len() {
        assert_eq!(
            lines[2 * i],
            lines[2 * i + 1],
            "interleaved divergence at input {} (bits {:#010x})",
            i,
            v[i].to_bits()
        );
    }
}

// Row 18
#[test]
fn cfg18_output_shape_invariant() {
    let mut rng = Rng::new(SEED ^ 18);
    let v: Vec<f32> = (0..2_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    let c = c_driver();
    let out = capture_stdout(|| {
        for &x in &v {
            unsafe { c(x) }
        }
    });
    assert_eq!(out.len(), v.len() * 9);
    for (i, line) in out.split(|&b| b == b'\n').take(v.len()).enumerate() {
        assert_eq!(line.len(), 8, "line {i} is not 8 hex digits");
        assert!(
            line.iter().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
            "line {i} is not lowercase hex: {}",
            String::from_utf8_lossy(line)
        );
    }
    // And the Rust library agrees.
    assert_same("output shape invariant", &v);
}

// ===========================================================================
// Phase C — ERRORS.md rows (the C library has no rejection path, so every row
// asserts that C and Rust behave IDENTICALLY on the boundary input)
// ===========================================================================

// E1
#[test]
fn err_signed_zeros() {
    assert_same(
        "E1 signed zeros",
        &[f32::from_bits(0x0000_0000), f32::from_bits(0x8000_0000)],
    );
}

// E2
#[test]
fn err_subnormal_boundaries() {
    assert_same(
        "E2 subnormal boundaries",
        &[
            f32::from_bits(0x0000_0001),
            f32::from_bits(0x0000_0002),
            f32::from_bits(0x007f_fffe),
            f32::from_bits(0x007f_ffff),
            f32::from_bits(0x8000_0001),
            f32::from_bits(0x807f_ffff),
        ],
    );
}

// E3
#[test]
fn err_normal_boundaries() {
    assert_same(
        "E3 normal boundaries",
        &[
            f32::from_bits(0x0080_0000), // FLT_MIN
            f32::from_bits(0x0080_0001),
            f32::from_bits(0x7f7f_fffe),
            f32::from_bits(0x7f7f_ffff), // FLT_MAX
            f32::from_bits(0x7f80_0000), // one step past FLT_MAX
            f32::from_bits(0xff7f_ffff), // -FLT_MAX
            f32::from_bits(0xff80_0000), // one step past -FLT_MAX
        ],
    );
}

// E4
#[test]
fn err_infinities() {
    assert_same(
        "E4 infinities",
        &[f32::from_bits(0x7f80_0000), f32::from_bits(0xff80_0000)],
    );
}

// E5 — bit patterns with no "valid value": the float analogue of an
// out-of-range enum crossing the FFI boundary.
#[test]
fn err_nan_payloads() {
    assert_same(
        "E5 NaN payloads",
        &[
            f32::from_bits(0x7fc0_0000), // canonical qNaN
            f32::from_bits(0x7fa0_0000), // sNaN
            f32::from_bits(0x7f80_0001), // smallest-payload sNaN
            f32::from_bits(0x7fbf_ffff), // largest-payload sNaN
            f32::from_bits(0x7fff_ffff), // largest-payload qNaN
            f32::from_bits(0xffc0_0000), // negative qNaN
            f32::from_bits(0xffa0_0000), // negative sNaN
            f32::from_bits(0xff80_0001),
            f32::from_bits(0xffff_ffff),
        ],
    );
}

// E6
#[test]
fn err_all_byte_values_all_positions() {
    // Same coverage as row 13 but also with the other bytes saturated to 0xff,
    // so each of the 4 loop iterations sees both < 0x10 and >= 0x80 values.
    let mut v = Vec::with_capacity(2048);
    for pos in 0..4u32 {
        for b in 0..256u32 {
            let mask = 0xffff_ffffu32 & !(0xffu32 << (8 * pos));
            v.push(f32::from_bits(mask | (b << (8 * pos))));
            v.push(f32::from_bits(b << (8 * pos)));
        }
    }
    assert_same("E6 every byte value in every position", &v);
}

// E7 — full argument domain sweep (there is no invalid value to reject).
#[test]
fn err_full_domain_sweep() {
    let mut rng = Rng::new(SEED ^ 0xE7);
    let v: Vec<f32> = (0..100_000)
        .map(|_| f32::from_bits(rng.next_u32()))
        .collect();
    assert_same("E7 full domain sweep", &v);
}

// E8
#[test]
fn err_repeated_interleaved_calls() {
    let c = c_driver();
    let r = rust_driver();
    // The same value many times in a row: no hidden state may accumulate.
    let out = capture_stdout(|| {
        for _ in 0..500 {
            unsafe {
                c(1.5f32);
                r(1.5f32);
            }
        }
    });
    let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1000);
    assert!(
        lines.iter().all(|l| *l == lines[0]),
        "repeated calls produced varying output"
    );
}

// ===========================================================================
// Phase D — symbol parity, checked from inside the test suite
// ===========================================================================

#[test]
fn symbol_parity_driver_is_exported_by_both() {
    // Reaching these without panicking proves both `.so` files export `driver`.
    let _c = c_driver();
    let _r = rust_driver();

    // `print_hex` was `static` in C and must not be exported by either library.
    unsafe {
        assert!(
            libs().c.get::<DriverFn>(b"print_hex\0").is_err(),
            "C .so unexpectedly exports print_hex"
        );
        assert!(
            libs().rust.get::<DriverFn>(b"print_hex\0").is_err(),
            "Rust .so unexpectedly exports print_hex"
        );
    }
}
