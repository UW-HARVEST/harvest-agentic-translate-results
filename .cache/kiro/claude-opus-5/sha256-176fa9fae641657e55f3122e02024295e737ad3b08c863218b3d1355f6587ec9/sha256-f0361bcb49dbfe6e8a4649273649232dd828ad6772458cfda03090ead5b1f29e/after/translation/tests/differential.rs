//! Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//!
//! BOTH libraries are loaded with `libloading` and driven only through their
//! exported symbols, exactly as an external consumer would — the Rust crate is
//! never linked directly, so the `#[no_mangle]` export wrapper is under test
//! too.
//!
//! `driver` returns `void` and communicates solely through `stdout` (C
//! `printf`), so every comparison captures file descriptor 1 around the call
//! and compares the raw bytes.

use std::ffi::{c_int, c_uint};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    repo_root().join("c_src/build/libdriver.so")
}

/// Path to the Rust `cdylib` under test.
///
/// `cargo test` does **not** build a `cdylib`-only crate (an integration test
/// cannot link one), so relying on whatever `.so` happens to sit in
/// `target/release/` would mean verifying a stale artifact — a silent false
/// pass. This builds the library on demand into a dedicated target directory
/// (separate from the one the running `cargo test` holds a lock on) so the
/// loaded `.so` always corresponds to the current `src/lib.rs`.
fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target").join("ffi");
    let so = target_dir.join("release").join("libdriver.so");

    let status = std::process::Command::new(env!("CARGO"))
        .current_dir(&manifest)
        .args(["build", "--release", "--target-dir"])
        .arg(&target_dir)
        .status();

    match status {
        Ok(s) if s.success() => {}
        other => {
            diag(&format!(
                "failed to build the Rust cdylib under test ({other:?}); \
                 run `cargo build --release --target-dir target/ffi` manually\n"
            ));
            panic!("cannot build Rust cdylib under test");
        }
    }

    if !so.exists() {
        diag(&format!("expected cdylib at {} but it is absent\n", so.display()));
        panic!("Rust cdylib not produced");
    }
    assert_not_stale(&so, &manifest);
    so
}

/// Writes straight to fd 2. `[profile.release] panic = "abort"` applies to the
/// test binary too, which can swallow buffered panic output, so diagnostics go
/// out unbuffered.
fn diag(msg: &str) {
    // SAFETY: writing a valid byte range to fd 2.
    unsafe {
        libc::write(2, msg.as_ptr().cast(), msg.len());
    }
}

/// Backstop for the on-demand build above.
fn assert_not_stale(so: &PathBuf, manifest: &PathBuf) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat rust .so");
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest.join(src);
        let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) else {
            continue;
        };
        if so_mtime < m {
            diag(&format!(
                "STALE ARTIFACT: {} is older than {}\n",
                so.display(),
                p.display()
            ));
            panic!("stale Rust cdylib");
        }
    }
}

struct Libs {
    c: Library,
    rust: Library,
}

// SAFETY: the two `Library` handles are only ever used to look up symbols and
// call them; the underlying dlopen handles are valid for the process lifetime.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        assert!(cp.exists(), "missing C shared library at {}", cp.display());
        assert!(rp.exists(), "missing Rust shared library at {}", rp.display());
        // SAFETY: loading a plain C shared library with no init side effects
        // beyond libc's own.
        unsafe {
            Libs {
                c: Library::new(&cp).expect("dlopen C libdriver.so"),
                rust: Library::new(&rp).expect("dlopen Rust libdriver.so"),
            }
        }
    })
}

/// Serialises the fd-1 redirection used by `capture`, because `cargo test`
/// runs test functions on multiple threads.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Runs `f`, capturing everything written to file descriptor 1 (including
/// writes from inside the dynamically loaded libraries) and returning it.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = tempfile();

    // SAFETY: raw fd juggling; every fd used below is checked.
    unsafe {
        // Flush anything already pending so it does not end up in the capture
        // file: both Rust's own line-buffered `stdout` (libtest writes its
        // progress text there without a trailing newline) and every C stdio
        // stream.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        assert!(libc::dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        f();

        // Force the libraries' stdio buffer out to fd 1 before restoring.
        // `fflush(NULL)` flushes every open output stream, including the
        // `stdout` FILE the loaded `.so`s' `printf` writes through.
        libc::fflush(std::ptr::null_mut());

        assert!(libc::dup2(saved, 1) >= 0, "restoring fd 1 failed");
        libc::close(saved);
    }

    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek capture file");
    tmp.read_to_end(&mut out).expect("read capture file");
    out
}

