//! Differential harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compares `decode_base64` through the FFI boundary.
//!
//! Nothing here calls the Rust crate directly, so the `#[no_mangle]` export
//! wrapper is exercised exactly as an external consumer would exercise it.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

type DecodeBase64 = unsafe extern "C" fn(*const c_char) -> *mut c_char;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    crate_root()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // `verify.sh` sets DRIVER_SO so the profile under test is unambiguous.
    if let Ok(p) = std::env::var("DRIVER_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_SO points at a missing file: {}", p.display());
        return p;
    }
    // Otherwise prefer the newest of the two profiles that exists.
    let root = crate_root().join("target");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for p in ["release/libdriver.so", "debug/libdriver.so"] {
        let cand = root.join(p);
        if let Ok(t) = std::fs::metadata(&cand).and_then(|m| m.modified()) {
            if best.as_ref().is_none_or(|(bt, _)| t > *bt) {
                best = Some((t, cand));
            }
        }
    }
    match best {
        Some((_, p)) => p,
        None => panic!(
            "no libdriver.so found under {} — run `cargo build --release` first \
             (cargo test does NOT build a cdylib)",
            root.display()
        ),
    }
}

struct Libs {
    c: Library,
    rust: Library,
    free: unsafe extern "C" fn(*mut c_void),
}

// Safety: both libraries are pure functions over their argument plus libc
// malloc; the `Library` handles are only ever read.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact (the test
/// binaries never link it), so without this guard the suite happily validates a
/// STALE `libdriver.so` and reports success for source that was never compiled.
/// Refuse to run in that case.
fn assert_so_fresh(so: &std::path::Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = crate_root().join(src);
        let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            m <= so_mtime,
            "STALE ARTIFACT: {} is newer than {}.\n\
             `cargo test` does not rebuild a cdylib — run\n    \
             cargo build --release && cargo test --release\n\
             (refusing to validate a .so that does not match the source)",
            p.display(),
            so.display()
        );
    }
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "C shared library missing at {} — build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            cp.display()
        );
        assert_so_fresh(&rp);
        unsafe {
            let c = Library::new(&cp).expect("load C .so");
            let rust = Library::new(&rp).expect("load Rust .so");
            // `free` comes from libc, resolved through the C library handle so
            // that both allocations are released by the same allocator that
            // produced them.
            let free_sym: Symbol<unsafe extern "C" fn(*mut c_void)> =
                c.get(b"free\0").expect("libc free via C .so");
            let free = *free_sym;
            Libs { c, rust, free }
        }
    })
}

fn c_decode() -> Symbol<'static, DecodeBase64> {
    unsafe { libs().c.get(b"decode_base64\0").expect("C decode_base64") }
}

fn rust_decode() -> Symbol<'static, DecodeBase64> {
    unsafe {
        libs()
            .rust
            .get(b"decode_base64\0")
            .expect("Rust decode_base64")
    }
}

// ---------------------------------------------------------------------------
// Comparison core
// ---------------------------------------------------------------------------

/// Result of one call, captured so the allocation can be freed immediately.
#[derive(PartialEq, Eq)]
enum Outcome {
    Null,
    /// The *entire* `calloc`'d region. Its size is fully determined by the
    /// input (`strlen(src) + 1 + 13`) and `calloc` zeroes it, so every byte is
    /// deterministic — comparing all of it catches over- and under-writes that
    /// a NUL-terminated-prefix comparison would miss.
    Buf(Vec<u8>),
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Null => write!(f, "NULL"),
            Outcome::Buf(b) => write!(f, "Buf(len={}, {:02x?})", b.len(), b),
        }
    }
}

/// Call `f` with a NUL-terminated copy of `input` and capture the outcome.
///
/// `input` must not contain an interior NUL (the C API is NUL-terminated, so an
/// interior NUL would simply truncate the input — tests that want that use
/// `call_raw`).
unsafe fn call(f: &DecodeBase64, input: &[u8]) -> Outcome {
    let mut z = Vec::with_capacity(input.len() + 1);
    z.extend_from_slice(input);
    z.push(0);
    call_raw(f, &z)
}

