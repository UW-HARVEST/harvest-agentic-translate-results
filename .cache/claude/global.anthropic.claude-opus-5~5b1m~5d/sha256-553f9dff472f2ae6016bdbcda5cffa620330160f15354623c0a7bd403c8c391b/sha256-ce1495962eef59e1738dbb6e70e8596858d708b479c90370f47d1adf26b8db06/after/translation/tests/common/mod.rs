//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported `flip_horizontal` symbol. The Rust
//! implementation is NEVER called directly as a Rust function — every call goes
//! through the `cdylib`'s C ABI export, so the `#[no_mangle]`/`extern "C"`
//! wrapper is under test too.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// ABI mirror of include/lib.h
// ---------------------------------------------------------------------------

/// `typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// `typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

pub type FlipFn = unsafe extern "C" fn(*mut CpImage);

/// Compile-time sanity: the harness struct must match the C ABI exactly.
const _: () = {
    assert!(core::mem::size_of::<CpPixel>() == 4);
    assert!(core::mem::align_of::<CpPixel>() == 1);
    assert!(core::mem::size_of::<CpImage>() == 16);
    assert!(core::mem::align_of::<CpImage>() == 8);
};

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<parent-dir-name>.so` — the CMake project name is derived
/// from the *parent* directory name, so glob rather than hardcode it.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("lib"))
            {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C shared library found in {}\n\
             build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        ),
        _ => found.remove(0),
    }
}

/// `target/{debug,release}/libflip_horizontal_lib.so` — prefer the profile this
/// test binary was itself compiled with, fall back to the other.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let name = format!("{}flip_horizontal_lib{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);

    // Walk up from the test executable: target/<profile>/deps/<exe>
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join(&name));
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join(&name));
            }
        }
    }
    let target = manifest_dir().join("target");
    let preferred = if cfg!(debug_assertions) { "debug" } else { "release" };
    let other = if cfg!(debug_assertions) { "release" } else { "debug" };
    candidates.push(target.join(preferred).join(&name));
    candidates.push(target.join(other).join(&name));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib `{name}` not found; build it with `cargo build` (and \
         `cargo build --release` for release tests).\nlooked in: {candidates:#?}"
    );
}

unsafe fn open(path: &Path) -> Library {
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()))
}

/// Holds both libraries open and hands out the `flip_horizontal` symbol of each.
pub struct Libs {
    c_lib: Library,
    rust_lib: Library,
}

impl Libs {
    pub fn load() -> Self {
        unsafe {
            Self {
                c_lib: open(&c_so_path()),
                rust_lib: open(&rust_so_path()),
            }
        }
    }

    fn sym<'a>(lib: &'a Library, which: &str) -> Symbol<'a, FlipFn> {
        unsafe { lib.get(b"flip_horizontal\0") }.unwrap_or_else(|e| {
            panic!("{which} .so does not export `flip_horizontal`: {e}")
        })
    }

    pub fn c_flip(&self) -> Symbol<'_, FlipFn> {
        Self::sym(&self.c_lib, "C")
    }

    pub fn rust_flip(&self) -> Symbol<'_, FlipFn> {
        Self::sym(&self.rust_lib, "Rust")
    }
}

/// Which implementation to invoke.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — no external dependency, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }

    /// Uniform-ish in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn pixels(&mut self, n: usize) -> Vec<CpPixel> {
        (0..n)
            .map(|_| CpPixel {
                r: self.next_u8(),
                g: self.next_u8(),
                b: self.next_u8(),
                a: self.next_u8(),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// The core differential driver
// ---------------------------------------------------------------------------

/// Guard pixels placed around the real buffer to catch out-of-bounds writes.
/// Chosen to be an unlikely-to-occur-by-accident pattern.
pub const GUARD: CpPixel = CpPixel {
    r: 0xDE,
    g: 0xAD,
    b: 0xBE,
    a: 0xEF,
};
pub const GUARD_LEN: usize = 8;

/// Result of running one implementation over one input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutcome {
    /// Pixel buffer contents after the call (the real region only).
    pub pixels: Vec<CpPixel>,
    /// Guard region before the real buffer, after the call.
    pub guard_lo: Vec<CpPixel>,
    /// Guard region after the real buffer, after the call.
    pub guard_hi: Vec<CpPixel>,
    /// `img.w` after the call (must be unchanged).
    pub w_after: c_int,
    /// `img.h` after the call (must be unchanged).
    pub h_after: c_int,
    /// Whether `img.pix` still points where it did (must be unchanged).
    pub pix_unchanged: bool,
}

/// Run one implementation with a *guard-padded* copy of `pixels`.
///
/// `pixels` is the logical buffer contents; `w`/`h` are passed to the callee
/// verbatim (they may be inconsistent with `pixels.len()` on purpose, as long
/// as no dereference is due — see ERRORS.md rows 6–8).
pub fn run_one(libs: &Libs, which: Impl, w: c_int, h: c_int, pixels: &[CpPixel]) -> RunOutcome {
    let mut buf: Vec<CpPixel> = Vec::with_capacity(pixels.len() + 2 * GUARD_LEN);
    buf.extend(std::iter::repeat_n(GUARD, GUARD_LEN));
    buf.extend_from_slice(pixels);
    buf.extend(std::iter::repeat_n(GUARD, GUARD_LEN));

    let base = unsafe { buf.as_mut_ptr().add(GUARD_LEN) };
    let mut img = CpImage { w, h, pix: base };

    match which {
        Impl::C => unsafe { (libs.c_flip())(&mut img) },
        Impl::Rust => unsafe { (libs.rust_flip())(&mut img) },
    }

    RunOutcome {
        pixels: buf[GUARD_LEN..GUARD_LEN + pixels.len()].to_vec(),
        guard_lo: buf[..GUARD_LEN].to_vec(),
        guard_hi: buf[GUARD_LEN + pixels.len()..].to_vec(),
        w_after: img.w,
        h_after: img.h,
        pix_unchanged: std::ptr::eq(img.pix, base),
    }
}

/// Run one implementation with `pix == NULL` (valid only when no dereference is
/// due — ERRORS.md rows 9–11).
pub fn run_one_null_pix(libs: &Libs, which: Impl, w: c_int, h: c_int) -> (c_int, c_int, bool) {
    let mut img = CpImage {
        w,
        h,
        pix: std::ptr::null_mut(),
    };
    match which {
        Impl::C => unsafe { (libs.c_flip())(&mut img) },
        Impl::Rust => unsafe { (libs.rust_flip())(&mut img) },
    }
    (img.w, img.h, img.pix.is_null())
}

/// THE differential assertion: both `.so`s, same input, byte-identical output.
#[track_caller]
pub fn assert_same(libs: &Libs, w: c_int, h: c_int, pixels: &[CpPixel], ctx: &str) -> RunOutcome {
    let c = run_one(libs, Impl::C, w, h, pixels);
    let r = run_one(libs, Impl::Rust, w, h, pixels);

    assert_eq!(
        c.pixels, r.pixels,
        "PIXEL MISMATCH ({ctx}) w={w} h={h} len={}\n  C   = {:?}\n  Rust= {:?}",
        pixels.len(),
        c.pixels,
        r.pixels
    );
    assert_eq!(
        c.guard_lo, r.guard_lo,
        "LOW GUARD MISMATCH ({ctx}) w={w} h={h}"
    );
    assert_eq!(
        c.guard_hi, r.guard_hi,
        "HIGH GUARD MISMATCH ({ctx}) w={w} h={h}"
    );
    assert_eq!(
        (c.w_after, c.h_after, c.pix_unchanged),
        (r.w_after, r.h_after, r.pix_unchanged),
        "cp_image_t FIELD MISMATCH ({ctx}) w={w} h={h}"
    );

    // Neither implementation may write outside the logical buffer.
    let intact = vec![GUARD; GUARD_LEN];
    assert_eq!(c.guard_lo, intact, "C wrote below the buffer ({ctx}) w={w} h={h}");
    assert_eq!(c.guard_hi, intact, "C wrote above the buffer ({ctx}) w={w} h={h}");
    assert_eq!(r.guard_lo, intact, "Rust wrote below the buffer ({ctx}) w={w} h={h}");
    assert_eq!(r.guard_hi, intact, "Rust wrote above the buffer ({ctx}) w={w} h={h}");

    // Neither implementation may mutate the descriptor.
    assert_eq!((c.w_after, c.h_after), (w, h), "C mutated img->w/h ({ctx})");
    assert_eq!((r.w_after, r.h_after), (w, h), "Rust mutated img->w/h ({ctx})");
    assert!(c.pix_unchanged, "C mutated img->pix ({ctx})");
    assert!(r.pix_unchanged, "Rust mutated img->pix ({ctx})");

    c
}

/// Independent reference model of the C loop, used to confirm the tests are
/// actually observing a flip (guards against "both are no-ops" false passes).
pub fn model(w: c_int, h: c_int, pixels: &[CpPixel]) -> Vec<CpPixel> {
    let mut out = pixels.to_vec();
    let flips = h / 2;
    for i in 0..flips.max(0) {
        for j in 0..w.max(0) {
            let a = (w as isize * i as isize + j as isize) as usize;
            let b = (w as isize * (h as isize - i as isize - 1) + j as isize) as usize;
            out.swap(a, b);
        }
    }
    out
}

/// Shapes used by the data-pattern rows.
pub const REPRESENTATIVE_SHAPES: &[(c_int, c_int)] = &[
    (1, 1),
    (1, 2),
    (1, 3),
    (2, 2),
    (3, 5),
    (8, 1),
    (8, 2),
    (8, 3),
    (8, 4),
    (37, 6),
];
