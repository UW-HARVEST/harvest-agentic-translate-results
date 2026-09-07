//! Shared differential-testing harness.
//!
//! Both libraries are loaded as shared objects via `libloading` and called only
//! through their exported `premultiply` symbol. The Rust implementation is never
//! called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Mirrors `cp_pixel_t` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Mirrors `cp_image_t` from `c_src/include/lib.h`.
#[repr(C)]
pub struct CpImage {
    pub w: i32,
    pub h: i32,
    pub pix: *mut CpPixel,
}

pub const PIXEL_SIZE: usize = 4;

/// `sizeof`/`align_of` sanity: the Rust view of the structs must match the C ABI.
pub fn assert_abi_layout() {
    assert_eq!(std::mem::size_of::<CpPixel>(), 4, "sizeof(cp_pixel_t)");
    assert_eq!(std::mem::align_of::<CpPixel>(), 1, "alignof(cp_pixel_t)");
    assert_eq!(std::mem::size_of::<CpImage>(), 16, "sizeof(cp_image_t)");
    assert_eq!(std::mem::align_of::<CpImage>(), 8, "alignof(cp_image_t)");
}

type PremultiplyFn = unsafe extern "C" fn(*mut CpImage);

/// A dlopen'd library plus its resolved `premultiply` symbol.
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    premultiply: PremultiplyFn,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {name} at {}: {e}", path.display()));
        let premultiply: PremultiplyFn = unsafe {
            let sym: Symbol<PremultiplyFn> = lib
                .get(b"premultiply\0")
                .unwrap_or_else(|e| panic!("dlsym premultiply in {name}: {e}"));
            *sym
        };
        Lib {
            name,
            path,
            _lib: lib,
            premultiply,
        }
    }

    /// Call `premultiply` through the shared-object export.
    ///
    /// # Safety
    /// `img` must satisfy whatever the C requires for the given `w`/`h`/`pix`.
    pub unsafe fn premultiply(&self, img: *mut CpImage) {
        (self.premultiply)(img)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C `.so`. Its file name is derived from the parent directory name
/// by `CMakeLists.txt`, so glob the build directory rather than hardcoding it.
fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "so").unwrap_or(false) && p.is_file() {
            found.push(p);
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build_dir.display(),
        found
    );
    found.pop().unwrap()
}

/// Locate the Rust cdylib.
///
/// `DIFF_RUST_SO` overrides the choice so the verification script can run the
/// whole suite against both the debug and the release cdylib (the release
/// profile sets `panic = "abort"`, so it is a genuinely different artifact).
/// Otherwise the most recently built profile wins.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DIFF_RUST_SO does not exist: {}", p.display());
        return p;
    }
    let base = manifest_dir().join("target");
    let candidates = [
        base.join("release/libpremultiply_lib.so"),
        base.join("debug/libpremultiply_lib.so"),
    ];
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for c in candidates.iter() {
        if let Ok(md) = std::fs::metadata(c) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, c.clone()));
            }
        }
    }
    match best {
        Some((_, p)) => p,
        None => panic!(
            "Rust cdylib not found in {}. Build it first: cargo build && cargo build --release",
            base.display()
        ),
    }
}

/// The C and Rust libraries, both dlopen'd.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

/// Open both libraries. Called once per test (dlopen is refcounted and cheap).
pub fn load_pair() -> Pair {
    assert_abi_layout();
    let c_path = c_so_path();
    let rust_path = rust_so_path();
    assert_ne!(c_path, rust_path);
    Pair {
        c: Lib::open("C", c_path),
        rust: Lib::open("Rust", rust_path),
    }
}

/// Deterministic PRNG (xorshift64*). Fixed seed => reproducible test inputs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

/// Byte value used to poison guard regions around the live span.
pub const POISON: u8 = 0xA5;

/// A test arena: `pre_pad` poison bytes, then `live` bytes, then `post_pad`
/// poison bytes. `pix` points at the start of the live region (plus `offset`).
pub struct Arena {
    pub buf: Vec<u8>,
    pub pix_off: usize,
}

impl Arena {
    /// `live` live bytes with `pad` poison bytes on each side, and the pixel
    /// pointer placed `offset` bytes into the live region (to test misalignment).
    pub fn new(live: usize, pad: usize, offset: usize) -> Arena {
        let mut buf = vec![POISON; pad + live + offset + pad];
        // Live span (including the offset shift) starts at `pad`.
        for b in buf[pad..pad + live + offset].iter_mut() {
            *b = 0;
        }
        Arena {
            buf,
            pix_off: pad + offset,
        }
    }

    pub fn pix_ptr(&mut self) -> *mut CpPixel {
        unsafe { self.buf.as_mut_ptr().add(self.pix_off) as *mut CpPixel }
    }
}

