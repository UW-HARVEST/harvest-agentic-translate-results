//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported `contrast_ratio` symbol. The Rust
//! functions are never called directly, so the `#[no_mangle] extern "C"`
//! wrapper and the struct-by-value ABI are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// Mirror of the C `cb_rgb_255` struct: three `unsigned char` channels,
/// size 3, align 1. Passed by value in one x86-64 SysV INTEGER register.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    /// The struct's 3 bytes placed in the low 24 bits of a register-sized
    /// word, with `garbage` occupying the 5 unused high bytes.
    pub fn to_reg(self, garbage: u64) -> u64 {
        let payload = (self.r as u64) | ((self.g as u64) << 8) | ((self.b as u64) << 16);
        payload | (garbage & !0x00FF_FFFFu64)
    }
}

/// The natural signature: struct by value, `float` return.
type ContrastFn = unsafe extern "C" fn(Rgb, Rgb) -> f32;
/// A register-level view of the *same* symbol, used to inject garbage into the
/// argument registers' unused high bytes.
type ContrastRegFn = unsafe extern "C" fn(u64, u64) -> f32;

/// One loaded implementation.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    // Field order matters: the symbols borrow from `lib`, so `lib` must be
    // declared last to be dropped last.
    sym: ContrastFn,
    sym_reg: ContrastRegFn,
    _lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
            let sym: Symbol<ContrastFn> = lib.get(b"contrast_ratio\0").unwrap_or_else(|e| {
                panic!("{name} .so does not export `contrast_ratio`: {e}")
            });
            let sym_reg: Symbol<ContrastRegFn> = lib.get(b"contrast_ratio\0").unwrap();
            // Copy the raw fn pointers out so we are not fighting the borrow
            // checker on every call; `_lib` keeps the mapping alive.
            let sym = *sym;
            let sym_reg = *sym_reg;
            Impl {
                name,
                path,
                sym,
                sym_reg,
                _lib: lib,
            }
        }
    }

    #[inline]
    pub fn call(&self, a: Rgb, b: Rgb) -> f32 {
        unsafe { (self.sym)(a, b) }
    }

    #[inline]
    pub fn call_reg(&self, a: u64, b: u64) -> f32 {
        unsafe { (self.sym_reg)(a, b) }
    }

    /// Address `dlsym` resolved `contrast_ratio` to, for harness self-checks.
    pub fn fn_addr(&self) -> usize {
        self.sym as usize
    }
}

/// The C `.so` and the Rust `.so`, both dlopen'd.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object. Its file name is derived from the parent
/// directory name by `c_src/CMakeLists.txt`, so it must be discovered rather
/// than hard-coded.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = crate_root().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C .so found in {}.\nBuild it first:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        ),
        _ => found.remove(0),
    }
}

/// Locate the Rust `cdylib`, preferring the profile the test itself was built
/// with so that a stale artifact from the other profile is not picked up.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let name = "libcontrast_ratio_lib.so";
    let target = crate_root().join("target");
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in preferred {
        let p = target.join(profile).join(name);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "no Rust {name} found under {}. Build it with `cargo build` / `cargo build --release`.",
        target.display()
    );
}

pub fn load() -> Pair {
    let c_so = find_c_so();
    let rust_so = find_rust_so();
    assert!(
        Path::new(&c_so).is_file(),
        "C .so missing: {}",
        c_so.display()
    );
    Pair {
        c: Impl::load("C", c_so),
        rust: Impl::load("Rust", rust_so),
    }
}

impl Pair {
    /// Assert the two implementations agree **bit for bit** (NaN payload and
    /// sign included) for one input pair.
    #[inline]
    #[track_caller]
    pub fn assert_same(&self, a: Rgb, b: Rgb, row: &str) {
        let cv = self.c.call(a, b);
        let rv = self.rust.call(a, b);
        if cv.to_bits() != rv.to_bits() {
            panic!(
                "[{row}] divergence for A=({},{},{}) B=({},{},{}):\n  \
                 C    = {cv:?}  bits=0x{:08X}\n  \
                 Rust = {rv:?}  bits=0x{:08X}",
                a.r,
                a.g,
                a.b,
                b.r,
                b.g,
                b.b,
                cv.to_bits(),
                rv.to_bits(),
            );
        }
    }

