//! Shared differential-testing harness.
//!
//! Loads BOTH shared objects with `libloading` and calls only their exported
//! `extern "C"` symbols — the Rust crate is never linked directly, so the
//! `#[no_mangle]` wrappers are under test too.
//!
//! Everything is compared as **raw bit patterns**, never as float values:
//! `==` on floats would silently accept `NaN != NaN` and would treat `+0.0`
//! and `-0.0` as equal. Both distinctions matter for this library.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

pub type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, c_int, f64) -> c_int;
pub type SpectralFn = unsafe extern "C" fn(*mut f32, *mut f32, c_int) -> f64;

/// One loaded implementation (C or Rust) plus its resolved entry points.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub match_fn: MatchFn,
    pub spectral_fn: SpectralFn,
}

impl Impl {
    unsafe fn open(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let m: Symbol<MatchFn> = unsafe { lib.get(b"match\0") }
            .unwrap_or_else(|e| panic!("{} is missing symbol `match`: {e}", path.display()));
        let s: Symbol<SpectralFn> = unsafe { lib.get(b"spectral_contrast\0") }.unwrap_or_else(|e| {
            panic!(
                "{} is missing symbol `spectral_contrast`: {e}",
                path.display()
            )
        });
        let match_fn = *m;
        let spectral_fn = *s;
        Impl {
            name,
            path,
            _lib: lib,
            match_fn,
            spectral_fn,
        }
    }
}

/// The crate root (`translation/`).
pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The working directory that holds both `c_src/` and `translation/`.
pub fn work_root() -> PathBuf {
    crate_root().parent().unwrap().to_path_buf()
}