/// Call `f` with an already-NUL-terminated byte buffer.
unsafe fn call_raw(f: &DecodeBase64, nul_terminated: &[u8]) -> Outcome {
    assert_eq!(
        nul_terminated.last().copied(),
        Some(0u8),
        "call_raw needs a NUL terminator"
    );
    let p = f(nul_terminated.as_ptr() as *const c_char);
    if p.is_null() {
        return Outcome::Null;
    }
    // strlen(src) as the C library computes it, i.e. up to the first NUL.
    let slen = nul_terminated
        .iter()
        .position(|&b| b == 0)
        .expect("NUL present");
    let alloc_len = slen + 1 + 13; // calloc(1, strlen(src) + 1 + 13)
    let bytes = std::slice::from_raw_parts(p as *const u8, alloc_len).to_vec();
    (libs().free)(p as *mut c_void);
    Outcome::Buf(bytes)
}

/// The one assertion every row funnels through.
fn diff(input: &[u8], ctx: &str) {
    let (cf, rf) = (c_decode(), rust_decode());
    let (c, r) = unsafe { (call(&cf, input), call(&rf, input)) };
    if c != r {
        panic!(
            "DIVERGENCE [{ctx}]\n  input ({} bytes) = {:02x?}\n  as text          = {:?}\n  C    = {:?}\n  Rust = {:?}",
            input.len(),
            input,
            String::from_utf8_lossy(input),
            c,
            r
        );
    }
}

fn diff_raw(nul_terminated: &[u8], ctx: &str) {
    let (cf, rf) = (c_decode(), rust_decode());
    let (c, r) = unsafe { (call_raw(&cf, nul_terminated), call_raw(&rf, nul_terminated)) };
    assert_eq!(c, r, "DIVERGENCE [{ctx}] raw={:02x?}", nul_terminated);
}

// ---------------------------------------------------------------------------
// Seeded RNG (SplitMix64) — reproducible, no external dependency
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    fn pick(&mut self, pool: &[u8]) -> u8 {
        pool[self.below(pool.len())]
    }
    /// Random bytes drawn from `pool`, length exactly `len`.
    fn string(&mut self, pool: &[u8], len: usize) -> Vec<u8> {
        (0..len).map(|_| self.pick(pool)).collect()
    }
}

// Character pools mirroring the branch classes in the C source.
const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGITS: &[u8] = b"0123456789";
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
/// Printable ASCII that `is_base64` REJECTS (no NUL, no `=`).
const IGNORED: &[u8] = b" !\"#$%&'()*,-.:;<>?@[\\]^_`{|}~\t\r\n";

const ROUNDS: usize = 512;

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md rows
// ---------------------------------------------------------------------------

/// Rows 1-3: a single `decode()` branch class in isolation, `l % 4 == 0`.
fn single_class(pool: &[u8], seed: u64, ctx: &str) {
    let mut rng = Rng::new(seed);
    for len in [4usize, 8, 12, 16, 40] {
        for _ in 0..ROUNDS / 4 {
            diff(&rng.string(pool, len), ctx);
        }
    }
}

#[test]
fn cfg_01_upper_only() {
    single_class(UPPER, 0x01, "cfg_01_upper_only");
}

#[test]
fn cfg_02_lower_only() {
    single_class(LOWER, 0x02, "cfg_02_lower_only");
}

#[test]
fn cfg_03_digits_only() {
    single_class(DIGITS, 0x03, "cfg_03_digits_only");
}

/// Row 4: `'+'` — the explicit `return 62` branch.
#[test]
fn cfg_04_plus_only() {
    for len in 1..=16 {
        diff(&vec![b'+'; len], "cfg_04_plus_only");
    }
}

