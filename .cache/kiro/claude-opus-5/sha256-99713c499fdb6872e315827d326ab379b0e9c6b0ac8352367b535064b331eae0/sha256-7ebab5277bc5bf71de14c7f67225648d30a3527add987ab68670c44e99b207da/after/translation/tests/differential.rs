//! Differential test: loads the C `.so` and the Rust `.so` via `libloading`
//! and compares `update_frame_header` byte-for-byte across both.
//!
//! Neither library is linked directly; both are called purely through their
//! exported symbols, so the `#[no_mangle]` wrapper is exercised as well.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Mirrors `struct tflac` from c_src/include/lib.h.
///
/// The C struct has a padding hole after `channel_mode`; comparisons are done
/// on the raw byte image so any difference in padding handling would show up.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tflac {
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    frame_header: u32,
    cur_blocksize: u32,
}

const TFLAC_SIZE: usize = std::mem::size_of::<Tflac>();

type UpdateFrameHeader = unsafe extern "C" fn(*mut Tflac);

struct Lib {
    _lib: Library,
    update_frame_header: RawFn,
}

/// Raw function pointer extracted from the loaded library. Kept alongside the
/// `Library` so the mapping outlives every call.
struct RawFn(UpdateFrameHeader);

impl Lib {
    fn open(path: &Path) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        let f: Symbol<UpdateFrameHeader> = unsafe { lib.get(b"update_frame_header\0") }
            .unwrap_or_else(|e| {
                panic!(
                    "symbol `update_frame_header` missing from {}: {e}",
                    path.display()
                )
            });
        let raw = RawFn(*f);
        drop(f);
        Lib {
            _lib: lib,
            update_frame_header: raw,
        }
    }

    /// Run the function over a zeroed-padding struct image and return the raw
    /// bytes of the result.
    fn call(&self, input: &Tflac) -> [u8; TFLAC_SIZE] {
        // Start from an all-zero image so padding bytes are deterministic,
        // then write the live fields.
        let mut buf = [0u8; TFLAC_SIZE];
        {
            let p = buf.as_mut_ptr().cast::<Tflac>();
            unsafe { std::ptr::write(p, *input) };
        }
        // Re-zero the padding hole after `channel_mode` so that both libraries
        // observe (and are compared on) identical padding.
        zero_padding(&mut buf);

        let p = buf.as_mut_ptr().cast::<Tflac>();
        unsafe { (self.update_frame_header.0)(p) };
        buf
    }
}

/// Zero the bytes that are not covered by any struct field.
fn zero_padding(buf: &mut [u8; TFLAC_SIZE]) {
    let base = buf.as_ptr() as usize;
    let mut covered = [false; TFLAC_SIZE];
    // SAFETY: offsets of a #[repr(C)] struct with no interior mutability.
    let t = buf.as_ptr().cast::<Tflac>();
    let mut mark = |off: usize, len: usize| {
        for c in covered.iter_mut().skip(off).take(len) {
            *c = true;
        }
    };
    unsafe {
        mark(std::ptr::addr_of!((*t).samplerate) as usize - base, 4);
        mark(std::ptr::addr_of!((*t).channels) as usize - base, 4);
        mark(std::ptr::addr_of!((*t).bitdepth) as usize - base, 4);
        mark(std::ptr::addr_of!((*t).channel_mode) as usize - base, 1);
        mark(std::ptr::addr_of!((*t).frame_header) as usize - base, 4);
        mark(std::ptr::addr_of!((*t).cur_blocksize) as usize - base, 4);
    }
    for i in 0..TFLAC_SIZE {
        if !covered[i] {
            buf[i] = 0;
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let entries = std::fs::read_dir(&build).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    });
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib sits one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let name = "libupdate_frame_header_lib.so";
    let direct = profile_dir.join(name);
    if direct.exists() {
        return direct;
    }
    // Fall back to scanning the usual profiles.
    let target = workspace_root().join("translation/target");
    for p in ["debug", "release"] {
        let cand = target.join(p).join(name);
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "could not locate {name}; expected at {}",
        direct.display()
    );
}

struct Pair {
    c: Lib,
    rust: Lib,
}

