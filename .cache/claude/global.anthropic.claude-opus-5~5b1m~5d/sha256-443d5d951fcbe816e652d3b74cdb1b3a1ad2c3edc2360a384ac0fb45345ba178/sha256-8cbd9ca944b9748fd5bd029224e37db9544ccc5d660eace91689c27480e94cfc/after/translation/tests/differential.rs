//! Differential tests: load BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compare their stdout byte-for-byte through the FFI
//! boundary. The Rust implementation is NEVER called directly — always via
//! `dlsym` on the built cdylib, exactly as an external C caller would.
//!
//! Run with `--test-threads=1` is NOT required: all stdout redirection is
//! serialized by `CAPTURE_LOCK`.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

type DriverFn = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let base = manifest_dir().join("target");
    let (first, second) = if cfg!(debug_assertions) {
        ("debug", "release")
    } else {
        ("release", "debug")
    };
    let a = base.join(first).join("libdriver.so");
    if a.exists() {
        return a;
    }
    base.join(second).join("libdriver.so")
}

struct Libs {
    c: Library,
    rust: Library,
}

// Safety: the libraries are loaded once and never unloaded; `driver` is
// re-entrant-safe apart from stdout, which the capture lock serializes.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        assert!(cp.exists(), "C library not built: {}", cp.display());
        assert!(rp.exists(), "Rust cdylib not built: {}", rp.display());
        unsafe {
            Libs {
                c: Library::new(&cp).expect("dlopen C libdriver.so"),
                rust: Library::new(&rp).expect("dlopen Rust libdriver.so"),
            }
        }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C driver symbol") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("Rust driver symbol") }
}

// ---------------------------------------------------------------------------
// stdout capture (fd-level, so it catches libc `printf` from inside the .so)
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

fn tmp_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        tag,
        n
    ))
}

/// Redirect fd 1 to a temp file, run `f`, flush all C streams, restore fd 1
/// and return the captured bytes.
fn capture<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = tmp_path(tag);
    let bytes = {
        let file = fs::File::create(&path).expect("create temp capture file");
        let _ = std::io::stdout().flush();
        unsafe {
            fflush(std::ptr::null_mut());
        }
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

        f();

        unsafe {
            fflush(std::ptr::null_mut());
        }
        assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
        unsafe {
            close(saved);
        }
        drop(file);
        fs::read(&path).expect("read capture file")
    };
    let _ = fs::remove_file(&path);
    drop(guard);
    bytes
}

fn call_c(x: c_int) -> Vec<u8> {
    let f = c_driver();
    capture("c", || unsafe { f(x) })
}

fn call_rust(x: c_int) -> Vec<u8> {
    let f = rust_driver();
    capture("rust", || unsafe { f(x) })
}

#[track_caller]
fn assert_same(x: c_int, row: &str) -> Vec<u8> {
    let c = call_c(x);
    let r = call_rust(x);
    assert_eq!(
        c,
        r,
        "[{row}] divergence for x = {x} (0x{:08x})\n  C   : {:?}\n  Rust: {:?}",
        x as u32,
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r),
    );
    // Shape invariant (CONFIGS row C12 / ERRORS row E9): sizeof(house_t) == 16
    assert_eq!(c.len(), 33, "[{row}] expected 32 hex chars + newline, got {c:?}");
    assert_eq!(*c.last().unwrap(), b'\n', "[{row}] missing trailing newline");
    assert!(
        c[..32].iter().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
        "[{row}] non lowercase-hex output: {:?}",
        String::from_utf8_lossy(&c)
    );
    c
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64), fixed seed for reproducibility
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Rng(0x5DEECE66D)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `lo..=hi`.
    fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
}

