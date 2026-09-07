//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported `read_side_info` symbol — the Rust
//! function is never called directly, so the `#[no_mangle] extern "C"` wrapper
//! and the C ABI struct layout are part of what is under test.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI types (mirrors of the C header — declared here independently of the
// crate under test so a layout mistake in src/lib.rs cannot hide itself).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: i32,
    pub limit: i32,
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

const _: () = assert!(core::mem::size_of::<GrInfo>() == 32);
const _: () = assert!(core::mem::align_of::<GrInfo>() == 8);
const _: () = assert!(core::mem::size_of::<BsT>() == 16);

pub type ReadSideInfoFn = unsafe extern "C" fn(*mut BsT, *mut GrInfo, *const u8) -> i32;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Some(p) = std::env::var_os("C_SO") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    assert!(
        !candidates.is_empty(),
        "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    candidates.sort();
    candidates.remove(0)
}

fn find_rust_so() -> PathBuf {
    // Allow pinning a specific build profile: RUST_SO=<path> or PROFILE=release.
    if let Some(p) = std::env::var_os("RUST_SO") {
        return PathBuf::from(p);
    }
    let target = repo_root().join("translation").join("target");
    if let Ok(prof) = std::env::var("RUST_SO_PROFILE") {
        return target.join(prof).join("libread_side_info_lib.so");
    }
    // Prefer the profile the tests themselves were built with, then the other.
    let profiles = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for prof in profiles {
        let p = target.join(prof).join("libread_side_info_lib.so");
        if let Ok(md) = std::fs::metadata(&p) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if newest.as_ref().map_or(true, |(bt, _)| t > *bt) {
                newest = Some((t, p));
            }
        }
    }
    newest
        .map(|(_, p)| p)
        .unwrap_or_else(|| panic!("libread_side_info_lib.so not found under {}", target.display()))
}

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: ReadSideInfoFn,
    pub rust: ReadSideInfoFn,
}

// Safety: both libraries stay loaded for the process lifetime (leaked via
// OnceLock) and `read_side_info` is a pure function of its arguments.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

static IMPLS: OnceLock<Impls> = OnceLock::new();

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| unsafe {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust_lib = Library::new(&r_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", r_path.display()));
        let c: Symbol<ReadSideInfoFn> = c_lib
            .get(b"read_side_info\0")
            .expect("C .so does not export read_side_info");
        let rust: Symbol<ReadSideInfoFn> = rust_lib
            .get(b"read_side_info\0")
            .expect("Rust .so does not export read_side_info");
        let c = *c;
        let rust = *rust;
        Impls { _c_lib: c_lib, _rust_lib: rust_lib, c, rust }
    })
}

// ---------------------------------------------------------------------------
// Observable outcome of one call
// ---------------------------------------------------------------------------

/// Everything an external caller can observe: the return value, the mutated
/// `bs_t`, and the raw 32 bytes of every granule slot. `sfbtab` is a pointer
/// into the callee's own read-only data so its numeric value necessarily
/// differs; it is compared via `sfbtab_bytes` (the pointed-to row) instead, and
/// via `sfbtab_kind` (which of the three tables and which row it selects).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub ret: i32,
    pub pos: i32,
    pub limit: i32,
    /// 32 raw bytes per granule slot with the 8 `sfbtab` pointer bytes zeroed.
    pub gr_bytes: Vec<[u8; 32]>,
    /// `sfbtab[0..len]` for each granule slot, or `None` if `sfbtab` was never
    /// written (still the sentinel the test pre-filled).
    pub sfbtab_bytes: Vec<Option<Vec<u8>>>,
}

/// How many granule slots the C would touch for this header.
pub fn gr_count_for(hdr: &[u8; 4]) -> usize {
    let mut n = if (hdr[3] & 0xC0) == 0xC0 { 1 } else { 2 };
    if (hdr[1] & 0x8) != 0 {
        n *= 2;
    }
    n
}

pub fn sr_idx_for(hdr: &[u8; 4]) -> i32 {
    let h1 = hdr[1] as i32;
    let h2 = hdr[2] as i32;
    let mut s = ((h2 >> 2) & 3) + (((h1 >> 3) & 1) + ((h1 >> 4) & 1)) * 3;
    s -= (s != 0) as i32;
    s
}

/// Number of `sfbtab` bytes worth comparing: the full declared row length of
/// whichever table was selected. Long rows are 23 bytes, short/mixed rows 40.
fn sfbtab_len(gr: &GrInfo) -> usize {
    if gr.n_long_sfb == 22 && gr.n_short_sfb == 0 {
        23 // g_scf_long
    } else {
        40 // g_scf_short or g_scf_mixed
    }
}

