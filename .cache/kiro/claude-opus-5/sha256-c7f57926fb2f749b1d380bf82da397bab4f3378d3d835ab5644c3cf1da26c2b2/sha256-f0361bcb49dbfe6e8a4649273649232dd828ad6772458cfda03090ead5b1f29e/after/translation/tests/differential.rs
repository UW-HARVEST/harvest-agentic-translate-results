//! Differential test harness: loads BOTH the C `libdriver.so` and the Rust
//! `libdriver.so` with `libloading` and compares their stdout byte-for-byte
//! through the FFI boundary.
//!
//! The Rust implementation is NEVER called directly — it is always reached via
//! `dlopen`/`dlsym` on the built `cdylib`, exactly as an external C consumer
//! would, so the `#[no_mangle] extern "C"` export wrapper is under test too.

use std::ffi::c_int;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed to capture file descriptor 1 around each call.
//
// Both shared objects write with glibc's `printf`, so both share the *same*
// `stdout` FILE object in this process. Redirecting fd 1 and calling
// `fflush(NULL)` therefore captures either library's output identically.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

/// fd 1 is process-global state, so captures must not overlap.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Redirect fd 1 to a temp file, run `f`, flush all C streams, restore fd 1 and
/// return the raw bytes that were written.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver-diff-{}-{}-{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let file = std::fs::File::create(&path).expect("create capture file");
    let file_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    // Flush Rust's own buffered stdout too, so none of the runner's reporting
    // output can land inside the captured region.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    let bytes = unsafe {
        // Flush anything already pending so it is not attributed to `f`.
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file_fd, 1) >= 0, "dup2 onto fd 1 failed");

        f();

        // Force the library's buffered output out before restoring fd 1.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restore of fd 1 failed");
        close(saved);

        let mut buf = Vec::new();
        std::fs::File::open(&path)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        buf
    };

    drop(file);
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust `.so` sits in the same profile directory as this test binary
/// (`target/<profile>/libdriver.so`, test binary is `target/<profile>/deps/…`).
///
/// IMPORTANT: `cargo test` does **not** refresh a `cdylib` artifact — it only
/// rebuilds the test binaries. Loading whatever `.so` happens to be on disk can
/// therefore silently compare against a stale library and make every assertion
/// pass vacuously. `assert_rust_so_is_fresh` below turns that into a loud
/// failure; always run `cargo build` before `cargo test` (see
/// `scripts/run_all.sh`).
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    for _ in 0..3 {
        let cand = dir.join("libdriver.so");
        if cand.is_file() {
            return cand;
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => break,
        }
    }
    // Fallback: whichever profile dir has it.
    for profile in ["debug", "release"] {
        let cand = workspace_root()
            .join("translation/target")
            .join(profile)
            .join("libdriver.so");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("Rust cdylib libdriver.so not found; run `cargo build` first");
}

/// Refuse to run against a `.so` older than the Rust sources it was built from.
fn assert_rust_so_is_fresh() {
    let so = rust_so_path();
    let so_mtime = std::fs::metadata(&so)
        .and_then(|m| m.modified())
        .expect("stat Rust .so");

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src_dir];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).expect("read_dir src") {
            let entry = entry.expect("dir entry");
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let m = entry
                .metadata()
                .and_then(|m| m.modified())
                .expect("stat source");
            if newest.as_ref().map_or(true, |(_, t)| m > *t) {
                newest = Some((p, m));
            }
        }
    }

    if let Some((newest_src, newest_mtime)) = newest {
        assert!(
            newest_mtime <= so_mtime,
            "STALE ARTIFACT: {} is newer than {}.\n`cargo test` does not rebuild \
             cdylib artifacts, so this run would have compared against an \
             out-of-date library and passed vacuously.\nRun `cargo build` (or \
             `scripts/run_all.sh`) first.",
            newest_src.display(),
            so.display()
        );
    }
}

type DriverFn = unsafe extern "C" fn(c_int);

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        // dlopen with RTLD_LOCAL (libloading's default) so the two identically
        // named `driver` symbols cannot interpose on one another.
        let c = Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");
        Libs { c, rust }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("dlsym driver in C .so") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("dlsym driver in Rust .so") }
}

