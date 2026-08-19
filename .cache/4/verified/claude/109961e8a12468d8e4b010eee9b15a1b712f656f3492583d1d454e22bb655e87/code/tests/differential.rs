//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! The Rust implementation is NEVER called directly as a Rust function; it is
//! always reached through `dlsym` on `target/<profile>/libdriver.so`, exactly as
//! an external C consumer would, so the `#[no_mangle] extern "C"` export wrapper
//! is part of what is under test.
//!
//! Both libraries write into the *same* glibc `stdout` FILE (they share the one
//! `libc.so.6` mapping in this process), so the comparison is a true byte-level
//! comparison of the emitted stream.
//!
//! Row IDs (`cfg_NN`) map 1:1 to `CONFIGS.md`; (`err_NN`) map 1:1 to `ERRORS.md`.

use std::ffi::c_int;
use std::ffi::c_void;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::Library;

// ---------------------------------------------------------------------------
// libc bits used only by the harness (never by the code under test)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, including the `stdout`
    /// buffer that both `.so`s append to.
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;

/// Exactly 8 lowercase hex digits + `'\n'` per `driver` call.
const REC: usize = 9;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(f32);

struct Libs {
    c_driver: DriverFn,
    r_driver: DriverFn,
    c_lib: Library,
    r_lib: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    // The test executable lives in target/<profile>/deps/, so the cdylib that
    // cargo built for this same profile is one directory up.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            let candidate = profile_dir.join("libdriver.so");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    for profile in ["debug", "release"] {
        let candidate = manifest_dir().join("target").join(profile).join("libdriver.so");
        if candidate.exists() {
            return candidate;
        }
    }
    panic!("could not locate the Rust libdriver.so; run `cargo build` first");
}

/// Newest modification time found anywhere under `dir`.
fn newest_mtime(dir: &std::path::Path) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                if newest.is_none_or(|n| m > n) {
                    newest = Some(m);
                }
            }
        }
    }
    newest
}

/// Fail loudly if `so` is older than the sources it is built from.
#[track_caller]
fn assert_fresh(so: &std::path::Path, src_dir: &std::path::Path, rebuild_cmd: &str) {
    let Ok(so_mtime) = std::fs::metadata(so).and_then(|m| m.modified()) else { return };
    let Some(src_mtime) = newest_mtime(src_dir) else { return };
    assert!(
        so_mtime >= src_mtime,
        "{} is STALE (older than sources in {}). `cargo test` does not rebuild \
         a cdylib, so these results would be meaningless. Re-run `{rebuild_cmd}` \
         first (run_all_features.sh does this automatically).",
        so.display(),
        src_dir.display()
    );
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library missing at {}; build it with cmake first",
            c_path.display()
        );
        assert!(
            r_path.exists(),
            "Rust shared library missing at {}; run cargo build first",
            r_path.display()
        );
        // `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` library,
        // so without this guard the tests would happily validate a stale
        // `libdriver.so` and report a false pass after a source change.
        assert_fresh(&r_path, &manifest_dir().join("src"), "cargo build");
        assert_fresh(&c_path, &manifest_dir().join("c_src/src"), "cmake --build");
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let r_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", r_path.display()));
            let c_driver = *c_lib
                .get::<DriverFn>(b"driver\0")
                .expect("C .so does not export `driver`");
            let r_driver = *r_lib
                .get::<DriverFn>(b"driver\0")
                .expect("Rust .so does not export `driver` (missing #[no_mangle] wrapper?)");
            Libs { c_driver, r_driver, c_lib, r_lib }
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
//
// fd 1 is process-wide, so captures must be serialized against each other.
// ---------------------------------------------------------------------------

fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let m = LOCK.get_or_init(|| Mutex::new(()));
    // Tolerate poisoning: a panic in one test must not disable every later one.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn temp_path() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-diff-{}-{}.out",
        std::process::id(),
        n
    ))
}

