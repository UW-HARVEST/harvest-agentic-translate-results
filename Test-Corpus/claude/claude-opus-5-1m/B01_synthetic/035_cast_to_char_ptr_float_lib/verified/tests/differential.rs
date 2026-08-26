//! Differential tests for the C-to-Rust translation of `c_src/` (`driver`).
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called through their exported `driver` symbol. The Rust side is NEVER called
//! directly as a Rust function — it is always reached through
//! `dlopen("libdriver.so") + dlsym("driver")`, exactly as an external C consumer
//! would, so the `#[no_mangle] extern "C"` export wrapper is under test too.
//!
//! `driver` returns `void` and communicates only by writing to the libc `stdout`
//! stream, so "compare the outputs" means: redirect file descriptor 1, run the
//! call(s), flush, and compare the captured bytes byte-for-byte.
//!
//! Phase B rows live in `CONFIGS.md`; Phase C rows live in `ERRORS.md`.

use libc::{c_int, c_void, FILE};
use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Loading both shared objects
// ---------------------------------------------------------------------------

/// `void driver(float x);`
type DriverFn = unsafe extern "C" fn(f32);

struct Api {
    c: DriverFn,
    rust: DriverFn,
    c_path: PathBuf,
    rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src").join("build").join("libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built at {}\n\
         Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    // The test executable lives at target/<profile>/deps/<test-exe>; the cdylib
    // is produced at target/<profile>/libdriver.so (hard-linked into deps/).
    // Whichever profile/feature combination cargo was invoked with, this walks
    // up from the running test binary, so the .so under test always matches the
    // configuration the test was built for.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("exe parent").to_path_buf();
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(profile) = deps.parent() {
        candidates.push(profile.join("libdriver.so"));
    }
    candidates.push(deps.join("libdriver.so"));
    if let Some(profile) = deps.parent() {
        if let Some(target) = profile.parent() {
            candidates.push(target.join("debug").join("libdriver.so"));
        }
    }
    for c in &candidates {
        if c.exists() {
            assert_fresh(c);
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib `libdriver.so` not found (looked at {candidates:?}) — \
         run `cargo build` for the same profile/features first"
    );
}

/// `cargo test` does NOT rebuild a `cdylib`-only lib target, so without this
/// guard the suite could silently validate a STALE `.so` and report success for
/// code that is no longer what is in `src/`. Refuse to run in that case.
fn assert_fresh(so: &std::path::Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat the Rust .so");

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut consider = |p: PathBuf| {
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if newest.as_ref().map_or(true, |(_, n)| t > *n) {
                newest = Some((p, t));
            }
        }
    };
    consider(manifest_dir().join("Cargo.toml"));
    let mut stack = vec![manifest_dir().join("src")];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    consider(p);
                }
            }
        }
    }

    if let Some((path, t)) = newest {
        assert!(
            t <= so_mtime,
            "STALE ARTIFACT: {} is newer than {}.\n\
             `cargo test` does not rebuild a cdylib-only lib target, so the suite \
             would be testing an out-of-date shared object.\n\
             Run `cargo build` (same profile and features) first, or use \
             ./verify_all.sh which does it for you.",
            path.display(),
            so.display()
        );
    }
}

fn api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));

            let c = {
                let s: Symbol<DriverFn> = c_lib
                    .get(b"driver\0")
                    .expect("dlsym(\"driver\") in the C .so");
                *s
            };
            let rust = {
                let s: Symbol<DriverFn> = rust_lib
                    .get(b"driver\0")
                    .expect("dlsym(\"driver\") in the Rust .so");
                *s
            };

            assert_ne!(
                c as usize, rust as usize,
                "both dlopen'ed `driver` symbols resolved to the SAME address: the \
                 dynamic loader deduplicated the two libraries and every comparison \
                 below would be vacuous"
            );

            // Keep both libraries mapped for the whole process lifetime.
            std::mem::forget(c_lib);
            std::mem::forget(rust_lib);

            Api { c, rust, c_path, rust_path }
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture plumbing (fd 1 is process-global => everything is serialised)
// ---------------------------------------------------------------------------

static IO_LOCK: Mutex<()> = Mutex::new(());
static SEQ: AtomicU64 = AtomicU64::new(0);

fn io_lock() -> MutexGuard<'static, ()> {
    IO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn tmp_file(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "driver_diff_{}_{}_{}.bin",
        std::process::id(),
        tag,
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    p
}

// glibc's `stdout` global. The `libc` crate does not expose it for this target,
// so bind it directly: this is the very same `FILE *` that both libraries write
// through, which is what makes the byte-level comparison meaningful.
extern "C" {
    static mut stdout: *mut FILE;
}

fn c_stdout() -> *mut FILE {
    unsafe { stdout }
}

unsafe fn errno() -> c_int {
    *libc::__errno_location()
}

unsafe fn set_errno(v: c_int) {
    *libc::__errno_location() = v;
}

/// Best-effort `setvbuf` on the libc `stdout`. Returns whether it took effect.
/// POSIX only guarantees `setvbuf` before first use, so a failure is tolerated:
/// the differential comparison is still valid, it just runs in whatever mode the
/// stream already had.
unsafe fn set_buf_mode(mode: c_int) -> bool {
    libc::fflush(c_stdout());
    let size = if mode == libc::_IONBF { 0 } else { 4096 };
    libc::setvbuf(c_stdout(), std::ptr::null_mut(), mode, size) == 0
}

struct Redirect {
    saved: c_int,
}

impl Redirect {
    /// Point fd 1 at `fd`, after flushing whatever is pending on `stdout`.
    unsafe fn to_fd(fd: c_int) -> Redirect {
        libc::fflush(c_stdout());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed, errno={}", errno());
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed, errno={}", errno());
        Redirect { saved }
    }

    /// Flush, then put the original fd 1 back. Returns `(fflush ret, ferror)`.
    unsafe fn finish(self) -> (c_int, c_int) {
        let fr = libc::fflush(c_stdout());
        let fe = libc::ferror(c_stdout());
        // Never let a failed flush leave bytes sitting in the FILE buffer where
        // they could leak into a later capture.
        libc::clearerr(c_stdout());
        libc::fflush(c_stdout());
        libc::clearerr(c_stdout());
        assert!(libc::dup2(self.saved, 1) >= 0, "restoring fd 1 failed");
        libc::close(self.saved);
        (fr, fe)
    }
}

/// Run `f` with fd 1 redirected to a fresh temp file; return everything written.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _g = io_lock();
    unsafe { capture_locked(f) }
}

unsafe fn capture_locked<F: FnOnce()>(f: F) -> Vec<u8> {
    let path = tmp_file("cap");
    let file = File::create(&path).expect("create capture temp file");
    let red = Redirect::to_fd(file.as_raw_fd());
    f();
    let _ = red.finish();
    drop(file);
    let data = std::fs::read(&path).expect("read capture temp file");
    let _ = std::fs::remove_file(&path);
    data
}

/// Same as [`capture`] but forces a `stdout` buffering mode first.
fn capture_with_mode<F: FnOnce()>(mode: c_int, f: F) -> (Vec<u8>, bool) {
    let _g = io_lock();
    unsafe {
        let path = tmp_file("capmode");
        let file = File::create(&path).expect("create capture temp file");
        let red = Redirect::to_fd(file.as_raw_fd());
        let applied = set_buf_mode(mode);
        f();
        let _ = red.finish();
        set_buf_mode(libc::_IOFBF);
        drop(file);
        let data = std::fs::read(&path).expect("read capture temp file");
        let _ = std::fs::remove_file(&path);
        (data, applied)
    }
}

