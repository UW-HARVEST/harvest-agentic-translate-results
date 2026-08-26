//! Shared differential-testing harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! are only ever called through their exported C symbols — the Rust crate is
//! never linked directly, so the `#[no_mangle]` export wrappers are part of
//! what is under test.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirror of include/lib.h
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut u8,
}

pub type LoadPngMemFn = unsafe extern "C" fn(*const u8, c_int) -> CpImage;
pub type CpInflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

/// One loaded implementation (`c` or `rust`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub load_png_mem: LoadPngMemFn,
    pub cp_inflate: CpInflateFn,
    /// address of the `const char *cp_error_reason;` global
    pub err: *mut *const c_char,
    pub fixed_table: *mut u8,
    pub permutation_order: *mut u8,
    pub len_extra_bits: *mut u8,
    pub len_base: *mut u8,
    pub dist_extra_bits: *mut u8,
    pub dist_base: *mut u8,
}

impl Impl {
    fn open(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            macro_rules! data {
                ($sym:expr) => {
                    *lib
                        .get::<*mut u8>($sym)
                        .unwrap_or_else(|e| panic!("{} missing {:?}: {e}", name, $sym))
                };
            }
            let load_png_mem = *lib
                .get::<LoadPngMemFn>(b"load_png_mem\0")
                .expect("load_png_mem");
            let cp_inflate = *lib.get::<CpInflateFn>(b"cp_inflate\0").expect("cp_inflate");
            let err = *lib
                .get::<*mut *const c_char>(b"cp_error_reason\0")
                .expect("cp_error_reason");
            let i = Impl {
                name,
                load_png_mem,
                cp_inflate,
                err,
                fixed_table: data!(b"cp_fixed_table\0"),
                permutation_order: data!(b"cp_permutation_order\0"),
                len_extra_bits: data!(b"cp_len_extra_bits\0"),
                len_base: data!(b"cp_len_base\0"),
                dist_extra_bits: data!(b"cp_dist_extra_bits\0"),
                dist_base: data!(b"cp_dist_base\0"),
                _lib: lib,
            };
            i
        }
    }

    pub fn clear_error(&self) {
        unsafe { *self.err = std::ptr::null() }
    }

    /// Current value of `cp_error_reason`, as an owned string.
    pub fn error(&self) -> Option<String> {
        unsafe {
            let p = *self.err;
            if p.is_null() {
                None
            } else {
                Some(CStr::from_ptr(p).to_string_lossy().into_owned())
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

// The raw pointers inside `Impl` are addresses of globals inside the two
// dlopen'ed shared objects; they stay valid for the whole process lifetime.
// Access from several test threads is fine because every test uses its own
// input/output buffers and the only shared mutable state is
// `cp_error_reason`, which the differential drivers serialise (see
// `ERROR_LOCK`).
unsafe impl Send for Impl {}
unsafe impl Sync for Impl {}

/// `cp_error_reason` is a process-wide global inside each `.so`, so the
/// clear/call/read sequence has to be serialised across test threads.
pub static ERROR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` of the currently running test binary.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf()
}

fn newest(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .filter(|p| p.is_file())
        .max_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .cloned()
}

pub fn rust_so_path() -> PathBuf {
    let d = profile_dir();
    let cands = [
        d.join("libload_png_mem_lib.so"),
        d.join("deps/libload_png_mem_lib.so"),
    ];
    newest(&cands).unwrap_or_else(|| {
        panic!(
            "the Rust cdylib was not found; looked in {:?}. Run `cargo build` first.",
            cands
        )
    })
}

pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().join("c_src/build");
    let so = build.join("libtranslated_rust.so");
    if !so.is_file() {
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let ok = Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            && Command::new("cmake")
                .arg("--build")
                .arg(".")
                .current_dir(&build)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        assert!(ok && so.is_file(), "failed to build the C shared library");
    }
    so
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::open("c", &c_so_path()),
        rust: Impl::open("rust", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// deterministic PRNG (splitmix64) — reproducible, fixed seeds
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xDEAD_BEEF_CAFE_F00D)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    pub fn u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
    /// uniform in `0..n`
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
    /// bytes drawn from a small alphabet -> highly compressible
    pub fn repetitive(&mut self, n: usize, alphabet: u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let v = (self.u8() % alphabet.max(1)) as u8;
            let run = 1 + (self.u8() % 40) as usize;
            for _ in 0..run {
                if out.len() == n {
                    break;
                }
                out.push(v);
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// bit writer (DEFLATE bit order: LSB first for values, MSB first for codes)
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub out: Vec<u8>,
    cur: u32,
    n: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter {
            out: Vec::new(),
            cur: 0,
            n: 0,
        }
    }
    /// `n` bits of `v`, least significant bit first (DEFLATE "value" order).
    pub fn bits(&mut self, v: u32, n: u32) {
        for i in 0..n {
            let b = (v >> i) & 1;
            self.cur |= b << self.n;
            self.n += 1;
            if self.n == 8 {
                self.out.push(self.cur as u8);
                self.cur = 0;
                self.n = 0;
            }
        }
    }
    /// `n` bits of a Huffman code, most significant bit first.
    pub fn code(&mut self, c: u32, n: u32) {
        for i in (0..n).rev() {
            self.bits((c >> i) & 1, 1);
        }
    }
    pub fn align(&mut self) {
        if self.n != 0 {
            self.out.push(self.cur as u8);
            self.cur = 0;
            self.n = 0;
        }
    }
    pub fn raw(&mut self, b: &[u8]) {
        assert_eq!(self.n, 0, "raw() requires byte alignment");
        self.out.extend_from_slice(b);
    }
    /// append `n` zero *bits* (used to pad a stream out to a wanted length)
    pub fn raw_pad(&mut self, n: u32) {
        for _ in 0..n {
            self.bits(0, 1);
        }
    }
    pub fn finish(mut self) -> Vec<u8> {
        self.align();
        self.out
    }
}

// ---------------------------------------------------------------------------
// DEFLATE tables (mirror of cp_len_base & friends)
// ---------------------------------------------------------------------------

pub const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
pub const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
pub const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049,
    3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
pub const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
pub const PERM: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// `(litlen symbol, extra value, extra bit count)` for a match length 3..=258.
pub fn len_sym(l: u32) -> (u32, u32, u32) {
    assert!((3..=258).contains(&l));
    let mut i = 28;
    while LEN_BASE[i] > l {
        i -= 1;
    }
    (257 + i as u32, l - LEN_BASE[i], LEN_EXTRA[i])
}

/// `(dist symbol, extra value, extra bit count)` for a distance 1..=32768.
pub fn dist_sym(d: u32) -> (u32, u32, u32) {
    assert!((1..=32768).contains(&d));
    let mut i = 29;
    while DIST_BASE[i] > d {
        i -= 1;
    }
    (i as u32, d - DIST_BASE[i], DIST_EXTRA[i])
}

#[derive(Clone, Copy, Debug)]
pub enum Tok {
    Lit(u8),
    Match { len: u32, dist: u32 },
}

/// Expand tokens to the bytes they decode to (the expected inflate output).
pub fn expand(toks: &[Tok]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::Match { len, dist } => {
                assert!(dist as usize <= out.len(), "invalid back-reference");
                for _ in 0..len {
                    let b = out[out.len() - dist as usize];
                    out.push(b);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// fixed-Huffman block (BTYPE = 1)
// ---------------------------------------------------------------------------

fn fixed_lit(bw: &mut BitWriter, sym: u32) {
    if sym <= 143 {
        bw.code(0x30 + sym, 8);
    } else if sym <= 255 {
        bw.code(0x190 + (sym - 144), 9);
    } else if sym <= 279 {
        bw.code(sym - 256, 7);
    } else {
        bw.code(0xC0 + (sym - 280), 8);
    }
}

pub fn write_fixed_block(bw: &mut BitWriter, toks: &[Tok], bfinal: bool) {
    bw.bits(bfinal as u32, 1);
    bw.bits(1, 2); // BTYPE = 01 (fixed)
    for t in toks {
        match *t {
            Tok::Lit(b) => fixed_lit(bw, b as u32),
            Tok::Match { len, dist } => {
                let (ls, lx, lnb) = len_sym(len);
                fixed_lit(bw, ls);
                bw.bits(lx, lnb);
                let (ds, dx, dnb) = dist_sym(dist);
                bw.code(ds, 5);
                bw.bits(dx, dnb);
            }
        }
    }
    fixed_lit(bw, 256); // end of block
}

// ---------------------------------------------------------------------------
// stored block (BTYPE = 0)
// ---------------------------------------------------------------------------

pub fn write_stored_block(bw: &mut BitWriter, data: &[u8], bfinal: bool, nlen_override: Option<u16>) {
    assert!(data.len() <= 0xFFFF);
    bw.bits(bfinal as u32, 1);
    bw.bits(0, 2); // BTYPE = 00 (stored)
    bw.align();
    let len = data.len() as u16;
    let nlen = nlen_override.unwrap_or(!len);
    bw.raw(&len.to_le_bytes());
    bw.raw(&nlen.to_le_bytes());
    bw.raw(data);
}

// ---------------------------------------------------------------------------
// dynamic-Huffman block (BTYPE = 2)
// ---------------------------------------------------------------------------

/// Canonical Huffman codes from code lengths, exactly like RFC1951 / `cp_build`.
pub fn canonical_codes(lens: &[u8]) -> Vec<u32> {
    let mut counts = [0u32; 16];
    for &l in lens {
        if l > 0 {
            counts[l as usize] += 1;
        }
    }
    let mut next = [0u32; 16];
    let mut code = 0u32;
    for b in 1..16 {
        code = (code + counts[b - 1]) << 1;
        next[b] = code;
    }
    lens.iter()
        .map(|&l| {
            if l == 0 {
                0
            } else {
                let c = next[l as usize];
                next[l as usize] += 1;
                c
            }
        })
        .collect()
}

/// Assign a *complete* set of canonical code lengths to `used` symbols
/// (Huffman code for equal frequencies: lengths floor/ceil(log2 m)).
pub fn balanced_lens(used: &[usize], total: usize) -> Vec<u8> {
    let mut lens = vec![0u8; total];
    let m = used.len();
    if m == 0 {
        return lens;
    }
    if m == 1 {
        lens[used[0]] = 1;
        return lens;
    }
    let k = (usize::BITS - 1 - m.leading_zeros()) as usize; // floor(log2 m)
    let r = m - (1usize << k);
    for (i, &s) in used.iter().enumerate() {
        lens[s] = if i < 2 * r { (k + 1) as u8 } else { k as u8 };
    }
    assert!(lens.iter().all(|&l| l <= 15));
    lens
}

/// RLE-encode a code-length sequence into `(symbol, extra, extra_bits)`.
pub fn rle_code_lengths(seq: &[u8], use_rle: bool) -> Vec<(usize, u32, u32)> {
    let mut out: Vec<(usize, u32, u32)> = Vec::new();
    if !use_rle {
        return seq.iter().map(|&v| (v as usize, 0, 0)).collect();
    }
    let mut i = 0;
    while i < seq.len() {
        let v = seq[i];
        let mut run = 1;
        while i + run < seq.len() && seq[i + run] == v {
            run += 1;
        }
        if v == 0 {
            let mut left = run;
            while left >= 11 {
                let n = left.min(138);
                out.push((18, (n - 11) as u32, 7));
                left -= n;
            }
            while left >= 3 {
                let n = left.min(10);
                out.push((17, (n - 3) as u32, 3));
                left -= n;
            }
            for _ in 0..left {
                out.push((0, 0, 0));
            }
        } else {
            out.push((v as usize, 0, 0));
            let mut left = run - 1;
            while left >= 3 {
                let n = left.min(6);
                out.push((16, (n - 3) as u32, 2));
                left -= n;
            }
            for _ in 0..left {
                out.push((v as usize, 0, 0));
            }
        }
        i += run;
    }
    out
}

pub fn write_dynamic_block(bw: &mut BitWriter, toks: &[Tok], bfinal: bool, use_rle: bool) {
    // 1. which litlen / dist symbols are used?
    let mut lit_used = vec![false; 288];
    let mut dst_used = vec![false; 32];
    lit_used[256] = true; // end of block
    for t in toks {
        match *t {
            Tok::Lit(b) => lit_used[b as usize] = true,
            Tok::Match { len, dist } => {
                lit_used[len_sym(len).0 as usize] = true;
                dst_used[dist_sym(dist).0 as usize] = true;
            }
        }
    }
    if lit_used.iter().filter(|&&u| u).count() == 1 {
        // need at least two symbols for a complete code
        lit_used[0] = true;
    }
    let ndst_used = dst_used.iter().filter(|&&u| u).count();
    if ndst_used == 0 {
        dst_used[0] = true;
        dst_used[1] = true;
    } else if ndst_used == 1 {
        // add a second (unused) distance symbol so the code stays complete
        let first = dst_used.iter().position(|&u| u).unwrap();
        let other = if first == 0 { 1 } else { 0 };
        dst_used[other] = true;
    }

    let nlit = 257.max(lit_used.iter().rposition(|&u| u).unwrap() + 1);
    let ndst = 1.max(dst_used.iter().rposition(|&u| u).unwrap() + 1);
    assert!(nlit <= 288 && ndst <= 32);

    let lit_syms: Vec<usize> = (0..nlit).filter(|&i| lit_used[i]).collect();
    let dst_syms: Vec<usize> = (0..ndst).filter(|&i| dst_used[i]).collect();
    let lit_lens = balanced_lens(&lit_syms, nlit);
    let dst_lens = balanced_lens(&dst_syms, ndst);
    let lit_codes = canonical_codes(&lit_lens);
    let dst_codes = canonical_codes(&dst_lens);

    // 2. code-length alphabet
    let mut seq: Vec<u8> = Vec::new();
    seq.extend_from_slice(&lit_lens);
    seq.extend_from_slice(&dst_lens);
    let items = rle_code_lengths(&seq, use_rle);
    let mut cl_used = vec![false; 19];
    for &(s, _, _) in &items {
        cl_used[s] = true;
    }
    if cl_used.iter().filter(|&&u| u).count() == 1 {
        let first = cl_used.iter().position(|&u| u).unwrap();
        cl_used[if first == 0 { 1 } else { 0 }] = true;
    }
    let cl_syms: Vec<usize> = (0..19).filter(|&i| cl_used[i]).collect();
    let cl_lens = balanced_lens(&cl_syms, 19);
    let cl_codes = canonical_codes(&cl_lens);
    // HCLEN: how many entries of the permutation order must be transmitted
    let mut nlen = 4usize;
    for i in 0..19 {
        if cl_lens[PERM[i]] != 0 {
            nlen = nlen.max(i + 1);
        }
    }

    // 3. header
    bw.bits(bfinal as u32, 1);
    bw.bits(2, 2); // BTYPE = 10 (dynamic)
    bw.bits((nlit - 257) as u32, 5);
    bw.bits((ndst - 1) as u32, 5);
    bw.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        bw.bits(cl_lens[PERM[i]] as u32, 3);
    }
    // 4. the code-length sequence
    for &(s, extra, nb) in &items {
        bw.code(cl_codes[s], cl_lens[s] as u32);
        bw.bits(extra, nb);
    }
    // 5. the payload
    for t in toks {
        match *t {
            Tok::Lit(b) => {
                let s = b as usize;
                bw.code(lit_codes[s], lit_lens[s] as u32);
            }
            Tok::Match { len, dist } => {
                let (ls, lx, lnb) = len_sym(len);
                bw.code(lit_codes[ls as usize], lit_lens[ls as usize] as u32);
                bw.bits(lx, lnb);
                let (ds, dx, dnb) = dist_sym(dist);
                bw.code(dst_codes[ds as usize], dst_lens[ds as usize] as u32);
                bw.bits(dx, dnb);
            }
        }
    }
    bw.code(lit_codes[256], lit_lens[256] as u32);
}

// ---------------------------------------------------------------------------
// convenience wrappers producing complete raw DEFLATE streams
// ---------------------------------------------------------------------------

pub fn deflate_fixed(toks: &[Tok]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, toks, true);
    bw.finish()
}

pub fn deflate_dynamic(toks: &[Tok], use_rle: bool) -> Vec<u8> {
    let mut bw = BitWriter::new();
    write_dynamic_block(&mut bw, toks, true, use_rle);
    bw.finish()
}

pub fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    write_stored_block(&mut bw, data, true, None);
    bw.finish()
}

pub fn deflate_literals_fixed(data: &[u8]) -> Vec<u8> {
    let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
    deflate_fixed(&toks)
}

/// raw DEFLATE produced by flate2/miniz_oxide at the given level (0..=9)
pub fn deflate_flate2(data: &[u8], level: u32) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

// ---------------------------------------------------------------------------
// zlib / PNG containers
// ---------------------------------------------------------------------------

pub fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

pub fn zlib_wrap(deflate: &[u8], cmf: u8, flg: u8, adler: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(deflate.len() + 6);
    v.push(cmf);
    v.push(flg);
    v.extend_from_slice(deflate);
    v.extend_from_slice(&adler.to_be_bytes());
    v
}

pub fn zlib(deflate: &[u8], raw: &[u8]) -> Vec<u8> {
    zlib_wrap(deflate, 0x78, 0x9C, adler32(raw))
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

pub const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Clone)]
pub struct Chunk {
    pub name: [u8; 4],
    pub data: Vec<u8>,
    /// overrides the length field written to the file (defaults to `data.len()`)
    pub len_override: Option<u32>,
    /// overrides the CRC (never checked by the C code)
    pub crc_override: Option<u32>,
}

impl Chunk {
    pub fn new(name: &[u8; 4], data: Vec<u8>) -> Chunk {
        Chunk {
            name: *name,
            data,
            len_override: None,
            crc_override: None,
        }
    }
    pub fn encode(&self, out: &mut Vec<u8>) {
        let len = self.len_override.unwrap_or(self.data.len() as u32);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&self.name);
        out.extend_from_slice(&self.data);
        let mut crc_in = self.name.to_vec();
        crc_in.extend_from_slice(&self.data);
        let crc = self.crc_override.unwrap_or_else(|| crc32(&crc_in));
        out.extend_from_slice(&crc.to_be_bytes());
    }
}

/// A 13-byte IHDR payload.
pub fn ihdr(w: u32, h: u32, bit_depth: u8, color_type: u8, comp: u8, filt: u8, inter: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(13);
    v.extend_from_slice(&w.to_be_bytes());
    v.extend_from_slice(&h.to_be_bytes());
    v.push(bit_depth);
    v.push(color_type);
    v.push(comp);
    v.push(filt);
    v.push(inter);
    v
}

pub fn build_png(sig: &[u8], chunks: &[Chunk]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(sig);
    for c in chunks {
        c.encode(&mut out);
    }
    out
}

pub fn bpp_of(color_type: u8) -> usize {
    match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => panic!("unsupported colour type {color_type}"),
    }
}

/// Build the *raw* (already filtered) scanline stream: `h` rows of
/// `1 + w*bpp` bytes, with `filters[y]` as the filter byte of row `y`.
pub fn scanlines(w: usize, h: usize, bpp: usize, filters: &[u8], payload: &[u8]) -> Vec<u8> {
    let stride = w * bpp;
    let mut out = Vec::with_capacity(h * (1 + stride));
    for y in 0..h {
        out.push(filters[y % filters.len()]);
        for x in 0..stride {
            out.push(payload[(y * stride + x) % payload.len()]);
        }
    }
    out
}

/// Split `data` into `n` contiguous IDAT chunks (roughly equal parts).
pub fn split_idat(data: &[u8], n: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    if n == 0 {
        return chunks;
    }
    let per = (data.len() + n - 1) / n.max(1);
    let mut off = 0;
    for _ in 0..n {
        let end = (off + per).min(data.len());
        chunks.push(Chunk::new(b"IDAT", data[off..end].to_vec()));
        off = end;
    }
    if off < data.len() {
        chunks.push(Chunk::new(b"IDAT", data[off..].to_vec()));
    }
    chunks
}

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct PngResult {
    pub w: c_int,
    pub h: c_int,
    pub ok: bool,
    pub pixels: Vec<u8>,
    pub err: Option<String>,
}

/// Call `load_png_mem` on one implementation. `png` is passed *by pointer*, so
/// both implementations observe the very same bytes (including any
/// out-of-bounds read the C code performs).
pub fn call_load_png(im: &Impl, png: &[u8], len: c_int) -> PngResult {
    im.clear_error();
    let img = unsafe { (im.load_png_mem)(png.as_ptr(), len) };
    let ok = !img.pix.is_null();
    let mut pixels = Vec::new();
    if ok {
        let n = (img.w as i64) * (img.h as i64) * 4;
        assert!(n >= 0, "{}: negative pixel count", im.name);
        pixels = unsafe { std::slice::from_raw_parts(img.pix, n as usize) }.to_vec();
        unsafe { libc::free(img.pix as *mut c_void) };
    }
    PngResult {
        w: img.w,
        h: img.h,
        ok,
        pixels,
        err: im.error(),
    }
}

/// The C code reads out of bounds in several places (`cp_make32` /
/// `memcmp` past `png.end`, `plte[c*3]` past a short PLTE, `trns[index]` past a
/// short tRNS).  Those reads must observe *identical* memory in both
/// implementations, so every input is copied into a generously padded buffer
/// with a fixed fill pattern before being handed to the two `.so`s.
pub const PNG_PAD: usize = 16384;
pub const PNG_PAD_BYTE: u8 = 0x5A;

pub fn padded(png: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(png.len() + PNG_PAD);
    v.extend_from_slice(png);
    v.resize(png.len() + PNG_PAD, PNG_PAD_BYTE);
    v
}

pub fn diff_png_len(png: &[u8], len: c_int, label: &str) -> PngResult {
    let p = pair();
    let _g = ERROR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let png = &padded(png)[..];
    let rc = call_load_png(&p.c, png, len);
    let rr = call_load_png(&p.rust, png, len);
    assert_eq!(rc.ok, rr.ok, "[{label}] pix!=NULL mismatch: c={rc:?} rust={rr:?}");
    assert_eq!(rc.w, rr.w, "[{label}] img.w mismatch: c={} rust={}", rc.w, rr.w);
    assert_eq!(rc.h, rr.h, "[{label}] img.h mismatch: c={} rust={}", rc.h, rr.h);
    assert_eq!(
        rc.err, rr.err,
        "[{label}] cp_error_reason mismatch: c={:?} rust={:?}",
        rc.err, rr.err
    );
    if rc.pixels != rr.pixels {
        let i = rc
            .pixels
            .iter()
            .zip(rr.pixels.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(rc.pixels.len().min(rr.pixels.len()));
        panic!(
            "[{label}] pixel mismatch at byte {i} (len c={} rust={}): c={:?} rust={:?}",
            rc.pixels.len(),
            rr.pixels.len(),
            &rc.pixels[i.saturating_sub(4)..(i + 8).min(rc.pixels.len())],
            &rr.pixels[i.saturating_sub(4)..(i + 8).min(rr.pixels.len())],
        );
    }
    rc
}

pub fn diff_png(png: &[u8], label: &str) -> PngResult {
    diff_png_len(png, png.len() as c_int, label)
}

#[derive(Debug, PartialEq, Eq)]
pub struct InflateResult {
    pub rc: c_int,
    pub out: Vec<u8>,
    pub err: Option<String>,
}

/// Build a buffer whose payload starts at `addr % 4 == align`.
pub fn aligned_input(data: &[u8], align: usize) -> (Vec<u8>, usize) {
    let mut buf = vec![0u8; data.len() + 8];
    let base = buf.as_ptr() as usize;
    let off = (align + 4 - (base % 4)) % 4;
    buf[off..off + data.len()].copy_from_slice(data);
    (buf, off)
}

pub fn call_inflate(
    im: &Impl,
    input: *mut c_void,
    in_bytes: c_int,
    out_bytes: c_int,
    out_alloc: usize,
) -> InflateResult {
    im.clear_error();
    let mut out = vec![0xCDu8; out_alloc];
    let rc = unsafe { (im.cp_inflate)(input, in_bytes, out.as_mut_ptr() as *mut c_void, out_bytes) };
    InflateResult {
        rc,
        out,
        err: im.error(),
    }
}

/// Differential `cp_inflate`: `align` selects the input-pointer alignment
/// (`first_bytes = (4 - align) % 4`), `slack` extra guard bytes are appended to
/// the output allocation and compared as well.
pub fn diff_inflate_full(
    deflate: &[u8],
    in_bytes: c_int,
    align: usize,
    out_bytes: c_int,
    slack: usize,
    label: &str,
) -> InflateResult {
    let p = pair();
    let _g = ERROR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (mut buf, off) = aligned_input(deflate, align);
    let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut c_void;
    assert_eq!(ptr as usize % 4, align % 4, "alignment setup failed");
    let alloc = (out_bytes.max(0) as usize) + slack;
    let rc = call_inflate(&p.c, ptr, in_bytes, out_bytes, alloc);
    let rr = call_inflate(&p.rust, ptr, in_bytes, out_bytes, alloc);
    assert_eq!(
        rc.rc, rr.rc,
        "[{label}] cp_inflate return mismatch: c={} rust={} (c_err={:?} rust_err={:?})",
        rc.rc, rr.rc, rc.err, rr.err
    );
    assert_eq!(
        rc.err, rr.err,
        "[{label}] cp_error_reason mismatch: c={:?} rust={:?}",
        rc.err, rr.err
    );
    if rc.out != rr.out {
        let i = rc
            .out
            .iter()
            .zip(rr.out.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "[{label}] output mismatch at byte {i}: c={:?} rust={:?}",
            &rc.out[i..(i + 16).min(rc.out.len())],
            &rr.out[i..(i + 16).min(rr.out.len())],
        );
    }
    rc
}

pub fn diff_inflate(deflate: &[u8], align: usize, out_bytes: c_int, label: &str) -> InflateResult {
    diff_inflate_full(deflate, deflate.len() as c_int, align, out_bytes, 64, label)
}

// ---------------------------------------------------------------------------
// fork-based differential driver (needed because the C .so is built *without*
// NDEBUG: a failing assert() terminates the process with SIGABRT)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32, Vec<u8>),
    Signaled(i32),
}

pub fn run_forked<F: FnOnce() -> Vec<u8>>(f: F) -> Outcome {
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---- child ----
        unsafe {
            libc::close(fds[0]);
            // no core dumps, no assert chatter on stderr
            let rl = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 2);
            }
            libc::alarm(5);
        }
        let out = f();
        let mut off = 0usize;
        while off < out.len() {
            let n = unsafe {
                libc::write(
                    fds[1],
                    out[off..].as_ptr() as *const c_void,
                    out.len() - off,
                )
            };
            if n <= 0 {
                break;
            }
            off += n as usize;
        }
        unsafe {
            libc::close(fds[1]);
            libc::_exit(0)
        };
    }
    // ---- parent ----
    unsafe { libc::close(fds[1]) };
    let mut data = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = unsafe { libc::read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        data.extend_from_slice(&buf[..n as usize]);
    }
    unsafe { libc::close(fds[0]) };
    let mut status = 0i32;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    if libc::WIFSIGNALED(status) {
        Outcome::Signaled(libc::WTERMSIG(status))
    } else {
        Outcome::Exited(libc::WEXITSTATUS(status), data)
    }
}

fn encode_png_result(r: &PngResult) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&r.w.to_le_bytes());
    v.extend_from_slice(&r.h.to_le_bytes());
    v.push(r.ok as u8);
    let e = r.err.clone().unwrap_or_default();
    v.extend_from_slice(&(e.len() as u32).to_le_bytes());
    v.extend_from_slice(e.as_bytes());
    v.extend_from_slice(&r.pixels);
    v
}

fn encode_inflate_result(r: &InflateResult) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&r.rc.to_le_bytes());
    let e = r.err.clone().unwrap_or_default();
    v.extend_from_slice(&(e.len() as u32).to_le_bytes());
    v.extend_from_slice(e.as_bytes());
    v.extend_from_slice(&r.out);
    v
}

/// Differential `load_png_mem` that also compares *termination status*, so
/// inputs which make the C library `abort()` (live `assert()`) are testable.
pub fn diff_png_forked(png: &[u8], len: c_int, label: &str) -> Outcome {
    let p = pair();
    let _g = ERROR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let buf = padded(png);
    let png = &buf[..];
    let (a, b) = run_forked_pair(
        || encode_png_result(&call_load_png(&p.c, png, len)),
        || encode_png_result(&call_load_png(&p.rust, png, len)),
    );
    assert_eq!(
        a.outcome, b.outcome,
        "[{label}] forked load_png_mem outcome mismatch (c stderr {:?}, rust stderr {:?})",
        a.stderr, b.stderr
    );
    a.outcome
}

/// Differential `cp_inflate` that also compares *termination status*.
pub fn diff_inflate_forked(
    deflate: &[u8],
    in_bytes: c_int,
    align: usize,
    out_bytes: c_int,
    slack: usize,
    label: &str,
) -> Outcome {
    let p = pair();
    let _g = ERROR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (mut buf, off) = aligned_input(deflate, align);
    let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut c_void;
    let alloc = (out_bytes.max(0) as usize) + slack;
    let (a, b) = run_forked_pair(
        || encode_inflate_result(&call_inflate(&p.c, ptr, in_bytes, out_bytes, alloc)),
        || encode_inflate_result(&call_inflate(&p.rust, ptr, in_bytes, out_bytes, alloc)),
    );
    assert_eq!(
        a.outcome, b.outcome,
        "[{label}] forked cp_inflate outcome mismatch (c stderr {:?}, rust stderr {:?})",
        a.stderr, b.stderr
    );
    a.outcome
}

// ---------------------------------------------------------------------------
// declarative PNG builder
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Deflate {
    Stored,
    Fixed,
    Dynamic { rle: bool },
    Flate2(u32),
}

impl Deflate {
    pub fn run(&self, raw: &[u8]) -> Vec<u8> {
        match *self {
            Deflate::Stored => deflate_stored(raw),
            Deflate::Fixed => deflate_literals_fixed(raw),
            Deflate::Dynamic { rle } => {
                let toks: Vec<Tok> = raw.iter().map(|&b| Tok::Lit(b)).collect();
                deflate_dynamic(&toks, rle)
            }
            Deflate::Flate2(l) => deflate_flate2(raw, l),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Order {
    /// IHDR, [pre], PLTE, tRNS, [mid], IDAT..., IEND
    Normal,
    /// IHDR, [pre], tRNS, PLTE, [mid], IDAT..., IEND  (`cp_find` then misses tRNS)
    TrnsBeforePlte,
    /// IHDR, [pre], PLTE, [mid], IDAT..., tRNS, IEND  (`first` jumps past the IDATs)
    TrnsAfterIdat,
    /// IHDR, [pre], [mid], IDAT..., PLTE, IEND
    PlteAfterIdat,
}

#[derive(Clone)]
pub struct Spec {
    pub w: u32,
    pub h: u32,
    pub ct: u8,
    pub bit_depth: u8,
    pub comp: u8,
    pub filt: u8,
    pub inter: u8,
    /// overrides for the width/height/colour type written into IHDR
    pub ihdr_w: Option<u32>,
    pub ihdr_h: Option<u32>,
    pub ihdr_ct: Option<u8>,
    /// extra padding bytes appended to the IHDR payload (length stays >= 13)
    pub ihdr_extra: usize,
    pub filters: Vec<u8>,
    pub payload: Vec<u8>,
    pub plte: Option<Vec<u8>>,
    pub trns: Option<Vec<u8>>,
    pub order: Order,
    pub pre_chunks: Vec<Chunk>,
    pub mid_chunks: Vec<Chunk>,
    pub n_idat: usize,
    /// insert a zero-length IDAT before every real one
    pub empty_idats: bool,
    /// put a non-IDAT chunk between the first and the second IDAT
    pub idat_gap: bool,
    pub deflate: Deflate,
    pub cmf: u8,
    pub flg: u8,
    pub adler_override: Option<u32>,
    /// scramble every chunk CRC (the C code never checks them)
    pub bad_crc: bool,
    pub iend: bool,
    pub trailing: Vec<u8>,
    pub sig: Vec<u8>,
    /// replaces the whole zlib stream if set
    pub raw_zlib: Option<Vec<u8>>,
}

impl Spec {
    pub fn new(w: u32, h: u32, ct: u8) -> Spec {
        let bpp = bpp_of(ct);
        let n = (w as usize) * (h as usize) * bpp;
        let payload: Vec<u8> = (0..n.max(1)).map(|i| (i * 37 + 11) as u8).collect();
        Spec {
            w,
            h,
            ct,
            bit_depth: 8,
            comp: 0,
            filt: 0,
            inter: 0,
            ihdr_w: None,
            ihdr_h: None,
            ihdr_ct: None,
            ihdr_extra: 0,
            filters: vec![0],
            payload,
            plte: if ct == 3 {
                Some((0..256 * 3).map(|i| (i * 5 + 1) as u8).collect())
            } else {
                None
            },
            trns: None,
            order: Order::Normal,
            pre_chunks: Vec::new(),
            mid_chunks: Vec::new(),
            n_idat: 1,
            empty_idats: false,
            idat_gap: false,
            deflate: Deflate::Fixed,
            cmf: 0x78,
            flg: 0x9C,
            adler_override: None,
            bad_crc: false,
            iend: true,
            trailing: Vec::new(),
            sig: PNG_SIG.to_vec(),
            raw_zlib: None,
        }
    }

    pub fn bpp(&self) -> usize {
        bpp_of(self.ct)
    }

    /// the raw (filtered) scanline stream that IDAT must carry
    pub fn raw(&self) -> Vec<u8> {
        scanlines(
            self.w as usize,
            self.h as usize,
            self.bpp(),
            &self.filters,
            &self.payload,
        )
    }

    pub fn zlib_stream(&self) -> Vec<u8> {
        if let Some(z) = &self.raw_zlib {
            return z.clone();
        }
        let raw = self.raw();
        let d = self.deflate.run(&raw);
        let adler = self.adler_override.unwrap_or_else(|| adler32(&raw));
        zlib_wrap(&d, self.cmf, self.flg, adler)
    }

    pub fn build(&self) -> Vec<u8> {
        let mut ihdr_data = ihdr(
            self.ihdr_w.unwrap_or(self.w),
            self.ihdr_h.unwrap_or(self.h),
            self.bit_depth,
            self.ihdr_ct.unwrap_or(self.ct),
            self.comp,
            self.filt,
            self.inter,
        );
        for i in 0..self.ihdr_extra {
            ihdr_data.push(0xA5u8.wrapping_add(i as u8));
        }

        let mut chunks: Vec<Chunk> = vec![Chunk::new(b"IHDR", ihdr_data)];
        chunks.extend(self.pre_chunks.iter().cloned());

        let plte = self.plte.as_ref().map(|p| Chunk::new(b"PLTE", p.clone()));
        let trns = self.trns.as_ref().map(|t| Chunk::new(b"tRNS", t.clone()));

        let mut idats: Vec<Chunk> = Vec::new();
        let z = self.zlib_stream();
        for (i, c) in split_idat(&z, self.n_idat).into_iter().enumerate() {
            if self.empty_idats {
                idats.push(Chunk::new(b"IDAT", vec![]));
            }
            if self.idat_gap && i == 1 {
                idats.push(Chunk::new(b"gAMA", vec![0, 1, 2, 3]));
            }
            idats.push(c);
        }

        match self.order {
            Order::Normal => {
                chunks.extend(plte.clone());
                chunks.extend(trns.clone());
                chunks.extend(self.mid_chunks.iter().cloned());
                chunks.extend(idats);
            }
            Order::TrnsBeforePlte => {
                chunks.extend(trns.clone());
                chunks.extend(plte.clone());
                chunks.extend(self.mid_chunks.iter().cloned());
                chunks.extend(idats);
            }
            Order::TrnsAfterIdat => {
                chunks.extend(plte.clone());
                chunks.extend(self.mid_chunks.iter().cloned());
                chunks.extend(idats);
                chunks.extend(trns.clone());
            }
            Order::PlteAfterIdat => {
                chunks.extend(trns.clone());
                chunks.extend(self.mid_chunks.iter().cloned());
                chunks.extend(idats);
                chunks.extend(plte.clone());
            }
        }
        if self.iend {
            chunks.push(Chunk::new(b"IEND", vec![]));
        }
        if self.bad_crc {
            for (i, c) in chunks.iter_mut().enumerate() {
                c.crc_override = Some(0xDEAD_0000u32 ^ i as u32);
            }
        }
        let mut out = build_png(&self.sig, &chunks);
        out.extend_from_slice(&self.trailing);
        out
    }
}

// ---------------------------------------------------------------------------
// independent reference decoder (PNG spec) — used to prove that the
// differential tests really exercise the happy path and not two identical bugs
// ---------------------------------------------------------------------------

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let (pa, pb, pc) = ((p - a as i32).abs(), (p - b as i32).abs(), (p - c as i32).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Unfilter `raw` (`h` rows of `1 + w*bpp` bytes) in place, per RFC 2083.
pub fn reference_unfilter(w: usize, h: usize, bpp: usize, raw: &[u8]) -> Option<Vec<u8>> {
    let stride = w * bpp;
    let mut rows: Vec<Vec<u8>> = Vec::with_capacity(h);
    for y in 0..h {
        let base = y * (1 + stride);
        let f = raw[base];
        let mut cur = raw[base + 1..base + 1 + stride].to_vec();
        let zero = vec![0u8; stride];
        let prev: &[u8] = if y == 0 { &zero } else { &rows[y - 1] };
        match f {
            0 => {}
            1 => {
                for x in bpp..stride {
                    cur[x] = cur[x].wrapping_add(cur[x - bpp]);
                }
            }
            2 => {
                for x in 0..stride {
                    cur[x] = cur[x].wrapping_add(prev[x]);
                }
            }
            3 => {
                for x in 0..stride {
                    let a = if x >= bpp { cur[x - bpp] as u32 } else { 0 };
                    cur[x] = cur[x].wrapping_add(((a + prev[x] as u32) / 2) as u8);
                }
            }
            4 => {
                for x in 0..stride {
                    let a = if x >= bpp { cur[x - bpp] } else { 0 };
                    let c = if x >= bpp { prev[x - bpp] } else { 0 };
                    cur[x] = cur[x].wrapping_add(paeth(a, prev[x], c));
                }
            }
            _ => return None,
        }
        rows.push(cur);
    }
    Some(rows.concat())
}

/// Reference RGBA output for a `Spec` (only for colour types the C code
/// supports and bit depth 8).
pub fn reference_rgba(spec: &Spec) -> Option<Vec<u8>> {
    let (w, h, bpp) = (spec.w as usize, spec.h as usize, spec.bpp());
    let raw = spec.raw();
    let un = reference_unfilter(w, h, bpp, &raw)?;
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let s = &un[y * w * bpp + x * bpp..];
            let px = match spec.ct {
                0 => [s[0], s[0], s[0], 0xFF],
                2 => [s[0], s[1], s[2], 0xFF],
                4 => [s[0], s[0], s[0], s[1]],
                6 => [s[0], s[1], s[2], s[3]],
                3 => {
                    let c = s[0] as usize;
                    let plte = spec.plte.as_ref()?;
                    let a = match &spec.trns {
                        Some(t) if c < t.len() => t[c],
                        _ => 255,
                    };
                    [*plte.get(c * 3)?, *plte.get(c * 3 + 1)?, *plte.get(c * 3 + 2)?, a]
                }
                _ => return None,
            };
            out.extend_from_slice(&px);
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// forked driver that also captures the child's stderr, so glibc's
// `__assert_fail` message identifies *which* assert() of lib.c fired
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ForkResult {
    pub outcome: Outcome,
    pub stderr: String,
}

impl ForkResult {
    pub fn aborted(&self) -> bool {
        matches!(self.outcome, Outcome::Signaled(s) if s == libc::SIGABRT)
    }
    /// the `Assertion \`...' failed` expression, if any
    pub fn assertion(&self) -> Option<String> {
        let s = &self.stderr;
        let i = s.find("Assertion `")? + "Assertion `".len();
        let j = s[i..].find("' failed")?;
        Some(s[i..i + j].to_string())
    }
}

fn read_all(fd: i32) -> Vec<u8> {
    let mut v = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        v.extend_from_slice(&buf[..n as usize]);
    }
    v
}

pub fn run_forked_capture<F: FnOnce() -> Vec<u8>>(f: F) -> ForkResult {
    let mut data = [0i32; 2];
    let mut errp = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(data.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { libc::pipe(errp.as_mut_ptr()) }, 0);
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            libc::close(data[0]);
            libc::close(errp[0]);
            let rl = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            libc::dup2(errp[1], 2);
            libc::close(errp[1]);
            // watchdog: a wedged child must not wedge the test suite
            libc::alarm(5);
        }
        let out = f();
        let mut off = 0usize;
        while off < out.len() {
            let n = unsafe {
                libc::write(data[1], out[off..].as_ptr() as *const c_void, out.len() - off)
            };
            if n <= 0 {
                break;
            }
            off += n as usize;
        }
        unsafe {
            libc::close(data[1]);
            libc::_exit(0)
        };
    }
    unsafe {
        libc::close(data[1]);
        libc::close(errp[1]);
    }
    let out = read_all(data[0]);
    let err = read_all(errp[0]);
    unsafe {
        libc::close(data[0]);
        libc::close(errp[0]);
    }
    let mut status = 0i32;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    let outcome = if libc::WIFSIGNALED(status) {
        Outcome::Signaled(libc::WTERMSIG(status))
    } else {
        Outcome::Exited(libc::WEXITSTATUS(status), out)
    };
    ForkResult {
        outcome,
        stderr: String::from_utf8_lossy(&err).into_owned(),
    }
}

