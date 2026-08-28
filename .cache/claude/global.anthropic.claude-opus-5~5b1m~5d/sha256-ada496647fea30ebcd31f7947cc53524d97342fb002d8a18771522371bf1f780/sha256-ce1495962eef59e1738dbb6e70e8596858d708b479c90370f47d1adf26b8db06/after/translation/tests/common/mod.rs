//! Shared differential-testing harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `read_side_info`
//! only through their exported symbols — the Rust implementation is never
//! called directly, so the `#[unsafe(no_mangle)] extern "C"` wrapper and the
//! struct ABI are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirrors of the C types (independent re-declaration: if `src/lib.rs` got
// the layout wrong, these tests would catch it).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

/// `sizeof(L3_gr_info_t)` from the C compiler.
pub const GR_INFO_SIZE: usize = 32;

pub type RsiFn = unsafe extern "C" fn(*mut BsT, *mut GrInfo, *const u8) -> c_int;

/// Number of `L3_gr_info_t` slots handed to the callee. The C code writes at
/// most `gr_count == 4`; the extra slots detect over-writing.
pub const NGR: usize = 8;

/// Byte size of the three scalefactor-band tables as laid out in the C
/// object's `.rodata` (`g_scf_long` 184 + 8 pad + `g_scf_short` 320 +
/// `g_scf_mixed` 320). Reading past this in the C library leaves `.rodata`.
pub const RODATA_TABLE_BYTES: isize = 832;

pub const OFF_LONG: isize = 0;
pub const OFF_SHORT: isize = 192;
pub const OFF_MIXED: isize = 512;

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let dir = crate_root().parent().unwrap().join("c_src/build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C .so under {}\nbuild it with:\n  cd {} && mkdir -p build && cd build && \\\n  \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display(),
            crate_root().parent().unwrap().join("c_src").display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    const NAME: &str = "libread_side_info_lib.so";
    // .../target/<profile>/deps/<test-binary>  ->  .../target/<profile>/<NAME>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            let p = profile_dir.join(NAME);
            if p.is_file() {
                return p;
            }
        }
    }
    for profile in ["release", "debug"] {
        let p = crate_root().join("target").join(profile).join(NAME);
        if p.is_file() {
            return p;
        }
    }
    panic!("could not locate {NAME}; run `cargo build --release` first");
}

pub struct Harness {
    _c_lib: Library,
    _r_lib: Library,
    pub c_fn: RsiFn,
    pub r_fn: RsiFn,
    /// `g_scf_long[0]` in each library — the anchor all `sfbtab` pointers are
    /// normalised against so the two address spaces become comparable.
    pub c_long_base: *const u8,
    pub r_long_base: *const u8,
}

// The libraries stay loaded for the whole process; the fn pointers are plain
// code addresses and the base pointers point at read-only data.
unsafe impl Send for Harness {}
unsafe impl Sync for Harness {}

static HARNESS: OnceLock<Harness> = OnceLock::new();

pub fn harness() -> &'static Harness {
    HARNESS.get_or_init(|| {
        assert_eq!(
            std::mem::size_of::<GrInfo>(),
            GR_INFO_SIZE,
            "L3_gr_info_t size mismatch"
        );
        assert_eq!(std::mem::size_of::<BsT>(), 16, "bs_t size mismatch");

        let c_path = find_c_so();
        let r_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let r_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));
            let c_sym: Symbol<RsiFn> = c_lib
                .get(b"read_side_info\0")
                .expect("C .so does not export read_side_info");
            let r_sym: Symbol<RsiFn> = r_lib
                .get(b"read_side_info\0")
                .expect("Rust .so does not export read_side_info");
            let c_fn = *c_sym;
            let r_fn = *r_sym;

            let c_long_base = calibrate_long_base(c_fn);
            let r_long_base = calibrate_long_base(r_fn);

            Harness {
                _c_lib: c_lib,
                _r_lib: r_lib,
                c_fn,
                r_fn,
                c_long_base,
                r_long_base,
            }
        }
    })
}

