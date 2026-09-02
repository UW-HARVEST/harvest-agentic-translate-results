//! Differential tests: load BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compare their observable output byte-for-byte.
//!
//! Neither side is ever called as a Rust function directly — every invocation
//! goes through a `dlsym`'d `extern "C"` symbol, so the `#[unsafe(no_mangle)]`
//! export wrapper is part of what is under test.
//!
//! `driver` returns `void` and communicates only through libc `stdout`, so the
//! "output" being compared is the exact byte stream each `.so` writes to file
//! descriptor 1. `capture_fd1` redirects fd 1 to a temporary file around the
//! call, flushes every libc stream, restores fd 1 and reads the bytes back.

use std::ffi::c_int;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bindings used by the stdout-capture harness.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// `fflush(NULL)` — flush every open libc output stream, including the `stdout`
/// that both shared objects' `printf`/`putchar` calls write into.
fn fflush_all() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

/// Run `f` with file descriptor 1 redirected to a fresh temporary file and
/// return every byte written to it.
///
/// fd 1 is process-global, so captures must not overlap; the mutex makes the
/// suite correct under `cargo test`'s default parallel harness as well as
/// `--test-threads=1`.
fn capture_fd1<F: FnOnce()>(f: F) -> Vec<u8> {
    static FD1_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = FD1_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Push out anything this harness itself has buffered in Rust's `stdout`
    // wrapper, then flush every libc stream, so nothing pre-existing lands on
    // the wrong side of the redirection.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    fflush_all();

    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(format!(
        "driver-diff-{}-{:?}-{}.out",
        std::process::id(),
        std::thread::current().id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("create capture temp file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    // The call under test.
    f();

    // Push the library's buffered bytes into the temp file *before* putting the
    // real stdout back, otherwise they would be flushed to the wrong place.
    fflush_all();

    assert!(unsafe { dup2(saved, 1) } >= 0, "restore fd 1 failed");
    unsafe {
        close(saved);
    }

    let mut buf = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("rewind capture file");
    file.read_to_end(&mut buf).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&tmp_path);
    buf
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// Loading the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().expect("crate parent dir").to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/Release/libdriver.so"),
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

/// Resolve the Rust `.so` under test.
///
/// IMPORTANT: `cargo test` does **not** build this crate's `cdylib` artifact —
/// the crate declares `crate-type = ["cdylib"]` only, so there is no `rlib` for
/// the integration test to link and Cargo emits no `libdriver.so` for the test
/// profile. Simply globbing `target/{debug,release}` therefore risks loading a
/// STALE shared object and silently passing every differential assertion, so
/// the harness builds the cdylib itself into a dedicated target directory
/// (separate from the one the running `cargo test` holds a lock on) and then
/// asserts the artifact is newer than the sources.
///
/// Extra flags — notably `--no-default-features` / `--features <combo>` — are
/// forwarded through the `DIFFTEST_CARGO_ARGS` environment variable so a
/// feature-combination sweep tests the matching `.so`.
fn rust_library_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    static BUILT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BUILT.get_or_init(build_rust_cdylib).clone()
}

fn build_rust_cdylib() -> PathBuf {
    let manifest = manifest_dir();
    let target_dir = manifest.join("target").join("difftest");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut cmd = std::process::Command::new(&cargo);
    cmd.current_dir(&manifest)
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        // Ask Cargo which file it produced instead of guessing a path: this is
        // authoritative, so a `.so` Cargo did not just build can never be
        // mistaken for the artifact under test.
        .arg("--message-format=json-render-diagnostics");
    let extra = std::env::var("DIFFTEST_CARGO_ARGS").unwrap_or_default();
    for a in extra.split_whitespace() {
        cmd.arg(a);
    }
    // Do not let the outer `cargo test` invocation's per-crate env leak into the
    // nested build.
    cmd.env_remove("CARGO_MANIFEST_DIR")
        .env_remove("CARGO_PKG_NAME")
        .env_remove("CARGO_CRATE_NAME")
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTC_WRAPPER");

    let out = cmd.output().expect("spawn nested `cargo build` for the cdylib");
    assert!(
        out.status.success(),
        "nested `cargo build --release {extra}` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Pull the cdylib path out of the `compiler-artifact` message for the
    // `driver` target. Minimal scanning keeps the harness dependency-free.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut found: Option<PathBuf> = None;
    for line in stdout.lines() {
        if !line.contains("\"compiler-artifact\"") {
            continue;
        }
        for piece in line.split('"') {
            if piece.ends_with("libdriver.so") && piece.starts_with('/') {
                found = Some(PathBuf::from(piece));
            }
        }
    }

    let so = match found {
        Some(p) => p,
        // A no-op rebuild emits no artifact message; fall back to the canonical
        // location inside our private target dir, which only this harness writes.
        None => target_dir.join("release").join("libdriver.so"),
    };
    assert!(
        so.is_file(),
        "nested build reported success but {so:?} does not exist"
    );
    assert!(
        so.starts_with(&target_dir),
        "resolved {so:?} outside the harness's private target dir {target_dir:?}"
    );
    so
}

/// The two libraries plus their resolved `driver` symbols.
struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c_driver: Symbol<'static, unsafe extern "C" fn(f32)>,
    rust_driver: Symbol<'static, unsafe extern "C" fn(f32)>,
}

impl Pair {
    fn load() -> Self {
        let c_lib = unsafe { Library::new(c_library_path()) }.expect("dlopen C libdriver.so");
        let rust_lib = unsafe { Library::new(rust_library_path()) }.expect("dlopen Rust libdriver.so");

        // SAFETY: the symbols borrow from libraries that live in the same
        // struct and are never moved out or dropped before the symbols.
        let c_driver: Symbol<unsafe extern "C" fn(f32)> =
            unsafe { c_lib.get(b"driver\0") }.expect("C `driver` symbol");
        let rust_driver: Symbol<unsafe extern "C" fn(f32)> =
            unsafe { rust_lib.get(b"driver\0") }.expect("Rust `driver` symbol (no_mangle export)");
        let c_driver = unsafe {
            std::mem::transmute::<Symbol<unsafe extern "C" fn(f32)>, Symbol<'static, unsafe extern "C" fn(f32)>>(
                c_driver,
            )
        };
        let rust_driver = unsafe {
            std::mem::transmute::<Symbol<unsafe extern "C" fn(f32)>, Symbol<'static, unsafe extern "C" fn(f32)>>(
                rust_driver,
            )
        };

        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_driver,
            rust_driver,
        }
    }

    /// Call the C `driver` once per element of `bits` (each reinterpreted as a
    /// `float`) inside a single fd-1 capture.
    fn c_batch(&self, bits: &[u32]) -> Vec<u8> {
        let f = &self.c_driver;
        capture_fd1(|| {
            for &b in bits {
                unsafe { f(f32::from_bits(b)) };
            }
        })
    }

    /// Same, through the Rust `.so`'s exported symbol.
    fn rust_batch(&self, bits: &[u32]) -> Vec<u8> {
        let f = &self.rust_driver;
        capture_fd1(|| {
            for &b in bits {
                unsafe { f(f32::from_bits(b)) };
            }
        })
    }

    /// Assert the two `.so`s emit byte-identical output for `bits`, and that
    /// the output has the shape the C code must produce (one 8-hex-digit line
    /// per call, `\n`-terminated, nothing else).
    fn assert_same(&self, label: &str, bits: &[u32]) {
        let c_out = self.c_batch(bits);
        let rust_out = self.rust_batch(bits);

        if c_out != rust_out {
            // Narrow the failure down to the first diverging input.
            for &b in bits {
                let c1 = self.c_batch(&[b]);
                let r1 = self.rust_batch(&[b]);
                if c1 != r1 {
                    panic!(
                        "[{label}] divergence on bits=0x{b:08x} (as f32: {})\n  C   : {:?}\n  Rust: {:?}",
                        f32::from_bits(b),
                        String::from_utf8_lossy(&c1),
                        String::from_utf8_lossy(&r1)
                    );
                }
            }
            panic!(
                "[{label}] batch output differs but no single input diverges \
                 (stream-level difference): C {} bytes, Rust {} bytes",
                c_out.len(),
                rust_out.len()
            );
        }

        assert_eq!(
            c_out.len(),
            bits.len() * 9,
            "[{label}] expected 9 bytes (8 hex digits + newline) per call, got {} for {} calls",
            c_out.len(),
            bits.len()
        );
        for (i, (chunk, &b)) in c_out.chunks(9).zip(bits.iter()).enumerate() {
            assert_eq!(chunk[8], b'\n', "[{label}] call {i} not newline-terminated");
            let hex = std::str::from_utf8(&chunk[..8]).expect("ascii hex");
            assert!(
                hex.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
                "[{label}] call {i} produced non-lowercase-hex {hex:?}"
            );
            // Independent re-derivation of the expected bytes: native-endian
            // object representation of the float, two lowercase hex digits per
            // byte (this is what `%02x` over `unsigned char` must yield).
            let expected: String = f32::from_bits(b)
                .to_ne_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            assert_eq!(hex, expected, "[{label}] call {i} bits=0x{b:08x}");
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible property-style testing).
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    fn next_u32(&mut self) -> u32 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 16) as u32
    }
}