/// Sentinel `sfbtab` value used to detect "never written".
const SFBTAB_SENTINEL: usize = 0xDEAD_BEEF_0000_0001;

/// Byte pattern the granule array is pre-filled with, so that fields the C
/// leaves untouched are compared as "untouched" rather than as zero.
pub const GR_FILL: u8 = 0xA5;

fn fresh_gr_slots(n: usize) -> Vec<GrInfo> {
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        // Build from a filled byte pattern, then plant the pointer sentinel.
        let mut raw = [GR_FILL; 32];
        raw[0..8].copy_from_slice(&SFBTAB_SENTINEL.to_ne_bytes());
        v.push(unsafe { core::mem::transmute::<[u8; 32], GrInfo>(raw) });
    }
    v
}

/// Run one implementation and capture everything observable.
///
/// `slots` granule slots are provided (always >= what the C touches, so the C
/// never writes out of bounds). `skip_sfbtab_deref` suppresses reading through
/// `sfbtab` (used for the one case where C points outside its own `.rodata`).
unsafe fn run_one(
    f: ReadSideInfoFn,
    buf: &[u8],
    pos: i32,
    limit: i32,
    hdr: &[u8; 4],
    slots: usize,
    skip_sfbtab_deref: bool,
) -> Outcome {
    let mut bs = BsT { buf: buf.as_ptr(), pos, limit };
    let mut grs = fresh_gr_slots(slots);
    let ret = unsafe { f(&mut bs, grs.as_mut_ptr(), hdr.as_ptr()) };

    let mut gr_bytes = Vec::with_capacity(slots);
    let mut sfbtab_bytes = Vec::with_capacity(slots);
    for gr in &grs {
        let raw: [u8; 32] = unsafe { core::mem::transmute_copy(gr) };
        let mut masked = raw;
        masked[0..8].fill(0); // pointer value is implementation-specific
        gr_bytes.push(masked);

        let ptr_val = usize::from_ne_bytes(raw[0..8].try_into().unwrap());
        if ptr_val == SFBTAB_SENTINEL || gr.sfbtab.is_null() || skip_sfbtab_deref {
            sfbtab_bytes.push(None);
        } else {
            let n = sfbtab_len(gr);
            let s = unsafe { core::slice::from_raw_parts(gr.sfbtab, n) };
            sfbtab_bytes.push(Some(s.to_vec()));
        }
    }
    Outcome { ret, pos: bs.pos, limit: bs.limit, gr_bytes, sfbtab_bytes }
}

/// Call C and Rust with identical inputs and return `(c_outcome, rust_outcome)`.
pub fn run_both(buf: &[u8], pos: i32, limit: i32, hdr: &[u8; 4]) -> (Outcome, Outcome) {
    run_both_opts(buf, pos, limit, hdr, false)
}

pub fn run_both_opts(
    buf: &[u8],
    pos: i32,
    limit: i32,
    hdr: &[u8; 4],
    skip_sfbtab_deref: bool,
) -> (Outcome, Outcome) {
    let im = impls();
    // 4 slots is the maximum gr_count; always pass 4 so neither side can write
    // past the array regardless of the header.
    let slots = 4;
    let c = unsafe { run_one(im.c, buf, pos, limit, hdr, slots, skip_sfbtab_deref) };
    let r = unsafe { run_one(im.rust, buf, pos, limit, hdr, slots, skip_sfbtab_deref) };
    (c, r)
}

/// Which of the three tables is the LAST one in the C `.so`'s `.rodata`, and
/// therefore the one whose out-of-range row 8 runs off the end of the section
/// into unreproducible bytes (ERRORS.md N5).
///
/// `-O0` (the default `cmake ..` build) emits long, short, mixed → `mixed`.
/// `-O2` (`-DCMAKE_BUILD_TYPE=Release`) emits mixed, short, long → `long`.
/// Override with `UNREPRODUCIBLE_ROW8=long|short|mixed|none`.
pub fn unreproducible_row8_kind() -> &'static str {
    static K: OnceLock<String> = OnceLock::new();
    K.get_or_init(|| std::env::var("UNREPRODUCIBLE_ROW8").unwrap_or_else(|_| "mixed".into()))
}