/// Recovers `&g_scf_long[0][0]` from a library by parsing a minimal side info
/// with `sr_idx == 0` on the non-window-switching path, where the C code always
/// assigns `gr->sfbtab = g_scf_long[sr_idx]`.
unsafe fn calibrate_long_base(f: RsiFn) -> *const u8 {
    // hdr[1] bit3 = 0 (mpeg2) and bit4 = 0, hdr[2] sample-rate bits = 0
    // => raw sr sum 0 => sr_idx 0.  hdr[3] top bits 0xC0 => 1 granule.
    let hdr = [0u8, 0x00, 0x00, 0xC0];
    let mut gi = GranuleSpec::zeroed();
    gi.ws = false; // long block => sfbtab = g_scf_long[0]
    let data = build_side_info(&HdrSpec::from_bytes(&hdr), 0, 0, 0, &[gi]);
    let mut buf = vec![0u8; data.len() + 64];
    buf[..data.len()].copy_from_slice(&data);
    let mut bs = BsT {
        buf: buf.as_ptr(),
        pos: 0,
        limit: (buf.len() * 8) as c_int,
    };
    let mut gr = [GrInfo::filled(0); NGR];
    let ret = unsafe { f(&mut bs, gr.as_mut_ptr(), hdr.as_ptr()) };
    assert!(ret >= 0, "calibration parse failed with {ret}");
    assert_eq!(gr[0].n_long_sfb, 22, "calibration did not take the long path");
    assert!(!gr[0].sfbtab.is_null(), "calibration produced a null sfbtab");
    gr[0].sfbtab
}

impl GrInfo {
    /// A `L3_gr_info_t` whose every byte is `b`. Used to pre-fill the caller's
    /// array so that fields the C code never writes (notably `region_count[2]`
    /// on the window-switching path) can be checked for identical retention.
    pub fn filled(b: u8) -> Self {
        unsafe { std::mem::transmute([b; GR_INFO_SIZE]) }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), so every "randomized" case is reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// `n` random bits.
    pub fn bits(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() >> (32 - n)
        }
    }
    pub fn boolean(&mut self) -> bool {
        self.next_u32() & 1 != 0
    }
}

// ---------------------------------------------------------------------------
// Bit writer — emits fields MSB-first, exactly the order `get_bits` reads them
// ---------------------------------------------------------------------------

pub struct BitWriter {
    bytes: Vec<u8>,
    nbits: usize,
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            nbits: 0,
        }
    }
    pub fn bit_len(&self) -> usize {
        self.nbits
    }
    pub fn put(&mut self, val: u32, n: u32) {
        for i in (0..n).rev() {
            let bit = ((val >> i) & 1) as u8;
            let byte = self.nbits >> 3;
            let shift = 7 - (self.nbits & 7);
            if byte >= self.bytes.len() {
                self.bytes.push(0);
            }
            self.bytes[byte] |= bit << shift;
            self.nbits += 1;
        }
    }
    /// Pad with `n` bits taken from `pattern` (so leading padding is not all
    /// zeroes and a misaligned start is actually exercised).
    pub fn put_pattern(&mut self, n: usize, pattern: u64) {
        let mut p = pattern;
        for _ in 0..n {
            self.put((p & 1) as u32, 1);
            p = p.rotate_right(1);
        }
    }
    pub fn into_bytes(mut self) -> Vec<u8> {
        // Trailing slack so `get_bits` never reads past the allocation.
        for _ in 0..16 {
            self.bytes.push(0);
        }
        self.bytes
    }
}

// ---------------------------------------------------------------------------
// Side-info construction
// ---------------------------------------------------------------------------

/// The three header bits `read_side_info` actually branches on.
#[derive(Clone, Copy, Debug)]
pub struct HdrSpec {
    /// `hdr[1] & 0x8` — MPEG1. Doubles `gr_count`, narrows
    /// `scalefac_compress` to 4 bits, enables the `scfsi` read and the
    /// bitstream `preflag`, and contributes 3 to the raw `sr_idx` sum.
    pub mpeg1: bool,
    /// `(hdr[1] >> 4) & 1` — the other `sr_idx` contributor.
    pub hdr1_bit4: bool,
    /// `(hdr[2] >> 2) & 3`.
    pub sr2: u8,
    /// `(hdr[3] & 0xC0) == 0xC0` — single channel.
    pub mono: bool,
}

