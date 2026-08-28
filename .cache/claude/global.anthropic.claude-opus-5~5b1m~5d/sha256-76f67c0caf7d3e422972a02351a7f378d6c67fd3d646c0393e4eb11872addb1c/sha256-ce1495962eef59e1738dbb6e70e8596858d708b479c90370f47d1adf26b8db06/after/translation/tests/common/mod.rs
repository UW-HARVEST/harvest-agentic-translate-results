//! Shared differential-test harness.
//!
//! Both the C reference `.so` and the Rust `.so` are loaded with `libloading`
//! and driven *only* through their exported symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test.

#![allow(dead_code)]

pub mod fork;

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// pixel type (must match the C `cp_pixel_t` layout exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub type ConvertPixFn = unsafe extern "C" fn(
    bpp: std::ffi::c_int,
    w: std::ffi::c_int,
    h: std::ffi::c_int,
    src: *mut u8,
    dst: *mut Pixel,
);

pub type CpInflateFn = unsafe extern "C" fn(
    r#in: *mut std::ffi::c_void,
    in_bytes: std::ffi::c_int,
    out: *mut std::ffi::c_void,
    out_bytes: std::ffi::c_int,
) -> std::ffi::c_int;

// ---------------------------------------------------------------------------
// locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory holding the C build tree (`c_src/build`).
fn c_build_dir() -> PathBuf {
    let m = manifest_dir();
    for cand in [
        m.join("..").join("c_src").join("build"),
        m.join("c_src").join("build"),
        m.join("..").join("..").join("c_src").join("build"),
    ] {
        if cand.is_dir() {
            return cand;
        }
    }
    panic!("could not locate c_src/build — build the C library first");
}

pub fn c_so_path() -> PathBuf {
    let dir = c_build_dir();
    let mut found: Option<PathBuf> = None;
    for e in std::fs::read_dir(&dir).expect("read c_src/build") {
        let p = e.expect("dir entry").path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no lib*.so in {}", dir.display()))
}

/// The Rust `.so` under test.
///
/// Defaults to the *release* cdylib, i.e. exactly the artifact the task's build
/// command (`cargo build --release`) produces.  Override with `CP_RUST_SO` to
/// point the whole suite at a different build (used to re-run everything
/// against the debug cdylib, where overflow checks are on).
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("CP_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "CP_RUST_SO={} does not exist", p.display());
        return p;
    }
    let m = manifest_dir();
    let release = m.join("target").join("release").join("libconvert_pix_lib.so");
    if release.is_file() {
        return release;
    }
    let status = Command::new(env!("CARGO"))
        .args(["build", "--offline", "--release"])
        .current_dir(&m)
        .status();
    if matches!(status, Ok(s) if s.success()) && release.is_file() {
        return release;
    }
    panic!(
        "could not locate {} — run `cargo build --release` first",
        release.display()
    );
}

pub struct Lib {
    pub lib: Library,
    pub name: &'static str,
}

impl Lib {
    fn open(path: &Path, name: &'static str) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        Lib { lib, name }
    }

    pub fn convert_pix(&self) -> Symbol<'_, ConvertPixFn> {
        unsafe { self.lib.get(b"convert_pix\0") }.expect("convert_pix")
    }

    pub fn cp_inflate(&self) -> Symbol<'_, CpInflateFn> {
        unsafe { self.lib.get(b"cp_inflate\0") }.expect("cp_inflate")
    }

    /// Raw address of an exported data symbol.
    ///
    /// `libloading`'s `Deref for Symbol<T>` reinterprets the *stored address*
    /// as a `T`; asking for `T = *mut U` therefore yields the symbol's address
    /// itself, which is what a data symbol needs (as opposed to a function
    /// symbol).
    pub fn data<T>(&self, sym: &[u8]) -> *mut T {
        let s: Symbol<'_, *mut T> = unsafe { self.lib.get(sym) }
            .unwrap_or_else(|e| panic!("{}: {e}", String::from_utf8_lossy(sym)));
        *s
    }

    pub fn error_reason(&self) -> Option<Vec<u8>> {
        let p: *mut *const std::ffi::c_char = self.data(b"cp_error_reason\0");
        unsafe {
            let s = *p;
            if s.is_null() {
                return None;
            }
            let mut n = 0usize;
            while *s.add(n) != 0 {
                n += 1;
            }
            Some(std::slice::from_raw_parts(s as *const u8, n).to_vec())
        }
    }

    pub fn set_error_reason_null(&self) {
        let p: *mut *const std::ffi::c_char = self.data(b"cp_error_reason\0");
        unsafe { *p = std::ptr::null() };
    }
}