/// Differential `cp_inflate` with termination-status *and* assertion-message
/// reporting.  Returns the C side's `ForkResult` after asserting that the Rust
/// side terminated identically.
pub fn diff_inflate_abort(
    deflate: &[u8],
    in_bytes: c_int,
    align: usize,
    out_bytes: c_int,
    label: &str,
) -> ForkResult {
    let p = pair();
    let _g = ERROR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (mut buf, off) = aligned_input(deflate, align);
    let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut c_void;
    let alloc = (out_bytes.max(0) as usize) + 64;
    let (a, b) = run_forked_pair(
        || encode_inflate_result(&call_inflate(&p.c, ptr, in_bytes, out_bytes, alloc)),
        || encode_inflate_result(&call_inflate(&p.rust, ptr, in_bytes, out_bytes, alloc)),
    );
    assert_eq!(
        a.outcome, b.outcome,
        "[{label}] cp_inflate termination mismatch (c stderr: {:?})",
        a.stderr
    );
    a
}

/// Differential `load_png_mem` with termination-status *and* assertion-message
/// reporting.
pub fn diff_png_abort(png: &[u8], len: c_int, label: &str) -> ForkResult {
    let p = pair();
    let _g = ERROR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let buf = padded(png);
    let png = &buf[..];
    let (a, b) = run_forked_pair(
        || encode_png_result(&call_load_png(&p.c, png, len)),
        || encode_png_result(&call_load_png(&p.rust, png, len)),
    );
    assert_eq!(
        a.outcome, b.outcome,
        "[{label}] load_png_mem termination mismatch (c stderr: {:?})",
        a.stderr
    );
    a
}