// ===========================================================================
// Phase A sanity: the exported surface really is what SYMBOLS.md claims.
// ===========================================================================

fn test_harness_is_not_vacuous() {
    // 1. The capture harness must actually observe bytes (a silently-empty
    //    capture would make every differential assertion pass trivially).
    let pair = Pair::load();
    let c = pair.c_batch(&[1.0f32.to_bits()]);
    assert_eq!(
        c, b"0000803f\n",
        "capture harness did not observe the C library's stdout output"
    );
    assert_eq!(pair.rust_batch(&[1.0f32.to_bits()]), b"0000803f\n");

    // 2. `assert_same` must reject a difference. Feed it a deliberately wrong
    //    expectation by comparing captures of different inputs.
    let a = pair.c_batch(&[1.0f32.to_bits()]);
    let b = pair.rust_batch(&[2.0f32.to_bits()]);
    assert_ne!(a, b, "harness cannot distinguish different inputs");

    // 3. Byte order must be native little-endian on this target, i.e. the C
    //    `memcpy` output is NOT big-endian. A translation using `to_be_bytes`
    //    would produce "3f800000\n" here, so this pins the endianness down.
    assert_ne!(c, b"3f800000\n", "unexpected big-endian output from the C library");
}

fn test_both_libraries_export_driver() {
    let pair = Pair::load();
    // Resolving the symbols is the test; touch them so nothing is optimised out.
    let _ = &pair.c_driver;
    let _ = &pair.rust_driver;
}

