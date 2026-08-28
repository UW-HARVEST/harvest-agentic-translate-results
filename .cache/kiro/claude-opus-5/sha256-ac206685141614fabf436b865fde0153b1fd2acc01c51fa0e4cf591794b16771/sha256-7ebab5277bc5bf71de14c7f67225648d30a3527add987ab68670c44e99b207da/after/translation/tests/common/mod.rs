//! Shared harness: loads the C reference `.so` and the Rust `.so` and calls
//! `read_side_info` through the FFI boundary in both, so only exported symbols
//! are exercised.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GrInfo {
    pub sfbtab: *const u8,
    pub part_23_length: u16,
    pub big_values: u16,
    pub scalefac_compress: u16,
    pub global_gain: u8,
    pub block_type: u8,
    pub mixed_block_flag: u8,
    pub n_long_sfb: u8,
    pub n_short_sfb: u8,
    pub table_select: [u8; 3],
    pub region_count: [u8; 3],
    pub subblock_gain: [u8; 3],
    pub preflag: u8,
    pub scalefac_scale: u8,
    pub count1_table: u8,
    pub scfsi: u8,
}

/// The struct has no interior padding (8 + 6 + 5 + 9 + 4 == 32), so raw byte
/// comparison of the non-pointer tail is meaningful.
pub const GR_SIZE: usize = 32;
const _: () = assert!(std::mem::size_of::<GrInfo>() == GR_SIZE);
const _: () = assert!(std::mem::align_of::<GrInfo>() == 8);

pub const POISON: u8 = 0xA5;
/// Number of granule slots handed to the callee (max real usage is 4).
pub const NGR: usize = 8;

type ReadSideInfoFn =
    unsafe extern "C" fn(*mut BsT, *mut GrInfo, *const u8) -> c_int;

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    f: ReadSideInfoFn,
    /// `&g_scf_long[0][0]`, `&g_scf_short[0][0]`, `&g_scf_mixed[0][0]`
    pub bases: [*const u8; 3],
}

/// Everything one call produced: the return value, the mutated reader state and
/// the raw granule bytes. The `sfbtab` pointer cannot be compared literally
/// (the two libraries place their tables at different addresses), so it is
/// normalized to the set of `(table, row)` identities it matches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ret: c_int,
    pub pos: c_int,
    pub limit: c_int,
    /// Per granule: `None` if the slot was never written (still poison),
    /// otherwise the table indices (0=long, 1=short, 2=mixed) whose row
    /// `sr_idx` the pointer equals.
    pub sfbtab: Vec<Option<Vec<usize>>>,
    /// Per granule: the 24 bytes after the pointer field.
    pub tail: Vec<[u8; GR_SIZE - 8]>,
}

/// Row stride of `g_scf_long` / `g_scf_short` / `g_scf_mixed`.
pub const STRIDE: [usize; 3] = [23, 40, 40];

/// `sr_idx` as computed by `read_side_info`; the row index into every table.
pub fn sr_idx_of(hdr: &[u8; 4]) -> i32 {
    let raw = (((hdr[2] >> 2) & 3) as i32)
        + ((((hdr[1] >> 3) & 1) + ((hdr[1] >> 4) & 1)) as i32) * 3;
    raw - (raw != 0) as i32
}

impl Lib {
    pub fn call(&self, buf: &[u8], pos: c_int, limit: c_int, hdr: &[u8; 4]) -> Outcome {
        let mut grs = [0u8; GR_SIZE * NGR];
        grs.fill(POISON);
        let mut bs = BsT {
            buf: buf.as_ptr(),
            pos,
            limit,
        };
        let ret = unsafe { (self.f)(&mut bs, grs.as_mut_ptr().cast(), hdr.as_ptr()) };

        let row = sr_idx_of(hdr) as usize;
        let poison_ptr = usize::from_ne_bytes([POISON; 8]);
        let mut sfbtab = Vec::with_capacity(NGR);
        let mut tail = Vec::with_capacity(NGR);
        for i in 0..NGR {
            let base = i * GR_SIZE;
            let p = usize::from_ne_bytes(grs[base..base + 8].try_into().unwrap());
            sfbtab.push(if p == poison_ptr {
                None
            } else {
                let hits: Vec<usize> = (0..3)
                    .filter(|&t| p == self.bases[t] as usize + row * STRIDE[t])
                    .collect();
                assert!(
                    !hits.is_empty(),
                    "[{}] sfbtab {p:#x} matches no table row (sr_idx={row}, \
                     bases={:#x?}) hdr={hdr:02X?}",
                    self.name,
                    self.bases.map(|b| b as usize)
                );
                Some(hits)
            });
            tail.push(grs[base + 8..base + GR_SIZE].try_into().unwrap());
        }
        Outcome {
            ret,
            pos: bs.pos,
            limit: bs.limit,
            sfbtab,
            tail,
        }
    }

    /// Reads `len` bytes of the scalefactor-band row that `sfbtab` points at.
    /// Only valid for in-range rows.
    pub fn read_row(&self, buf: &[u8], pos: c_int, limit: c_int, hdr: &[u8; 4], len: usize) -> Vec<u8> {
        let mut grs = [0u8; GR_SIZE * NGR];
        grs.fill(POISON);
        let mut bs = BsT {
            buf: buf.as_ptr(),
            pos,
            limit,
        };
        unsafe { (self.f)(&mut bs, grs.as_mut_ptr().cast(), hdr.as_ptr()) };
        let p = usize::from_ne_bytes(grs[0..8].try_into().unwrap()) as *const u8;
        assert_ne!(p as usize, usize::from_ne_bytes([POISON; 8]), "sfbtab unwritten");
        unsafe { std::slice::from_raw_parts(p, len).to_vec() }
    }
}