/// Redirect fd 1 to a temp file, run `f`, restore, and return everything that
/// was written (by either `.so`, through libc's buffered `stdout`).
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();
    let path = temp_path();
    let file = File::create(&path).expect("create capture file");

    unsafe {
        // Flush anything already pending so it lands on the real stdout.
        fflush(null_mut());
    }
    let saved = unsafe { dup(STDOUT_FD) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(
        unsafe { dup2(file.as_raw_fd(), STDOUT_FD) } >= 0,
        "dup2 onto stdout failed"
    );

    f();

    unsafe {
        // Force libc to drain the stdout buffer into the temp file *before* the
        // redirection is undone.
        fflush(null_mut());
        assert!(dup2(saved, STDOUT_FD) >= 0, "restoring stdout failed");
        close(saved);
    }
    drop(file);

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// Safety net for the capture mechanism itself, applied ONLY to the C reference
/// stream.
///
/// fd 1 is process-wide, so if the tests were ever run concurrently, libtest's
/// own `test <name> ... ok` progress lines would be written into the capture and
/// silently corrupt every comparison. The C library — the ground truth — can only
/// ever emit `[0-9a-f]` and `'\n'` in 9-byte records, so anything else in ITS
/// stream means the capture was polluted rather than that a library misbehaved.
/// `.cargo/config.toml` forces `RUST_TEST_THREADS=1` to prevent this; this check
/// makes a mis-invocation fail loudly instead of producing bogus results.
///
/// The Rust stream is deliberately NOT filtered this way: whatever it emits is
/// compared byte-for-byte against the C stream, so a Rust divergence is reported
/// as a divergence (with the offending input) instead of being misattributed to
/// harness pollution.
#[track_caller]
fn assert_reference_stream_clean(out: &[u8], calls: usize) {
    if let Some(pos) = out
        .iter()
        .position(|c| !(c.is_ascii_digit() || (b'a'..=b'f').contains(c) || *c == b'\n'))
    {
        panic!(
            "the C reference stdout capture was polluted at offset {pos} \
             (byte {:#04x}); the C library cannot emit that byte, so something \
             else wrote to fd 1. Run the tests single-threaded \
             (RUST_TEST_THREADS=1 / `-- --test-threads=1`). Captured prefix: {:?}",
            out[pos],
            String::from_utf8_lossy(&out[..out.len().min(200)])
        );
    }
    assert_eq!(
        out.len(),
        calls * REC,
        "the C reference capture is {} bytes for {calls} calls, expected {} \
         (9 per call); the capture was polluted -- run the tests single-threaded",
        out.len(),
        calls * REC
    );
}

fn capture_c(bits: &[u32]) -> Vec<u8> {
    let f = libs().c_driver;
    let out = capture(|| {
        for &b in bits {
            unsafe { f(f32::from_bits(b)) };
        }
    });
    assert_reference_stream_clean(&out, bits.len());
    out
}

fn capture_rust(bits: &[u32]) -> Vec<u8> {
    let f = libs().r_driver;
    capture(|| {
        for &b in bits {
            unsafe { f(f32::from_bits(b)) };
        }
    })
}

// ---------------------------------------------------------------------------
// comparison
// ---------------------------------------------------------------------------

fn model_record(bits: u32) -> String {
    let b = f32::from_bits(bits).to_bits().to_ne_bytes();
    format!("{:02x}{:02x}{:02x}{:02x}\n", b[0], b[1], b[2], b[3])
}

fn show(rec: &[u8]) -> String {
    String::from_utf8_lossy(rec).escape_debug().to_string()
}

/// The core assertion used by every Phase B / Phase C row.
///
/// * captures BOTH `.so`s' output for the same input list,
/// * asserts the streams are byte-identical,
/// * asserts the 9-byte framing invariant (`ERRORS.md` row 11 / `CONFIGS.md`
///   row 22) and the lowercase-hex character set,
/// * cross-checks the C output against an independent model of
///   `printf("%02x")` over the argument's object representation, which would
///   catch a harness bug (e.g. a value mangled on its way through the FFI call)
///   masking a real divergence.
#[track_caller]
fn assert_same(row: &str, bits: &[u32]) {
    assert!(!bits.is_empty(), "{row}: empty input list");
    let c_out = capture_c(bits);
    let r_out = capture_rust(bits);

    // Report the first byte-level divergence together with the input that
    // produced it, before any length assertion, so a Rust-side format bug is
    // attributed to the value it broke on rather than to a stream-length delta.
    if c_out != r_out {
        let off = c_out
            .iter()
            .zip(r_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c_out.len().min(r_out.len()));
        let i = off / REC;
        let culprit = bits.get(i).copied();
        panic!(
            "{row}: C and Rust streams diverge at byte offset {off} \
             (record {i}{}).\n  C   : {:?}\n  Rust: {:?}\n  \
             C stream len={} Rust stream len={}",
            culprit.map_or(String::new(), |b| format!(
                ", input bits {b:#010x} = f32 {:?}",
                f32::from_bits(b)
            )),
            show(&c_out[i * REC..c_out.len().min((i + 3) * REC)]),
            show(&r_out[(i * REC).min(r_out.len())..r_out.len().min((i + 3) * REC)]),
            c_out.len(),
            r_out.len()
        );
    }
    assert_eq!(
        r_out.len(),
        bits.len() * REC,
        "{row}: expected {} bytes for {} calls (9 per call)",
        bits.len() * REC,
        bits.len()
    );

    for (i, &b) in bits.iter().enumerate() {
        let c_rec = &c_out[i * REC..(i + 1) * REC];
        let r_rec = &r_out[i * REC..(i + 1) * REC];
        assert_eq!(
            c_rec,
            r_rec,
            "{row}: divergence at index {i} for input bits {b:#010x} \
             (f32 {:?}): C={:?} Rust={:?}",
            f32::from_bits(b),
            show(c_rec),
            show(r_rec)
        );
        // framing / charset invariant
        assert_eq!(c_rec[8], b'\n', "{row}: record {i} not newline-terminated");
        assert!(
            c_rec[..8].iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c)),
            "{row}: record {i} is not 8 lowercase hex digits: {:?}",
            show(c_rec)
        );
        assert_eq!(
            c_rec,
            model_record(b).as_bytes(),
            "{row}: harness model disagrees with C at index {i} for {b:#010x} \
             (argument mangled in transit?)"
        );
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) -- fixed seed for reproducibility
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_C0FF_EE00_0001;

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Rng(SEED)
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
    /// uniform in `0..n`
    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// uniform in `lo..=hi`
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }
}