fn tempfile() -> std::fs::File {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-diff-{}-{}-{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");
    // Unlink immediately; the open fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Symbol invocation helpers — one per ABI view of `void driver(int)`
// ---------------------------------------------------------------------------

/// `void driver(int)` on the C library.
fn c_driver(x: c_int) -> Vec<u8> {
    capture(|| {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            unsafe { libs().c.get(b"driver\0").expect("C driver symbol") };
        unsafe { f(x) }
    })
}

/// `void driver(int)` on the Rust library.
fn rust_driver(x: c_int) -> Vec<u8> {
    capture(|| {
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            unsafe { libs().rust.get(b"driver\0").expect("Rust driver symbol") };
        unsafe { f(x) }
    })
}

/// Same symbol, viewed as taking an `unsigned int` — models a caller that
/// passes a value with no valid signed reading (Phase C, rows E5/E6/E7).
fn c_driver_u32(x: c_uint) -> Vec<u8> {
    capture(|| {
        let f: Symbol<unsafe extern "C" fn(c_uint)> =
            unsafe { libs().c.get(b"driver\0").expect("C driver symbol") };
        unsafe { f(x) }
    })
}

fn rust_driver_u32(x: c_uint) -> Vec<u8> {
    capture(|| {
        let f: Symbol<unsafe extern "C" fn(c_uint)> =
            unsafe { libs().rust.get(b"driver\0").expect("Rust driver symbol") };
        unsafe { f(x) }
    })
}

/// Same symbol, viewed as taking a 64-bit value — models an oversized
/// argument whose upper half must be ignored by the SysV ABI (row E8).
fn c_driver_u64(x: u64) -> Vec<u8> {
    capture(|| {
        let f: Symbol<unsafe extern "C" fn(u64)> =
            unsafe { libs().c.get(b"driver\0").expect("C driver symbol") };
        unsafe { f(x) }
    })
}

fn rust_driver_u64(x: u64) -> Vec<u8> {
    capture(|| {
        let f: Symbol<unsafe extern "C" fn(u64)> =
            unsafe { libs().rust.get(b"driver\0").expect("Rust driver symbol") };
        unsafe { f(x) }
    })
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

#[track_caller]
fn assert_same(label: &str, x: i64, c: &[u8], r: &[u8]) {
    // Guard against harness text leaking into the capture: the libraries only
    // ever emit lowercase hex digits and newlines. Without this, contamination
    // could silently look like agreement.
    for (who, buf) in [("C", c), ("Rust", r)] {
        assert!(
            buf.iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b) || *b == b'\n'),
            "[{label}] {who} capture contaminated (non hex/newline bytes) for input {x}: {}",
            show(buf)
        );
    }
    assert_eq!(
        c,
        r,
        "[{label}] divergence for input {x} (0x{:x}):\n  C   : {}\n  Rust: {}",
        x as u64,
        show(c),
        show(r)
    );
}

#[track_caller]
fn check(label: &str, x: c_int) {
    let c = c_driver(x);
    let r = rust_driver(x);
    assert_same(label, i64::from(x), &c, &r);
    assert!(!c.is_empty(), "[{label}] C produced no output for {x}");
}

/// Deterministic 64-bit LCG (fixed seed) so every run drives the exact same
/// randomized inputs.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u32(&mut self) -> u32 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

const SEED: u64 = 0x2545_F491_4F6C_DD1D;

// ===========================================================================
// Phase B — valid-path differential tests (one per CONFIGS.md row)
// ===========================================================================

/// CONFIGS.md C1 — exhaustive small neighbourhood.
#[test]
fn config_c1_exhaustive_small_neighbourhood() {
    for x in -1024i32..=1024 {
        check("C1", x);
    }
}

/// CONFIGS.md C2 — 20000 seeded random `i32` values over the full range.
#[test]
fn config_c2_randomized_full_range() {
    let mut rng = Rng::new(SEED);
    for _ in 0..20_000 {
        check("C2", rng.next_i32());
    }
}

/// CONFIGS.md C3 — every single-bit pattern and its complement.
#[test]
fn config_c3_bit_pattern_sweep() {
    for bit in 0..32u32 {
        let v = 1u32 << bit;
        check("C3", v as i32);
        check("C3", !v as i32);
    }
}

/// CONFIGS.md C4 — boundary values.
#[test]
fn config_c4_boundary_values() {
    let cases: [i32; 15] = [
        0,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        0x7F,
        0x80,
        0xFF,
        0x100,
        0xFFFF,
        0x10000,
        0x00FF_00FF,
        -0x7FFF_FFFF,
    ];
    for x in cases {
        check("C4", x);
    }
}

/// CONFIGS.md C5 — repeated invocation with the same value must be stateless.
#[test]
fn config_c5_repeated_invocation_is_stateless() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    for _ in 0..64 {
        let x = rng.next_i32();
        let first_c = c_driver(x);
        let first_r = rust_driver(x);
        assert_same("C5", i64::from(x), &first_c, &first_r);
        for rep in 0..64 {
            let c = c_driver(x);
            let r = rust_driver(x);
            assert_same("C5", i64::from(x), &c, &r);
            assert_eq!(c, first_c, "C5: C output changed on repeat {rep} of {x}");
            assert_eq!(r, first_r, "C5: Rust output changed on repeat {rep} of {x}");
        }
    }
}

