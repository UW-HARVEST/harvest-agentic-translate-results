//! Differential tests: load BOTH the C `.so` and the Rust `.so` via `libloading`
//! and compare `max_size_frame` outputs through the FFI boundary.
//!
//! The Rust function is NEVER called directly — always through the dynamic
//! symbol exported by the cdylib, exactly as an external C caller would, so the
//! `#[no_mangle] extern "C"` wrapper itself is under test.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type MaxSizeFrame = unsafe extern "C" fn(u32, u32, u32) -> u32;

const SYM: &[u8] = b"max_size_frame\0";

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.remove(0)
}

/// Every Rust cdylib artifact we can find (release and/or debug). Both are
/// exercised so the exported wrapper is validated under both opt levels.
fn rust_so_paths() -> Vec<PathBuf> {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let mut out = Vec::new();
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libmax_size_frame_lib.so");
        if p.is_file() {
            out.push(p);
        }
    }
    assert!(
        !out.is_empty(),
        "no Rust cdylib found under {}; build it with `cargo build --release`",
        target.display()
    );
    out
}

/// A loaded pair of implementations, plus the raw fn pointers.
struct Pair {
    _c_lib: Library,
    _rust_libs: Vec<Library>,
    c: MaxSizeFrame,
    rust: Vec<(String, MaxSizeFrame)>,
}

impl Pair {
    fn load() -> Self {
        let c_path = c_so_path();
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen C .so {}: {e}", c_path.display()));
        let c: MaxSizeFrame = unsafe {
            let s: Symbol<MaxSizeFrame> = c_lib
                .get(SYM)
                .unwrap_or_else(|e| panic!("C .so is missing symbol `max_size_frame`: {e}"));
            *s
        };

        let mut rust_libs = Vec::new();
        let mut rust = Vec::new();
        for p in rust_so_paths() {
            let lib = unsafe { Library::new(&p) }
                .unwrap_or_else(|e| panic!("dlopen Rust .so {}: {e}", p.display()));
            let f: MaxSizeFrame = unsafe {
                let s: Symbol<MaxSizeFrame> = lib.get(SYM).unwrap_or_else(|e| {
                    panic!(
                        "Rust .so {} is missing exported symbol `max_size_frame` \
                         (is the #[no_mangle] extern \"C\" wrapper present?): {e}",
                        p.display()
                    )
                });
                *s
            };
            rust.push((p.display().to_string(), f));
            rust_libs.push(lib);
        }

        Pair {
            _c_lib: c_lib,
            _rust_libs: rust_libs,
            c,
            rust,
        }
    }