fn bits_of(sign: u32, exp: u32, mant: u32) -> u32 {
    (sign << 31) | (exp << 23) | mant
}

// ===========================================================================
// PHASE B -- valid-path differential tests (one per CONFIGS.md row)
// ===========================================================================

#[test]
fn cfg_01_positive_zero() {
    assert_same("cfg_01", &[0x0000_0000]);
}

#[test]
fn cfg_02_negative_zero() {
    assert_same("cfg_02", &[0x8000_0000]);
}

#[test]
fn cfg_03_positive_normals_random() {
    let mut rng = Rng::new();
    let bits: Vec<u32> = (0..10_000)
        .map(|_| bits_of(0, rng.range(1, 254), rng.below(0x80_0000)))
        .collect();
    assert_same("cfg_03", &bits);
}

#[test]
fn cfg_04_negative_normals_random() {
    let mut rng = Rng::new();
    let bits: Vec<u32> = (0..10_000)
        .map(|_| bits_of(1, rng.range(1, 254), rng.below(0x80_0000)))
        .collect();
    assert_same("cfg_04", &bits);
}

#[test]
fn cfg_05_positive_subnormals_random() {
    let mut rng = Rng::new();
    let mut bits: Vec<u32> = vec![
        bits_of(0, 0, 1),         // FLT_TRUE_MIN, smallest subnormal
        bits_of(0, 0, 0x7f_ffff), // largest subnormal
    ];
    bits.extend((0..10_000).map(|_| bits_of(0, 0, rng.range(1, 0x7f_ffff))));
    assert_same("cfg_05", &bits);
}

#[test]
fn cfg_06_negative_subnormals_random() {
    let mut rng = Rng::new();
    let mut bits: Vec<u32> = vec![bits_of(1, 0, 1), bits_of(1, 0, 0x7f_ffff)];
    bits.extend((0..10_000).map(|_| bits_of(1, 0, rng.range(1, 0x7f_ffff))));
    assert_same("cfg_06", &bits);
}

#[test]
fn cfg_07_integral_values_exhaustive() {
    let bits: Vec<u32> = (-2048i32..=2048).map(|i| (i as f32).to_bits()).collect();
    assert_same("cfg_07", &bits);
}

#[test]
fn cfg_08_powers_of_two_exhaustive() {
    let mut bits = Vec::new();
    for k in -149i32..=127 {
        let v = (2.0f64).powi(k) as f32;
        bits.push(v.to_bits());
        bits.push((-v).to_bits());
    }
    assert_same("cfg_08", &bits);
}

#[test]
fn cfg_09_named_float_limits() {
    let vals: [f32; 14] = [
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
        -f32::EPSILON,
        f32::from_bits(1), // FLT_TRUE_MIN
        f32::from_bits(0x8000_0001),
        1.0,
        -1.0,
        0.0,
        -0.0,
        f32::from_bits(0x007f_ffff), // largest subnormal
        f32::from_bits(0x0080_0000), // smallest normal
    ];
    let bits: Vec<u32> = vals.iter().map(|v| v.to_bits()).collect();
    assert_same("cfg_09", &bits);
}

#[test]
fn cfg_10_infinities() {
    assert_same("cfg_10", &[0x7f80_0000, 0xff80_0000]);
}

#[test]
fn cfg_11_nan_payload_matrix() {
    let mut rng = Rng::new();
    let mut bits = Vec::new();
    for sign in [0u32, 1] {
        for mant in [1u32, 2, 0x40_0000, 0x3f_ffff, 0x7f_ffff, 0x20_0000] {
            bits.push(bits_of(sign, 0xff, mant));
        }
        for _ in 0..4_000 {
            bits.push(bits_of(sign, 0xff, rng.range(1, 0x7f_ffff)));
        }
    }
    assert_same("cfg_11", &bits);
}

