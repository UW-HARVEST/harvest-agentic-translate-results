//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `read_side_info` only through those dynamic exports. No Rust
//! function is ever called directly, so the `#[no_mangle]` wrapper is under
//! test too.
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// FFI types (must match c_src/include/lib.h exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: i32,
    pub limit: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct L3_gr_info_t {
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

const _: () = {
    assert!(size_of::<L3_gr_info_t>() == 32);
    assert!(size_of::<bs_t>() == 16);
};

pub const MAX_GR: usize = 4;

pub type ReadSideInfoFn =
    unsafe extern "C" fn(*mut bs_t, *mut L3_gr_info_t, *const u8) -> i32;

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found = None;
    for e in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C lib first.", build.display()))
    {
        let p = e.unwrap().path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found = Some(p);
        }
    }
    found.unwrap_or_else(|| panic!("no .so in {}", build.display()))
}

pub fn rust_so_path() -> PathBuf {
    let t = workspace_root().join("translation/target");
    // RUST_SO lets the caller point at a specific build (e.g. the
    // overflow-checked debug cdylib) instead of the default release one.
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO={} does not exist", p.display());
        return p;
    }
    for profile in ["release", "debug"] {
        let p = t.join(profile).join("libread_side_info_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libread_side_info_lib.so not found under {}; run `cargo build --release`",
        t.display()
    )
}

/// Holds both libraries plus the resolved `read_side_info` from each.
pub struct Pair {
    _c_lib: Library,
    _r_lib: Library,
    pub c: ReadSideInfoFn,
    pub r: ReadSideInfoFn,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("load C .so");
            let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
            let c: Symbol<ReadSideInfoFn> =
                c_lib.get(b"read_side_info\0").expect("C read_side_info");
            let r: Symbol<ReadSideInfoFn> = r_lib
                .get(b"read_side_info\0")
                .expect("Rust read_side_info (is #[no_mangle] present?)");
            let c = *c;
            let r = *r;
            Pair { _c_lib: c_lib, _r_lib: r_lib, c, r }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed -> reproducible)
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        for b in out.iter_mut() {
            *b = self.next_u32() as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Bit writer that emits fields in exactly the order `read_side_info` reads them
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub bits: Vec<u8>, // one entry per bit, value 0/1
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter { bits: Vec::new() }
    }
    pub fn put(&mut self, n: u32, v: u32) {
        for i in (0..n).rev() {
            self.bits.push(((v >> i) & 1) as u8);
        }
    }
    pub fn len(&self) -> usize {
        self.bits.len()
    }
    /// Pack into a byte buffer starting at bit offset `start_bit`; bits before
    /// `start_bit` and after the payload come from `filler`.
    pub fn pack(&self, start_bit: usize, total_bytes: usize, filler: &mut Rng) -> Vec<u8> {
        let mut buf = vec![0u8; total_bytes];
        filler.fill(&mut buf);
        for (i, &b) in self.bits.iter().enumerate() {
            let bit = start_bit + i;
            let byte = bit >> 3;
            if byte >= total_bytes {
                break;
            }
            let mask = 0x80u8 >> (bit & 7);
            if b != 0 {
                buf[byte] |= mask;
            } else {
                buf[byte] &= !mask;
            }
        }
        buf
    }
}

// ---------------------------------------------------------------------------
// Header helpers (only hdr[1], hdr[2], hdr[3] are read by the C)
// ---------------------------------------------------------------------------

/// `hdr[1] & 0x8`
pub fn hdr_is_mpeg1(hdr: &[u8; 4]) -> bool {
    hdr[1] & 0x8 != 0
}
/// `(hdr[3] & 0xC0) == 0xC0`
pub fn hdr_is_mono(hdr: &[u8; 4]) -> bool {
    hdr[3] & 0xC0 == 0xC0
}
pub fn hdr_sr_idx(hdr: &[u8; 4]) -> i32 {
    let h1 = hdr[1] as i32;
    let h2 = hdr[2] as i32;
    let mut sr = ((h2 >> 2) & 3) + (((h1 >> 3) & 1) + ((h1 >> 4) & 1)) * 3;
    sr -= (sr != 0) as i32;
    sr
}
pub fn hdr_gr_count(hdr: &[u8; 4]) -> i32 {
    let mut g = if hdr_is_mono(hdr) { 1 } else { 2 };
    if hdr_is_mpeg1(hdr) {
        g *= 2;
    }
    g
}

