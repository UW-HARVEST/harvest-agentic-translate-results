//! Shared differential-test harness.
//!
//! Both the C reference library and the Rust translation are loaded as shared
//! objects with `libloading` and driven **only** through their exported
//! `premultiply` symbol. No Rust function is ever called directly, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirrors of `c_src/include/lib.h`
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// ```c
/// typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

pub type PremultiplyFn = unsafe extern "C" fn(*mut CpImage);

/// `end` as the C computes it: `(int)stride * h` where
/// `stride = (int)(w * sizeof(cp_pixel_t))`, all with 32-bit wrapping
/// (GCC `-O0` emits `shl $0x2` then `imul`, both 32-bit).
pub fn c_end(w: i32, h: i32) -> i32 {
    w.wrapping_mul(4).wrapping_mul(h)
}

/// Number of loop iterations (pixels touched) the C performs.
pub fn c_iterations(w: i32, h: i32) -> u64 {
    let end = c_end(w, h);
    if end <= 0 {
        0
    } else {
        // `end` is always a multiple of 4 (see CONFIGS.md), so this is exact.
        (end as u64 + 3) / 4
    }
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch_dir() -> PathBuf {
    let d = crate_root().join("target").join("difftest");
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// Path to the C reference `.so`.
///
/// Prefers the CMake build described in the task
/// (`c_src/build/libtranslated_rust.so`); falls back to compiling `c_src`
/// with `cc -shared -fPIC` (the exact flags CMake used: `C_FLAGS = -fPIC`,
/// no optimisation level, i.e. `-O0`) so the suite is self-contained.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("PREMULT_C_SO") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    for name in ["libtranslated_rust.so", "libpremultiply_lib.so", "liblib.so"] {
        let p = root.join("c_src").join("build").join(name);
        if p.is_file() {
            return p;
        }
    }
    // Fallback build.
    let out = scratch_dir().join("libc_reference.so");
    let src = root.join("c_src").join("src").join("lib.c");
    let stale = match (mtime(&out), mtime(&src)) {
        (Some(o), Some(s)) => o < s,
        _ => true,
    };
    if stale {
        let st = Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-I")
            .arg(root.join("c_src").join("include"))
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .status()
            .expect("spawn cc to build the C reference library");
        assert!(st.success(), "cc failed to build the C reference library");
    }
    out
}

/// Path to the Rust translation's `.so`.
///
/// Prefers the artifact Cargo produced (`cargo build --lib`). Because the
/// crate is `crate-type = ["cdylib"]`, a bare `cargo test` does not build it;
/// in that case the harness compiles `src/lib.rs` itself with `rustc
/// --crate-type cdylib` (the crate has no non-`std` dependencies, so this is
/// byte-for-byte the same translation unit).
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("PREMULT_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("target"));
    let src = root.join("src").join("lib.rs");
    let src_t = mtime(&src);

    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["debug", "release"] {
        let p = target.join(profile).join("libpremultiply_lib.so");
        if let Some(t) = mtime(&p) {
            if src_t.map(|s| t >= s).unwrap_or(false) {
                if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                    best = Some((t, p));
                }
            }
        }
    }
    if let Some((_, p)) = best {
        return p;
    }

    // Fallback: compile the cdylib directly with rustc.
    let out = scratch_dir().join("librust_translation.so");
    let stale = match (mtime(&out), src_t) {
        (Some(o), Some(s)) => o < s,
        _ => true,
    };
    if stale {
        let st = Command::new("rustc")
            .arg("--edition")
            .arg("2021")
            .arg("--crate-type")
            .arg("cdylib")
            .arg("--crate-name")
            .arg("premultiply_lib")
            .arg("-C")
            .arg("debug-assertions=on")
            .arg("-C")
            .arg("overflow-checks=on")
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .status()
            .expect("spawn rustc to build the Rust cdylib");
        assert!(st.success(), "rustc failed to build the Rust cdylib");
    }
    out
}

struct Loaded {
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    c: PremultiplyFn,
    rust: PremultiplyFn,
}

// SAFETY: the libraries are leaked into a `'static` `OnceLock` and never
// unloaded; the function pointers stay valid for the whole process lifetime.
unsafe impl Send for Loaded {}
unsafe impl Sync for Loaded {}

static LOADED: OnceLock<Loaded> = OnceLock::new();