#[test]
fn cfg_12_all_bytes_below_0x10() {
    let mut rng = Rng::new();
    let mut bits: Vec<u32> = vec![0x0000_0001, 0x0101_0101, 0x0f0f_0f0f, 0x0f00_0f00, 0x0000_000f];
    bits.extend((0..2_000).map(|_| {
        let b = |r: &mut Rng| r.below(0x10);
        b(&mut rng) | (b(&mut rng) << 8) | (b(&mut rng) << 16) | (b(&mut rng) << 24)
    }));
    assert_same("cfg_12", &bits);
}

#[test]
fn cfg_13_all_bytes_at_or_above_0x80() {
    let mut rng = Rng::new();
    let mut bits: Vec<u32> = vec![0xffff_ffff, 0x8080_8080, 0xf0f0_f0f0, 0x80ff_80ff];
    bits.extend((0..2_000).map(|_| {
        let b = |r: &mut Rng| 0x80 + r.below(0x80);
        b(&mut rng) | (b(&mut rng) << 8) | (b(&mut rng) << 16) | (b(&mut rng) << 24)
    }));
    assert_same("cfg_13", &bits);
}

#[test]
fn cfg_14_per_byte_position_sweep() {
    let mut bits = Vec::new();
    for pos in 0..4u32 {
        for byte in 0..=0xffu32 {
            bits.push(byte << (8 * pos)); // other bytes 0x00
            let mask = 0xffff_ffffu32 & !(0xffu32 << (8 * pos));
            bits.push(mask | (byte << (8 * pos))); // other bytes 0xff
        }
    }
    assert_same("cfg_14", &bits);
}

#[test]
fn cfg_15_exhaustive_low_16_bits() {
    let bits: Vec<u32> = (0u32..=0xffff).map(|lo| 0x3f80_0000 | lo).collect();
    assert_same("cfg_15", &bits);
}

#[test]
fn cfg_16_exhaustive_high_16_bits() {
    let bits: Vec<u32> = (0u32..=0xffff).map(|hi| hi << 16).collect();
    assert_same("cfg_16", &bits);
}

#[test]
fn cfg_17_full_domain_random() {
    let mut rng = Rng::new();
    let bits: Vec<u32> = (0..200_000).map(|_| rng.next_u32()).collect();
    assert_same("cfg_17", &bits);
}

#[test]
fn cfg_18_decimal_literals() {
    let vals: [f32; 20] = [
        0.1, -0.1, 0.5, -0.5, 1.5, -1.5, 3.14159, -3.14159, 2.71828, 1e-30, -1e-30, 1e30,
        -1e30, 1e-45, 3.4e38, -3.4e38, 100.0, 255.0, 65535.0, 16_777_216.0,
    ];
    let bits: Vec<u32> = vals.iter().map(|v| v.to_bits()).collect();
    assert_same("cfg_18", &bits);
}

#[test]
fn cfg_19_repeated_identical_calls() {
    let bits = vec![0x4048_f5c3u32; 1_000];
    assert_same("cfg_19", &bits);
    // Every record must equal the first: no per-call state.
    let out = capture_rust(&bits);
    let first = &out[..REC];
    for i in 1..bits.len() {
        assert_eq!(&out[i * REC..(i + 1) * REC], first, "cfg_19: record {i} differs");
    }
}

#[test]
fn cfg_20_interleaved_c_and_rust() {
    let mut rng = Rng::new();
    let bits: Vec<u32> = (0..2_000)
        .map(|i| match i % 5 {
            0 => rng.next_u32(),
            1 => bits_of(0, 0xff, rng.range(1, 0x7f_ffff)), // NaN
            2 => bits_of(1, 0, rng.range(1, 0x7f_ffff)),    // -subnormal
            3 => 0x7f80_0000,                               // +Inf
            _ => bits_of(1, rng.range(1, 254), rng.below(0x80_0000)),
        })
        .collect();

    let (cf, rf) = (libs().c_driver, libs().r_driver);
    // C,R,C,R,... into the one shared glibc stdout buffer.
    let out = capture(|| {
        for &b in &bits {
            unsafe {
                cf(f32::from_bits(b));
                rf(f32::from_bits(b));
            }
        }
    });
    assert_eq!(out.len(), bits.len() * 2 * REC, "cfg_20: unexpected stream length");
    for (i, &b) in bits.iter().enumerate() {
        let c_rec = &out[(2 * i) * REC..(2 * i + 1) * REC];
        let r_rec = &out[(2 * i + 1) * REC..(2 * i + 2) * REC];
        assert_eq!(
            c_rec,
            r_rec,
            "cfg_20: interleaved divergence at {i} for {b:#010x}: C={:?} Rust={:?}",
            show(c_rec),
            show(r_rec)
        );
    }
}