impl Pair {
    fn load() -> Pair {
        Pair {
            c: Lib::open(&find_c_so()),
            rust: Lib::open(&find_rust_so()),
        }
    }

    fn assert_match(&self, input: &Tflac) {
        let c_out = self.c.call(input);
        let r_out = self.rust.call(input);
        assert_eq!(
            c_out, r_out,
            "mismatch for samplerate={} channels={} bitdepth={} channel_mode={} \
             cur_blocksize={}\n  C   : {:02x?}\n  Rust: {:02x?}\n  \
             C frame_header=0x{:08X} Rust frame_header=0x{:08X}",
            input.samplerate,
            input.channels,
            input.bitdepth,
            input.channel_mode,
            input.cur_blocksize,
            c_out,
            r_out,
            read_frame_header(&c_out),
            read_frame_header(&r_out),
        );
    }
}

fn read_frame_header(buf: &[u8; TFLAC_SIZE]) -> u32 {
    let t = buf.as_ptr().cast::<Tflac>();
    unsafe { std::ptr::read_unaligned(std::ptr::addr_of!((*t).frame_header)) }
}

fn mk(samplerate: u32, channels: u32, bitdepth: u32, channel_mode: u8, cur_blocksize: u32) -> Tflac {
    Tflac {
        samplerate,
        channels,
        bitdepth,
        channel_mode,
        // Pre-seeded with a non-zero value: the C code assigns unconditionally,
        // so any failure to overwrite would surface here.
        frame_header: 0xDEAD_BEEF,
        cur_blocksize,
    }
}

/// Block sizes with a dedicated case in the C switch, plus values that fall
/// through to the `<= 256` / `> 256` default.
const BLOCKSIZES: &[u32] = &[
    0, 1, 2, 15, 16, 100, 191, 192, 193, 255, 256, 257, 511, 512, 513, 575, 576, 577, 1023, 1024,
    1025, 1151, 1152, 1153, 2047, 2048, 2049, 2303, 2304, 2305, 4095, 4096, 4097, 4607, 4608, 4609,
    8191, 8192, 8193, 16383, 16384, 16385, 32767, 32768, 32769, 65535, 65536, 100_000,
    u32::MAX - 1,
    u32::MAX,
];

/// Sample rates: every explicit case, the boundaries of each default branch,
/// and assorted values that hit the "no bits set" holes.
const SAMPLERATES: &[u32] = &[
    0, 1, 2, 999, 1000, 1001, 7999, 8000, 8001, 10, 100, 110, 16000, 22050, 22051, 24000, 32000,
    44100, 48000, 96000, 176400, 192000, 882000, 88200, 255_000, 256_000, 257_000, 1000,
    65535, 65536, 65537, 65530, 65540, 655_350, 655_360, 655_370, 4095, 12345, 54321,
    // rate % 1000 == 0 but rate/1000 >= 256 -> no bits
    300_000, 1_000_000,
    // rate % 10 == 0, rate/10 < 65536 -> 0x0E
    655_350 - 10, 70_000, 123_450,
    // rate % 10 == 0 but rate/10 >= 65536 -> no bits
    655_360 + 10, 2_000_010,
    // odd large values -> no bits at all
    65537 * 3, 999_999, 4_294_967_295, 4_294_967_290, 4_294_967_291,
];

const BITDEPTHS: &[u32] = &[
    0, 1, 4, 7, 8, 9, 11, 12, 13, 15, 16, 17, 19, 20, 21, 23, 24, 25, 31, 32, 33, 64, 255, 256,
    u32::MAX,
];

const CHANNELS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 255, 256, 65535, 65536, u32::MAX - 1, u32::MAX,
];

#[test]
fn exports_match_c() {
    // Loading both libraries proves the Rust `.so` exports the same symbol
    // name the C `.so` does.
    let _ = Pair::load();
}

#[test]
fn blocksize_paths() {
    let p = Pair::load();
    for &bs in BLOCKSIZES {
        p.assert_match(&mk(44100, 2, 16, 0, bs));
    }
}

#[test]
fn samplerate_paths() {
    let p = Pair::load();
    for &sr in SAMPLERATES {
        p.assert_match(&mk(sr, 2, 16, 0, 4096));
    }
}

