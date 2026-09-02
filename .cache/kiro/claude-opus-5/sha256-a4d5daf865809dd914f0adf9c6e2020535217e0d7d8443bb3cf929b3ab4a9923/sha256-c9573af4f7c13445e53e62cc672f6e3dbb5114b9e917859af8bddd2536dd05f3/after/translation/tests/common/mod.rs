//! Shared harness: loads BOTH shared objects via `libloading` and calls
//! `update_frame_header` through the FFI boundary only.
//!
//! Nothing here links the Rust crate directly — the Rust side is always reached
//! through `dlopen` + `dlsym` on `libupdate_frame_header_lib.so`, exactly as an
//! external C consumer would, so the `#[no_mangle] extern "C"` wrapper is under
//! test too.

#![allow(dead_code, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Byte-for-byte mirror of `struct tflac` from `c_src/include/lib.h`.
///
/// Layout verified against the C compiler (`offsetof`):
/// size 24, align 4, offsets 0/4/8/12/16/20.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Tflac {
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub frame_header: u32,
    pub cur_blocksize: u32,
}

pub type UpdateFn = unsafe extern "C" fn(*mut Tflac);

/// Repository root (the directory containing `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let name = "libupdate_frame_header_lib.so";
    // Prefer the profile the test binary itself was built with, so a debug test
    // run exercises the debug .so (arithmetic overflow checks enabled) rather
    // than silently falling back to a stale release artifact.
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in preferred {
        let p = base.join(profile).join(name);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust .so found under {}. Build it first: cargo build --release",
        base.display()
    );
}

/// Both implementations, kept alive for the duration of a test.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: UpdateFn,
    pub rust: UpdateFn,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_path = c_so_path();
            let r_path = rust_so_path();
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rust_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", r_path.display()));

            let c_sym: Symbol<UpdateFn> = c_lib
                .get(b"update_frame_header\0")
                .expect("C .so does not export update_frame_header");
            let r_sym: Symbol<UpdateFn> = rust_lib
                .get(b"update_frame_header\0")
                .expect("Rust .so does not export update_frame_header");

            let c = *c_sym;
            let rust = *r_sym;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Run one input through both `.so`s and compare the FULL 24-byte struct
    /// image, not just `frame_header`: this catches a Rust translation that
    /// clobbers a field the C leaves alone.
    #[track_caller]
    pub fn check(&self, input: Tflac) {
        let (c_out, r_out) = self.run(input);
        if c_out != r_out {
            panic!(
                "divergence\n  input:  {input:?}\n  C:      {c_out:?}  frame_header=0x{:08X}\n  Rust:   {r_out:?}  frame_header=0x{:08X}",
                c_out.frame_header, r_out.frame_header
            );
        }
        // Byte-level comparison of the raw struct image.
        let cb = as_bytes(&c_out);
        let rb = as_bytes(&r_out);
        assert_eq!(cb, rb, "raw struct bytes differ for input {input:?}");
    }

    pub fn run(&self, input: Tflac) -> (Tflac, Tflac) {
        let mut a = input;
        let mut b = input;
        unsafe {
            (self.c)(&mut a);
            (self.rust)(&mut b);
        }
        (a, b)
    }

    /// `frame_header` from the C side only (for asserting documented values).
    pub fn c_header(&self, input: Tflac) -> u32 {
        let mut a = input;
        unsafe { (self.c)(&mut a) };
        a.frame_header
    }
}

fn as_bytes(t: &Tflac) -> [u8; 24] {
    // SAFETY: Tflac is repr(C), 24 bytes, all fields are integers with no
    // niches; reading it as bytes is sound. Padding at offset 13..16 is
    // identical in both runs because both start from the same `input` copy.
    let mut out = [0u8; 24];
    unsafe {
        std::ptr::copy_nonoverlapping(
            (t as *const Tflac) as *const u8,
            out.as_mut_ptr(),
            std::mem::size_of::<Tflac>(),
        );
    }
    out
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed) — no external dev-dependency needed.
// ---------------------------------------------------------------------------

/// SplitMix64: tiny, high quality, fully reproducible.
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// The constant sets the C switches on (transcribed from c_src/src/lib.c).
// ---------------------------------------------------------------------------

pub const BLOCKSIZES: [u32; 13] = [
    192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];

pub const SAMPLERATES: [u32; 11] = [
    882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];

pub const BITDEPTHS: [u32; 6] = [8, 12, 16, 20, 24, 32];

/// `BASE = 0xFFF8U << 16`, written unconditionally by the C.
pub const BASE: u32 = 0xFFF8u32 << 16;

/// Fill the axes NOT under test with random-but-wide values.
pub fn rand_other_axes(rng: &mut Rng) -> Tflac {
    Tflac {
        samplerate: random_samplerate(rng),
        channels: random_channels(rng),
        bitdepth: random_bitdepth(rng),
        channel_mode: rng.next_u8(),
        // Pre-seed frame_header with garbage: the C overwrites it, so any Rust
        // version that OR-ed into the old value would be caught here.
        frame_header: rng.next_u32(),
        cur_blocksize: random_blocksize(rng),
    }
}

pub fn random_blocksize(rng: &mut Rng) -> u32 {
    match rng.next_u64() % 4 {
        0 => rng.pick(&BLOCKSIZES),
        1 => rng.range_u32(0, 256),
        2 => rng.range_u32(257, 100_000),
        _ => rng.next_u32(),
    }
}

pub fn random_samplerate(rng: &mut Rng) -> u32 {
    match rng.next_u64() % 6 {
        0 => rng.pick(&SAMPLERATES),
        1 => rng.range_u32(0, 300) * 1000,
        2 => rng.range_u32(0, 65535),
        3 => rng.range_u32(0, 70000) * 10,
        4 => rng.range_u32(60000, 70000),
        _ => rng.next_u32(),
    }
}

pub fn random_channels(rng: &mut Rng) -> u32 {
    match rng.next_u64() % 4 {
        0 => rng.range_u32(1, 8),
        1 => rng.range_u32(0, 20),
        2 => rng.next_u32(),
        _ => u32::MAX - rng.range_u32(0, 3),
    }
}

pub fn random_bitdepth(rng: &mut Rng) -> u32 {
    match rng.next_u64() % 3 {
        0 => rng.pick(&BITDEPTHS),
        1 => rng.range_u32(0, 40),
        _ => rng.next_u32(),
    }
}