#[test]
fn cfg_21_stdout_buffer_boundary_crossing() {
    // >3000 calls * 9 bytes = >27 KiB, crossing libc's 4 KiB stdout buffer many
    // times so that flush boundaries fall in the middle of records.
    let mut rng = Rng::new();
    let bits: Vec<u32> = (0..3_500).map(|_| rng.next_u32()).collect();
    assert_same("cfg_21", &bits);
}

#[test]
fn cfg_22_output_framing_invariant() {
    // A mixed sample drawn from every class above; assert_same already enforces
    // the 9-byte / lowercase-hex / little-endian framing invariant per record.
    let mut rng = Rng::new();
    let mut bits: Vec<u32> = vec![
        0x0000_0000,
        0x8000_0000,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0xffc0_0000,
        0x0000_0001,
        0xffff_ffff,
        0x3f80_0000,
    ];
    bits.extend((0..5_000).map(|_| rng.next_u32()));
    assert_same("cfg_22", &bits);

    let out = capture_c(&bits);
    for (i, chunk) in out.chunks(REC).enumerate() {
        assert_eq!(chunk.len(), REC, "cfg_22: record {i} truncated");
        assert_eq!(chunk[8], b'\n', "cfg_22: record {i} missing newline");
        for &c in &chunk[..8] {
            assert!(
                c.is_ascii_digit() || (b'a'..=b'f').contains(&c),
                "cfg_22: record {i} has non-lowercase-hex byte {c:#04x}"
            );
        }
    }
}

// ===========================================================================
// PHASE C -- error-path differential tests (one per ERRORS.md row)
// ===========================================================================

/// ERRORS.md row 1: there is no error path at all -- `driver` is total.
#[test]
fn err_01_no_error_path_total_function() {
    let bits = [0x3f80_0000u32, 0x4048_f5c3, 0xc048_f5c3];
    // Both libraries must simply return (no abort/trap) and print 9 bytes each.
    assert_same("err_01", &bits);

    let c_out = capture_c(&bits);
    let r_out = capture_rust(&bits);
    assert_eq!(c_out.len(), bits.len() * REC, "err_01: C output length");
    assert_eq!(r_out, c_out, "err_01: streams differ");
}

/// ERRORS.md row 2 / 3: quiet NaN, both signs, must not be canonicalized.
#[test]
fn err_02_quiet_nan_positive() {
    assert_same("err_02", &[0x7fc0_0000]);
    let out = capture_rust(&[0x7fc0_0000]);
    assert_eq!(&out, b"0000c07f\n", "err_02: unexpected qNaN encoding");
}

#[test]
fn err_03_quiet_nan_negative() {
    assert_same("err_03", &[0xffc0_0000]);
    let out = capture_rust(&[0xffc0_0000]);
    assert_eq!(&out, b"0000c0ff\n", "err_03: unexpected -qNaN encoding");
}

/// ERRORS.md row 4: signalling NaNs (mantissa MSB clear) must pass through
/// bit-exact -- neither `memcpy` nor `f32::to_bits` may quiet them.
#[test]
fn err_04_signalling_nan() {
    let mut bits = vec![
        0x7f80_0001u32,
        0xff80_0001,
        0x7fbf_ffff, // largest positive sNaN payload
        0xffbf_ffff,
        0x7f80_0002,
        0xff80_4000,
    ];
    let mut rng = Rng::new();
    // Random sNaN payloads: exponent all ones, mantissa MSB clear, mantissa != 0.
    bits.extend((0..2_000).map(|_| {
        let sign = rng.below(2);
        bits_of(sign, 0xff, rng.range(1, 0x3f_ffff))
    }));
    assert_same("err_04", &bits);
}

/// ERRORS.md row 5: every distinct NaN payload class, both signs.
#[test]
fn err_05_nan_all_payloads() {
    let mut bits = Vec::new();
    for sign in [0u32, 1] {
        for mant in [
            0x00_0001, 0x00_0002, 0x00_0004, 0x0f_ffff, 0x3f_ffff, // sNaN payloads
            0x40_0000, 0x40_0001, 0x5f_ffff, 0x7f_ffff, // qNaN payloads
        ] {
            bits.push(bits_of(sign, 0xff, mant));
        }
        // Sweep one bit at a time through the mantissa.
        for k in 0..23 {
            bits.push(bits_of(sign, 0xff, 1u32 << k));
        }
    }
    assert_same("err_05", &bits);
}