fn test_static_helper_is_not_exported_by_either_so() {
    // `print_hex` is `static` in C, so neither `.so` may export it. A Rust
    // translation that leaked it would be a symbol-surface mismatch.
    let c_lib = unsafe { Library::new(c_library_path()) }.unwrap();
    let rust_lib = unsafe { Library::new(rust_library_path()) }.unwrap();
    let c: Result<Symbol<unsafe extern "C" fn(*const u8, c_int)>, _> =
        unsafe { c_lib.get(b"print_hex\0") };
    let r: Result<Symbol<unsafe extern "C" fn(*const u8, c_int)>, _> =
        unsafe { rust_lib.get(b"print_hex\0") };
    assert!(c.is_err(), "C .so unexpectedly exports the static print_hex");
    assert!(
        r.is_err(),
        "Rust .so exports print_hex but the C original has internal linkage"
    );
}

// ===========================================================================
// Phase B — one test per CONFIGS.md row.
// ===========================================================================

/// CONFIGS.md row 1 — signed zeros.
fn config_row_01_signed_zeros() {
    let pair = Pair::load();
    pair.assert_same("row1/signed zeros", &[0x0000_0000, 0x8000_0000]);
}

/// CONFIGS.md row 2 — positive subnormals.
fn config_row_02_positive_subnormals() {
    let pair = Pair::load();
    let mut bits = vec![0x0000_0001, 0x0000_0002, 0x0000_00ff, 0x0000_ffff, 0x007f_ffff, 0x0040_0000];
    let mut rng = Rng::new(0x5011);
    for _ in 0..2000 {
        // exponent field 0, random non-zero mantissa, sign clear
        let m = (rng.next_u32() & 0x007f_ffff) | 1;
        bits.push(m);
    }
    pair.assert_same("row2/+subnormal", &bits);
}

/// CONFIGS.md row 3 — negative subnormals.
fn config_row_03_negative_subnormals() {
    let pair = Pair::load();
    let mut bits = vec![0x8000_0001, 0x8000_0002, 0x8000_00ff, 0x8000_ffff, 0x807f_ffff];
    let mut rng = Rng::new(0x5012);
    for _ in 0..2000 {
        let m = (rng.next_u32() & 0x007f_ffff) | 1;
        bits.push(0x8000_0000 | m);
    }
    pair.assert_same("row3/-subnormal", &bits);
}

