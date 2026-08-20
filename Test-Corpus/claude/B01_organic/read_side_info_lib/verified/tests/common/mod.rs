//! Shared differential-test harness.
//!
//! Both the C and the Rust implementation are loaded as *shared objects* with
//! `libloading` and called only through their exported `read_side_info` symbol,
//! exactly like an external C consumer would. Nothing in the crate under test is
//! called directly.

#![allow(dead_code)]

use std::env;
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirrors of the C types (c_src/include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// `sizeof(L3_gr_info_t)` == 32 with *no* interior padding (every field offset
/// is contiguous: ptr@0, u16@8/10/12, u8@14..=18, [u8;3]@19/22/25, u8@28..=31),
/// so the whole struct can be compared byte-for-byte.
pub const GR_SIZE: usize = 32;
/// Number of `L3_gr_info_t` slots handed to the library. The C writes at most
/// `gr_count` (<= 4) of them; the extra slots detect out-of-bounds writes.
pub const N_GR: usize = 6;

// Field offsets inside L3_gr_info_t.
pub const O_SFBTAB: usize = 0;
pub const O_PART_23_LENGTH: usize = 8;
pub const O_BIG_VALUES: usize = 10;
pub const O_SCALEFAC_COMPRESS: usize = 12;
pub const O_GLOBAL_GAIN: usize = 14;
pub const O_BLOCK_TYPE: usize = 15;
pub const O_MIXED_BLOCK_FLAG: usize = 16;
pub const O_N_LONG_SFB: usize = 17;
pub const O_N_SHORT_SFB: usize = 18;
pub const O_TABLE_SELECT: usize = 19;
pub const O_REGION_COUNT: usize = 22;
pub const O_SUBBLOCK_GAIN: usize = 25;
pub const O_PREFLAG: usize = 28;
pub const O_SCALEFAC_SCALE: usize = 29;
pub const O_COUNT1_TABLE: usize = 30;
pub const O_SCFSI: usize = 31;

#[repr(C, align(8))]
pub struct GrArray(pub [u8; N_GR * GR_SIZE]);

impl GrArray {
    pub fn new(fill: &[u8]) -> Self {
        let mut a = [0u8; N_GR * GR_SIZE];
        a.copy_from_slice(fill);
        GrArray(a)
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

pub type ReadSideInfoFn = unsafe extern "C" fn(*mut BsT, *mut u8, *const u8) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = env::var("C_SO") {
        return PathBuf::from(p);
    }
    let path = manifest_dir().join("c_src/build/libtranslated_rust.so");
    if !path.is_file() {
        build_c_reference();
    }
    assert!(
        path.is_file(),
        "C reference {path:?} is missing; build it with\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    path
}

/// Build `c_src` exactly as the task specifies, once per process, if the
/// reference `.so` is not there yet. Nothing inside `c_src` is modified except
/// the (git-ignored) `build/` directory that the documented build command
/// creates itself.
fn build_c_reference() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = manifest_dir().join("c_src/build");
        std::fs::create_dir_all(&dir).expect("create c_src/build");
        for args in [
            vec!["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"],
            vec!["--build", "."],
        ] {
            let st = std::process::Command::new("cmake")
                .args(&args)
                .current_dir(&dir)
                .status()
                .unwrap_or_else(|e| panic!("failed to run cmake {args:?}: {e}"));
            assert!(st.success(), "cmake {args:?} failed");
        }
    });
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = env::current_exe().expect("current_exe");
    for anc in exe.ancestors() {
        let cand = anc.join("libread_side_info_lib.so");
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "libread_side_info_lib.so not found near {exe:?}; run `cargo build` first \
         or set RUST_SO=<path>"
    );
}

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    fun: libloading::os::unix::Symbol<ReadSideInfoFn>,
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
            let sym: libloading::Symbol<ReadSideInfoFn> = lib
                .get(b"read_side_info\0")
                .unwrap_or_else(|e| panic!("dlsym read_side_info in {path:?}: {e}"));
            let fun = sym.into_raw();
            Lib {
                name,
                _lib: lib,
                fun,
            }
        }
    }

    /// The one and only FFI entry point.
    pub unsafe fn read_side_info(
        &self,
        bs: *mut BsT,
        gr: *mut u8,
        hdr: *const u8,
    ) -> c_int {
        unsafe { (self.fun)(bs, gr, hdr) }
    }
}

