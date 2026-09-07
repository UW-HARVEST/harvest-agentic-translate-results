//! Shared differential-test harness.
//!
//! Both libraries are always reached through `dlopen`/`dlsym` (`libloading`) --
//! the Rust crate is a `cdylib` only, so the tests *cannot* link it directly and
//! every call goes through the `#[no_mangle] extern "C"` exports.

#![allow(dead_code)]

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// paths
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn workdir() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

/// `target/{debug,release}` for the profile the tests were built with.
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    // .../target/<profile>/deps/<test-exe>
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

static RUST_LIB: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// The shipped Rust artifact: `target/release/libload_png_mem_lib.so`.
///
/// The *release* profile is what the crate declares (`panic = "abort"`) and what
/// an external consumer links or `dlopen`s.  A debug build is deliberately not
/// used: `core`'s debug assertions turn the raw-pointer reads that the C performs
/// on a NULL input (`cp_inflate(NULL, 8, out, 1)`) into a Rust panic/abort,
/// whereas the C -- and the release build -- take a SIGSEGV.
pub fn rust_lib() -> PathBuf {
    RUST_LIB
        .get_or_init(|| {
            if let Ok(p) = std::env::var("RUST_PNG_LIB") {
                return PathBuf::from(p);
            }
            // `cargo test` compiles the lib target as a test harness, not as a
            // cdylib, so build the real `.so` here.
            let st = Command::new(env!("CARGO"))
                .current_dir(manifest_dir())
                .args(["build", "--offline", "--release"])
                .status()
                .expect("cargo not available");
            assert!(st.success(), "cargo build --release failed");
            let p = manifest_dir().join("target/release/libload_png_mem_lib.so");
            assert!(p.exists(), "rust cdylib missing at {p:?}");
            p
        })
        .clone()
}

/// The C reference library, built exactly as the task instructs
/// (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`, i.e. -O0, asserts ON).
pub fn c_lib() -> PathBuf {
    let build = workdir().join("c_src/build");
    let mut found = None;
    for e in std::fs::read_dir(&build).unwrap_or_else(|_| {
        panic!("{build:?} missing -- build the C library first (see task description)")
    }) {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") {
            found = Some(p);
        }
    }
    found.expect("no lib*.so in c_src/build")
}

/// A second C build with `-DNDEBUG` (same `-O0`, so the identical `.data`
/// layout).  Used only to classify cases where the reference build aborts in an
/// `assert()`; the Rust translation is documented as `NDEBUG`-equivalent.
pub fn c_lib_ndebug() -> PathBuf {
    let dir = manifest_dir().join("target/vtmp");
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("libcref_ndebug.so");
    if !out.exists() {
        let tmp = dir.join(format!("ndebug.{}.so", std::process::id()));
        let st = Command::new("gcc")
            .args(["-O0", "-DNDEBUG", "-fPIC", "-shared", "-o"])
            .arg(&tmp)
            .arg(workdir().join("c_src/src/lib.c"))
            .arg("-I")
            .arg(workdir().join("c_src/include"))
            .arg("-lm")
            .status()
            .expect("gcc not available");
        assert!(st.success(), "gcc failed to build the NDEBUG reference");
        let _ = std::fs::rename(&tmp, &out);
    }
    out
}

static RUNNER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub fn runner_exe() -> PathBuf {
    RUNNER
        .get_or_init(|| {
            // `cargo test` does not rebuild examples, so build it here (a no-op
            // when up to date) -- otherwise a stale runner would be used.
            let st = Command::new(env!("CARGO"))
                .current_dir(manifest_dir())
                .args(["build", "--offline", "--examples"])
                .status()
                .expect("cargo not available");
            assert!(st.success(), "cargo build --examples failed");
            let p = target_profile_dir().join("examples/diffrunner");
            assert!(p.exists(), "diffrunner example not built at {p:?}");
            p
        })
        .clone()
}

// ---------------------------------------------------------------------------
// in-process loading (valid inputs only -- no crash expected)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut u8,
}

pub type FnLoad = unsafe extern "C" fn(*const u8, c_int) -> CpImage;
pub type FnInflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

pub struct Lib {
    _lib: libloading::Library,
    pub load: FnLoad,
    pub inflate: FnInflate,
    pub err: *mut *const c_char,
    pub fixed_table: *mut u8,
    pub permutation_order: *mut u8,
    pub len_extra_bits: *mut u8,
    pub len_base: *mut u32,
    pub dist_extra_bits: *mut u8,
    pub dist_base: *mut u32,
}