/// CONFIGS.md row 4 — smallest normals (subnormal/normal boundary).
fn config_row_04_smallest_normals() {
    let pair = Pair::load();
    pair.assert_same(
        "row4/smallest normal",
        &[
            f32::MIN_POSITIVE.to_bits(),
            (-f32::MIN_POSITIVE).to_bits(),
            0x0080_0000,
            0x8080_0000,
            0x007f_ffff, // largest subnormal, one step below
            0x0080_0001, // one step above the smallest normal
        ],
    );
}

/// CONFIGS.md row 5 — ordinary normal values, both signs.
fn config_row_05_ordinary_normals() {
    let pair = Pair::load();
    let handpicked: Vec<u32> = [
        0.0f32, 1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 3.14159265, -2.718281828, 1e-30, -1e-30, 1e30,
        -1e30, 255.0, 256.0, 65535.0, 16777216.0, 16777217.0, -0.1, 0.1, 1.0 / 3.0, 123456.789,
    ]
    .iter()
    .map(|v| v.to_bits())
    .collect();
    pair.assert_same("row5/normals handpicked", &handpicked);

    // Randomized normals: exponent in 1..=254, arbitrary sign and mantissa.
    let mut rng = Rng::new(0x5015);
    let mut bits = Vec::with_capacity(4000);
    for _ in 0..4000 {
        let sign = (rng.next_u32() & 1) << 31;
        let exp = 1 + (rng.next_u32() % 254);
        let mant = rng.next_u32() & 0x007f_ffff;
        bits.push(sign | (exp << 23) | mant);
    }
    pair.assert_same("row5/normals random", &bits);
}

/// CONFIGS.md row 6 — largest finite magnitudes.
fn config_row_06_largest_finite() {
    let pair = Pair::load();
    pair.assert_same(
        "row6/largest finite",
        &[
            f32::MAX.to_bits(),
            f32::MIN.to_bits(),
            0x7f7f_ffff,
            0xff7f_ffff,
            0x7f7f_fffe,
            0x7f00_0000,
        ],
    );
}

/// CONFIGS.md row 7 — infinities.
fn config_row_07_infinities() {
    let pair = Pair::load();
    pair.assert_same(
        "row7/infinities",
        &[
            f32::INFINITY.to_bits(),
            f32::NEG_INFINITY.to_bits(),
            0x7f80_0000,
            0xff80_0000,
        ],
    );
}

/// CONFIGS.md row 8 — quiet NaNs, every payload class, both signs.
fn config_row_08_quiet_nan_payloads() {
    let pair = Pair::load();
    let mut bits = vec![
        0x7fc0_0000, // canonical qNaN
        0xffc0_0000, // negative qNaN
        0x7fff_ffff, // all-ones payload
        0xffff_ffff,
        0x7fc0_0001,
        f32::NAN.to_bits(),
    ];
    let mut rng = Rng::new(0x5018);
    for _ in 0..3000 {
        let sign = (rng.next_u32() & 1) << 31;
        // exponent all ones, mantissa high bit set => quiet NaN
        let mant = (rng.next_u32() & 0x007f_ffff) | 0x0040_0000;
        bits.push(sign | 0x7f80_0000 | mant);
    }
    pair.assert_same("row8/qNaN", &bits);
}

/// CONFIGS.md row 9 — signalling NaNs must survive the FFI boundary unaltered.
///
/// This is the closest analogue to "an out-of-range enum value": a bit pattern
/// with no meaningful numeric interpretation, which the C code nevertheless
/// copies byte-for-byte. Any NaN canonicalisation on the Rust side (a real
/// hazard for float arguments) would show up here.
fn config_row_09_signalling_nan_payloads() {
    let pair = Pair::load();
    let mut bits = vec![
        0x7f80_0001, // smallest sNaN payload
        0xff80_0001,
        0x7fbf_ffff, // largest sNaN payload
        0xffbf_ffff,
        0x7f80_0002,
        0x7fa0_0000,
    ];
    let mut rng = Rng::new(0x5019);
    for _ in 0..3000 {
        let sign = (rng.next_u32() & 1) << 31;
        // exponent all ones, mantissa high bit CLEAR and payload non-zero
        let mant = ((rng.next_u32() & 0x003f_ffff) | 1) & 0x003f_ffff;
        bits.push(sign | 0x7f80_0000 | mant);
    }
    pair.assert_same("row9/sNaN", &bits);
}