/// Run `f` with fd 1 redirected to the write end of a pipe; return what was read.
/// Only for small payloads (well under the 64 KiB pipe capacity).
fn capture_via_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    let _g = io_lock();
    unsafe {
        let mut fds = [0 as c_int; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        let red = Redirect::to_fd(fds[1]);
        f();
        let _ = red.finish();
        libc::close(fds[1]);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = libc::read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        libc::close(fds[0]);
        out
    }
}

/// Observable stream state after driving `f` with fd 1 pointed at a target that
/// may reject writes. Flags are normalised to 0/1 so we compare *behaviour*,
/// not incidental glibc return values.
#[derive(Debug, PartialEq, Eq)]
struct StreamOutcome {
    err_after_call: c_int,
    flush_failed: c_int,
    err_after_flush: c_int,
    errno_after_flush: c_int,
}

fn run_with_stdout_target<F: FnOnce()>(
    target: &str,
    flags: c_int,
    mode: c_int,
    f: F,
) -> StreamOutcome {
    let _g = io_lock();
    unsafe {
        let cp = CString::new(target).unwrap();
        let fd = libc::open(cp.as_ptr(), flags);
        assert!(fd >= 0, "open({target}) failed, errno={}", errno());

        libc::fflush(c_stdout());
        libc::clearerr(c_stdout());
        let saved = libc::dup(1);
        assert!(saved >= 0);
        assert!(libc::dup2(fd, 1) >= 0);
        // Unbuffered so a write failure happens *inside* `driver` and no bytes
        // are retained in the FILE buffer afterwards.
        set_buf_mode(mode);
        set_errno(0);

        f();

        let err_after_call = libc::ferror(c_stdout());
        let flush_ret = libc::fflush(c_stdout());
        let err_after_flush = libc::ferror(c_stdout());
        let errno_after_flush = errno();

        libc::clearerr(c_stdout());
        libc::fflush(c_stdout());
        libc::clearerr(c_stdout());
        assert!(libc::dup2(saved, 1) >= 0);
        libc::close(saved);
        libc::close(fd);
        set_buf_mode(libc::_IOFBF);
        set_errno(0);

        StreamOutcome {
            err_after_call: (err_after_call != 0) as c_int,
            flush_failed: (flush_ret != 0) as c_int,
            err_after_flush: (err_after_flush != 0) as c_int,
            errno_after_flush,
        }
    }
}

// ---------------------------------------------------------------------------
// Oracle + comparison
// ---------------------------------------------------------------------------

