//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and calls
//! `update_frame_header` across the FFI boundary in both. The Rust
//! implementation is NEVER called directly — always via its exported symbol,
//! so the `#[no_mangle]`/`extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::Command;

/// Raw, ABI-exact mirror of `struct tflac`.
///
/// Verified against the real C compiler: size 24, align 4, offsets
/// 0 / 4 / 8 / 12 / 16 / 20 (see `SYMBOLS.md`).
pub const TFLAC_SIZE: usize = 24;
pub const OFF_SAMPLERATE: usize = 0;
pub const OFF_CHANNELS: usize = 4;
pub const OFF_BITDEPTH: usize = 8;
pub const OFF_CHANNEL_MODE: usize = 12;
pub const OFF_FRAME_HEADER: usize = 16;
pub const OFF_CUR_BLOCKSIZE: usize = 20;

/// A `tflac` value held as raw bytes so that *both* libraries see literally the
/// same 24 bytes, padding included. Any layout disagreement therefore shows up
/// as an output divergence rather than being hidden by two separate structs.
#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Raw(pub [u8; TFLAC_SIZE]);

impl Raw {
    pub fn zeroed() -> Self {
        Raw([0u8; TFLAC_SIZE])
    }

    pub fn get_u32(&self, off: usize) -> u32 {
        u32::from_ne_bytes(self.0[off..off + 4].try_into().unwrap())
    }

    pub fn set_u32(&mut self, off: usize, v: u32) {
        self.0[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }

    pub fn samplerate(&self) -> u32 {
        self.get_u32(OFF_SAMPLERATE)
    }
    pub fn channels(&self) -> u32 {
        self.get_u32(OFF_CHANNELS)
    }
    pub fn bitdepth(&self) -> u32 {
        self.get_u32(OFF_BITDEPTH)
    }
    pub fn channel_mode(&self) -> u8 {
        self.0[OFF_CHANNEL_MODE]
    }
    pub fn frame_header(&self) -> u32 {
        self.get_u32(OFF_FRAME_HEADER)
    }
    pub fn cur_blocksize(&self) -> u32 {
        self.get_u32(OFF_CUR_BLOCKSIZE)
    }
}

impl std::fmt::Debug for Raw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tflac {{ samplerate: {}, channels: {}, bitdepth: {}, channel_mode: {}, \
             frame_header: 0x{:08X}, cur_blocksize: {}, pad13_15: {:02X?} }} bytes={:02X?}",
            self.samplerate(),
            self.channels(),
            self.bitdepth(),
            self.channel_mode(),
            self.frame_header(),
            self.cur_blocksize(),
            &self.0[13..16],
            self.0
        )
    }
}

/// Builder for a `tflac` input.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub frame_header_initial: u32,
    pub cur_blocksize: u32,
    pub padding: u8,
}

impl Default for Input {
    fn default() -> Self {
        Input {
            samplerate: 44100,
            channels: 2,
            bitdepth: 16,
            channel_mode: 0,
            frame_header_initial: 0,
            cur_blocksize: 4096,
            padding: 0,
        }
    }
}

impl Input {
    pub fn to_raw(self) -> Raw {
        let mut r = Raw::zeroed();
        r.set_u32(OFF_SAMPLERATE, self.samplerate);
        r.set_u32(OFF_CHANNELS, self.channels);
        r.set_u32(OFF_BITDEPTH, self.bitdepth);
        r.0[OFF_CHANNEL_MODE] = self.channel_mode;
        // tail padding, bytes 13..16
        r.0[13] = self.padding;
        r.0[14] = self.padding;
        r.0[15] = self.padding;
        r.set_u32(OFF_FRAME_HEADER, self.frame_header_initial);
        r.set_u32(OFF_CUR_BLOCKSIZE, self.cur_blocksize);
        r
    }
}

type UpdateFn = unsafe extern "C" fn(*mut u8);

pub struct Libs {
    _c: Library,
    _rust: Library,
    c_fn: UpdateFn,
    rust_fn: UpdateFn,
}