/// ERRORS.md row 6: +/-Inf, the exponent-overflow boundary.
#[test]
fn err_06_infinities() {
    assert_same("err_06", &[0x7f80_0000, 0xff80_0000]);
    assert_eq!(&capture_rust(&[0x7f80_0000]), b"0000807f\n", "err_06: +Inf");
    assert_eq!(&capture_rust(&[0xff80_0000]), b"000080ff\n", "err_06: -Inf");
}

/// ERRORS.md row 7: -0.0 must not collapse to +0.0.
#[test]
fn err_07_negative_zero() {
    assert_same("err_07", &[0x8000_0000, 0x0000_0000]);
    let neg = capture_rust(&[0x8000_0000]);
    let pos = capture_rust(&[0x0000_0000]);
    assert_eq!(&neg, b"00000080\n", "err_07: -0.0 encoding");
    assert_eq!(&pos, b"00000000\n", "err_07: +0.0 encoding");
    assert_ne!(neg, pos, "err_07: -0.0 collapsed to +0.0");
}

/// ERRORS.md row 8: the `static` helper must be un-exported in BOTH libraries,
/// i.e. `dlsym` must reject the lookup identically.
#[test]
fn err_08_static_helper_not_exported() {
    let l = libs();
    // Every spelling the internal helper could plausibly leak under.
    let names: [&[u8]; 3] = [b"print_hex\0", b"_print_hex\0", b"driver::print_hex\0"];
    for name in names {
        let c_found = unsafe { l.c_lib.get::<*const c_void>(name) }.is_ok();
        let r_found = unsafe { l.r_lib.get::<*const c_void>(name) }.is_ok();
        assert!(
            !c_found,
            "err_08: C .so unexpectedly exports {:?}",
            String::from_utf8_lossy(name)
        );
        assert_eq!(
            c_found, r_found,
            "err_08: {:?} export mismatch (C={c_found}, Rust={r_found}); \
             the Rust translation must keep the `static` helper private",
            String::from_utf8_lossy(name)
        );
    }
}

/// ERRORS.md row 9: unknown-symbol lookups are rejected the same way by both.
#[test]
fn err_09_unknown_symbols_rejected() {
    let l = libs();
    let names: [&[u8]; 8] = [
        b"driver_init\0",
        b"driver_free\0",
        b"driver2\0",
        b"print_hex_impl\0",
        b"Driver\0",
        b"driver_\0",
        b"_driver\0",
        b"rust_driver\0",
    ];
    for name in names {
        let c_found = unsafe { l.c_lib.get::<*const c_void>(name) }.is_ok();
        let r_found = unsafe { l.r_lib.get::<*const c_void>(name) }.is_ok();
        assert!(
            !c_found && !r_found,
            "err_09: symbol {:?} resolved (C={c_found}, Rust={r_found}) but must not exist",
            String::from_utf8_lossy(name)
        );
    }
    // ...and the one real symbol resolves in both.
    assert!(unsafe { l.c_lib.get::<DriverFn>(b"driver\0") }.is_ok());
    assert!(unsafe { l.r_lib.get::<DriverFn>(b"driver\0") }.is_ok());
}

/// ERRORS.md row 10: one step past each float-class boundary.
#[test]
fn err_10_one_past_class_boundaries() {
    let bits: Vec<u32> = vec![
        0x0000_0000, // +0
        0x0000_0001, // one past +0 (smallest subnormal)
        0x8000_0000, // -0
        0x8000_0001, // one past -0
        0x007f_ffff, // largest subnormal
        0x0080_0000, // one past: smallest normal (FLT_MIN)
        0x807f_ffff,
        0x8080_0000,
        0x7f7f_ffff, // FLT_MAX
        0x7f80_0000, // one past FLT_MAX: +Inf
        0xff7f_ffff, // -FLT_MAX
        0xff80_0000, // one past: -Inf
        0x7f80_0001, // one past +Inf: sNaN
        0xff80_0001, // one past -Inf: -sNaN
        0x7fff_ffff, // last positive bit pattern
        0xffff_ffff, // last bit pattern of the whole domain
    ];
    assert_same("err_10", &bits);
}

/// ERRORS.md row 11: `print_hex`'s `len` is hard-wired to `sizeof(float)`, so
/// the output is always exactly 9 bytes -- never 0, never 5+ bytes of hex.
#[test]
fn err_11_output_length_invariant() {
    let mut rng = Rng::new();
    let mut bits: Vec<u32> = vec![0, 0xffff_ffff, 0x7fc0_0000];
    bits.extend((0..5_000).map(|_| rng.next_u32()));

    let c_out = capture_c(&bits);
    let r_out = capture_rust(&bits);
    assert_eq!(c_out.len(), bits.len() * REC, "err_11: C length");
    assert_eq!(r_out.len(), bits.len() * REC, "err_11: Rust length");
    assert_eq!(r_out, c_out, "err_11: streams differ");
    // Exactly one newline per call => exactly 4 bytes dumped per call.
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        bits.len(),
        "err_11: newline count != call count"
    );
    assert_eq!(
        r_out.iter().filter(|&&b| b == b'\n').count(),
        bits.len(),
        "err_11: Rust newline count != call count"
    );
}