pub struct Libs {
    pub c: Lib,
    pub rust: Lib,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| Libs {
        c: Lib::open("C", &c_so_path()),
        rust: Lib::open("Rust", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

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
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }
    /// `n` random bits (n <= 32).
    pub fn bits(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() >> (32 - n)
        }
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        for b in out.iter_mut() {
            *b = self.next_u32() as u8;
        }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 != 0
    }
}

// ---------------------------------------------------------------------------
// Bit writer: the exact dual of the C `get_bits` (MSB-first, bit addressed)
// ---------------------------------------------------------------------------

pub fn write_bits(buf: &mut [u8], bit_off: usize, n: u32, val: u32) {
    for k in 0..n {
        let bit = if n == 0 { 0 } else { (val >> (n - 1 - k)) & 1 };
        let idx = bit_off + k as usize;
        let byte = idx >> 3;
        let mask = 0x80u8 >> (idx & 7);
        if bit != 0 {
            buf[byte] |= mask;
        } else {
            buf[byte] &= !mask;
        }
    }
}

// ---------------------------------------------------------------------------
// Header helpers — mirror of lib.c:87..92
// ---------------------------------------------------------------------------

pub fn is_mpeg1(hdr: &[u8; 4]) -> bool {
    hdr[1] & 0x8 != 0
}
pub fn is_mono(hdr: &[u8; 4]) -> bool {
    (hdr[3] & 0xC0) == 0xC0
}
pub fn sr_idx(hdr: &[u8; 4]) -> i32 {
    let mut s = (((hdr[2] as i32) >> 2) & 3)
        + ((((hdr[1] as i32) >> 3) & 1) + (((hdr[1] as i32) >> 4) & 1)) * 3;
    s -= (s != 0) as i32;
    s
}
pub fn gr_count(hdr: &[u8; 4]) -> usize {
    let mut g = if is_mono(hdr) { 1 } else { 2 };
    if is_mpeg1(hdr) {
        g *= 2;
    }
    g
}

/// Build a header byte triple with the requested properties.
/// `srate` = value of `hdr[2]` bits 2..3, `bit4` = `hdr[1] & 0x10`.
/// All other bits are filled randomly (the C never looks at them).
pub fn make_hdr(rng: &mut Rng, mpeg1: bool, mono: bool, srate: u32, bit4: bool) -> [u8; 4] {
    let mut h = [0u8; 4];
    rng.fill(&mut h);
    h[1] = (h[1] & !0x18) | if mpeg1 { 0x08 } else { 0 } | if bit4 { 0x10 } else { 0 };
    h[2] = (h[2] & !0x0C) | ((srate & 3) << 2) as u8;
    // hdr[3] bits 6..7 == 0b11 <=> mono. Anything else <=> not mono.
    if mono {
        h[3] |= 0xC0;
    } else {
        // pick one of the three non-0xC0 patterns at random
        h[3] = (h[3] & 0x3F) | ((rng.below(3) as u8) << 6);
    }
    h
}