/// CONFIGS.md row 10 — full exponent-field sweep plus exponent boundaries.
fn config_row_10_exponent_sweep() {
    let pair = Pair::load();
    let mut bits = Vec::new();
    for exp in 0u32..=255 {
        for &mant in &[0u32, 1, 0x0040_0000, 0x007f_ffff] {
            bits.push((exp << 23) | mant);
            bits.push(0x8000_0000 | (exp << 23) | mant);
        }
    }
    pair.assert_same("row10/exponent sweep", &bits);
}

/// CONFIGS.md row 11 — every byte value 0x00..=0xff in every byte position.
///
/// Covers `%02x` zero-padding for 0x00..0x0f and, critically, that 0x80..0xff
/// are *not* sign-extended by the `unsigned char` -> `int` variadic promotion
/// (a bug there would print `ffffff80` instead of `80`).
fn config_row_11_every_byte_value_in_every_position() {
    let pair = Pair::load();
    for pos in 0..4u32 {
        let mut bits = Vec::with_capacity(256);
        for v in 0u32..=255 {
            bits.push(v << (8 * pos));
        }
        pair.assert_same(&format!("row11/byte pos {pos}"), &bits);
    }
    // And all four positions holding the same value simultaneously.
    let mut bits = Vec::with_capacity(256);
    for v in 0u32..=255 {
        bits.push(v | (v << 8) | (v << 16) | (v << 24));
    }
    pair.assert_same("row11/all positions", &bits);
}

/// CONFIGS.md row 12 — seeded uniform-random full-domain sweep.
fn config_row_12_random_full_domain() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xDEAD_BEEF);
    let bits: Vec<u32> = (0..20_000).map(|_| rng.next_u32()).collect();
    pair.assert_same("row12/random", &bits);
}

/// CONFIGS.md row 13 — exhaustive low-order bit-pattern window.
fn config_row_13_exhaustive_low_half() {
    let pair = Pair::load();
    let bits: Vec<u32> = (0u32..=0xffff).collect();
    pair.assert_same("row13/low half exhaustive", &bits);
}

/// CONFIGS.md row 14 — exhaustive high-order bit-pattern sweep.
fn config_row_14_exhaustive_high_half() {
    let pair = Pair::load();
    let bits: Vec<u32> = (0u32..=0xffff).map(|h| h << 16).collect();
    pair.assert_same("row14/high half exhaustive", &bits);
}

/// CONFIGS.md row 15 — the composed pipeline: many sequential calls sharing one
/// `stdout` stream. Verifies framing does not drift and that per-call output
/// concatenates identically, which per-call captures cannot show.
fn config_row_15_many_sequential_calls_one_stream() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1005);
    let bits: Vec<u32> = (0..500).map(|_| rng.next_u32()).collect();

    let batched_c = pair.c_batch(&bits);
    let batched_rust = pair.rust_batch(&bits);
    assert_eq!(batched_c, batched_rust, "row15: batched streams differ");

    // The concatenation of individual captures must equal the single batched
    // capture, for both libraries.
    let mut per_call_c = Vec::new();
    let mut per_call_rust = Vec::new();
    for &b in bits.iter().take(60) {
        per_call_c.extend_from_slice(&pair.c_batch(&[b]));
        per_call_rust.extend_from_slice(&pair.rust_batch(&[b]));
    }
    assert_eq!(per_call_c, per_call_rust, "row15: per-call streams differ");
    assert_eq!(
        per_call_c,
        batched_c[..60 * 9].to_vec(),
        "row15: C batching changes framing"
    );
    assert_eq!(
        per_call_rust,
        batched_rust[..60 * 9].to_vec(),
        "row15: Rust batching changes framing"
    );
}

// ===========================================================================
// Phase C — error / boundary paths (see ERRORS.md).
//
// The C library has NO rejection path: `void driver(float)` has no return
// value, no pointer parameter, no enum/flag parameter, no assertion and no
// range check. The tests below pin down that fact and cover the generic
// boundaries in the only forms this API can express them.
// ===========================================================================