impl HdrSpec {
    pub fn to_bytes(self, filler: &mut Rng) -> [u8; 4] {
        // Every bit the function does not read is randomised, proving it is
        // genuinely ignored.
        let mut h = [0u8; 4];
        h[0] = filler.bits(8) as u8; // never read by read_side_info
        h[1] = (filler.bits(8) as u8 & !0x18)
            | if self.mpeg1 { 0x08 } else { 0 }
            | if self.hdr1_bit4 { 0x10 } else { 0 };
        h[2] = (filler.bits(8) as u8 & !0x0C) | ((self.sr2 & 3) << 2);
        h[3] = if self.mono {
            (filler.bits(8) as u8 & !0xC0) | 0xC0
        } else {
            // any top-2-bit value other than 0b11
            (filler.bits(8) as u8 & !0xC0) | (filler.below(3) as u8) << 6
        };
        h
    }

    pub fn from_bytes(h: &[u8; 4]) -> Self {
        HdrSpec {
            mpeg1: h[1] & 0x8 != 0,
            hdr1_bit4: (h[1] >> 4) & 1 != 0,
            sr2: (h[2] >> 2) & 3,
            mono: (h[3] & 0xC0) == 0xC0,
        }
    }

    /// Replicates the C computation at lines 87–89.
    pub fn sr_idx(&self) -> i32 {
        let hdr1 = (if self.mpeg1 { 0x08 } else { 0 } | if self.hdr1_bit4 { 0x10 } else { 0 }) as i32;
        let raw = (self.sr2 as i32) + (((hdr1 >> 3) & 1) + ((hdr1 >> 4) & 1)) * 3;
        raw - (raw != 0) as i32
    }

    /// Replicates the C computation at lines 90–92.
    pub fn gr_count(&self) -> u32 {
        let base = if self.mono { 1 } else { 2 };
        if self.mpeg1 { base * 2 } else { base }
    }
}

/// Every field of one granule, in bitstream order.
#[derive(Clone, Copy, Debug)]
pub struct GranuleSpec {
    pub part_23_length: u32,
    pub big_values: u32,
    pub global_gain: u32,
    pub scalefac_compress: u32,
    pub ws: bool,
    pub block_type: u32,
    pub mixed: u32,
    /// 10 bits when `ws`, 15 bits otherwise.
    pub tables: u32,
    pub subblock_gain: [u32; 3],
    pub region_count0: u32,
    pub region_count1: u32,
    pub preflag: u32,
    pub scalefac_scale: u32,
    pub count1_table: u32,
}

impl GranuleSpec {
    pub fn zeroed() -> Self {
        GranuleSpec {
            part_23_length: 0,
            big_values: 0,
            global_gain: 0,
            scalefac_compress: 0,
            ws: false,
            block_type: 1,
            mixed: 0,
            tables: 0,
            subblock_gain: [0; 3],
            region_count0: 0,
            region_count1: 0,
            preflag: 0,
            scalefac_scale: 0,
            count1_table: 0,
        }
    }

    /// Random values in every field; branch selectors left to the caller.
    pub fn random(rng: &mut Rng) -> Self {
        GranuleSpec {
            part_23_length: rng.bits(12),
            big_values: rng.below(289), // stay inside the valid range
            global_gain: rng.bits(8),
            scalefac_compress: rng.bits(9),
            ws: rng.boolean(),
            block_type: 1 + rng.below(3), // 1..=3, avoid the E6 rejection
            mixed: rng.bits(1),
            tables: rng.bits(15),
            subblock_gain: [rng.bits(3), rng.bits(3), rng.bits(3)],
            region_count0: rng.bits(4),
            region_count1: rng.bits(3),
            preflag: rng.bits(1),
            scalefac_scale: rng.bits(1),
            count1_table: rng.bits(1),
        }
    }
}