fn first_so_in(dir: &Path) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Locate the C shared object. Its file name is derived from the *parent
/// directory name* by `c_src/CMakeLists.txt`
/// (`cmake_path(GET parent FILENAME project_name)`), so it must be discovered
/// rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    // Escape hatch used by `run_all.sh` to re-run the whole suite against C
    // objects built at other optimization levels.
    if let Some(p) = std::env::var_os("C_SO") {
        return PathBuf::from(p);
    }
    let dir = work_root().join("c_src/build");
    first_so_in(&dir).unwrap_or_else(|| {
        panic!(
            "no .so under {}. Build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

/// Locate the Rust cdylib. Prefers `target/release` (the artifact an external
/// consumer would ship) and falls back to the `target/debug` copy that
/// `cargo test` builds as a side effect.
pub fn rust_so_path() -> PathBuf {
    let name = "libunderhanded_c_nuke_lib.so";
    // CARGO_TARGET_DIR may relocate `target/`; honour it.
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root().join("target"));
    for profile in ["release", "debug"] {
        let p = target.join(profile).join(name);
        if p.exists() {
            return p;
        }
    }

    // `cargo test` alone does not emit the cdylib (nothing links it -- the
    // tests load it through `dlopen`), so build it on demand. This keeps a
    // bare `cargo test` self-sufficient.
    let status = std::process::Command::new(env!("CARGO"))
        .args(["build", "--offline", "--release"])
        .current_dir(crate_root())
        .status();
    if !matches!(&status, Ok(s) if s.success()) {
        // Retry without --offline in case the registry is reachable.
        let _ = std::process::Command::new(env!("CARGO"))
            .args(["build", "--release"])
            .current_dir(crate_root())
            .status();
    }
    let p = target.join("release").join(name);
    if p.exists() {
        return p;
    }
    panic!(
        "{name} not found under {}/{{release,debug}} and building it failed. Build it manually:\n  \
         cd translation && cargo build --offline --release",
        target.display()
    )
}

/// Both implementations, loaded once per test process.
pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn load() -> Pair {
    unsafe {
        Pair {
            c: Impl::open("C", c_so_path()),
            rs: Impl::open("Rust", rust_so_path()),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    /// Fixed seed so every reported divergence is reproducible.
    pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

    pub fn new() -> Rng {
        Rng(Rng::SEED)
    }
    pub fn with_seed(s: u64) -> Rng {
        Rng(s)
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

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Completely unconstrained f64 — every bit pattern, including NaNs,
    /// infinities and denormals.
    pub fn raw_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    /// Completely unconstrained f32.
    pub fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A plausible non-negative spectral-energy bin.
    pub fn spectrum_bin(&mut self) -> f64 {
        self.range(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Bit-level comparison helpers
// ---------------------------------------------------------------------------

pub fn bits64(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

pub fn bits32(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

fn fmt64(v: &[f64]) -> String {
    let mut s = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{:016x}", x.to_bits()));
        if i == 31 && v.len() > 32 {
            s.push_str(" ...");
            break;
        }
    }
    s.push(']');
    s
}

fn fmt32(v: &[f32]) -> String {
    let mut s = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{:08x}", x.to_bits()));
        if i == 63 && v.len() > 64 {
            s.push_str(" ...");
            break;
        }
    }
    s.push(']');
    s
}

/// An 8-byte-aligned scratch buffer that can be viewed as `f64`s or as `f32`s.
/// `match` writes `double`s into it; `spectral_contrast` reads `float`s out of
/// the very same bytes. That dual view is the whole point of this library's
/// `float_t` confusion, so the harness models it explicitly.
#[derive(Clone)]
pub struct Buf {
    pub d: Vec<f64>,
}

impl Buf {
    pub fn from_f64(v: &[f64]) -> Buf {
        Buf { d: v.to_vec() }
    }

    /// A buffer holding `n` f32 elements (padded up to a whole f64 count).
    pub fn f32_len(n: usize) -> Buf {
        Buf {
            d: vec![0.0; n.div_ceil(2).max(1)],
        }
    }

    pub fn from_f32(v: &[f32]) -> Buf {
        let mut b = Buf::f32_len(v.len());
        b.as_f32_mut(v.len()).copy_from_slice(v);
        b
    }

    pub fn as_f64_ptr(&mut self) -> *mut f64 {
        self.d.as_mut_ptr()
    }

    pub fn as_f32_ptr(&mut self) -> *mut f32 {
        self.d.as_mut_ptr() as *mut f32
    }

    pub fn as_f32_mut(&mut self, n: usize) -> &mut [f32] {
        assert!(n <= self.d.len() * 2);
        unsafe { std::slice::from_raw_parts_mut(self.d.as_mut_ptr() as *mut f32, n) }
    }

    pub fn as_f32(&self, n: usize) -> &[f32] {
        assert!(n <= self.d.len() * 2);
        unsafe { std::slice::from_raw_parts(self.d.as_ptr() as *const f32, n) }
    }

    /// Every byte of the buffer, as f64 bit patterns — used for the
    /// "buffer unchanged / buffer mutated identically" assertions.
    pub fn all_bits(&self) -> Vec<u64> {
        bits64(&self.d)
    }
}

// ---------------------------------------------------------------------------
// The two differential drivers
// ---------------------------------------------------------------------------

/// Call `spectral_contrast(a, b, length)` on both `.so`s and compare the
/// returned double's bits AND both in-place-mutated buffers, byte for byte.
///
/// `alias` selects the pointer relationship, so the aliasing rows of
/// CONFIGS.md go through the same code path as the disjoint ones.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Alias {
    /// two separate buffers
    Disjoint,
    /// `b == a`
    Same,
    /// `b == a + 1` (partial overlap)
    OffsetB(usize),
    /// `a == b + 1` (partial overlap, other direction)
    OffsetA(usize),
}

pub fn diff_spectral(
    p: &Pair,
    ctx: &str,
    a_init: &[f32],
    b_init: &[f32],
    length: c_int,
    alias: Alias,
) {
    // How many f32 slots must the backing store hold?
    let n = length.max(0) as usize;
    let pad = match alias {
        Alias::OffsetB(k) | Alias::OffsetA(k) => k,
        _ => 0,
    };
    let slots = (a_init.len().max(b_init.len()).max(n) + pad).max(1);

    let run = |im: &Impl| -> (u64, Vec<u64>, Vec<u64>) {
        let mut ba = Buf::f32_len(slots);
        ba.as_f32_mut(a_init.len()).copy_from_slice(a_init);
        let mut bb = Buf::f32_len(slots);
        bb.as_f32_mut(b_init.len()).copy_from_slice(b_init);
        let (pa, pb) = match alias {
            Alias::Disjoint => (ba.as_f32_ptr(), bb.as_f32_ptr()),
            Alias::Same => (ba.as_f32_ptr(), ba.as_f32_ptr()),
            Alias::OffsetB(k) => (ba.as_f32_ptr(), unsafe { ba.as_f32_ptr().add(k) }),
            Alias::OffsetA(k) => (unsafe { ba.as_f32_ptr().add(k) }, ba.as_f32_ptr()),
        };
        let r = unsafe { (im.spectral_fn)(pa, pb, length) };
        (r.to_bits(), ba.all_bits(), bb.all_bits())
    };

    let (rc, ac, bc) = run(&p.c);
    let (rr, ar, br) = run(&p.rs);

    if rc != rr || ac != ar || bc != br {
        let mut msg = format!(
            "DIVERGENCE in spectral_contrast [{ctx}]\n  length = {length}, alias = {alias:?}\n"
        );
        msg += &format!("  a_init = {}\n", fmt32(a_init));
        msg += &format!("  b_init = {}\n", fmt32(b_init));
        msg += &format!("  return  C = {rc:016x}   Rust = {rr:016x}\n");
        if ac != ar {
            msg += &format!(
                "  buffer a after: C = {}\n                  R = {}\n",
                fmt64(&ac.iter().map(|&x| f64::from_bits(x)).collect::<Vec<_>>()),
                fmt64(&ar.iter().map(|&x| f64::from_bits(x)).collect::<Vec<_>>())
            );
        }
        if bc != br {
            msg += &format!(
                "  buffer b after: C = {}\n                  R = {}\n",
                fmt64(&bc.iter().map(|&x| f64::from_bits(x)).collect::<Vec<_>>()),
                fmt64(&br.iter().map(|&x| f64::from_bits(x)).collect::<Vec<_>>())
            );
        }
        panic!("{msg}");
    }
}

/// Call `match(test, reference, bins, threshold)` on both `.so`s and compare
/// the returned `int` AND both input buffers (which `match` must NOT modify —
/// it copies into its own VLAs — so this also checks for stray writes).
pub fn diff_match(
    p: &Pair,
    ctx: &str,
    test: &[f64],
    reference: &[f64],
    bins: c_int,
    threshold: f64,
    same_ptr: bool,
) {
    let run = |im: &Impl| -> (c_int, Vec<u64>, Vec<u64>) {
        let mut bt = Buf::from_f64(test);
        let mut br = Buf::from_f64(reference);
        let (pt, pr) = if same_ptr {
            (bt.as_f64_ptr(), bt.as_f64_ptr())
        } else {
            (bt.as_f64_ptr(), br.as_f64_ptr())
        };
        let r = unsafe { (im.match_fn)(pt, pr, bins, threshold) };
        (r, bt.all_bits(), br.all_bits())
    };

    let (rc, tc, refc) = run(&p.c);
    let (rr, tr, refr) = run(&p.rs);

    if rc != rr || tc != tr || refc != refr {
        let mut msg = format!("DIVERGENCE in match [{ctx}]\n");
        msg += &format!(
            "  bins = {bins}, threshold = {:016x} ({threshold:e}), same_ptr = {same_ptr}\n",
            threshold.to_bits()
        );
        msg += &format!("  test      = {}\n", fmt64(test));
        msg += &format!("  reference = {}\n", fmt64(reference));
        msg += &format!("  return  C = {rc}   Rust = {rr}\n");
        if tc != tr {
            msg += "  *** C and Rust left `test` in different states (stray write) ***\n";
        }
        if refc != refr {
            msg += "  *** C and Rust left `reference` in different states (stray write) ***\n";
        }
        panic!("{msg}");
    }
}

/// The interesting `threshold` value classes, from CONFIGS.md rows C15/C22/C30.
pub fn threshold_classes() -> Vec<f64> {
    vec![
        -1.0,
        -0.5,
        -0.0,
        0.0,
        f64::MIN_POSITIVE,
        1e-300,
        0.25,
        0.5,
        0.9,
        1.0,
        1.0 + f64::EPSILON,
        1.5,
        2.0,
        1e300,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001), // qNaN, payload 1
        f64::from_bits(0xFFF8_0000_0000_0000), // x86 indefinite qNaN
        f64::from_bits(0x7FF0_0000_0000_0001), // sNaN
    ]
}

/// The lengths / `bins` values the C code branches on (loop guards,
/// `N_SMOOTH == 16` window clamping, `length - 1`, odd/even f32 parity).
pub fn interesting_lengths() -> Vec<c_int> {
    vec![
        1, 2, 3, 4, 5, 6, 7, 8, 9, 14, 15, 16, 17, 18, 31, 32, 33, 63, 64, 65, 100, 127, 128, 255,
        256, 1024,
    ]
}