/// Row 5: `'/'` — reaches `decode`'s fallthrough `return 63`.
#[test]
fn cfg_05_slash_only() {
    for len in 1..=16 {
        diff(&vec![b'/'; len], "cfg_05_slash_only");
    }
}

/// Rows 6-9: full alphabet at every `l % 4` residue.
fn mod_row(residue: usize, seed: u64, ctx: &str) {
    let mut rng = Rng::new(seed);
    for groups in 0..8usize {
        let len = groups * 4 + residue;
        if len == 0 {
            continue; // empty string is an ERRORS.md row, not a CONFIGS row
        }
        for _ in 0..ROUNDS / 4 {
            diff(&rng.string(ALPHABET, len), ctx);
        }
    }
}

#[test]
fn cfg_06_full_alphabet_mod0() {
    mod_row(0, 0x06, "cfg_06_full_alphabet_mod0");
}

#[test]
fn cfg_07_mod1() {
    mod_row(1, 0x07, "cfg_07_mod1");
}

#[test]
fn cfg_08_mod2() {
    mod_row(2, 0x08, "cfg_08_mod2");
}

#[test]
fn cfg_09_mod3() {
    mod_row(3, 0x09, "cfg_09_mod3");
}

/// Row 10: canonical single padding `xxx=` — `c4 == '='`, `c3 != '='`.
#[test]
fn cfg_10_pad_one() {
    let mut rng = Rng::new(0x10);
    for groups in 1..=4usize {
        for _ in 0..ROUNDS {
            let mut s = rng.string(ALPHABET, groups * 4);
            *s.last_mut().unwrap() = b'=';
            diff(&s, "cfg_10_pad_one");
        }
    }
}

/// Row 11: canonical double padding `xx==` — both gates suppressed.
#[test]
fn cfg_11_pad_two() {
    let mut rng = Rng::new(0x11);
    for groups in 1..=4usize {
        for _ in 0..ROUNDS {
            let mut s = rng.string(ALPHABET, groups * 4);
            let n = s.len();
            s[n - 2] = b'=';
            s[n - 1] = b'=';
            diff(&s, "cfg_11_pad_two");
        }
    }
}

/// Row 12: pathological padding — `'='` at the `c2` slot, and `c3 == '='` while
/// `c4 != '='` (a combination canonical base64 never produces but C accepts).
#[test]
fn cfg_12_pad_pathological() {
    let mut rng = Rng::new(0x12);
    for _ in 0..ROUNDS {
        // x=x=
        let mut a = rng.string(ALPHABET, 4);
        a[1] = b'=';
        a[3] = b'=';
        diff(&a, "cfg_12_pad_pathological x=x=");
        // x==x  → c3=='=' suppresses byte 2, c4!='=' still writes byte 3
        let mut b = rng.string(ALPHABET, 4);
        b[1] = b'=';
        b[2] = b'=';
        diff(&b, "cfg_12_pad_pathological x==x");
        // xx=x
        let mut c = rng.string(ALPHABET, 4);
        c[2] = b'=';
        diff(&c, "cfg_12_pad_pathological xx=x");
    }
}

/// Row 13: `'='` occupying the `c1` slot.
#[test]
fn cfg_13_pad_leading() {
    let mut rng = Rng::new(0x13);
    for _ in 0..ROUNDS {
        let mut a = rng.string(ALPHABET, 4);
        a[0] = b'=';
        diff(&a, "cfg_13_pad_leading =xxx");
        let mut b = rng.string(ALPHABET, 4);
        b[0] = b'=';
        b[1] = b'=';
        b[2] = b'=';
        diff(&b, "cfg_13_pad_leading ===x");
    }
    diff(b"=", "cfg_13_pad_leading single =");
    diff(b"==", "cfg_13_pad_leading ==");
    diff(b"===", "cfg_13_pad_leading ===");
}