/// Serialises a complete side-info payload in exactly the order the C code
/// consumes it. `start_bit` bits of non-zero padding are emitted first so the
/// caller can start the parse at any `bs->pos`.
pub fn build_side_info(
    hdr: &HdrSpec,
    start_bit: usize,
    main_data_begin: u32,
    scfsi: u32,
    granules: &[GranuleSpec],
) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.put_pattern(start_bit, 0xB5D9_2E47_1C6A_F308);
    let gr_count = hdr.gr_count();

    if hdr.mpeg1 {
        w.put(main_data_begin & 0x1FF, 9);
        w.put(scfsi, 7 + gr_count);
    } else {
        // C: main_data_begin = get_bits(8 + gr_count) >> gr_count, so the low
        // `gr_count` bits are private_bits and get discarded.
        let private_bits = scfsi & ((1 << gr_count) - 1);
        w.put(((main_data_begin & 0xFF) << gr_count) | private_bits, 8 + gr_count);
    }

    for g in granules.iter().take(gr_count as usize) {
        w.put(g.part_23_length & 0xFFF, 12);
        w.put(g.big_values & 0x1FF, 9);
        w.put(g.global_gain & 0xFF, 8);
        if hdr.mpeg1 {
            w.put(g.scalefac_compress & 0xF, 4);
        } else {
            w.put(g.scalefac_compress & 0x1FF, 9);
        }
        w.put(g.ws as u32, 1);
        if g.ws {
            w.put(g.block_type & 3, 2);
            w.put(g.mixed & 1, 1);
            w.put(g.tables & 0x3FF, 10);
            w.put(g.subblock_gain[0] & 7, 3);
            w.put(g.subblock_gain[1] & 7, 3);
            w.put(g.subblock_gain[2] & 7, 3);
        } else {
            w.put(g.tables & 0x7FFF, 15);
            w.put(g.region_count0 & 0xF, 4);
            w.put(g.region_count1 & 7, 3);
        }
        if hdr.mpeg1 {
            w.put(g.preflag & 1, 1);
        }
        w.put(g.scalefac_scale & 1, 1);
        w.put(g.count1_table & 1, 1);
    }
    w.into_bytes()
}

/// Total number of bits `read_side_info` consumes for a given shape, assuming
/// no early error return. Lets tests place `bs->limit` exactly.
pub fn side_info_bits(hdr: &HdrSpec, granules: &[GranuleSpec]) -> usize {
    let gr_count = hdr.gr_count() as usize;
    let mut n = if hdr.mpeg1 {
        9 + 7 + gr_count
    } else {
        8 + gr_count
    };
    for g in granules.iter().take(gr_count) {
        n += 12 + 9 + 8;
        n += if hdr.mpeg1 { 4 } else { 9 };
        n += 1;
        n += if g.ws { 2 + 1 + 10 + 3 + 3 + 3 } else { 15 + 4 + 3 };
        if hdr.mpeg1 {
            n += 1;
        }
        n += 1 + 1;
    }
    n
}

// ---------------------------------------------------------------------------
// Running a case against both libraries and comparing the results
// ---------------------------------------------------------------------------

/// Bytes of deterministic guard data placed before and after the bitstream so
/// that a negative `bs->pos`, or a `bs->limit` larger than the payload, reads
/// defined bytes instead of unmapped memory. Both libraries are handed the
/// *same* pointer, so even those out-of-bounds reads must agree.
pub const GUARD: usize = 512;

#[derive(Clone, Debug)]
pub struct Case {
    pub hdr: [u8; 4],
    /// Bitstream bytes; `bs->buf` points at byte 0 of this.
    pub data: Vec<u8>,
    pub pos: c_int,
    pub limit: c_int,
    /// Byte written into every slot of the caller's `L3_gr_info_t` array
    /// before the call.
    pub prefill: u8,
}

impl Case {
    pub fn new(hdr: [u8; 4], data: Vec<u8>) -> Self {
        let limit = (data.len() * 8) as c_int;
        Case {
            hdr,
            data,
            pos: 0,
            limit,
            prefill: 0,
        }
    }
    pub fn pos(mut self, p: c_int) -> Self {
        self.pos = p;
        self
    }
    pub fn limit(mut self, l: c_int) -> Self {
        self.limit = l;
        self
    }
    pub fn prefill(mut self, b: u8) -> Self {
        self.prefill = b;
        self
    }
}