/// ERRORS.md B1 — there is no pointer parameter to pass NULL through.
///
/// Asserted structurally: `driver`'s ABI signature takes a single `float` in a
/// vector register. Calling it through a mistyped pointer-taking signature is
/// not a valid input to the C API, so the "null pointer" boundary is N/A. What
/// *is* testable is that neither `.so` exposes any pointer-taking entry point.
fn errors_b1_no_pointer_parameter_documented() {
    let c_lib = unsafe { Library::new(c_library_path()) }.unwrap();
    let rust_lib = unsafe { Library::new(rust_library_path()) }.unwrap();
    // The full export set is `driver` alone (see SYMBOLS.md); confirm the only
    // plausible pointer-taking names are absent from both.
    for name in [
        &b"print_hex\0"[..],
        &b"driver_ptr\0"[..],
        &b"driver_ex\0"[..],
        &b"driver_buf\0"[..],
    ] {
        let c: Result<Symbol<unsafe extern "C" fn()>, _> = unsafe { c_lib.get(name) };
        let r: Result<Symbol<unsafe extern "C" fn()>, _> = unsafe { rust_lib.get(name) };
        assert_eq!(
            c.is_ok(),
            r.is_ok(),
            "export presence differs for {:?}",
            String::from_utf8_lossy(name)
        );
    }
}

/// ERRORS.md B2/B3 — the length passed to the internal `print_hex` is the
/// compile-time constant `sizeof(float)`, so zero/oversized lengths are
/// unreachable from the public API. What must hold for *every* input is the
/// output shape: exactly 8 hex digits plus one newline, never more, never less.
fn errors_b2_b3_output_shape_is_always_nine_bytes() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xB23B_23);
    let mut bits: Vec<u32> = vec![0, 0xffff_ffff, 0x7f80_0000, 0x0000_0001];
    bits.extend((0..3000).map(|_| rng.next_u32()));

    for &b in &bits {
        let c_out = pair.c_batch(std::slice::from_ref(&b));
        let r_out = pair.rust_batch(std::slice::from_ref(&b));
        assert_eq!(c_out, r_out, "shape divergence on 0x{b:08x}");
        assert_eq!(c_out.len(), 9, "bits=0x{b:08x} produced {} bytes", c_out.len());
        assert_eq!(c_out[8], b'\n');
    }
}

/// ERRORS.md B4 — the analogue of an out-of-range enum crossing FFI: a `float`
/// bit pattern that is not a number at all. Every one of the 2^24-2 NaN
/// encodings per sign is a legal input the C copies verbatim; sampled densely
/// here, including the qNaN/sNaN discriminator boundary in both directions.
fn errors_b4_nan_and_invalid_encodings_pass_through_verbatim() {
    let pair = Pair::load();
    let mut bits = Vec::new();
    // Walk the whole NaN mantissa space in strides, both signs, so both the
    // sNaN (high mantissa bit 0) and qNaN (high mantissa bit 1) halves are hit.
    let mut mant = 1u32;
    while mant < 0x0080_0000 {
        bits.push(0x7f80_0000 | mant);
        bits.push(0xff80_0000 | mant);
        mant = mant.wrapping_add(0x0000_1fff);
    }
    // Exact discriminator boundary.
    bits.extend_from_slice(&[
        0x7fbf_ffff, // last sNaN
        0x7fc0_0000, // first qNaN
        0xffbf_ffff,
        0xffc0_0000,
    ]);
    pair.assert_same("B4/NaN encodings", &bits);
}

/// ERRORS.md B5 — one step past each documented range endpoint. `float`'s
/// documented range is the whole type, so the endpoints are the encoding
/// extremes and their immediate neighbours in bit-pattern order.
fn errors_b5_one_step_past_every_range_endpoint() {
    let pair = Pair::load();
    let endpoints: [u32; 12] = [
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x007f_ffff, // largest +subnormal
        0x0080_0000, // smallest +normal (one step past subnormal range)
        0x7f7f_ffff, // f32::MAX
        0x7f80_0000, // one step past MAX -> +inf
        0x7f80_0001, // one step past +inf -> sNaN
        0xff7f_ffff, // f32::MIN
        0xff80_0000, // one step past MIN -> -inf
        0xff80_0001, // one step past -inf -> -sNaN
        0x7fff_ffff, // last +NaN encoding
        0xffff_ffff, // last -NaN encoding
    ];
    let mut bits = Vec::new();
    for e in endpoints {
        bits.push(e.wrapping_sub(1));
        bits.push(e);
        bits.push(e.wrapping_add(1));
    }
    pair.assert_same("B5/range endpoints +-1", &bits);
}