/// Row 14: `'='` interior to a multi-group input — the C code does NOT stop at
/// padding, it keeps decoding, and `decode('=')` yields 63.
#[test]
fn cfg_14_pad_interior_multigroup() {
    let mut rng = Rng::new(0x14);
    for _ in 0..ROUNDS {
        let mut s = rng.string(ALPHABET, 16);
        // scatter 1..4 '=' anywhere in the first 12 bytes
        let n = 1 + rng.below(4);
        for _ in 0..n {
            let i = rng.below(12);
            s[i] = b'=';
        }
        diff(&s, "cfg_14_pad_interior_multigroup");
    }
    diff(b"QQ==QQ==", "cfg_14 QQ==QQ==");
    diff(b"====AAAA", "cfg_14 ====AAAA");
}

/// Row 15: bytes the filter drops, interspersed among valid ones (so the raw
/// length and the filtered length `l` differ).
#[test]
fn cfg_15_ignored_interspersed() {
    let mut rng = Rng::new(0x15);
    let mixed: Vec<u8> = ALPHABET
        .iter()
        .chain(IGNORED.iter())
        .chain(b"=".iter())
        .copied()
        .collect();
    for len in [1usize, 2, 3, 5, 8, 13, 21, 34, 64] {
        for _ in 0..ROUNDS / 2 {
            diff(&rng.string(&mixed, len), "cfg_15_ignored_interspersed");
        }
    }
}

/// Row 16: every byte ignored → filtered length `l == 0`, decode loop never
/// entered, so a non-NULL all-zero buffer comes back.
#[test]
fn cfg_16_all_ignored() {
    let mut rng = Rng::new(0x16);
    for len in 1..=32usize {
        for _ in 0..16 {
            diff(&rng.string(IGNORED, len), "cfg_16_all_ignored");
        }
    }
}

/// Row 17: arbitrary bytes `0x01..=0xFF`, including the high-bit values that are
/// NEGATIVE as a signed `char` on x86-64 and so fail every range check.
#[test]
fn cfg_17_arbitrary_bytes() {
    let mut rng = Rng::new(0x17);
    let all: Vec<u8> = (1u8..=255).collect();
    for _ in 0..2000 {
        let len = 1 + rng.below(48);
        diff(&rng.string(&all, len), "cfg_17_arbitrary_bytes");
    }
    // high-bit only
    let hi: Vec<u8> = (0x80u8..=0xFF).collect();
    for len in 1..=8usize {
        for _ in 0..32 {
            diff(&rng.string(&hi, len), "cfg_17_arbitrary_bytes high-bit");
        }
    }
}

/// Row 18: exhaustive short lengths, randomized content per length.
#[test]
fn cfg_18_all_short_lengths() {
    let mut rng = Rng::new(0x18);
    let mixed: Vec<u8> = ALPHABET.iter().chain(b"=!.".iter()).copied().collect();
    for len in 1..=8usize {
        for _ in 0..ROUNDS {
            diff(&rng.string(&mixed, len), "cfg_18_all_short_lengths");
        }
    }
}

/// Row 19: every possible single-byte input.
#[test]
fn cfg_19_single_byte_exhaustive() {
    for b in 1u8..=255 {
        diff(&[b], "cfg_19_single_byte_exhaustive");
    }
}

/// Row 20: every possible two-byte input (65 025 cases).
#[test]
fn cfg_20_two_byte_exhaustive() {
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            diff(&[a, b], "cfg_20_two_byte_exhaustive");
        }
    }
}

/// Row 21: 1 MiB input.
#[test]
fn cfg_21_large_input() {
    let mut rng = Rng::new(0x21);
    let mixed: Vec<u8> = ALPHABET
        .iter()
        .chain(IGNORED.iter())
        .chain(b"=".iter())
        .copied()
        .collect();
    for len in [1usize << 16, 1 << 20] {
        diff(&rng.string(&mixed, len), "cfg_21_large_input");
    }
    // pure alphabet, exact multiple of 4
    diff(&vec![b'Z'; 1 << 20], "cfg_21_large_input ZZZZ...");
}