/// ERRORS.md row 12: `%02x` zero-padding for bytes below 0x10, in every position.
#[test]
fn err_12_zero_padding_low_bytes() {
    let bits: Vec<u32> = vec![
        0x0000_0001, 0x0000_0100, 0x0001_0000, 0x0100_0000, 0x0101_0101, 0x0f0f_0f0f,
        0x0000_000f, 0x0900_0000, 0x0000_0000,
    ];
    assert_same("err_12", &bits);
    // Spot-check the exact expected framing: 8 digits, leading zeros present.
    assert_eq!(&capture_rust(&[0x0000_0001]), b"01000000\n", "err_12: padding");
    assert_eq!(&capture_rust(&[0x0100_0000]), b"00000001\n", "err_12: padding");
}

/// ERRORS.md row 13: `unsigned char` -> `int` promotion must zero-extend, so a
/// byte >= 0x80 prints as two digits, never as a sign-extended `ffffffff`.
#[test]
fn err_13_no_sign_extension_high_bytes() {
    let bits: Vec<u32> = vec![
        0xffff_ffff, 0x8080_8080, 0x0000_0080, 0x0000_8000, 0x0080_0000, 0x8000_0000,
        0xf0f0_f0f0, 0x00ff_00ff,
    ];
    assert_same("err_13", &bits);
    assert_eq!(&capture_rust(&[0xffff_ffff]), b"ffffffff\n", "err_13: all-ones");
    assert_eq!(&capture_rust(&[0x0000_0080]), b"80000000\n", "err_13: single high byte");
    assert_eq!(&capture_rust(&[0x8080_8080]), b"80808080\n", "err_13: all high bytes");
}

/// ERRORS.md row 14: repeated and interleaved invocation -- no hidden one-shot
/// init, no per-call state, on the shared glibc stdout.
#[test]
fn err_14_repeated_and_interleaved_calls() {
    let (cf, rf) = (libs().c_driver, libs().r_driver);
    let value = f32::from_bits(0x7f80_0001); // sNaN, the least forgiving input

    // R,R,C,C,R,C ... orderings, then confirm every record is identical.
    let out = capture(|| unsafe {
        rf(value);
        rf(value);
        cf(value);
        cf(value);
        rf(value);
        cf(value);
        for _ in 0..500 {
            cf(value);
            rf(value);
        }
    });
    let n = 6 + 1_000;
    assert_eq!(out.len(), n * REC, "err_14: unexpected stream length");
    let first = &out[..REC];
    assert_eq!(first, b"0100807f\n", "err_14: unexpected sNaN encoding");
    for i in 1..n {
        assert_eq!(
            &out[i * REC..(i + 1) * REC],
            first,
            "err_14: record {i} differs from the first -- hidden state?"
        );
    }
}

// ===========================================================================
// Harness self-checks -- these must fail if the capture mechanism is broken,
// otherwise every row above could pass vacuously.
// ===========================================================================

/// The capture must return exactly what the libraries wrote and nothing else.
#[test]
fn harness_00_capture_mechanism_is_clean() {
    // Nothing emitted => empty capture. If libtest (or anything else) were
    // writing to fd 1 concurrently, this would not be empty.
    let empty = capture(|| {});
    assert!(
        empty.is_empty(),
        "capture picked up {} stray byte(s) when nothing was called: {:?} -- \
         the tests must run single-threaded",
        empty.len(),
        String::from_utf8_lossy(&empty)
    );

    // One call => exactly one 9-byte record, from each library.
    let c = capture_c(&[0x3f80_0000]);
    let r = capture_rust(&[0x3f80_0000]);
    assert_eq!(c.len(), REC, "one C call must emit exactly 9 bytes");
    assert_eq!(r.len(), REC, "one Rust call must emit exactly 9 bytes");
    assert_eq!(c, r);
    assert_eq!(&c, b"0000803f\n", "unexpected encoding of 1.0f");
}

/// The two `.so`s really are two distinct objects, and `driver` really is
/// resolved out of each of them (i.e. we are not accidentally comparing one
/// library against itself, which would make every row pass vacuously).
#[test]
fn harness_01_two_distinct_libraries_loaded() {
    let l = libs();
    let c_addr = l.c_driver as usize;
    let r_addr = l.r_driver as usize;
    assert_ne!(
        c_addr, r_addr,
        "the C and Rust `driver` symbols resolved to the SAME address -- the \
         differential comparison would be vacuous"
    );
    assert_ne!(
        c_so_path().canonicalize().unwrap(),
        rust_so_path().canonicalize().unwrap(),
        "the C and Rust .so paths are the same file"
    );
}