pub fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so under {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Prefer the release artifact; fall back to debug. Build if absent.
    let root = workspace_root().join("translation");
    for profile in ["release", "debug"] {
        let p = root
            .join("target")
            .join(profile)
            .join("libupdate_frame_header_lib.so");
        if p.exists() {
            return p;
        }
    }
    let ok = Command::new(env!("CARGO"))
        .args(["build", "--release"])
        .current_dir(&root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let p = root.join("target/release/libupdate_frame_header_lib.so");
    assert!(
        ok && p.exists(),
        "could not build/locate the Rust cdylib at {}",
        p.display()
    );
    p
}

impl Libs {
    pub fn load() -> Self {
        unsafe {
            let c = Library::new(find_c_so()).expect("load C .so");
            let rust = Library::new(find_rust_so()).expect("load Rust .so");
            let c_sym: Symbol<UpdateFn> = c
                .get(b"update_frame_header\0")
                .expect("C .so exports update_frame_header");
            let r_sym: Symbol<UpdateFn> = rust
                .get(b"update_frame_header\0")
                .expect("Rust .so exports update_frame_header");
            let c_fn = *c_sym;
            let rust_fn = *r_sym;
            Libs {
                _c: c,
                _rust: rust,
                c_fn,
                rust_fn,
            }
        }
    }

    pub fn call_c(&self, r: &mut Raw) {
        unsafe { (self.c_fn)(r.0.as_mut_ptr()) }
    }

    pub fn call_rust(&self, r: &mut Raw) {
        unsafe { (self.rust_fn)(r.0.as_mut_ptr()) }
    }

    /// Call the C export on an arbitrary raw pointer (used for array/stride and
    /// null-pointer tests).
    ///
    /// # Safety
    /// `p` must point at a valid, 4-byte-aligned 24-byte `tflac`, or be a
    /// pointer whose fault behaviour is what the caller intends to observe.
    pub unsafe fn call_c_raw(&self, p: *mut u8) {
        (self.c_fn)(p)
    }

    /// Call the Rust export on an arbitrary raw pointer. Same safety contract as
    /// [`Libs::call_c_raw`].
    pub unsafe fn call_rust_raw(&self, p: *mut u8) {
        (self.rust_fn)(p)
    }

    /// Run one input through both `.so`s and assert all 24 bytes agree.
    pub fn assert_same(&self, input: Input, ctx: &str) {
        let mut c = input.to_raw();
        let mut r = input.to_raw();
        self.call_c(&mut c);
        self.call_rust(&mut r);
        if c != r {
            panic!(
                "DIVERGENCE [{ctx}]\n  input: {input:?}\n  C   : {c:?}\n  RUST: {r:?}\n  \
                 first differing byte offset: {:?}",
                (0..TFLAC_SIZE).find(|&i| c.0[i] != r.0[i])
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi as u64) - (lo as u64) + 1;
        lo.wrapping_add((self.next_u64() % span) as u32)
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Axis class generators, derived from the C source's branch structure.
// ---------------------------------------------------------------------------

/// The 13 explicit `case` labels of the block-size `switch`.
pub const BLOCKSIZE_CASES: [u32; 13] = [
    192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];

/// The 11 explicit `case` labels of the sample-rate `switch`.
pub const SAMPLERATE_CASES: [u32; 11] = [
    882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];

/// The 6 explicit `case` labels of the bit-depth `switch`.
pub const BITDEPTH_CASES: [u32; 6] = [8, 12, 16, 20, 24, 32];

pub fn is_blocksize_case(v: u32) -> bool {
    BLOCKSIZE_CASES.contains(&v)
}
pub fn is_samplerate_case(v: u32) -> bool {
    SAMPLERATE_CASES.contains(&v)
}
pub fn is_bitdepth_case(v: u32) -> bool {
    BITDEPTH_CASES.contains(&v)
}

/// Axis BS: random value from *any* of the 15 block-size classes.
pub fn rand_blocksize_any(rng: &mut Rng) -> u32 {
    match rng.next_u64() % 3 {
        0 => *rng.pick(&BLOCKSIZE_CASES),
        1 => rand_blocksize_default_le256(rng),
        _ => rand_blocksize_default_gt256(rng),
    }
}

/// Axis BS class: `default` with `cur_blocksize <= 256`.
pub fn rand_blocksize_default_le256(rng: &mut Rng) -> u32 {
    loop {
        let v = rng.range_u32(0, 256);
        if !is_blocksize_case(v) {
            return v;
        }
    }
}

/// Axis BS class: `default` with `cur_blocksize > 256`.
pub fn rand_blocksize_default_gt256(rng: &mut Rng) -> u32 {
    loop {
        // Mix small-ish and full-range magnitudes.
        let v = match rng.next_u64() % 3 {
            0 => rng.range_u32(257, 70000),
            1 => rng.range_u32(257, u32::MAX),
            _ => *rng.pick(&[257u32, 258, 4095, 4097, 32769, 65535, 65536, u32::MAX - 1, u32::MAX]),
        };
        if v > 256 && !is_blocksize_case(v) {
            return v;
        }
    }
}

/// Axis SR: random value from *any* of the 17 sample-rate classes.
pub fn rand_samplerate_any(rng: &mut Rng) -> u32 {
    match rng.next_u64() % 7 {
        0 => *rng.pick(&SAMPLERATE_CASES),
        1 => rand_sr_kilo_ok(rng),
        2 => rand_sr_kilo_overflow(rng),
        3 => rand_sr_lt_65536(rng),
        4 => rand_sr_deca_ok(rng),
        5 => rand_sr_deca_overflow(rng),
        _ => rand_sr_none(rng),
    }
}

/// SR class: `%1000==0 && /1000 < 256` → ORs `0x0C << 8`.
pub fn rand_sr_kilo_ok(rng: &mut Rng) -> u32 {
    loop {
        let v = rng.range_u32(0, 255) * 1000;
        if !is_samplerate_case(v) {
            return v;
        }
    }
}

/// SR class: `%1000==0 && /1000 >= 256` → silently ORs nothing.
pub fn rand_sr_kilo_overflow(rng: &mut Rng) -> u32 {
    loop {
        let k = rng.range_u32(256, u32::MAX / 1000);
        let v = k * 1000;
        if v % 1000 == 0 && v / 1000 >= 256 && !is_samplerate_case(v) {
            return v;
        }
    }
}

/// SR class: `%1000!=0 && < 65536` → ORs `0x0D << 8`.
pub fn rand_sr_lt_65536(rng: &mut Rng) -> u32 {
    loop {
        let v = rng.range_u32(0, 65535);
        if v % 1000 != 0 && !is_samplerate_case(v) {
            return v;
        }
    }
}

/// SR class: `%1000!=0 && >=65536 && %10==0 && /10 < 65536` → ORs `0x0E << 8`.
pub fn rand_sr_deca_ok(rng: &mut Rng) -> u32 {
    loop {
        let v = rng.range_u32(6554, 65535) * 10; // 65540 ..= 655350
        if v >= 65536 && v % 1000 != 0 && v % 10 == 0 && v / 10 < 65536 && !is_samplerate_case(v) {
            return v;
        }
    }
}

/// SR class: `%1000!=0 && >=65536 && %10==0 && /10 >= 65536` → silently nothing.
pub fn rand_sr_deca_overflow(rng: &mut Rng) -> u32 {
    loop {
        let v = rng.range_u32(65536, u32::MAX / 10) * 10;
        if v >= 65536 && v % 1000 != 0 && v % 10 == 0 && v / 10 >= 65536 && !is_samplerate_case(v) {
            return v;
        }
    }
}

/// SR class: `%1000!=0 && >=65536 && %10!=0` → silently nothing.
pub fn rand_sr_none(rng: &mut Rng) -> u32 {
    loop {
        let v = rng.range_u32(65536, u32::MAX);
        if v % 1000 != 0 && v % 10 != 0 && !is_samplerate_case(v) {
            return v;
        }
    }
}

/// Axis BD: random value from any of the 7 bit-depth classes.
pub fn rand_bitdepth_any(rng: &mut Rng) -> u32 {
    if rng.bool() {
        *rng.pick(&BITDEPTH_CASES)
    } else {
        rand_bitdepth_default(rng)
    }
}

/// BD class: `default` (silently ORs nothing).
pub fn rand_bitdepth_default(rng: &mut Rng) -> u32 {
    loop {
        let v = match rng.next_u64() % 3 {
            0 => rng.range_u32(0, 64),
            1 => rng.range_u32(0, u32::MAX),
            _ => *rng.pick(&[0u32, 1, 7, 9, 11, 13, 15, 17, 31, 33, u32::MAX - 1, u32::MAX]),
        };
        if !is_bitdepth_case(v) {
            return v;
        }
    }
}

/// Axis CH: random value from any of the 6 channel-count shapes.
pub fn rand_channels_any(rng: &mut Rng) -> u32 {
    match rng.next_u64() % 6 {
        0 => 0,
        1 => 1,
        2 => rng.range_u32(2, 8),
        3 => rng.range_u32(9, 16),
        4 => rng.range_u32(17, 0xEFFF_FFFF),
        _ => rng.range_u32(0xF000_0000, u32::MAX),
    }
}

/// A fully random `Input` with every axis drawn from its class set.
pub fn rand_input(rng: &mut Rng) -> Input {
    Input {
        samplerate: rand_samplerate_any(rng),
        channels: rand_channels_any(rng),
        bitdepth: rand_bitdepth_any(rng),
        channel_mode: rng.next_u8(),
        frame_header_initial: if rng.bool() { 0 } else { rng.next_u32() },
        cur_blocksize: rand_blocksize_any(rng),
        padding: if rng.bool() { 0 } else { rng.next_u8() },
    }
}
