//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols — the Rust functions are never
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]
#![allow(clippy::int_plus_one, clippy::collapsible_if)]

pub mod child;

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

pub type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
pub type FilterFn = unsafe extern "C" fn(*const c_char, u8) -> *mut c_char;
/// Same function, but the `_Bool` argument is declared 64 bit wide so a test
/// can put garbage in the upper bits of the argument register.
pub type FilterFnWide = unsafe extern "C" fn(*const c_char, u64) -> *mut c_char;

unsafe extern "C" {
    fn free(p: *mut c_void);
    fn malloc_usable_size(p: *mut c_void) -> usize;
    fn strlen(s: *const c_char) -> usize;
}

// ---------------------------------------------------------------------------
// library loading
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub drop_: DropFn,
    pub filter: FilterFn,
    pub filter_wide: FilterFnWide,
    _lib: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libdriver.so")
}

/// `target/<profile>/libdriver.so`, derived from the location of the running
/// test binary (`target/<profile>/deps/<test>-<hash>`), with a fallback scan.
pub fn rust_so_path() -> PathBuf {
    let name = "libdriver.so";
    // `DIFF_RUST_SO` lets the same suite be run against the debug *and* the
    // release cdylib (the release one is the shipped artifact and is built with
    // `panic = "abort"` + optimisations, i.e. a different code path through the
    // compiler).
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            let a = deps.join(name);
            if a.is_file() {
                return a;
            }
            if let Some(profile) = deps.parent() {
                let b = profile.join(name);
                if b.is_file() {
                    return b;
                }
            }
        }
    }
    for profile in ["debug", "release"] {
        let p = manifest_dir().join("target").join(profile).join(name);
        if p.is_file() {
            return p;
        }
    }
    panic!("could not locate the Rust cdylib {name}; run `cargo build` first");
}

/// Load exactly one implementation — used by the child processes, which must
/// not have the other library mapped (its allocations would pollute the trace).
pub fn load_one(which: &str) -> Impl {
    match which {
        "c" | "C" => load("C", &c_so_path()),
        "rust" | "RUST" => load("RUST", &rust_so_path()),
        other => panic!("unknown implementation selector {other:?}"),
    }
}