/// Expected 32-hex-char line, computed independently from the C source:
/// `{ int floors = x; int bedrooms = 3; double bathrooms = 2.0; }`
/// little-endian, 16 bytes, no padding.
fn expected_line(x: c_int) -> String {
    let mut bytes = Vec::with_capacity(16);
    bytes.extend_from_slice(&(x as i32).to_le_bytes());
    bytes.extend_from_slice(&3i32.to_le_bytes());
    bytes.extend_from_slice(&2.0f64.to_le_bytes());
    let mut s: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    s.push('\n');
    s
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

#[test]
fn cfg_c1_zero() {
    let out = assert_same(0, "C1");
    assert_eq!(
        String::from_utf8_lossy(&out),
        "00000000030000000000000000000040\n"
    );
}

#[test]
fn cfg_c2_small_positive() {
    for x in 1..=9 {
        assert_same(x, "C2");
    }
}

#[test]
fn cfg_c3_one_byte_range() {
    for x in [0x10, 0x1f, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
        assert_same(x, "C3");
    }
    let mut rng = Rng::new();
    for _ in 0..64 {
        assert_same(rng.range_u32(0x10, 0xff) as c_int, "C3");
    }
}

#[test]
fn cfg_c4_two_byte_range() {
    for x in [0x100, 0x0fff, 0x7fff, 0x8000, 0xffff] {
        assert_same(x, "C4");
    }
    let mut rng = Rng::new();
    for _ in 0..64 {
        assert_same(rng.range_u32(0x100, 0xffff) as c_int, "C4");
    }
}

#[test]
fn cfg_c5_three_byte_range() {
    for x in [0x1_0000, 0x7f_ffff, 0x80_0000, 0xff_ffff] {
        assert_same(x, "C5");
    }
    let mut rng = Rng::new();
    for _ in 0..64 {
        assert_same(rng.range_u32(0x1_0000, 0xff_ffff) as c_int, "C5");
    }
}

#[test]
fn cfg_c6_four_byte_range() {
    for x in [0x0100_0000, 0x1234_5678, 0x7fff_fffe, i32::MAX as u32] {
        assert_same(x as c_int, "C6");
    }
    let mut rng = Rng::new();
    for _ in 0..64 {
        assert_same(rng.range_u32(0x0100_0000, 0x7fff_ffff) as c_int, "C6");
    }
}

#[test]
fn cfg_c7_negative() {
    for x in [-1, -2, -3, -0x100, -0x1_0000, -0x100_0000, -1_000_000, i32::MIN] {
        assert_same(x, "C7");
    }
    let mut rng = Rng::new();
    for _ in 0..128 {
        let v = -(rng.range_u32(1, 0x8000_0000u32 - 1) as i64) as i32;
        assert_same(v, "C7");
    }
}

#[test]
fn cfg_c8_high_bytes() {
    for x in [
        0x8080_8080u32,
        0xffff_ffffu32,
        0x8000_0000u32,
        0xf0f0_f0f0u32,
        0xdead_beefu32,
        0xcafe_babeu32,
    ] {
        assert_same(x as c_int, "C8");
    }
    // every byte independently drawn from 0x80..=0xff
    let mut rng = Rng::new();
    for _ in 0..64 {
        let mut v: u32 = 0;
        for i in 0..4 {
            v |= (rng.range_u32(0x80, 0xff) & 0xff) << (8 * i);
        }
        assert_same(v as c_int, "C8");
    }
}

#[test]
fn cfg_c9_random_full_range() {
    let mut rng = Rng::new();
    for _ in 0..4096 {
        let v = rng.next_u32();
        let out = assert_same(v as c_int, "C9");
        assert_eq!(
            String::from_utf8_lossy(&out),
            expected_line(v as c_int),
            "C9 output disagrees with independently computed expectation for 0x{v:08x}"
        );
    }
}

#[test]
fn cfg_c10_exhaustive_low_byte() {
    let mut rng = Rng::new();
    for low in 0u32..=0xff {
        let high = rng.next_u32() & 0xffff_ff00;
        assert_same((high | low) as c_int, "C10");
    }
}

#[test]
fn cfg_c11_repeat_and_interleave() {
    // Same value repeatedly: must be idempotent in both libraries.
    let c1 = call_c(42);
    let c2 = call_c(42);
    let r1 = call_rust(42);
    let r2 = call_rust(42);
    assert_eq!(c1, c2, "C11 C not idempotent");
    assert_eq!(r1, r2, "C11 Rust not idempotent");
    assert_eq!(c1, r1, "C11 C vs Rust");

    // Interleaved sequences of many calls, compared as a whole transcript.
    let mut rng = Rng::new();
    let values: Vec<c_int> = (0..64).map(|_| rng.next_u32() as c_int).collect();

    let cf = c_driver();
    let vs = values.clone();
    let c_all = capture("c-seq", || {
        for v in &vs {
            unsafe { cf(*v) }
        }
    });
    let rf = rust_driver();
    let vs = values.clone();
    let r_all = capture("rust-seq", || {
        for v in &vs {
            unsafe { rf(*v) }
        }
    });
    assert_eq!(c_all, r_all, "C11 interleaved transcript mismatch");
    assert_eq!(c_all.len(), 64 * 33);

    // Alternate C/Rust calls in one capture: output must be pairwise equal.
    for v in values.iter().take(16) {
        let mixed = capture("mixed", || {
            unsafe { cf(*v) };
            unsafe { rf(*v) };
        });
        let (a, b) = mixed.split_at(33);
        assert_eq!(a, b, "C11 mixed-call divergence for {v}");
    }
}

#[test]
fn cfg_c12_output_shape_and_tail() {
    let mut rng = Rng::new();
    let mut values: Vec<c_int> = vec![0, 1, -1, i32::MAX, i32::MIN, 0x0f0f_0f0fu32 as c_int];
    values.extend((0..64).map(|_| rng.next_u32() as c_int));
    for v in values {
        let out = assert_same(v, "C12");
        let s = String::from_utf8(out).unwrap();
        // bedrooms == 3, bathrooms == 2.0 (IEEE-754 LE) => fixed 24-char tail
        assert_eq!(
            &s[8..32],
            "030000000000000000000040",
            "C12 constant tail changed for {v}"
        );
        // floors is the little-endian image of x
        let expected_head: String = (v as i32)
            .to_le_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(&s[0..8], expected_head, "C12 floors bytes wrong for {v}");
    }
}

#[test]
fn cfg_c13_fresh_process_each_lib() {
    // Child mode: a fresh process that loads exactly ONE library.
    if let Ok(which) = std::env::var("DRIVER_CHILD") {
        let x: i32 = std::env::var("DRIVER_X").unwrap().parse().unwrap();
        let out = std::env::var("DRIVER_OUT").unwrap();
        let path = if which == "c" { c_lib_path() } else { rust_lib_path() };
        let file = fs::File::create(&out).unwrap();
        let _ = std::io::stdout().flush();
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(file.as_raw_fd(), 1);
        }
        unsafe {
            let lib = Library::new(&path).unwrap();
            let f: Symbol<DriverFn> = lib.get(b"driver\0").unwrap();
            f(x);
            fflush(std::ptr::null_mut());
        }
        std::process::exit(0);
    }

    let exe = std::env::current_exe().unwrap();
    let mut rng = Rng::new();
    let mut values: Vec<i32> = vec![0, -1, i32::MAX, i32::MIN, 3];
    values.extend((0..8).map(|_| rng.next_u32() as i32));

    for v in values {
        let mut outs = Vec::new();
        for which in ["c", "rust"] {
            let out = tmp_path(&format!("child-{which}"));
            let status = std::process::Command::new(&exe)
                .args(["--exact", "cfg_c13_fresh_process_each_lib", "--nocapture"])
                .env("DRIVER_CHILD", which)
                .env("DRIVER_X", v.to_string())
                .env("DRIVER_OUT", &out)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .expect("spawn child");
            assert!(status.success(), "child ({which}) failed for {v}: {status:?}");
            let bytes = fs::read(&out).expect("child output");
            let _ = fs::remove_file(&out);
            outs.push(bytes);
        }
        assert_eq!(
            String::from_utf8_lossy(&outs[0]),
            String::from_utf8_lossy(&outs[1]),
            "C13 fresh-process divergence for {v}"
        );
        assert_eq!(
            String::from_utf8_lossy(&outs[0]),
            expected_line(v),
            "C13 unexpected output for {v}"
        );
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

#[test]
fn err_e1_zero() {
    let out = assert_same(0, "E1");
    assert_eq!(
        String::from_utf8_lossy(&out),
        "00000000030000000000000000000040\n"
    );
}

#[test]
fn err_e2_minus_one() {
    let out = assert_same(-1, "E2");
    let s = String::from_utf8(out).unwrap();
    assert!(s.starts_with("ffffffff"), "E2: {s}");
    assert_eq!(s, expected_line(-1));
}

#[test]
fn err_e3_int_max() {
    let out = assert_same(i32::MAX, "E3");
    let s = String::from_utf8(out).unwrap();
    assert!(s.starts_with("ffffff7f"), "E3: {s}");
    // one step below the top of the range too
    assert_same(i32::MAX - 1, "E3");
}

#[test]
fn err_e4_int_min() {
    let out = assert_same(i32::MIN, "E4");
    let s = String::from_utf8(out).unwrap();
    assert!(s.starts_with("00000080"), "E4: {s}");
    assert_same(i32::MIN + 1, "E4");
}

#[test]
fn err_e5_one_past_int_max() {
    // 0x80000000: one past INT_MAX, arrives as INT_MIN across the FFI boundary.
    let x = 0x8000_0000u32 as c_int;
    let a = assert_same(x, "E5");
    let b = call_c(i32::MIN);
    assert_eq!(a, b, "E5: 0x80000000 must behave exactly as INT_MIN");
}

#[test]
fn err_e6_uint_max_pattern() {
    let x = 0xFFFF_FFFFu32 as c_int;
    let a = assert_same(x, "E6");
    let b = call_c(-1);
    assert_eq!(a, b, "E6: 0xffffffff must behave exactly as -1");
}

#[test]
fn err_e7_arbitrary_no_special_case() {
    // No enum/switch exists in the C: every int takes the identical path.
    // These are "out-of-range enum" analogues: values with no special meaning.
    let mut rng = Rng::new();
    let mut values: Vec<c_int> = vec![
        3,
        0x2a,
        0x7fff_fffe,
        -1_000_000,
        1,
        -2,
        0x7fff_ffff,
        0x0000_0100,
        i32::MIN,
        0x5555_5555u32 as c_int,
        0xaaaa_aaaau32 as c_int,
    ];
    values.extend((0..256).map(|_| rng.next_u32() as c_int));
    for v in values {
        let out = assert_same(v, "E7");
        assert_eq!(
            String::from_utf8_lossy(&out),
            expected_line(v),
            "E7 value-dependent path differs for {v}"
        );
    }
}

#[test]
fn err_e8_repeated_calls_no_state() {
    let cf = c_driver();
    let rf = rust_driver();
    let mut rng = Rng::new();
    let values: Vec<c_int> = (0..200).map(|_| rng.next_u32() as c_int).collect();

    let vs = values.clone();
    let c_all = capture("e8-c", || {
        for v in &vs {
            unsafe { cf(*v) }
        }
    });
    let vs = values.clone();
    let r_all = capture("e8-rust", || {
        for v in &vs {
            unsafe { rf(*v) }
        }
    });
    assert_eq!(c_all, r_all, "E8 transcript mismatch");

    // Each line must depend only on its own argument (no leaked state).
    let expected: String = values.iter().map(|v| expected_line(*v)).collect();
    assert_eq!(String::from_utf8_lossy(&c_all), expected, "E8 C transcript");
    assert_eq!(String::from_utf8_lossy(&r_all), expected, "E8 Rust transcript");
}

#[test]
fn err_e9_len_always_16() {
    // print_hex has no bound/null check, but driver always passes
    // (&raw, sizeof(house_t)). Verify the emitted length is invariably 16 bytes
    // -> 33 output bytes, for a wide sweep of inputs, in BOTH libraries.
    let mut rng = Rng::new();
    for _ in 0..256 {
        let v = rng.next_u32() as c_int;
        let c = call_c(v);
        let r = call_rust(v);
        assert_eq!(c.len(), 33, "E9 C length for {v}: {c:?}");
        assert_eq!(r.len(), 33, "E9 Rust length for {v}: {r:?}");
        assert_eq!(c, r, "E9 mismatch for {v}");
    }
}
