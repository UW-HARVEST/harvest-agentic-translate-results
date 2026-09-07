//! Differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls
//! `dequantize_granule` only through its exported symbol -- never the Rust
//! function directly -- so the `#[no_mangle] extern "C"` wrapper and the ABI
//! of the two structs are exercised exactly as an external C caller would.
//!
//! Every call runs in a `fork()`ed child on `MAP_SHARED` anonymous memory.
//! That is necessary, not paranoia: the C code indexes `sci->bitalloc[i]` for
//! `i` up to `2*255-1 = 509` on a 64-byte array, walks `grbuf` up to ~5.4k
//! floats past its base, and can advance `bs->pos` by up to 1_879_048_195 bits
//! per `get_bits` call (which overflows `int` after two calls and then makes
//! the C dereference `buf + 234881024`). Those inputs are real -- the C accepts
//! them -- and some of them crash. Forking lets us compare the *crash status*
//! of the two libraries as well as their memory effects, instead of taking the
//! whole test runner down.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI mirror of the C types (c_src/include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// `sizeof(L12_scale_info)` as reported by the C compiler on this target.
pub const SCI_SIZE: usize = 900;
/// `offsetof(L12_scale_info, total_bands)`
pub const OFF_TOTAL_BANDS: usize = 768;
/// `offsetof(L12_scale_info, stereo_bands)`
pub const OFF_STEREO_BANDS: usize = 769;
/// `offsetof(L12_scale_info, bitalloc)`
pub const OFF_BITALLOC: usize = 770;
/// `offsetof(L12_scale_info, scfcod)`
pub const OFF_SCFCOD: usize = 834;

/// The C reads `bitalloc[i]` for `i < 2*total_bands`, i.e. up to `i == 509`,
/// which is byte `770 + 509 == 1279` of the object. We hand both libraries a
/// region that is deliberately larger than the struct and filled with known
/// bytes, so the out-of-bounds read is *deterministic and identical* for both.
pub const SCI_REGION: usize = 2048;

/// `dst` reaches at most `4*group_size + 576 + 18*254` floats past `grbuf`.
/// A negative `group_size` makes `dst` start `3*|group_size|` floats *before*
/// `grbuf`, so we also reserve a prefix and hand out a pointer into the middle.
pub const GR_PREFIX: usize = 512;
pub const GR_MAIN: usize = 16384;
pub const GR_TOTAL: usize = GR_PREFIX + GR_MAIN;

