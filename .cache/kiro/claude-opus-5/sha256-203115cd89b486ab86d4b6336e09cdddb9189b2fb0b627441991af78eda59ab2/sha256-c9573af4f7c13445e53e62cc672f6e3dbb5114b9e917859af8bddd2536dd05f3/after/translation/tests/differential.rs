//! Differential tests: load BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compare their observable behaviour through the FFI
//! boundary. The Rust implementation is never called directly — only through
//! its `#[no_mangle]` export, exactly as an external C consumer would.
//!
//! Row IDs in the test names refer to `CONFIGS.md` (Phase B) and `ERRORS.md`
//! (Phase C).

use std::ffi::c_void;
use std::os::raw::c_char;
use std::path::PathBuf;

use libloading::{Library, Symbol};

extern "C" {
    fn free(ptr: *mut c_void);
}

/// Fixed seed so every randomized row is reproducible.
pub const SEED: u64 = 0x5DEE_CE66_D0D0_F00D;

type CreateLinePointers =
    unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

struct Impls {
    _c_lib: Library,
    _r_lib: Library,
    c: CreateLinePointers,
    r: CreateLinePointers,
}

impl Impls {
    fn load() -> Impls {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C shared library not found at {cp:?}");
        assert!(rp.exists(), "Rust shared library not found at {rp:?}");
        unsafe {
            let c_lib = Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}"));
            let r_lib = Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}"));
            let c_sym: Symbol<CreateLinePointers> = c_lib
                .get(b"UTIL_createLinePointers\0")
                .expect("C .so is missing UTIL_createLinePointers");
            let r_sym: Symbol<CreateLinePointers> = r_lib
                .get(b"UTIL_createLinePointers\0")
                .expect("Rust .so is missing UTIL_createLinePointers");
            let c = *c_sym;
            let r = *r_sym;
            Impls {
                _c_lib: c_lib,
                _r_lib: r_lib,
                c,
                r,
            }
        }
    }
}

fn impls() -> &'static Impls {
    use std::sync::OnceLock;
    static ONCE: OnceLock<Impls> = OnceLock::new();
    ONCE.get_or_init(Impls::load)
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// Outcome of one call, in a form that can be compared byte-for-byte.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The function rejected the input (returned NULL).
    Null,
    /// Success. Holds the `read_back` pointer values from the returned array,
    /// expressed as raw addresses.
    Ok(Vec<usize>),
    /// Success, but the caller asked us not to read the array (used when
    /// `numLines` is astronomically large).
    OkOpaque,
}

unsafe fn invoke(
    f: CreateLinePointers,
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
    read_back: Option<usize>,
) -> Outcome {
    let ret = f(buffer, num_lines, buffer_size);
    if ret.is_null() {
        return Outcome::Null;
    }
    let outcome = match read_back {
        None => Outcome::OkOpaque,
        Some(n) => {
            let mut v = Vec::with_capacity(n);
            for i in 0..n {
                v.push(*ret.add(i) as usize);
            }
            Outcome::Ok(v)
        }
    };
    // The C `free`s on its own error path only; on success the array is owned
    // by us. Both implementations allocate with libc `malloc`.
    free(ret as *mut c_void);
    outcome
}

/// Run one configuration through both implementations and assert equality.
///
/// `buf` is passed to both by the *same* address, so the pointer values stored
/// in the returned arrays must be bit-identical, not merely equivalent.
#[track_caller]
fn diff(label: &str, buf: &mut [u8], num_lines: usize, buffer_size: usize) {
    assert!(
        buffer_size <= buf.len() || num_lines == 0,
        "{label}: test would read out of bounds"
    );
    let ptr = buf.as_mut_ptr() as *mut c_char;
    let read_back = Some(num_lines);
    let (c_out, r_out) = unsafe {
        let c = invoke(impls().c, ptr, num_lines, buffer_size, read_back);
        let r = invoke(impls().r, ptr, num_lines, buffer_size, read_back);
        (c, r)
    };
    assert_eq!(
        c_out, r_out,
        "{label}: divergence for numLines={num_lines} bufferSize={buffer_size} buf={:?}",
        &buf[..buffer_size.min(buf.len()).min(96)]
    );
}

