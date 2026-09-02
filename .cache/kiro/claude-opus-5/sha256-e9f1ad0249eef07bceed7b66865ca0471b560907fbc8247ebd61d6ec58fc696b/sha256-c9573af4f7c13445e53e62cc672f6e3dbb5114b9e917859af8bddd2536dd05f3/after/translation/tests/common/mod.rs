//! Shared differential-testing harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and calls
//! `read_side_info` through the FFI boundary in each. The Rust implementation is
//! never called directly — only via its exported `#[no_mangle]` symbol, exactly
//! as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// FFI types (must mirror c_src/include/lib.h byte-for-byte)
// ---------------------------------------------------------------------------

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

pub const GR_SLOTS: usize = 8;
pub const GR_SIZE: usize = 32;

const _: () = {
    assert!(size_of::<BsT>() == 16);
    assert!(size_of::<GrInfo>() == GR_SIZE);
};

pub type ReadSideInfoFn = unsafe extern "C" fn(*mut BsT, *mut GrInfo, *const u8) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub read_side_info: ReadSideInfoFn,
    /// Base addresses of `g_scf_long`, `g_scf_short`, `g_scf_mixed` inside this
    /// shared object, recovered by calibration (see `calibrate`).
    pub table_bases: [usize; 3],
}

pub const T_LONG: usize = 0;
pub const T_SHORT: usize = 1;
pub const T_MIXED: usize = 2;
pub const ROW_SIZE: [usize; 3] = [23, 40, 40];
pub const TABLE_NAME: [&str; 3] = ["g_scf_long", "g_scf_short", "g_scf_mixed"];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    // Allow pointing the harness at an alternative C build (e.g. -O2) without
    // touching c_src/.
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HARVEST_C_SO={} is not a file", p.display());
        return p;
    }
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let build = root.join("c_src").join("build");
    let mut found: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HARVEST_RUST_SO={} is not a file", p.display());
        return p;
    }
    let base = manifest_dir().join("target");
    // Load the cdylib built with the SAME profile as this test binary, so that
    // `cargo test` and `cargo test --release` each verify their own artifact.
    let order: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in order {
        let p = base.join(profile).join("libread_side_info_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}. Build it with: cargo build --release",
        base.display()
    )
}

unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    let f: Symbol<ReadSideInfoFn> = unsafe { lib.get(b"read_side_info\0") }
        .unwrap_or_else(|e| panic!("symbol read_side_info missing from {}: {e}", path.display()));
    let read_side_info = *f;
    drop(f);
    let mut im = Impl {
        name,
        _lib: lib,
        read_side_info,
        table_bases: [0; 3],
    };
    im.table_bases = unsafe { calibrate(&im) };
    im
}

/// The two implementations, loaded once per test process.
pub fn impls() -> &'static (Impl, Impl) {
    use std::sync::OnceLock;
    static ONCE: OnceLock<(Impl, Impl)> = OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        let c = load("C", &find_c_so());
        let r = load("Rust", &find_rust_so());
        (c, r)
    })
}

// ---------------------------------------------------------------------------
// Bit writer (MSB-first, matching `get_bits`)
// ---------------------------------------------------------------------------

pub const BUF_BYTES: usize = 512;

pub struct Bw {
    pub buf: Vec<u8>,
    pub pos: usize,
}

impl Bw {
    pub fn new(start_bit: usize, rng: &mut Pcg32) -> Self {
        let mut buf = vec![0u8; BUF_BYTES];
        for b in buf.iter_mut() {
            *b = rng.next_u32() as u8;
        }
        Bw {
            buf,
            pos: start_bit,
        }
    }