/// Row 22: one value either side of every range check in `decode` and
/// `is_base64` — `@ A Z [`, `` ` a z { ``, `/ 0 9 :`, `* + ,`, `=`.
#[test]
fn cfg_22_range_boundaries() {
    const B: &[u8] = b"@AZ[`az{/09:*+,=<>";
    for &c in B {
        diff(&[c], "cfg_22 single");
    }
    // every ordered pair, triple and quad of boundary chars
    for &a in B {
        for &b in B {
            diff(&[a, b], "cfg_22 pair");
            for &c in B {
                diff(&[a, b, c], "cfg_22 triple");
                diff(&[a, b, c, a], "cfg_22 quad");
            }
        }
    }
}

/// Row 23: no cross-call state — the same input repeated and interleaved must
/// give the same answer every time in both libraries.
#[test]
fn cfg_23_no_hidden_state() {
    let inputs: [&[u8]; 5] = [b"QUJD", b"a", b"////", b"====", b"Zm9vYmFy"];
    let mut baseline = Vec::new();
    for i in &inputs {
        let cf = c_decode();
        baseline.push(unsafe { call(&cf, i) });
    }
    for _ in 0..3 {
        for (idx, i) in inputs.iter().enumerate() {
            diff(i, "cfg_23_no_hidden_state");
            let rf = rust_decode();
            let r = unsafe { call(&rf, i) };
            assert_eq!(baseline[idx], r, "cross-call state drift on {:?}", i);
        }
    }
}