pub fn load(name: &'static str, path: &Path) -> Impl {
    assert!(
        path.is_file(),
        "shared object {} does not exist — build it first",
        path.display()
    );
    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let drop_: DropFn = {
            let s: Symbol<DropFn> = lib
                .get(b"w_utf8_drop\0")
                .unwrap_or_else(|e| panic!("{} has no w_utf8_drop: {e}", path.display()));
            *s
        };
        let filter: FilterFn = {
            let s: Symbol<FilterFn> = lib
                .get(b"w_utf8_filter\0")
                .unwrap_or_else(|e| panic!("{} has no w_utf8_filter: {e}", path.display()));
            *s
        };
        let filter_wide: FilterFnWide = std::mem::transmute::<FilterFn, FilterFnWide>(filter);
        Impl {
            name,
            drop_,
            filter,
            filter_wide,
            _lib: lib,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn pair() -> Pair {
    Pair {
        c: load("C", &c_so_path()),
        rs: load("RUST", &rust_so_path()),
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Build a NUL-terminated buffer with 16 extra zero bytes of padding so that a
/// hypothetical over-read stays inside our allocation (both implementations see
/// exactly the same bytes).
pub fn cstr_buf(bytes: &[u8]) -> Vec<u8> {
    assert!(
        !bytes.contains(&0),
        "test inputs must not contain interior NUL bytes"
    );
    let mut v = Vec::with_capacity(bytes.len() + 16);
    v.extend_from_slice(bytes);
    v.extend_from_slice(&[0u8; 16]);
    v
}

/// Like `cstr_buf` but tolerates interior NUL bytes (used by the row that
/// checks that both implementations honour `strlen` semantics).
pub fn cstr_buf_raw(bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes.len() + 16);
    v.extend_from_slice(bytes);
    v.extend_from_slice(&[0u8; 16]);
    v
}

/// Differential comparison on a buffer that may contain interior NULs, and at
/// an arbitrary offset inside a larger allocation (unaligned start pointer).
pub fn diff_raw_at(p: &Pair, bytes: &[u8], offset: usize, replacement: u8) {
    let mut buf = vec![0x41u8; offset];
    buf.extend_from_slice(bytes);
    buf.extend_from_slice(&[0u8; 16]);
    let base = unsafe { (buf.as_ptr() as *const c_char).add(offset) };
    unsafe {
        let co = (p.c.drop_)(base);
        let ro = (p.rs.drop_)(base);
        assert_eq!(
            co as usize - base as usize,
            ro as usize - base as usize,
            "w_utf8_drop offset mismatch at start offset {offset} for [{}]",
            hex_trunc(bytes, 64)
        );
        let cp = (p.c.filter)(base, replacement);
        let rp = (p.rs.filter)(base, replacement);
        assert_eq!(cp.is_null(), rp.is_null());
        if cp.is_null() {
            return;
        }
        let cs = std::slice::from_raw_parts(cp as *const u8, strlen(cp)).to_vec();
        let rs = std::slice::from_raw_parts(rp as *const u8, strlen(rp)).to_vec();
        free(cp.cast());
        free(rp.cast());
        assert_eq!(
            cs,
            rs,
            "w_utf8_filter mismatch at start offset {offset} (replacement={replacement}) \
             for [{}]",
            hex_trunc(bytes, 64)
        );
    }
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{b:02X}"));
    }
    s
}

pub fn hex_trunc(bytes: &[u8], max: usize) -> String {
    if bytes.len() <= max {
        hex(bytes)
    } else {
        format!("{} … (+{} bytes)", hex(&bytes[..max]), bytes.len() - max)
    }
}

unsafe fn out_bytes(p: *const c_char) -> Vec<u8> {
    unsafe {
        let n = strlen(p);
        std::slice::from_raw_parts(p as *const u8, n).to_vec()
    }
}

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

/// `w_utf8_drop`: both implementations get the *same* pointer, so the returned
/// pointers must be bit-identical. Compared as offsets for a readable message.
pub fn diff_drop(p: &Pair, bytes: &[u8]) {
    let buf = cstr_buf(bytes);
    let base = buf.as_ptr() as *const c_char;
    let (co, ro) = unsafe { ((p.c.drop_)(base), (p.rs.drop_)(base)) };
    let coff = (co as usize).wrapping_sub(base as usize);
    let roff = (ro as usize).wrapping_sub(base as usize);
    assert_eq!(
        coff,
        roff,
        "w_utf8_drop offset mismatch: C={coff} RUST={roff} for input [{}]",
        hex_trunc(bytes, 64)
    );
    assert!(
        coff <= bytes.len(),
        "w_utf8_drop returned a pointer past the terminator ({coff} > {}) for [{}]",
        bytes.len(),
        hex_trunc(bytes, 64)
    );
}

/// `w_utf8_filter`: compare NULL-ness, the NUL-terminated payload and the
/// usable size of the returned heap block.
pub fn diff_filter(p: &Pair, bytes: &[u8], replacement: u8) {
    let buf = cstr_buf(bytes);
    let base = buf.as_ptr() as *const c_char;
    unsafe {
        let cp = (p.c.filter)(base, replacement);
        let rp = (p.rs.filter)(base, replacement);
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "w_utf8_filter NULL-ness mismatch (C null={}, RUST null={}) \
             replacement={replacement} input [{}]",
            cp.is_null(),
            rp.is_null(),
            hex_trunc(bytes, 64)
        );
        if cp.is_null() {
            return;
        }
        let cs = out_bytes(cp);
        let rs = out_bytes(rp);
        if cs != rs {
            let msg = format!(
                "w_utf8_filter output mismatch (replacement={replacement})\n  input: [{}]\n  C   : [{}]\n  RUST: [{}]",
                hex_trunc(bytes, 96),
                hex_trunc(&cs, 96),
                hex_trunc(&rs, 96)
            );
            free(cp.cast());
            free(rp.cast());
            panic!("{msg}");
        }
        // Heap-overflow detector: the implementation wrote cs.len()+1 bytes
        // (payload + NUL) into the block, so the block must be at least that
        // big. `malloc_usable_size` is *not* usable as an equality oracle
        // (it depends on which chunk the allocator happened to recycle) — the
        // exact malloc/realloc/strdup request sequence is compared instead by
        // the `alloc_trace_*` tests, which record it with an LD_PRELOAD
        // interposer in a child process.
        let cu = malloc_usable_size(cp.cast());
        let ru = malloc_usable_size(rp.cast());
        let need = cs.len() + 1;
        free(cp.cast());
        free(rp.cast());
        assert!(
            cu >= need,
            "C w_utf8_filter overflowed its buffer: usable={cu} < {need}"
        );
        assert!(
            ru >= need,
            "RUST w_utf8_filter overflowed its buffer: usable={ru} < {need} \
             (replacement={replacement}) input [{}]",
            hex_trunc(bytes, 64)
        );
    }
}

/// `w_utf8_filter` called through a 64-bit-wide `_Bool` argument.
pub fn diff_filter_wide(p: &Pair, bytes: &[u8], replacement: u64) {
    let buf = cstr_buf(bytes);
    let base = buf.as_ptr() as *const c_char;
    unsafe {
        let cp = (p.c.filter_wide)(base, replacement);
        let rp = (p.rs.filter_wide)(base, replacement);
        assert_eq!(cp.is_null(), rp.is_null());
        if cp.is_null() {
            return;
        }
        let cs = out_bytes(cp);
        let rs = out_bytes(rp);
        let eq = cs == rs;
        let cu = malloc_usable_size(cp.cast());
        let ru = malloc_usable_size(rp.cast());
        free(cp.cast());
        free(rp.cast());
        assert!(
            eq,
            "wide-bool w_utf8_filter mismatch (replacement=0x{replacement:X})\n  \
             input [{}]\n  C   : [{}]\n  RUST: [{}]",
            hex_trunc(bytes, 96),
            hex_trunc(&cs, 96),
            hex_trunc(&rs, 96)
        );
        assert!(cu >= cs.len() + 1 && ru >= rs.len() + 1, "buffer overflow");
    }
}

/// Exercise every entry point on one input: the low-level scanner and the
/// high-level filter with false / true / non-canonical-true.
pub fn diff_all(p: &Pair, bytes: &[u8]) {
    diff_drop(p, bytes);
    for r in [0u8, 1, 0xFF] {
        diff_filter(p, bytes, r);
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (splitmix64)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as u8
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// input generators
// ---------------------------------------------------------------------------

/// Bytes that sit exactly on the decision boundaries of `valid_1`..`valid_4`.
pub const INTERESTING: &[u8] = &[
    0x01, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0, 0xC1, 0xC2, 0xDF, 0xE0, 0xE1, 0xEC, 0xED,
    0xEE, 0xEF, 0xF0, 0xF1, 0xF4, 0xF5, 0xF7, 0xF8, 0xFF,
];

pub fn gen_ascii(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.range_u8(0x01, 0x7F)).collect()
}

pub fn gen_uniform(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.range_u8(0x01, 0xFF)).collect()
}

pub fn gen_interesting(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.pick(INTERESTING)).collect()
}