/// Run both libraries on identical copies of `data` with the given `w`/`h` and
/// assert the resulting buffers are byte-identical.
///
/// `pad` poison bytes surround the data on both sides and are compared too, so
/// any out-of-span write is caught. `offset` shifts the pixel pointer to test
/// byte-misaligned buffers.
///
/// Returns the (identical) output bytes of the live span for further inspection.
pub fn diff_run_ex(
    pair: &Pair,
    w: i32,
    h: i32,
    data: &[u8],
    pad: usize,
    offset: usize,
    ctx: &str,
) -> Vec<u8> {
    let live = data.len();

    let mk = || {
        let mut a = Arena::new(live, pad, offset);
        a.buf[a.pix_off..a.pix_off + live].copy_from_slice(data);
        a
    };
    let mut c_arena = mk();
    let mut rust_arena = mk();
    assert_eq!(c_arena.buf, rust_arena.buf, "[{ctx}] arena setup mismatch");

    let mut c_img = CpImage {
        w,
        h,
        pix: c_arena.pix_ptr(),
    };
    let mut rust_img = CpImage {
        w,
        h,
        pix: rust_arena.pix_ptr(),
    };

    unsafe {
        pair.c.premultiply(&mut c_img);
        pair.rust.premultiply(&mut rust_img);
    }

    // Struct fields must be untouched by both (the C only reads them).
    assert_eq!(
        (c_img.w, c_img.h),
        (w, h),
        "[{ctx}] C mutated cp_image_t fields"
    );
    assert_eq!(
        (rust_img.w, rust_img.h),
        (w, h),
        "[{ctx}] Rust mutated cp_image_t fields"
    );

    if c_arena.buf != rust_arena.buf {
        let first = c_arena
            .buf
            .iter()
            .zip(rust_arena.buf.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        let lo = first.saturating_sub(8);
        let hi = (first + 8).min(c_arena.buf.len());
        let pix_rel = first as isize - c_arena.pix_off as isize;
        panic!(
            "[{ctx}] DIVERGENCE w={w} h={h} live={live} offset={offset}\n\
             expected_iterations={}\n\
             first differing arena byte index {first} (pix starts at {}, \
             byte {pix_rel} relative to pix)\n\
             arena[{lo}..{hi}]  C: {:02x?}\n\
             arena[{lo}..{hi}]  R: {:02x?}",
            expected_iterations(w, h),
            c_arena.pix_off,
            &c_arena.buf[lo..hi],
            &rust_arena.buf[lo..hi],
        );
    }

    // Guard regions must still be poison in both.
    let pre = &c_arena.buf[..pad];
    let post = &c_arena.buf[pad + live + offset..];
    assert!(
        pre.iter().all(|&b| b == POISON),
        "[{ctx}] pre-guard clobbered (w={w} h={h})"
    );
    assert!(
        post.iter().all(|&b| b == POISON),
        "[{ctx}] post-guard clobbered (w={w} h={h})"
    );

    c_arena.buf[c_arena.pix_off..c_arena.pix_off + live].to_vec()
}

/// `diff_run_ex` with a default 64-byte guard and no misalignment.
pub fn diff_run(pair: &Pair, w: i32, h: i32, data: &[u8], ctx: &str) -> Vec<u8> {
    diff_run_ex(pair, w, h, data, 64, 0, ctx)
}

/// Number of loop iterations the C performs, per the disassembly:
/// `stride = wrap32(w << 2)`, `end = wrap32(stride * h)`, `i < end` signed.
pub fn expected_iterations(w: i32, h: i32) -> u32 {
    let stride = w.wrapping_mul(4);
    let end = stride.wrapping_mul(h);
    if end > 0 {
        (end as u32) / 4
    } else {
        0
    }
}

/// The full set of boundary dimension values swept by the generic tests: every
/// interesting `int` boundary and one/two steps past each, in both fields.
pub fn boundary_values() -> Vec<i32> {
    let mut values: Vec<i32> = Vec::new();
    for base in [
        i32::MIN,
        -0x4000_0000i32,
        -0x2000_0000,
        -1024,
        -4,
        -1,
        0,
        1,
        4,
        1024,
        0x2000_0000,
        0x4000_0000,
        i32::MAX,
    ] {
        for delta in [-2i32, -1, 0, 1, 2] {
            values.push(base.wrapping_add(delta));
        }
    }
    values.sort_unstable();
    values.dedup();
    values
}

/// Trip counts at or below this are checked in-process by the generic sweep;
/// larger ones are covered by `phase_c_large.rs`.
pub const INPROC_MAX_PX: usize = 1 << 20;

/// Reference model of one pixel's transform, mirroring the C float pipeline.
pub fn model_pixel(px: [u8; 4]) -> [u8; 4] {
    let a = f32::from(px[3]) / 255.0f32;
    let r = (f32::from(px[0]) / 255.0f32) * a;
    let g = (f32::from(px[1]) / 255.0f32) * a;
    let b = (f32::from(px[2]) / 255.0f32) * a;
    [
        (r * 255.0f32) as i32 as u8,
        (g * 255.0f32) as i32 as u8,
        (b * 255.0f32) as i32 as u8,
        px[3], // alpha is read but never written back by the C
    ]
}

/// Helper: build `n` random pixels as a flat byte vector.
pub fn random_bytes(rng: &mut Rng, n_pixels: usize) -> Vec<u8> {
    let mut v = vec![0u8; n_pixels * PIXEL_SIZE];
    rng.fill(&mut v);
    v
}

/// Path helpers exposed for the symbol-parity test.
pub fn paths() -> (PathBuf, PathBuf) {
    (c_so_path(), rust_so_path())
}

pub fn crate_root() -> PathBuf {
    manifest_dir()
}

pub fn exists(p: &Path) -> bool {
    p.exists()
}