// ---------------------------------------------------------------------------
// paired fork driver
//
// `load_png_mem` lets `cp_inflate` write up to `pix_bytes - out_size(bpp)`
// bytes past the end of `img.pix` (because `out + out_bytes` exceeds
// `img.pix + pix_bytes` whenever `bpp < 4`).  For malformed streams that
// clobbers glibc heap metadata, and *whether glibc notices* depends on the
// surrounding heap.  So the two children must be forked back to back, from
// byte-identical parent state, before either one's output is read -- otherwise
// the comparison measures the parent's allocator history instead of the
// libraries.
// ---------------------------------------------------------------------------

struct ChildPipes {
    data: [i32; 2],
    err: [i32; 2],
}

fn new_pipes() -> ChildPipes {
    let mut data = [0i32; 2];
    let mut err = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(data.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { libc::pipe(err.as_mut_ptr()) }, 0);
    ChildPipes { data, err }
}

unsafe fn child_setup(mine: &ChildPipes, other: &ChildPipes) {
    libc::close(other.data[0]);
    libc::close(other.data[1]);
    libc::close(other.err[0]);
    libc::close(other.err[1]);
    libc::close(mine.data[0]);
    libc::close(mine.err[0]);
    let rl = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    libc::setrlimit(libc::RLIMIT_CORE, &rl);
    libc::dup2(mine.err[1], 2);
    libc::close(mine.err[1]);
    libc::alarm(5);
}