/// CONFIGS.md C6 — interleaved C/Rust/C calls through the shared stdio buffer.
#[test]
fn config_c6_interleaved_pipeline() {
    let mut rng = Rng::new(SEED ^ 0xC6);
    for _ in 0..2000 {
        let x = rng.next_i32();
        let y = rng.next_i32();

        // C, Rust, C, Rust in one capture each: the concatenation must match.
        let both_c = capture(|| {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { libs().c.get(b"driver\0").unwrap() };
            unsafe {
                f(x);
                f(y);
                f(x);
            }
        });
        let both_r = capture(|| {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { libs().rust.get(b"driver\0").unwrap() };
            unsafe {
                f(x);
                f(y);
                f(x);
            }
        });
        assert_same("C6", i64::from(x), &both_c, &both_r);

        // And a genuinely mixed sequence: C then Rust must equal Rust then C
        // reversed — i.e. neither library perturbs the other's output.
        let mixed = capture(|| {
            let cf: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { libs().c.get(b"driver\0").unwrap() };
            let rf: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { libs().rust.get(b"driver\0").unwrap() };
            unsafe {
                cf(x);
                rf(x);
                cf(y);
                rf(y);
            }
        });
        let lines: Vec<&[u8]> = mixed.split(|&b| b == b'\n').collect();
        // 4 lines + trailing empty
        assert_eq!(lines.len(), 5, "C6: unexpected line count in {}", show(&mixed));
        assert_eq!(lines[0], lines[1], "C6: C and Rust lines differ for {x}");
        assert_eq!(lines[2], lines[3], "C6: C and Rust lines differ for {y}");
        assert!(lines[4].is_empty(), "C6: trailing data after last newline");
    }
}