    pub fn put(&mut self, val: u32, n: u32) {
        for i in (0..n).rev() {
            let bit = (val >> i) & 1;
            let byte = self.pos >> 3;
            let sh = 7 - (self.pos & 7);
            self.buf[byte] &= !(1u8 << sh);
            self.buf[byte] |= (bit as u8) << sh;
            self.pos += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (PCG32)
// ---------------------------------------------------------------------------

pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let mut p = Pcg32 {
            state: 0,
            inc: (seed << 1) | 1,
        };
        p.next_u32();
        p.state = p.state.wrapping_add(seed);
        p.next_u32();
        p
    }
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.next_u32() % n
    }
    pub fn range_i32(&mut self, lo: i32, hi_inclusive: i32) -> i32 {
        lo + self.below((hi_inclusive - lo + 1) as u32) as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Granule block configuration
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blk {
    /// window_switching == 0 (long block)
    L,
    /// window_switching == 1, block_type == 1
    S1,
    /// window_switching == 1, block_type == 2, mixed_block_flag == 0
    S2M0,
    /// window_switching == 1, block_type == 2, mixed_block_flag == 1
    S2M1,
    /// window_switching == 1, block_type == 3
    S3,
    /// window_switching == 1, block_type == 0 (error path E4)
    S0,
}

impl Blk {
    pub fn all_valid() -> [Blk; 5] {
        [Blk::L, Blk::S1, Blk::S2M0, Blk::S2M1, Blk::S3]
    }
    pub fn rand(rng: &mut Pcg32) -> Blk {
        Blk::all_valid()[rng.below(5) as usize]
    }
}

/// What to force `scalefac_compress` (non-EXT) / `preflag` (EXT) to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreMode {
    Any,
    /// non-EXT: scalefac_compress >= 500. EXT: preflag bit = 1.
    On,
    /// non-EXT: scalefac_compress < 500. EXT: preflag bit = 0.
    Off,
}

// ---------------------------------------------------------------------------
// Input construction
// ---------------------------------------------------------------------------

/// A fully materialised differential test input.
pub struct Input {
    pub hdr: [u8; 4],
    pub buf: Vec<u8>,
    pub pos: c_int,
    pub limit: c_int,
    /// Human-readable description used in assertion messages.
    pub desc: String,
}

pub fn hdr_for(ext: bool, mono: bool, sr_idx: i32, rng: &mut Pcg32) -> [u8; 4] {
    // sr_idx = base + (b3+b4)*3, then -= (sum != 0); b3 == ext bit.
    let b3 = if ext { 1 } else { 0 };
    // Choose b4 and base such that the target sr_idx is produced.
    let mut candidates: Vec<(u32, u32)> = Vec::new();
    for b4 in 0..2u32 {
        for base in 0..4u32 {
            let sum = base as i32 + ((b3 + b4 as i32) * 3);
            let got = sum - if sum != 0 { 1 } else { 0 };
            if got == sr_idx {
                candidates.push((b4, base));
            }
        }
    }
    assert!(
        !candidates.is_empty(),
        "sr_idx {sr_idx} unreachable with ext={ext}"
    );
    let (b4, base) = candidates[rng.below(candidates.len() as u32) as usize];

    let mut hdr = [0u8; 4];
    hdr[0] = rng.next_u32() as u8; // never read by the C
    // hdr[1]: bit3 = b3 (== the 0x8 EXT flag), bit4 = b4, other bits random.
    let mut h1 = rng.next_u32() as u8;
    h1 = (h1 & !0x18) | ((b3 as u8) << 3) | ((b4 as u8) << 4);
    hdr[1] = h1;
    // hdr[2]: bits 2..3 = base, others random.
    let mut h2 = rng.next_u32() as u8;
    h2 = (h2 & !0x0C) | ((base as u8) << 2);
    hdr[2] = h2;
    // hdr[3]: bits 6..7 == 0b11 for mono; anything else for stereo.
    let mut h3 = rng.next_u32() as u8;
    if mono {
        h3 |= 0xC0;
    } else {
        // force NOT both bits set
        let pick = rng.below(3);
        h3 &= 0x3F;
        h3 |= match pick {
            0 => 0x00,
            1 => 0x40,
            _ => 0x80,
        };
    }
    hdr[3] = h3;
    hdr
}

pub fn gr_count_of(hdr: &[u8; 4]) -> i32 {
    let mono = (hdr[3] & 0xC0) == 0xC0;
    let mut n = if mono { 1 } else { 2 };
    if hdr[1] & 0x8 != 0 {
        n *= 2;
    }
    n
}

pub fn sr_idx_of(hdr: &[u8; 4]) -> i32 {
    let h1 = hdr[1] as i32;
    let h2 = hdr[2] as i32;
    let sum = ((h2 >> 2) & 3) + (((h1 >> 3) & 1) + ((h1 >> 4) & 1)) * 3;
    sum - if sum != 0 { 1 } else { 0 }
}

/// Which of the three scalefactor tables the C selects for a granule.
pub fn table_of(blk: Blk) -> usize {
    match blk {
        Blk::S2M0 => T_SHORT,
        Blk::S2M1 => T_MIXED,
        _ => T_LONG,
    }
}

pub struct BuildOpts {
    pub blocks: Vec<Blk>,
    pub pre: PreMode,
    /// `Some(v)` forces main_data_begin, `None` randomises it.
    pub main_data_begin: Option<u32>,
    /// `Some(v)` forces the scfsi field, `None` randomises it.
    pub scfsi: Option<u32>,
    /// `Some(v)` forces big_values for every granule, else random in 0..=288.
    pub big_values: Option<u32>,
    /// `Some(v)` forces part_23_length for every granule.
    pub part_23_length: Option<u32>,
    /// limit selection: `Ample` = pos + written + slack, `Exact` = tight fit,
    /// `Bits(n)` = literal value.
    pub limit: LimitMode,
    pub start_bit: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum LimitMode {
    Ample,
    /// Make `part_23_sum + pos == limit + main_data_begin*8` exactly.
    ExactBoundary,
    /// Make `part_23_sum + pos == limit + main_data_begin*8 + 1` (E5 fires).
    OneOverBoundary,
    Literal(c_int),
}

impl Default for BuildOpts {
    fn default() -> Self {
        BuildOpts {
            blocks: vec![Blk::L],
            pre: PreMode::Any,
            main_data_begin: None,
            scfsi: None,
            big_values: None,
            part_23_length: None,
            limit: LimitMode::Ample,
            start_bit: 0,
        }
    }
}

/// Encode a side-info bitstream exactly as `read_side_info` decodes it.
pub fn build(hdr: [u8; 4], opts: &BuildOpts, rng: &mut Pcg32) -> Input {
    let ext = hdr[1] & 0x8 != 0;
    let gr_count = gr_count_of(&hdr);
    let mut bw = Bw::new(opts.start_bit, rng);

    let mdb_bits = if ext { 9 } else { 8 + gr_count as u32 };
    let mdb_max = (1u32 << if ext { 9 } else { 8 + gr_count as u32 - gr_count as u32 }) - 1;
    let _ = mdb_max;

    let main_data_begin: u32;
    if ext {
        let mdb = opts.main_data_begin.unwrap_or_else(|| rng.below(512));
        assert!(mdb < 512);
        bw.put(mdb, 9);
        main_data_begin = mdb;
        let scfsi_bits = 7 + gr_count as u32;
        let scfsi = opts
            .scfsi
            .unwrap_or_else(|| rng.below(1u32 << scfsi_bits))
            & ((1u32 << scfsi_bits) - 1);
        bw.put(scfsi, scfsi_bits);
    } else {
        // read (8+gr_count) bits then >> gr_count
        let hi_bits = 8u32; // main_data_begin occupies the top 8 bits
        let mdb = opts.main_data_begin.unwrap_or_else(|| rng.below(1 << hi_bits));
        assert!(mdb < (1 << hi_bits));
        let low = opts.scfsi.unwrap_or_else(|| rng.next_u32()) & ((1u32 << gr_count) - 1);
        bw.put((mdb << gr_count) | low, mdb_bits);
        main_data_begin = mdb;
    }

    let mut part_23_sum: u32 = 0;
    for g in 0..gr_count as usize {
        let blk = opts.blocks[g % opts.blocks.len()];

        let p23 = opts.part_23_length.unwrap_or_else(|| rng.below(4096));
        bw.put(p23, 12);
        part_23_sum += p23;

        let bv = opts.big_values.unwrap_or_else(|| rng.below(289));
        bw.put(bv, 9);
        if bv > 288 {
            // Error path E3: the C returns before reading anything else.
            break;
        }

        bw.put(rng.below(256), 8); // global_gain

        if ext {
            bw.put(rng.below(16), 4); // scalefac_compress (4 bits)
        } else {
            let sc = match opts.pre {
                PreMode::Any => rng.below(512),
                PreMode::On => 500 + rng.below(12),  // 500..=511
                PreMode::Off => rng.below(500),      // 0..=499
            };
            bw.put(sc, 9);
        }

        match blk {
            Blk::L => {
                bw.put(0, 1); // window_switching = 0
                bw.put(rng.below(1 << 15), 15); // tables
                bw.put(rng.below(16), 4); // region_count[0]
                bw.put(rng.below(8), 3); // region_count[1]
            }
            _ => {
                bw.put(1, 1); // window_switching = 1
                let (bt, mixed): (u32, Option<u32>) = match blk {
                    Blk::S0 => (0, None),
                    Blk::S1 => (1, Some(rng.below(2))),
                    Blk::S2M0 => (2, Some(0)),
                    Blk::S2M1 => (2, Some(1)),
                    Blk::S3 => (3, Some(rng.below(2))),
                    Blk::L => unreachable!(),
                };
                bw.put(bt, 2);
                match mixed {
                    None => break, // Error path E4: C returns right here.
                    Some(m) => bw.put(m, 1),
                }
                bw.put(rng.below(1 << 10), 10); // tables
                bw.put(rng.below(8), 3); // subblock_gain[0]
                bw.put(rng.below(8), 3); // subblock_gain[1]
                bw.put(rng.below(8), 3); // subblock_gain[2]
            }
        }

        if ext {
            let pf = match opts.pre {
                PreMode::Any => rng.below(2),
                PreMode::On => 1,
                PreMode::Off => 0,
            };
            bw.put(pf, 1);
        }
        bw.put(rng.below(2), 1); // scalefac_scale
        bw.put(rng.below(2), 1); // count1_table
    }

    let end_bit = bw.pos as c_int;
    let limit = match opts.limit {
        LimitMode::Ample => {
            // Enough for every read plus the final reservoir check.
            let need = end_bit + part_23_sum as c_int + 64;
            need
        }
        LimitMode::ExactBoundary => {
            // part_23_sum + pos == limit + mdb*8  =>  limit = sum + pos - mdb*8
            let l = part_23_sum as c_int + end_bit - (main_data_begin as c_int) * 8;
            // must still be >= end_bit so no get_bits was rejected
            l.max(end_bit)
        }
        LimitMode::OneOverBoundary => {
            let l = part_23_sum as c_int + end_bit - (main_data_begin as c_int) * 8 - 1;
            l.max(end_bit)
        }
        LimitMode::Literal(v) => v,
    };

    let desc = format!(
        "hdr={:02x?} ext={} mono={} sr_idx={} gr_count={} blocks={:?} pre={:?} start_bit={} limit={} mdb={} p23sum={}",
        hdr,
        ext,
        (hdr[3] & 0xC0) == 0xC0,
        sr_idx_of(&hdr),
        gr_count,
        opts.blocks,
        opts.pre,
        opts.start_bit,
        limit,
        main_data_begin,
        part_23_sum
    );

    Input {
        hdr,
        buf: bw.buf,
        pos: opts.start_bit as c_int,
        limit,
        desc,
    }
}

// ---------------------------------------------------------------------------
// Calibration: recover the three table base addresses in each `.so`
// ---------------------------------------------------------------------------

/// Runs three fixed inputs that force `sfbtab` to `&g_scf_{long,short,mixed}[0]`
/// with `sr_idx == 0`, which reveals each table's base address in that library.
unsafe fn calibrate(im: &Impl) -> [usize; 3] {
    let mut rng = Pcg32::new(1);
    let mut out = [0usize; 3];
    for (slot, blk) in [(T_LONG, Blk::L), (T_SHORT, Blk::S2M0), (T_MIXED, Blk::S2M1)] {
        // sr_idx == 0 requires ext=false (b3=0), b4=0, base in {0,1}.
        let hdr = hdr_for(false, true, 0, &mut rng);
        assert_eq!(sr_idx_of(&hdr), 0);
        let opts = BuildOpts {
            blocks: vec![blk],
            ..Default::default()
        };
        let input = build(hdr, &opts, &mut rng);
        let (grs, _bs, _rv) = unsafe { run(im, &input) };
        let p = grs[0].sfbtab as usize;
        assert!(p != 0, "calibration produced a null sfbtab for {:?}", blk);
        out[slot] = p;
    }
    // Sanity: three distinct tables.
    assert!(
        out[0] != out[1] && out[1] != out[2] && out[0] != out[2],
        "calibration collision in {}: {:x?}",
        im.name,
        out
    );
    out
}

// ---------------------------------------------------------------------------
// Invocation + comparison
// ---------------------------------------------------------------------------

/// Sentinel used to pre-fill the granule array so untouched bytes are
/// comparable across the two implementations.
const GR_FILL: u8 = 0xA5;

pub unsafe fn run(im: &Impl, input: &Input) -> (Vec<GrInfo>, BsT, c_int) {
    let mut grs_bytes = vec![GR_FILL; GR_SLOTS * GR_SIZE];
    // sfbtab of every slot starts as null so an untouched slot is unambiguous.
    for g in 0..GR_SLOTS {
        grs_bytes[g * GR_SIZE..g * GR_SIZE + 8].fill(0);
    }
    let buf = input.buf.clone();
    let mut bs = BsT {
        buf: buf.as_ptr(),
        pos: input.pos,
        limit: input.limit,
    };
    let rv = unsafe {
        (im.read_side_info)(
            &mut bs,
            grs_bytes.as_mut_ptr() as *mut GrInfo,
            input.hdr.as_ptr(),
        )
    };
    let mut grs = Vec::with_capacity(GR_SLOTS);
    for g in 0..GR_SLOTS {
        let mut gi = std::mem::MaybeUninit::<GrInfo>::uninit();
        unsafe {
            std::ptr::copy_nonoverlapping(
                grs_bytes.as_ptr().add(g * GR_SIZE),
                gi.as_mut_ptr() as *mut u8,
                GR_SIZE,
            );
            grs.push(gi.assume_init());
        }
    }
    // `bs.buf` points at our local copy; normalise it so comparisons only look
    // at pos/limit (the C never modifies buf anyway).
    bs.buf = std::ptr::null();
    drop(buf);
    (grs, bs, rv)
}

fn gr_bytes(g: &GrInfo) -> [u8; GR_SIZE] {
    let mut out = [0u8; GR_SIZE];
    unsafe {
        std::ptr::copy_nonoverlapping((g as *const GrInfo) as *const u8, out.as_mut_ptr(), GR_SIZE);
    }
    out
}

/// Classify which table `sfbtab` points into, using only observable output.
fn table_from_output(g: &GrInfo) -> usize {
    if g.n_short_sfb == 0 {
        T_LONG
    } else if g.n_long_sfb == 0 && g.n_short_sfb == 39 {
        T_SHORT
    } else {
        T_MIXED
    }
}

/// Run one input through both `.so`s and assert byte-identical results.
pub fn compare(row: &str, input: &Input) {
    let (c, r) = impls();
    let (cg, cbs, crv) = unsafe { run(c, input) };
    let (rg, rbs, rrv) = unsafe { run(r, input) };

    assert_eq!(
        crv, rrv,
        "[{row}] return value mismatch: C={crv} Rust={rrv}\n  {}",
        input.desc
    );
    assert_eq!(
        cbs.pos, rbs.pos,
        "[{row}] bs.pos mismatch: C={} Rust={}\n  {}",
        cbs.pos, rbs.pos, input.desc
    );
    assert_eq!(
        cbs.limit, rbs.limit,
        "[{row}] bs.limit mismatch: C={} Rust={}\n  {}",
        cbs.limit, rbs.limit, input.desc
    );

    let sr_idx = sr_idx_of(&input.hdr);

    for g in 0..GR_SLOTS {
        let cb = gr_bytes(&cg[g]);
        let rb = gr_bytes(&rg[g]);
        // Bytes 8..32 must be bit-identical.
        assert_eq!(
            &cb[8..],
            &rb[8..],
            "[{row}] granule {g} payload mismatch\n  C   = {:02x?}\n  Rust= {:02x?}\n  {}",
            &cb[8..],
            &rb[8..],
            input.desc
        );

        // sfbtab: pointers necessarily differ between the two shared objects,
        // so compare the *normalised* (table, byte offset) pair instead.
        let cp = cg[g].sfbtab as usize;
        let rp = rg[g].sfbtab as usize;
        if cp == 0 || rp == 0 {
            assert_eq!(
                cp == 0,
                rp == 0,
                "[{row}] granule {g}: one sfbtab is null and the other is not (C={cp:#x} Rust={rp:#x})\n  {}",
                input.desc
            );
            continue;
        }
        let ct = table_from_output(&cg[g]);
        let rt = table_from_output(&rg[g]);
        assert_eq!(ct, rt, "[{row}] granule {g}: table classification differs");
        let coff = cp.wrapping_sub(c.table_bases[ct]);
        let roff = rp.wrapping_sub(r.table_bases[rt]);
        assert_eq!(
            coff, roff,
            "[{row}] granule {g}: sfbtab offset into {} differs: C=+{} Rust=+{}\n  {}",
            TABLE_NAME[ct], coff as isize, roff as isize, input.desc
        );
        assert_eq!(
            coff,
            sr_idx as usize * ROW_SIZE[ct],
            "[{row}] granule {g}: sfbtab offset into {} is +{} but sr_idx*{} = {}\n  {}",
            TABLE_NAME[ct],
            coff,
            ROW_SIZE[ct],
            sr_idx as usize * ROW_SIZE[ct],
            input.desc
        );

        // For in-range sr_idx the pointed-to row must be byte-identical, which
        // validates the transcribed table literals themselves.
        if (0..8).contains(&sr_idx) {
            let n = ROW_SIZE[ct];
            let crow = unsafe { std::slice::from_raw_parts(cp as *const u8, n) };
            let rrow = unsafe { std::slice::from_raw_parts(rp as *const u8, n) };
            assert_eq!(
                crow, rrow,
                "[{row}] granule {g}: {}[{sr_idx}] contents differ\n  C   = {:?}\n  Rust= {:?}\n  {}",
                TABLE_NAME[ct], crow, rrow, input.desc
            );
        }
    }
}

/// Convenience: build + compare with a fresh description-tagged row id.
pub fn check(row: &str, hdr: [u8; 4], opts: &BuildOpts, rng: &mut Pcg32) {
    let input = build(hdr, opts, rng);
    compare(row, &input);
}
