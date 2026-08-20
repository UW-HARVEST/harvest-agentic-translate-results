//! Differential tests: the C shared library vs. the Rust shared library.
//!
//! BOTH implementations are loaded with `libloading` and called only through
//! their exported `encode_base64` symbol. The Rust functions are never called
//! directly, so the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! Layout of the checks:
//!   * `cfgNN_*` — Phase B, one test per row of `CONFIGS.md`.
//!   * `errNN_*` — Phase C, one test per row of `ERRORS.md`.
//!
//! Every comparison checks
//!   1. NULL / non-NULL agreement (the library's only error signal), and
//!   2. the **entire** `size * 4 / 3 + 4` byte extent of the returned
//!      allocation, not merely the bytes up to the first NUL, so the
//!      `calloc()` zero-fill tail and the `'='` padding are compared too.

use libloading::{Library, Symbol};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

unsafe extern "C" {
    /// The C contract is "the caller must free the returned string"; both
    /// libraries allocate with the process-wide glibc `calloc`, so the
    /// process-wide `free` releases either one.
    fn free(ptr: *mut c_void);

    /// Used for a **one-sided** invariant only: the returned block must be big
    /// enough for the encoded output plus its terminating NUL.
    ///
    /// It deliberately is NOT used to compare the two allocations for equality:
    /// glibc does not always hand back an exact-fit chunk (it may serve a
    /// request from a larger free chunk when splitting is not profitable), so
    /// `malloc_usable_size` is not a function of the requested size and an
    /// equality assertion on it is flaky in both directions. Undersized
    /// allocations are instead caught by this lower bound plus the
    /// `MALLOC_CHECK_`/`MALLOC_PERTURB_` run documented in VERIFICATION.md.
    fn malloc_usable_size(ptr: *mut c_void) -> usize;
}

// ---------------------------------------------------------------------------
// Loading both shared objects
// ---------------------------------------------------------------------------

fn c_so_path() -> PathBuf {
    // Overridable so the same suite can be pointed at a C library built with
    // different compiler flags (see VERIFICATION.md).
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // current_exe() is target/<profile>/deps/<testname>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for cand in [profile.join("libdriver.so"), deps.join("libdriver.so")] {
        if cand.exists() {
            return cand;
        }
    }

    // `cargo test` does not always relink the `cdylib` target, so build it into
    // a private target directory (a separate directory ⇒ a separate cargo lock,
    // so this does not deadlock against the `cargo test` invocation that is
    // running us).
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-cdylib");
    let cargo = option_env!("CARGO").unwrap_or("cargo");
    let status = std::process::Command::new(cargo)
        .args(["build", "--lib"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CARGO_TARGET_DIR", &out_dir)
        .env_remove("RUSTFLAGS")
        .status();
    let built = out_dir.join("debug/libdriver.so");
    match status {
        Ok(s) if s.success() && built.exists() => built,
        other => panic!(
            "Rust cdylib libdriver.so not found next to {} and the fallback \
             `{cargo} build --lib` did not produce {} (status: {other:?}). \
             Run `cargo build` before `cargo test`.",
            exe.display(),
            built.display()
        ),
    }
}

struct Impls {
    c: EncodeBase64,
    rust: EncodeBase64,
}

fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library missing at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );

        // Leaked so the returned function pointers stay valid for 'static.
        let c_lib: &'static Library = Box::leak(Box::new(
            unsafe { Library::new(&c_path) }
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display())),
        ));
        let r_lib: &'static Library = Box::leak(Box::new(
            unsafe { Library::new(&r_path) }
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", r_path.display())),
        ));

        let c_sym: Symbol<EncodeBase64> = unsafe { c_lib.get(b"encode_base64\0") }
            .expect("C .so does not export encode_base64");
        let r_sym: Symbol<EncodeBase64> = unsafe { r_lib.get(b"encode_base64\0") }
            .expect("Rust .so does not export encode_base64");

        Impls {
            c: *c_sym,
            rust: *r_sym,
        }
    })
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// Mirrors the C allocation-size expression `size * 4 / 3 + 4`, evaluated in
/// `int` with two's-complement wrap-around and truncating-toward-zero division.
fn nbytes(effective_size: c_int) -> c_int {
    effective_size
        .wrapping_mul(4)
        .wrapping_div(3)
        .wrapping_add(4)
}