fn load(name: &'static str, path: &Path) -> Lib {
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
    let f: ReadSideInfoFn = unsafe {
        let sym: libloading::Symbol<ReadSideInfoFn> = lib
            .get(b"read_side_info\0")
            .unwrap_or_else(|e| panic!("no read_side_info in {}: {e}", path.display()));
        *sym
    };
    let mut me = Lib {
        name,
        _lib: lib,
        f,
        bases: [std::ptr::null(); 3],
    };
    me.bases = probe_bases(&me);
    me
}

/// Recovers the three table base addresses by driving the decoder down the
/// long / short / mixed paths with `sr_idx == 0`.
fn probe_bases(lib: &Lib) -> [*const u8; 3] {
    let mut out = [std::ptr::null(); 3];
    for (i, variant) in [Sel::Long, Sel::Short, Sel::Mixed].into_iter().enumerate() {
        let buf = base_probe_stream(variant);
        let mut grs = [0u8; GR_SIZE * NGR];
        grs.fill(POISON);
        let mut bs = BsT {
            buf: buf.as_ptr(),
            pos: 0,
            limit: (buf.len() * 8) as c_int,
        };
        // hdr[1] = 0, hdr[2] = 0 -> sr_idx == 0; hdr[3] = 0 -> gr_count == 2.
        let hdr = [0xFFu8, 0x00, 0x00, 0x00];
        unsafe { (lib.f)(&mut bs, grs.as_mut_ptr().cast(), hdr.as_ptr()) };
        let p = usize::from_ne_bytes(grs[0..8].try_into().unwrap());
        assert_ne!(p, usize::from_ne_bytes([POISON; 8]), "base probe failed");
        out[i] = p as *const u8;
    }
    assert!(out[0] != out[1] && out[1] != out[2] && out[0] != out[2]);
    out
}

#[derive(Clone, Copy)]
enum Sel {
    Long,
    Short,
    Mixed,
}

/// MPEG2/2.5 side info (hdr[1] & 0x8 == 0) whose first granule selects a given
/// table. Field widths follow `read_side_info` exactly.
fn base_probe_stream(sel: Sel) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.put(10, 0); // main_data_begin field (8 + gr_count, gr_count == 2)
    w.put(12, 0); // part_23_length
    w.put(9, 0); // big_values (<= 288 so we do not bail out)
    w.put(8, 0); // global_gain
    w.put(9, 0); // scalefac_compress
    match sel {
        Sel::Long => w.put(1, 0), // no window switching -> g_scf_long
        Sel::Short => {
            w.put(1, 1);
            w.put(2, 2); // block_type == 2
            w.put(1, 0); // mixed_block_flag == 0 -> g_scf_short
        }
        Sel::Mixed => {
            w.put(1, 1);
            w.put(2, 2);
            w.put(1, 1); // mixed_block_flag == 1 -> g_scf_mixed
        }
    }
    w.finish(64)
}

pub struct BitWriter {
    bits: Vec<u8>,
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter { bits: Vec::new() }
    }
    pub fn put(&mut self, n: u32, v: u32) {
        for i in (0..n).rev() {
            self.bits.push(((v >> i) & 1) as u8);
        }
    }
    pub fn len_bits(&self) -> usize {
        self.bits.len()
    }
    /// Packs MSB-first into at least `min_bytes` bytes, zero padded.
    pub fn finish(&self, min_bytes: usize) -> Vec<u8> {
        let mut out = vec![0u8; min_bytes.max((self.bits.len() + 7) / 8)];
        for (i, b) in self.bits.iter().enumerate() {
            if *b != 0 {
                out[i / 8] |= 0x80 >> (i % 8);
            }
        }
        out
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn pair() -> Pair {
    Pair {
        c: load("C", &c_so()),
        rs: load("Rust", &rust_so()),
    }
}

impl Pair {
    /// Runs one input through both libraries and asserts full agreement.
    pub fn check(&self, label: &str, buf: &[u8], pos: c_int, limit: c_int, hdr: &[u8; 4]) {
        let a = self.c.call(buf, pos, limit, hdr);
        let b = self.rs.call(buf, pos, limit, hdr);
        if a != b {
            panic!(
                "mismatch [{label}] hdr={hdr:02X?} pos={pos} limit={limit}\n\
                 buf={buf:02X?}\n C   : ret={} pos={} sfbtab={:?}\n Rust: ret={} pos={} sfbtab={:?}\n\
                 tail C   : {:02X?}\n tail Rust: {:02X?}",
                a.ret, a.pos, a.sfbtab, b.ret, b.pos, b.sfbtab, a.tail, b.tail
            );
        }
    }
}

fn rust_so() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>/
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe.parent().unwrap().parent().unwrap();
    let p = dir.join("libread_side_info_lib.so");
    assert!(p.exists(), "rust cdylib not found at {}", p.display());
    p
}

fn c_so() -> PathBuf {
    let build = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build");
    let mut found = None;
    for e in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("build the C library first ({}): {e}", build.display()))
    {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no lib*.so in {}", build.display()))
}

/// Small deterministic PRNG so failures reproduce.
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u32() as u8).collect()
    }
}
