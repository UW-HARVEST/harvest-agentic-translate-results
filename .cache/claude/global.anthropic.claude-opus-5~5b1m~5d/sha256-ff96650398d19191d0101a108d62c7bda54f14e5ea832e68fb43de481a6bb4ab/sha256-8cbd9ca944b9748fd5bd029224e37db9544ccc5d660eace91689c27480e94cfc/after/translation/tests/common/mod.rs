//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects via `libloading` and
//! called through their exported `update_frame_header` symbol. The Rust
//! implementation is NEVER called directly as a Rust function — this way the
//! `#[no_mangle]` / `extern "C"` wrapper and the `#[repr(C)]` struct layout are
//! part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The C `struct tflac`. Verified against a compiled C probe:
/// `size=24 align=4 off=0 4 8 12 16 20`.
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

pub const TFLAC_SIZE: usize = 24;

/// The struct as a raw 24-byte image, so that padding bytes are compared too.
#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RawTflac(pub [u8; TFLAC_SIZE]);

impl RawTflac {
    pub fn from_fields(t: &Tflac) -> Self {
        let mut b = [0u8; TFLAC_SIZE];
        b[0..4].copy_from_slice(&t.samplerate.to_ne_bytes());
        b[4..8].copy_from_slice(&t.channels.to_ne_bytes());
        b[8..12].copy_from_slice(&t.bitdepth.to_ne_bytes());
        b[12] = t.channel_mode;
        b[16..20].copy_from_slice(&t.frame_header.to_ne_bytes());
        b[20..24].copy_from_slice(&t.cur_blocksize.to_ne_bytes());
        RawTflac(b)
    }

    pub fn frame_header(&self) -> u32 {
        u32::from_ne_bytes(self.0[16..20].try_into().unwrap())
    }
}

impl std::fmt::Debug for RawTflac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RawTflac{{ bytes: {:02x?}, frame_header: 0x{:08X} }}", self.0, self.frame_header())
    }
}

type UpdateFn = unsafe extern "C" fn(*mut Tflac);

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    func: UpdateFn,
}

impl Impl {
    /// Call `update_frame_header` on a whole 24-byte struct image and return the
    /// resulting image (including padding bytes).
    pub fn call_raw(&self, input: &RawTflac) -> RawTflac {
        let mut buf = *input;
        unsafe { (self.func)(buf.0.as_mut_ptr() as *mut Tflac) };
        buf
    }

    /// Call with a field-wise struct and return the resulting `frame_header`.
    pub fn call(&self, t: &Tflac) -> u32 {
        self.call_raw(&RawTflac::from_fields(t)).frame_header()
    }

    /// Call on a pointer into a caller-owned buffer (for the guard-byte test).
    pub unsafe fn call_ptr(&self, p: *mut u8) {
        unsafe { (self.func)(p as *mut Tflac) }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.remove(0)
}

fn find_rust_so() -> PathBuf {
    // `RUST_SO=<path>` lets the same suite be run against a differently built
    // Rust cdylib (notably the debug profile, where Rust's arithmetic overflow
    // checks are ON — the C wraps silently, so a `-` instead of `wrapping_sub`
    // would panic there but pass in release).
    if let Some(p) = std::env::var_os("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO={} does not exist", p.display());
        return p;
    }
    // Prefer the profile the tests were built with, then fall back.
    let target = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libupdate_frame_header_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust .so found under {}. Build it first: cargo build --release",
        target.display()
    );
}

unsafe fn load(name: &'static str, path: &Path) -> Impl {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
    let func: UpdateFn = unsafe {
        let s: Symbol<UpdateFn> = lib
            .get(b"update_frame_header\0")
            .unwrap_or_else(|e| panic!("{} does not export update_frame_header: {e}", path.display()));
        *s
    };
    Impl { name, _lib: lib, func }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

/// Both libraries, loaded once per test process.
pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| unsafe {
        Pair { c: load("C", &find_c_so()), rust: load("Rust", &find_rust_so()) }
    })
}

/// Deterministic PRNG (xorshift64*) so every row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        let span = (hi as u64) - (lo as u64) + 1;
        lo.wrapping_add((self.next_u64() % span) as u32)
    }
    /// A u32 biased towards interesting magnitudes as well as the full range.
    pub fn spicy_u32(&mut self) -> u32 {
        match self.next_u64() % 8 {
            0 => 0,
            1 => 1,
            2 => u32::MAX,
            3 => self.range(0, 64),
            4 => self.range(0, 65_600),
            5 => 1u32 << (self.next_u64() % 32) as u32,
            6 => (1u32 << (self.next_u64() % 32) as u32).wrapping_sub(1),
            _ => self.next_u32(),
        }
    }
    pub fn random_tflac(&mut self) -> Tflac {
        Tflac {
            samplerate: self.spicy_u32(),
            channels: self.spicy_u32(),
            bitdepth: self.spicy_u32(),
            channel_mode: self.next_u8(),
            frame_header: self.spicy_u32(),
            cur_blocksize: self.spicy_u32(),
        }
    }
    /// Randomized struct whose channel-assignment contribution to
    /// `frame_header` is exactly zero: `channel_mode == 0` (INDEPENDENT) and
    /// `channels == 1`, so the C ORs `(1 - 1) << 4 == 0`. This leaves the
    /// blocksize nibble (bits 12..15) and samplerate nibble (bits 8..11)
    /// readable in isolation, which the Phase C nibble-pinning tests need —
    /// with a random `channels` in mode 0, `(channels-1) << 4` legitimately
    /// sets bits 4..31 and swamps both nibbles.
    pub fn random_tflac_neutral_channels(&mut self) -> Tflac {
        let mut t = self.random_tflac();
        t.channel_mode = 0;
        t.channels = 1;
        t
    }

    /// A fully random 24-byte struct image, padding bytes included.
    pub fn random_raw(&mut self) -> RawTflac {
        let mut b = [0u8; TFLAC_SIZE];
        for x in b.iter_mut() {
            *x = self.next_u8();
        }
        RawTflac(b)
    }
}

/// Assert that C and Rust produce byte-identical 24-byte struct images.
#[track_caller]
pub fn assert_same_raw(row: &str, input: &RawTflac) {
    let p = pair();
    let c_out = p.c.call_raw(input);
    let r_out = p.rust.call_raw(input);
    assert_eq!(
        c_out, r_out,
        "[{row}] divergence.\n  input: {input:?}\n  C:     {c_out:?}\n  Rust:  {r_out:?}"
    );
}

/// Assert C and Rust agree on a field-wise input (compares the whole image).
#[track_caller]
pub fn assert_same(row: &str, t: &Tflac) {
    assert_same_raw(row, &RawTflac::from_fields(t));
}

/// The 13 `cur_blocksize` values with an explicit `case` in the C.
pub const BLOCKSIZE_CASES: [u32; 13] =
    [192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768];

/// The 11 `samplerate` values with an explicit `case` in the C.
pub const SAMPLERATE_CASES: [u32; 11] =
    [882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000];

/// The 6 `bitdepth` values with an explicit `case` in the C.
pub const BITDEPTH_CASES: [u32; 6] = [8, 12, 16, 20, 24, 32];