#[test]
fn bitdepth_paths() {
    let p = Pair::load();
    for &bd in BITDEPTHS {
        p.assert_match(&mk(44100, 2, bd, 0, 4096));
    }
}

/// `channel_mode` is reduced with `% 4`, so all 256 byte values must be
/// covered, including those above `TFLAC_CHANNEL_MODE_COUNT`.
#[test]
fn channel_mode_all_bytes() {
    let p = Pair::load();
    for mode in 0u8..=255 {
        for &ch in CHANNELS {
            p.assert_match(&mk(44100, ch, 16, mode, 4096));
        }
    }
}

/// Independent mode uses `(channels - 1) << 4` on unsigned values, so the
/// `channels == 0` wraparound must match.
#[test]
fn channels_wraparound_independent_mode() {
    let p = Pair::load();
    for &ch in CHANNELS {
        for mode in [0u8, 4, 8, 12, 100, 252] {
            p.assert_match(&mk(48000, ch, 24, mode, 192));
        }
    }
}

#[test]
fn cross_product_representatives() {
    let p = Pair::load();
    let srs = [0u32, 8000, 22050, 44100, 96000, 192000, 882000, 65535, 300_000, 123_450, 999_999];
    let bss = [0u32, 192, 256, 257, 4096, 32768, 65536, u32::MAX];
    let bds = [0u32, 8, 12, 16, 20, 24, 32, 33];
    let chs = [0u32, 1, 2, 8, 16, u32::MAX];
    let modes = [0u8, 1, 2, 3, 4, 7, 255];
    for &sr in &srs {
        for &bs in &bss {
            for &bd in &bds {
                for &ch in &chs {
                    for &m in &modes {
                        p.assert_match(&mk(sr, ch, bd, m, bs));
                    }
                }
            }
        }
    }
}

/// Deterministic pseudo-random sweep over the full 32-bit input space.
#[test]
fn randomized_sweep() {
    let p = Pair::load();
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    for _ in 0..200_000 {
        let a = next();
        let b = next();
        let samplerate = a as u32;
        let channels = (a >> 32) as u32;
        let bitdepth = b as u32;
        let cur_blocksize = (b >> 32) as u32;
        let channel_mode = (next() & 0xFF) as u8;
        p.assert_match(&mk(
            samplerate,
            channels,
            bitdepth,
            channel_mode,
            cur_blocksize,
        ));
    }
}

/// Same sweep, but biased towards small values where most switch cases live.
#[test]
fn randomized_sweep_small_values() {
    let p = Pair::load();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };
    for _ in 0..200_000 {
        let samplerate = (next() % 200_001) as u32;
        let channels = (next() % 40) as u32;
        let bitdepth = (next() % 40) as u32;
        let cur_blocksize = (next() % 40_000) as u32;
        let channel_mode = (next() & 0xFF) as u8;
        p.assert_match(&mk(
            samplerate,
            channels,
            bitdepth,
            channel_mode,
            cur_blocksize,
        ));
    }
}

/// The C code assigns `frame_header` unconditionally; confirm the incoming
/// value never leaks through, for either library.
#[test]
fn frame_header_is_always_overwritten() {
    let p = Pair::load();
    for seed in [0u32, 1, 0xDEAD_BEEF, u32::MAX, 0x5555_5555] {
        for &sr in &[0u32, 44100, 96000, 999_999] {
            let mut t = mk(sr, 2, 16, 0, 4096);
            t.frame_header = seed;
            p.assert_match(&t);
            let out = read_frame_header(&p.rust.call(&t));
            assert_eq!(
                out & 0xFFF8_0000,
                0xFFF8_0000,
                "sync code missing from result 0x{out:08X}"
            );
        }
    }
}

/// Sanity check that the layout the tests use matches the C ABI: 3×u32,
/// u8 + 3 bytes padding, 2×u32.
#[test]
fn struct_layout_matches_c_abi() {
    assert_eq!(TFLAC_SIZE, 24, "unexpected sizeof(struct tflac)");
    assert_eq!(std::mem::align_of::<Tflac>(), 4);
}