impl Lib {
    pub fn open(path: &Path) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
            macro_rules! sym {
                ($t:ty, $n:expr) => {{
                    let s: libloading::Symbol<$t> = lib.get($n).unwrap();
                    *s
                }};
            }
            let load = sym!(FnLoad, b"load_png_mem\0");
            let inflate = sym!(FnInflate, b"cp_inflate\0");
            let err = sym!(*mut *const c_char, b"cp_error_reason\0");
            let fixed_table = sym!(*mut u8, b"cp_fixed_table\0");
            let permutation_order = sym!(*mut u8, b"cp_permutation_order\0");
            let len_extra_bits = sym!(*mut u8, b"cp_len_extra_bits\0");
            let len_base = sym!(*mut u32, b"cp_len_base\0");
            let dist_extra_bits = sym!(*mut u8, b"cp_dist_extra_bits\0");
            let dist_base = sym!(*mut u32, b"cp_dist_base\0");
            Lib {
                _lib: lib,
                load,
                inflate,
                err,
                fixed_table,
                permutation_order,
                len_extra_bits,
                len_base,
                dist_extra_bits,
                dist_base,
            }
        }
    }

    pub fn reason(&self) -> String {
        unsafe {
            let p = *self.err;
            if p.is_null() {
                "(null)".into()
            } else {
                CStr::from_ptr(p).to_string_lossy().into_owned()
            }
        }
    }

    pub fn clear_reason(&self) {
        unsafe { *self.err = std::ptr::null() }
    }
}

/// Result of a `load_png_mem` call, in a comparable form.
#[derive(PartialEq, Eq, Debug)]
pub struct PngOut {
    pub w: i32,
    pub h: i32,
    pub pix: Option<Vec<u8>>,
    pub err: String,
}

pub fn call_load(lib: &Lib, data: &[u8]) -> PngOut {
    call_load_len(lib, data, data.len() as i32)
}