fn loaded() -> &'static Loaded {
    LOADED.get_or_init(|| unsafe {
        let cp = c_so_path();
        let rp = rust_so_path();
        let c_lib = libloading::Library::new(&cp)
            .unwrap_or_else(|e| panic!("failed to dlopen C .so {}: {e}", cp.display()));
        let rust_lib = libloading::Library::new(&rp)
            .unwrap_or_else(|e| panic!("failed to dlopen Rust .so {}: {e}", rp.display()));
        let c_sym: libloading::Symbol<PremultiplyFn> = c_lib
            .get(b"premultiply\0")
            .expect("C .so does not export `premultiply`");
        let rust_sym: libloading::Symbol<PremultiplyFn> = rust_lib
            .get(b"premultiply\0")
            .expect("Rust .so does not export `premultiply`");
        let c = *c_sym;
        let rust = *rust_sym;
        Loaded {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    })
}

/// `premultiply` as exported by the **C** `.so`.
pub fn c_fn() -> PremultiplyFn {
    loaded().c
}

/// `premultiply` as exported by the **Rust** `.so`.
pub fn rust_fn() -> PremultiplyFn {
    loaded().rust
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

pub struct Rng(u64);

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
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn i32(&mut self) -> i32 {
        self.u32() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.u8();
        }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// 8-byte aligned scratch buffer with guard regions
// ---------------------------------------------------------------------------

/// Byte buffer whose base pointer is 8-byte aligned (so `pix` can be placed at
/// a controlled alignment by adding a small offset).
pub struct AlignedBuf {
    raw: Vec<u64>,
    len: usize,
}

impl AlignedBuf {
    pub fn new(len: usize) -> Self {
        AlignedBuf {
            raw: vec![0u64; len / 8 + 2],
            len,
        }
    }
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut b = Self::new(bytes.len());
        b.as_mut_slice().copy_from_slice(bytes);
        b
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.raw.as_ptr() as *const u8
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.raw.as_mut_ptr() as *mut u8
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len) }
    }
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.raw.as_mut_ptr() as *mut u8, self.len) }
    }
}

// ---------------------------------------------------------------------------
// The differential primitives
// ---------------------------------------------------------------------------

/// Result of one differential invocation.
pub struct DiffOutcome {
    pub c_bytes: Vec<u8>,
    pub rust_bytes: Vec<u8>,
    pub c_img: CpImage,
    pub rust_img: CpImage,
}