/// Independent re-implementation of the C's observable contract:
/// `for each of the 4 object-representation bytes: printf("%02x", b); printf("\n")`.
fn expected_line(bits: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(9);
    for b in bits.to_ne_bytes() {
        out.push(HEX[(b >> 4) as usize]);
        out.push(HEX[(b & 0x0f) as usize]);
    }
    out.push(b'\n');
    out
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn expected_batch(bits: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bits.len() * 9);
    for &b in bits {
        out.extend_from_slice(&expected_line(b));
    }
    out
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Compare C output vs Rust output (the hard requirement) and then both against
/// the independent `%02x` oracle (a spec cross-check).
fn compare(label: &str, bits: &[u32], c_out: &[u8], r_out: &[u8]) {
    assert_eq!(
        c_out.len(),
        bits.len() * 9,
        "{label}: C emitted {} bytes for {} call(s); every call must emit exactly \
         8 hex digits + '\\n'",
        c_out.len(),
        bits.len()
    );
    assert_eq!(
        r_out.len(),
        bits.len() * 9,
        "{label}: Rust emitted {} bytes for {} call(s); expected {}. First 64 bytes: {}",
        r_out.len(),
        bits.len(),
        bits.len() * 9,
        show(&r_out[..r_out.len().min(64)])
    );
    if c_out != r_out {
        for (i, (cl, rl)) in c_out.chunks(9).zip(r_out.chunks(9)).enumerate() {
            if cl != rl {
                panic!(
                    "{label}: DIVERGENCE at call #{i}, input bits=0x{:08x} \
                     (as f32 = {:?})\n  C   : \"{}\"\n  Rust: \"{}\"",
                    bits[i],
                    f32::from_bits(bits[i]),
                    show(cl),
                    show(rl)
                );
            }
        }
        panic!("{label}: outputs differ but no differing 9-byte line was found");
    }
    let exp = expected_batch(bits);
    assert_eq!(
        c_out,
        &exp[..],
        "{label}: the C output disagrees with the independent %02x oracle"
    );
}

/// Drive both libraries over the same batch of inputs and compare.
fn diff_bits(label: &str, bits: &[u32]) {
    assert!(!bits.is_empty(), "{label}: empty input batch");
    let a = api();
    let c_out = capture(|| {
        for &b in bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let r_out = capture(|| {
        for &b in bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    compare(label, bits, &c_out, &r_out);
}

fn diff_floats(label: &str, xs: &[f32]) {
    let bits: Vec<u32> = xs.iter().map(|x| x.to_bits()).collect();
    diff_bits(label, &bits);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed => reproducible runs
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_1234_ABCD_0001;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
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
    /// Uniform in `[lo, hi]` inclusive.
    fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ===========================================================================
// PHASE B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

/// Helper for B1/E12: every byte value 0x00..=0xFF in every byte position.
fn all_byte_values_all_positions(filler: u32) -> Vec<u32> {
    let mut bits = Vec::with_capacity(4 * 256);
    for pos in 0..4u32 {
        for v in 0..=0xffu32 {
            let mask = 0xffu32 << (8 * pos);
            bits.push((filler & !mask) | (v << (8 * pos)));
        }
    }
    bits
}

/// B1 — the `%02x` conversion domain, exhaustively, in every byte position.
fn b1_percent02x_domain_all_bytes_all_positions() {
    // Three fillers so the "other" bytes are zero, all-ones, and a random mix.
    let mut rng = Rng::new(SEED ^ 0xB1);
    for filler in [0x0000_0000u32, 0xffff_ffff, rng.next_u32()] {
        let bits = all_byte_values_all_positions(filler);
        diff_bits(&format!("B1 filler=0x{filler:08x}"), &bits);
    }
}

/// B2 — full 32-bit bit-space sweep (covers all 7 IEEE-754 classes jointly).
fn b2_full_32bit_random_sweep() {
    let mut rng = Rng::new(SEED ^ 0xB2);
    let bits: Vec<u32> = (0..20_000).map(|_| rng.next_u32()).collect();
    diff_bits("B2 random 32-bit sweep", &bits);
}

/// B3 — all-bytes-equal patterns 0xVVVVVVVV for every V.
fn b3_all_bytes_equal_patterns() {
    let bits: Vec<u32> = (0..=0xffu32).map(|v| u32::from_ne_bytes([v as u8; 4])).collect();
    diff_bits("B3 all-bytes-equal", &bits);
}

/// B4 — exhaustive cross-product of per-byte boundary values (9^4 = 6561).
fn b4_byte_boundary_cross_product() {
    const B: [u8; 9] = [0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0x81, 0xfe, 0xff];
    let mut bits = Vec::with_capacity(9 * 9 * 9 * 9);
    for &a in &B {
        for &b in &B {
            for &c in &B {
                for &d in &B {
                    bits.push(u32::from_ne_bytes([a, b, c, d]));
                }
            }
        }
    }
    assert_eq!(bits.len(), 6561);
    diff_bits("B4 byte-boundary cross product", &bits);
}

/// B5 — signed zeros: numerically equal, bit-wise distinct.
fn b5_signed_zeros() {
    diff_bits("B5 signed zeros", &[0x0000_0000, 0x8000_0000]);
    let a = api();
    let c_pos = capture(|| unsafe { (a.c)(0.0f32) });
    let c_neg = capture(|| unsafe { (a.c)(-0.0f32) });
    assert_ne!(
        c_pos, c_neg,
        "the C library must distinguish +0.0 from -0.0 (it prints raw bytes)"
    );
    let r_pos = capture(|| unsafe { (a.rust)(0.0f32) });
    let r_neg = capture(|| unsafe { (a.rust)(-0.0f32) });
    assert_eq!(c_pos, r_pos, "+0.0");
    assert_eq!(c_neg, r_neg, "-0.0");
    assert_eq!(c_pos, b"00000000\n");
    assert_eq!(c_neg, b"00000080\n");
}

/// B6 — positive normals, randomised over the whole normal exponent range.
fn b6_positive_normals() {
    let mut rng = Rng::new(SEED ^ 0xB6);
    let bits: Vec<u32> = (0..5_000)
        .map(|_| rng.range_u32(0x0080_0000, 0x7f7f_ffff))
        .collect();
    for &b in &bits {
        let x = f32::from_bits(b);
        assert!(x.is_normal() && x > 0.0, "bits 0x{b:08x} is not a positive normal");
    }
    diff_bits("B6 positive normals", &bits);
}

/// B7 — negative normals.
fn b7_negative_normals() {
    let mut rng = Rng::new(SEED ^ 0xB7);
    let bits: Vec<u32> = (0..5_000)
        .map(|_| rng.range_u32(0x0080_0000, 0x7f7f_ffff) | 0x8000_0000)
        .collect();
    for &b in &bits {
        let x = f32::from_bits(b);
        assert!(x.is_normal() && x < 0.0, "bits 0x{b:08x} is not a negative normal");
    }
    diff_bits("B7 negative normals", &bits);
}

/// B8 — subnormals, both signs.
fn b8_subnormals() {
    let mut rng = Rng::new(SEED ^ 0xB8);
    let bits: Vec<u32> = (0..5_000)
        .map(|_| {
            let m = rng.range_u32(0x0000_0001, 0x007f_ffff);
            if rng.bool() { m | 0x8000_0000 } else { m }
        })
        .collect();
    for &b in &bits {
        assert!(
            f32::from_bits(b).is_subnormal(),
            "bits 0x{b:08x} is not subnormal"
        );
    }
    diff_bits("B8 subnormals", &bits);
}

/// B9 — infinities.
fn b9_infinities() {
    diff_bits("B9 infinities", &[0x7f80_0000, 0xff80_0000]);
    let a = api();
    assert_eq!(capture(|| unsafe { (a.c)(f32::INFINITY) }), b"0000807f\n");
    assert_eq!(
        capture(|| unsafe { (a.rust)(f32::INFINITY) }),
        capture(|| unsafe { (a.c)(f32::INFINITY) })
    );
    assert_eq!(
        capture(|| unsafe { (a.rust)(f32::NEG_INFINITY) }),
        capture(|| unsafe { (a.c)(f32::NEG_INFINITY) })
    );
}

/// B10 — quiet NaNs with randomised payloads, both signs.
fn b10_quiet_nans() {
    let mut rng = Rng::new(SEED ^ 0xB10);
    let bits: Vec<u32> = (0..5_000)
        .map(|_| {
            // exponent all-ones, mantissa MSB set => quiet NaN
            let payload = rng.range_u32(0, 0x003f_ffff);
            let sign = if rng.bool() { 0x8000_0000 } else { 0 };
            sign | 0x7f80_0000 | 0x0040_0000 | payload
        })
        .collect();
    for &b in &bits {
        assert!(f32::from_bits(b).is_nan(), "bits 0x{b:08x} is not NaN");
    }
    diff_bits("B10 quiet NaNs", &bits);
}

/// B11 — SIGNALLING NaNs: mantissa MSB clear, payload non-zero. These are the
/// patterns a value-preserving-but-not-bit-preserving translation would quiet.
fn b11_signalling_nans() {
    let mut rng = Rng::new(SEED ^ 0xB11);
    let bits: Vec<u32> = (0..5_000)
        .map(|_| {
            let payload = rng.range_u32(0x0000_0001, 0x003f_ffff);
            let sign = if rng.bool() { 0x8000_0000 } else { 0 };
            sign | 0x7f80_0000 | payload
        })
        .collect();
    for &b in &bits {
        assert!(f32::from_bits(b).is_nan(), "bits 0x{b:08x} is not NaN");
        assert_eq!(b & 0x0040_0000, 0, "bits 0x{b:08x} is not signalling");
    }
    diff_bits("B11 signalling NaNs", &bits);
    // Explicit smallest/largest sNaN payloads.
    diff_bits(
        "B11 sNaN extremes",
        &[0x7f80_0001, 0xff80_0001, 0x7fbf_ffff, 0xffbf_ffff],
    );
}

/// B12 — exact IEEE boundary constants and every power of two.
fn b12_ieee_boundary_constants() {
    let mut xs: Vec<f32> = vec![
        f32::MIN_POSITIVE,          // 0x00800000
        -f32::MIN_POSITIVE,
        f32::MAX,                   // 0x7f7fffff
        f32::MIN,                   // 0xff7fffff
        f32::EPSILON,
        f32::from_bits(0x0000_0001), // FLT_TRUE_MIN
        f32::from_bits(0x007f_ffff), // largest subnormal
        f32::from_bits(0x7f80_0000), // one step past FLT_MAX => inf
        1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 3.0, 10.0, 1e-38, 1e38, -1e38,
    ];
    for e in -149i32..=127 {
        // 2^e for every representable exponent, including the subnormal range.
        let v = if e >= -126 {
            f32::from_bits((((e + 127) as u32) << 23) & 0x7f80_0000)
        } else {
            f32::from_bits(1u32 << (e + 149))
        };
        xs.push(v);
        xs.push(-v);
    }
    diff_floats("B12 IEEE boundary constants", &xs);
}

/// B13 — the "ordinary" values a real consumer passes.
fn b13_integral_and_decimal_values() {
    let mut xs: Vec<f32> = Vec::new();
    for i in 0..=1024i32 {
        xs.push(i as f32);
        xs.push(-(i as f32));
    }
    for e in 0..=30u32 {
        xs.push(10f32.powi(e as i32));
        xs.push(-10f32.powi(e as i32));
    }
    xs.extend_from_slice(&[0.1, 0.2, 0.3, 1.0 / 3.0, 2.0 / 3.0, 3.14159265, 2.71828183]);
    diff_floats("B13 ordinary values", &xs);
}

/// B14 — exactly one call: exactly 9 bytes, nothing else.
fn b14_single_call_exact_bytes() {
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xB14);
    for _ in 0..200 {
        let bits = rng.next_u32();
        let x = f32::from_bits(bits);
        let c_out = capture(|| unsafe { (a.c)(x) });
        let r_out = capture(|| unsafe { (a.rust)(x) });
        assert_eq!(c_out.len(), 9, "C emitted {} bytes for one call", c_out.len());
        assert_eq!(c_out, r_out, "single call, bits=0x{bits:08x}");
        assert_eq!(c_out, expected_line(bits), "bits=0x{bits:08x}");
    }
}

/// B15 — zero calls, and no load-time side effects from either `.so`.
fn b15_zero_calls_and_no_load_side_effects() {
    let a = api(); // ensure both libraries are already loaded
    assert_eq!(capture(|| {}), Vec::<u8>::new(), "empty capture window");

    // Independently dlopen fresh copies *inside* a capture window: a library
    // constructor that printed anything would show up here.
    let dir = std::env::temp_dir().join(format!("driver_copies_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let c_copy = dir.join("libdriver_c_copy.so");
    let r_copy = dir.join("libdriver_rust_copy.so");
    std::fs::copy(&a.c_path, &c_copy).unwrap();
    std::fs::copy(&a.rust_path, &r_copy).unwrap();

    let out = capture(|| unsafe {
        let l1 = Library::new(&c_copy).expect("dlopen C copy");
        let l2 = Library::new(&r_copy).expect("dlopen Rust copy");
        // Both copies must still export a resolvable `driver`.
        let s1: Symbol<Option<DriverFn>> = l1.get(b"driver\0").expect("driver in C copy");
        let s2: Symbol<Option<DriverFn>> = l2.get(b"driver\0").expect("driver in Rust copy");
        assert!((*s1).is_some(), "C copy: driver resolved to NULL");
        assert!((*s2).is_some(), "Rust copy: driver resolved to NULL");
    });
    assert_eq!(
        out,
        Vec::<u8>::new(),
        "loading the shared objects must not write to stdout, got {:?}",
        show(&out)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// B16 — many calls, fully buffered, so lines straddle the 4096-byte buffer.
fn b16_many_calls_buffer_boundary() {
    let mut rng = Rng::new(SEED ^ 0xB16);
    let bits: Vec<u32> = (0..10_000).map(|_| rng.next_u32()).collect();
    let a = api();
    let (c_out, applied) = capture_with_mode(libc::_IOFBF, || {
        for &b in &bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let (r_out, _) = capture_with_mode(libc::_IOFBF, || {
        for &b in &bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    compare(
        &format!("B16 fully buffered (setvbuf applied={applied})"),
        &bits,
        &c_out,
        &r_out,
    );
    // 10_000 * 9 = 90_000 bytes: the buffer boundary is crossed ~22 times and
    // 9 does not divide 4096, so lines genuinely straddle it.
    assert_eq!(c_out.len(), 90_000);
}

/// B17 — unbuffered `stdout` (one write syscall per `%02x`).
fn b17_unbuffered_mode() {
    let bits = all_byte_values_all_positions(0x5a5a_5a5a);
    let a = api();
    let (c_out, applied) = capture_with_mode(libc::_IONBF, || {
        for &b in &bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let (r_out, _) = capture_with_mode(libc::_IONBF, || {
        for &b in &bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    compare(
        &format!("B17 unbuffered (setvbuf applied={applied})"),
        &bits,
        &c_out,
        &r_out,
    );
}

/// B18 — line-buffered `stdout` (flush triggered by the '\n' inside `driver`).
fn b18_line_buffered_mode() {
    let mut rng = Rng::new(SEED ^ 0xB18);
    let bits: Vec<u32> = (0..2_000).map(|_| rng.next_u32()).collect();
    let a = api();
    let (c_out, applied) = capture_with_mode(libc::_IOLBF, || {
        for &b in &bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let (r_out, _) = capture_with_mode(libc::_IOLBF, || {
        for &b in &bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    compare(
        &format!("B18 line buffered (setvbuf applied={applied})"),
        &bits,
        &c_out,
        &r_out,
    );
}

/// B19 — interleaving with the caller's own libc `printf` on the same stream.
fn b19_interleaved_with_caller_printf() {
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xB19);
    let bits: Vec<u32> = (0..500).map(|_| rng.next_u32()).collect();

    let run = |f: DriverFn| {
        capture(|| {
            for (i, &b) in bits.iter().enumerate() {
                unsafe {
                    libc::printf(c"<%d>".as_ptr(), i as c_int);
                    f(f32::from_bits(b));
                    libc::printf(c"[%d]\n".as_ptr(), i as c_int);
                }
            }
        })
    };
    let c_out = run(a.c);
    let r_out = run(a.rust);
    assert_eq!(
        c_out, r_out,
        "B19: interleaved output differs between C and Rust"
    );

    // Build the expected interleaving independently.
    let mut exp = Vec::new();
    for (i, &b) in bits.iter().enumerate() {
        exp.extend_from_slice(format!("<{i}>").as_bytes());
        exp.extend_from_slice(&expected_line(b));
        exp.extend_from_slice(format!("[{i}]\n").as_bytes());
    }
    assert_eq!(c_out, exp, "B19: C output is not the expected interleaving");
}

/// B20 — alternate C and Rust `driver` calls on one shared `stdout`.
fn b20_alternating_c_and_rust_same_stream() {
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xB20);
    let bits: Vec<u32> = (0..2_000).map(|_| rng.next_u32()).collect();
    let out = capture(|| {
        for &b in &bits {
            let x = f32::from_bits(b);
            unsafe {
                (a.c)(x);
                (a.rust)(x);
            }
        }
    });
    assert_eq!(out.len(), bits.len() * 18, "B20: wrong total byte count");
    for (i, (pair, &b)) in out.chunks(18).zip(bits.iter()).enumerate() {
        let (c_line, r_line) = pair.split_at(9);
        assert_eq!(
            c_line,
            r_line,
            "B20: call #{i} bits=0x{b:08x}: C=\"{}\" Rust=\"{}\"",
            show(c_line),
            show(r_line)
        );
        assert_eq!(c_line, expected_line(b), "B20: call #{i} vs oracle");
    }
}

/// B21 — the byte stream is identical whether fd 1 is a file, a pipe, or /dev/null.
fn b21_output_target_shapes() {
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xB21);
    let bits: Vec<u32> = (0..1_000).map(|_| rng.next_u32()).collect(); // 9000 bytes < pipe cap

    // (a) regular file
    let c_file = capture(|| {
        for &b in &bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let r_file = capture(|| {
        for &b in &bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    compare("B21 regular file", &bits, &c_file, &r_file);

    // (b) pipe
    let c_pipe = capture_via_pipe(|| {
        for &b in &bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let r_pipe = capture_via_pipe(|| {
        for &b in &bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    compare("B21 pipe", &bits, &c_pipe, &r_pipe);
    assert_eq!(c_pipe, c_file, "B21: pipe and file output must be identical");

    // (c) /dev/null — nothing observable to read back, so compare stream state.
    let c_null = run_with_stdout_target("/dev/null", libc::O_WRONLY, libc::_IOFBF, || {
        for &b in &bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let r_null = run_with_stdout_target("/dev/null", libc::O_WRONLY, libc::_IOFBF, || {
        for &b in &bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    assert_eq!(c_null, r_null, "B21 /dev/null: stream state differs");
    assert_eq!(
        c_null,
        StreamOutcome {
            err_after_call: 0,
            flush_failed: 0,
            err_after_flush: 0,
            errno_after_flush: c_null.errno_after_flush,
        },
        "B21 /dev/null: writing to /dev/null must not set an error"
    );
}

/// B22 — the same 32-bit pattern reached three different ways must produce one
/// identical line (no canonicalisation anywhere on the argument-passing path).
fn b22_same_bits_reached_three_ways() {
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xB22);
    let mut cases: Vec<u32> = (0..500).map(|_| rng.next_u32()).collect();
    cases.extend_from_slice(&[
        0x0000_0000, 0x8000_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000, 0x7f80_0001,
        0x3f80_0000, 0xffff_ffff,
    ]);
    for &bits in &cases {
        let via_from_bits = f32::from_bits(bits);
        let via_ne_bytes = f32::from_ne_bytes(bits.to_ne_bytes());
        let via_roundtrip = f32::from_bits(via_from_bits.to_bits());
        let out = capture(|| unsafe {
            (a.c)(via_from_bits);
            (a.c)(via_ne_bytes);
            (a.c)(via_roundtrip);
            (a.rust)(via_from_bits);
            (a.rust)(via_ne_bytes);
            (a.rust)(via_roundtrip);
        });
        assert_eq!(out.len(), 54, "B22: bits=0x{bits:08x}");
        let want = expected_line(bits);
        for (i, line) in out.chunks(9).enumerate() {
            assert_eq!(
                line,
                &want[..],
                "B22: bits=0x{bits:08x} variant #{i} produced \"{}\", expected \"{}\"",
                show(line),
                show(&want)
            );
        }
    }
}

/// B23 — record which feature configuration this binary was compiled with, so a
/// failing run identifies the configuration. The crate declares no non-default
/// features, so there is exactly one combination (the empty set).
fn b23_feature_configuration() {
    let features: Vec<&str> = Vec::new(); // no `feature = "..."` cfgs exist
    assert!(
        features.is_empty(),
        "unexpected feature set active: {features:?}"
    );
    // Smoke-check that the single configuration really is exercised end to end.
    diff_bits("B23 feature combo (empty)", &[0x3f80_0000, 0xdead_beef]);
}

// ===========================================================================
// PHASE C — error-path differential tests, one per ERRORS.md row
// ===========================================================================

fn c_header_text() -> String {
    std::fs::read_to_string(manifest_dir().join("c_src").join("include").join("driver.h"))
        .expect("read driver.h")
}

fn c_source_text() -> String {
    std::fs::read_to_string(manifest_dir().join("c_src").join("src").join("driver.c"))
        .expect("read driver.c")
}

/// Strip `//` comments so structural greps only see real code.
fn code_only(s: &str) -> String {
    s.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// E1 — there is no error return anywhere; every call succeeds and emits 9 bytes.
fn e1_no_error_return_ever() {
    let code = code_only(&c_source_text());
    assert!(
        !code.contains("return "),
        "driver.c gained a `return` statement; ERRORS.md must be re-derived"
    );
    assert!(
        code_only(&c_header_text()).contains("void driver(float x);"),
        "driver.h no longer declares `void driver(float x);`"
    );

    // Behavioural half: nothing is ever rejected, for any input.
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xE1);
    let bits: Vec<u32> = (0..3_000).map(|_| rng.next_u32()).collect();
    let c_out = capture(|| {
        for &b in &bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let r_out = capture(|| {
        for &b in &bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    assert_eq!(c_out.len(), bits.len() * 9, "E1: C rejected some input");
    assert_eq!(r_out.len(), bits.len() * 9, "E1: Rust rejected some input");
    compare("E1 nothing is ever rejected", &bits, &c_out, &r_out);
}

/// E2 — the `len <= 0` branch of `print_hex` is dead: `driver` never emits a
/// bare newline, and every emitted line is exactly 8 lowercase hex digits.
fn e2_never_emits_bare_newline() {
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xE2);
    let mut bits: Vec<u32> = (0..3_000).map(|_| rng.next_u32()).collect();
    bits.extend_from_slice(&[0, 0xffff_ffff, 0x8000_0000, 0x7f80_0000]);
    for (label, f) in [("C", a.c), ("Rust", a.rust)] {
        let out = capture(|| {
            for &b in &bits {
                unsafe { f(f32::from_bits(b)) }
            }
        });
        assert_eq!(out.len(), bits.len() * 9, "{label}: wrong byte count");
        for (i, line) in out.chunks(9).enumerate() {
            assert_eq!(line.len(), 9, "{label}: line #{i} is not 9 bytes");
            assert_ne!(line, b"\n", "{label}: line #{i} is a bare newline");
            assert_eq!(line[8], b'\n', "{label}: line #{i} does not end with '\\n'");
            for (j, &ch) in line[..8].iter().enumerate() {
                assert!(
                    ch.is_ascii_digit() || (b'a'..=b'f').contains(&ch),
                    "{label}: line #{i} char {j} = {ch:#04x} is not a lowercase hex digit"
                );
            }
        }
    }
}

/// E3 — quiet NaN.
fn e3_quiet_nan() {
    let a = api();
    let out_c = capture(|| unsafe { (a.c)(f32::NAN) });
    let out_r = capture(|| unsafe { (a.rust)(f32::NAN) });
    assert_eq!(out_c, out_r, "E3: quiet NaN diverges");
    assert_eq!(out_c, expected_line(f32::NAN.to_bits()));
    diff_bits("E3 quiet NaN patterns", &[0x7fc0_0000, 0xffc0_0000]);
}

/// E4 — signalling NaN bits must survive the parameter pass unquieted.
fn e4_signalling_nan_bits_preserved() {
    let snans: [u32; 8] = [
        0x7f80_0001, 0x7f80_0002, 0x7fbf_ffff, 0x7fa5_5a5a,
        0xff80_0001, 0xff80_0002, 0xffbf_ffff, 0xffa5_5a5a,
    ];
    diff_bits("E4 signalling NaNs", &snans);
    // And explicitly: the output must NOT be the quiet form.
    let a = api();
    for &b in &snans {
        let out = capture(|| unsafe { (a.rust)(f32::from_bits(b)) });
        assert_eq!(
            out,
            expected_line(b),
            "E4: bits 0x{b:08x} were quieted/canonicalised by the Rust side"
        );
        let quiet = expected_line(b | 0x0040_0000);
        assert_ne!(out, quiet, "E4: bits 0x{b:08x} printed as the quiet form");
    }
}

/// E5 — arbitrary NaN payloads.
fn e5_nan_payloads_random() {
    let mut rng = Rng::new(SEED ^ 0xE5);
    let bits: Vec<u32> = (0..5_000)
        .map(|_| {
            let payload = rng.range_u32(0x0000_0001, 0x007f_ffff);
            let sign = if rng.bool() { 0x8000_0000 } else { 0 };
            sign | 0x7f80_0000 | payload
        })
        .collect();
    for &b in &bits {
        assert!(f32::from_bits(b).is_nan());
    }
    diff_bits("E5 NaN payloads", &bits);
}

/// E6 — infinities.
fn e6_infinities() {
    let a = api();
    let c_out = capture(|| unsafe {
        (a.c)(f32::INFINITY);
        (a.c)(f32::NEG_INFINITY);
    });
    let r_out = capture(|| unsafe {
        (a.rust)(f32::INFINITY);
        (a.rust)(f32::NEG_INFINITY);
    });
    assert_eq!(c_out, r_out, "E6: infinities diverge");
    assert_eq!(c_out, b"0000807f\n000080ff\n");
}

/// E7 — negative zero must not be folded into positive zero.
fn e7_negative_zero() {
    let a = api();
    let c_out = capture(|| unsafe { (a.c)(-0.0f32) });
    let r_out = capture(|| unsafe { (a.rust)(-0.0f32) });
    assert_eq!(c_out, r_out, "E7: -0.0 diverges");
    assert_eq!(c_out, b"00000080\n");
    assert_ne!(c_out, b"00000000\n", "E7: -0.0 was folded to +0.0");
}

/// E8 — `%02x` zero padding: `+0.0` must print eight '0' characters.
fn e8_positive_zero_padding() {
    let a = api();
    let c_out = capture(|| unsafe { (a.c)(0.0f32) });
    let r_out = capture(|| unsafe { (a.rust)(0.0f32) });
    assert_eq!(c_out, r_out, "E8: +0.0 diverges");
    assert_eq!(c_out, b"00000000\n", "E8: expected 8 zero-padded hex digits");
    // Any pattern containing a zero byte and a single-digit byte.
    diff_bits(
        "E8 zero padding patterns",
        &[0x0000_0000, 0x0000_0001, 0x0100_0000, 0x000f_0000, 0x0001_0203],
    );
}

/// E9 — subnormals below FLT_MIN must not be flushed to zero.
fn e9_subnormals() {
    let bits: Vec<u32> = vec![
        0x0000_0001, 0x0000_0002, 0x0000_00ff, 0x0040_0000, 0x007f_ffff,
        0x8000_0001, 0x8000_0002, 0x8040_0000, 0x807f_ffff,
    ];
    for &b in &bits {
        assert!(f32::from_bits(b).is_subnormal(), "0x{b:08x}");
    }
    diff_bits("E9 subnormals", &bits);
    let a = api();
    let tiny = f32::from_bits(0x0000_0001);
    let out = capture(|| unsafe { (a.rust)(tiny) });
    assert_eq!(out, b"01000000\n", "E9: smallest subnormal was flushed to zero");
}

/// E10 — range boundaries and one step past them.
fn e10_range_boundaries_one_step_past() {
    let bits: Vec<u32> = vec![
        0x007f_ffff, // largest subnormal
        0x0080_0000, // FLT_MIN (smallest normal)
        0x0080_0001, // one step past FLT_MIN
        0x7f7f_fffe,
        0x7f7f_ffff, // FLT_MAX
        0x7f80_0000, // one step past FLT_MAX => +inf
        0x7f80_0001, // one step past +inf => sNaN
        0xff7f_ffff, // -FLT_MAX
        0xff80_0000, // -inf
        0xff80_0001,
        0x3400_0000, // FLT_EPSILON
        0x0000_0000,
        0x8000_0000,
        0xffff_ffff, // the numerically largest bit pattern
    ];
    diff_bits("E10 range boundaries", &bits);
}

/// E11 — bytes with the high bit set must not be sign-extended by `%02x`.
fn e11_high_bit_bytes() {
    let mut bits: Vec<u32> = vec![
        0xffff_ffff, 0x8080_8080, 0xff00_ff00, 0x00ff_00ff, 0x80ff_7f01,
    ];
    for v in 0x80u32..=0xff {
        bits.push(u32::from_ne_bytes([v as u8; 4]));
    }
    diff_bits("E11 high-bit bytes", &bits);
    let a = api();
    let out = capture(|| unsafe { (a.rust)(f32::from_bits(0xffff_ffff)) });
    assert_eq!(out, b"ffffffff\n", "E11: sign extension or wrong width");
    assert_eq!(out.len(), 9, "E11: output is not 8 hex digits + newline");
}

/// E12 — the whole `%02x` formatting domain: every byte value, every position.
fn e12_all_256_byte_values_in_all_4_positions() {
    for filler in [0x0000_0000u32, 0xffff_ffff, 0x1234_5678, 0x8000_0001] {
        let bits = all_byte_values_all_positions(filler);
        assert_eq!(bits.len(), 1024);
        diff_bits(&format!("E12 filler=0x{filler:08x}"), &bits);
    }
}

/// E13 — `stdout` write failure with ENOSPC (`/dev/full`): the C ignores it.
fn e13_stdout_enospc_dev_full() {
    if !std::path::Path::new("/dev/full").exists() {
        eprintln!("E13 skipped: /dev/full is not available");
        return;
    }
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xE13);
    let bits: Vec<u32> = (0..50).map(|_| rng.next_u32()).collect();

    let run = |f: DriverFn| {
        run_with_stdout_target("/dev/full", libc::O_WRONLY, libc::_IONBF, || {
            for &b in &bits {
                unsafe { f(f32::from_bits(b)) }
            }
        })
    };
    let c_out = run(a.c);
    let r_out = run(a.rust);
    assert_eq!(
        c_out, r_out,
        "E13: C and Rust react differently to a failing stdout (ENOSPC)"
    );
    // The failure really happened, and `driver` still returned normally.
    assert_eq!(
        c_out.err_after_call, 1,
        "E13: precondition not met — writing to /dev/full did not fail (outcome={c_out:?})"
    );
    assert_eq!(
        c_out.errno_after_flush,
        libc::ENOSPC,
        "E13: expected ENOSPC, got errno={} (outcome={c_out:?})",
        c_out.errno_after_flush
    );

    // stdout is usable again and no stale bytes leaked into the next capture.
    assert_eq!(capture(|| {}), Vec::<u8>::new(), "E13: stale bytes leaked");
    diff_bits("E13 stdout recovered", &[0x3f80_0000]);
}

/// E14 — `stdout` write failure with EBADF (fd 1 is a read-only descriptor).
fn e14_stdout_ebadf_read_only_fd() {
    let a = api();
    let bits: Vec<u32> = vec![0x0000_0000, 0x3f80_0000, 0xffff_ffff, 0x7f80_0001];

    let run = |f: DriverFn| {
        run_with_stdout_target("/dev/null", libc::O_RDONLY, libc::_IONBF, || {
            for &b in &bits {
                unsafe { f(f32::from_bits(b)) }
            }
        })
    };
    let c_out = run(a.c);
    let r_out = run(a.rust);
    assert_eq!(
        c_out, r_out,
        "E14: C and Rust react differently to a failing stdout (EBADF)"
    );
    assert_eq!(
        c_out.err_after_call, 1,
        "E14: precondition not met — writing to a read-only fd did not fail \
         (outcome={c_out:?})"
    );
    assert_eq!(
        c_out.errno_after_flush,
        libc::EBADF,
        "E14: expected EBADF, got errno={}",
        c_out.errno_after_flush
    );

    assert_eq!(capture(|| {}), Vec::<u8>::new(), "E14: stale bytes leaked");
    diff_bits("E14 stdout recovered", &[0x3f80_0000]);
}

/// E15 — `stdout` already in its sticky error state before the call: the C
/// neither checks nor clears it, so output must still be produced.
fn e15_preexisting_error_state() {
    let a = api();
    let bits: Vec<u32> = vec![0x3f80_0000, 0x8000_0000, 0x7f80_0001, 0xffff_ffff];

    let run = |f: DriverFn| -> (Vec<u8>, c_int, c_int) {
        let _g = io_lock();
        unsafe {
            // 1. Poison stdout: write to a read-only fd with no buffer.
            libc::fflush(c_stdout());
            libc::clearerr(c_stdout());
            let ro = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
            assert!(ro >= 0);
            let saved = libc::dup(1);
            assert!(saved >= 0);
            assert!(libc::dup2(ro, 1) >= 0);
            set_buf_mode(libc::_IONBF);
            libc::printf(c"X".as_ptr());
            let poisoned = libc::ferror(c_stdout());
            libc::close(ro);

            // 2. Without clearing the error flag, point fd 1 at a real file.
            let path = tmp_file("e15");
            let file = File::create(&path).expect("temp file");
            assert!(libc::dup2(file.as_raw_fd(), 1) >= 0);

            // 3. Call `driver` with the error flag still set.
            for &b in &bits {
                f(f32::from_bits(b));
            }
            let err_after = libc::ferror(c_stdout());

            libc::fflush(c_stdout());
            libc::clearerr(c_stdout());
            libc::fflush(c_stdout());
            libc::clearerr(c_stdout());
            assert!(libc::dup2(saved, 1) >= 0);
            libc::close(saved);
            set_buf_mode(libc::_IOFBF);
            set_errno(0);

            drop(file);
            let data = std::fs::read(&path).expect("read temp");
            let _ = std::fs::remove_file(&path);
            ((data), (poisoned != 0) as c_int, (err_after != 0) as c_int)
        }
    };

    let (c_data, c_poisoned, c_err) = run(a.c);
    let (r_data, r_poisoned, r_err) = run(a.rust);
    assert_eq!(c_poisoned, 1, "E15: precondition not met — stdout was not poisoned");
    assert_eq!(r_poisoned, 1, "E15: precondition not met — stdout was not poisoned");
    assert_eq!(
        (c_data.clone(), c_err),
        (r_data, r_err),
        "E15: behaviour with a pre-set error flag differs"
    );
    // glibc keeps writing on an error-flagged output stream, so both libraries
    // must have produced the full output.
    assert_eq!(
        c_data,
        expected_batch(&bits),
        "E15: C did not emit the full output with the error flag set"
    );
}

/// E16 — /dev/null target under both buffered and unbuffered modes.
fn e16_unbuffered_and_devnull() {
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xE16);
    let bits: Vec<u32> = (0..300).map(|_| rng.next_u32()).collect();

    for &mode in &[libc::_IOFBF, libc::_IONBF, libc::_IOLBF] {
        let run = |f: DriverFn| {
            run_with_stdout_target("/dev/null", libc::O_WRONLY, mode, || {
                for &b in &bits {
                    unsafe { f(f32::from_bits(b)) }
                }
            })
        };
        let c_out = run(a.c);
        let r_out = run(a.rust);
        assert_eq!(c_out, r_out, "E16: mode={mode} stream state differs");
        assert_eq!(c_out.err_after_call, 0, "E16: mode={mode} unexpected error");
        assert_eq!(c_out.flush_failed, 0, "E16: mode={mode} flush failed");
    }

    // And the *bytes* are identical in every buffering mode.
    let mut per_mode: Vec<(c_int, Vec<u8>, Vec<u8>)> = Vec::new();
    for &mode in &[libc::_IOFBF, libc::_IONBF, libc::_IOLBF] {
        let (c_out, _) = capture_with_mode(mode, || {
            for &b in &bits {
                unsafe { (a.c)(f32::from_bits(b)) }
            }
        });
        let (r_out, _) = capture_with_mode(mode, || {
            for &b in &bits {
                unsafe { (a.rust)(f32::from_bits(b)) }
            }
        });
        compare(&format!("E16 mode={mode}"), &bits, &c_out, &r_out);
        per_mode.push((mode, c_out, r_out));
    }
    for w in per_mode.windows(2) {
        assert_eq!(
            w[0].1, w[1].1,
            "E16: buffering mode changed the C byte stream ({} vs {})",
            w[0].0, w[1].0
        );
    }
}

/// E17 — there is no enum/int parameter, so there is no out-of-range enum value
/// to reject; the entire 2^32 input domain is valid. Verified structurally and
/// behaviourally.
fn e17_full_32bit_input_domain_is_valid() {
    let hdr = code_only(&c_header_text());
    assert!(
        !hdr.contains("enum"),
        "driver.h gained an enum; ERRORS.md must be re-derived"
    );
    let decl = hdr
        .lines()
        .find(|l| l.contains("driver("))
        .expect("driver declaration in driver.h")
        .trim()
        .to_string();
    assert_eq!(
        decl, "void driver(float x);",
        "the public signature changed; ERRORS.md must be re-derived"
    );
    assert!(
        !decl.contains("int") && !decl.contains("enum"),
        "the public API gained an int/enum parameter: {decl}"
    );

    // Behavioural: sweep the 32-bit domain in a strided walk that touches every
    // exponent and every byte pattern class, plus the extremes.
    let mut bits: Vec<u32> = Vec::new();
    let stride = 0x0010_0001u32; // co-prime-ish stride => hits all exponents
    let mut v = 0u32;
    for _ in 0..4_000 {
        bits.push(v);
        v = v.wrapping_add(stride);
    }
    bits.extend_from_slice(&[u32::MIN, u32::MAX, 0x7fff_ffff, 0x8000_0000]);
    diff_bits("E17 strided 32-bit domain walk", &bits);
}

/// E18 — there is no pointer or length parameter in the public API, so the
/// null-pointer / zero-length / oversized-length classes do not exist.
fn e18_no_pointer_or_length_parameters() {
    let hdr = code_only(&c_header_text());
    let decl = hdr
        .lines()
        .find(|l| l.contains("driver("))
        .expect("driver declaration")
        .trim()
        .to_string();
    assert!(
        !decl.contains('*'),
        "the public API gained a pointer parameter: {decl}"
    );
    let params = &decl[decl.find('(').unwrap() + 1..decl.rfind(')').unwrap()];
    assert_eq!(
        params.trim(),
        "float x",
        "the public parameter list changed: {params}"
    );

    // The only pointer in the library is the internal `&x`, which can never be
    // null; and `len` is the constant `sizeof(float)`. Confirm the C source
    // still passes exactly that.
    let src = code_only(&c_source_text());
    assert!(
        src.contains("print_hex((unsigned char *)&x, sizeof(x))"),
        "the internal call shape changed; ERRORS.md must be re-derived"
    );
    // And no caller-visible way to reach print_hex: it is static.
    assert!(
        src.contains("static void print_hex"),
        "print_hex is no longer static; it would become part of the ABI"
    );
    diff_bits("E18 sanity", &[0x0000_0000, 0xffff_ffff]);
}

/// E19 — thousands of calls with the buffer boundary crossed mid-line.
fn e19_buffer_boundary_many_calls() {
    let a = api();
    let mut rng = Rng::new(SEED ^ 0xE19);
    let bits: Vec<u32> = (0..20_000).map(|_| rng.next_u32()).collect();
    let (c_out, _) = capture_with_mode(libc::_IOFBF, || {
        for &b in &bits {
            unsafe { (a.c)(f32::from_bits(b)) }
        }
    });
    let (r_out, _) = capture_with_mode(libc::_IOFBF, || {
        for &b in &bits {
            unsafe { (a.rust)(f32::from_bits(b)) }
        }
    });
    compare("E19 buffer boundary", &bits, &c_out, &r_out);
    assert_eq!(c_out.len(), 180_000);
}

// ===========================================================================
// Phase D — symbol parity, checked from inside the test suite as well
// ===========================================================================

/// Every symbol the C `.so` exports must be exported by the Rust `.so` too, and
/// must be reachable through `dlsym` with the exact same name.
fn d1_symbol_parity_via_dlsym() {
    let a = api();
    let c_exports = nm_defined(&a.c_path);
    let r_exports = nm_defined(&a.rust_path);
    assert!(
        !c_exports.is_empty(),
        "nm reported no exported symbols for the C .so"
    );
    let missing: Vec<&String> = c_exports.iter().filter(|s| !r_exports.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C exports: {c_exports:?}\nRust exports: {r_exports:?}"
    );

    // And each one really resolves through dlsym on the Rust library.
    unsafe {
        let lib = Library::new(&a.rust_path).expect("dlopen Rust .so");
        for name in &c_exports {
            let mut b = name.clone().into_bytes();
            b.push(0);
            let sym: Result<Symbol<*const c_void>, _> = lib.get(&b);
            assert!(sym.is_ok(), "dlsym(\"{name}\") failed on the Rust .so");
        }
        std::mem::forget(lib);
    }

    // `print_hex` is `static` in C: it must NOT be exported by either library.
    assert!(
        !c_exports.iter().any(|s| s == "print_hex"),
        "print_hex unexpectedly exported by the C .so"
    );
    assert!(
        !r_exports.iter().any(|s| s == "print_hex"),
        "print_hex must stay private in the Rust .so to match the C ABI"
    );
}

/// Exported symbol names from `nm -D --defined-only`, minus crt/gcc glue.
fn nm_defined(path: &std::path::Path) -> Vec<String> {
    const GLUE: &[&str] = &[
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__gmon_start__",
        "_edata",
        "_end",
        "_fini",
        "_init",
        "__bss_start",
    ];
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !GLUE.contains(&s.as_str()))
        .collect();
    v.sort();
    v.dedup();
    v
}

// ===========================================================================
// Sequential runner (`harness = false`)
// ===========================================================================

/// A pristine duplicate of the real fd 1, taken before anything is redirected.
static REAL_STDOUT: OnceLock<c_int> = OnceLock::new();

fn save_real_stdout() {
    REAL_STDOUT.get_or_init(|| {
        let fd = unsafe { libc::dup(1) };
        assert!(fd >= 0, "dup(1) failed at start-up");
        fd
    });
}

/// Put fd 1 and the `stdout` FILE back into a known-good state. Called after
/// every case, including after a panic that skipped a `Redirect::finish`.
fn reset_stdout() {
    let real = *REAL_STDOUT.get().expect("REAL_STDOUT initialised");
    unsafe {
        // Drain whatever is buffered into wherever fd 1 currently points, so it
        // cannot leak onto the real stdout later.
        libc::fflush(c_stdout());
        libc::clearerr(c_stdout());
        libc::dup2(real, 1);
        set_buf_mode(libc::_IOFBF);
        libc::clearerr(c_stdout());
        set_errno(0);
    }
}

type Case = (&'static str, fn());

fn cases() -> Vec<Case> {
    vec![
        // ---- Phase B: CONFIGS.md rows -------------------------------------
        ("B1  b1_percent02x_domain_all_bytes_all_positions", b1_percent02x_domain_all_bytes_all_positions),
        ("B2  b2_full_32bit_random_sweep", b2_full_32bit_random_sweep),
        ("B3  b3_all_bytes_equal_patterns", b3_all_bytes_equal_patterns),
        ("B4  b4_byte_boundary_cross_product", b4_byte_boundary_cross_product),
        ("B5  b5_signed_zeros", b5_signed_zeros),
        ("B6  b6_positive_normals", b6_positive_normals),
        ("B7  b7_negative_normals", b7_negative_normals),
        ("B8  b8_subnormals", b8_subnormals),
        ("B9  b9_infinities", b9_infinities),
        ("B10 b10_quiet_nans", b10_quiet_nans),
        ("B11 b11_signalling_nans", b11_signalling_nans),
        ("B12 b12_ieee_boundary_constants", b12_ieee_boundary_constants),
        ("B13 b13_integral_and_decimal_values", b13_integral_and_decimal_values),
        ("B14 b14_single_call_exact_bytes", b14_single_call_exact_bytes),
        ("B15 b15_zero_calls_and_no_load_side_effects", b15_zero_calls_and_no_load_side_effects),
        ("B16 b16_many_calls_buffer_boundary", b16_many_calls_buffer_boundary),
        ("B17 b17_unbuffered_mode", b17_unbuffered_mode),
        ("B18 b18_line_buffered_mode", b18_line_buffered_mode),
        ("B19 b19_interleaved_with_caller_printf", b19_interleaved_with_caller_printf),
        ("B20 b20_alternating_c_and_rust_same_stream", b20_alternating_c_and_rust_same_stream),
        ("B21 b21_output_target_shapes", b21_output_target_shapes),
        ("B22 b22_same_bits_reached_three_ways", b22_same_bits_reached_three_ways),
        ("B23 b23_feature_configuration", b23_feature_configuration),
        // ---- Phase C: ERRORS.md rows --------------------------------------
        ("E1  e1_no_error_return_ever", e1_no_error_return_ever),
        ("E2  e2_never_emits_bare_newline", e2_never_emits_bare_newline),
        ("E3  e3_quiet_nan", e3_quiet_nan),
        ("E4  e4_signalling_nan_bits_preserved", e4_signalling_nan_bits_preserved),
        ("E5  e5_nan_payloads_random", e5_nan_payloads_random),
        ("E6  e6_infinities", e6_infinities),
        ("E7  e7_negative_zero", e7_negative_zero),
        ("E8  e8_positive_zero_padding", e8_positive_zero_padding),
        ("E9  e9_subnormals", e9_subnormals),
        ("E10 e10_range_boundaries_one_step_past", e10_range_boundaries_one_step_past),
        ("E11 e11_high_bit_bytes", e11_high_bit_bytes),
        ("E12 e12_all_256_byte_values_in_all_4_positions", e12_all_256_byte_values_in_all_4_positions),
        ("E13 e13_stdout_enospc_dev_full", e13_stdout_enospc_dev_full),
        ("E14 e14_stdout_ebadf_read_only_fd", e14_stdout_ebadf_read_only_fd),
        ("E15 e15_preexisting_error_state", e15_preexisting_error_state),
        ("E16 e16_unbuffered_and_devnull", e16_unbuffered_and_devnull),
        ("E17 e17_full_32bit_input_domain_is_valid", e17_full_32bit_input_domain_is_valid),
        ("E18 e18_no_pointer_or_length_parameters", e18_no_pointer_or_length_parameters),
        ("E19 e19_buffer_boundary_many_calls", e19_buffer_boundary_many_calls),
        // ---- Phase D ------------------------------------------------------
        ("D1  d1_symbol_parity_via_dlsym", d1_symbol_parity_via_dlsym),
    ]
}

fn main() {
    save_real_stdout();

    // Accept a substring filter, like libtest does.
    let filter: Option<String> = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-') && a != "--");

    let all = cases();
    let selected: Vec<&Case> = all
        .iter()
        .filter(|(name, _)| match &filter {
            Some(f) => name.contains(f.as_str()),
            None => true,
        })
        .collect();

    eprintln!(
        "\nrunning {} differential case(s) sequentially (seed 0x{:016X})",
        selected.len(),
        SEED
    );

    // Force both libraries to load before the first capture window so that any
    // loader output is not attributed to a test.
    let a = api();
    eprintln!("  C    .so: {}", a.c_path.display());
    eprintln!("  Rust .so: {}", a.rust_path.display());

    let mut failures: Vec<(&str, String)> = Vec::new();
    for (name, f) in selected {
        let started = std::time::Instant::now();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        reset_stdout();
        match res {
            Ok(()) => eprintln!("test {name} ... ok ({:?})", started.elapsed()),
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic payload>".to_string()
                };
                eprintln!("test {name} ... FAILED ({:?})", started.elapsed());
                failures.push((name, msg));
            }
        }
    }

    if failures.is_empty() {
        eprintln!("\ndifferential result: ok. {} passed; 0 failed\n", cases().len().min(usize::MAX));
        return;
    }
    eprintln!("\nfailures:");
    for (name, msg) in &failures {
        eprintln!("---- {name} ----\n{msg}\n");
    }
    eprintln!("differential result: FAILED. {} failed\n", failures.len());
    std::process::exit(1);
}