/// Like [`diff`] but does not read the returned array (for absurd `numLines`)
/// and allows a null / short `buffer`, because the configuration provably never
/// dereferences it.
#[track_caller]
#[allow(dead_code)]
fn diff_raw(label: &str, ptr: *mut c_char, num_lines: usize, buffer_size: usize) {
    let (c_out, r_out) = unsafe {
        let c = invoke(impls().c, ptr, num_lines, buffer_size, None);
        let r = invoke(impls().r, ptr, num_lines, buffer_size, None);
        (c, r)
    };
    assert_eq!(
        c_out, r_out,
        "{label}: divergence for numLines={num_lines} bufferSize={buffer_size}"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(stream: u64) -> Rng {
        Rng(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// A byte guaranteed non-NUL, drawn from the full 0x01..=0xFF range so that
    /// signed-`char` handling is exercised.
    fn nonzero_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
}

/// Build a buffer of `n` NUL-terminated records with random lengths in
/// `len_range`. Returns (bytes, total_len_including_final_nul).
fn records(rng: &mut Rng, n: usize, min_len: usize, max_len: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..n {
        let len = if max_len == 0 {
            0
        } else {
            rng.range(min_len, max_len)
        };
        for _ in 0..len {
            v.push(rng.nonzero_byte());
        }
        v.push(0);
    }
    v
}

/// Reference model of the C loop: returns the offsets the C would store, or
/// `None` if the C would reject. Used only to sanity-check that a generated
/// case actually lands on the intended branch.
fn model(buf: &[u8], num_lines: usize, buffer_size: usize) -> Option<Vec<usize>> {
    let mut line_index = 0usize;
    let mut pos = 0usize;
    let mut out = Vec::new();
    while line_index < num_lines && pos < buffer_size {
        let mut len = 0usize;
        out.push(pos);
        line_index += 1;
        while pos + len < buffer_size && buf[pos + len] != 0 {
            len += 1;
        }
        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }
    if line_index != num_lines {
        None
    } else {
        Some(out)
    }
}

// ===========================================================================
// PHASE B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

/// C1: numLines = 0, bufferSize = 0, valid non-empty allocation.
#[test]
fn cfg_c1_zero_lines_zero_size() {
    let mut buf = vec![0u8; 32];
    diff("C1", &mut buf, 0, 0);
    // Also assert the *success* nature explicitly: malloc(0) is non-NULL on
    // glibc, so both must return a non-NULL zero-element array.
    let ptr = buf.as_mut_ptr() as *mut c_char;
    unsafe {
        assert_eq!(invoke(impls().c, ptr, 0, 0, Some(0)), Outcome::Ok(vec![]));
        assert_eq!(invoke(impls().r, ptr, 0, 0, Some(0)), Outcome::Ok(vec![]));
    }
}

/// C2: numLines = 0 with a random non-empty buffer — nothing may be read.
#[test]
fn cfg_c2_zero_lines_random_buffer() {
    let mut rng = Rng::new(2);
    for _ in 0..512 {
        let n = rng.range(1, 64);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let size = rng.range(1, n);
        diff("C2", &mut buf, 0, size);
    }
}

/// C3: single empty record, bufferSize = 1 (`"\0"`), NUL-skip NOT taken.
#[test]
fn cfg_c3_single_empty_record() {
    let mut buf = vec![0u8; 1];
    assert_eq!(model(&buf, 1, 1), Some(vec![0]), "C3 branch precondition");
    diff("C3", &mut buf, 1, 1);
}

/// C4: single unterminated record truncated by bufferSize = 1.
#[test]
fn cfg_c4_single_unterminated_byte() {
    let mut rng = Rng::new(4);
    for _ in 0..255 {
        let mut buf = vec![rng.nonzero_byte()];
        assert_eq!(model(&buf, 1, 1), Some(vec![0]), "C4 branch precondition");
        diff("C4", &mut buf, 1, 1);
    }
}

/// C5: one NUL-terminated record, bufferSize = len + 1 (NUL included).
#[test]
fn cfg_c5_one_record_exact_with_nul() {
    let mut rng = Rng::new(5);
    for _ in 0..512 {
        let mut buf = records(&mut rng, 1, 0, 24);
        let size = buf.len();
        assert!(model(&buf, 1, size).is_some(), "C5 branch precondition");
        diff("C5", &mut buf, 1, size);
    }
}

/// C6: one record, bufferSize excludes the trailing NUL.
#[test]
fn cfg_c6_one_record_nul_excluded() {
    let mut rng = Rng::new(6);
    for _ in 0..512 {
        let mut buf = records(&mut rng, 1, 1, 24);
        let size = buf.len() - 1; // drop the NUL
        assert!(model(&buf, 1, size).is_some(), "C6 branch precondition");
        diff("C6", &mut buf, 1, size);
    }
}

/// C7: exactly N NUL-terminated records, bufferSize = exact total.
#[test]
fn cfg_c7_n_records_exact() {
    let mut rng = Rng::new(7);
    for _ in 0..512 {
        let n = rng.range(2, 16);
        let mut buf = records(&mut rng, n, 0, 12);
        let size = buf.len();
        assert!(model(&buf, n, size).is_some(), "C7 branch precondition");
        diff("C7", &mut buf, n, size);
    }
}

/// C8: N records, final one unterminated (bufferSize excludes the last NUL).
#[test]
fn cfg_c8_n_records_last_unterminated() {
    let mut rng = Rng::new(8);
    for _ in 0..512 {
        let n = rng.range(2, 16);
        let mut buf = records(&mut rng, n, 1, 12);
        let size = buf.len() - 1;
        assert!(model(&buf, n, size).is_some(), "C8 branch precondition");
        diff("C8", &mut buf, n, size);
    }
}

/// C9: more records available than requested — trailing bytes ignored.
#[test]
fn cfg_c9_fewer_lines_than_records() {
    let mut rng = Rng::new(9);
    for _ in 0..512 {
        let available = rng.range(3, 20);
        let want = rng.range(1, available - 1);
        let mut buf = records(&mut rng, available, 0, 10);
        let size = buf.len();
        assert!(model(&buf, want, size).is_some(), "C9 branch precondition");
        diff("C9", &mut buf, want, size);
    }
}

/// C10: all-empty records (buffer of NULs) — every iteration has len == 0.
#[test]
fn cfg_c10_all_empty_records() {
    let mut rng = Rng::new(10);
    for _ in 0..256 {
        let n = rng.range(1, 24);
        let mut buf = vec![0u8; n];
        // asking for exactly n records succeeds; the loop consumes 1 byte each
        assert!(model(&buf, n, n).is_some(), "C10 branch precondition");
        diff("C10", &mut buf, n, n);
        // and asking for fewer also succeeds
        let fewer = rng.range(1, n);
        diff("C10b", &mut buf, fewer, n);
    }
}

/// C11: leading NUL (first record empty), rest non-empty.
#[test]
fn cfg_c11_leading_nul() {
    let mut rng = Rng::new(11);
    for _ in 0..512 {
        let n = rng.range(2, 12);
        let mut buf = vec![0u8];
        buf.extend_from_slice(&records(&mut rng, n - 1, 1, 8));
        let size = buf.len();
        assert!(model(&buf, n, size).is_some(), "C11 branch precondition");
        diff("C11", &mut buf, n, size);
    }
}

/// C12: random runs of consecutive NULs interleaved with random records.
#[test]
fn cfg_c12_mixed_empty_and_nonempty() {
    let mut rng = Rng::new(12);
    for _ in 0..1024 {
        let n = rng.range(1, 20);
        let mut buf = Vec::new();
        for _ in 0..n {
            if rng.below(3) == 0 {
                // a run of empty records
                let runs = rng.range(1, 3);
                for _ in 0..runs {
                    buf.push(0u8);
                }
            } else {
                let len = rng.range(1, 9);
                for _ in 0..len {
                    buf.push(rng.nonzero_byte());
                }
                buf.push(0);
            }
        }
        let size = buf.len();
        diff("C12", &mut buf, n, size);
    }
}

/// C13: record bytes across the whole 0x01..=0xFF range (signed-char check).
#[test]
fn cfg_c13_high_byte_values() {
    let mut rng = Rng::new(13);
    // Deterministic sweep: every possible non-NUL byte as a 1-byte record.
    for b in 1u16..=255 {
        let mut buf = vec![b as u8, 0u8];
        diff("C13-sweep", &mut buf, 1, 2);
        diff("C13-sweep-notrail", &mut buf, 1, 1);
    }
    // Randomized: records made only of high (negative-as-signed) bytes.
    for _ in 0..512 {
        let n = rng.range(1, 10);
        let mut buf = Vec::new();
        for _ in 0..n {
            let len = rng.range(1, 6);
            for _ in 0..len {
                buf.push(0x80 | (rng.next_u64() as u8 & 0x7f));
            }
            buf.push(0);
        }
        let size = buf.len();
        diff("C13", &mut buf, n, size);
    }
}

/// C14: bufferSize truncates mid-record at a random offset. Depending on where
/// the cut lands this either succeeds or returns NULL; both must agree.
#[test]
fn cfg_c14_truncated_mid_record() {
    let mut rng = Rng::new(14);
    let mut saw_ok = false;
    let mut saw_null = false;
    for _ in 0..2048 {
        let n = rng.range(2, 12);
        let mut buf = records(&mut rng, n, 1, 10);
        let size = rng.range(1, buf.len());
        match model(&buf, n, size) {
            Some(_) => saw_ok = true,
            None => saw_null = true,
        }
        diff("C14", &mut buf, n, size);
    }
    assert!(saw_ok && saw_null, "C14 must cover both verdicts");
}

/// C15: unpruned fuzz over the full axis cross-product.
#[test]
fn cfg_c15_full_fuzz() {
    let mut rng = Rng::new(15);
    for _ in 0..20_000 {
        let cap = rng.range(1, 64);
        let nul_pct = rng.below(101);
        let mut buf: Vec<u8> = (0..cap)
            .map(|_| {
                if rng.below(100) < nul_pct {
                    0u8
                } else {
                    rng.nonzero_byte()
                }
            })
            .collect();
        let size = rng.below(cap + 1); // 0..=cap
        let num_lines = rng.below(21); // 0..=20
        diff("C15", &mut buf, num_lines, size);
    }
}

/// C16: one long unterminated record in a large buffer.
#[test]
fn cfg_c16_long_unterminated() {
    let mut rng = Rng::new(16);
    let mut buf: Vec<u8> = (0..4096).map(|_| rng.nonzero_byte()).collect();
    diff("C16", &mut buf, 1, 4096);
    diff("C16b", &mut buf, 1, 1);
    diff("C16c", &mut buf, 1, 4095);
}

/// C17: large numLines with exactly that many records.
#[test]
fn cfg_c17_many_records() {
    let mut rng = Rng::new(17);
    for &n in &[256usize, 1024] {
        let mut buf = Vec::with_capacity(n * 2);
        for _ in 0..n {
            buf.push(rng.nonzero_byte());
            buf.push(0);
        }
        let size = buf.len();
        assert!(model(&buf, n, size).is_some(), "C17 branch precondition");
        diff("C17", &mut buf, n, size);
        // exact-fit minus the trailing NUL still succeeds
        diff("C17b", &mut buf, n, size - 1);
    }
}

/// C18: bufferSize larger than the bytes the N records need; trailing garbage
/// after the N-th record is never read because the loop exits first.
#[test]
fn cfg_c18_trailing_garbage_not_read() {
    let mut rng = Rng::new(18);
    for _ in 0..512 {
        let n = rng.range(1, 12);
        let mut buf = records(&mut rng, n, 1, 8);
        let needed = buf.len();
        // append garbage that must never be touched
        let extra = rng.range(1, 32);
        for _ in 0..extra {
            buf.push(rng.byte());
        }
        let size = buf.len();
        let m = model(&buf, n, size).expect("C18 branch precondition");
        assert_eq!(m.len(), n);
        assert!(needed <= size);
        diff("C18", &mut buf, n, size);
    }
}

// ===========================================================================
// PHASE C — error-path differential tests, one per ERRORS.md row
// ===========================================================================

/// Assert both implementations return exactly NULL (the library's only error
/// sentinel — there is no errno/enum channel in this API).
#[track_caller]
fn assert_both_null(label: &str, ptr: *mut c_char, num_lines: usize, buffer_size: usize) {
    let (c_out, r_out) = unsafe {
        let c = invoke(impls().c, ptr, num_lines, buffer_size, None);
        let r = invoke(impls().r, ptr, num_lines, buffer_size, None);
        (c, r)
    };
    assert_eq!(c_out, r_out, "{label}: C and Rust disagree");
    assert_eq!(
        c_out,
        Outcome::Null,
        "{label}: expected the C to return NULL for \
         numLines={num_lines} bufferSize={buffer_size}"
    );
}

#[track_caller]
fn assert_both_non_null(label: &str, ptr: *mut c_char, num_lines: usize, buffer_size: usize) {
    let (c_out, r_out) = unsafe {
        let c = invoke(impls().c, ptr, num_lines, buffer_size, None);
        let r = invoke(impls().r, ptr, num_lines, buffer_size, None);
        (c, r)
    };
    assert_eq!(c_out, r_out, "{label}: C and Rust disagree");
    assert_eq!(
        c_out,
        Outcome::OkOpaque,
        "{label}: expected the C to succeed for \
         numLines={num_lines} bufferSize={buffer_size}"
    );
}

/// E1: `malloc` fails — numLines = 1<<60, product 2^63 (no wrap).
#[test]
fn err_e1_malloc_failure_1_shl_60() {
    let num = 1usize << 60;
    assert_eq!(num.wrapping_mul(8), 1usize << 63, "no wrap expected");
    let mut buf = vec![0u8; 8];
    assert_both_null("E1", buf.as_mut_ptr() as *mut c_char, num, 8);
}

/// E2: `malloc` fails — largest non-wrapping product, numLines = SIZE_MAX/8.
#[test]
fn err_e2_malloc_failure_size_max_div_8() {
    let num = usize::MAX / 8;
    assert_eq!(num.wrapping_mul(8), usize::MAX & !7usize, "no wrap expected");
    let mut buf = vec![0u8; 8];
    assert_both_null("E2", buf.as_mut_ptr() as *mut c_char, num, 8);
}

/// E3: bufferSize == 0 with numLines > 0 — loop never entered.
#[test]
fn err_e3_zero_buffer_size() {
    let mut rng = Rng::new(103);
    let mut buf = vec![0u8; 16];
    for _ in 0..256 {
        let num = rng.range(1, 4096);
        assert_both_null("E3", buf.as_mut_ptr() as *mut c_char, num, 0);
    }
}

/// E4: null `buffer`, bufferSize == 0, numLines > 0 — no dereference happens.
#[test]
fn err_e4_null_buffer_zero_size() {
    for num in [1usize, 2, 7, 64, 4096] {
        assert_both_null("E4", std::ptr::null_mut(), num, 0);
    }
}

/// E5: numLines greater than the number of records that fit in bufferSize.
#[test]
fn err_e5_more_lines_than_records() {
    let mut rng = Rng::new(105);
    for _ in 0..1024 {
        let available = rng.range(1, 12);
        let mut buf = records(&mut rng, available, 0, 8);
        let size = buf.len();
        // How many records the C can actually find in `size` bytes:
        let mut found = 0usize;
        let mut pos = 0usize;
        while pos < size {
            let mut len = 0usize;
            found += 1;
            while pos + len < size && buf[pos + len] != 0 {
                len += 1;
            }
            pos += len;
            if pos < size {
                pos += 1;
            }
        }
        let want = found + rng.range(1, 5);
        assert!(model(&buf, want, size).is_none(), "E5 precondition");
        assert_both_null("E5", buf.as_mut_ptr() as *mut c_char, want, size);
    }
}

/// E6: no NUL anywhere — one unterminated record consumes the whole buffer, so
/// any numLines >= 2 is rejected.
#[test]
fn err_e6_no_terminator_multiple_lines() {
    let mut rng = Rng::new(106);
    for _ in 0..512 {
        let cap = rng.range(1, 48);
        let mut buf: Vec<u8> = (0..cap).map(|_| rng.nonzero_byte()).collect();
        let num = rng.range(2, 32);
        assert!(model(&buf, num, cap).is_none(), "E6 precondition");
        assert_both_null("E6", buf.as_mut_ptr() as *mut c_char, num, cap);
        // exactly one line, however, succeeds
        assert_both_non_null("E6-ok", buf.as_mut_ptr() as *mut c_char, 1, cap);
    }
}

/// E7: numLines = SIZE_MAX — the product wraps to SIZE_MAX-7, malloc fails.
#[test]
fn err_e7_size_max_num_lines() {
    let num = usize::MAX;
    assert_eq!(num.wrapping_mul(8), usize::MAX - 7, "wrap expected");
    assert_both_null("E7", std::ptr::null_mut(), num, 0);
    let mut buf = vec![0u8; 8];
    assert_both_null("E7b", buf.as_mut_ptr() as *mut c_char, num, 8);
}

/// E8: numLines = 1<<61 — the product wraps to exactly 0, so malloc(0)
/// SUCCEEDS (non-NULL on glibc) and the rejection comes from the
/// `lineIndex != numLines` check instead. bufferSize = 0 keeps this safe.
#[test]
fn err_e8_product_wraps_to_zero() {
    let num = 1usize << 61;
    assert_eq!(num.wrapping_mul(8), 0, "product must wrap to 0");
    assert_both_null("E8", std::ptr::null_mut(), num, 0);
}

/// E9: numLines = (1<<61)+1 — the product wraps to 8, an absurdly undersized
/// allocation that still succeeds. bufferSize = 0 prevents the OOB stores.
#[test]
fn err_e9_product_wraps_to_eight() {
    let num = (1usize << 61) + 1;
    assert_eq!(num.wrapping_mul(8), 8, "product must wrap to 8");
    assert_both_null("E9", std::ptr::null_mut(), num, 0);

    // A few more wrapping values, all with bufferSize == 0.
    for k in 1usize..=32 {
        let n = (1usize << 61).wrapping_add(k);
        assert_eq!(n.wrapping_mul(8), k * 8);
        assert_both_null("E9-sweep", std::ptr::null_mut(), n, 0);
    }
    // And numLines = 1<<62 / 1<<63, which wrap to 0 as well.
    for n in [1usize << 62, 1usize << 63] {
        assert_eq!(n.wrapping_mul(8), 0);
        assert_both_null("E9-pow2", std::ptr::null_mut(), n, 0);
    }
}

/// E10 / E15: exactly-N succeeds, N+1 is the first rejection. Asserted as a
/// pair so the boundary itself is pinned, not just one side of it.
#[test]
fn err_e10_e15_one_past_valid_boundary() {
    let mut rng = Rng::new(110);
    for _ in 0..1024 {
        let n = rng.range(1, 16);
        let mut buf = records(&mut rng, n, 0, 10);
        let size = buf.len();
        let ptr = buf.as_mut_ptr() as *mut c_char;
        assert_both_non_null("E15-exact", ptr, n, size);
        assert_both_null("E10-plus-one", ptr, n + 1, size);
        // and two past, for good measure
        assert_both_null("E10-plus-two", ptr, n + 2, size);
    }
}

/// E11: null `buffer` with bufferSize > 0 but numLines == 0 — the outer guard
/// short-circuits before any dereference, so this SUCCEEDS.
#[test]
fn err_e11_null_buffer_zero_lines() {
    for size in [1usize, 2, 64, 1 << 20, usize::MAX] {
        assert_both_non_null("E11", std::ptr::null_mut(), 0, size);
    }
}

/// E12: both lengths zero.
#[test]
fn err_e12_all_zero() {
    assert_both_non_null("E12", std::ptr::null_mut(), 0, 0);
    let mut buf = vec![0u8; 4];
    assert_both_non_null("E12b", buf.as_mut_ptr() as *mut c_char, 0, 0);
}

/// E13: numLines == 0 with a real buffer and bufferSize > 0 — zero elements.
#[test]
fn err_e13_zero_lines_real_buffer() {
    let mut rng = Rng::new(113);
    for _ in 0..256 {
        let cap = rng.range(1, 64);
        let mut buf: Vec<u8> = (0..cap).map(|_| rng.byte()).collect();
        diff("E13", &mut buf, 0, cap);
    }
}

/// E14: bufferSize wildly larger than the real allocation, but numLines is
/// satisfied before the excess is reached, so nothing out of bounds is read.
#[test]
fn err_e14_oversized_buffer_size_safe() {
    let mut rng = Rng::new(114);
    for _ in 0..256 {
        let n = rng.range(1, 12);
        let mut buf = records(&mut rng, n, 1, 8);
        let needed = buf.len();
        let ptr = buf.as_mut_ptr() as *mut c_char;
        // The loop stops at lineIndex == n with pos == needed, which is far
        // below `huge`, so no byte past `needed` is ever read.
        let huge = 1usize << 40;
        let m = model(&buf, n, needed).expect("E14 precondition");
        assert_eq!(m.len(), n, "E14: all n records must be found within `needed`");
        assert!(
            *m.last().unwrap() < needed,
            "E14: last record offset must stay inside the real allocation"
        );
        let c_out = unsafe { invoke(impls().c, ptr, n, huge, Some(n)) };
        let r_out = unsafe { invoke(impls().r, ptr, n, huge, Some(n)) };
        assert_eq!(c_out, r_out, "E14: divergence with oversized bufferSize");
        assert_ne!(c_out, Outcome::Null, "E14 should succeed");
    }
}

/// Additional generic FFI boundary: repeated calls must not leak state between
/// the two libraries (there is no global state in the C, so results must be
/// identical when the same call is repeated).
#[test]
fn err_generic_idempotent_repeat() {
    let mut rng = Rng::new(200);
    for _ in 0..256 {
        let n = rng.range(1, 8);
        let mut buf = records(&mut rng, n, 0, 8);
        let size = buf.len();
        for _ in 0..4 {
            diff("repeat", &mut buf, n, size);
        }
    }
}

// ===========================================================================
// Allocation-size parity
//
// The return value alone cannot distinguish `numLines * sizeof(const char**)`
// from other multipliers, because a non-NULL result requires
// `numLines <= bufferSize` (each loop iteration advances `pos` by at least 1),
// so on every REACHABLE success path the product is small and never wraps.
// `malloc_usable_size` lets us observe the request size directly and pin the
// element width instead of inferring it.
// ===========================================================================

extern "C" {
    fn malloc_usable_size(ptr: *mut c_void) -> usize;
}

#[test]
fn alloc_size_parity() {
    let mut rng = Rng::new(300);
    for _ in 0..512 {
        let n = rng.range(1, 64);
        let mut buf = records(&mut rng, n, 0, 6);
        let size = buf.len();
        let ptr = buf.as_mut_ptr() as *mut c_char;
        unsafe {
            let c = (impls().c)(ptr, n, size);
            let r = (impls().r)(ptr, n, size);
            assert!(!c.is_null() && !r.is_null(), "alloc_size_parity precondition");
            let cu = malloc_usable_size(c as *mut c_void);
            let ru = malloc_usable_size(r as *mut c_void);
            assert_eq!(
                cu, ru,
                "allocation size differs for numLines={n}: C={cu} Rust={ru}"
            );
            assert!(
                cu >= n * std::mem::size_of::<usize>(),
                "C allocation smaller than numLines*8 (n={n}, usable={cu})"
            );
            free(c as *mut c_void);
            free(r as *mut c_void);
        }
    }
}

/// `numLines == 0` must request a zero-byte allocation in both, and glibc must
/// hand back a non-NULL pointer that we can `free`.
#[test]
fn alloc_size_parity_zero() {
    unsafe {
        let c = (impls().c)(std::ptr::null_mut(), 0, 0);
        let r = (impls().r)(std::ptr::null_mut(), 0, 0);
        assert!(!c.is_null(), "glibc malloc(0) is non-NULL");
        assert!(!r.is_null(), "Rust must also return the malloc(0) pointer");
        assert_eq!(
            malloc_usable_size(c as *mut c_void),
            malloc_usable_size(r as *mut c_void),
            "malloc(0) usable size differs"
        );
        free(c as *mut c_void);
        free(r as *mut c_void);
    }
}

// ===========================================================================
// Exhaustive small-input sweep
//
// Property-style randomization can miss a specific byte pattern. For small
// sizes we can enumerate the ENTIRE input space instead of sampling it.
// ===========================================================================

/// Enumerate every buffer over `alphabet` of length `len`, for every
/// `bufferSize` in `0..=len` and every `numLines` in `0..=len+1`.
fn exhaustive_over(alphabet: &[u8], len: usize) -> usize {
    let radix = alphabet.len();
    let total = radix.pow(len as u32);
    let mut buf = vec![0u8; len.max(1)];
    let mut calls = 0usize;
    for mut code in 0..total {
        for i in 0..len {
            buf[i] = alphabet[code % radix];
            code /= radix;
        }
        let ptr = buf.as_mut_ptr() as *mut c_char;
        for size in 0..=len {
            for num in 0..=(len + 1) {
                let c_out = unsafe { invoke(impls().c, ptr, num, size, Some(num)) };
                let r_out = unsafe { invoke(impls().r, ptr, num, size, Some(num)) };
                assert_eq!(
                    c_out,
                    r_out,
                    "exhaustive divergence: buf={:?} numLines={num} bufferSize={size}",
                    &buf[..len]
                );
                calls += 1;
            }
        }
    }
    calls
}

/// Binary alphabet {NUL, 'a'} up to length 12 — every possible NUL placement.
#[test]
fn exhaustive_binary_alphabet() {
    let mut calls = 0;
    for len in 0..=12 {
        calls += exhaustive_over(&[0u8, b'a'], len);
    }
    assert!(calls > 100_000, "expected a large sweep, got {calls}");
}

/// Three-symbol alphabet including a high (negative-as-signed `char`) byte, so
/// sign handling is covered exhaustively too.
#[test]
fn exhaustive_ternary_alphabet_with_high_byte() {
    let mut calls = 0;
    for len in 0..=8 {
        calls += exhaustive_over(&[0u8, 0x01, 0xFFu8], len);
    }
    assert!(calls > 100_000, "expected a large sweep, got {calls}");
}

// ===========================================================================
// Guard-page test: prove neither implementation reads past `bufferSize`
//
// The buffer is placed so that its last byte is the last byte of a readable
// page, with an inaccessible (PROT_NONE) page immediately after. Any read at
// offset >= bufferSize faults, so if this test completes, neither the C nor the
// Rust over-read. This is what pins the inner loop's `pos + len < bufferSize`
// bound, which the return value alone cannot observe.
// ===========================================================================

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

extern "C" {
    fn mmap(
        addr: *mut c_void,
        length: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
}

struct GuardedBuf {
    base: *mut u8,
    page: usize,
    len: usize,
}

impl GuardedBuf {
    /// Allocate `len` bytes ending exactly at a guard page boundary.
    fn new(len: usize) -> GuardedBuf {
        let page = 4096usize;
        assert!(len <= page);
        let total = page * 2;
        let base = unsafe {
            mmap(
                std::ptr::null_mut(),
                total,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(base as isize != -1, "mmap failed");
        let rc = unsafe { mprotect((base as *mut u8).add(page) as *mut c_void, page, PROT_NONE) };
        assert_eq!(rc, 0, "mprotect failed");
        GuardedBuf {
            base: base as *mut u8,
            page,
            len,
        }
    }
    /// Pointer to the first byte of the guarded region.
    fn ptr(&self) -> *mut u8 {
        unsafe { self.base.add(self.page - self.len) }
    }
    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr(), self.len) }
    }
}

impl Drop for GuardedBuf {
    fn drop(&mut self) {
        unsafe {
            munmap(self.base as *mut c_void, self.page * 2);
        }
    }
}

#[test]
fn guard_page_no_read_past_buffer_size() {
    let mut rng = Rng::new(400);

    // Sanity check that the guard really is inaccessible would require catching
    // SIGSEGV, so instead we rely on the process surviving: if either
    // implementation read past `bufferSize`, this test would crash the harness.

    // 1) Fully unterminated buffers of every length 1..=64: the inner loop must
    //    stop exactly at `bufferSize`.
    for len in 1..=64usize {
        let mut g = GuardedBuf::new(len);
        for b in g.as_mut_slice().iter_mut() {
            *b = 0xAA;
        }
        let p = g.ptr() as *mut c_char;
        for num in [1usize, 2, len] {
            let c_out = unsafe { invoke(impls().c, p, num, len, Some(num)) };
            let r_out = unsafe { invoke(impls().r, p, num, len, Some(num)) };
            assert_eq!(c_out, r_out, "guard-page divergence len={len} num={num}");
        }
    }

    // 2) Randomized contents, always with a non-NUL final byte so a missing
    //    bound check would run straight into the guard page.
    for _ in 0..2048 {
        let len = rng.range(1, 512).min(4096);
        let mut g = GuardedBuf::new(len);
        {
            let s = g.as_mut_slice();
            for b in s.iter_mut() {
                *b = if rng.below(4) == 0 { 0 } else { rng.nonzero_byte() };
            }
            let last = s.len() - 1;
            s[last] = rng.nonzero_byte(); // unterminated tail
        }
        let p = g.ptr() as *mut c_char;
        let num = rng.range(1, 24);
        let c_out = unsafe { invoke(impls().c, p, num, len, Some(num)) };
        let r_out = unsafe { invoke(impls().r, p, num, len, Some(num)) };
        assert_eq!(c_out, r_out, "guard-page divergence len={len} num={num}");
    }

    // 3) Zero-length guarded buffer: bufferSize == 0 must not read byte 0.
    let g = GuardedBuf::new(0);
    let p = g.ptr() as *mut c_char;
    for num in [0usize, 1, 5] {
        let c_out = unsafe { invoke(impls().c, p, num, 0, None) };
        let r_out = unsafe { invoke(impls().r, p, num, 0, None) };
        assert_eq!(c_out, r_out, "guard-page zero-len divergence num={num}");
    }
}

/// Reports the exhaustive call counts so the sweep size is visible in the log
/// (`cargo test --release -- --nocapture exhaustive_call_count`).
#[test]
fn exhaustive_call_count_report() {
    let t = std::time::Instant::now();
    let mut binary = 0usize;
    for len in 0..=12 {
        binary += exhaustive_over(&[0u8, b'a'], len);
    }
    let t1 = t.elapsed();
    let mut ternary = 0usize;
    for len in 0..=8 {
        ternary += exhaustive_over(&[0u8, 0x01, 0xFFu8], len);
    }
    println!(
        "exhaustive: binary={binary} pairs in {t1:?}, ternary={ternary} pairs in {:?}, \
         total FFI calls={}",
        t.elapsed() - t1,
        (binary + ternary) * 2
    );
    assert!(binary >= 1_294_334, "binary sweep too small: {binary}");
    assert!(ternary >= 590_000, "ternary sweep too small: {ternary}");
}

// ===========================================================================
// Allocator-accounting tests (active only under the LD_PRELOAD interposer)
//
// Run via ./run_with_interpose.sh. Without the interposer these tests skip.
// They pin two things the return value cannot express:
//   * the malloc REQUEST SIZE (so `numLines * sizeof(const char**)` and its
//     wrapping behaviour are observable, not merely inferred), and
//   * whether the error path calls free() (leak parity).
// ===========================================================================

struct Trace {
    _lib: Library,
    reset: unsafe extern "C" fn(),
    enable: unsafe extern "C" fn(),
    disable: unsafe extern "C" fn(),
    malloc_calls: unsafe extern "C" fn() -> u64,
    free_calls: unsafe extern "C" fn() -> u64,
    last_size: unsafe extern "C" fn() -> usize,
}

fn trace() -> Option<&'static Trace> {
    use std::sync::OnceLock;
    static ONCE: OnceLock<Option<Trace>> = OnceLock::new();
    ONCE.get_or_init(|| {
        let path = std::env::var("MALLOC_TRACE_SO").ok()?;
        unsafe {
            let lib = Library::new(&path).ok()?;
            let g = |n: &[u8]| -> Option<usize> {
                let s: Symbol<*const c_void> = lib.get(n).ok()?;
                Some(*s as usize)
            };
            g(b"mt_present\0")?;
            let reset: Symbol<unsafe extern "C" fn()> = lib.get(b"mt_reset\0").ok()?;
            let enable: Symbol<unsafe extern "C" fn()> = lib.get(b"mt_enable\0").ok()?;
            let disable: Symbol<unsafe extern "C" fn()> = lib.get(b"mt_disable\0").ok()?;
            let mc: Symbol<unsafe extern "C" fn() -> u64> =
                lib.get(b"mt_malloc_calls\0").ok()?;
            let fc: Symbol<unsafe extern "C" fn() -> u64> = lib.get(b"mt_free_calls\0").ok()?;
            let ls: Symbol<unsafe extern "C" fn() -> usize> =
                lib.get(b"mt_last_malloc_size\0").ok()?;
            let t = Trace {
                reset: *reset,
                enable: *enable,
                disable: *disable,
                malloc_calls: *mc,
                free_calls: *fc,
                last_size: *ls,
            _lib: lib,
            };
            Some(t)
        }
    })
    .as_ref()
}

/// One traced invocation: (returned pointer is-null, malloc calls, free calls,
/// last requested size).
unsafe fn traced(
    t: &Trace,
    f: CreateLinePointers,
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> (bool, u64, u64, usize) {
    (t.reset)();
    (t.enable)();
    let ret = f(buffer, num_lines, buffer_size);
    (t.disable)();
    let m = (t.malloc_calls)();
    let fr = (t.free_calls)();
    let sz = (t.last_size)();
    if !ret.is_null() {
        free(ret as *mut c_void);
    }
    (ret.is_null(), m, fr, sz)
}

#[test]
fn interpose_malloc_size_and_free_parity() {
    let Some(t) = trace() else {
        eprintln!("SKIP interpose_malloc_size_and_free_parity: run ./run_with_interpose.sh");
        return;
    };

    let mut rng = Rng::new(500);
    let mut cases: Vec<(Vec<u8>, usize, usize)> = Vec::new();

    // success paths of many shapes
    for _ in 0..400 {
        let n = rng.range(1, 40);
        let buf = records(&mut rng, n, 0, 6);
        let size = buf.len();
        cases.push((buf, n, size));
    }
    // error path: bufferSize == 0 (allocation made, then freed)
    for n in [1usize, 2, 17, 4096] {
        cases.push((vec![0u8; 4], n, 0));
    }
    // numLines == 0 -> malloc(0)
    cases.push((vec![0u8; 4], 0, 0));
    cases.push((vec![1u8, 0, 2, 0], 0, 4));
    // error path from buffer exhaustion
    for _ in 0..100 {
        let n = rng.range(1, 8);
        let buf = records(&mut rng, n, 1, 6);
        let size = buf.len();
        cases.push((buf, n + rng.range(1, 3), size));
    }
    // wrapping products, kept safe with bufferSize == 0
    for n in [
        1usize << 61,
        (1usize << 61) + 1,
        (1usize << 61) + 7,
        1usize << 62,
        1usize << 63,
        usize::MAX,
        usize::MAX / 8,
        1usize << 60,
    ] {
        cases.push((vec![0u8; 4], n, 0));
    }

    for (mut buf, num, size) in cases {
        let ptr = buf.as_mut_ptr() as *mut c_char;
        let c = unsafe { traced(t, impls().c, ptr, num, size) };
        let r = unsafe { traced(t, impls().r, ptr, num, size) };
        assert_eq!(
            c, r,
            "allocator-behaviour divergence for numLines={num} bufferSize={size}: \
             (is_null, mallocs, frees, last_size) C={c:?} Rust={r:?}"
        );
        // Cross-check the request size against the C's own formula, so the test
        // fails if BOTH sides changed the multiplier in the same way.
        let expected = num.wrapping_mul(std::mem::size_of::<*const *const c_char>());
        assert_eq!(
            c.3, expected,
            "C requested {} bytes but numLines*sizeof(const char**) == {expected} \
             (numLines={num})",
            c.3
        );
        assert_eq!(c.1, 1, "exactly one malloc expected (numLines={num})");
    }
}

/// The error path must free its allocation in both implementations, and the
/// success path must not.
#[test]
fn interpose_error_path_frees() {
    let Some(t) = trace() else {
        eprintln!("SKIP interpose_error_path_frees: run ./run_with_interpose.sh");
        return;
    };
    let mut buf = vec![b'a', 0, b'b', 0];

    // Success: allocation handed to the caller, no free inside the library.
    let ptr = buf.as_mut_ptr() as *mut c_char;
    let c = unsafe { traced(t, impls().c, ptr, 2, 4) };
    let r = unsafe { traced(t, impls().r, ptr, 2, 4) };
    assert_eq!(c, r);
    assert!(!c.0, "expected success");
    assert_eq!(c.2, 0, "success path must not free");

    // Error (buffer exhausted): the library must free before returning NULL.
    let c = unsafe { traced(t, impls().c, ptr, 9, 4) };
    let r = unsafe { traced(t, impls().r, ptr, 9, 4) };
    assert_eq!(c, r);
    assert!(c.0, "expected NULL");
    assert_eq!(c.2, 1, "error path must free exactly once");

    // Error (malloc failed): nothing to free.
    let c = unsafe { traced(t, impls().c, ptr, 1usize << 60, 4) };
    let r = unsafe { traced(t, impls().r, ptr, 1usize << 60, 4) };
    assert_eq!(c, r);
    assert!(c.0, "expected NULL from failed malloc");
    assert_eq!(c.2, 0, "nothing was allocated, so nothing to free");
}