    /// Same, through the register-level view with garbage in the unused high
    /// bytes of the argument registers.
    #[inline]
    #[track_caller]
    pub fn assert_same_reg(&self, a: Rgb, b: Rgb, ga: u64, gb: u64, row: &str) {
        let (wa, wb) = (a.to_reg(ga), b.to_reg(gb));
        let cv = self.c.call_reg(wa, wb);
        let rv = self.rust.call_reg(wa, wb);
        if cv.to_bits() != rv.to_bits() {
            panic!(
                "[{row}] divergence for A=({},{},{}) B=({},{},{}) \
                 regs=0x{wa:016X}/0x{wb:016X}:\n  \
                 C    = {cv:?}  bits=0x{:08X}\n  Rust = {rv:?}  bits=0x{:08X}",
                a.r,
                a.g,
                a.b,
                b.r,
                b.g,
                b.b,
                cv.to_bits(),
                rv.to_bits(),
            );
        }
        // The padding bits must not be observable at all: both sides must also
        // agree with the clean, struct-typed call.
        let clean_c = self.c.call(a, b);
        if cv.to_bits() != clean_c.to_bits() {
            panic!(
                "[{row}] C itself changed behaviour when argument-register \
                 padding was dirtied (0x{wa:016X}/0x{wb:016X}): clean={clean_c:?} dirty={cv:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    /// Fresh stream per test row so rows are independent yet reproducible.
    pub fn for_row(row: u64) -> Self {
        Rng(SEED ^ row.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `lo..=hi`.
    #[inline]
    pub fn byte_in(&mut self, lo: u8, hi: u8) -> u8 {
        let span = (hi - lo) as u64 + 1;
        lo + ((self.next_u64() >> 32) % span) as u8
    }
    #[inline]
    pub fn rgb(&mut self) -> Rgb {
        Rgb::new(self.next_u8(), self.next_u8(), self.next_u8())
    }
    #[inline]
    pub fn rgb_in(&mut self, lo: u8, hi: u8) -> Rgb {
        Rgb::new(
            self.byte_in(lo, hi),
            self.byte_in(lo, hi),
            self.byte_in(lo, hi),
        )
    }
}

// ---------------------------------------------------------------------------
// Facts derived from the C source, used to build configuration rows.
// ---------------------------------------------------------------------------

/// `0.04045 * 255 == 10.31475`, so byte <= 10 takes the linear branch
/// `C / 12.92` and byte >= 11 takes the `pow` branch.
pub const LINEAR_MAX: u8 = 10;
pub const POW_MIN: u8 = 11;

/// Boundary bytes: range ends, both sides of the transfer threshold.
pub const BOUNDARY_BYTES: [u8; 6] = [0, 1, 10, 11, 254, 255];

/// The 8 pure corners of the RGB cube.
pub const CORNERS: [Rgb; 8] = [
    Rgb::new(0, 0, 0),
    Rgb::new(255, 0, 0),
    Rgb::new(0, 255, 0),
    Rgb::new(0, 0, 255),
    Rgb::new(0, 255, 255),
    Rgb::new(255, 0, 255),
    Rgb::new(255, 255, 0),
    Rgb::new(255, 255, 255),
];

pub const BLACK: Rgb = Rgb::new(0, 0, 0);
pub const WHITE: Rgb = Rgb::new(255, 255, 255);
pub const MIDGRAY: Rgb = Rgb::new(128, 128, 128);

/// Draw a byte that lands on the requested transfer branch.
/// `pow_branch == false` → `0..=10`; `true` → `11..=255`.
pub fn byte_on_branch(rng: &mut Rng, pow_branch: bool) -> u8 {
    if pow_branch {
        rng.byte_in(POW_MIN, 255)
    } else {
        rng.byte_in(0, LINEAR_MAX)
    }
}

/// Build a color whose three channels take the branches encoded in the low 3
/// bits of `pattern` (bit0 = R, bit1 = G, bit2 = B; set = `pow` branch).
pub fn rgb_on_pattern(rng: &mut Rng, pattern: u8) -> Rgb {
    Rgb::new(
        byte_on_branch(rng, pattern & 1 != 0),
        byte_on_branch(rng, pattern & 2 != 0),
        byte_on_branch(rng, pattern & 4 != 0),
    )
}

/// Number of randomized samples per row. Lower it with `HARVEST_SAMPLES=…`
/// (only affects sample counts, never which rows run).
pub fn samples(default: usize) -> usize {
    match std::env::var("HARVEST_SAMPLES") {
        Ok(v) => v.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Stride for the exhaustive 2^24 sweeps. `1` = truly exhaustive (the default);
/// set `HARVEST_STRIDE` to subsample if the machine is slow.
pub fn stride() -> usize {
    match std::env::var("HARVEST_STRIDE") {
        Ok(v) => v.parse().unwrap_or(1).max(1),
        Err(_) => 1,
    }
}