/// Append one *valid* UTF-8 sequence of the requested width (1..=4).
pub fn push_valid(out: &mut Vec<u8>, rng: &mut Rng, width: u8) {
    match width {
        1 => out.push(rng.range_u8(0x01, 0x7F)),
        2 => {
            out.push(rng.range_u8(0xC2, 0xDF));
            out.push(rng.range_u8(0x80, 0xBF));
        }
        3 => {
            let b0 = rng.range_u8(0xE0, 0xEF);
            let b1 = match b0 {
                0xE0 => rng.range_u8(0xA0, 0xBF),
                0xED => rng.range_u8(0x80, 0x9F),
                _ => rng.range_u8(0x80, 0xBF),
            };
            out.push(b0);
            out.push(b1);
            out.push(rng.range_u8(0x80, 0xBF));
        }
        4 => {
            let b0 = rng.range_u8(0xF0, 0xF4);
            let b1 = match b0 {
                0xF0 => rng.range_u8(0x90, 0xBF),
                0xF4 => rng.range_u8(0x80, 0x8F),
                _ => rng.range_u8(0x80, 0xBF),
            };
            out.push(b0);
            out.push(b1);
            out.push(rng.range_u8(0x80, 0xBF));
            out.push(rng.range_u8(0x80, 0xBF));
        }
        _ => unreachable!(),
    }
}

pub fn gen_valid(rng: &mut Rng, nseq: usize, widths: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..nseq {
        let w = rng.pick(widths);
        push_valid(&mut v, rng, w);
    }
    v
}

/// `gen_valid` with a random sequence count in `0..max` (avoids nested
/// borrows of the RNG at call sites).
pub fn gen_valid_n(rng: &mut Rng, max: usize) -> Vec<u8> {
    let n = rng.below(max);
    gen_valid(rng, n, &[1, 2, 3, 4])
}

/// The distinct classes of *invalid* byte sequences the C code rejects.
pub const INVALID_CLASSES: usize = 12;