/// CONFIGS.md C7 — output-shape invariants on top of the byte comparison.
#[test]
fn config_c7_output_shape_invariants() {
    let mut rng = Rng::new(SEED ^ 0xC7);
    let mut inputs: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX];
    for _ in 0..2000 {
        inputs.push(rng.next_i32());
    }

    for x in inputs {
        let c = c_driver(x);
        let r = rust_driver(x);
        assert_same("C7", i64::from(x), &c, &r);

        // 16 bytes -> 32 hex digits + '\n'
        assert_eq!(c.len(), 33, "C7: unexpected output length for {x}: {}", show(&c));
        assert_eq!(c[32], b'\n', "C7: output not newline-terminated for {x}");
        assert!(
            c[..32].iter().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
            "C7: non-lowercase-hex output for {x}: {}",
            show(&c)
        );

        // Decode the 16-byte house_t image.
        let hex = std::str::from_utf8(&c[..32]).unwrap();
        let bytes: Vec<u8> = (0..16)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
            .collect();

        let floors = i32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let bedrooms = i32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let bathrooms = f64::from_le_bytes(bytes[8..16].try_into().unwrap());

        assert_eq!(floors, x, "C7: floors field mismatch for {x}");
        assert_eq!(bedrooms, 3, "C7: bedrooms field must be 3 for {x}");
        assert_eq!(bathrooms, 2.0, "C7: bathrooms field must be 2.0 for {x}");
        assert_eq!(
            &bytes[8..16],
            &2.0f64.to_le_bytes(),
            "C7: bathrooms IEEE-754 image mismatch for {x}"
        );
    }
}

/// CONFIGS.md C8 — `print_hex` must not be an exported symbol in either
/// library, so `driver` is genuinely the only reachable entry point.
#[test]
fn config_c8_static_helper_not_exported() {
    for (name, lib) in [("C", &libs().c), ("Rust", &libs().rust)] {
        let sym: Result<Symbol<unsafe extern "C" fn(*const u8, c_int)>, _> =
            unsafe { lib.get(b"print_hex\0") };
        assert!(
            sym.is_err(),
            "{name} library unexpectedly exports `print_hex`"
        );
    }
}

// ===========================================================================
// Phase C — error-path differential tests (one per ERRORS.md row)
// ===========================================================================

/// ERRORS.md E1–E4 — the zero / extreme / all-bits-set boundaries.
#[test]
fn error_surface_e1_e4_boundaries() {
    for (row, x) in [("E1", 0i32), ("E2", i32::MAX), ("E3", i32::MIN), ("E4", -1)] {
        let c = c_driver(x);
        let r = rust_driver(x);
        assert_same(row, i64::from(x), &c, &r);
        // The C rejects nothing: it always emits a full 33-byte line.
        assert_eq!(c.len(), 33, "{row}: C unexpectedly did not print a full line");
        assert_eq!(r.len(), 33, "{row}: Rust unexpectedly did not print a full line");
    }

    // Exact expected images, so the test pins the C's actual behaviour rather
    // than only "both agree".
    assert_eq!(c_driver(0), b"00000000030000000000000000000040\n");
    assert_eq!(rust_driver(0), b"00000000030000000000000000000040\n");
    assert_eq!(c_driver(-1), b"ffffffff030000000000000000000040\n");
    assert_eq!(rust_driver(-1), b"ffffffff030000000000000000000040\n");
    assert_eq!(c_driver(i32::MAX), b"ffffff7f030000000000000000000040\n");
    assert_eq!(rust_driver(i32::MAX), b"ffffff7f030000000000000000000040\n");
    assert_eq!(c_driver(i32::MIN), b"00000080030000000000000000000040\n");
    assert_eq!(rust_driver(i32::MIN), b"00000080030000000000000000000040\n");
}

/// ERRORS.md E5/E6 — values one step past the signed range, passed as
/// `unsigned int` across the FFI boundary. There is no valid-range check in
/// the C, so both must reinterpret identically.
#[test]
fn error_surface_e5_e6_out_of_signed_range() {
    for x in [
        0x8000_0000u32,
        0xFFFF_FFFFu32,
        0x8000_0001u32,
        0x7FFF_FFFFu32 + 1,
        u32::MAX - 1,
    ] {
        let c = c_driver_u32(x);
        let r = rust_driver_u32(x);
        assert_same("E5/E6", i64::from(x), &c, &r);
        // Must be identical to the signed reinterpretation — no rejection.
        assert_eq!(
            c,
            c_driver(x as i32),
            "E5/E6: C treated unsigned {x:#x} differently from its signed reading"
        );
        assert_eq!(
            r,
            rust_driver(x as i32),
            "E5/E6: Rust treated unsigned {x:#x} differently from its signed reading"
        );
    }
    assert_eq!(c_driver_u32(0x8000_0000), b"00000080030000000000000000000040\n");
    assert_eq!(rust_driver_u32(0x8000_0000), b"00000080030000000000000000000040\n");
}