unsafe fn child_finish(mine: &ChildPipes, out: &[u8]) -> ! {
    let mut off = 0usize;
    while off < out.len() {
        let n = libc::write(
            mine.data[1],
            out[off..].as_ptr() as *const c_void,
            out.len() - off,
        );
        if n <= 0 {
            break;
        }
        off += n as usize;
    }
    libc::close(mine.data[1]);
    libc::_exit(0)
}

fn collect(pipes: &ChildPipes, pid: i32) -> ForkResult {
    let out = read_all(pipes.data[0]);
    let err = read_all(pipes.err[0]);
    unsafe {
        libc::close(pipes.data[0]);
        libc::close(pipes.err[0]);
    }
    let mut status = 0i32;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    let outcome = if libc::WIFSIGNALED(status) {
        Outcome::Signaled(libc::WTERMSIG(status))
    } else {
        Outcome::Exited(libc::WEXITSTATUS(status), out)
    };
    ForkResult {
        outcome,
        stderr: String::from_utf8_lossy(&err).into_owned(),
    }
}

pub fn run_forked_pair<FA, FB>(fa: FA, fb: FB) -> (ForkResult, ForkResult)
where
    FA: FnOnce() -> Vec<u8>,
    FB: FnOnce() -> Vec<u8>,
{
    let pa = new_pipes();
    let pb = new_pipes();
    let pid_a = unsafe { libc::fork() };
    assert!(pid_a >= 0, "fork failed");
    if pid_a == 0 {
        unsafe { child_setup(&pa, &pb) };
        let out = fa();
        unsafe { child_finish(&pa, &out) };
    }
    let pid_b = unsafe { libc::fork() };
    assert!(pid_b >= 0, "fork failed");
    if pid_b == 0 {
        unsafe { child_setup(&pb, &pa) };
        let out = fb();
        unsafe { child_finish(&pb, &out) };
    }
    unsafe {
        libc::close(pa.data[1]);
        libc::close(pa.err[1]);
        libc::close(pb.data[1]);
        libc::close(pb.err[1]);
    }
    let a = collect(&pa, pid_a);
    let b = collect(&pb, pid_b);
    (a, b)
}