/// ERRORS.md B6 — `%02x` value-dependent formatting: zero padding for small
/// bytes and no sign extension for bytes >= 0x80, in isolation and combined.
fn errors_b6_hex_formatting_edge_values() {
    let pair = Pair::load();
    let mut bits = Vec::new();
    for pos in 0..4u32 {
        for v in [0x00u32, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0x81, 0xf0, 0xfe, 0xff] {
            bits.push(v << (8 * pos));
            bits.push(0xffff_ffff ^ (v << (8 * pos)));
        }
    }
    pair.assert_same("B6/hex formatting", &bits);

    // Spot-check the literal expected text for the two classic failure modes.
    let low = pair.c_batch(&[0x0000_0001]);
    assert_eq!(&low, b"01000000\n", "0x01 must zero-pad to \"01\"");
    assert_eq!(pair.rust_batch(&[0x0000_0001]), low);

    let high = pair.c_batch(&[0x0000_0080]);
    assert_eq!(&high, b"80000000\n", "0x80 must not sign-extend");
    assert_eq!(pair.rust_batch(&[0x0000_0080]), high);
}

/// ABI regression guard for the `float` parameter's register class.
///
/// `void driver(float)` passes its argument in a vector register (`%xmm0` on
/// x86-64 SysV); an `int`/`u32` parameter would be passed in a general-purpose
/// register (`%edi`). A translation that declares the parameter as an integer
/// still compiles, still exports `driver`, and still passes `nm -D` parity — it
/// just reads the wrong register and returns garbage to every caller.
///
/// This case pins the expected bytes as literals so it fails deterministically
/// on such a mismatch, rather than depending on whatever happens to be left in
/// the integer registers at the call site.
fn errors_b8_float_abi_register_class() {
    let pair = Pair::load();
    // (input, exact expected stdout) — little-endian object representation.
    let vectors: [(f32, &[u8]); 6] = [
        (1.0, b"0000803f\n"),
        (-1.0, b"000080bf\n"),
        (2.0, b"00000040\n"),
        (0.5, b"0000003f\n"),
        (f32::INFINITY, b"0000807f\n"),
        (255.0, b"00007f43\n"),
    ];
    for (v, expected) in vectors {
        let c = pair.c_batch(&[v.to_bits()]);
        let r = pair.rust_batch(&[v.to_bits()]);
        assert_eq!(
            c, expected,
            "C .so output for {v} changed: {:?}",
            String::from_utf8_lossy(&c)
        );
        assert_eq!(
            r,
            expected,
            "Rust .so output for {v} is {:?}, expected {:?} — the `float` argument is \
             probably being read from the wrong register (integer instead of vector), \
             i.e. the exported signature is not `extern \"C\" fn(f32)`",
            String::from_utf8_lossy(&r),
            String::from_utf8_lossy(expected)
        );
    }
}

/// Repeated interleaving of the two libraries inside a single capture: if the
/// Rust translation used a different output mechanism (e.g. Rust's own
/// line-buffered `std::io::stdout` instead of libc `printf`), the interleaved
/// order would break even though every isolated call matched.
fn errors_b7_interleaved_calls_preserve_ordering() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1234_5678_9abc_def0);
    let bits: Vec<u32> = (0..200).map(|_| rng.next_u32()).collect();

    let c = &pair.c_driver;
    let r = &pair.rust_driver;
    let interleaved = capture_fd1(|| {
        for &b in &bits {
            unsafe { c(f32::from_bits(b)) };
            unsafe { r(f32::from_bits(b)) };
        }
    });

    // Each input must appear twice in a row, identically.
    assert_eq!(interleaved.len(), bits.len() * 2 * 9);
    for (i, pair_of_lines) in interleaved.chunks(18).enumerate() {
        assert_eq!(
            &pair_of_lines[..9],
            &pair_of_lines[9..],
            "interleaved call {i} (bits=0x{:08x}) diverged",
            bits[i]
        );
    }
}