/// The two libraries under test.  Loaded once per test process.
pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn pair() -> Pair {
    Pair {
        c: Lib::open(&c_so_path(), "C"),
        rs: Lib::open(&rust_so_path(), "Rust"),
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) — no external crates
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[0, n)` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        (self.next_u64() % n as u64) as u32
    }
    pub fn range(&mut self, lo: u32, hi_inclusive: u32) -> u32 {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
}

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

// ---------------------------------------------------------------------------
// 4-aligned buffer with a controllable pointer offset
// ---------------------------------------------------------------------------

/// Heap block whose payload starts at `base + offset` where `base` is 16-byte
/// aligned, so `first_bytes` inside `cp_inflate` is exactly `(4 - offset) % 4`.
pub struct AlignedBuf {
    raw: Vec<u64>,
    offset: usize,
    len: usize,
}

impl AlignedBuf {
    pub fn new(data: &[u8], offset: usize) -> AlignedBuf {
        let total = data.len() + offset + 16;
        let mut raw = vec![0u64; total / 8 + 2];
        let len = data.len();
        unsafe {
            let p = (raw.as_mut_ptr() as *mut u8).add(offset);
            std::ptr::copy_nonoverlapping(data.as_ptr(), p, len);
        }
        AlignedBuf { raw, offset, len }
    }
    pub fn zeroed(len: usize, offset: usize) -> AlignedBuf {
        AlignedBuf::new(&vec![0u8; len], offset)
    }
    pub fn ptr(&mut self) -> *mut u8 {
        unsafe { (self.raw.as_mut_ptr() as *mut u8).add(self.offset) }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whole backing store, so out-of-bounds writes by the library are visible
    /// and comparable instead of corrupting unrelated memory.
    pub fn all_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.raw.as_ptr() as *const u8, self.raw.len() * 8)
        }
    }
    pub fn payload(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts((self.raw.as_ptr() as *const u8).add(self.offset), self.len)
        }
    }
}

// ---------------------------------------------------------------------------
// DEFLATE bit writer + encoder (produces the streams the C decoder consumes)
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct BitWriter {
    pub bytes: Vec<u8>,
    nbits: u32, // bits used in the last byte
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter { bytes: Vec::new(), nbits: 0 }
    }
    pub fn bit(&mut self, b: u32) {
        if self.nbits == 0 {
            self.bytes.push(0);
            self.nbits = 8;
        }
        let last = self.bytes.len() - 1;
        let used = 8 - self.nbits;
        self.bytes[last] |= ((b & 1) as u8) << used;
        self.nbits -= 1;
    }
    /// Little-endian value (DEFLATE "extra bits" order).
    pub fn bits_lsb(&mut self, v: u32, n: u32) {
        for i in 0..n {
            self.bit((v >> i) & 1);
        }
    }
    /// Huffman code: most-significant bit first.
    pub fn code(&mut self, c: u32, n: u32) {
        for i in (0..n).rev() {
            self.bit((c >> i) & 1);
        }
    }
    pub fn align_byte(&mut self) {
        while self.nbits != 0 {
            self.bit(0);
        }
    }
    pub fn push_byte(&mut self, b: u8) {
        assert_eq!(self.nbits, 0, "not byte aligned");
        self.bytes.push(b);
    }
}

/// Canonical Huffman code assignment, byte-for-byte the same algorithm as the
/// C's `cp_build`, so codes emitted here are exactly the ones it decodes.
pub fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let mut counts = [0i32; 16];
    for &l in lens {
        assert!(l < 16);
        counts[l as usize] += 1;
    }
    let mut codes = [0i32; 16];
    counts[0] = 0;
    codes[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
    }
    let mut out = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[i] = codes[l as usize] as u32;
            codes[l as usize] += 1;
        }
    }
    out
}

// --- the fixed (BTYPE 1) code lengths, exactly as `cp_fixed_table` holds them
pub fn fixed_lit_lens() -> Vec<u8> {
    let mut v = vec![8u8; 288];
    for i in 144..256 {
        v[i] = 9;
    }
    for i in 256..280 {
        v[i] = 7;
    }
    for i in 280..288 {
        v[i] = 8;
    }
    v
}

pub fn fixed_dist_lens() -> Vec<u8> {
    vec![5u8; 32]
}

pub const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
pub const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
pub const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
pub const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

/// Length symbol index (0-based, i.e. `symbol - 257`) plus extra value.
pub fn length_code(len: u32) -> (usize, u32, u32) {
    assert!((3..=258).contains(&len));
    let mut idx = 0usize;
    for i in 0..29 {
        if LEN_BASE[i] <= len {
            idx = i;
        }
    }
    // symbol 285 (idx 28) is the exact-258 code with 0 extra bits
    if len == 258 {
        idx = 28;
    }
    (idx, len - LEN_BASE[idx], LEN_EXTRA[idx] as u32)
}

pub fn distance_code(dist: u32) -> (usize, u32, u32) {
    assert!((1..=32768).contains(&dist));
    let mut idx = 0usize;
    for i in 0..30 {
        if DIST_BASE[i] <= dist {
            idx = i;
        }
    }
    (idx, dist - DIST_BASE[idx], DIST_EXTRA[idx] as u32)
}

/// One item in a compressed block.
#[derive(Clone, Copy, Debug)]
pub enum Item {
    Lit(u8),
    /// (length, distance)
    Match(u32, u32),
    /// raw length symbol index (`symbol - 257`) + extra value, distance idx + extra
    RawMatch {
        len_idx: usize,
        len_extra: u32,
        dist_idx: usize,
        dist_extra: u32,
    },
}

/// The exact bytes a block of `items` decompresses to, given the already
/// produced output (needed for back-references).
pub fn expand(items: &[Item], out: &mut Vec<u8>) {
    for it in items {
        match *it {
            Item::Lit(b) => out.push(b),
            Item::Match(len, dist) => {
                let start = out.len() - dist as usize;
                for k in 0..len as usize {
                    let b = out[start + k];
                    out.push(b);
                }
            }
            Item::RawMatch { len_idx, len_extra, dist_idx, dist_extra } => {
                let len = LEN_BASE[len_idx] + len_extra;
                let dist = DIST_BASE[dist_idx] + dist_extra;
                let start = out.len() - dist as usize;
                for k in 0..len as usize {
                    let b = out[start + k];
                    out.push(b);
                }
            }
        }
    }
}

pub struct Huff {
    pub lens: Vec<u8>,
    pub codes: Vec<u32>,
}

impl Huff {
    pub fn new(lens: Vec<u8>) -> Huff {
        let codes = canonical_codes(&lens);
        Huff { lens, codes }
    }
    pub fn put(&self, w: &mut BitWriter, sym: usize) {
        let l = self.lens[sym];
        assert!(l != 0, "symbol {sym} has no code");
        w.code(self.codes[sym], l as u32);
    }
}

/// `cp_len_extra_bits` / `cp_dist_extra_bits` as the C declares them
/// (31 and 32 entries — the last one or two are padding zeros).
pub const LEN_EXTRA31: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];
pub const DIST_EXTRA32: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

/// Like [`write_items`] but with explicit extra-bit widths, so tests that
/// tamper with `cp_len_extra_bits` / `cp_dist_extra_bits` can emit a stream the
/// tampered decoder actually understands.
pub fn write_items_tables(
    w: &mut BitWriter,
    lit: &Huff,
    dst: &Huff,
    items: &[Item],
    len_extra_bits: &[u8],
    dist_extra_bits: &[u8],
) {
    for it in items {
        match *it {
            Item::Lit(b) => lit.put(w, b as usize),
            Item::RawMatch { len_idx, len_extra, dist_idx, dist_extra } => {
                lit.put(w, 257 + len_idx);
                w.bits_lsb(len_extra, len_extra_bits[len_idx] as u32);
                dst.put(w, dist_idx);
                w.bits_lsb(dist_extra, dist_extra_bits[dist_idx] as u32);
            }
            Item::Match(..) => panic!("Item::Match needs the standard tables; use RawMatch"),
        }
    }
    lit.put(w, 256);
}

pub fn write_items(w: &mut BitWriter, lit: &Huff, dst: &Huff, items: &[Item]) {
    for it in items {
        match *it {
            Item::Lit(b) => lit.put(w, b as usize),
            Item::Match(len, dist) => {
                let (li, lx, ln) = length_code(len);
                lit.put(w, 257 + li);
                w.bits_lsb(lx, ln);
                let (di, dx, dn) = distance_code(dist);
                dst.put(w, di);
                w.bits_lsb(dx, dn);
            }
            Item::RawMatch { len_idx, len_extra, dist_idx, dist_extra } => {
                lit.put(w, 257 + len_idx);
                w.bits_lsb(len_extra, LEN_EXTRA[len_idx] as u32);
                dst.put(w, dist_idx);
                w.bits_lsb(dist_extra, DIST_EXTRA[dist_idx] as u32);
            }
        }
    }
    lit.put(w, 256); // end of block
}

/// Emit a BTYPE=1 (fixed Huffman) block.
pub fn write_fixed_block(w: &mut BitWriter, bfinal: bool, items: &[Item]) {
    w.bit(bfinal as u32);
    w.bits_lsb(1, 2); // BTYPE = 1
    let lit = Huff::new(fixed_lit_lens());
    let dst = Huff::new(fixed_dist_lens());
    write_items(w, &lit, &dst, items);
}

pub const PERMUTATION_ORDER: [usize; 19] =
    [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// Code-length-alphabet symbols to emit for the concatenated
/// `lit_lens ++ dst_lens` sequence.
#[derive(Clone, Copy, Debug)]
pub enum ClSym {
    Lit(u8),
    /// repeat previous, 3..=6
    Rep16(u32),
    /// zeros, 3..=10
    Rep17(u32),
    /// zeros, 11..=138
    Rep18(u32),
}

/// Emit a BTYPE=2 (dynamic Huffman) block.
///
/// `cl_syms` must expand to exactly `lit_lens ++ dst_lens`; this is checked.
/// `perm` is the permutation table the decoder will use (normally
/// `PERMUTATION_ORDER`, but tests may tamper with `cp_permutation_order`).
pub fn write_dynamic_block(
    w: &mut BitWriter,
    bfinal: bool,
    lit_lens: &[u8],
    dst_lens: &[u8],
    cl_syms: &[ClSym],
    cl_lens: &[u8; 19],
    nlen: usize,
    perm: &[usize; 19],
    items: &[Item],
) {
    // sanity: the code-length stream must reproduce lit_lens ++ dst_lens
    let mut expanded: Vec<u8> = Vec::new();
    for s in cl_syms {
        match *s {
            ClSym::Lit(v) => expanded.push(v),
            ClSym::Rep16(n) => {
                let prev = *expanded.last().expect("rep16 with no previous");
                for _ in 0..n {
                    expanded.push(prev);
                }
            }
            ClSym::Rep17(n) | ClSym::Rep18(n) => {
                for _ in 0..n {
                    expanded.push(0);
                }
            }
        }
    }
    let mut want: Vec<u8> = lit_lens.to_vec();
    want.extend_from_slice(dst_lens);
    assert_eq!(expanded, want, "code-length stream does not match the tables");

    assert!((4..=19).contains(&nlen));
    // every code-length symbol actually used must have a code
    for s in cl_syms {
        let idx = match *s {
            ClSym::Lit(v) => v as usize,
            ClSym::Rep16(_) => 16,
            ClSym::Rep17(_) => 17,
            ClSym::Rep18(_) => 18,
        };
        assert!(cl_lens[idx] != 0, "cl symbol {idx} has no code");
        // and must be inside the transmitted prefix of the permutation
        let pos = perm[..nlen].iter().position(|&p| p == idx);
        assert!(pos.is_some(), "cl symbol {idx} not transmitted (nlen={nlen})");
    }

    w.bit(bfinal as u32);
    w.bits_lsb(2, 2); // BTYPE = 2
    w.bits_lsb((lit_lens.len() - 257) as u32, 5);
    w.bits_lsb((dst_lens.len() - 1) as u32, 5);
    w.bits_lsb((nlen - 4) as u32, 4);
    for i in 0..nlen {
        w.bits_lsb(cl_lens[perm[i]] as u32, 3);
    }
    let cl = Huff::new(cl_lens.to_vec());
    for s in cl_syms {
        match *s {
            ClSym::Lit(v) => cl.put(w, v as usize),
            ClSym::Rep16(n) => {
                cl.put(w, 16);
                w.bits_lsb(n - 3, 2);
            }
            ClSym::Rep17(n) => {
                cl.put(w, 17);
                w.bits_lsb(n - 3, 3);
            }
            ClSym::Rep18(n) => {
                cl.put(w, 18);
                w.bits_lsb(n - 11, 7);
            }
        }
    }
    let lit = Huff::new(lit_lens.to_vec());
    let dst = Huff::new(dst_lens.to_vec());
    write_items(w, &lit, &dst, items);
}

/// Trivial (no-RLE) code-length stream: every entry as a literal symbol.
pub fn cl_stream_literal(lit_lens: &[u8], dst_lens: &[u8]) -> Vec<ClSym> {
    lit_lens
        .iter()
        .chain(dst_lens.iter())
        .map(|&v| ClSym::Lit(v))
        .collect()
}

/// Code lengths for the code-length alphabet covering exactly the symbols used
/// by `cl_syms`, as a complete-enough canonical tree.
pub fn cl_lens_for(cl_syms: &[ClSym]) -> ([u8; 19], usize) {
    let mut used = [false; 19];
    for s in cl_syms {
        let idx = match *s {
            ClSym::Lit(v) => v as usize,
            ClSym::Rep16(_) => 16,
            ClSym::Rep17(_) => 17,
            ClSym::Rep18(_) => 18,
        };
        used[idx] = true;
    }
    let syms: Vec<usize> = (0..19).filter(|&i| used[i]).collect();
    let mut lens = [0u8; 19];
    // A complete binary tree over `k` symbols needs a mix of lengths; use the
    // simple "balanced" assignment: with k symbols, give the first
    // `2^ceil(log2 k) - k` symbols length ceil(log2 k)-1 ... instead, build a
    // canonical complete tree by repeatedly splitting.
    let k = syms.len().max(1);
    let mut depth = 0u32;
    while (1usize << depth) < k {
        depth += 1;
    }
    if depth == 0 {
        // single symbol: give it length 1 (an incomplete tree, which is what a
        // real encoder emits for HDIST=1 too)
        lens[syms[0]] = 1;
    } else {
        let full = 1usize << depth;
        let short_count = full - k; // symbols that get `depth - 1` bits
        for (i, &s) in syms.iter().enumerate() {
            lens[s] = if i < short_count && depth >= 1 {
                (depth - 1) as u8
            } else {
                depth as u8
            };
        }
        // The Kraft sum must be exactly 1.  With `short_count` symbols at
        // depth-1 and the rest at depth this holds by construction.
    }
    // nlen: smallest transmitted prefix covering every used symbol
    let mut nlen = 4usize;
    for (pos, &p) in PERMUTATION_ORDER.iter().enumerate() {
        if used[p] {
            nlen = nlen.max(pos + 1);
        }
    }
    (lens, nlen)
}

/// Build literal/length code lengths that give a *complete* canonical tree over
/// the symbols in `used` (which must contain 256).
pub fn balanced_lens(nsym: usize, used: &[usize]) -> Vec<u8> {
    let mut u: Vec<usize> = used.to_vec();
    u.sort_unstable();
    u.dedup();
    assert!(u.iter().all(|&s| s < nsym));
    let mut lens = vec![0u8; nsym];
    let k = u.len();
    if k == 1 {
        lens[u[0]] = 1;
        return lens;
    }
    let mut depth = 0u32;
    while (1usize << depth) < k {
        depth += 1;
    }
    let full = 1usize << depth;
    let short_count = full - k;
    for (i, &s) in u.iter().enumerate() {
        lens[s] = if i < short_count { (depth - 1) as u8 } else { depth as u8 };
    }
    lens
}

/// Build a BTYPE=0 (stored) stream: exactly `5 + payload.len()` bytes, which is
/// what the C's `s->bits_left / 8 <= LEN` check accepts.
pub fn stored_stream(payload: &[u8], bfinal: bool) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.bit(bfinal as u32);
    w.bits_lsb(0, 2); // BTYPE = 0
    w.align_byte();
    let len = payload.len() as u16;
    let nlen = !len;
    w.push_byte((len & 0xFF) as u8);
    w.push_byte((len >> 8) as u8);
    w.push_byte((nlen & 0xFF) as u8);
    w.push_byte((nlen >> 8) as u8);
    for &b in payload {
        w.push_byte(b);
    }
    w.bytes
}

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

/// Run `cp_inflate` on both libraries with identical inputs (identical pointer
/// offsets too) and assert the return value, the output buffer *and* the
/// `cp_error_reason` string agree.
pub fn diff_inflate(
    p: &Pair,
    stream: &[u8],
    in_offset: usize,
    out_len: usize,
    out_offset: usize,
    label: &str,
) -> (i32, Vec<u8>) {
    let f_c = p.c.cp_inflate();
    let f_rs = p.rs.cp_inflate();

    p.c.set_error_reason_null();
    p.rs.set_error_reason_null();

    let mut in_c = AlignedBuf::new(stream, in_offset);
    let mut in_rs = AlignedBuf::new(stream, in_offset);
    let mut out_c = AlignedBuf::zeroed(out_len, out_offset);
    let mut out_rs = AlignedBuf::zeroed(out_len, out_offset);

    let n = stream.len() as std::ffi::c_int;
    let rc_c = unsafe {
        f_c(
            in_c.ptr() as *mut std::ffi::c_void,
            n,
            out_c.ptr() as *mut std::ffi::c_void,
            out_len as std::ffi::c_int,
        )
    };
    let rc_rs = unsafe {
        f_rs(
            in_rs.ptr() as *mut std::ffi::c_void,
            n,
            out_rs.ptr() as *mut std::ffi::c_void,
            out_len as std::ffi::c_int,
        )
    };

    assert_eq!(rc_c, rc_rs, "[{label}] return value differs");
    assert_eq!(
        out_c.all_bytes(),
        out_rs.all_bytes(),
        "[{label}] output buffer differs (rc={rc_c})"
    );
    assert_eq!(in_c.all_bytes(), in_rs.all_bytes(), "[{label}] input buffer mutated differently");
    let e_c = p.c.error_reason();
    let e_rs = p.rs.error_reason();
    assert_eq!(
        e_c.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        e_rs.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        "[{label}] cp_error_reason differs"
    );
    (rc_c, out_c.payload().to_vec())
}

/// Same, but also asserts the decompressed payload equals `expect`.
pub fn diff_inflate_expect(p: &Pair, stream: &[u8], expect: &[u8], label: &str) {
    for in_off in [0usize, 1, 2, 3] {
        let (rc, got) = diff_inflate(p, stream, in_off, expect.len(), 0, &format!("{label}/off{in_off}"));
        assert_eq!(rc, 1, "[{label}/off{in_off}] expected success");
        assert_eq!(got, expect, "[{label}/off{in_off}] payload mismatch vs encoder model");
    }
}

pub fn diff_convert_pix(
    p: &Pair,
    bpp: i32,
    w: i32,
    h: i32,
    src: &[u8],
    dst_pixels: usize,
    label: &str,
) {
    let f_c = p.c.convert_pix();
    let f_rs = p.rs.convert_pix();
    let mut src_c = src.to_vec();
    let mut src_rs = src.to_vec();
    let mut dst_c = vec![Pixel { r: 0xAA, g: 0xBB, b: 0xCC, a: 0xDD }; dst_pixels + 8];
    let mut dst_rs = dst_c.clone();
    unsafe {
        f_c(bpp, w, h, src_c.as_mut_ptr(), dst_c.as_mut_ptr());
        f_rs(bpp, w, h, src_rs.as_mut_ptr(), dst_rs.as_mut_ptr());
    }
    assert_eq!(dst_c, dst_rs, "[{label}] dst differs (bpp={bpp} w={w} h={h})");
    assert_eq!(src_c, src_rs, "[{label}] src mutated differently");
}