/// Append one invalid sequence of the given class (0..INVALID_CLASSES).
pub fn push_invalid(out: &mut Vec<u8>, rng: &mut Rng, class: usize) {
    match class % INVALID_CLASSES {
        // bare continuation byte
        0 => out.push(rng.range_u8(0x80, 0xBF)),
        // overlong 2-byte lead
        1 => {
            out.push(rng.pick(&[0xC0u8, 0xC1]));
            out.push(rng.range_u8(0x80, 0xBF));
        }
        // 2-byte lead with bad continuation
        2 => {
            out.push(rng.range_u8(0xC2, 0xDF));
            out.push(rng.range_u8(0x01, 0x7F));
        }
        // 3-byte lead, bad first continuation
        3 => {
            out.push(rng.range_u8(0xE0, 0xEF));
            out.push(rng.range_u8(0xC0, 0xFF));
        }
        // 3-byte lead, bad second continuation
        4 => {
            out.push(rng.range_u8(0xE1, 0xEC));
            out.push(rng.range_u8(0x80, 0xBF));
            out.push(rng.range_u8(0x01, 0x7F));
        }
        // overlong 3-byte (E0 80..9F)
        5 => {
            out.push(0xE0);
            out.push(rng.range_u8(0x80, 0x9F));
            out.push(rng.range_u8(0x80, 0xBF));
        }
        // surrogate (ED A0..BF)
        6 => {
            out.push(0xED);
            out.push(rng.range_u8(0xA0, 0xBF));
            out.push(rng.range_u8(0x80, 0xBF));
        }
        // 4-byte lead above 0xF4
        7 => {
            out.push(rng.range_u8(0xF5, 0xF7));
            out.push(rng.range_u8(0x80, 0xBF));
            out.push(rng.range_u8(0x80, 0xBF));
            out.push(rng.range_u8(0x80, 0xBF));
        }
        // overlong 4-byte (F0 80..8F)
        8 => {
            out.push(0xF0);
            out.push(rng.range_u8(0x80, 0x8F));
            out.push(rng.range_u8(0x80, 0xBF));
            out.push(rng.range_u8(0x80, 0xBF));
        }
        // above U+10FFFF (F4 90..BF)
        9 => {
            out.push(0xF4);
            out.push(rng.range_u8(0x90, 0xBF));
            out.push(rng.range_u8(0x80, 0xBF));
            out.push(rng.range_u8(0x80, 0xBF));
        }
        // 0xF8..0xFF — matches no lead-byte mask at all
        10 => out.push(rng.range_u8(0xF8, 0xFF)),
        // 4-byte lead with bad third continuation
        _ => {
            out.push(rng.range_u8(0xF1, 0xF3));
            out.push(rng.range_u8(0x80, 0xBF));
            out.push(rng.range_u8(0x80, 0xBF));
            out.push(rng.range_u8(0x01, 0x7F));
        }
    }
}

/// Mixed valid sequences with invalid sequences of every class injected.
pub fn gen_mixed(rng: &mut Rng, nseq: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..nseq {
        if rng.below(3) == 0 {
            let class = rng.below(INVALID_CLASSES);
            push_invalid(&mut v, rng, class);
        } else {
            let w = rng.pick(&[1u8, 2, 3, 4]);
            push_valid(&mut v, rng, w);
        }
    }
    v
}

/// UTF-8 encoding of a scalar value (used for the code-point boundary table).
pub fn encode_utf8(cp: u32) -> Vec<u8> {
    match cp {
        0x0000..=0x007F => vec![cp as u8],
        0x0080..=0x07FF => vec![0xC0 | (cp >> 6) as u8, 0x80 | (cp & 0x3F) as u8],
        0x0800..=0xFFFF => vec![
            0xE0 | (cp >> 12) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ],
        _ => vec![
            0xF0 | (cp >> 18) as u8,
            0x80 | ((cp >> 12) & 0x3F) as u8,
            0x80 | ((cp >> 6) & 0x3F) as u8,
            0x80 | (cp & 0x3F) as u8,
        ],
    }
}

pub const BOUNDARY_CODEPOINTS: &[u32] = &[
    0x0001, 0x007F, 0x0080, 0x07FF, 0x0800, 0x0FFF, 0x1000, 0xD7FF, 0xE000, 0xFFFD, 0xFFFF,
    0x1_0000, 0x3_FFFF, 0x4_0000, 0xF_FFFF, 0x10_0000, 0x10_FFFF,
];