/// Capture the stdout produced by one `driver(x)` call in each library.
fn both(x: c_int) -> (Vec<u8>, Vec<u8>) {
    let c = c_driver();
    let r = rust_driver();
    let out_c = capture_stdout(|| unsafe { c(x) });
    let out_r = capture_stdout(|| unsafe { r(x) });
    (out_c, out_r)
}

#[track_caller]
fn assert_same(x: c_int, ctx: &str) {
    let (c, r) = both(x);
    assert_eq!(
        c,
        r,
        "{ctx}: divergence for driver({x}) [0x{x:08x}]\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    // Sanity: the C really did produce output (guards against a broken capture
    // silently making every comparison pass on two empty buffers).
    assert!(
        !c.is_empty(),
        "{ctx}: capture produced no bytes for driver({x}) — harness is broken"
    );
}

#[track_caller]
fn assert_same_many(values: &[c_int], ctx: &str) {
    for &x in values {
        assert_same(x, ctx);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every property-style row reproduces.
// ---------------------------------------------------------------------------
const SEED: u64 = 0x5EED_1234_ABCD_EF01;

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
}

// ===========================================================================
// Harness self-checks
// ===========================================================================

fn harness_loads_both_shared_objects_and_symbol_is_present() {
    let _ = c_driver();
    let _ = rust_driver();
    // A capture must actually observe C output.
    let out = capture_stdout(|| unsafe { c_driver()(0) });
    assert!(!out.is_empty(), "capture_stdout observed nothing from the C .so");
    assert_eq!(*out.last().unwrap(), b'\n', "C output must end with a newline");
}

fn harness_capture_is_not_trivially_equal() {
    // Guard against a capture implementation that returns the same bytes
    // regardless of input: two different inputs must differ.
    let a = capture_stdout(|| unsafe { c_driver()(0) });
    let b = capture_stdout(|| unsafe { c_driver()(1) });
    assert_ne!(a, b, "capture is insensitive to input — harness is broken");
}

// ===========================================================================
// Phase B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

fn cfg_c1_zero() {
    assert_same(0, "C1");
}

fn cfg_c2_one_needs_zero_padding() {
    assert_same(1, "C2");
}

fn cfg_c3_ascending_bytes_byte_order() {
    assert_same(0x0102_0304, "C3");
}

fn cfg_c4_descending_bytes_byte_order() {
    assert_same(0x0403_0201, "C4");
}

fn cfg_c5_all_bytes_below_0x10() {
    assert_same_many(&[0x0f0e_0d0c, 0x0101_0101, 0x0000_0000, 0x0f0f_0f0f], "C5");
}

fn cfg_c6_all_bytes_high_bit_set() {
    assert_same_many(
        &[
            0x8081_8283u32 as i32,
            0xffff_ffffu32 as i32,
            0x8080_8080u32 as i32,
            0xfefd_fcfbu32 as i32,
        ],
        "C6",
    );
}

fn cfg_c7_single_high_bit_byte_swept_across_positions() {
    assert_same_many(
        &[
            0x0000_0080u32 as i32,
            0x0000_8000u32 as i32,
            0x0080_0000u32 as i32,
            0x8000_0000u32 as i32,
            0x0000_00ffu32 as i32,
            0x0000_ff00u32 as i32,
            0x00ff_0000u32 as i32,
            0xff00_0000u32 as i32,
        ],
        "C7",
    );
}

fn cfg_c8_single_small_nonzero_byte_swept_across_positions() {
    assert_same_many(
        &[0x0000_0001, 0x0000_0100, 0x0001_0000, 0x0100_0000],
        "C8",
    );
}

fn cfg_c9_value_range_boundaries() {
    assert_same_many(
        &[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, -1, 1, 0],
        "C9",
    );
}

fn cfg_c10_exhaustive_low_byte() {
    let vals: Vec<c_int> = (0..=255u32).map(|b| b as i32).collect();
    assert_same_many(&vals, "C10");
}

fn cfg_c11_exhaustive_single_byte_in_each_position() {
    let mut vals: Vec<c_int> = Vec::with_capacity(1024);
    for shift in [0u32, 8, 16, 24] {
        for b in 0..=255u32 {
            vals.push((b << shift) as i32);
        }
    }
    assert_same_many(&vals, "C11");
}

fn cfg_c12_random_full_range() {
    let mut rng = Rng::new(12);
    // 20000 captures would be slow (two temp files each); 4000 keeps the row
    // well under the time budget while still sweeping the value space.
    let vals: Vec<c_int> = (0..4000).map(|_| rng.next_i32()).collect();
    assert_same_many(&vals, "C12");
}

fn cfg_c13_random_small_positive() {
    let mut rng = Rng::new(13);
    let vals: Vec<c_int> = (0..2000).map(|_| (rng.next_u64() % 256) as i32).collect();
    assert_same_many(&vals, "C13");
}

fn cfg_c14_random_negative() {
    let mut rng = Rng::new(14);
    let vals: Vec<c_int> = (0..2000)
        .map(|_| {
            let v = rng.next_i32();
            if v > 0 {
                -v
            } else {
                v
            }
        })
        .collect();
    assert_same_many(&vals, "C14");
}

fn cfg_c15_random_bytes_from_edge_alphabet() {
    let mut rng = Rng::new(15);
    const ALPHABET: [u32; 6] = [0x00, 0x01, 0x0f, 0x7f, 0x80, 0xff];
    let vals: Vec<c_int> = (0..2000)
        .map(|_| {
            let mut v: u32 = 0;
            for i in 0..4 {
                let b = ALPHABET[(rng.next_u64() % ALPHABET.len() as u64) as usize];
                v |= b << (8 * i);
            }
            v as i32
        })
        .collect();
    assert_same_many(&vals, "C15");
}

fn cfg_c16_many_calls_in_one_capture() {
    let mut rng = Rng::new(16);
    let vals: Vec<c_int> = (0..2000).map(|_| rng.next_i32()).collect();

    let c = c_driver();
    let r = rust_driver();
    let out_c = capture_stdout(|| {
        for &x in &vals {
            unsafe { c(x) }
        }
    });
    let out_r = capture_stdout(|| {
        for &x in &vals {
            unsafe { r(x) }
        }
    });
    assert_eq!(out_c, out_r, "C16: batched-call stdout diverges");
    assert_eq!(
        out_c.iter().filter(|&&b| b == b'\n').count(),
        vals.len(),
        "C16: wrong number of lines"
    );
}

fn cfg_c17_single_call_exact_shape() {
    let (c, r) = both(0x1234_5678);
    assert_eq!(c, r, "C17");
    // 4 bytes * 2 hex digits + '\n'
    assert_eq!(c.len(), 2 * std::mem::size_of::<c_int>() + 1, "C17: length");
    assert!(
        c[..c.len() - 1]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
        "C17: non lowercase-hex byte in output: {:?}",
        String::from_utf8_lossy(&c)
    );
}

fn cfg_c18_interleaved_call_order() {
    let x: c_int = 0x0a0b_0c0d;
    let c = c_driver();
    let r = rust_driver();

    let single_c = capture_stdout(|| unsafe { c(x) });
    let single_r = capture_stdout(|| unsafe { r(x) });

    let cr = capture_stdout(|| unsafe {
        c(x);
        r(x);
    });
    let rc = capture_stdout(|| unsafe {
        r(x);
        c(x);
    });

    let mut expected = single_c.clone();
    expected.extend_from_slice(&single_r);
    assert_eq!(cr, expected, "C18: C-then-Rust interleaving diverges");
    assert_eq!(rc, expected, "C18: Rust-then-C interleaving diverges");
}

fn cfg_c19_repeated_invocation_is_stateless() {
    for &x in &[0, 1, -1, i32::MIN, i32::MAX, 0x0102_0304] {
        let (c1, r1) = both(x);
        let (c2, r2) = both(x);
        assert_eq!(c1, c2, "C19: C is not stateless for {x}");
        assert_eq!(r1, r2, "C19: Rust is not stateless for {x}");
        assert_eq!(c1, r1, "C19: divergence for {x}");
    }
}

// ===========================================================================
// Phase C — error/boundary-path differential tests, one per ERRORS.md row
// ===========================================================================

fn err_e1_int_min() {
    assert_same(i32::MIN, "E1");
}

fn err_e2_int_max() {
    assert_same(i32::MAX, "E2");
}

fn err_e3_minus_one_all_ones() {
    assert_same(-1, "E3");
}

fn err_e4_zero() {
    assert_same(0, "E4");
}

fn err_e5_high_bit_bytes_no_sign_extension() {
    // If either side promoted the byte as *signed* char, `%02x` would emit
    // `ffffff80` instead of `80`. Assert byte equality AND the exact width, so
    // a matching-but-wrong pair cannot slip through.
    for &x in &[
        0x8080_8080u32 as i32,
        0xffff_ffffu32 as i32,
        0x0000_0080u32 as i32,
        0x8000_0000u32 as i32,
    ] {
        let (c, r) = both(x);
        assert_eq!(c, r, "E5: divergence for 0x{x:08x}");
        assert_eq!(
            c.len(),
            2 * std::mem::size_of::<c_int>() + 1,
            "E5: sign-extension changed the output width for 0x{x:08x}: {:?}",
            String::from_utf8_lossy(&c)
        );
    }
}

/// Out-of-range argument: call through a *wider unsigned* signature so the
/// argument register carries garbage above bit 31. Both sides must observe only
/// the low 32 bits.
fn err_e6_oversized_unsigned_arg_abi_truncation() {
    type WideFn = unsafe extern "C" fn(u64);
    let c: Symbol<WideFn> = unsafe { libs().c.get(b"driver\0").unwrap() };
    let r: Symbol<WideFn> = unsafe { libs().rust.get(b"driver\0").unwrap() };

    for &wide in &[
        0xDEAD_BEEF_0000_0001u64,
        0xFFFF_FFFF_FFFF_FFFFu64,
        0x0000_0001_0000_0000u64,
        0x7FFF_FFFF_8000_0000u64,
    ] {
        let out_c = capture_stdout(|| unsafe { c(wide) });
        let out_r = capture_stdout(|| unsafe { r(wide) });
        assert_eq!(out_c, out_r, "E6: divergence for wide arg 0x{wide:016x}");

        // Truncation must agree with the narrow call on the low 32 bits.
        let narrow = wide as u32 as i32;
        let (nc, nr) = both(narrow);
        assert_eq!(out_c, nc, "E6: C did not truncate 0x{wide:016x} to 32 bits");
        assert_eq!(out_r, nr, "E6: Rust did not truncate 0x{wide:016x} to 32 bits");
    }
}

/// Same as E6 but through a *signed* 64-bit signature, including `i64::MIN`.
fn err_e7_oversized_signed_arg_abi_truncation() {
    type WideFn = unsafe extern "C" fn(i64);
    let c: Symbol<WideFn> = unsafe { libs().c.get(b"driver\0").unwrap() };
    let r: Symbol<WideFn> = unsafe { libs().rust.get(b"driver\0").unwrap() };

    for &wide in &[i64::MIN, i64::MAX, -1i64, 1i64 << 32, (1i64 << 32) | 0x7f] {
        let out_c = capture_stdout(|| unsafe { c(wide) });
        let out_r = capture_stdout(|| unsafe { r(wide) });
        assert_eq!(out_c, out_r, "E7: divergence for wide arg {wide}");

        let narrow = wide as i32;
        let (nc, nr) = both(narrow);
        assert_eq!(out_c, nc, "E7: C did not truncate {wide} to 32 bits");
        assert_eq!(out_r, nr, "E7: Rust did not truncate {wide} to 32 bits");
    }
}

/// E9: 1000 back-to-back calls must not corrupt state, drop a line, or emit
/// separators. (E8 — null pointer / zero or oversized length — is unreachable:
/// the public ABI exposes neither a pointer nor a length; see ERRORS.md.)
fn err_e9_repeated_calls_no_state_corruption() {
    let mut rng = Rng::new(9);
    let vals: Vec<c_int> = (0..1000).map(|_| rng.next_i32()).collect();

    let c = c_driver();
    let r = rust_driver();
    let out_c = capture_stdout(|| {
        for &x in &vals {
            unsafe { c(x) }
        }
    });
    let out_r = capture_stdout(|| {
        for &x in &vals {
            unsafe { r(x) }
        }
    });
    assert_eq!(out_c, out_r, "E9: divergence under repeated invocation");
    assert_eq!(
        out_c.len(),
        vals.len() * (2 * std::mem::size_of::<c_int>() + 1),
        "E9: unexpected total byte count (extra or missing bytes)"
    );
}

// ===========================================================================
// Sequential runner (harness = false)
// ===========================================================================

type Case = (&'static str, fn());

const CASES: &[Case] = &[
    // harness self-checks
    (
        "harness_loads_both_shared_objects_and_symbol_is_present",
        harness_loads_both_shared_objects_and_symbol_is_present,
    ),
    (
        "harness_capture_is_not_trivially_equal",
        harness_capture_is_not_trivially_equal,
    ),
    // Phase B — CONFIGS.md rows C1..C19
    ("cfg_c1_zero", cfg_c1_zero),
    ("cfg_c2_one_needs_zero_padding", cfg_c2_one_needs_zero_padding),
    (
        "cfg_c3_ascending_bytes_byte_order",
        cfg_c3_ascending_bytes_byte_order,
    ),
    (
        "cfg_c4_descending_bytes_byte_order",
        cfg_c4_descending_bytes_byte_order,
    ),
    ("cfg_c5_all_bytes_below_0x10", cfg_c5_all_bytes_below_0x10),
    ("cfg_c6_all_bytes_high_bit_set", cfg_c6_all_bytes_high_bit_set),
    (
        "cfg_c7_single_high_bit_byte_swept_across_positions",
        cfg_c7_single_high_bit_byte_swept_across_positions,
    ),
    (
        "cfg_c8_single_small_nonzero_byte_swept_across_positions",
        cfg_c8_single_small_nonzero_byte_swept_across_positions,
    ),
    ("cfg_c9_value_range_boundaries", cfg_c9_value_range_boundaries),
    ("cfg_c10_exhaustive_low_byte", cfg_c10_exhaustive_low_byte),
    (
        "cfg_c11_exhaustive_single_byte_in_each_position",
        cfg_c11_exhaustive_single_byte_in_each_position,
    ),
    ("cfg_c12_random_full_range", cfg_c12_random_full_range),
    ("cfg_c13_random_small_positive", cfg_c13_random_small_positive),
    ("cfg_c14_random_negative", cfg_c14_random_negative),
    (
        "cfg_c15_random_bytes_from_edge_alphabet",
        cfg_c15_random_bytes_from_edge_alphabet,
    ),
    (
        "cfg_c16_many_calls_in_one_capture",
        cfg_c16_many_calls_in_one_capture,
    ),
    ("cfg_c17_single_call_exact_shape", cfg_c17_single_call_exact_shape),
    ("cfg_c18_interleaved_call_order", cfg_c18_interleaved_call_order),
    (
        "cfg_c19_repeated_invocation_is_stateless",
        cfg_c19_repeated_invocation_is_stateless,
    ),
    // Phase C — ERRORS.md rows E1..E9 (E8 unreachable, see ERRORS.md)
    ("err_e1_int_min", err_e1_int_min),
    ("err_e2_int_max", err_e2_int_max),
    ("err_e3_minus_one_all_ones", err_e3_minus_one_all_ones),
    ("err_e4_zero", err_e4_zero),
    (
        "err_e5_high_bit_bytes_no_sign_extension",
        err_e5_high_bit_bytes_no_sign_extension,
    ),
    (
        "err_e6_oversized_unsigned_arg_abi_truncation",
        err_e6_oversized_unsigned_arg_abi_truncation,
    ),
    (
        "err_e7_oversized_signed_arg_abi_truncation",
        err_e7_oversized_signed_arg_abi_truncation,
    ),
    (
        "err_e9_repeated_calls_no_state_corruption",
        err_e9_repeated_calls_no_state_corruption,
    ),
];

fn main() {
    // Optional substring filter, mirroring libtest's CLI.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filters: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(|s| s.as_str())
        .collect();

    eprintln!("C   .so: {}", c_so_path().display());
    eprintln!("Rust.so: {}", rust_so_path().display());
    assert_rust_so_is_fresh();
    eprintln!("running {} differential cases sequentially", CASES.len());

    let mut passed = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    let mut skipped = 0usize;

    for (name, f) in CASES {
        if !filters.is_empty() && !filters.iter().any(|flt| name.contains(flt)) {
            skipped += 1;
            continue;
        }
        eprint!("test {name} ... ");
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => {
                eprintln!("ok");
                passed += 1;
            }
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    eprintln!(
        "\nresult: {}. {passed} passed; {} failed; {skipped} filtered out",
        if failed.is_empty() { "ok" } else { "FAILED" },
        failed.len()
    );
    if !failed.is_empty() {
        for name in &failed {
            eprintln!("  failed: {name}");
        }
        std::process::exit(1);
    }
}
