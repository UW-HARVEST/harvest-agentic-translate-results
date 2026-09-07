//! Shared differential-test harness.
//!
//! BOTH libraries are loaded as shared objects through `libloading` and called
//! only through their exported `flip_horizontal` symbol. The Rust
//! implementation is never called directly, so the `#[no_mangle] extern "C"`
//! wrapper is part of what is under test.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// FFI types (mirror c_src/include/lib.h exactly)
// ---------------------------------------------------------------------------

/// `typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

// ---------------------------------------------------------------------------
// Library discovery + loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir().parent().expect("crate has a parent dir").to_path_buf()
}

/// The C `.so` produced by CMake. Its name is derived from the parent directory
/// name, so it is discovered by globbing rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    let build_dir = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build_dir.display(),
        candidates
    );
    candidates.pop().unwrap()
}

/// The Rust `cdylib`.
///
/// `HARVEST_RUST_SO` overrides the path; that is used only by the negative
/// control (`tools/negative_control.sh`), which points the harness at a
/// deliberately mutated build to prove these tests can actually detect a
/// divergence.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARVEST_RUST_SO={} does not exist", p.display());
        return p;
    }
    let p = manifest_dir()
        .join("target")
        .join("release")
        .join("libflip_horizontal_lib.so");
    assert!(
        p.exists(),
        "{} not found; build it first with `cargo build --release`",
        p.display()
    );
    p
}

pub struct FlipLib {
    _lib: Library,
    flip: FlipFn,
    pub name: &'static str,
}

impl FlipLib {
    fn open(path: &Path, name: &'static str) -> FlipLib {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            let sym: Symbol<FlipFn> = lib.get(b"flip_horizontal\0").unwrap_or_else(|e| {
                panic!("dlsym(flip_horizontal) in {} failed: {e}", path.display())
            });
            let flip = *sym;
            FlipLib { _lib: lib, flip, name }
        }
    }

    /// Call the exported `flip_horizontal` through the FFI boundary.
    ///
    /// # Safety
    /// `img` must be whatever the caller intends to hand the C ABI (including
    /// deliberately invalid values in the error-path tests).
    pub unsafe fn flip(&self, img: *mut CpImage) {
        unsafe { (self.flip)(img) }
    }
}

pub struct Pair {
    pub c: FlipLib,
    pub rust: FlipLib,
}

/// Both libraries, loaded once per test process.
pub fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: FlipLib::open(&c_so_path(), "C"),
        rust: FlipLib::open(&rust_so_path(), "Rust"),
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_DEAD_BEEF;

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
    /// Uniform-ish in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

// ---------------------------------------------------------------------------
// A padded pixel arena: canary padding on both sides of the payload so that
// any write outside `[pix, pix + w*h)` is detected, and so that deliberately
// out-of-range accesses still land in memory this test owns.
// ---------------------------------------------------------------------------

pub const PAD: usize = 4096;

#[derive(Clone)]
pub struct Arena {
    /// Whole allocation: PAD bytes of canary, payload, PAD bytes of canary.
    pub bytes: Vec<u8>,
    /// Byte offset of `pix` inside `bytes`.
    pub pix_off: usize,
    /// Payload length in bytes.
    pub payload_len: usize,
}

impl Arena {
    /// `payload_len` bytes of pixel payload, `PAD` canary bytes either side.
    pub fn new(payload_len: usize, canary: u8) -> Arena {
        Arena::with_pad(payload_len, canary, PAD, 0)
    }

    /// `extra_shift` lets the payload start at an unaligned byte offset.
    pub fn with_pad(payload_len: usize, canary: u8, pad: usize, extra_shift: usize) -> Arena {
        let pix_off = pad + extra_shift;
        let bytes = vec![canary; pix_off + payload_len + pad];
        Arena { bytes, pix_off, payload_len }
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        let o = self.pix_off;
        let l = self.payload_len;
        &mut self.bytes[o..o + l]
    }

    pub fn pix_ptr(&mut self) -> *mut CpPixel {
        unsafe { self.bytes.as_mut_ptr().add(self.pix_off) }.cast::<CpPixel>()
    }
}

/// Outcome of one call, everything an external caller can observe.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Whole arena (payload + both canary regions) after the call.
    pub bytes: Vec<u8>,
    /// `img.w` after the call (C must not modify it).
    pub w: c_int,
    /// `img.h` after the call.
    pub h: c_int,
    /// Whether `img.pix` still points where it did before the call.
    pub pix_unchanged: bool,
}

/// Run one library on its own private copy of `arena`.
pub fn run(lib: &FlipLib, w: c_int, h: c_int, arena: &Arena) -> Outcome {
    let mut a = arena.clone();
    let pix = a.pix_ptr();
    let mut img = CpImage { w, h, pix };
    unsafe { lib.flip(&mut img) };
    Outcome { bytes: a.bytes, w: img.w, h: img.h, pix_unchanged: std::ptr::eq(img.pix, pix) }
}

/// Byte-for-byte differential assertion for one configuration.
#[track_caller]
pub fn assert_same(label: &str, w: c_int, h: c_int, arena: &Arena) -> Outcome {
    let p = libs();
    let out_c = run(&p.c, w, h, arena);
    let out_r = run(&p.rust, w, h, arena);

    if out_c.bytes != out_r.bytes {
        let n = out_c.bytes.len();
        let first = (0..n).find(|&i| out_c.bytes[i] != out_r.bytes[i]).unwrap();
        let diffs = (0..n).filter(|&i| out_c.bytes[i] != out_r.bytes[i]).count();
        panic!(
            "[{label}] buffer mismatch for w={w} h={h}: {diffs}/{n} bytes differ; \
             first at byte {first} (pix_off={}, payload_len={}): C=0x{:02x} Rust=0x{:02x}",
            arena.pix_off, arena.payload_len, out_c.bytes[first], out_r.bytes[first]
        );
    }
    assert_eq!(out_c.w, out_r.w, "[{label}] img.w diverged for w={w} h={h}");
    assert_eq!(out_c.h, out_r.h, "[{label}] img.h diverged for w={w} h={h}");
    assert_eq!(
        out_c.pix_unchanged, out_r.pix_unchanged,
        "[{label}] img.pix mutation diverged for w={w} h={h}"
    );
    out_c
}

/// Number of payload bytes a `w * h` image needs (saturating, non-negative).
pub fn payload_bytes(w: c_int, h: c_int) -> usize {
    let w = w.max(0) as usize;
    let h = h.max(0) as usize;
    w * h * 4
}

/// Build an arena for `w * h` filled with random pixel data.
pub fn random_arena(rng: &mut Rng, w: c_int, h: c_int) -> Arena {
    let mut a = Arena::new(payload_bytes(w, h), 0xC5);
    rng.fill(a.payload_mut());
    a
}

/// Reference model of the C loop, used only as an extra cross-check that the
/// *shared* observed behaviour is the intended row swap (not that C and Rust
/// merely agree on something wrong).
pub fn model_flip(w: c_int, h: c_int, payload: &mut [u8]) {
    if w <= 0 || h <= 0 {
        return;
    }
    let w = w as usize;
    let flips = (h / 2) as usize;
    for i in 0..flips {
        let a = w * i * 4;
        let b = w * (h as usize - i - 1) * 4;
        for j in 0..(w * 4) {
            payload.swap(a + j, b + j);
        }
    }
}
