//! Shared differential-test harness.
//!
//! Both the C shared object and the Rust shared object are loaded with
//! `libloading` and driven purely through their exported `hsv_to_rgb` symbol —
//! the Rust implementation is never called directly, so the `#[no_mangle]`
//! `extern "C"` wrapper and the real ABI are what get tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// ABI of the single exported function (`c_src/include/lib.h`).
pub type HsvToRgbFn = unsafe extern "C" fn(*mut f32, *const f32);

pub struct Impl {
    // Kept alive so the loaded object is not unmapped while `f` is callable.
    _lib: Library,
    pub f: HsvToRgbFn,
    pub name: &'static str,
    pub path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libtranslated_rust.so`, overridable with `HARVEST_C_LIB`.
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_LIB") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

/// The Rust cdylib. Defaults to the sibling of the test binary's directory
/// (`target/<profile>/libhsv_to_rgb_lib.so`), overridable with
/// `HARVEST_RUST_LIB` so the same test binary can be pointed at the release
/// artifact.
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_LIB") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test>-<hash>  ->  target/<profile>/
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary layout")
        .to_path_buf();
    profile_dir.join("libhsv_to_rgb_lib.so")
}

/// True when the Rust object under test is a release build (no debug-only
/// pointer-UB assertions). Used to gate the deliberate-UB probes.
pub fn rust_lib_is_release() -> bool {
    let p = rust_lib_path();
    let s = p.to_string_lossy();
    s.contains("/release/") || std::env::var("HARVEST_UB_PROBE").as_deref() == Ok("1")
}

fn load(path: PathBuf, name: &'static str) -> Impl {
    assert!(
        path.exists(),
        "shared object {} not found for {name}. Build it first:\n  \
         C:    cd c_src && mkdir -p build && cd build && cmake .. \
         -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
         Rust: cargo build --offline --no-default-features",
        path.display()
    );
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    let f: HsvToRgbFn = unsafe {
        let sym: Symbol<HsvToRgbFn> = lib
            .get(b"hsv_to_rgb\0")
            .unwrap_or_else(|e| panic!("{} does not export hsv_to_rgb: {e}", path.display()));
        *sym
    };
    Impl {
        _lib: lib,
        f,
        name,
        path,
    }
}

pub fn load_c() -> Impl {
    load(c_lib_path(), "C")
}

pub fn load_rust() -> Impl {
    load(rust_lib_path(), "Rust")
}

pub fn load_pair() -> (Impl, Impl) {
    (load_c(), load_rust())
}

// ---------------------------------------------------------------------------
// Buffers
// ---------------------------------------------------------------------------

/// Number of 32-bit words in each scratch buffer.
pub const BUF_WORDS: usize = 16;
/// Index of the 3-float window inside a buffer (leaves canaries on both sides).
pub const WINDOW: usize = 6;

/// 16-word, 16-byte-aligned scratch buffer. Held as raw `u32` words so that no
/// `f32` value (which could be a signaling NaN) is ever materialised in Rust —
/// only the C/Rust libraries under test interpret the memory as `float`.
#[repr(C, align(16))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Buf(pub [u32; BUF_WORDS]);

/// Distinctive canary pattern; any of these words changing means the library
/// wrote outside `dest[0..3]`.
pub fn canaries() -> Buf {
    let mut b = Buf([0; BUF_WORDS]);
    for (i, w) in b.0.iter_mut().enumerate() {
        *w = 0xCA11_0000u32 ^ ((i as u32) * 0x0101_0101);
    }
    b
}

/// How `dest` relates to `src`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alias {
    /// Two disjoint buffers.
    Separate,
    /// `dest == src`.
    Same,
    /// `dest == src + 1`.
    DestPlus1,
    /// `dest == src - 1`.
    DestMinus1,
}

impl Alias {
    pub const ALL: [Alias; 4] = [
        Alias::Separate,
        Alias::Same,
        Alias::DestPlus1,
        Alias::DestMinus1,
    ];
}

/// Complete observable outcome of one call: the final contents of both scratch
/// buffers (payload *and* canaries).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub sbuf: Buf,
    pub dbuf: Buf,
}

/// Invoke one implementation once with the given `src` words and aliasing mode.
pub fn call(imp: &Impl, src: [u32; 3], alias: Alias) -> Outcome {
    let mut sbuf = canaries();
    let mut dbuf = canaries();
    sbuf.0[WINDOW] = src[0];
    sbuf.0[WINDOW + 1] = src[1];
    sbuf.0[WINDOW + 2] = src[2];

    unsafe {
        let sbase = sbuf.0.as_mut_ptr();
        let src_ptr = sbase.add(WINDOW) as *const f32;
        let dest_ptr = match alias {
            Alias::Separate => dbuf.0.as_mut_ptr().add(WINDOW) as *mut f32,
            Alias::Same => sbase.add(WINDOW) as *mut f32,
            Alias::DestPlus1 => sbase.add(WINDOW + 1) as *mut f32,
            Alias::DestMinus1 => sbase.add(WINDOW - 1) as *mut f32,
        };
        (imp.f)(dest_ptr, src_ptr);
    }

    Outcome { sbuf, dbuf }
}

/// Convenience: the three `dest` words after a disjoint-buffer call.
pub fn dest3(imp: &Impl, src: [u32; 3]) -> [u32; 3] {
    let o = call(imp, src, Alias::Separate);
    [
        o.dbuf.0[WINDOW],
        o.dbuf.0[WINDOW + 1],
        o.dbuf.0[WINDOW + 2],
    ]
}

pub fn hex3(x: [u32; 3]) -> String {
    format!("{:08x} {:08x} {:08x}", x[0], x[1], x[2])
}

fn fmt_words(b: &Buf) -> String {
    b.0.iter()
        .map(|w| format!("{w:08x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Which canary words must stay intact for a given aliasing mode.
fn expect_canaries(alias: Alias, out: &Outcome, ctx: &str, who: &str) {
    let base = canaries();
    // Words the library is allowed to write.
    let written: &[usize] = match alias {
        Alias::Separate => &[],
        Alias::Same => &[WINDOW, WINDOW + 1, WINDOW + 2],
        Alias::DestPlus1 => &[WINDOW + 1, WINDOW + 2, WINDOW + 3],
        Alias::DestMinus1 => &[WINDOW - 1, WINDOW, WINDOW + 1],
    };
    for i in 0..BUF_WORDS {
        // src payload words are seeded by us, skip them
        if (WINDOW..WINDOW + 3).contains(&i) || written.contains(&i) {
            continue;
        }
        assert_eq!(
            out.sbuf.0[i], base.0[i],
            "{who} clobbered src-buffer canary word {i} ({ctx}, alias {alias:?})\n\
             got: {}",
            fmt_words(&out.sbuf)
        );
    }
    if alias == Alias::Separate {
        for i in 0..BUF_WORDS {
            if (WINDOW..WINDOW + 3).contains(&i) {
                continue;
            }
            assert_eq!(
                out.dbuf.0[i], base.0[i],
                "{who} clobbered dest-buffer canary word {i} ({ctx})\n\
                 got: {}",
                fmt_words(&out.dbuf)
            );
        }
    } else {
        assert_eq!(
            out.dbuf, base,
            "{who} touched the unused dest buffer ({ctx}, alias {alias:?})"
        );
    }
}

/// Differential check: run both libraries with the identical input and require
/// byte-identical buffers afterwards.
pub fn assert_same(c: &Impl, r: &Impl, src: [u32; 3], alias: Alias, ctx: &str) {
    let oc = call(c, src, alias);
    let or = call(r, src, alias);
    if oc != or {
        panic!(
            "DIVERGENCE ({ctx}, alias {alias:?})\n\
             src bits : {:08x} {:08x} {:08x}\n\
             src float: {} {} {}\n\
             C   sbuf : {}\n\
             Rust sbuf: {}\n\
             C   dbuf : {}\n\
             Rust dbuf: {}\n\
             C lib    : {}\n\
             Rust lib : {}",
            src[0],
            src[1],
            src[2],
            f32::from_bits(src[0]),
            f32::from_bits(src[1]),
            f32::from_bits(src[2]),
            fmt_words(&oc.sbuf),
            fmt_words(&or.sbuf),
            fmt_words(&oc.dbuf),
            fmt_words(&or.dbuf),
            c.path.display(),
            r.path.display(),
        );
    }
    expect_canaries(alias, &oc, ctx, "C");
    expect_canaries(alias, &or, ctx, "Rust");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn seeded() -> Self {
        Rng(SEED)
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
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    /// Any of the 2^32 bit patterns (all float classes, both signs).
    pub fn any_bits(&mut self) -> u32 {
        self.next_u32()
    }
    /// Pick an element of a slice.
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u32() as usize) % xs.len()]
    }
}

// ---------------------------------------------------------------------------
// Interesting constants
// ---------------------------------------------------------------------------

pub const POS_ZERO: u32 = 0x0000_0000;
pub const NEG_ZERO: u32 = 0x8000_0000;
pub const POS_INF: u32 = 0x7F80_0000;
pub const NEG_INF: u32 = 0xFF80_0000;

/// Quiet + signaling NaNs with distinct payloads and both signs.
pub const NANS: [u32; 8] = [
    0x7FC0_0000, // canonical qNaN
    0xFFC0_0000, // x86 "indefinite" qNaN
    0x7FC0_1234, // qNaN, payload
    0xFFDE_AD01, // negative qNaN, payload
    0x7F80_0001, // sNaN, smallest payload
    0xFF80_0001, // negative sNaN
    0x7FBF_FFFF, // sNaN, max payload
    0x7FFF_FFFF, // qNaN, all payload bits set
];

/// Subnormals, zeros and the normal/subnormal boundary.
pub const TINY: [u32; 10] = [
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x0000_0001, // +smallest subnormal
    0x8000_0001, // -smallest subnormal
    0x0040_0000, // mid subnormal
    0x007F_FFFF, // largest subnormal
    0x807F_FFFF, // -largest subnormal
    0x0080_0000, // FLT_MIN
    0x8080_0000, // -FLT_MIN
    0x0080_0001, // just above FLT_MIN
];

/// A broad set of "interesting" float bit patterns.
pub const SPECIALS: [u32; 26] = [
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x0000_0001, // +min subnormal
    0x8000_0001, // -min subnormal
    0x007F_FFFF, // max subnormal
    0x0080_0000, // FLT_MIN
    0x3F80_0000, // 1.0
    0xBF80_0000, // -1.0
    0x3F00_0000, // 0.5
    0x4270_0000, // 60.0
    0x42F0_0000, // 120.0
    0x4334_0000, // 180.0
    0x4370_0000, // 240.0
    0x4396_0000, // 300.0
    0x43B4_0000, // 360.0
    0x461C_4000, // 10000.0
    0x4F00_0000, // 2^31
    0xCF00_0000, // -2^31
    0x7F7F_FFFF, // FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // qNaN
    0xFFC0_0000, // -qNaN
    0x7F80_0001, // sNaN
    0x3F7F_FFFF, // just below 1.0
];

/// Bits of `f32` `x`.
pub fn bits(x: f32) -> u32 {
    x.to_bits()
}