// ===========================================================================
// Sequential runner (`harness = false`).
//
// Runs every case one at a time so that no other thread can write to fd 1
// while a capture window is open. Failures are collected rather than aborting,
// so one diverging row does not hide the others. All runner output goes to
// stderr, which is never redirected.
// ===========================================================================

fn all_cases() -> Vec<(&'static str, fn())> {
    vec![
        // Harness self-checks / Phase A symbol surface.
        ("test_harness_is_not_vacuous", test_harness_is_not_vacuous as fn()),
        ("test_both_libraries_export_driver", test_both_libraries_export_driver),
        (
            "test_static_helper_is_not_exported_by_either_so",
            test_static_helper_is_not_exported_by_either_so,
        ),
        // Phase B — CONFIGS.md rows 1..15.
        ("config_row_01_signed_zeros", config_row_01_signed_zeros),
        ("config_row_02_positive_subnormals", config_row_02_positive_subnormals),
        ("config_row_03_negative_subnormals", config_row_03_negative_subnormals),
        ("config_row_04_smallest_normals", config_row_04_smallest_normals),
        ("config_row_05_ordinary_normals", config_row_05_ordinary_normals),
        ("config_row_06_largest_finite", config_row_06_largest_finite),
        ("config_row_07_infinities", config_row_07_infinities),
        ("config_row_08_quiet_nan_payloads", config_row_08_quiet_nan_payloads),
        (
            "config_row_09_signalling_nan_payloads",
            config_row_09_signalling_nan_payloads,
        ),
        ("config_row_10_exponent_sweep", config_row_10_exponent_sweep),
        (
            "config_row_11_every_byte_value_in_every_position",
            config_row_11_every_byte_value_in_every_position,
        ),
        ("config_row_12_random_full_domain", config_row_12_random_full_domain),
        ("config_row_13_exhaustive_low_half", config_row_13_exhaustive_low_half),
        ("config_row_14_exhaustive_high_half", config_row_14_exhaustive_high_half),
        (
            "config_row_15_many_sequential_calls_one_stream",
            config_row_15_many_sequential_calls_one_stream,
        ),
        // Phase C — ERRORS.md rows / generic boundaries.
        (
            "errors_b1_no_pointer_parameter_documented",
            errors_b1_no_pointer_parameter_documented,
        ),
        (
            "errors_b2_b3_output_shape_is_always_nine_bytes",
            errors_b2_b3_output_shape_is_always_nine_bytes,
        ),
        (
            "errors_b4_nan_and_invalid_encodings_pass_through_verbatim",
            errors_b4_nan_and_invalid_encodings_pass_through_verbatim,
        ),
        (
            "errors_b5_one_step_past_every_range_endpoint",
            errors_b5_one_step_past_every_range_endpoint,
        ),
        ("errors_b6_hex_formatting_edge_values", errors_b6_hex_formatting_edge_values),
        (
            "errors_b7_interleaved_calls_preserve_ordering",
            errors_b7_interleaved_calls_preserve_ordering,
        ),
        ("errors_b8_float_abi_register_class", errors_b8_float_abi_register_class),
    ]
}

fn main() {
    use std::io::Write;

    // Accept an optional substring filter; ignore libtest-style flags such as
    // `--test-threads=1` so `cargo test -- --test-threads=1` still works.
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));

    let selected: Vec<_> = all_cases()
        .into_iter()
        .filter(|(name, _)| match filter.as_deref() {
            Some(f) => name.contains(f),
            None => true,
        })
        .collect();

    eprintln!("\nrunning {} differential cases", selected.len());
    // Resolve the libraries once up front so a build/dlopen problem is reported
    // as a setup failure rather than as N identical case failures.
    eprintln!("  C    .so: {}", c_library_path().display());
    eprintln!("  Rust .so: {}", rust_library_path().display());

    let mut failed: Vec<&str> = Vec::new();
    for (name, case) in &selected {
        eprint!("case {name} ... ");
        let _ = std::io::stderr().flush();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(case)) {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    eprintln!();
    if failed.is_empty() {
        eprintln!("differential result: ok. {} passed; 0 failed", selected.len());
    } else {
        eprintln!("failures:");
        for f in &failed {
            eprintln!("    {f}");
        }
        eprintln!(
            "differential result: FAILED. {} passed; {} failed",
            selected.len() - failed.len(),
            failed.len()
        );
        std::process::exit(1);
    }
}