/// `sr_idx == 8` selects a row one past the end of a table. For two of the
/// three tables that row still lands inside the C's own `.rodata` and IS
/// compared byte-for-byte; for the last table in the section it does not, and
/// only that dereference is skipped.
pub fn deref_is_unreproducible(hdr: &[u8; 4], c: &Outcome) -> bool {
    if sr_idx_for(hdr) != 8 {
        return false;
    }
    // Per granule: block_type @15, mixed_block_flag @16, n_long_sfb @17,
    // n_short_sfb @18 identify which table `sfbtab` was set from.
    let kind = unreproducible_row8_kind();
    c.gr_bytes.iter().any(|g| match kind {
        // g_scf_mixed: block_type==2, mixed_block_flag==1, n_short_sfb==30
        "mixed" => g[15] == 2 && g[16] == 1 && g[18] == 30,
        // g_scf_short: block_type==2, mixed_block_flag==0, n_short_sfb==39
        "short" => g[15] == 2 && g[16] == 0 && g[18] == 39,
        // g_scf_long: the default assignment, n_long_sfb==22 && n_short_sfb==0
        "long" => g[17] == 22 && g[18] == 0,
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// Assertion
// ---------------------------------------------------------------------------

fn describe(o: &Outcome, n: usize) -> String {
    let mut s = format!("ret={} pos={} limit={}\n", o.ret, o.pos, o.limit);
    for i in 0..n.min(o.gr_bytes.len()) {
        s += &format!("  gr[{i}] bytes = {:02x?}\n", o.gr_bytes[i]);
        s += &format!("        sfbtab = {:?}\n", o.sfbtab_bytes[i]);
    }
    s
}

pub fn assert_same(label: &str, buf: &[u8], pos: i32, limit: i32, hdr: &[u8; 4]) {
    let (c, r) = run_both(buf, pos, limit, hdr);
    let (c, r) = if c != r && deref_is_unreproducible(hdr, &c) {
        run_both_opts(buf, pos, limit, hdr, true)
    } else {
        (c, r)
    };
    if c != r {
        let n = gr_count_for(hdr);
        panic!(
            "DIVERGENCE [{label}]\n\
             hdr = {hdr:02x?}  pos={pos} limit={limit} gr_count={n} sr_idx={}\n\
             buf[..32] = {:02x?}\n\
             --- C ---\n{}\
             --- RUST ---\n{}",
            sr_idx_for(hdr),
            &buf[..buf.len().min(32)],
            describe(&c, 4),
            describe(&r, 4),
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x5DEE_CE66_D000_1234;

    pub fn new() -> Self {
        Rng(Self::SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(if s == 0 { Self::SEED } else { s })
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
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
    pub fn range(&mut self, lo: i32, hi_incl: i32) -> i32 {
        lo + self.below((hi_incl - lo + 1) as u32) as i32
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            let n = chunk.len();
            chunk.copy_from_slice(&v[..n]);
        }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 != 0
    }
}

// ---------------------------------------------------------------------------
// Bit writer — builds side-info bitstreams field by field so tests can pin
// specific configurations (window_switching, block_type, ...) exactly.
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub bytes: Vec<u8>,
    pub nbits: usize,
}

impl BitWriter {
    /// Start writing at bit offset `start_pos`; bits before it are `pad`.
    pub fn new(start_pos: usize, pad: u8) -> Self {
        let nbytes = (start_pos + 7) / 8;
        let mut bytes = vec![pad; nbytes];
        // Bits [start_pos, 8*nbytes) of the last padding byte must stay as pad;
        // they are only ever read via the mask `255 >> s`, which the callee
        // applies itself, so leaving them as `pad` is exactly right.
        if start_pos % 8 != 0 && nbytes > 0 {
            // keep pad bits; nothing to do
            let _ = &mut bytes;
        }
        BitWriter { bytes, nbits: start_pos }
    }

    pub fn put(&mut self, n: usize, val: u32) {
        assert!(n <= 32);
        for i in (0..n).rev() {
            let bit = ((val >> i) & 1) as u8;
            let idx = self.nbits / 8;
            let off = 7 - (self.nbits % 8);
            if idx >= self.bytes.len() {
                self.bytes.push(0);
            }
            self.bytes[idx] &= !(1u8 << off);
            self.bytes[idx] |= bit << off;
            self.nbits += 1;
        }
    }

    /// Finish: pad the buffer with `extra` trailing bytes so the callee's
    /// look-ahead read of the byte at `p` can never run off the allocation.
    pub fn finish(mut self, extra: usize) -> (Vec<u8>, i32) {
        let nbits = self.nbits;
        while self.bytes.len() < (nbits + 7) / 8 + extra {
            self.bytes.push(0);
        }
        (self.bytes, nbits as i32)
    }
}

/// Per-granule field values for a synthesised side-info bitstream.
#[derive(Clone, Copy, Debug)]
pub struct Granule {
    pub part_23_length: u32,
    pub big_values: u32,
    pub global_gain: u32,
    pub scalefac_compress: u32,
    pub window_switching: bool,
    pub block_type: u32,
    pub mixed_block_flag: u32,
    pub tables_short: u32,   // 10 bits, used when window_switching
    pub subblock_gain: [u32; 3],
    pub tables_long: u32,    // 15 bits, used when !window_switching
    pub region_count0: u32,  // 4 bits
    pub region_count1: u32,  // 3 bits
    pub preflag_bit: u32,    // 1 bit, MPEG1 only
    pub scalefac_scale: u32,
    pub count1_table: u32,
}

impl Default for Granule {
    fn default() -> Self {
        Granule {
            part_23_length: 0,
            big_values: 0,
            global_gain: 0,
            scalefac_compress: 0,
            window_switching: false,
            block_type: 1,
            mixed_block_flag: 0,
            tables_short: 0,
            subblock_gain: [0; 3],
            tables_long: 0,
            region_count0: 0,
            region_count1: 0,
            preflag_bit: 0,
            scalefac_scale: 0,
            count1_table: 0,
        }
    }
}

impl Granule {
    pub fn random(rng: &mut Rng) -> Self {
        Granule {
            part_23_length: rng.below(4096),
            big_values: rng.below(289),
            global_gain: rng.below(256),
            scalefac_compress: rng.below(512),
            window_switching: rng.bool(),
            block_type: 1 + rng.below(3),
            mixed_block_flag: rng.below(2),
            tables_short: rng.below(1024),
            subblock_gain: [rng.below(8), rng.below(8), rng.below(8)],
            tables_long: rng.below(32768),
            region_count0: rng.below(16),
            region_count1: rng.below(8),
            preflag_bit: rng.below(2),
            scalefac_scale: rng.below(2),
            count1_table: rng.below(2),
        }
    }
}

/// Build a side-info bitstream exactly as `read_side_info` parses it.
///
/// Returns `(buffer, consumed_bits_end_pos)` where the end position is the
/// absolute bit position after the last field (so `limit = end_pos` is the
/// "exactly enough" case).
pub fn build_side_info(
    hdr: &[u8; 4],
    start_pos: usize,
    pad: u8,
    main_data_begin: u32,
    scfsi_raw: u32,
    grs: &[Granule],
    trailing_bytes: usize,
) -> (Vec<u8>, i32) {
    let mpeg1 = (hdr[1] & 0x8) != 0;
    let mono = (hdr[3] & 0xC0) == 0xC0;
    let mut gr_count = if mono { 1 } else { 2 };
    if mpeg1 {
        gr_count *= 2;
    }
    assert!(grs.len() >= gr_count, "need {gr_count} granules, got {}", grs.len());

    let mut w = BitWriter::new(start_pos, pad);
    if mpeg1 {
        w.put(9, main_data_begin);
        w.put(7 + gr_count as usize, scfsi_raw);
    } else {
        // C reads (8+gr_count) bits then shifts right by gr_count, so the low
        // `gr_count` bits are don't-care; place main_data_begin in the high bits.
        w.put(8 + gr_count as usize, (main_data_begin << gr_count) | (scfsi_raw & ((1 << gr_count) - 1)));
    }
    for g in grs.iter().take(gr_count) {
        w.put(12, g.part_23_length);
        w.put(9, g.big_values);
        w.put(8, g.global_gain);
        w.put(if mpeg1 { 4 } else { 9 }, g.scalefac_compress);
        w.put(1, g.window_switching as u32);
        if g.window_switching {
            w.put(2, g.block_type);
            w.put(1, g.mixed_block_flag);
            w.put(10, g.tables_short);
            w.put(3, g.subblock_gain[0]);
            w.put(3, g.subblock_gain[1]);
            w.put(3, g.subblock_gain[2]);
        } else {
            w.put(15, g.tables_long);
            w.put(4, g.region_count0);
            w.put(3, g.region_count1);
        }
        if mpeg1 {
            w.put(1, g.preflag_bit);
        }
        w.put(1, g.scalefac_scale);
        w.put(1, g.count1_table);
    }
    w.finish(trailing_bytes.max(8))
}

/// Header byte builder. `sr_bits` = (hdr[2]>>2)&3, `ext` = (hdr[1]>>4)&1,
/// `chan` = hdr[3]>>6 (3 == mono).
pub fn make_hdr(mpeg1: bool, ext: u32, sr_bits: u32, chan: u32, noise: u8) -> [u8; 4] {
    let h1 = ((ext as u8 & 1) << 4) | (if mpeg1 { 0x8 } else { 0 }) | (noise & 0x07);
    let h2 = ((sr_bits as u8 & 3) << 2) | (noise & 0xF3);
    let h3 = ((chan as u8 & 3) << 6) | (noise & 0x3F);
    [noise, h1, h2, h3]
}