/// Where an `sfbtab` pointer ended up, expressed as a byte offset from
/// `g_scf_long[0]` in the *same* library. This is what makes the two address
/// spaces comparable, and it pins down the table, the row, **and** the relative
/// layout of the three tables.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Sfbtab {
    /// The C code never assigned the field; the caller's pre-fill survived.
    Untouched(usize),
    Assigned {
        offset: isize,
        /// `None` when the row lies past the end of `.rodata` in the C
        /// library, where there is no library data to compare (CONFIGS C17).
        row: Option<Vec<u8>>,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GrNorm {
    pub sfbtab: Sfbtab,
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub ret: c_int,
    pub pos: c_int,
    pub limit: c_int,
    pub gr: Vec<GrNorm>,
}

fn normalise(gr: &GrInfo, long_base: *const u8, prefill: u8) -> GrNorm {
    let sentinel = usize::from_ne_bytes([prefill; 8]);
    let sfbtab = if gr.sfbtab as usize == sentinel {
        Sfbtab::Untouched(sentinel)
    } else {
        let offset = (gr.sfbtab as isize) - (long_base as isize);
        // g_scf_long rows are 23 bytes, g_scf_short / g_scf_mixed rows 40.
        let row_len: isize = if offset < OFF_SHORT { 23 } else { 40 };
        let row = if offset >= 0 && offset + row_len <= RODATA_TABLE_BYTES {
            Some(unsafe { std::slice::from_raw_parts(gr.sfbtab, row_len as usize).to_vec() })
        } else {
            None
        };
        Sfbtab::Assigned { offset, row }
    };
    GrNorm {
        sfbtab,
        part_23_length: gr.part_23_length,
        big_values: gr.big_values,
        scalefac_compress: gr.scalefac_compress,
        global_gain: gr.global_gain,
        block_type: gr.block_type,
        mixed_block_flag: gr.mixed_block_flag,
        n_long_sfb: gr.n_long_sfb,
        n_short_sfb: gr.n_short_sfb,
        table_select: gr.table_select,
        region_count: gr.region_count,
        subblock_gain: gr.subblock_gain,
        preflag: gr.preflag,
        scalefac_scale: gr.scalefac_scale,
        count1_table: gr.count1_table,
        scfsi: gr.scfsi,
    }
}

impl Harness {
    fn run_one(&self, case: &Case, which: Which) -> Outcome {
        // Guarded buffer: identical contents for both libraries, so reads that
        // stray outside the payload still produce identical bytes.
        let mut backing = vec![0u8; GUARD * 2 + case.data.len()];
        for (i, b) in backing.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(0x5A);
        }
        backing[GUARD..GUARD + case.data.len()].copy_from_slice(&case.data);

        let (f, long_base) = match which {
            Which::C => (self.c_fn, self.c_long_base),
            Which::Rust => (self.r_fn, self.r_long_base),
        };

        let mut bs = BsT {
            buf: unsafe { backing.as_ptr().add(GUARD) },
            pos: case.pos,
            limit: case.limit,
        };
        let mut gr = [GrInfo::filled(case.prefill); NGR];
        let ret = unsafe { f(&mut bs, gr.as_mut_ptr(), case.hdr.as_ptr()) };

        Outcome {
            ret,
            pos: bs.pos,
            limit: bs.limit,
            gr: gr
                .iter()
                .map(|g| normalise(g, long_base, case.prefill))
                .collect(),
        }
    }

    pub fn run_c(&self, case: &Case) -> Outcome {
        self.run_one(case, Which::C)
    }
    pub fn run_rust(&self, case: &Case) -> Outcome {
        self.run_one(case, Which::Rust)
    }

    /// Runs `case` through both `.so`s and asserts byte-for-byte equality of
    /// the return value, the mutated `bs_t`, and all `NGR` output structs.
    #[track_caller]
    pub fn assert_same(&self, label: &str, case: &Case) -> Outcome {
        let c = self.run_c(case);
        let r = self.run_rust(case);
        if c != r {
            let mut msg = format!(
                "DIVERGENCE [{label}]\n  hdr = {:02x?}\n  pos = {}  limit = {}  prefill = {:#04x}\n  \
                 data[0..24] = {:02x?}\n",
                case.hdr,
                case.pos,
                case.limit,
                case.prefill,
                &case.data[..case.data.len().min(24)]
            );
            if c.ret != r.ret {
                msg += &format!("  ret:   C = {:<12} Rust = {}\n", c.ret, r.ret);
            }
            if c.pos != r.pos {
                msg += &format!("  pos:   C = {:<12} Rust = {}\n", c.pos, r.pos);
            }
            if c.limit != r.limit {
                msg += &format!("  limit: C = {:<12} Rust = {}\n", c.limit, r.limit);
            }
            for i in 0..NGR {
                if c.gr[i] != r.gr[i] {
                    msg += &format!("  granule {i}:\n    C    = {:?}\n    Rust = {:?}\n", c.gr[i], r.gr[i]);
                }
            }
            panic!("{msg}");
        }
        c
    }
}