/// ERRORS.md row 15: ABI-level "out of range" input.
///
/// This library declares no `enum`, so the analogue of passing an int with no
/// valid enum variant across the FFI boundary is passing a wider value than the
/// declared parameter: on x86-64 SysV a `float` argument occupies only the low
/// 32 bits of `xmm0` and the upper 96 bits are unspecified, so a real C caller
/// can leave arbitrary junk there. Both implementations must read only the low
/// 32 bits. Calling the exported symbol through an `extern "C" fn(f64)` pointer
/// puts caller-chosen junk in the upper lane.
#[test]
fn err_15_abi_upper_lane_garbage_ignored() {
    type F64Fn = unsafe extern "C" fn(f64);
    let l = libs();
    let cf = unsafe { *l.c_lib.get::<F64Fn>(b"driver\0").unwrap() };
    let rf = unsafe { *l.r_lib.get::<F64Fn>(b"driver\0").unwrap() };

    let mut rng = Rng::new();
    let mut cases: Vec<(u32, u32)> = Vec::new();
    for hi in [0x0000_0000u32, 0xffff_ffff, 0xdead_beef, 0x7ff8_0000, 0x8000_0000] {
        for lo in [
            0x0000_0000u32,
            0x3f80_0000,
            0x7f80_0001, // sNaN
            0x7fc0_0000, // qNaN
            0xffff_ffff,
            0x8000_0000,
            0x007f_ffff,
        ] {
            cases.push((hi, lo));
        }
    }
    for _ in 0..200 {
        cases.push((rng.next_u32(), rng.next_u32()));
    }

    for (hi, lo) in cases {
        let d = f64::from_bits(((hi as u64) << 32) | lo as u64);
        let c = capture(|| unsafe { cf(d) });
        let r = capture(|| unsafe { rf(d) });
        assert_eq!(
            c, r,
            "err_15: divergence with upper-lane junk {hi:#010x}, low word {lo:#010x}: \
             C={:?} Rust={:?}",
            show(&c),
            show(&r)
        );
        // Both must have ignored the upper lane entirely.
        assert_eq!(
            c,
            model_record(lo).as_bytes(),
            "err_15: upper-lane junk {hi:#010x} leaked into the output for low word {lo:#010x}"
        );
    }
}

/// ERRORS.md row 16: failure of the underlying output stream.
///
/// The C code ignores `printf`'s and `putchar`'s return values, so a write error
/// cannot change its control flow -- but the Rust translation must ignore them
/// too (e.g. it must not panic, unwrap, or short-circuit the loop). Redirecting
/// fd 1 to `/dev/full` makes every write fail with `ENOSPC`; both libraries must
/// survive it and leave the stream in the same error state.
#[test]
fn err_16_output_stream_write_failure() {
    unsafe extern "C" {
        static mut stdout: *mut c_void;
        fn ferror(stream: *mut c_void) -> c_int;
        fn clearerr(stream: *mut c_void);
        fn open(path: *const std::ffi::c_char, flags: c_int) -> c_int;
    }
    const O_WRONLY: c_int = 1;

    let _guard = capture_lock();
    let l = libs();

    let full = unsafe { open(c"/dev/full".as_ptr(), O_WRONLY) };
    if full < 0 {
        eprintln!("err_16: /dev/full unavailable, skipping");
        return;
    }

    unsafe { fflush(null_mut()) };
    let saved = unsafe { dup(STDOUT_FD) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(full, STDOUT_FD) } >= 0, "dup2(/dev/full) failed");

    // Each library writes into a stream whose flush is guaranteed to fail.
    let probe = |f: DriverFn| unsafe {
        f(f32::from_bits(0x3f80_0000));
        f(f32::from_bits(0x7f80_0001));
        let flush_rc = fflush(stdout);
        let err = ferror(stdout);
        clearerr(stdout);
        (flush_rc, err != 0)
    };
    let c_res = probe(l.c_driver);
    let r_res = probe(l.r_driver);

    unsafe {
        clearerr(stdout);
        assert!(dup2(saved, STDOUT_FD) >= 0, "restoring stdout failed");
        close(saved);
        close(full);
        clearerr(stdout);
    }

    assert_eq!(
        c_res, r_res,
        "err_16: write-failure behaviour differs: C (fflush rc, ferror)={c_res:?}, \
         Rust={r_res:?}"
    );
    assert_eq!(c_res.0, -1, "err_16: expected the flush to /dev/full to fail");
    assert!(c_res.1, "err_16: expected the stream error flag to be set");
}