/// Upper bound on the `calloc` request a test is allowed to make.
///
/// Whether `calloc` succeeds for a multi-hundred-megabyte request depends on
/// the machine's overcommit configuration and its current memory pressure, so
/// "C and Rust agree" stops being a *deterministic* property up there. Sizes
/// whose wrapped `size*4/3+4` exceeds this bound are skipped by the randomized
/// sweeps; `wrap_positive_nbytes_*` covers the `int`-wrap code path with
/// magnitudes that stay under the bound.
const MAX_TESTABLE_NBYTES: i64 = 4 << 20; // 4 MiB

fn testable(size: c_int) -> bool {
    (nbytes(size) as i64) <= MAX_TESTABLE_NBYTES
}

/// `strlen` of a caller-supplied buffer (the buffer must contain a NUL).
fn c_strlen(buf: &[u8]) -> c_int {
    buf.iter()
        .position(|&b| b == 0)
        .expect("test buffer must be NUL-terminated when size == 0") as c_int
}

fn snapshot(p: *mut c_char, n: c_int) -> (Vec<u8>, Vec<u8>) {
    if n <= 0 {
        // calloc(1, 0) returns a non-NULL zero-length block: nothing readable.
        return (Vec::new(), Vec::new());
    }
    let full = unsafe { std::slice::from_raw_parts(p as *const u8, n as usize) }.to_vec();
    // Safe: the C never emits a NUL byte itself and the allocation always has
    // at least one zero-filled trailing byte, so a NUL is always in range.
    let as_cstr = unsafe { CStr::from_ptr(p) }.to_bytes().to_vec();
    (full, as_cstr)
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::new();
    for (i, b) in bytes.iter().enumerate() {
        if i == 64 {
            s.push_str("...");
            break;
        }
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Result of one differential call, once C and Rust have been shown to agree.
#[derive(Debug, PartialEq, Eq)]
enum Same {
    /// Both implementations returned NULL.
    Null,
    /// Both returned identical buffers: (full allocation extent, C string).
    Buf(Vec<u8>, Vec<u8>),
}

/// Call BOTH shared objects with the same arguments and assert byte-identical
/// results. `src == None` passes a NULL pointer.
#[track_caller]
fn diff(size: c_int, src: Option<&[u8]>) -> Same {
    let i = impls();

    let (ptr, effective) = match src {
        None => (std::ptr::null::<c_char>(), size),
        Some(b) => {
            let eff = if size == 0 { c_strlen(b) } else { size };
            if size > 0 {
                assert!(
                    (size as usize) <= b.len(),
                    "test bug: size {size} exceeds the {} readable bytes supplied",
                    b.len()
                );
            }
            (b.as_ptr() as *const c_char, eff)
        }
    };

    let cp = unsafe { (i.c)(size, ptr) };
    let rp = unsafe { (i.rust)(size, ptr) };

    let ctx = || {
        format!(
            "size={size} src={} effective_size={effective} nbytes={}",
            match src {
                None => "NULL".to_string(),
                Some(b) => format!("[{} bytes: {}]", b.len(), hex(b)),
            },
            nbytes(effective)
        )
    };

    if cp.is_null() != rp.is_null() {
        if !cp.is_null() {
            unsafe { free(cp as *mut c_void) };
        }
        if !rp.is_null() {
            unsafe { free(rp as *mut c_void) };
        }
        panic!(
            "NULL-ness divergence: C returned {}, Rust returned {} ({})",
            if cp.is_null() { "NULL" } else { "non-NULL" },
            if rp.is_null() { "NULL" } else { "non-NULL" },
            ctx()
        );
    }

    if cp.is_null() {
        return Same::Null;
    }

    let n = nbytes(effective);

    // Compare the SIZE of the two allocations, not only their contents.
    let c_usable = unsafe { malloc_usable_size(cp as *mut c_void) };
    let r_usable = unsafe { malloc_usable_size(rp as *mut c_void) };

    let (c_full, c_str) = snapshot(cp, n);
    let (r_full, r_str) = snapshot(rp, n);
    unsafe {
        free(cp as *mut c_void);
        free(rp as *mut c_void);
    }

    // One-sided invariant: whatever each side allocated, it must be able to
    // hold the bytes it actually produced plus the terminating NUL. A Rust
    // translation that under-allocated is caught here (and, for the writes
    // themselves, by the MALLOC_CHECK_ run).
    let needed = c_str.len() + 1;
    assert!(
        c_usable >= needed,
        "C block too small: {c_usable} usable bytes for {needed} needed ({})",
        ctx()
    );
    assert!(
        r_usable >= needed,
        "Rust block too small: {r_usable} usable bytes for {needed} needed — the \
         Rust translation under-allocated relative to `size * 4 / 3 + 4` ({})",
        ctx()
    );

    if c_full != r_full {
        let at = c_full
            .iter()
            .zip(r_full.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c_full.len().min(r_full.len()));
        panic!(
            "full-extent divergence at byte {at}: C={:02x?} Rust={:02x?}\n  C   = {}\n  Rust= {}\n  {}",
            c_full.get(at),
            r_full.get(at),
            hex(&c_full),
            hex(&r_full),
            ctx()
        );
    }
    assert_eq!(
        c_str,
        r_str,
        "C-string divergence:\n  C   = {:?}\n  Rust= {:?}\n  {}",
        String::from_utf8_lossy(&c_str),
        String::from_utf8_lossy(&r_str),
        ctx()
    );

    Same::Buf(c_full, c_str)
}

/// Assert both implementations rejected the input with the NULL sentinel.
#[track_caller]
fn diff_expect_null(size: c_int, src: Option<&[u8]>) {
    let got = diff(size, src);
    assert_eq!(
        got,
        Same::Null,
        "expected BOTH implementations to return the NULL sentinel for size={size}, \
         but they agreed on a non-NULL buffer instead"
    );
}

/// Assert both implementations accepted the input and produced an empty string.
#[track_caller]
fn diff_expect_empty(size: c_int, src: Option<&[u8]>) {
    match diff(size, src) {
        Same::Null => panic!(
            "expected BOTH implementations to return a non-NULL empty string for size={size}, \
             but both returned NULL"
        ),
        Same::Buf(full, cstr) => {
            assert!(cstr.is_empty(), "expected empty C string, got {cstr:?}");
            assert!(
                full.iter().all(|&b| b == 0),
                "expected an all-zero calloc buffer, got {}",
                hex(&full)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seeds keep failures reproducible
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform-ish in `lo..=hi`.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}

// ===========================================================================
// Phase B — valid-path differential tests (one per CONFIGS.md row)
// ===========================================================================

/// CONFIGS row 1: size > 0, size % 3 == 0 (no `'='` padding), random bytes.
#[test]
fn cfg01_size_mod3_0_random_bytes() {
    let mut rng = Rng::new(0x0000_0001_C0FF_EE01);
    for _ in 0..2000 {
        let groups = rng.range(1, 64) as usize;
        let size = groups * 3;
        let buf = rng.bytes(size);
        let out = diff(size as c_int, Some(&buf));
        // no padding at all when size % 3 == 0
        if let Same::Buf(_, s) = out {
            assert_eq!(s.len(), groups * 4);
            assert!(!s.contains(&b'='), "unexpected '=' padding for size {size}");
        }
    }
}

/// CONFIGS row 2: size > 0, size % 3 == 1 ⇒ two `'='` bytes.
#[test]
fn cfg02_size_mod3_1_random_bytes() {
    let mut rng = Rng::new(0x0000_0002_C0FF_EE02);
    for _ in 0..2000 {
        let size = (rng.range(0, 63) * 3 + 1) as usize;
        let buf = rng.bytes(size);
        if let Same::Buf(_, s) = diff(size as c_int, Some(&buf)) {
            assert_eq!(&s[s.len() - 2..], b"==", "size {size} must end with two '='");
        }
    }
}

/// CONFIGS row 3: size > 0, size % 3 == 2 ⇒ exactly one `'='` byte.
#[test]
fn cfg03_size_mod3_2_random_bytes() {
    let mut rng = Rng::new(0x0000_0003_C0FF_EE03);
    for _ in 0..2000 {
        let size = (rng.range(0, 63) * 3 + 2) as usize;
        let buf = rng.bytes(size);
        if let Same::Buf(_, s) = diff(size as c_int, Some(&buf)) {
            assert_eq!(s[s.len() - 1], b'=', "size {size} must end with one '='");
            assert_ne!(s[s.len() - 2], b'=', "size {size} must have only one '='");
        }
    }
}

/// CONFIGS row 4: the smallest non-empty sizes, exhaustive over byte values.
#[test]
fn cfg04_tiny_sizes_exhaustive() {
    // size == 1: all 256 byte values.
    for b in 0u16..=255 {
        let buf = [b as u8, 0, 0, 0];
        diff(1, Some(&buf));
    }
    // size == 2: all 65536 byte pairs.
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            let buf = [a as u8, b as u8, 0, 0];
            diff(2, Some(&buf));
        }
    }
    // size == 3: randomized triples (the full 2^24 space is covered
    // statistically here and structurally by cfg07).
    let mut rng = Rng::new(0x0000_0004_C0FF_EE04);
    for _ in 0..20_000 {
        let buf = rng.bytes(3);
        diff(3, Some(&buf));
    }
}

/// CONFIGS row 5: all-zero input ⇒ every 6-bit group is 0 ⇒ all `'A'`.
#[test]
fn cfg05_all_zero_bytes() {
    for size in 1..=200i32 {
        let buf = vec![0u8; size as usize];
        if let Same::Buf(_, s) = diff(size, Some(&buf)) {
            assert!(
                s.iter().all(|&c| c == b'A' || c == b'='),
                "all-zero input must encode to 'A'/'=' only, got {:?}",
                String::from_utf8_lossy(&s)
            );
        }
    }
}

/// CONFIGS row 6: all-0xFF input — exercises the signed `char` →
/// `unsigned char` conversion of `b1 = src[i]`.
///
/// When `size % 3 == 0` every 6-bit group is 63, so the output is all `'/'`.
/// When `size % 3 != 0` the C zero-fills `b2`/`b3`, so the last written group
/// is `(0xFF & 0x3) << 4 == 48` ⇒ `'w'` (e.g. `size == 1` ⇒ `"/w=="`); the
/// assertion below only constrains the fully populated groups.
#[test]
fn cfg06_all_ff_bytes() {
    for size in 1..=200i32 {
        let buf = vec![0xFFu8; size as usize];
        if let Same::Buf(_, s) = diff(size, Some(&buf)) {
            if size % 3 == 0 {
                assert!(
                    s.iter().all(|&c| c == b'/'),
                    "all-0xFF input of size {size} must encode to '/' only, got {:?}",
                    String::from_utf8_lossy(&s)
                );
            } else {
                // all complete 3-byte groups first, then the padded tail
                let complete = (size as usize / 3) * 4;
                assert!(
                    s[..complete].iter().all(|&c| c == b'/'),
                    "leading groups of an all-0xFF input must be '/', got {:?}",
                    String::from_utf8_lossy(&s)
                );
                let tail = &s[complete..];
                assert_eq!(tail[0], b'/', "b4 = 0xFF >> 2 = 63 ⇒ '/'");
                if size % 3 == 1 {
                    assert_eq!(tail, b"/w==", "b5 = (0xFF & 3) << 4 = 48 ⇒ 'w'");
                } else {
                    assert_eq!(tail, b"//8=", "b6 = (0xFF & 0xf) << 2 = 60 ⇒ '8'");
                }
            }
        }
    }
}

/// CONFIGS row 7: drive `encode()` through **every** one of its 64 inputs,
/// including the `u == 61` ('9'), `u == 62` ('+') and `u == 63` ('/') branches.
#[test]
fn cfg07_encode_alphabet_exhaustive() {
    for v in 0u8..64 {
        // 24-bit value whose four 6-bit groups all equal `v`.
        let b1 = (v << 2) | (v >> 4);
        let b2 = ((v & 0x0F) << 4) | (v >> 2);
        let b3 = ((v & 0x03) << 6) | v;
        let buf = [b1, b2, b3];
        match diff(3, Some(&buf)) {
            Same::Buf(_, s) => {
                assert_eq!(s.len(), 4);
                assert!(
                    s.iter().all(|&c| c == s[0]),
                    "expected 4 identical symbols for group value {v}, got {:?}",
                    String::from_utf8_lossy(&s)
                );
            }
            Same::Null => panic!("unexpected NULL for a 3-byte input"),
        }
    }
    // And the three top branches once more, isolated in the leading group.
    for b1 in [0xF4u8, 0xF8, 0xFC, 0xFF] {
        for extra in [0x00u8, 0x55, 0xAA, 0xFF] {
            let buf = [b1, extra, extra];
            diff(3, Some(&buf));
            diff(2, Some(&buf));
            diff(1, Some(&buf));
        }
    }
}

/// CONFIGS row 8: only high-bit bytes (negative `char` values).
#[test]
fn cfg08_high_bit_bytes_only() {
    let mut rng = Rng::new(0x0000_0008_C0FF_EE08);
    for _ in 0..1500 {
        let size = rng.range(1, 190) as usize;
        let buf: Vec<u8> = (0..size).map(|_| 0x80 | (rng.byte() & 0x7F)).collect();
        assert!(buf.iter().all(|&b| b >= 0x80));
        diff(size as c_int, Some(&buf));
    }
}

/// CONFIGS row 9: embedded NUL bytes — `size`, not the NUL, ends the loop.
#[test]
fn cfg09_embedded_nul_bytes() {
    let mut rng = Rng::new(0x0000_0009_C0FF_EE09);
    for _ in 0..1500 {
        let size = rng.range(1, 190) as usize;
        let mut buf = rng.bytes(size);
        // sprinkle NULs, including at index 0 sometimes
        let holes = rng.range(1, 5) as usize;
        for _ in 0..holes {
            let idx = rng.range(0, size as i64 - 1) as usize;
            buf[idx] = 0;
        }
        if rng.range(0, 3) == 0 {
            buf[0] = 0;
        }
        let out = diff(size as c_int, Some(&buf));
        if let Same::Buf(_, s) = out {
            // explicit size means the output length ignores the NULs entirely
            assert_eq!(s.len(), size.div_ceil(3) * 4);
        }
    }
}

/// CONFIGS row 10: size == 0 ⇒ the `strlen()` path.
#[test]
fn cfg10_size_zero_uses_strlen() {
    let mut rng = Rng::new(0x0000_000A_C0FF_EE0A);
    for _ in 0..2000 {
        let len = rng.range(1, 190) as usize;
        // NUL-free payload, then the terminator, then trailing garbage that
        // strlen() must not see.
        let mut buf: Vec<u8> = (0..len).map(|_| 1 + (rng.byte() % 255)).collect();
        assert!(buf.iter().all(|&b| b != 0));
        buf.push(0);
        buf.extend_from_slice(&rng.bytes(8));
        let out = diff(0, Some(&buf));
        if let Same::Buf(_, s) = out {
            assert_eq!(s.len(), len.div_ceil(3) * 4);
        }
        // The same payload with an explicit size must match the strlen result.
        diff(len as c_int, Some(&buf));
    }
}

/// CONFIGS row 11: size == 0 with an empty string.
#[test]
fn cfg11_size_zero_empty_string() {
    let buf = [0u8; 1];
    diff_expect_empty(0, Some(&buf));
    let buf2 = *b"\0";
    diff_expect_empty(0, Some(&buf2));
}

/// CONFIGS row 12: size == 0, leading NUL followed by garbage.
#[test]
fn cfg12_size_zero_leading_nul_then_garbage() {
    let mut rng = Rng::new(0x0000_000C_C0FF_EE0C);
    for _ in 0..500 {
        let mut buf = vec![0u8];
        let n = rng.range(1, 32) as usize;
        buf.extend_from_slice(&rng.bytes(n));
        diff_expect_empty(0, Some(&buf));
    }
}

/// CONFIGS row 13: explicit size **smaller** than strlen ⇒ truncated encode.
#[test]
fn cfg13_size_less_than_strlen() {
    let mut rng = Rng::new(0x0000_000D_C0FF_EE0D);
    for _ in 0..2000 {
        let len = rng.range(4, 190) as usize;
        let mut buf: Vec<u8> = (0..len).map(|_| 1 + (rng.byte() % 255)).collect();
        buf.push(0);
        let size = rng.range(1, len as i64 - 1) as c_int;
        if let Same::Buf(_, s) = diff(size, Some(&buf)) {
            assert_eq!(s.len(), (size as usize).div_ceil(3) * 4);
        }
    }
}

/// CONFIGS row 14: explicit size **larger** than strlen ⇒ reads past the NUL.
#[test]
fn cfg14_size_greater_than_strlen() {
    let mut rng = Rng::new(0x0000_000E_C0FF_EE0E);
    for _ in 0..2000 {
        let strlen_part = rng.range(0, 40) as usize;
        let mut buf: Vec<u8> = (0..strlen_part).map(|_| 1 + (rng.byte() % 255)).collect();
        buf.push(0); // the NUL the C will read straight through
        let tail = rng.range(1, 60) as usize;
        buf.extend_from_slice(&rng.bytes(tail));
        let size = buf.len() as c_int; // strictly greater than strlen
        assert!(size as usize > strlen_part);
        diff(size, Some(&buf));
    }
}

/// CONFIGS row 15: large inputs ⇒ many loop iterations, all residues.
#[test]
fn cfg15_large_buffers() {
    let mut rng = Rng::new(0x0000_000F_C0FF_EE0F);
    for _ in 0..120 {
        let size = rng.range(1000, 4096) as usize;
        let buf = rng.bytes(size);
        diff(size as c_int, Some(&buf));
    }
    // A few much larger ones, still far below the 2^29 int-overflow threshold.
    for size in [65_536usize, 100_003, 262_144] {
        let buf = rng.bytes(size);
        diff(size as c_int, Some(&buf));
    }
}

/// CONFIGS row 16: exhaustive size sweep 1..=256 with randomized payloads.
#[test]
fn cfg16_size_sweep_1_to_256() {
    let mut rng = Rng::new(0x0000_0010_C0FF_EE10);
    for size in 1..=256i32 {
        for _ in 0..8 {
            let buf = rng.bytes(size as usize);
            diff(size, Some(&buf));
        }
        diff(size, Some(&vec![0u8; size as usize]));
        diff(size, Some(&vec![0xFFu8; size as usize]));
        diff(size, Some(&vec![0x55u8; size as usize]));
        diff(size, Some(&vec![0xAAu8; size as usize]));
    }
}

/// CONFIGS row 17: size in {-1,-2,-3} ⇒ calloc still SUCCEEDS, loop skipped.
#[test]
fn cfg17_small_negative_sizes_succeed() {
    let buf = *b"hello world\0";
    for size in [-1i32, -2, -3] {
        assert!(nbytes(size) >= 0, "nbytes({size}) = {}", nbytes(size));
        diff_expect_empty(size, Some(&buf));
    }
}

/// CONFIGS row 18: size == i32::MIN ⇒ `size*4` wraps to 0 ⇒ nbytes == 4.
#[test]
fn cfg18_size_int_min() {
    let buf = *b"payload\0";
    assert_eq!(nbytes(i32::MIN), 4);
    diff_expect_empty(i32::MIN, Some(&buf));
}

/// CONFIGS row 19: sizes just above i32::MIN ⇒ wrap to small positives.
#[test]
fn cfg19_sizes_near_int_min() {
    let buf = *b"payload\0";
    for k in 0..=64i64 {
        let size = (i32::MIN as i64 + k) as i32;
        if nbytes(size) >= 0 {
            diff_expect_empty(size, Some(&buf));
        } else {
            diff_expect_null(size, Some(&buf));
        }
    }
}

/// CONFIGS row 20: randomized negative sizes over the whole negative range.
#[test]
fn cfg20_random_negative_sizes() {
    let mut rng = Rng::new(0x0000_0014_C0FF_EE14);
    let buf = *b"some data here\0";
    let mut tested = 0usize;
    for _ in 0..4000 {
        let size = rng.range(i32::MIN as i64, -1) as i32;
        if !testable(size) {
            continue;
        }
        tested += 1;
        // Both must agree; whether that is NULL or an empty string is decided
        // purely by the sign of the wrapped `size * 4 / 3 + 4`.
        match diff(size, Some(&buf)) {
            Same::Null => assert!(
                nbytes(size) < 0,
                "size {size} gave NULL but nbytes = {}",
                nbytes(size)
            ),
            Same::Buf(full, s) => {
                assert!(
                    nbytes(size) >= 0,
                    "size {size} gave a buffer but nbytes = {}",
                    nbytes(size)
                );
                assert!(s.is_empty());
                assert!(full.iter().all(|&b| b == 0));
            }
        }
    }
    assert!(
        tested > 1000,
        "too few negative sizes were actually exercised: {tested}"
    );
}

/// CONFIGS row 19/20 companion: the `int`-wrap path where `size * 4` overflows
/// to a **positive** value, so `calloc` succeeds even though `size` is a huge
/// negative number. Magnitudes are kept under `MAX_TESTABLE_NBYTES`.
#[test]
fn wrap_positive_nbytes_from_huge_negative_sizes() {
    let buf = *b"payload for the wrap cases\0";
    let mut ks: Vec<i64> = vec![1, 2, 3, 4, 5, 7, 8, 9, 100, 1000, 65_536];
    let mut k = 1i64;
    while k < 3_000_000 {
        ks.push(k);
        k = k * 7 / 3 + 1;
    }
    let mut rng = Rng::new(0x0000_0016_C0FF_EE16);
    for _ in 0..200 {
        ks.push(rng.range(1, 3_000_000));
    }
    let mut wrapped = 0usize;
    for k in ks {
        let size = (i32::MIN as i64 + k) as i32;
        if !testable(size) {
            continue;
        }
        // `size` is hugely negative, yet `size * 4` wrapped positive.
        assert!(size < -2_000_000_000);
        if nbytes(size) >= 0 {
            wrapped += 1;
            diff_expect_empty(size, Some(&buf));
        } else {
            diff_expect_null(size, Some(&buf));
        }
    }
    assert!(wrapped > 20, "wrap path barely exercised: {wrapped}");
}

/// CONFIGS row 21: the full `size*4/3+4` extent, including the zero-filled
/// tail, is identical — verified explicitly here (and implicitly everywhere
/// else, since `diff()` always compares the whole extent).
#[test]
fn cfg21_full_allocation_extent_matches() {
    let mut rng = Rng::new(0x0000_0015_C0FF_EE15);
    for size in 1..=120i32 {
        let buf = rng.bytes(size as usize);
        match diff(size, Some(&buf)) {
            Same::Buf(full, s) => {
                let n = nbytes(size) as usize;
                assert_eq!(full.len(), n, "extent length for size {size}");
                let out_len = (size as usize).div_ceil(3) * 4;
                assert_eq!(s.len(), out_len);
                // Everything after the encoded output must be calloc's zeros.
                assert!(
                    full[out_len..].iter().all(|&b| b == 0),
                    "tail of the allocation is not zero-filled for size {size}: {}",
                    hex(&full[out_len..])
                );
                assert!(n > out_len, "C buffer must have room for the NUL");
            }
            Same::Null => panic!("unexpected NULL for size {size}"),
        }
    }
}

// ===========================================================================
// Phase C — error-path differential tests (one per ERRORS.md row)
// ===========================================================================

/// ERRORS row 1: src == NULL, size == 0 (`if (!src) return NULL;`).
#[test]
fn err01_null_src_size_zero() {
    diff_expect_null(0, None);
}

/// ERRORS row 2: src == NULL, size > 0.
#[test]
fn err02_null_src_positive_size() {
    for size in [1i32, 2, 3, 4, 5, 6, 7, 63, 64, 65, 1024, 4096, 1 << 20] {
        diff_expect_null(size, None);
    }
    let mut rng = Rng::new(0x0000_0002_E770_0002);
    for _ in 0..2000 {
        let size = rng.range(1, i32::MAX as i64) as i32;
        diff_expect_null(size, None);
    }
}

/// ERRORS row 3: src == NULL, size < 0.
#[test]
fn err03_null_src_negative_size() {
    for size in [-1i32, -2, -3, -4, -5, -6, -7, -100, -1024, -(1 << 20)] {
        diff_expect_null(size, None);
    }
    let mut rng = Rng::new(0x0000_0003_E770_0003);
    for _ in 0..2000 {
        let size = rng.range(i32::MIN as i64, -1) as i32;
        diff_expect_null(size, None);
    }
}

/// ERRORS row 4: src == NULL with the extreme sizes — the NULL check
/// short-circuits before any of the overflowing arithmetic runs.
#[test]
fn err04_null_src_extreme_sizes() {
    for size in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        diff_expect_null(size, None);
    }
}

/// ERRORS row 5: size == -4 ⇒ nbytes == -1 ⇒ calloc(1, SIZE_MAX) fails.
#[test]
fn err05_size_minus4_calloc_fails() {
    let buf = *b"data\0";
    assert_eq!(nbytes(-4), -1);
    diff_expect_null(-4, Some(&buf));
}

/// ERRORS row 6: size == -5 ⇒ nbytes == -2 ⇒ calloc fails.
#[test]
fn err06_size_minus5_calloc_fails() {
    let buf = *b"data\0";
    assert_eq!(nbytes(-5), -2);
    diff_expect_null(-5, Some(&buf));
}

/// ERRORS row 7: size == -6 ⇒ nbytes == -4 ⇒ calloc fails.
#[test]
fn err07_size_minus6_calloc_fails() {
    let buf = *b"data\0";
    assert_eq!(nbytes(-6), -4);
    diff_expect_null(-6, Some(&buf));
}

/// ERRORS row 8: size == -7 ⇒ nbytes == -5 ⇒ calloc fails.
#[test]
fn err08_size_minus7_calloc_fails() {
    let buf = *b"data\0";
    assert_eq!(nbytes(-7), -5);
    diff_expect_null(-7, Some(&buf));
}

/// ERRORS row 9: size == -100 ⇒ nbytes == -129 ⇒ calloc fails.
#[test]
fn err09_size_minus100_calloc_fails() {
    let buf = *b"data\0";
    assert_eq!(nbytes(-100), -129);
    diff_expect_null(-100, Some(&buf));
}

/// ERRORS row 10: randomized negative sizes with no `int` wrap ⇒ always NULL.
#[test]
fn err10_random_negative_sizes_reject() {
    let mut rng = Rng::new(0x0000_000A_E770_000A);
    let buf = *b"a longer payload buffer\0";
    for _ in 0..4000 {
        let size = rng.range(-536_870_000, -4) as i32;
        assert!(nbytes(size) < 0, "test bug: nbytes({size}) >= 0");
        diff_expect_null(size, Some(&buf));
    }
    // walk the exact rejection boundary densely
    for size in -4000..=-4i32 {
        diff_expect_null(size, Some(&buf));
    }
}

/// ERRORS row 11: size == -3 ⇒ nbytes == 0 ⇒ `calloc(1, 0)` ⇒ NOT rejected.
#[test]
fn err11_size_minus3_zero_alloc_not_rejected() {
    let buf = *b"data\0";
    assert_eq!(nbytes(-3), 0);
    diff_expect_empty(-3, Some(&buf));
}

/// ERRORS row 12: size == -1 (one step past the valid range) ⇒ NOT rejected.
#[test]
fn err12_size_minus1_not_rejected() {
    let buf = *b"data\0";
    assert_eq!(nbytes(-1), 3);
    diff_expect_empty(-1, Some(&buf));
}

/// ERRORS row 13: size == -2 ⇒ NOT rejected.
#[test]
fn err13_size_minus2_not_rejected() {
    let buf = *b"data\0";
    assert_eq!(nbytes(-2), 2);
    diff_expect_empty(-2, Some(&buf));
}

/// ERRORS row 14: size == i32::MIN ⇒ wraps to nbytes == 4 ⇒ NOT rejected.
#[test]
fn err14_size_int_min_not_rejected() {
    let buf = *b"data\0";
    assert_eq!(nbytes(i32::MIN), 4);
    diff_expect_empty(i32::MIN, Some(&buf));
}

/// ERRORS row 15: i32::MIN+1 ..= i32::MIN+8 ⇒ wrap positive ⇒ NOT rejected.
#[test]
fn err15_sizes_just_above_int_min_not_rejected() {
    let buf = *b"data\0";
    for k in 1..=8i64 {
        let size = (i32::MIN as i64 + k) as i32;
        assert!(nbytes(size) >= 0, "nbytes({size}) = {}", nbytes(size));
        diff_expect_empty(size, Some(&buf));
    }
}

/// ERRORS row 16: zero length — size == 0 with an empty string ⇒ NOT rejected.
#[test]
fn err16_zero_length_not_rejected() {
    let buf = [0u8; 4];
    assert_eq!(nbytes(0), 4);
    diff_expect_empty(0, Some(&buf));
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep: the entire i32 domain of the only scalar
// parameter, exercised at every representable-boundary neighbourhood. This is
// the stand-in for "out-of-range enum values", since the API has no enums.
// ---------------------------------------------------------------------------

/// Every `size` around each interesting boundary, with a valid `src`, limited
/// to the values the C can evaluate without invoking out-of-bounds writes
/// (i.e. `size <= 0`, plus small positives).
#[test]
fn boundary_size_domain_sweep_with_valid_src() {
    let payload: Vec<u8> = {
        let mut v: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
        v[0] = 1; // keep a NUL out of index 0 so strlen() is non-zero
        v.push(0);
        v
    };

    let mut sizes: Vec<i32> = Vec::new();
    // negative / zero region: fully evaluable by the C (loop never runs)
    for anchor in [i32::MIN, i32::MIN + 1, -536_870_912, -1_000_000, -1000, 0] {
        for d in -8i64..=8 {
            let v = anchor as i64 + d;
            if (i32::MIN as i64..=0).contains(&v) {
                sizes.push(v as i32);
            }
        }
    }
    // small positives, always within the supplied payload
    for s in 1..=(payload.len() as i32) {
        sizes.push(s);
    }
    sizes.sort_unstable();
    sizes.dedup();

    for size in sizes {
        // A NULL src is rejected before any arithmetic, so every size is safe.
        diff_expect_null(size, None);
        // With a valid src the allocation actually happens, so skip the
        // wrapped sizes whose request is too big to be deterministic.
        if testable(size) {
            diff(size, Some(&payload));
        }
    }
}

/// The documented-range edges, checked as a set so a regression in any single
/// one is reported with its neighbours.
#[test]
fn boundary_null_vs_nonnull_agreement_table() {
    let buf = *b"0123456789abcdef\0";
    let cases: [(i32, bool); 12] = [
        // (size, expect_null_when_src_valid)
        (i32::MIN, false),
        (i32::MIN + 1, false),
        (-536_870_912, true),
        (-1_000_000, true),
        (-8, true),
        (-7, true),
        (-6, true),
        (-5, true),
        (-4, true),
        (-3, false),
        (-2, false),
        (-1, false),
    ];
    for (size, expect_null) in cases {
        if expect_null {
            diff_expect_null(size, Some(&buf));
        } else {
            diff_expect_empty(size, Some(&buf));
        }
        // NULL src always wins, whatever the size
        diff_expect_null(size, None);
    }
    // size 0 and small positives never reject
    for size in [0i32, 1, 2, 3, 16] {
        match diff(size, Some(&buf)) {
            Same::Buf(..) => {}
            Same::Null => panic!("unexpected rejection for size {size}"),
        }
    }
}