/// Row 24: known vectors, including the RFC 4648 test set.
#[test]
fn cfg_24_known_vectors() {
    const V: &[&[u8]] = &[
        b"",
        b"Zg==",
        b"Zm8=",
        b"Zm9v",
        b"Zm9vYg==",
        b"Zm9vYmE=",
        b"Zm9vYmFy",
        b"SGVsbG8sIFdvcmxkIQ==",
        b"TWFu",
        b"YW55IGNhcm5hbCBwbGVhc3VyZS4=",
        b"YW55IGNhcm5hbCBwbGVhc3VyZQ==",
        b"YW55IGNhcm5hbCBwbGVhc3Vy",
        b"/w==",
        b"//8=",
        b"////",
        b"+/+/",
        b"AAAA",
        b"////////",
        // with newlines, as MIME base64 arrives in the wild
        b"Zm9v\nYmFy",
        b"Zm9v\r\nYmFy\r\n",
        b"  Zm9vYmFy  ",
    ];
    for v in V {
        diff(v, "cfg_24_known_vectors");
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows
// ---------------------------------------------------------------------------

/// Row 1: `src == NULL`.
#[test]
fn err_01_null_pointer() {
    let (cf, rf) = (c_decode(), rust_decode());
    let (c, r) = unsafe { (cf(std::ptr::null()), rf(std::ptr::null())) };
    assert!(c.is_null(), "C must return NULL for a NULL src, got {c:?}");
    assert!(r.is_null(), "Rust must return NULL for a NULL src, got {r:?}");
}

/// Rows 2 and 12: empty string / zero length.
#[test]
fn err_02_empty_string() {
    let (cf, rf) = (c_decode(), rust_decode());
    for buf in [
        &b"\0"[..],
        &b"\0trailing garbage"[..], // zero-length view into a larger buffer
    ] {
        let (c, r) = unsafe {
            (
                cf(buf.as_ptr() as *const c_char),
                rf(buf.as_ptr() as *const c_char),
            )
        };
        assert!(c.is_null(), "C must return NULL for {buf:02x?}");
        assert!(r.is_null(), "Rust must return NULL for {buf:02x?}");
    }
}

/// Rows 3 and 4: the two allocation-failure branches. These cannot be induced
/// through the public ABI without process-wide allocator interposition, so they
/// are pinned structurally: both implementations must use the *libc* allocator
/// (not Rust's global allocator) and the `malloc` failure path must `free(dest)`
/// before returning NULL. See ERRORS.md for the rationale.
#[test]
fn err_03_04_allocation_failure_structure() {
    let root = crate_root();
    let rs = std::fs::read_to_string(root.join("src/lib.rs")).expect("read src/lib.rs");

    // The Rust translation must import the same libc allocator the C uses, so
    // that its allocation-failure thresholds are identical by construction.
    for needed in ["fn calloc(", "fn malloc(", "fn free("] {
        assert!(
            rs.contains(needed),
            "Rust must declare libc `{needed}` so allocation behaviour matches C"
        );
    }
    assert!(
        !rs.contains("Vec::") && !rs.contains("Box::") && !rs.contains("String::"),
        "Rust must not allocate the result with Rust's global allocator — the \
         caller frees it with libc free()"
    );

    // calloc-failure branch: return NULL, allocate nothing else first.
    assert!(
        rs.contains("if dest.is_null()"),
        "missing the calloc-failure branch (ERRORS.md row 3)"
    );
    // malloc-failure branch: free(dest) THEN return NULL.
    let buf_null = rs
        .find("if buf.is_null()")
        .expect("missing the malloc-failure branch (ERRORS.md row 4)");
    let tail = &rs[buf_null..];
    let brace_end = tail.find('}').expect("malloc-failure block is unterminated");
    let block = &tail[..brace_end];
    assert!(
        block.contains("free(dest"),
        "ERRORS.md row 4: the malloc-failure path must free(dest) before \
         returning NULL; block was:\n{block}"
    );
    assert!(
        block.contains("null_mut()"),
        "ERRORS.md row 4: the malloc-failure path must return NULL; block was:\n{block}"
    );

    // And the observable half: a successful call still returns non-NULL.
    let (cf, rf) = (c_decode(), rust_decode());
    let (c, r) = unsafe { (call(&cf, b"QQ=="), call(&rf, b"QQ==")) };
    assert!(!matches!(c, Outcome::Null));
    assert_eq!(c, r);
}

/// Row 5: a 1-char string with no base64 characters at all (`l == 0`) is NOT an
/// error — a non-NULL, all-zero buffer comes back.
#[test]
fn err_05_all_invalid_chars() {
    let (cf, rf) = (c_decode(), rust_decode());
    for s in [&b"!"[..], b" ", b"\n", b"!!!!", b"\x80", b"\xff\xfe\xfd"] {
        let (c, r) = unsafe { (call(&cf, s), call(&rf, s)) };
        assert!(
            !matches!(c, Outcome::Null),
            "C must NOT reject all-invalid input {s:02x?}"
        );
        assert!(
            !matches!(r, Outcome::Null),
            "Rust must NOT reject all-invalid input {s:02x?}"
        );
        assert_eq!(c, r, "divergence on all-invalid input {s:02x?}");
        if let Outcome::Buf(b) = &c {
            assert!(
                b.iter().all(|&x| x == 0),
                "expected an all-zero buffer for {s:02x?}, got {b:02x?}"
            );
        }
    }
}

/// Row 6: pure padding.
#[test]
fn err_06_only_padding() {
    for s in [&b"="[..], b"==", b"===", b"====", b"========"] {
        diff(s, "err_06_only_padding");
    }
    // and confirm the shape the C produces for "====": exactly one byte, and it
    // is 0xFF, not 0x00 — `decode('=')` falls through to `return 63`, so
    // b1 = b2 = 63 and (63 << 2) | (63 >> 4) == 0xFC | 0x03 == 0xFF.
    let cf = c_decode();
    if let Outcome::Buf(b) = unsafe { call(&cf, b"====") } {
        assert_eq!(b[0], 0xFF, "buffer was {b:02x?}");
        assert!(
            b[1..].iter().all(|&x| x == 0),
            "only one byte may be written for \"====\", got {b:02x?}"
        );
    } else {
        panic!("C returned NULL for \"====\"");
    }
}

/// Row 7: a dangling group (`l % 4 == 1`) still emits 3 bytes because `c2`,
/// `c3`, `c4` default to `'A'`.
#[test]
fn err_07_dangling_group() {
    let mut rng = Rng::new(0x07_07);
    for _ in 0..ROUNDS {
        let n = 1 + rng.below(3) * 4;
        let mut s = rng.string(ALPHABET, n);
        s.truncate(if s.len() % 4 == 1 { s.len() } else { 1 });
        diff(&s, "err_07_dangling_group");
    }
    for s in [&b"Q"[..], b"QQQQQ", b"QQQQQQQQQ", b"/", b"+"] {
        diff(s, "err_07_dangling_group fixed");
    }
}

/// Row 8: high-bit bytes are negative as signed `char`, so every range check in
/// `is_base64` is false and they are dropped.
#[test]
fn err_08_high_bit_bytes() {
    for b in 0x80u8..=0xFF {
        diff(&[b], "err_08_high_bit_bytes");
        diff(&[b, b'Q', b'Q', b'='], "err_08_high_bit_bytes mixed");
        diff(&[b'Q', b, b'Q', b'Q', b], "err_08_high_bit_bytes mixed2");
    }
}

/// Row 9: interior padding does not terminate decoding.
#[test]
fn err_09_interior_padding() {
    for s in [
        &b"QQ==QQ=="[..],
        b"Q=QQQQQQ",
        b"====QQQQ",
        b"Q===QQQ=",
        b"=Q=Q=Q=Q",
    ] {
        diff(s, "err_09_interior_padding");
    }
}

/// Row 10: every out-of-range byte value crossing the FFI boundary. `char`
/// accepts any of the 255 non-NUL values; none of them may diverge, and none may
/// produce a NULL.
#[test]
fn err_10_every_single_byte() {
    let (cf, rf) = (c_decode(), rust_decode());
    for b in 1u8..=255 {
        let (c, r) = unsafe { (call(&cf, &[b]), call(&rf, &[b])) };
        assert_eq!(c, r, "divergence on single byte 0x{b:02x}");
        assert!(
            !matches!(c, Outcome::Null),
            "0x{b:02x} must not be rejected"
        );
    }
    // Also as a 4-byte group so the padding gates see the odd value.
    for b in 1u8..=255 {
        diff(&[b, b, b, b], "err_10 quad");
        diff(&[b'Q', b'Q', b, b'='], "err_10 at c3");
        diff(&[b'Q', b'Q', b'=', b], "err_10 at c4");
    }
}

/// Row 11: oversized length.
#[test]
fn err_11_oversized_input() {
    let mut rng = Rng::new(0x11_11);
    let big = rng.string(ALPHABET, 1 << 20);
    diff(&big, "err_11_oversized_input alphabet");
    diff(&vec![b'='; 1 << 20], "err_11_oversized_input padding");
    diff(&vec![b'!'; 1 << 20], "err_11_oversized_input all-ignored");
}

/// Generic boundary sweep: interior NUL truncation, plus lengths straddling the
/// group boundary, driven through the raw (already NUL-terminated) entry.
#[test]
fn err_12_interior_nul_truncates() {
    diff_raw(b"QQ\0QQQQ\0", "err_12 interior NUL");
    diff_raw(b"\0QQQQ\0", "err_12 leading NUL");
    diff_raw(b"QQQQ\0\0\0", "err_12 trailing NULs");
    for len in 0..40usize {
        let mut v = vec![b'Q'; len];
        v.push(0);
        v.extend_from_slice(b"IGNORED-TAIL\0");
        diff_raw(&v, "err_12 length sweep");
    }
}