pub type DequantFn = unsafe extern "C" fn(*mut f32, *mut BsT, *mut u8, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so`; its name is derived from the parent directory name by
/// `c_src/CMakeLists.txt`, so we glob rather than hard-code it.
pub fn c_so_path() -> PathBuf {
    // `C_SO` lets the same suite be pointed at a C build made with different
    // optimisation settings, to confirm the Rust matches regardless of how the
    // C's undefined-behaviour constructs happen to be folded.
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one C .so in {}, got {found:?}", dir.display());
    found.pop().unwrap()
}

/// The Rust `cdylib`. Release is preferred (that is the artifact the crate
/// ships); `RUST_SO` overrides so the same tests can be pointed at a `.so`
/// built with a different feature set.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdequantize_granule_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust .so found under {}; run `cargo build --release`", base.display());
}

pub struct Libs {
    _c: libloading::Library,
    _r: libloading::Library,
    pub c: DequantFn,
    pub r: DequantFn,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c_lib = libloading::Library::new(c_so_path()).expect("dlopen C .so");
            let r_lib = libloading::Library::new(rust_so_path()).expect("dlopen Rust .so");
            let c: libloading::Symbol<DequantFn> =
                c_lib.get(b"dequantize_granule\0").expect("dlsym C dequantize_granule");
            let r: libloading::Symbol<DequantFn> =
                r_lib.get(b"dequantize_granule\0").expect("dlsym Rust dequantize_granule");
            let c = *c;
            let r = *r;
            Libs { _c: c_lib, _r: r_lib, c, r }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) -- fixed seed per test for reproducibility
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
    pub fn u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
    pub fn u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn fill(&mut self, dst: &mut [u8]) {
        for b in dst.iter_mut() {
            *b = self.u8();
        }
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// One test input
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Inputs {
    /// Initial `grbuf` contents as raw bit patterns (`GR_TOTAL` words). Filled
    /// with a sentinel so that "the C wrote here / the Rust did not" is visible.
    pub grbuf: Vec<u32>,
    /// The `L12_scale_info` region (`SCI_REGION` bytes; the struct is the first
    /// `SCI_SIZE`, the rest is the controlled past-the-end padding).
    pub sci: Vec<u8>,
    /// Bitstream bytes.
    pub buf: Vec<u8>,
    pub pos: c_int,
    pub limit: c_int,
    pub group_size: c_int,
    /// Pass a NULL `grbuf` / `bs` / `sci` across the FFI boundary. The C has no
    /// null check anywhere, so these are genuine inputs whose behaviour (return
    /// or `SIGSEGV`) must match.
    pub null_grbuf: bool,
    pub null_bs: bool,
    pub null_sci: bool,
}

impl Inputs {
    pub fn new(rng: &mut Rng, buf_bytes: usize) -> Inputs {
        let mut grbuf = vec![0u32; GR_TOTAL];
        // Sentinel: a signalling-NaN-ish pattern that the library never writes.
        for (i, w) in grbuf.iter_mut().enumerate() {
            *w = 0x7FA0_0000 ^ (i as u32).wrapping_mul(0x9E37_79B9);
        }
        let mut sci = vec![0u8; SCI_REGION];
        rng.fill(&mut sci);
        let mut buf = vec![0u8; buf_bytes];
        rng.fill(&mut buf);
        Inputs {
            grbuf,
            sci,
            buf,
            pos: 0,
            // Leave head-room: `get_bits` may read up to ~2 bytes past the last
            // bit the guard admits.
            limit: ((buf_bytes - 16) * 8) as c_int,
            group_size: 3,
            null_grbuf: false,
            null_bs: false,
            null_sci: false,
        }
    }

    pub fn total_bands(&mut self, v: u8) -> &mut Self {
        self.sci[OFF_TOTAL_BANDS] = v;
        self
    }
    pub fn stereo_bands(&mut self, v: u8) -> &mut Self {
        self.sci[OFF_STEREO_BANDS] = v;
        self
    }
    /// Writes `ba` into every `bitalloc` slot the C can reach, *including* the
    /// out-of-array indices (`scfcod` and the past-struct padding), so a row
    /// that says "bitalloc in 1..=16" really means every `ba` the run sees.
    pub fn ba_all(&mut self, mut f: impl FnMut(usize) -> u8) -> &mut Self {
        for i in 0..(SCI_REGION - OFF_BITALLOC) {
            self.sci[OFF_BITALLOC + i] = f(i);
        }
        self
    }
    pub fn ba_at(&mut self, i: usize, v: u8) -> &mut Self {
        self.sci[OFF_BITALLOC + i] = v;
        self
    }
    pub fn pos(&mut self, v: c_int) -> &mut Self {
        self.pos = v;
        self
    }
    pub fn limit(&mut self, v: c_int) -> &mut Self {
        self.limit = v;
        self
    }
    pub fn group_size(&mut self, v: c_int) -> &mut Self {
        self.group_size = v;
        self
    }
    pub fn buf_fill(&mut self, v: u8) -> &mut Self {
        for b in self.buf.iter_mut() {
            *b = v;
        }
        self
    }
    pub fn nulls(&mut self, grbuf: bool, bs: bool, sci: bool) -> &mut Self {
        self.null_grbuf = grbuf;
        self.null_bs = bs;
        self.null_sci = sci;
        self
    }
    pub fn done(&self) -> Inputs {
        self.clone()
    }
}

// ---------------------------------------------------------------------------
// Outcome of one call
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
pub struct Outcome {
    /// Raw `waitpid` status of the forked child (encodes normal exit vs signal).
    pub status: c_int,
    /// `1` if the child returned from the call, `0` if it died inside it.
    pub completed: u32,
    pub ret: c_int,
    pub pos: c_int,
    pub limit: c_int,
    pub grbuf: Vec<u32>,
    pub sci: Vec<u8>,
    pub buf: Vec<u8>,
}

impl Outcome {
    pub fn signal(&self) -> Option<c_int> {
        if self.status & 0x7f != 0 && self.status & 0x7f != 0x7f {
            Some(self.status & 0x7f)
        } else {
            None
        }
    }
    pub fn crashed(&self) -> bool {
        self.completed == 0
    }
    pub fn describe(&self) -> String {
        match self.signal() {
            Some(s) => format!("KILLED by signal {s} (completed={})", self.completed),
            None => format!(
                "exit={} completed={} ret={} pos={} limit={}",
                (self.status >> 8) & 0xff,
                self.completed,
                self.ret,
                self.pos,
                self.limit
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared-memory region layout
// ---------------------------------------------------------------------------

struct Region {
    base: *mut u8,
    len: usize,
    guard_base: *mut u8,
    outer: usize,
    gr_off: usize,
    sci_off: usize,
    buf_off: usize,
    bs_off: usize,
    res_off: usize,
}

fn align_up(x: usize, a: usize) -> usize {
    (x + a - 1) & !(a - 1)
}

/// `p = bs->buf + (bs->pos >> 3)` with a 32-bit `pos` can land anywhere in
/// `buf ± 256 MiB`. When the C's `int` overflow makes the exhaustion guard miss,
/// it dereferences exactly such a wild pointer. To make that *deterministic and
/// identical for both libraries* we surround the working region with a
/// `PROT_NONE` reservation wider than the whole reachable range, so any wild
/// access is guaranteed to fault instead of accidentally hitting some unrelated
/// mapping (which would differ between the two runs).
const GUARD: usize = 512 * 1024 * 1024;

impl Region {
    fn new(buf_bytes: usize) -> Region {
        let gr_off = 0usize;
        let sci_off = align_up(gr_off + GR_TOTAL * 4, 16);
        let buf_off = align_up(sci_off + SCI_REGION, 16);
        // Head-room so a legal `get_bits` over-read past `limit/8` stays mapped.
        let bs_off = align_up(buf_off + buf_bytes + 4096, 16);
        let res_off = align_up(bs_off + std::mem::size_of::<BsT>(), 16);
        let len = align_up(res_off + 16, 4096);

        let outer = GUARD + len + GUARD;
        let guard_base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                outer,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };
        assert!(guard_base != libc::MAP_FAILED, "guard mmap failed");
        let want = unsafe { (guard_base as *mut u8).add(GUARD) };
        let base = unsafe {
            libc::mmap(
                want as *mut libc::c_void,
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
                -1,
                0,
            )
        };
        assert!(base != libc::MAP_FAILED, "mmap failed");
        assert_eq!(base as usize, want as usize, "MAP_FIXED did not honour the address");
        Region {
            base: base as *mut u8,
            len,
            guard_base: guard_base as *mut u8,
            outer,
            gr_off,
            sci_off,
            buf_off,
            bs_off,
            res_off,
        }
    }
    unsafe fn at(&self, off: usize) -> *mut u8 {
        self.base.add(off)
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.guard_base as *mut libc::c_void, self.outer);
        }
    }
}

/// Runs one call of `f` on a private copy of `inputs` inside a forked child and
/// reports everything observable: crash status, return value, the mutated
/// `bs_t`, and the full `grbuf` / `sci` / `buf` memory afterwards.
pub fn run(f: DequantFn, inputs: &Inputs) -> Outcome {
    // The 1 GiB guard reservation is expensive to set up, so regions are cached
    // per bitstream size and re-initialised instead of re-mapped.
    thread_local! {
        static REGIONS: std::cell::RefCell<Vec<(usize, Region)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }
    REGIONS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if !cache.iter().any(|(n, _)| *n == inputs.buf.len()) {
            let r = Region::new(inputs.buf.len());
            cache.push((inputs.buf.len(), r));
        }
        let reg = &cache.iter().find(|(n, _)| *n == inputs.buf.len()).unwrap().1;
        run_in(reg, f, inputs)
    })
}

fn run_in(reg: &Region, f: DequantFn, inputs: &Inputs) -> Outcome {
    unsafe {
        std::ptr::copy_nonoverlapping(
            inputs.grbuf.as_ptr() as *const u8,
            reg.at(reg.gr_off),
            GR_TOTAL * 4,
        );
        std::ptr::copy_nonoverlapping(inputs.sci.as_ptr(), reg.at(reg.sci_off), SCI_REGION);
        std::ptr::copy_nonoverlapping(inputs.buf.as_ptr(), reg.at(reg.buf_off), inputs.buf.len());

        let bs = reg.at(reg.bs_off) as *mut BsT;
        (*bs) = BsT { buf: reg.at(reg.buf_off) as *const u8, pos: inputs.pos, limit: inputs.limit };

        let res = reg.at(reg.res_off) as *mut c_int;
        *res = 0; // ret
        *res.add(1) = 0; // completed flag

        // `grbuf` is handed out in the middle of the reservation so that a
        // negative `group_size` can walk backwards without leaving the mapping.
        let grbuf = reg.at(reg.gr_off + GR_PREFIX * 4) as *mut f32;
        let sci = reg.at(reg.sci_off);
        let gs = inputs.group_size;

        let grbuf = if inputs.null_grbuf { std::ptr::null_mut() } else { grbuf };
        let sci = if inputs.null_sci { std::ptr::null_mut() } else { sci };
        let bs_arg = if inputs.null_bs { std::ptr::null_mut() } else { bs };

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: nothing but the call and the two stores. No allocation, no
            // I/O, no locks -- all of which would be unsafe after fork().
            //
            // Several rows crash on purpose (the C's `int` overflow makes the
            // exhaustion guard miss and it dereferences a wild pointer). Drop
            // the core limit first, otherwise every such row pays for a core
            // dump / the system crash handler.
            let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            libc::prctl(libc::PR_SET_DUMPABLE, 0);
            let r = f(grbuf, bs_arg, sci, gs);
            *res = r;
            *res.add(1) = 1;
            libc::_exit(0);
        }

        let mut status: c_int = 0;
        loop {
            let w = libc::waitpid(pid, &mut status, 0);
            if w == pid {
                break;
            }
            assert!(w >= 0 || *libc::__errno_location() == libc::EINTR, "waitpid failed");
        }

        let mut grbuf_out = vec![0u32; GR_TOTAL];
        std::ptr::copy_nonoverlapping(
            reg.at(reg.gr_off),
            grbuf_out.as_mut_ptr() as *mut u8,
            GR_TOTAL * 4,
        );
        let mut sci_out = vec![0u8; SCI_REGION];
        std::ptr::copy_nonoverlapping(reg.at(reg.sci_off), sci_out.as_mut_ptr(), SCI_REGION);
        let mut buf_out = vec![0u8; inputs.buf.len()];
        std::ptr::copy_nonoverlapping(reg.at(reg.buf_off), buf_out.as_mut_ptr(), inputs.buf.len());

        Outcome {
            status,
            completed: *res.add(1) as u32,
            ret: *res,
            pos: (*bs).pos,
            limit: (*bs).limit,
            grbuf: grbuf_out,
            sci: sci_out,
            buf: buf_out,
        }
    }
}

/// First differing float in `grbuf`, expressed relative to the pointer the
/// library was handed.
fn first_grbuf_diff(a: &[u32], b: &[u32]) -> Option<(isize, u32, u32)> {
    for i in 0..a.len() {
        if a[i] != b[i] {
            return Some((i as isize - GR_PREFIX as isize, a[i], b[i]));
        }
    }
    None
}

/// The whole point: call both `.so`s on identical input and require identical
/// observable behaviour. Returns the two outcomes so a caller can additionally
/// assert *which* behaviour occurred (e.g. "both really did crash").
pub fn assert_same_ret(libs: &Libs, row: &str, iter: usize, inputs: &Inputs) -> (Outcome, Outcome) {
    let c = run(libs.c, inputs);
    let r = run(libs.r, inputs);

    let ctx = || {
        format!(
            "row {row} iter {iter}: total_bands={} group_size={} pos={} limit={} \
             ba[0..8]={:?} buf_len={} nulls=(gr={},bs={},sci={})",
            inputs.sci[OFF_TOTAL_BANDS],
            inputs.group_size,
            inputs.pos,
            inputs.limit,
            &inputs.sci[OFF_BITALLOC..OFF_BITALLOC + 8],
            inputs.buf.len(),
            inputs.null_grbuf,
            inputs.null_bs,
            inputs.null_sci,
        )
    };

    assert_eq!(
        c.completed,
        r.completed,
        "{}\n  one side survived the call and the other did not\n  C: {}\n  R: {}",
        ctx(),
        c.describe(),
        r.describe()
    );
    assert_eq!(
        c.status & 0x7f,
        r.status & 0x7f,
        "{}\n  different termination signal\n  C: {}\n  R: {}",
        ctx(),
        c.describe(),
        r.describe()
    );

    if c.completed == 0 {
        // Both died the same way inside the call; nothing further to compare.
        return (c, r);
    }

    assert_eq!(c.ret, r.ret, "{}\n  return value differs", ctx());
    assert_eq!(c.pos, r.pos, "{}\n  bs->pos differs after the call", ctx());
    assert_eq!(c.limit, r.limit, "{}\n  bs->limit was modified differently", ctx());
    if let Some((idx, cv, rv)) = first_grbuf_diff(&c.grbuf, &r.grbuf) {
        panic!(
            "{}\n  grbuf differs at grbuf[{idx}]: C=0x{cv:08X} ({}) Rust=0x{rv:08X} ({})",
            ctx(),
            f32::from_bits(cv),
            f32::from_bits(rv)
        );
    }
    assert!(c.sci == r.sci, "{}\n  L12_scale_info memory differs (spurious write)", ctx());
    assert!(c.buf == r.buf, "{}\n  bitstream buffer differs (spurious write)", ctx());
    (c, r)
}

pub fn assert_same(libs: &Libs, row: &str, iter: usize, inputs: &Inputs) {
    assert_same_ret(libs, row, iter, inputs);
}

// ---------------------------------------------------------------------------
// `ba` classification -- mirrors the C's own branch structure
// ---------------------------------------------------------------------------

/// Number of bits `get_bits` is asked for when `ba >= 17`, computed exactly the
/// way the C does it (including the masked shift that x86 performs for the
/// shift counts C leaves undefined).
pub fn grouped_n(ba: i32) -> i32 {
    let m = 2i32.wrapping_shl((ba - 17) as u32).wrapping_add(1) as u32;
    m.wrapping_add(2).wrapping_sub(m >> 3) as i32
}

/// `ba` values whose bit demand is small enough that thousands of `get_bits`
/// calls cannot overflow `bs->pos`. The excluded ones (`(ba-17) % 32` in
/// `12..=30`) ask for up to 1.8 billion bits; they are covered separately by
/// the rows that expect (and compare) the resulting crash.
pub fn ba_is_pos_safe(ba: u8) -> bool {
    if ba < 17 {
        return true;
    }
    grouped_n(ba as i32).unsigned_abs() <= 4096
}