/// ERRORS.md E7 — "out-of-range enum values". The library declares no enum, so
/// every int is a valid input and neither side may reject any of them.
#[test]
fn error_surface_e7_out_of_range_enum_like_values() {
    let cases: [i32; 11] = [
        i32::MIN,
        -2,
        -1,
        0,
        1,
        2,
        255,
        256,
        65535,
        65536,
        i32::MAX,
    ];
    for x in cases {
        let c = c_driver(x);
        let r = rust_driver(x);
        assert_same("E7", i64::from(x), &c, &r);
        assert_eq!(c.len(), 33, "E7: C rejected {x} (short output)");
        assert_eq!(r.len(), 33, "E7: Rust rejected {x} (short output)");
    }
    // Every distinct input must give a distinct output (the value is not
    // clamped, saturated, or normalised on either side).
    let mut c_outs: Vec<Vec<u8>> = cases.iter().map(|&x| c_driver(x)).collect();
    let mut r_outs: Vec<Vec<u8>> = cases.iter().map(|&x| rust_driver(x)).collect();
    assert_eq!(c_outs, r_outs, "E7: output sets diverge");
    c_outs.sort();
    c_outs.dedup();
    assert_eq!(c_outs.len(), cases.len(), "E7: C collapsed distinct inputs");
    r_outs.sort();
    r_outs.dedup();
    assert_eq!(r_outs.len(), cases.len(), "E7: Rust collapsed distinct inputs");
}

/// ERRORS.md E8 — oversized (64-bit) argument; the ABI truncates to 32 bits.
#[test]
fn error_surface_e8_oversized_argument() {
    let mut rng = Rng::new(SEED ^ 0xE8);
    let mut cases: Vec<u64> = vec![
        0x0000_0001_0000_0001,
        0xFFFF_FFFF_0000_0000,
        0xDEAD_BEEF_CAFE_BABE,
        u64::MAX,
        0x1_8000_0000,
    ];
    for _ in 0..256 {
        cases.push(((rng.next_u32() as u64) << 32) | rng.next_u32() as u64);
    }

    for x in cases {
        let c = c_driver_u64(x);
        let r = rust_driver_u64(x);
        assert_same("E8", x as i64, &c, &r);
        // Must equal the low-32-bit call on both sides.
        let low = x as u32 as i32;
        assert_eq!(c, c_driver(low), "E8: C did not truncate {x:#x} to {low}");
        assert_eq!(r, rust_driver(low), "E8: Rust did not truncate {x:#x} to {low}");
    }
}

/// ERRORS.md E9/E10 — documented as not applicable; this test proves the
/// premise (the public API takes exactly one scalar and no pointer/length) by
/// showing that calling `driver` with a pointer-shaped bit pattern is still
/// just an integer to both libraries, i.e. there is no pointer to be null.
#[test]
fn error_surface_e9_e10_no_pointer_or_length_parameter() {
    // A "null pointer"-looking argument is simply the integer 0 (== row E1),
    // and an arbitrary pointer-looking bit pattern is simply its low 32 bits.
    assert_eq!(c_driver(0), rust_driver(0));
    assert_eq!(c_driver(0), c_driver_u64(0));
    assert_eq!(rust_driver(0), rust_driver_u64(0));

    let ptrish = 0x7FFF_1234_5678u64;
    assert_eq!(c_driver_u64(ptrish), rust_driver_u64(ptrish));
    assert_eq!(c_driver_u64(ptrish), c_driver(ptrish as u32 as i32));

    // The header exposes no length parameter, so `print_hex`'s `len` cannot be
    // driven to 0 or an oversized value: the output width is invariant.
    let mut rng = Rng::new(SEED ^ 0xE9);
    for _ in 0..512 {
        let x = rng.next_i32();
        assert_eq!(c_driver(x).len(), 33, "output width varied for {x}");
        assert_eq!(rust_driver(x).len(), 33, "output width varied for {x}");
    }
}