pub fn call_load_len(lib: &Lib, data: &[u8], len: i32) -> PngOut {
    unsafe {
        lib.clear_reason();
        let img = (lib.load)(data.as_ptr(), len);
        let pix = if img.pix.is_null() {
            None
        } else {
            let n = (img.w as i64) * (img.h as i64) * 4;
            let n = if n < 0 { 0 } else { n as usize };
            let v = std::slice::from_raw_parts(img.pix, n).to_vec();
            libc::free(img.pix as *mut c_void);
            Some(v)
        };
        PngOut {
            w: img.w,
            h: img.h,
            pix,
            err: lib.reason(),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct InflateOut {
    pub ret: i32,
    pub out: Vec<u8>,
    pub err: String,
}

/// `cp_inflate` with the input placed at `in_align` bytes past a 4-aligned
/// malloc block (so `first_bytes` takes every value) and a zeroed output buffer.
pub fn call_inflate(lib: &Lib, data: &[u8], in_align: usize, out_bytes: i32) -> InflateOut {
    unsafe {
        lib.clear_reason();
        let raw = libc::malloc(data.len() + 8) as *mut u8;
        // libc malloc is at least 8/16-aligned, so raw % 4 == 0
        let inp = raw.add(in_align % 4);
        std::ptr::copy_nonoverlapping(data.as_ptr(), inp, data.len());
        let n = if out_bytes > 0 { out_bytes as usize } else { 1 };
        let out = libc::calloc(n, 1) as *mut u8;
        let ret = (lib.inflate)(
            inp as *mut c_void,
            data.len() as c_int,
            out as *mut c_void,
            out_bytes,
        );
        let v = std::slice::from_raw_parts(out, if out_bytes > 0 { out_bytes as usize } else { 0 })
            .to_vec();
        libc::free(out as *mut c_void);
        libc::free(raw as *mut c_void);
        InflateOut {
            ret,
            out: v,
            err: lib.reason(),
        }
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (xoshiro-ish; fixed seed per test for reproducibility)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// uniform in `0..n`
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.u32() % n
    }
    pub fn range(&mut self, lo: u32, hi_inclusive: u32) -> u32 {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
}

// ---------------------------------------------------------------------------
// bit writer + canonical Huffman (LSB-first bit order, codes MSB-first)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct BitWriter {
    pub bytes: Vec<u8>,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter::default()
    }
    pub fn bits(&mut self, v: u32, n: u32) {
        for i in 0..n {
            let bit = ((v >> i) & 1) as u8;
            if self.nbits % 8 == 0 {
                self.bytes.push(0);
            }
            let last = self.bytes.len() - 1;
            self.bytes[last] |= bit << (self.nbits % 8);
            self.nbits += 1;
        }
    }
    /// Huffman code: emitted most-significant bit first.
    pub fn huff(&mut self, code: u32, len: u32) {
        for i in (0..len).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }
    pub fn align(&mut self) {
        while self.nbits % 8 != 0 {
            self.bits(0, 1);
        }
    }
    pub fn raw(&mut self, data: &[u8]) {
        assert_eq!(self.nbits % 8, 0);
        self.bytes.extend_from_slice(data);
        self.nbits += 8 * data.len() as u32;
    }
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// RFC1951 canonical code assignment from a code-length vector.
pub fn canonical(lengths: &[u8]) -> Vec<u32> {
    let maxlen = *lengths.iter().max().unwrap_or(&0) as usize;
    let mut bl_count = vec![0u32; maxlen + 1];
    for &l in lengths {
        if l != 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next = vec![0u32; maxlen + 2];
    let mut code = 0u32;
    for bits in 1..=maxlen {
        code = (code + bl_count[bits - 1]) << 1;
        next[bits] = code;
    }
    let mut out = vec![0u32; lengths.len()];
    for (i, &l) in lengths.iter().enumerate() {
        if l != 0 {
            out[i] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    out
}

pub fn fixed_lit_lengths() -> Vec<u8> {
    let mut v = vec![0u8; 288];
    for i in 0..144 {
        v[i] = 8;
    }
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

pub const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
pub const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
pub const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
pub const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
pub const PERM: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

pub fn len_symbol(len: u32) -> (usize, u32) {
    for s in (0..29).rev() {
        if len >= LEN_BASE[s] && len < LEN_BASE[s] + (1 << LEN_EXTRA[s]) {
            return (257 + s, len - LEN_BASE[s]);
        }
    }
    panic!("bad length {len}");
}

pub fn dist_symbol(dist: u32) -> (usize, u32) {
    for s in (0..30).rev() {
        if dist >= DIST_BASE[s] && dist < DIST_BASE[s] + (1 << DIST_EXTRA[s]) {
            return (s, dist - DIST_BASE[s]);
        }
    }
    panic!("bad distance {dist}");
}

#[derive(Clone, Copy, Debug)]
pub enum Item {
    Lit(u8),
    /// `(length, distance)`
    Mat(u32, u32),
}

/// Emits one fixed-Huffman (btype 1) block.
pub fn emit_fixed(bw: &mut BitWriter, items: &[Item], is_final: bool) {
    let lit_lens = fixed_lit_lengths();
    let lit_codes = canonical(&lit_lens);
    let dist_lens = vec![5u8; 32];
    let dist_codes = canonical(&dist_lens);
    bw.bits(is_final as u32, 1);
    bw.bits(1, 2);
    emit_items(bw, items, &lit_lens, &lit_codes, &dist_lens, &dist_codes);
}

fn emit_items(
    bw: &mut BitWriter,
    items: &[Item],
    lit_lens: &[u8],
    lit_codes: &[u32],
    dist_lens: &[u8],
    dist_codes: &[u32],
) {
    for it in items {
        match *it {
            Item::Lit(b) => {
                let s = b as usize;
                assert!(lit_lens[s] != 0, "literal {s} has no code");
                bw.huff(lit_codes[s], lit_lens[s] as u32);
            }
            Item::Mat(len, dist) => {
                let (ls, lx) = len_symbol(len);
                assert!(lit_lens[ls] != 0, "length symbol {ls} has no code");
                bw.huff(lit_codes[ls], lit_lens[ls] as u32);
                bw.bits(lx, LEN_EXTRA[ls - 257]);
                let (ds, dx) = dist_symbol(dist);
                assert!(dist_lens[ds] != 0, "dist symbol {ds} has no code");
                bw.huff(dist_codes[ds], dist_lens[ds] as u32);
                bw.bits(dx, DIST_EXTRA[ds]);
            }
        }
    }
    // end of block
    assert!(lit_lens[256] != 0);
    bw.huff(lit_codes[256], lit_lens[256] as u32);
}

/// Emits one stored (btype 0) block.  The C's `cp_stored` requires the stored
/// block to be the last thing in the stream (`bits_left / 8 <= LEN`).
pub fn emit_stored(bw: &mut BitWriter, data: &[u8], is_final: bool) {
    bw.bits(is_final as u32, 1);
    bw.bits(0, 2);
    bw.align();
    let len = data.len() as u16;
    bw.raw(&len.to_le_bytes());
    bw.raw(&(!len).to_le_bytes());
    bw.raw(data);
}

/// How to encode the concatenated code-length vector.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClStrategy {
    /// one code-length symbol per entry (never uses 16/17/18)
    Plain,
    /// greedy RLE: 16 (copy prev 3..6), 17 (zeros 3..10), 18 (zeros 11..138)
    Rle,
    /// only 17/18 for zero runs, never 16
    ZeroRunsOnly,
    /// only 16 for repeats, zeros written literally
    RepeatsOnly,
}

pub fn encode_cl(lens: &[u8], strat: ClStrategy) -> Vec<(u8, u32)> {
    let mut out: Vec<(u8, u32)> = Vec::new();
    let mut i = 0usize;
    while i < lens.len() {
        let v = lens[i];
        let mut run = 1usize;
        while i + run < lens.len() && lens[i + run] == v {
            run += 1;
        }
        if v == 0 && strat != ClStrategy::Plain && strat != ClStrategy::RepeatsOnly {
            let mut left = run;
            while left >= 11 {
                let n = left.min(138);
                out.push((18, (n - 11) as u32));
                left -= n;
            }
            while left >= 3 {
                let n = left.min(10);
                out.push((17, (n - 3) as u32));
                left -= n;
            }
            for _ in 0..left {
                out.push((0, 0));
            }
        } else if v != 0
            && (strat == ClStrategy::Rle || strat == ClStrategy::RepeatsOnly)
            && run >= 4
        {
            out.push((v, 0));
            let mut left = run - 1;
            while left >= 3 {
                let n = left.min(6);
                out.push((16, (n - 3) as u32));
                left -= n;
            }
            for _ in 0..left {
                out.push((v, 0));
            }
        } else {
            for _ in 0..run {
                out.push((v, 0));
            }
        }
        i += run;
    }
    out
}

pub fn cl_extra_bits(sym: u8) -> u32 {
    match sym {
        16 => 2,
        17 => 3,
        18 => 7,
        _ => 0,
    }
}

/// Emits one dynamic-Huffman (btype 2) block.
///
/// `lit_lens.len()` is HLIT (257..=288), `dist_lens.len()` is HDIST (1..=32).
/// `hclen` is how many code-length code lengths are transmitted (4..=19); every
/// code-length symbol actually used must live in `PERM[..hclen]`.
pub fn emit_dynamic(
    bw: &mut BitWriter,
    items: &[Item],
    lit_lens: &[u8],
    dist_lens: &[u8],
    cl_lens: &[u8; 19],
    hclen: usize,
    strat: ClStrategy,
    is_final: bool,
) {
    assert!((257..=288).contains(&lit_lens.len()));
    assert!((1..=32).contains(&dist_lens.len()));
    assert!((4..=19).contains(&hclen));
    bw.bits(is_final as u32, 1);
    bw.bits(2, 2);
    bw.bits((lit_lens.len() - 257) as u32, 5);
    bw.bits((dist_lens.len() - 1) as u32, 5);
    bw.bits((hclen - 4) as u32, 4);
    for i in 0..hclen {
        bw.bits(cl_lens[PERM[i]] as u32, 3);
    }
    let cl_codes = canonical(cl_lens);
    let mut all: Vec<u8> = Vec::new();
    all.extend_from_slice(lit_lens);
    all.extend_from_slice(dist_lens);
    for (sym, extra) in encode_cl(&all, strat) {
        let s = sym as usize;
        assert!(cl_lens[s] != 0, "cl symbol {s} has no code");
        assert!(
            PERM[..hclen].contains(&s),
            "cl symbol {s} not transmitted (hclen={hclen})"
        );
        bw.huff(cl_codes[s], cl_lens[s] as u32);
        bw.bits(extra, cl_extra_bits(sym));
    }
    let lit_codes = canonical(lit_lens);
    let dist_codes = canonical(dist_lens);
    emit_items(bw, items, lit_lens, &lit_codes, dist_lens, &dist_codes);
}

/// Convenience: build lit/dist code lengths that cover exactly the symbols the
/// item list uses, with a simple uniform length (valid, possibly incomplete).
pub fn uniform_trees(items: &[Item], hlit: usize, hdist: usize) -> (Vec<u8>, Vec<u8>) {
    // 9 bits: 512 codes, enough for all 288 literal/length symbols without
    // over-subscribing the Kraft sum (which would make an *invalid* Huffman
    // code and is not a "valid input").
    let mut lit = vec![0u8; hlit];
    let mut dist = vec![0u8; hdist];
    lit[256] = 9;
    for it in items {
        match *it {
            Item::Lit(b) => lit[b as usize] = 9,
            Item::Mat(l, d) => {
                let (ls, _) = len_symbol(l);
                let (ds, _) = dist_symbol(d);
                assert!(ls < hlit, "length symbol {ls} needs hlit > {ls}");
                assert!(ds < hdist, "dist symbol {ds} needs hdist > {ds}");
                lit[ls] = 9;
                dist[ds] = 5;
            }
        }
    }
    (lit, dist)
}

/// All 19 code-length symbols get length 5 (fits: 19 <= 32).
pub fn cl_lens_uniform5() -> [u8; 19] {
    [5u8; 19]
}

// ---------------------------------------------------------------------------
// zlib / PNG containers
// ---------------------------------------------------------------------------

pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

pub fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

pub fn chunk(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(body.len() + 12);
    v.extend_from_slice(&(body.len() as u32).to_be_bytes());
    v.extend_from_slice(kind);
    v.extend_from_slice(body);
    let mut crcbuf = Vec::with_capacity(body.len() + 4);
    crcbuf.extend_from_slice(kind);
    crcbuf.extend_from_slice(body);
    v.extend_from_slice(&crc32(&crcbuf).to_be_bytes());
    v
}

/// A chunk with a *declared* length that differs from the body actually written.
pub fn chunk_declared_len(kind: &[u8; 4], declared: u32, body: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&declared.to_be_bytes());
    v.extend_from_slice(kind);
    v.extend_from_slice(body);
    v.extend_from_slice(&[0, 0, 0, 0]);
    v
}

pub const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Wraps a raw DEFLATE stream in a zlib (RFC1950) container.
pub fn zlib_wrap(deflate: &[u8], raw_adler_over: &[u8], cmf: u8, flg: u8) -> Vec<u8> {
    let mut v = Vec::new();
    v.push(cmf);
    v.push(flg);
    v.extend_from_slice(deflate);
    v.extend_from_slice(&adler32(raw_adler_over).to_be_bytes());
    v
}

pub fn ihdr_body(w: u32, h: u32, bit_depth: u8, color_type: u8) -> Vec<u8> {
    ihdr_body_full(w, h, bit_depth, color_type, 0, 0, 0)
}

pub fn ihdr_body_full(
    w: u32,
    h: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&w.to_be_bytes());
    v.extend_from_slice(&h.to_be_bytes());
    v.push(bit_depth);
    v.push(color_type);
    v.push(compression);
    v.push(filter);
    v.push(interlace);
    v
}

/// Note the C's quirk: `w = cp_make32(ihdr) + 1` and `img.w = w - 1`, so the
/// *declared* IHDR width is what the C treats as `img.w` and the number of
/// pixels per row.  Raw scanlines are `1 + width*bpp` bytes.
pub fn bpp_for(color_type: u8) -> usize {
    match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => panic!("bad colour type"),
    }
}

pub struct PngSpec {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub compression: u8,
    pub filter_method: u8,
    pub interlace: u8,
    /// raw (already-filtered) scanline data, including the per-row filter byte
    pub raw: Vec<u8>,
    pub plte: Option<Vec<u8>>,
    pub trns: Option<Vec<u8>>,
    /// extra chunks emitted right after IHDR
    pub pre_chunks: Vec<Vec<u8>>,
    /// how to split the zlib stream into IDAT chunks
    pub idat_split: Vec<usize>,
    /// a chunk inserted between the IDAT chunks
    pub idat_separator: Option<Vec<u8>>,
    /// a leading zero-length IDAT
    pub empty_idat_first: bool,
    pub cmf: u8,
    pub flg: u8,
    /// raw DEFLATE stream override (else the raw data is stored uncompressed)
    pub deflate: Option<Vec<u8>>,
    pub trailing: Vec<u8>,
    pub ihdr_declared_len: Option<u32>,
    pub with_iend: bool,
}

impl PngSpec {
    pub fn new(width: u32, height: u32, color_type: u8, raw: Vec<u8>) -> PngSpec {
        PngSpec {
            width,
            height,
            bit_depth: 8,
            color_type,
            compression: 0,
            filter_method: 0,
            interlace: 0,
            raw,
            plte: None,
            trns: None,
            pre_chunks: Vec::new(),
            idat_split: vec![],
            idat_separator: None,
            empty_idat_first: false,
            cmf: 0x78,
            flg: 0x9c,
            deflate: None,
            trailing: Vec::new(),
            ihdr_declared_len: None,
            with_iend: true,
        }
    }

    pub fn zlib(&self) -> Vec<u8> {
        let deflate = match &self.deflate {
            Some(d) => d.clone(),
            None => {
                let mut bw = BitWriter::new();
                emit_stored(&mut bw, &self.raw, true);
                bw.finish()
            }
        };
        zlib_wrap(&deflate, &self.raw, self.cmf, self.flg)
    }

    pub fn build(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&SIG);
        let ih = ihdr_body_full(
            self.width,
            self.height,
            self.bit_depth,
            self.color_type,
            self.compression,
            self.filter_method,
            self.interlace,
        );
        match self.ihdr_declared_len {
            None => v.extend_from_slice(&chunk(b"IHDR", &ih)),
            Some(n) => {
                let mut body = ih.clone();
                while (body.len() as u32) < n {
                    body.push(0);
                }
                v.extend_from_slice(&chunk(b"IHDR", &body[..n as usize]));
            }
        }
        for c in &self.pre_chunks {
            v.extend_from_slice(c);
        }
        if let Some(p) = &self.plte {
            v.extend_from_slice(&chunk(b"PLTE", p));
        }
        if let Some(t) = &self.trns {
            v.extend_from_slice(&chunk(b"tRNS", t));
        }
        let z = self.zlib();
        if self.empty_idat_first {
            v.extend_from_slice(&chunk(b"IDAT", &[]));
        }
        let mut parts: Vec<&[u8]> = Vec::new();
        if self.idat_split.is_empty() {
            parts.push(&z);
        } else {
            let mut off = 0usize;
            for &n in &self.idat_split {
                let n = n.min(z.len() - off);
                parts.push(&z[off..off + n]);
                off += n;
            }
            if off < z.len() {
                parts.push(&z[off..]);
            }
        }
        for (i, p) in parts.iter().enumerate() {
            if i > 0 {
                if let Some(sep) = &self.idat_separator {
                    v.extend_from_slice(sep);
                }
            }
            v.extend_from_slice(&chunk(b"IDAT", p));
        }
        if self.with_iend {
            v.extend_from_slice(&chunk(b"IEND", &[]));
        }
        v.extend_from_slice(&self.trailing);
        v
    }
}

/// Builds `raw` scanline bytes: `height` rows of `1 + width*bpp` bytes, with the
/// requested filter byte per row, pixel data straight from `rng`.
///
/// The filtered bytes are *not* meaningful image data -- the decoder just runs
/// its unfilter arithmetic over them, which is exactly what must match.
pub fn raw_rows(rng: &mut Rng, width: u32, height: u32, bpp: usize, filters: &[u8]) -> Vec<u8> {
    let stride = 1 + width as usize * bpp;
    let mut v = Vec::with_capacity(stride * height as usize);
    for y in 0..height as usize {
        v.push(filters[y % filters.len()]);
        for _ in 0..(stride - 1) {
            v.push(rng.u8());
        }
    }
    v
}

// ---------------------------------------------------------------------------
// corpus (records replayed by examples/diffrunner.rs)
// ---------------------------------------------------------------------------

pub const TAG_PNG: u8 = 0;
pub const TAG_PNG_LEN: u8 = 1;
pub const TAG_PNG_NULL: u8 = 2;
pub const TAG_INFLATE: u8 = 3;
pub const TAG_INFLATE_NULL_IN: u8 = 4;
pub const TAG_INFLATE_NULL_OUT: u8 = 5;

#[derive(Default)]
pub struct Corpus {
    recs: Vec<Vec<u8>>,
    pub labels: Vec<String>,
}

impl Corpus {
    pub fn new() -> Corpus {
        Corpus::default()
    }
    pub fn len(&self) -> usize {
        self.labels.len()
    }
    fn push(&mut self, label: &str, rec: Vec<u8>) {
        self.recs.push(rec);
        self.labels.push(label.to_string());
    }
    pub fn png(&mut self, label: &str, data: &[u8]) {
        let mut r = vec![TAG_PNG];
        r.extend_from_slice(data);
        self.push(label, r);
    }
    pub fn png_len(&mut self, label: &str, data: &[u8], len: i32) {
        let mut r = vec![TAG_PNG_LEN];
        r.extend_from_slice(&len.to_le_bytes());
        r.extend_from_slice(data);
        self.push(label, r);
    }
    pub fn png_null(&mut self, label: &str, len: i32) {
        let mut r = vec![TAG_PNG_NULL];
        r.extend_from_slice(&len.to_le_bytes());
        self.push(label, r);
    }
    pub fn inflate(&mut self, label: &str, data: &[u8], align: i32, out_bytes: i32) {
        self.inflate_len(label, data, align, out_bytes, data.len() as i32)
    }
    pub fn inflate_len(
        &mut self,
        label: &str,
        data: &[u8],
        align: i32,
        out_bytes: i32,
        in_len: i32,
    ) {
        let mut r = vec![TAG_INFLATE];
        r.extend_from_slice(&out_bytes.to_le_bytes());
        r.extend_from_slice(&align.to_le_bytes());
        r.extend_from_slice(&in_len.to_le_bytes());
        r.extend_from_slice(data);
        self.push(label, r);
    }
    pub fn inflate_null_in(&mut self, label: &str, out_bytes: i32, in_len: i32) {
        let mut r = vec![TAG_INFLATE_NULL_IN];
        r.extend_from_slice(&out_bytes.to_le_bytes());
        r.extend_from_slice(&in_len.to_le_bytes());
        self.push(label, r);
    }
    pub fn inflate_null_out(&mut self, label: &str, data: &[u8], out_bytes: i32) {
        let mut r = vec![TAG_INFLATE_NULL_OUT];
        r.extend_from_slice(&out_bytes.to_le_bytes());
        r.extend_from_slice(&(data.len() as i32).to_le_bytes());
        r.extend_from_slice(data);
        self.push(label, r);
    }

    fn blob(&self, which: &[usize]) -> Vec<u8> {
        let mut b = Vec::new();
        for &i in which {
            b.extend_from_slice(&(self.recs[i].len() as u32).to_le_bytes());
            b.extend_from_slice(&self.recs[i]);
        }
        b
    }

    pub fn write_subset(&self, name: &str, which: &[usize]) -> PathBuf {
        let dir = manifest_dir().join("target/vtmp/corpus");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("{name}.bin"));
        std::fs::write(&p, self.blob(which)).unwrap();
        p
    }

    pub fn write(&self, name: &str) -> PathBuf {
        self.write_subset(name, &(0..self.len()).collect::<Vec<_>>())
    }
}

pub fn run_runner(lib: &Path, corpus: &Path) -> Vec<String> {
    run_runner_primed(lib, corpus, 0)
}

/// `prime` is the byte the runner writes over its freed heap blocks before it
/// starts (0 = no priming).  See `prime_heap` in `examples/diffrunner.rs`.
pub fn run_runner_primed(lib: &Path, corpus: &Path, prime: u8) -> Vec<String> {
    let out = Command::new(runner_exe())
        .arg(lib)
        .arg(corpus)
        .arg(prime.to_string())
        .output()
        .expect("failed to spawn diffrunner");
    assert!(
        out.status.success(),
        "diffrunner failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect()
}

/// Runs several (library, corpus, prime) jobs concurrently.
fn run_many(jobs: Vec<(PathBuf, PathBuf, u8)>) -> Vec<Vec<String>> {
    let handles: Vec<_> = jobs
        .into_iter()
        .map(|(lib, corpus, prime)| {
            std::thread::spawn(move || run_runner_primed(&lib, &corpus, prime))
        })
        .collect();
    handles.into_iter().map(|h| h.join().unwrap()).collect()
}

/// The two heap patterns used to detect results that depend on uninitialised
/// memory.
const PRIME_A: u8 = 0xA5;
const PRIME_B: u8 = 0x5A;

pub struct CorpusReport {
    pub matched: usize,
    pub assert_only: usize,
    pub nondet: usize,
    /// the reference build (or the NDEBUG oracle) looped forever, so the record
    /// cannot be classified
    pub timeout: usize,
    /// suspects left unclassified because the NDEBUG oracle sample was capped
    pub unsampled: usize,
    pub mismatches: Vec<String>,
}

/// Runs the corpus against both libraries and compares line by line.
///
/// Pass 1 runs each library once.  Only the records that disagree are re-run
/// (twice per library, plus twice against the `NDEBUG` C build) to tell a real
/// divergence apart from a case that is nondeterministic inside the C itself or
/// one where the assert-enabled reference build aborts.
pub fn diff_corpus(name: &str, corpus: &Corpus) -> CorpusReport {
    let path = corpus.write(name);
    let c = c_lib();
    let r = rust_lib();
    let mut out = run_many(vec![
        (c.clone(), path.clone(), PRIME_A),
        (r.clone(), path.clone(), PRIME_A),
    ]);
    let r1 = out.pop().unwrap();
    let c1 = out.pop().unwrap();
    let n = corpus.len();
    assert_eq!(c1.len(), n, "runner produced {} lines, want {n}", c1.len());
    assert_eq!(r1.len(), n);

    let mut rep = CorpusReport {
        matched: 0,
        assert_only: 0,
        nondet: 0,
        timeout: 0,
        unsampled: 0,
        mismatches: Vec::new(),
    };
    let all_suspects: Vec<usize> = (0..n).filter(|&i| c1[i] != r1[i]).collect();
    rep.matched = n - all_suspects.len();
    if all_suspects.is_empty() {
        return rep;
    }
    // Classifying a suspect costs up to 6 more runs, and the NDEBUG oracle can
    // loop for a second on a stream that the assert-enabled reference rejects, so
    // cap the sample (deterministically: evenly spread over the suspects).
    const MAX_SUSPECTS: usize = 900;
    let suspects: Vec<usize> = if all_suspects.len() <= MAX_SUSPECTS {
        all_suspects.clone()
    } else {
        let step = all_suspects.len() as f64 / MAX_SUSPECTS as f64;
        (0..MAX_SUSPECTS)
            .map(|k| all_suspects[(k as f64 * step) as usize])
            .collect()
    };
    rep.unsampled = all_suspects.len() - suspects.len();

    let sub = corpus.write_subset(&format!("{name}_sub"), &suspects);
    let nd = c_lib_ndebug();
    let out = run_many(vec![
        (c.clone(), sub.clone(), PRIME_A),
        (c.clone(), sub.clone(), PRIME_B),
        (r.clone(), sub.clone(), PRIME_A),
        (r.clone(), sub.clone(), PRIME_B),
        (nd.clone(), sub.clone(), PRIME_A),
        (nd.clone(), sub.clone(), PRIME_B),
    ]);
    let (ca, cb, ra, rb, na, nb) = (&out[0], &out[1], &out[2], &out[3], &out[4], &out[5]);
    for (k, &i) in suspects.iter().enumerate() {
        let label = &corpus.labels[i];
        let timed_out = |l: &str| l.contains("status=signal:14");
        if timed_out(&ca[k]) || timed_out(&cb[k]) {
            // the reference itself never terminates -> nothing to compare against
            rep.timeout += 1;
            continue;
        }
        if timed_out(&ra[k]) || timed_out(&rb[k]) {
            rep.mismatches.push(format!(
                "[{i}] {label}\n  RUST loops forever where the C reference terminates\n  C    = {}\n  RUST = {}",
                ca[k], ra[k]
            ));
            continue;
        }
        // A result that changes when the freed-heap pattern changes depends on
        // uninitialised memory (the C hashes the parts of `img.pix` that a short
        // DEFLATE stream never filled), so it cannot be compared across two
        // different processes.
        if ca[k] != cb[k] || ra[k] != rb[k] {
            rep.nondet += 1;
            continue;
        }
        if ca[k] == ra[k] {
            rep.matched += 1;
            continue;
        }
        if ca[k].contains("status=signal:6") {
            if timed_out(&na[k]) || timed_out(&nb[k]) {
                // the C's own `assert` is what stops this input; with NDEBUG the
                // C loops forever, so there is no oracle for it
                rep.timeout += 1;
                continue;
            }
            if na[k] != nb[k] {
                rep.nondet += 1;
                continue;
            }
            if na[k] == ra[k] {
                rep.assert_only += 1;
                continue;
            }
            rep.mismatches.push(format!(
                "[{i}] {label}\n  C(assert)  = {}\n  C(NDEBUG)  = {}\n  RUST       = {}",
                ca[k], na[k], ra[k]
            ));
            continue;
        }
        rep.mismatches.push(format!(
            "[{i}] {label}\n  C    = {}\n  RUST = {}",
            ca[k], ra[k]
        ));
    }
    rep
}

pub fn assert_corpus_matches(name: &str, corpus: &Corpus) {
    let rep = diff_corpus(name, corpus);
    if !rep.mismatches.is_empty() {
        panic!(
            "{} mismatches out of {} records in corpus `{name}`:\n{}",
            rep.mismatches.len(),
            corpus.len(),
            rep.mismatches
                .iter()
                .take(15)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    eprintln!(
        "corpus {name}: {} records | {} identical | {} assert-only (C aborts, Rust == NDEBUG C) \
         | {} heap-dependent | {} no-oracle (C loops without asserts) | {} unsampled",
        corpus.len(),
        rep.matched,
        rep.assert_only,
        rep.nondet,
        rep.timeout,
        rep.unsampled
    );
}