/// `sr_idx` is not freely choosable: bit 3 of `hdr[1]` *is* the MPEG1 flag.
/// Returns `(srate, bit4)` producing the requested `sr_idx` for the given
/// `mpeg1`, or `None` if unreachable.
pub fn hdr_bits_for_sr_idx(mpeg1: bool, want: i32) -> Option<(u32, bool)> {
    for bit4 in [false, true] {
        for srate in 0..4u32 {
            let pre =
                srate as i32 + ((mpeg1 as i32) + (bit4 as i32)) * 3;
            let got = pre - ((pre != 0) as i32);
            if got == want {
                return Some((srate, bit4));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Side-info bitstream synthesis
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct Gran {
    pub part_23_length: u32, // 12 bits
    pub big_values: u32,     // 9 bits
    pub global_gain: u32,    // 8 bits
    pub scalefac_compress: u32, // 4 bits (MPEG1) / 9 bits (MPEG2)
    pub window: u32,         // 1 bit
    // window == 1 branch
    pub block_type: u32,     // 2 bits
    pub mixed: u32,          // 1 bit
    pub tables10: u32,       // 10 bits
    pub subblock_gain: [u32; 3], // 3 bits each
    // window == 0 branch
    pub tables15: u32,       // 15 bits
    pub region_count0: u32,  // 4 bits
    pub region_count1: u32,  // 3 bits
    // tail
    pub preflag: u32,        // 1 bit, MPEG1 only
    pub scalefac_scale: u32, // 1 bit
    pub count1_table: u32,   // 1 bit
}

impl Gran {
    pub fn random(rng: &mut Rng) -> Gran {
        Gran {
            part_23_length: rng.bits(12),
            big_values: rng.below(289),
            global_gain: rng.bits(8),
            scalefac_compress: rng.bits(9),
            window: rng.bits(1),
            block_type: 1 + rng.below(3),
            mixed: rng.bits(1),
            tables10: rng.bits(10),
            subblock_gain: [rng.bits(3), rng.bits(3), rng.bits(3)],
            tables15: rng.bits(15),
            region_count0: rng.bits(4),
            region_count1: rng.bits(3),
            preflag: rng.bits(1),
            scalefac_scale: rng.bits(1),
            count1_table: rng.bits(1),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SideInfo {
    /// raw `main_data_begin` field: 9 bits (MPEG1) or `8+gr_count` bits (MPEG2)
    pub main_data_begin: u32,
    /// raw `scfsi` field: `7+gr_count` bits, MPEG1 only
    pub scfsi: u32,
    pub gr: Vec<Gran>,
}

impl SideInfo {
    pub fn random(rng: &mut Rng, hdr: &[u8; 4]) -> SideInfo {
        let g = gr_count(hdr);
        SideInfo {
            main_data_begin: rng.bits(if is_mpeg1(hdr) { 9 } else { 8 + g as u32 }),
            scfsi: rng.bits(7 + g as u32),
            gr: (0..g).map(|_| Gran::random(rng)).collect(),
        }
    }
}

/// Serialise `si` into `buf` starting at absolute bit offset `bit_off`, in the
/// exact order `read_side_info` reads it. Returns the number of bits written
/// (== the number of bits the C consumes when no early `return -1` happens).
pub fn write_side_info(buf: &mut [u8], bit_off: usize, hdr: &[u8; 4], si: &SideInfo) -> usize {
    let mpeg1 = is_mpeg1(hdr);
    let g = gr_count(hdr);
    assert_eq!(si.gr.len(), g, "SideInfo granule count must match hdr");
    let mut o = bit_off;
    let put = |buf: &mut [u8], o: &mut usize, n: u32, v: u32| {
        write_bits(buf, *o, n, v);
        *o += n as usize;
    };
    if mpeg1 {
        put(buf, &mut o, 9, si.main_data_begin);
        put(buf, &mut o, 7 + g as u32, si.scfsi);
    } else {
        put(buf, &mut o, 8 + g as u32, si.main_data_begin);
    }
    for gr in &si.gr {
        put(buf, &mut o, 12, gr.part_23_length);
        put(buf, &mut o, 9, gr.big_values);
        put(buf, &mut o, 8, gr.global_gain);
        put(buf, &mut o, if mpeg1 { 4 } else { 9 }, gr.scalefac_compress);
        put(buf, &mut o, 1, gr.window);
        if gr.window != 0 {
            put(buf, &mut o, 2, gr.block_type);
            put(buf, &mut o, 1, gr.mixed);
            put(buf, &mut o, 10, gr.tables10);
            put(buf, &mut o, 3, gr.subblock_gain[0]);
            put(buf, &mut o, 3, gr.subblock_gain[1]);
            put(buf, &mut o, 3, gr.subblock_gain[2]);
        } else {
            put(buf, &mut o, 15, gr.tables15);
            put(buf, &mut o, 4, gr.region_count0);
            put(buf, &mut o, 3, gr.region_count1);
        }
        if mpeg1 {
            put(buf, &mut o, 1, gr.preflag);
        }
        put(buf, &mut o, 1, gr.scalefac_scale);
        put(buf, &mut o, 1, gr.count1_table);
    }
    o - bit_off
}

// ---------------------------------------------------------------------------
// Test-case plumbing
// ---------------------------------------------------------------------------

/// Backing buffer size. `bs->buf` points at `BUF_MID` so that negative
/// `bs->pos` values are still inside the allocation.
pub const BUF_LEN: usize = 512;
pub const BUF_MID: usize = 128;
pub const AMPLE_LIMIT: c_int = 1 << 20;

#[derive(Clone)]
pub struct Case {
    /// The whole backing buffer; `bs->buf == &buf[BUF_MID]`.
    pub buf: Vec<u8>,
    pub pos: c_int,
    pub limit: c_int,
    pub hdr: [u8; 4],
    /// Pre-fill of the `L3_gr_info_t` array (N_GR * 32 bytes).
    pub fill: Vec<u8>,
    /// `bs->buf` = null instead of `&buf[BUF_MID]`.
    pub null_buf: bool,
}

impl Case {
    pub fn new(rng: &mut Rng, hdr: [u8; 4]) -> Case {
        let mut buf = vec![0u8; BUF_LEN];
        rng.fill(&mut buf);
        let mut fill = vec![0u8; N_GR * GR_SIZE];
        rng.fill(&mut fill);
        // Make the sentinel `sfbtab` non-canonical so it can never be mistaken
        // for a real table pointer.
        for i in 0..N_GR {
            fill[i * GR_SIZE + 7] = 0xEE;
        }
        Case {
            buf,
            pos: 0,
            // "Ample" limit: far beyond anything the side info consumes, so no
            // `get_bits` truncation happens. `bs->limit` is only ever compared
            // against `bs->pos`, never used to bound a memory access, so a huge
            // value is safe (the C reads at most ~34 bytes from `bs->buf`).
            limit: AMPLE_LIMIT,
            hdr,
            fill,
            null_buf: false,
        }
    }

    /// Absolute bit offset in `buf` corresponding to `bs->pos`.
    pub fn bit_off(&self) -> usize {
        (BUF_MID as isize * 8 + self.pos as isize) as usize
    }

    pub fn put_side_info(&mut self, si: &SideInfo) -> usize {
        let off = self.bit_off();
        let hdr = self.hdr;
        write_side_info(&mut self.buf, off, &hdr, si)
    }
}

pub struct Outcome {
    pub ret: c_int,
    pub bs: BsT,
    pub gr: Vec<u8>,
    /// `sfbtab` of each granule, as raw integers.
    pub ptrs: Vec<u64>,
}

pub fn run(lib: &Lib, case: &Case) -> Outcome {
    // Fresh copy of the buffer for every call so that neither library can
    // influence the other (the C never writes to it, but be strict).
    let buf = case.buf.clone();
    let mut gr = GrArray::new(&case.fill);
    let mut bs = BsT {
        buf: if case.null_buf {
            std::ptr::null()
        } else {
            unsafe { buf.as_ptr().add(BUF_MID) }
        },
        pos: case.pos,
        limit: case.limit,
    };
    let ret = unsafe { lib.read_side_info(&mut bs, gr.as_mut_ptr(), case.hdr.as_ptr()) };
    let bytes = gr.0.to_vec();
    let ptrs = (0..N_GR)
        .map(|i| {
            let mut b = [0u8; 8];
            b.copy_from_slice(&bytes[i * GR_SIZE..i * GR_SIZE + 8]);
            u64::from_ne_bytes(b)
        })
        .collect();
    drop(buf);
    Outcome {
        ret,
        bs,
        gr: bytes,
        ptrs,
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" ")
}

/// Compare the two outcomes byte-for-byte.
///
/// * return value, `bs->pos`, `bs->limit`
/// * all 24 non-pointer bytes of every one of the `N_GR` `L3_gr_info_t` slots
/// * the `sfbtab` pointer: untouched slots must still hold the sentinel; touched
///   slots must have identical *relative* offsets between granules and must
///   point at identical table bytes.
pub fn compare(case: &Case, c: &Outcome, r: &Outcome, ctx: &str) {
    assert_eq!(
        c.ret, r.ret,
        "{ctx}: return value differs (C={} Rust={})\n  hdr={:02x?} pos={} limit={}",
        c.ret, r.ret, case.hdr, case.pos, case.limit
    );
    assert_eq!(
        c.bs.pos, r.bs.pos,
        "{ctx}: bs->pos differs (C={} Rust={})",
        c.bs.pos, r.bs.pos
    );
    assert_eq!(
        c.bs.limit, r.bs.limit,
        "{ctx}: bs->limit differs (C={} Rust={})",
        c.bs.limit, r.bs.limit
    );

    for i in 0..N_GR {
        let a = &c.gr[i * GR_SIZE + 8..(i + 1) * GR_SIZE];
        let b = &r.gr[i * GR_SIZE + 8..(i + 1) * GR_SIZE];
        assert_eq!(
            a,
            b,
            "{ctx}: gr[{i}] bytes 8..32 differ\n  C   ={}\n  Rust={}\n  hdr={:02x?} pos={} limit={} ret={}",
            hex(a),
            hex(b),
            case.hdr,
            case.pos,
            case.limit,
            c.ret
        );
    }

    // sfbtab pointers
    let mut sentinel = [0u8; 8];
    let mut touched: Vec<usize> = Vec::new();
    for i in 0..N_GR {
        sentinel.copy_from_slice(&case.fill[i * GR_SIZE..i * GR_SIZE + 8]);
        let s = u64::from_ne_bytes(sentinel);
        let cu = c.ptrs[i];
        let ru = r.ptrs[i];
        if cu == s || ru == s {
            assert_eq!(
                cu, ru,
                "{ctx}: gr[{i}].sfbtab: one side left the sentinel, the other did not \
                 (C={cu:#x} Rust={ru:#x} sentinel={s:#x})"
            );
        } else {
            touched.push(i);
        }
    }
    if let Some(&base) = touched.first() {
        for &i in &touched {
            let dc = c.ptrs[i].wrapping_sub(c.ptrs[base]);
            let dr = r.ptrs[i].wrapping_sub(r.ptrs[base]);
            assert_eq!(
                dc, dr,
                "{ctx}: gr[{i}].sfbtab offset relative to gr[{base}] differs \
                 (C={dc} Rust={dr})"
            );
        }
    }

    // Contents of the selected scalefactor-band rows.
    let sri = sr_idx(&case.hdr);
    for &i in &touched {
        let n_short = c.gr[i * GR_SIZE + O_N_SHORT_SFB];
        let block_type = c.gr[i * GR_SIZE + O_BLOCK_TYPE];
        let mixed = c.gr[i * GR_SIZE + O_MIXED_BLOCK_FLAG];
        // `&g_scf_mixed[8]` reads past the end of the C object's .rodata, into
        // build-specific unwind data (see ERRORS.md row 16) — unreproducible by
        // construction, so only that one read is exempt from the byte compare.
        if sri == 8 && block_type == 2 && mixed != 0 {
            continue;
        }
        let len = if n_short == 0 { 23usize } else { 40usize };
        let cb = unsafe { std::slice::from_raw_parts(c.ptrs[i] as *const u8, len) };
        let rb = unsafe { std::slice::from_raw_parts(r.ptrs[i] as *const u8, len) };
        assert_eq!(
            cb,
            rb,
            "{ctx}: gr[{i}].sfbtab contents differ (sr_idx={sri} block_type={block_type} \
             mixed={mixed} len={len})\n  C   ={}\n  Rust={}",
            hex(cb),
            hex(rb)
        );
    }
}

/// Run one case through both libraries and compare.
pub fn diff(case: &Case, ctx: &str) -> Outcome {
    let l = libs();
    let c = run(&l.c, case);
    let r = run(&l.rust, case);
    compare(case, &c, &r, ctx);
    c
}

/// Read a `uint16_t` field out of granule `i` of a raw `L3_gr_info_t` array.
pub fn gr_u16(gr: &[u8], i: usize, off: usize) -> u16 {
    let mut b = [0u8; 2];
    b.copy_from_slice(&gr[i * GR_SIZE + off..i * GR_SIZE + off + 2]);
    u16::from_ne_bytes(b)
}

/// Number of bits the header part of the side info occupies (`lib.c:91..97`).
pub fn header_bits(hdr: &[u8; 4]) -> usize {
    let g = gr_count(hdr);
    if is_mpeg1(hdr) {
        9 + 7 + g
    } else {
        8 + g
    }
}

/// Bit offset (relative to `bs->pos`) of granule `k`'s 2-bit `block_type`
/// field, assuming every preceding granule took the window-switching branch
/// (`window == 1`).
pub fn block_type_bit_offset_all_w1(hdr: &[u8; 4], k: usize) -> usize {
    let mpeg1 = is_mpeg1(hdr);
    let sfc = if mpeg1 { 4 } else { 9 };
    let prefix = 12 + 9 + 8 + sfc + 1;
    let whole_granule = prefix + (2 + 1 + 10 + 9) + (mpeg1 as usize) + 2;
    header_bits(hdr) + k * whole_granule + prefix
}

/// The value the C computes for `main_data_begin` (`lib.c:93` / `lib.c:96`).
pub fn main_data_begin_value(hdr: &[u8; 4], si: &SideInfo) -> i32 {
    if is_mpeg1(hdr) {
        si.main_data_begin as i32
    } else {
        (si.main_data_begin >> gr_count(hdr)) as i32
    }
}

pub fn set_window(si: &mut SideInfo, w: u32) {
    for g in si.gr.iter_mut() {
        g.window = w;
    }
}

/// Ask the C library where `bs->pos` ends up (used to build exact-boundary
/// limits without re-implementing the bit accounting).
pub fn probe_pos_end(case: &Case) -> c_int {
    run(&libs().c, case).bs.pos
}

/// Build a case with a synthesised side-info bitstream.
pub fn build(
    rng: &mut Rng,
    hdr: [u8; 4],
    f: impl FnOnce(&mut Rng, &mut SideInfo),
) -> (Case, SideInfo) {
    let mut si = SideInfo::random(rng, &hdr);
    f(rng, &mut si);
    let mut case = Case::new(rng, hdr);
    case.put_side_info(&si);
    (case, si)
}

/// Success/error tally, so a row cannot silently degenerate into "everything
/// returned -1 immediately".
#[derive(Default, Debug)]
pub struct Stats {
    pub ok: u32,
    pub err: u32,
}

impl Stats {
    pub fn add(&mut self, ret: c_int) {
        if ret < 0 {
            self.err += 1;
        } else {
            self.ok += 1;
        }
    }
    pub fn require_some_ok(&self, ctx: &str) {
        assert!(
            self.ok > 0,
            "{ctx}: every iteration returned an error ({self:?}) — the row would \
             not be exercising the happy path"
        );
    }
    pub fn require_some_err(&self, ctx: &str) {
        assert!(
            self.err > 0,
            "{ctx}: no iteration produced an error ({self:?})"
        );
    }
}