/// Calls the C and the Rust `premultiply` on two identical copies of `initial`,
/// with `img.pix` pointing at `base + pix_offset`.
pub fn call_both(w: i32, h: i32, initial: &[u8], pix_offset: usize) -> DiffOutcome {
    assert!(pix_offset <= initial.len());
    let mut cb = AlignedBuf::from_bytes(initial);
    let mut rb = AlignedBuf::from_bytes(initial);

    let c_img = unsafe {
        let pix = cb.as_mut_ptr().add(pix_offset) as *mut CpPixel;
        let mut img = CpImage { w, h, pix };
        (c_fn())(&mut img);
        img
    };
    let rust_img = unsafe {
        let pix = rb.as_mut_ptr().add(pix_offset) as *mut CpPixel;
        let mut img = CpImage { w, h, pix };
        (rust_fn())(&mut img);
        img
    };

    DiffOutcome {
        c_bytes: cb.as_slice().to_vec(),
        rust_bytes: rb.as_slice().to_vec(),
        c_img,
        rust_img,
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    if a.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    a.iter().zip(b.iter()).position(|(x, y)| x != y)
}

/// Runs both libraries and asserts byte-identical buffers **and** identical
/// (unmodified) `cp_image_t` structs.
pub fn assert_same(label: &str, w: i32, h: i32, initial: &[u8], pix_offset: usize) -> Vec<u8> {
    let o = call_both(w, h, initial, pix_offset);

    if let Some(i) = first_diff(&o.c_bytes, &o.rust_bytes) {
        let px = if i >= pix_offset {
            (i - pix_offset) / 4
        } else {
            usize::MAX
        };
        let lo = i.saturating_sub(4);
        let hi = (i + 8).min(o.c_bytes.len());
        panic!(
            "DIVERGENCE in `{label}`\n  \
             w={w} (0x{w:08x})  h={h} (0x{h:08x})  pix_offset={pix_offset}\n  \
             end=(int)stride*h = {end} (0x{end:08x})  iterations={iters}\n  \
             first differing byte index = {i} (pixel #{px}, channel {chan})\n  \
             input [{lo}..{hi}) = {inp:?}\n  \
             C     [{lo}..{hi}) = {cb:?}\n  \
             Rust  [{lo}..{hi}) = {rb:?}",
            end = c_end(w, h),
            iters = c_iterations(w, h),
            chan = if i >= pix_offset { (i - pix_offset) % 4 } else { 0 },
            inp = &initial[lo..hi],
            cb = &o.c_bytes[lo..hi],
            rb = &o.rust_bytes[lo..hi],
        );
    }

    assert_eq!(
        (o.c_img.w, o.c_img.h),
        (o.rust_img.w, o.rust_img.h),
        "`{label}`: cp_image_t w/h fields diverged (w={w}, h={h})"
    );
    assert_eq!(
        (o.c_img.w, o.c_img.h),
        (w, h),
        "`{label}`: C mutated the cp_image_t struct (it must not)"
    );

    o.c_bytes
}

/// Asserts that both libraries leave the buffer completely untouched.
pub fn assert_noop(label: &str, w: i32, h: i32, initial: &[u8], pix_offset: usize) {
    let out = assert_same(label, w, h, initial, pix_offset);
    assert_eq!(
        out,
        initial.to_vec(),
        "`{label}`: expected a no-op for w={w} (0x{w:08x}) h={h} (0x{h:08x}) \
         end={} but the buffer changed",
        c_end(w, h)
    );
}

/// Runs one `(w, h)` combination through both libraries with a correctly sized
/// guarded buffer.
///
/// * asserts C == Rust byte-for-byte,
/// * asserts neither library touched the guard regions,
/// * asserts a bitwise no-op whenever `end <= 0`.
///
/// Returns `None` if the combination was skipped because it would need a buffer
/// larger than [`MAX_BYTES`], otherwise `Some(did_modify_the_buffer)`.
pub fn run_combo(label: &str, w: i32, h: i32, guard: usize, rng: &mut Rng) -> Option<bool> {
    let end = c_end(w, h);
    let px = if end > 0 { (end / 4) as usize } else { 0 };
    let n = 4 * px;
    if n + 2 * guard > MAX_BYTES {
        return None;
    }
    let mut buf = vec![0xA5u8; guard * 2 + n.max(4)];
    rng.fill(&mut buf[guard..guard + n]);
    let out = assert_same(label, w, h, &buf, guard);
    assert_eq!(
        &out[..guard],
        &buf[..guard],
        "{label}: leading guard modified (w={w},h={h},end={end})"
    );
    assert_eq!(
        &out[guard + n..],
        &buf[guard + n..],
        "{label}: trailing guard modified (w={w},h={h},end={end})"
    );
    if end <= 0 {
        assert_eq!(
            out, buf,
            "{label}: expected a bitwise no-op for w={w},h={h},end={end}"
        );
    }
    // Alpha is never written.
    for k in 0..px {
        assert_eq!(
            out[guard + 4 * k + 3],
            buf[guard + 4 * k + 3],
            "{label}: alpha of pixel {k} was written (w={w},h={h})"
        );
    }
    Some(out != buf)
}

/// Reference model of the C loop, used only to cross-check that a test row is
/// really exercising the code path it claims (never used as the oracle).
pub fn model(w: i32, h: i32, data: &mut [u8]) {
    let end = c_end(w, h);
    let mut i: i32 = 0;
    while i < end {
        let k = i as usize;
        let a = f32::from(data[k + 3]) / 255.0f32;
        let r = f32::from(data[k]) / 255.0f32 * a;
        let g = f32::from(data[k + 1]) / 255.0f32 * a;
        let b = f32::from(data[k + 2]) / 255.0f32 * a;
        data[k] = (r * 255.0f32) as u8;
        data[k + 1] = (g * 255.0f32) as u8;
        data[k + 2] = (b * 255.0f32) as u8;
        i = i.wrapping_add(4);
    }
}

// ---------------------------------------------------------------------------
// Interesting-value pools (mechanically derived from CONFIGS.md axes A/B)
// ---------------------------------------------------------------------------

pub const DIM_SMALL: &[i32] = &[
    1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255, 256, 257, 1023, 1024,
];

pub const DIM_SPECIAL: &[i32] = &[
    0,
    1,
    2,
    3,
    -1,
    -2,
    -3,
    -17,
    -64,
    -1000,
    1000,
    0x0800_0000,
    0x1000_0000,
    0x2000_0000,
    0x3FFF_FFFF,
    0x4000_0000,
    0x4000_0001,
    0x4000_0002,
    0x4000_0401,
    0x7FFF_FFFF,      // INT_MAX
    -0x4000_0000,
    i32::MIN,
];

/// Largest buffer (in bytes) a row is allowed to allocate. Combinations whose
/// `end` exceeds this are skipped and reported by the caller.
pub const MAX_BYTES: usize = 1 << 23; // 8 MiB