/// Build a `hdr` with the requested MPEG1 flag, mono flag and `sr_idx`.
/// Returns `None` if the combination is unreachable.
pub fn make_hdr(mpeg1: bool, mono: bool, sr_idx: i32, rng: &mut Rng) -> Option<[u8; 4]> {
    for bit4 in [0u8, 1u8] {
        let base = ((if mpeg1 { 1 } else { 0 }) + bit4 as i32) * 3;
        for lo in 0..4i32 {
            let mut v = base + lo;
            v -= (v != 0) as i32;
            if v == sr_idx {
                let mut hdr = [0u8; 4];
                // hdr[0] is never read; randomise it.
                hdr[0] = rng.next_u32() as u8;
                // hdr[1]: bit3 = mpeg1, bit4 = bit4; other bits are unread ->
                // randomise them to prove they are ignored.
                let mut h1 = (rng.next_u32() as u8) & !0x18;
                if mpeg1 {
                    h1 |= 0x8;
                }
                h1 |= bit4 << 4;
                hdr[1] = h1;
                // hdr[2]: bits 2-3 = lo; other bits unread -> randomise.
                hdr[2] = ((rng.next_u32() as u8) & !0x0C) | ((lo as u8) << 2);
                // hdr[3]: bits 6-7 decide mono; rest unread -> randomise.
                let mut h3 = (rng.next_u32() as u8) & !0xC0;
                if mono {
                    h3 |= 0xC0;
                } else {
                    // any of 0x00/0x40/0x80
                    h3 |= [0x00u8, 0x40, 0x80][rng.below(3) as usize];
                }
                hdr[3] = h3;
                debug_assert_eq!(hdr_sr_idx(&hdr), sr_idx);
                debug_assert_eq!(hdr_is_mpeg1(&hdr), mpeg1);
                debug_assert_eq!(hdr_is_mono(&hdr), mono);
                return Some(hdr);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Side-info generation: mirrors the C read order so any configuration can be
// produced deliberately.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct GrChoice {
    /// window-switching bit
    pub w: bool,
    /// block_type (only used when `w`); 0 forces the E3 error path
    pub bt: u32,
    pub mb: bool,
    pub part_23_length: u32,
    pub big_values: u32,
    pub scalefac_compress: Option<u32>,
}

impl GrChoice {
    pub fn random(rng: &mut Rng) -> GrChoice {
        GrChoice {
            w: rng.bool(),
            bt: 1 + rng.below(3),
            mb: rng.bool(),
            part_23_length: rng.below(4096),
            big_values: rng.below(289),
            scalefac_compress: None,
        }
    }
}

/// Emit a full side-info bit string for `hdr`, honouring `choices`
/// (one per granule). `main_data_begin` is the raw field value.
pub fn build_side_info(
    hdr: &[u8; 4],
    main_data_begin: u32,
    scfsi_raw: u32,
    choices: &[GrChoice],
    rng: &mut Rng,
) -> BitWriter {
    let mut w = BitWriter::new();
    let mpeg1 = hdr_is_mpeg1(hdr);
    let gr_count = hdr_gr_count(hdr);

    if mpeg1 {
        w.put(9, main_data_begin);
        w.put((7 + gr_count) as u32, scfsi_raw);
    } else {
        // C reads get_bits(8+gr_count) then >> gr_count, so the top
        // (8) bits are main_data_begin and the low gr_count bits are dropped.
        let n = (8 + gr_count) as u32;
        let v = (main_data_begin << gr_count) | rng.below(1 << gr_count);
        w.put(n, v & ((1u32 << n) - 1));
    }

    for g in 0..gr_count as usize {
        let c = choices[g];
        w.put(12, c.part_23_length & 0xFFF);
        w.put(9, c.big_values & 0x1FF);
        w.put(8, rng.below(256)); // global_gain
        let sfc_bits = if mpeg1 { 4 } else { 9 };
        let sfc = c
            .scalefac_compress
            .unwrap_or_else(|| rng.below(1 << sfc_bits));
        w.put(sfc_bits, sfc & ((1 << sfc_bits) - 1));
        w.put(1, c.w as u32); // window switching
        if c.w {
            w.put(2, c.bt & 3);
            if c.bt & 3 == 0 {
                // C returns -1 right here; nothing more is read.
                return w;
            }
            w.put(1, c.mb as u32);
            w.put(10, rng.below(1024)); // tables
            w.put(3, rng.below(8)); // subblock_gain[0]
            w.put(3, rng.below(8));
            w.put(3, rng.below(8));
        } else {
            w.put(15, rng.below(1 << 15)); // tables
            w.put(4, rng.below(16)); // region_count[0]
            w.put(3, rng.below(8)); // region_count[1]
        }
        if mpeg1 {
            w.put(1, rng.below(2)); // preflag
        }
        w.put(1, rng.below(2)); // scalefac_scale
        w.put(1, rng.below(2)); // count1_table
        if c.big_values > 288 {
            // C returned -1 after reading big_values; the rest is unread
            // filler, but leaving it in the stream is harmless.
        }
    }
    w
}

// ---------------------------------------------------------------------------
// Differential invocation + comparison
// ---------------------------------------------------------------------------

/// Byte pattern used to pre-fill the granule array in both runs so that fields
/// the C leaves untouched are compared meaningfully.
pub const PREFILL: u8 = 0xA5;

pub struct Outcome {
    pub ret: i32,
    pub bs: bs_t,
    pub gr: [L3_gr_info_t; MAX_GR],
    /// bytes read through each granule's `sfbtab`, if it was written
    pub sfb: [Option<Vec<u8>>; MAX_GR],
}

/// How many bytes of the table `sfbtab` points at are meaningful, deduced from
/// the `(n_long_sfb, n_short_sfb)` pair the C sets alongside each assignment.
fn sfb_len(g: &L3_gr_info_t) -> Option<usize> {
    match (g.n_long_sfb, g.n_short_sfb) {
        (22, 0) => Some(23),        // g_scf_long
        (0, 39) => Some(40),        // g_scf_short
        (8, 30) | (6, 30) => Some(40), // g_scf_mixed
        _ => None,                  // sfbtab not written on this path
    }
}

unsafe fn invoke(
    f: ReadSideInfoFn,
    buf: &[u8],
    pos: i32,
    limit: i32,
    hdr: &[u8; 4],
    read_sfb: bool,
) -> Outcome {
    let mut bs = bs_t { buf: buf.as_ptr(), pos, limit };
    let mut gr = [L3_gr_info_t {
        sfbtab: PREFILL as usize as *const u8,
        part_23_length: u16::from_le_bytes([PREFILL, PREFILL]),
        big_values: u16::from_le_bytes([PREFILL, PREFILL]),
        scalefac_compress: u16::from_le_bytes([PREFILL, PREFILL]),
        global_gain: PREFILL,
        block_type: PREFILL,
        mixed_block_flag: PREFILL,
        n_long_sfb: PREFILL,
        n_short_sfb: PREFILL,
        table_select: [PREFILL; 3],
        region_count: [PREFILL; 3],
        subblock_gain: [PREFILL; 3],
        preflag: PREFILL,
        scalefac_scale: PREFILL,
        count1_table: PREFILL,
        scfsi: PREFILL,
    }; MAX_GR];

    let ret = unsafe { f(&mut bs, gr.as_mut_ptr(), hdr.as_ptr()) };

    let mut sfb: [Option<Vec<u8>>; MAX_GR] = [None, None, None, None];
    if read_sfb {
        for i in 0..MAX_GR {
            if let Some(n) = sfb_len(&gr[i]) {
                if !gr[i].sfbtab.is_null() && gr[i].sfbtab as usize != PREFILL as usize {
                    sfb[i] =
                        Some(unsafe { std::slice::from_raw_parts(gr[i].sfbtab, n) }.to_vec());
                }
            }
        }
    }
    Outcome { ret, bs, gr, sfb }
}

fn gr_bytes(g: &L3_gr_info_t) -> [u8; 24] {
    // everything except the 8-byte `sfbtab` pointer (which legitimately
    // differs between the two libraries' static tables)
    let raw: [u8; 32] = unsafe { std::mem::transmute(*g) };
    let mut out = [0u8; 24];
    out.copy_from_slice(&raw[8..32]);
    out
}

/// Whether `sfbtab` was written at all (compared as a tri-state, since the
/// pointer *values* differ by design).
fn sfb_written(g: &L3_gr_info_t) -> bool {
    g.sfbtab as usize != PREFILL as usize
}

pub struct Case<'a> {
    pub label: String,
    pub buf: &'a [u8],
    pub pos: i32,
    pub limit: i32,
    pub hdr: [u8; 4],
    /// false when the C would form an out-of-range table address (sr_idx == 8),
    /// in which case the pointed-to bytes are not comparable across libraries.
    pub compare_sfb_contents: bool,
}

/// Run both libraries on the same case and assert byte-identical results.
pub fn diff(p: &Pair, case: &Case) {
    let read = case.compare_sfb_contents;
    let c = unsafe { invoke(p.c, case.buf, case.pos, case.limit, &case.hdr, read) };
    let r = unsafe { invoke(p.r, case.buf, case.pos, case.limit, &case.hdr, read) };

    let ctx = || {
        format!(
            "{}\n  hdr={:02X?} pos={} limit={} buf[0..16]={:02X?}\n  mpeg1={} mono={} sr_idx={} gr_count={}",
            case.label,
            case.hdr,
            case.pos,
            case.limit,
            &case.buf[..case.buf.len().min(16)],
            hdr_is_mpeg1(&case.hdr),
            hdr_is_mono(&case.hdr),
            hdr_sr_idx(&case.hdr),
            hdr_gr_count(&case.hdr),
        )
    };

    assert_eq!(c.ret, r.ret, "return value diverges\n{}", ctx());
    assert_eq!(c.bs.pos, r.bs.pos, "bs.pos diverges\n{}", ctx());
    assert_eq!(c.bs.limit, r.bs.limit, "bs.limit diverges\n{}", ctx());
    assert_eq!(c.bs.buf, r.bs.buf, "bs.buf diverges\n{}", ctx());

    for i in 0..MAX_GR {
        assert_eq!(
            gr_bytes(&c.gr[i]),
            gr_bytes(&r.gr[i]),
            "granule {} body diverges\n  C: {:?}\n  R: {:?}\n{}",
            i,
            dump(&c.gr[i]),
            dump(&r.gr[i]),
            ctx()
        );
        assert_eq!(
            sfb_written(&c.gr[i]),
            sfb_written(&r.gr[i]),
            "granule {} sfbtab written-ness diverges\n{}",
            i,
            ctx()
        );
        if read {
            assert_eq!(
                c.sfb[i], r.sfb[i],
                "granule {} sfbtab table contents diverge\n{}",
                i,
                ctx()
            );
        }
    }
}

pub fn dump(g: &L3_gr_info_t) -> String {
    format!(
        "p23={} bv={} sfc={} gg={} bt={} mbf={} nl={} ns={} ts={:?} rc={:?} sbg={:?} pf={} sfs={} c1={} scfsi={}",
        g.part_23_length,
        g.big_values,
        g.scalefac_compress,
        g.global_gain,
        g.block_type,
        g.mixed_block_flag,
        g.n_long_sfb,
        g.n_short_sfb,
        g.table_select,
        g.region_count,
        g.subblock_gain,
        g.preflag,
        g.scalefac_scale,
        g.count1_table,
        g.scfsi
    )
}

/// Convenience: pack a side-info bit string into a padded buffer and diff.
/// Returns the number of side-info bits.
pub struct Packed {
    pub buf: Vec<u8>,
    pub pos: i32,
    pub nbits: usize,
}

pub const PAD: usize = 64;

/// Build a buffer with `lead` bytes of leading slack (so negative `pos` values
/// still land in mapped memory and read identical bytes in both libraries) and
/// `PAD` bytes of trailing slack.
pub fn pack_with_pad(w: &BitWriter, lead: usize, align: usize, rng: &mut Rng) -> Packed {
    let start_bit = lead * 8 + align;
    let total = lead + (align + w.len() + 7) / 8 + PAD + 8;
    let buf = w.pack(start_bit, total, rng);
    Packed { buf, pos: start_bit as i32, nbits: w.len() }
}

/// `sr_idx` values reachable for a given MPEG1 flag (derived from
/// `sr_idx = ((hdr[2]>>2)&3) + (((hdr[1]>>3)&1)+((hdr[1]>>4)&1))*3` then
/// `-= (sr_idx != 0)`).
pub fn reachable_sr(mpeg1: bool) -> &'static [i32] {
    if mpeg1 {
        &[2, 3, 4, 5, 6, 7, 8]
    } else {
        &[0, 1, 2, 3, 4, 5]
    }
}