    /// Compare C vs every Rust artifact for one input triple.
    #[track_caller]
    fn check(&self, bs: u32, ch: u32, bd: u32) {
        let expected = unsafe { (self.c)(bs, ch, bd) };
        for (name, f) in &self.rust {
            let got = unsafe { f(bs, ch, bd) };
            assert_eq!(
                expected, got,
                "divergence for max_size_frame(blocksize={bs}, channels={ch}, bitdepth={bd}): \
                 C .so returned {expected} (0x{expected:08x}), Rust .so `{name}` returned \
                 {got} (0x{got:08x})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed => reproducible runs
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_C0DE_5EED_C0DE;

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        // Ensure non-zero state.
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[lo, hi]` inclusive.
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    fn pick(&mut self, xs: &[u32]) -> u32 {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

const ITERS: usize = 2000;

/// Values interesting for `bitdepth` (32 is special-cased in the C).
const BITDEPTHS: &[u32] = &[
    0, 1, 2, 4, 7, 8, 12, 16, 20, 24, 31, 32, 33, 64, 255, 256, 65535, 65536, 0x7FFF_FFFF,
    0x8000_0000, 0x8000_0001, 0xFFFF_FFFE, u32::MAX,
];

/// Values interesting for `blocksize`.
const BLOCKSIZES: &[u32] = &[
    0, 1, 2, 7, 8, 9, 15, 16, 192, 576, 1152, 2304, 4096, 4608, 8192, 16384, 32768, 65535, 65536,
    0x00FF_FFFF, 0x0100_0000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF,
];

/// Values interesting for `channels` (2 is special-cased in the C).
const CHANNELS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 255, 256, 65535, 65536, 0x7FFF_FFFF, 0x8000_0000,
    0xFFFF_FFFE, u32::MAX,
];

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// Row 1: channels == 2 (stereo path) × bitdepth == 32 × random blocksize.
#[test]
fn config_row_01_stereo_bitdepth32_random_blocksize() {
    let p = Pair::load();
    let mut r = Rng::new(1);
    for bs in BLOCKSIZES {
        p.check(*bs, 2, 32);
    }
    for _ in 0..ITERS {
        p.check(r.u32(), 2, 32);
    }
}

/// Row 2: channels == 2 × bitdepth != 32 (random full range) × random blocksize.
#[test]
fn config_row_02_stereo_bitdepth_not32_random() {
    let p = Pair::load();
    let mut r = Rng::new(2);
    for _ in 0..ITERS {
        let mut bd = r.u32();
        if bd == 32 {
            bd = 33; // keep this row strictly on the `bitdepth != 32` side
        }
        p.check(r.u32(), 2, bd);
    }
}

/// Row 3: channels == 2 × bitdepth near the 32 boundary × random blocksize.
#[test]
fn config_row_03_stereo_bitdepth_near_32_boundary() {
    let p = Pair::load();
    let mut r = Rng::new(3);
    for bd in [0u32, 1, 8, 16, 24, 30, 31, 33, 34, 64] {
        for bs in BLOCKSIZES {
            p.check(*bs, 2, bd);
        }
        for _ in 0..(ITERS / 10) {
            p.check(r.u32(), 2, bd);
        }
    }
}

/// Row 4: channels == 2 × bitdepth == 32 × audio-realistic + boundary blocksizes.
#[test]
fn config_row_04_stereo_bitdepth32_realistic_blocksizes() {
    let p = Pair::load();
    for bs in [0u32, 1, 16, 192, 576, 1152, 4096, 4608, 65535, 65536] {
        p.check(bs, 2, 32);
    }
    let mut r = Rng::new(4);
    for _ in 0..ITERS {
        p.check(r.range(0, 65536), 2, 32);
    }
}

/// Row 5: channels == 0 — zeroes `term1` and the outer `+ channels`.
#[test]
fn config_row_05_channels_zero() {
    let p = Pair::load();
    let mut r = Rng::new(5);
    for bd in BITDEPTHS {
        for bs in BLOCKSIZES {
            p.check(*bs, 0, *bd);
        }
    }
    for _ in 0..ITERS {
        p.check(r.u32(), 0, r.u32());
    }
}

/// Row 6: channels == 1 (mono).
#[test]
fn config_row_06_channels_one_mono() {
    let p = Pair::load();
    let mut r = Rng::new(6);
    for bd in BITDEPTHS {
        for bs in BLOCKSIZES {
            p.check(*bs, 1, *bd);
        }
    }
    for _ in 0..ITERS {
        p.check(r.u32(), 1, r.u32());
    }
}

/// Row 7: channels == 3 — first count past the stereo special case.
#[test]
fn config_row_07_channels_three() {
    let p = Pair::load();
    let mut r = Rng::new(7);
    for bd in BITDEPTHS {
        for bs in BLOCKSIZES {
            p.check(*bs, 3, *bd);
        }
    }
    for _ in 0..ITERS {
        p.check(r.u32(), 3, r.u32());
    }
}

/// Row 8: multichannel FLAC-legal usage: channels 4..=8, bitdepth {8,16,24,32}.
#[test]
fn config_row_08_multichannel_realistic() {
    let p = Pair::load();
    let mut r = Rng::new(8);
    for ch in 4u32..=8 {
        for bd in [8u32, 16, 24, 32] {
            for bs in [1u32, 192, 576, 1152, 2304, 4096, 4608, 65535] {
                p.check(bs, ch, bd);
            }
            for _ in 0..(ITERS / 20) {
                p.check(r.range(1, 65535), ch, bd);
            }
        }
    }
}

/// Row 9: channels random full-range u32 (non-2 path, with wrapping).
#[test]
fn config_row_09_channels_full_range_random() {
    let p = Pair::load();
    let mut r = Rng::new(9);
    for _ in 0..(ITERS * 5) {
        p.check(r.u32(), r.u32(), r.u32());
    }
    for ch in CHANNELS {
        for bd in BITDEPTHS {
            for bs in BLOCKSIZES {
                p.check(*bs, *ch, *bd);
            }
        }
    }
}

/// Row 10: channels == 2 × bitdepth with the high bit set (wrapping stereo path).
#[test]
fn config_row_10_stereo_high_bit_bitdepth() {
    let p = Pair::load();
    let mut r = Rng::new(10);
    for bd in [0x8000_0000u32, 0x8000_0001, 0xC000_0000, 0xFFFF_FFFE, u32::MAX] {
        for bs in BLOCKSIZES {
            p.check(*bs, 2, bd);
        }
    }
    for _ in 0..ITERS {
        p.check(r.u32(), 2, r.u32() | 0x8000_0000);
    }
}

/// Row 11: blocksize == 0 — every term vanishes; result is `18 + channels`.
#[test]
fn config_row_11_blocksize_zero() {
    let p = Pair::load();
    let mut r = Rng::new(11);
    for ch in CHANNELS {
        for bd in BITDEPTHS {
            p.check(0, *ch, *bd);
        }
    }
    for _ in 0..ITERS {
        p.check(0, r.u32(), r.u32());
    }
}

/// Row 12: blocksize == 1.
#[test]
fn config_row_12_blocksize_one() {
    let p = Pair::load();
    let mut r = Rng::new(12);
    for ch in CHANNELS {
        for bd in BITDEPTHS {
            p.check(1, *ch, *bd);
        }
    }
    for _ in 0..ITERS {
        p.check(1, r.u32(), r.u32());
    }
}

/// Row 13: blocksize == u32::MAX.
#[test]
fn config_row_13_blocksize_max() {
    let p = Pair::load();
    let mut r = Rng::new(13);
    for ch in CHANNELS {
        for bd in BITDEPTHS {
            p.check(u32::MAX, *ch, *bd);
        }
    }
    for _ in 0..ITERS {
        p.check(u32::MAX, r.u32(), r.u32());
    }
}

/// Row 14: overflow-targeted — powers of two so products land on / near 2^32.
#[test]
fn config_row_14_overflow_targeted_powers_of_two() {
    let p = Pair::load();
    let pow2: Vec<u32> = (0..32).map(|i| 1u32 << i).collect();
    for bs in &pow2 {
        for bd in &pow2 {
            // stereo path
            p.check(*bs, 2, *bd);
            // non-stereo paths, including power-of-two channel counts
            for ch in [0u32, 1, 3, 4, 8, 1 << 16, 1 << 31] {
                p.check(*bs, ch, *bd);
            }
        }
    }
    // Values straddling exact 2^32 multiples: bs*bd == 2^32 exactly, ±1.
    let mut r = Rng::new(14);
    for i in 1..32u32 {
        let bs = 1u32 << i;
        let bd = 1u32 << (32 - i); // wraps to 0 when i==0; i>=1 keeps it in range
        for (b1, b2) in [
            (bs, bd),
            (bs.wrapping_sub(1), bd),
            (bs.wrapping_add(1), bd),
            (bs, bd.wrapping_sub(1)),
            (bs, bd.wrapping_add(1)),
        ] {
            p.check(b1, 2, b2);
            p.check(b1, 3, b2);
            p.check(b1, r.pick(CHANNELS), b2);
        }
    }
}

/// Row 15: rounding-targeted — sweep `sum mod 8` across 0..7 on both paths.
#[test]
fn config_row_15_rounding_residues() {
    let p = Pair::load();
    // For channels != 2: sum = bs*bd*ch + 7. Sweep small products so the
    // residue of bs*bd*ch mod 8 hits every value.
    for prod in 0u32..64 {
        p.check(prod, 1, 1); // bs*bd*ch == prod
        p.check(1, 1, prod);
        p.check(1, 3, prod);
        p.check(prod, 3, 1);
    }
    // For channels == 2: sum = bs*bd + bs*(bd + (bd!=32)) + 7.
    for bs in 0u32..32 {
        for bd in 0u32..40 {
            p.check(bs, 2, bd);
        }
    }
    // Wider sweep of residues near the /8 boundary.
    let mut r = Rng::new(15);
    for _ in 0..ITERS {
        let bs = r.range(0, 1023);
        let bd = r.range(0, 63);
        let ch = r.range(0, 9);
        p.check(bs, ch, bd);
    }
}

/// Row 16: channels == u32::MAX — outer `18U + channels` wraps past 2^32.
#[test]
fn config_row_16_channels_max_outer_wrap() {
    let p = Pair::load();
    let mut r = Rng::new(16);
    for ch in [
        0xFFFF_FFFFu32,
        0xFFFF_FFFE,
        0xFFFF_FFF0,
        0xFFFF_FFEE, // 18 + this == 0 exactly
        0x8000_0000,
    ] {
        for bd in BITDEPTHS {
            for bs in BLOCKSIZES {
                p.check(*bs, ch, *bd);
            }
        }
    }
    for _ in 0..ITERS {
        p.check(r.u32(), u32::MAX, r.u32());
    }
}

/// Row 17: fully unconstrained random sweep, 200k iterations, fixed seed.
#[test]
fn config_row_17_unconstrained_random_200k() {
    let p = Pair::load();
    let mut r = Rng::new(17);
    for _ in 0..200_000 {
        p.check(r.u32(), r.u32(), r.u32());
    }
}

/// Row 18: exhaustive dense cube [0,40]^3 == 68 921 triples.
#[test]
fn config_row_18_exhaustive_cube_0_40() {
    let p = Pair::load();
    for bs in 0u32..=40 {
        for ch in 0u32..=40 {
            for bd in 0u32..=40 {
                p.check(bs, ch, bd);
            }
        }
    }
}

/// Row 19: structured sweep across all three axes' interesting values.
#[test]
fn config_row_19_structured_sweep() {
    let p = Pair::load();
    for ch in 0u32..=10 {
        for bd in 0u32..=40 {
            for bs in [
                0u32,
                1,
                7,
                8,
                9,
                4095,
                4096,
                65535,
                0x8000_0000,
                0xFFFF_FFFF,
            ] {
                p.check(bs, ch, bd);
            }
        }
    }
}

/// Row 20: purity / no hidden state — replay the same call repeatedly and
/// interleave distinct inputs; a `static mut` regression in Rust would show.
#[test]
fn config_row_20_purity_no_hidden_state() {
    let p = Pair::load();
    let inputs: Vec<(u32, u32, u32)> = vec![
        (4096, 2, 32),
        (4096, 2, 16),
        (0, 0, 0),
        (u32::MAX, u32::MAX, u32::MAX),
        (1152, 6, 24),
        (65535, 1, 8),
    ];
    for _round in 0..3 {
        for (bs, ch, bd) in &inputs {
            p.check(*bs, *ch, *bd);
        }
    }
    // Interleaved: alternate between two very different configurations.
    for _ in 0..500 {
        p.check(4096, 2, 32);
        p.check(0xDEAD_BEEF, 7, 0xFEED_FACE);
        p.check(4096, 2, 32);
    }
}

// ===========================================================================
// Phase C — ERRORS.md: the rejection table is EMPTY (the C function has no
// error path at all). These cover the generic boundary classes E1..E7 that
// could still make the two implementations diverge.
// ===========================================================================

/// E1: every subset of the three arguments set to 0.
#[test]
fn err_zero_arguments() {
    let p = Pair::load();
    let vals = [0u32, 4096];
    for bs in vals {
        for ch in vals {
            for bd in vals {
                p.check(bs, ch, bd);
            }
        }
    }
    // Also with the "interesting" non-zero partner values.
    for ch in CHANNELS {
        p.check(0, *ch, 0);
        p.check(0, 0, *ch);
    }
    for bd in BITDEPTHS {
        p.check(0, 0, *bd);
        p.check(0, *bd, 0);
    }
}

/// E2: full cross product over {0, 1, u32::MAX} — zero and oversized lengths.
#[test]
fn err_extremes_cross_product() {
    let p = Pair::load();
    let vals = [0u32, 1, u32::MAX];
    for bs in vals {
        for ch in vals {
            for bd in vals {
                p.check(bs, ch, bd);
            }
        }
    }
    // And over a wider extreme set (27 -> 512 combinations).
    let wide = [
        0u32,
        1,
        2,
        32,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFE,
        u32::MAX,
    ];
    for bs in wide {
        for ch in wide {
            for bd in wide {
                p.check(bs, ch, bd);
            }
        }
    }
}

/// E3: one step past each special-cased value / documented range.
#[test]
fn err_one_past_range() {
    let p = Pair::load();
    // channels: 1, 2, 3 straddle the `channels == 2` special case.
    // bitdepth: 31, 32, 33 straddle the `bitdepth != 32` special case.
    // FLAC's real limits are channels<=8, bitdepth<=32, blocksize<=65535;
    // one step past each is still a legal u32 the C accepts.
    for ch in [1u32, 2, 3, 8, 9] {
        for bd in [31u32, 32, 33, 4, 5] {
            for bs in [0u32, 1, 65534, 65535, 65536, 65537] {
                p.check(bs, ch, bd);
            }
        }
    }
}

/// E4: out-of-range "enum-like" values passed across the FFI boundary.
/// C accepts any bit pattern for a `uint32_t` parameter, so values with no
/// musical/semantic meaning are real inputs both sides must handle alike.
#[test]
fn err_out_of_range_enum_like() {
    let p = Pair::load();
    let junk = [
        0u32,
        7,
        255,
        256,
        1000,
        0x0000_FFFF,
        0x0001_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0xDEAD_BEEF,
        0xFFFF_FFFF,
    ];
    for ch in junk {
        for bd in junk {
            for bs in [0u32, 1, 4096, 0x8000_0000, u32::MAX] {
                p.check(bs, ch, bd);
            }
        }
    }
}

/// E5: multiplication overflow / wraparound, incl. the `+7` carry at the wrap
/// point and exact 2^32 multiples.
#[test]
fn err_overflow_wraparound() {
    let p = Pair::load();
    // bs*bd*ch products that wrap.
    let big = [
        0x0001_0000u32,
        0x0002_0000,
        0x0100_0000,
        0x4000_0000,
        0x8000_0000,
        0xFFFF_FFFF,
    ];
    for bs in big {
        for bd in big {
            for ch in [0u32, 1, 2, 3, 4, 0x8000_0000, u32::MAX] {
                p.check(bs, ch, bd);
            }
        }
    }
    // Sums landing exactly at 2^32-7 .. 2^32-1 so `+7` carries.
    // channels != 2 path: sum = bs*bd*ch (+7).
    for k in 0u32..16 {
        let target = u32::MAX - k;
        // bd == 1, ch == 1 -> product == bs
        p.check(target, 1, 1);
        // bd == 1, ch == 3 -> product == bs*3
        p.check(target, 3, 1);
    }
    // channels == 2 path around the wrap point.
    let mut r = Rng::new(105);
    for _ in 0..ITERS {
        p.check(r.u32() | 0x8000_0000, 2, r.u32() | 0x8000_0000);
    }
}

/// E6: high-bit (sign-bit) values — a signed mis-translation would diverge.
#[test]
fn err_high_bit_values() {
    let p = Pair::load();
    let mut r = Rng::new(106);
    for _ in 0..(ITERS * 2) {
        p.check(
            r.u32() | 0x8000_0000,
            r.u32() | 0x8000_0000,
            r.u32() | 0x8000_0000,
        );
    }
    for bs in [0x8000_0000u32, 0xFFFF_FFFF] {
        for ch in [0x8000_0000u32, 0xFFFF_FFFF, 2] {
            for bd in [0x8000_0000u32, 0xFFFF_FFFF, 32] {
                p.check(bs, ch, bd);
            }
        }
    }
}

/// E7: exhaustive small domain [0,64]^3 == 274 625 triples.
#[test]
fn err_exhaustive_small_cube() {
    let p = Pair::load();
    for bs in 0u32..=64 {
        for ch in 0u32..=64 {
            for bd in 0u32..=64 {
                p.check(bs, ch, bd);
            }
        }
    }
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite
// ===========================================================================

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`.
#[test]
fn phase_d_symbol_parity() {
    fn dynsyms(p: &Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(p)
            .output()
            .expect("`nm` must be available to check symbol parity");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            p.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(str::to_string))
            .filter(|s| !s.is_empty())
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c_syms = dynsyms(&c_so_path());
    assert!(
        c_syms.contains(&"max_size_frame".to_string()),
        "sanity: C .so should export max_size_frame, got {c_syms:?}"
    );

    for rp in rust_so_paths() {
        let r_syms = dynsyms(&rp);
        let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "Rust .so {} is missing {} symbol(s) exported by the C .so: {:?}",
            rp.display(),
            missing.len(),
            missing
        );
    }
}

/// Both `.so`s must resolve the symbol and agree; also guards against the Rust
/// export being present but with a different calling convention/arity by
/// checking a hand-computed reference value.
#[test]
fn phase_d_reference_values() {
    let p = Pair::load();
    // Hand-computed from the C:
    //   max_size_frame(4096, 2, 32)
    //     term1 = 4096*32*(2*(2!=2)) = 0
    //     term2 = 4096*32*(2==2)     = 131072
    //     term3 = 4096*(32+(32!=32))*(2==2) = 4096*32 = 131072
    //     (0 + 131072 + 131072 + 7) / 8 = 262151/8 = 32768
    //     18 + 2 + 32768 = 32788
    let v = unsafe { (p.c)(4096, 2, 32) };
    assert_eq!(v, 32788, "C reference value drifted; check the derivation");
    p.check(4096, 2, 32);

    //   max_size_frame(4096, 2, 16)
    //     term2 = 4096*16 = 65536
    //     term3 = 4096*(16+1) = 69632
    //     (65536 + 69632 + 7)/8 = 135175/8 = 16896
    //     18 + 2 + 16896 = 16916
    assert_eq!(unsafe { (p.c)(4096, 2, 16) }, 16916);
    p.check(4096, 2, 16);

    //   max_size_frame(4096, 1, 16)  (channels != 2)
    //     term1 = 4096*16*(1*1) = 65536
    //     (65536+7)/8 = 8192
    //     18 + 1 + 8192 = 8211
    assert_eq!(unsafe { (p.c)(4096, 1, 16) }, 8211);
    p.check(4096, 1, 16);

    //   max_size_frame(0, 0, 0) = 18 + 0 + (7/8) = 18
    assert_eq!(unsafe { (p.c)(0, 0, 0) }, 18);
    p.check(0, 0, 0);
}

// ===========================================================================
// Deep stress (ignored by default; run with `--ignored --release`)
// ===========================================================================

/// 50 million pseudo-random triples plus a fully exhaustive sweep of the
/// (channels, bitdepth) plane over [0,300]^2 against many blocksizes.
/// `#[ignore]`d so the default suite stays fast.
#[test]
#[ignore]
fn deep_stress_50m() {
    let p = Pair::load();
    let c = p.c;
    // Fast path: resolve the Rust fns once, still through the .so exports.
    let rust: Vec<MaxSizeFrame> = p.rust.iter().map(|(_, f)| *f).collect();

    let mut r = Rng::new(0xDEEB_EEF);
    let mut mismatches = 0usize;
    for _ in 0..50_000_000u64 {
        let (bs, ch, bd) = (r.u32(), r.u32(), r.u32());
        let e = unsafe { c(bs, ch, bd) };
        for f in &rust {
            if unsafe { f(bs, ch, bd) } != e {
                mismatches += 1;
                if mismatches < 10 {
                    eprintln!("MISMATCH at ({bs}, {ch}, {bd})");
                }
            }
        }
    }
    assert_eq!(mismatches, 0, "{mismatches} random-triple divergences");

    // Exhaustive (channels, bitdepth) plane, several blocksizes.
    for bs in [0u32, 1, 3, 8, 4096, 65535, 0x8000_0000, u32::MAX] {
        for ch in 0u32..=300 {
            for bd in 0u32..=300 {
                let e = unsafe { c(bs, ch, bd) };
                for f in &rust {
                    assert_eq!(unsafe { f(bs, ch, bd) }, e, "plane ({bs}, {ch}, {bd})");
                }
            }
        }
    }
}