#[derive(Clone, Copy)]
enum Which {
    C,
    Rust,
}

// ---------------------------------------------------------------------------
// Coverage tracking — proves a randomized row really did reach every branch
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Coverage {
    seen: std::collections::BTreeSet<String>,
}

impl Coverage {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn mark(&mut self, what: impl Into<String>) {
        self.seen.insert(what.into());
    }
    pub fn observe(&mut self, o: &Outcome, gr_count: usize) {
        for g in o.gr.iter().take(gr_count) {
            if let Sfbtab::Assigned { offset, .. } = g.sfbtab {
                let table = if offset < OFF_SHORT {
                    "long"
                } else if offset < OFF_MIXED {
                    "short"
                } else {
                    "mixed"
                };
                self.mark(format!(
                    "bt={} mixed={} table={}",
                    g.block_type, g.mixed_block_flag, table
                ));
            }
        }
    }
    #[track_caller]
    pub fn require(&self, expected: &[&str]) {
        let missing: Vec<&&str> = expected.iter().filter(|e| !self.seen.contains(**e)).collect();
        assert!(
            missing.is_empty(),
            "randomized run never reached: {missing:?}\n  reached: {:?}",
            self.seen
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario — a complete, self-describing input shape
// ---------------------------------------------------------------------------

/// Everything needed to synthesise one call. Knows how many bits the C code
/// will consume and where the `main_data_begin` reservoir check (C line 159)
/// flips, so tests can place `bs->limit` exactly on either side of it.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub hs: HdrSpec,
    pub start_bit: usize,
    pub main_data_begin: u32,
    pub scfsi: u32,
    pub gs: Vec<GranuleSpec>,
}

impl Scenario {
    pub fn new(hs: HdrSpec, gs: Vec<GranuleSpec>) -> Self {
        Scenario {
            hs,
            start_bit: 0,
            main_data_begin: 0,
            scfsi: 0,
            gs,
        }
    }
    pub fn start_bit(mut self, b: usize) -> Self {
        self.start_bit = b;
        self
    }
    pub fn main_data_begin(mut self, v: u32) -> Self {
        self.main_data_begin = v;
        self
    }
    pub fn scfsi(mut self, v: u32) -> Self {
        self.scfsi = v;
        self
    }
    pub fn gr_count(&self) -> usize {
        self.hs.gr_count() as usize
    }
    /// Bits consumed by a complete, error-free parse.
    pub fn bits(&self) -> usize {
        side_info_bits(&self.hs, &self.gs)
    }
    /// `part_23_sum` as the C code accumulates it (12-bit fields, `gr_count`
    /// granules).
    pub fn part23_sum(&self) -> i64 {
        self.gs
            .iter()
            .take(self.gr_count())
            .map(|g| (g.part_23_length & 0xFFF) as i64)
            .sum()
    }
    /// Bit position the parse ends at.
    pub fn end_pos(&self) -> i64 {
        self.start_bit as i64 + self.bits() as i64
    }
    /// The largest `bs->limit` for which C line 159 still returns `-1` is
    /// `boundary_limit() - 1`; `boundary_limit()` itself is the first value
    /// that succeeds (the check is a strict `>`).
    pub fn boundary_limit(&self) -> i64 {
        self.part23_sum() + self.end_pos() - (self.main_data_begin as i64) * 8
    }
    pub fn data(&self) -> Vec<u8> {
        build_side_info(
            &self.hs,
            self.start_bit,
            self.main_data_begin,
            self.scfsi,
            &self.gs,
        )
    }
    /// A case with a comfortable bit budget: the reservoir check passes, so an
    /// error-free parse must return `main_data_begin`.
    pub fn case(&self, rng: &mut Rng) -> Case {
        let lim = self.boundary_limit() + rng.below(64) as i64;
        self.case_with_limit(rng, lim as c_int)
    }
    pub fn case_with_limit(&self, rng: &mut Rng, limit: c_int) -> Case {
        Case {
            hdr: self.hs.to_bytes(rng),
            data: self.data(),
            pos: self.start_bit as c_int,
            limit,
            prefill: 0,
        }
    }
}

/// Pre-fill patterns cycled through so that fields the C code leaves alone
/// (`region_count[2]` on the window-switching path) are checked for identical
/// retention rather than accidentally matching on zero.
pub const PREFILLS: [u8; 4] = [0x00, 0xAA, 0x5A, 0xFF];

/// All 16 reachable `(mpeg1, hdr1_bit4, sr2)` header-bit combinations, together
/// with the `sr_idx` each produces. Note `sr_idx == 8` — one row past the end
/// of every table — is reachable only with `mpeg1 && hdr1_bit4 && sr2 == 3`.
pub fn hdr_bit_combos() -> Vec<(bool, bool, u8, i32)> {
    let mut v = Vec::new();
    for mpeg1 in [false, true] {
        for hdr1_bit4 in [false, true] {
            for sr2 in 0u8..4 {
                let hs = HdrSpec {
                    mpeg1,
                    hdr1_bit4,
                    sr2,
                    mono: true,
                };
                v.push((mpeg1, hdr1_bit4, sr2, hs.sr_idx()));
            }
        }
    }
    v
}

/// Which scalefactor-band table a granule configuration selects, and the
/// expected byte offset of row `sr_idx` from `g_scf_long[0]`.
pub fn expected_sfbtab_offset(ws: bool, block_type: u32, mixed: u32, sr_idx: i32) -> isize {
    let idx = sr_idx as isize;
    if ws && block_type == 2 {
        if mixed == 0 {
            OFF_SHORT + 40 * idx
        } else {
            OFF_MIXED + 40 * idx
        }
    } else {
        OFF_LONG + 23 * idx
    }
}

// ---------------------------------------------------------------------------
// Aliased-`hdr` runs
// ---------------------------------------------------------------------------

impl Harness {
    /// Calls `read_side_info` with `hdr` pointing **into** the caller's
    /// `L3_gr_info_t` array, at byte offset `k`. The C code re-reads `hdr[1]`
    /// and `hdr[3]` from inside the granule loop, interleaved with its writes
    /// through `gr`, so this pins down *when* those loads happen — a translation
    /// that hoisted them into locals would diverge here.
    fn run_aliased(
        &self,
        which: bool, // true = C
        hdr: [u8; 4],
        k: usize,
        data: &[u8],
        pos: c_int,
        limit: c_int,
        prefill: u8,
    ) -> Outcome {
        let mut backing = vec![0u8; GUARD * 2 + data.len()];
        for (i, b) in backing.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(0x5A);
        }
        backing[GUARD..GUARD + data.len()].copy_from_slice(data);

        let (f, long_base) = if which {
            (self.c_fn, self.c_long_base)
        } else {
            (self.r_fn, self.r_long_base)
        };

        let mut bs = BsT {
            buf: unsafe { backing.as_ptr().add(GUARD) },
            pos,
            limit,
        };
        let mut gr = [GrInfo::filled(prefill); NGR];
        assert!(k + 4 <= NGR * GR_INFO_SIZE, "hdr must fit inside the gr array");
        let ret = unsafe {
            let base = gr.as_mut_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(hdr.as_ptr(), base.add(k), 4);
            f(&mut bs, gr.as_mut_ptr(), base.add(k) as *const u8)
        };
        Outcome {
            ret,
            pos: bs.pos,
            limit: bs.limit,
            gr: gr.iter().map(|g| normalise(g, long_base, prefill)).collect(),
        }
    }

    #[track_caller]
    pub fn assert_same_aliased(
        &self,
        label: &str,
        hdr: [u8; 4],
        k: usize,
        data: &[u8],
        pos: c_int,
        limit: c_int,
        prefill: u8,
    ) -> Outcome {
        let c = self.run_aliased(true, hdr, k, data, pos, limit, prefill);
        let r = self.run_aliased(false, hdr, k, data, pos, limit, prefill);
        assert_eq!(
            c, r,
            "DIVERGENCE [{label}] with hdr aliasing gr at byte offset {k}\n  \
             hdr = {hdr:02x?} pos = {pos} limit = {limit} prefill = {prefill:#04x}"
        );
        c
    }
}
