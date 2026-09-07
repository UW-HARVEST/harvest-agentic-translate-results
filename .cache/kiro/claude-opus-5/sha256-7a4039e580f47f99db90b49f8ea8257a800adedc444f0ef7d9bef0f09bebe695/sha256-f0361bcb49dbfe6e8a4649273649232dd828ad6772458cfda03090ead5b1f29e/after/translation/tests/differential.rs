//! Differential tests: load BOTH the C `.so` and the Rust `.so` via `libloading`
//! and compare their outputs through the FFI boundary.
//!
//! The Rust function is NEVER called directly — it is always resolved as an
//! exported symbol from `libmax_size_frame_lib.so`, exactly as an external C
//! consumer would, so the `#[no_mangle] extern "C"` wrapper is under test too.

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

type MaxSizeFrameFn = unsafe extern "C" fn(u32, u32, u32) -> u32;

/// Holder for the two dynamically loaded libraries.
struct Libs {
    // The `Library` values must outlive the raw function pointers taken from
    // them, so they are kept alive here for the whole process lifetime.
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: MaxSizeFrameFn,
    rust_fn: MaxSizeFrameFn,
}

// SAFETY: after loading, both fields are plain code pointers into mapped,
// never-unloaded shared objects; the target functions are pure integer
// arithmetic with no shared mutable state.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C shared library built by CMake. The CMake project name is
/// derived from the parent directory name, so the file name is not fixed;
/// discover it by scanning the build directory.
fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    assert!(
        build.is_dir(),
        "C build directory not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("readable c_src/build")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    assert!(
        !candidates.is_empty(),
        "no lib*.so found in {}",
        build.display()
    );
    candidates.remove(0)
}

/// Locate the Rust `cdylib`. Prefer the profile the tests were built with, but
/// accept either `debug/` or `release/`.
fn find_rust_so() -> PathBuf {
    let target = workspace_root().join("translation").join("target");
    let name = "libmax_size_frame_lib.so";
    // Order matters only for preference; both are the same source.
    for profile in ["release", "debug"] {
        let p = target.join(profile).join(name);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "Rust cdylib {name} not found under {}. Build it with:\n  cd translation && cargo build --release",
        target.display()
    );
}

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();

        // SAFETY: loading two well-formed shared objects that run no
        // constructors with side effects relevant to the test.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", rust_path.display()));

        // SAFETY: the symbol has the declared C signature in both libraries
        // (`tflac_u32 max_size_frame(tflac_u32, tflac_u32, tflac_u32)` where
        // `tflac_u32` is `uint32_t`).
        let c_fn = unsafe {
            let s: Symbol<MaxSizeFrameFn> = c_lib
                .get(b"max_size_frame\0")
                .expect("C .so must export max_size_frame");
            *s
        };
        let rust_fn = unsafe {
            let s: Symbol<MaxSizeFrameFn> = rust_lib
                .get(b"max_size_frame\0")
                .expect("Rust .so must export max_size_frame");
            *s
        };

        Libs {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_fn,
            rust_fn,
        }
    })
}

fn call_c(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    // SAFETY: pure arithmetic, no pointers involved.
    unsafe { (libs().c_fn)(blocksize, channels, bitdepth) }
}

fn call_rust(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    // SAFETY: pure arithmetic, no pointers involved.
    unsafe { (libs().rust_fn)(blocksize, channels, bitdepth) }
}

/// Compare one input triple through both `.so`s. Bytes of a `u32` are compared
/// explicitly so the assertion is a byte-for-byte comparison of the ABI return
/// value, not just a numeric one.
#[track_caller]
fn assert_same(blocksize: u32, channels: u32, bitdepth: u32) {
    let c = call_c(blocksize, channels, bitdepth);
    let r = call_rust(blocksize, channels, bitdepth);
    assert_eq!(
        c.to_le_bytes(),
        r.to_le_bytes(),
        "divergence for (blocksize={blocksize}, channels={channels}, bitdepth={bitdepth}): \
         C returned {c} (0x{c:08x}), Rust returned {r} (0x{r:08x})"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed) — reproducible property-style inputs.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid the zero state fixed point of xorshift64*.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }

    fn pick(&mut self, xs: &[u32]) -> u32 {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Iterations per randomized row. Kept high enough to be meaningful and low
/// enough that the whole suite finishes in well under a second.
const ITERS: usize = 20_000;

const TYPICAL_DEPTHS_NE32: &[u32] = &[8, 12, 16, 20, 24];
/// Non-stereo channel counts in the small/typical range (2 excluded).
const SMALL_CHANNELS_NON_STEREO: &[u32] = &[1, 3, 4, 5, 6, 7, 8];

/// Draw a `channels` value that is guaranteed to be != 2, over the full u32 range.
fn any_non_stereo(rng: &mut Rng) -> u32 {
    loop {
        let c = rng.next_u32();
        if c != 2 {
            return c;
        }
    }
}

/// Draw a `bitdepth` value that is guaranteed to be != 32, over the full u32 range.
fn any_depth_ne_32(rng: &mut Rng) -> u32 {
    loop {
        let d = rng.next_u32();
        if d != 32 {
            return d;
        }
    }
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// C1: non-stereo, bitdepth != 32, small typical shape.
#[test]
fn cfg_c1_non_stereo_typical() {
    let mut rng = Rng::new(0xC001);
    for _ in 0..ITERS {
        let channels = rng.pick(SMALL_CHANNELS_NON_STEREO);
        let bitdepth = rng.pick(TYPICAL_DEPTHS_NE32);
        let blocksize = rng.range(1, 4608);
        assert_same(blocksize, channels, bitdepth);
    }
}

/// C2: non-stereo, bitdepth == 32 exactly — the `bitdepth != 32` predicate is
/// dead here because it is multiplied by `(channels == 2)` = 0.
#[test]
fn cfg_c2_non_stereo_depth32_dead_predicate() {
    let mut rng = Rng::new(0xC002);
    for _ in 0..ITERS {
        let channels = rng.pick(SMALL_CHANNELS_NON_STEREO);
        let blocksize = rng.range(1, 4608);
        assert_same(blocksize, channels, 32);
    }
}

/// C3: stereo, bitdepth != 32 — the `+1` per-sample correction IS applied.
#[test]
fn cfg_c3_stereo_typical() {
    let mut rng = Rng::new(0xC003);
    for _ in 0..ITERS {
        let bitdepth = rng.pick(TYPICAL_DEPTHS_NE32);
        let blocksize = rng.range(1, 4608);
        assert_same(blocksize, 2, bitdepth);
    }
}

/// C4: stereo, bitdepth == 32 — correction suppressed.
#[test]
fn cfg_c4_stereo_depth32() {
    let mut rng = Rng::new(0xC004);
    for _ in 0..ITERS {
        let blocksize = rng.range(1, 4608);
        assert_same(blocksize, 2, 32);
    }
}

/// C5: blocksize == 0, both channel modes x both depth modes.
#[test]
fn cfg_c5_blocksize_zero() {
    let mut rng = Rng::new(0xC005);
    for _ in 0..ITERS {
        // stereo / non-stereo x depth==32 / depth!=32
        assert_same(0, 2, 32);
        assert_same(0, 2, any_depth_ne_32(&mut rng));
        assert_same(0, any_non_stereo(&mut rng), 32);
        assert_same(0, any_non_stereo(&mut rng), any_depth_ne_32(&mut rng));
        // and with small typical values too
        assert_same(0, rng.range(0, 16), rng.range(0, 40));
    }
}

/// C6: channels == 0 — non-stereo sub-case that also zeroes term1.
#[test]
fn cfg_c6_channels_zero() {
    let mut rng = Rng::new(0xC006);
    for _ in 0..ITERS {
        assert_same(rng.next_u32(), 0, 32);
        assert_same(rng.next_u32(), 0, any_depth_ne_32(&mut rng));
        assert_same(rng.range(0, 4608), 0, rng.range(0, 40));
    }
}

/// C7: channels == 1 (one below the stereo boundary).
#[test]
fn cfg_c7_channels_one() {
    let mut rng = Rng::new(0xC007);
    for _ in 0..ITERS {
        assert_same(rng.next_u32(), 1, 32);
        assert_same(rng.next_u32(), 1, any_depth_ne_32(&mut rng));
        assert_same(rng.range(0, 4608), 1, rng.range(0, 40));
    }
}

/// C8: channels == 3 (one above the stereo boundary).
#[test]
fn cfg_c8_channels_three() {
    let mut rng = Rng::new(0xC008);
    for _ in 0..ITERS {
        assert_same(rng.next_u32(), 3, 32);
        assert_same(rng.next_u32(), 3, any_depth_ne_32(&mut rng));
        assert_same(rng.range(0, 4608), 3, rng.range(0, 40));
    }
}

/// C9: bitdepth == 0, both channel modes.
#[test]
fn cfg_c9_bitdepth_zero() {
    let mut rng = Rng::new(0xC009);
    for _ in 0..ITERS {
        assert_same(rng.next_u32(), 2, 0);
        assert_same(rng.next_u32(), any_non_stereo(&mut rng), 0);
        assert_same(rng.range(0, 4608), rng.range(0, 16), 0);
    }
}

/// C10: bitdepth 31 / 33 — one step either side of the 32 boundary.
#[test]
fn cfg_c10_bitdepth_off_by_one() {
    let mut rng = Rng::new(0xC010);
    for _ in 0..ITERS {
        for &d in &[31u32, 33] {
            assert_same(rng.next_u32(), 2, d);
            assert_same(rng.next_u32(), any_non_stereo(&mut rng), d);
            assert_same(rng.range(0, 4608), rng.range(0, 16), d);
        }
    }
}

/// C11: blocksize == 1 — the division truncates hard.
#[test]
fn cfg_c11_blocksize_one() {
    let mut rng = Rng::new(0xC011);
    for _ in 0..ITERS {
        assert_same(1, 2, 32);
        assert_same(1, 2, any_depth_ne_32(&mut rng));
        assert_same(1, any_non_stereo(&mut rng), 32);
        assert_same(1, any_non_stereo(&mut rng), any_depth_ne_32(&mut rng));
        assert_same(1, rng.range(0, 16), rng.range(0, 40));
    }
}

/// C12: many channels (3..=255), exercising `channels * (channels != 2)` and
/// the `18 + channels` tail.
#[test]
fn cfg_c12_many_channels() {
    let mut rng = Rng::new(0xC012);
    for _ in 0..ITERS {
        let channels = rng.range(3, 255);
        let bitdepth = rng.pick(&[8, 16, 24, 32]);
        let blocksize = rng.range(1, 4608);
        assert_same(blocksize, channels, bitdepth);
    }
    // Deterministic sweep of the whole 0..=255 channel range as well.
    for channels in 0u32..=255 {
        for &bitdepth in &[0u32, 8, 16, 24, 31, 32, 33] {
            assert_same(4096, channels, bitdepth);
        }
    }
}

/// C13: large but non-wrapping shape.
#[test]
fn cfg_c13_large_non_wrapping() {
    let mut rng = Rng::new(0xC013);
    for _ in 0..ITERS {
        let blocksize = rng.range(1, 65_535);
        let bitdepth = rng.pick(&[8, 16, 24, 32]);
        let channels = rng.pick(&[1, 2, 3, 4]);
        assert_same(blocksize, channels, bitdepth);
    }
}

/// C14: overflow shape, non-stereo — `blocksize * bitdepth * channels` wraps.
#[test]
fn cfg_c14_overflow_non_stereo() {
    let mut rng = Rng::new(0xC014);
    for _ in 0..ITERS {
        // Pick factors whose product is comfortably above 2^32.
        let blocksize = rng.range(1 << 16, u32::MAX);
        let bitdepth = rng.range(1 << 8, 1 << 16);
        let channels = {
            let c = rng.range(3, 1 << 12);
            if c == 2 { 3 } else { c }
        };
        // Sanity: this row is only meaningful if it really overflows.
        let wide = (blocksize as u128) * (bitdepth as u128) * (channels as u128);
        assert!(wide > u32::MAX as u128, "row C14 input did not overflow");
        assert_same(blocksize, channels, bitdepth);
    }
    // A few pinned overflow triples from ERRORS.md E13.
    assert_same(0x1_0000, 0x100, 0x100);
    assert_same(u32::MAX, 3, 24);
    assert_same(0xFFFF, 0xFFFF, 0xFFFF);
}

/// C15: overflow shape, stereo — term2 + term3 wraps.
#[test]
fn cfg_c15_overflow_stereo() {
    let mut rng = Rng::new(0xC015);
    for _ in 0..ITERS {
        let blocksize = rng.range(1 << 16, u32::MAX);
        let bitdepth = rng.range(1 << 8, u32::MAX);
        assert_same(blocksize, 2, bitdepth);
    }
    assert_same(u32::MAX, 2, u32::MAX);
    assert_same(u32::MAX, 2, 32);
    assert_same(0x8000_0000, 2, 2);
}

/// C16: the `+7` itself overflows — the inner sum lands in
/// `UINT32_MAX-6 ..= UINT32_MAX`, so `+7` wraps past zero and the division
/// yields a tiny number.
#[test]
fn cfg_c16_plus7_overflow() {
    // Construct exact hits analytically for the non-stereo branch, where the
    // inner sum is `blocksize * bitdepth * channels` (mod 2^32).
    //
    // Target sums: 0xFFFF_FFF9 ..= 0xFFFF_FFFF (7 values that make +7 wrap).
    let mut hits = 0usize;
    for target in 0xFFFF_FFF9u32..=0xFFFF_FFFF {
        // With channels = 1 and bitdepth = 1, the inner sum is exactly blocksize.
        assert_same(target, 1, 1);
        // Confirm the C really wraps here: (target + 7) mod 2^32 is small, so
        // the byte count must be tiny rather than huge.
        let got = call_c(target, 1, 1);
        let bytes = got.wrapping_sub(18).wrapping_sub(1);
        assert!(
            bytes < 1,
            "expected +7 to wrap to a tiny byte count, got {bytes}"
        );
        hits += 1;
    }
    assert_eq!(hits, 7);

    // Stereo branch: inner sum = blocksize*bitdepth + blocksize*(bitdepth+1)
    //                          = blocksize*(2*bitdepth + 1)   (mod 2^32)
    // With bitdepth = 0 (!= 32) this is just blocksize.
    for target in 0xFFFF_FFF9u32..=0xFFFF_FFFF {
        assert_same(target, 2, 0);
    }

    // Randomized search for additional wrap-adjacent inner sums.
    let mut rng = Rng::new(0xC016);
    for _ in 0..ITERS {
        let bitdepth = rng.range(1, 64);
        let channels = {
            let c = rng.range(1, 64);
            if c == 2 { 1 } else { c }
        };
        let prod = bitdepth.wrapping_mul(channels);
        if prod == 0 {
            continue;
        }
        // Choose blocksize so that blocksize*prod is near 2^32.
        let blocksize = (0x1_0000_0000u64 / prod as u64) as u32;
        for delta in 0..4u32 {
            assert_same(blocksize.wrapping_sub(delta), channels, bitdepth);
            assert_same(blocksize.wrapping_add(delta), channels, bitdepth);
        }
    }
}

/// C17: the final `18 + channels + bytes` addition overflows.
#[test]
fn cfg_c17_final_add_overflow() {
    // channels near UINT32_MAX makes `18 + channels` alone wrap.
    for c in [
        u32::MAX,
        u32::MAX - 1,
        u32::MAX - 17,
        u32::MAX - 18,
        u32::MAX - 19,
        0xFFFF_FFEE,
    ] {
        for &bitdepth in &[0u32, 1, 8, 31, 32, 33, u32::MAX] {
            for &blocksize in &[0u32, 1, 2, 4096, u32::MAX] {
                assert_same(blocksize, c, bitdepth);
            }
        }
    }

    // Large byte count plus large channels.
    let mut rng = Rng::new(0xC017);
    for _ in 0..ITERS {
        let channels = rng.range(0xFFFF_0000, u32::MAX);
        assert_same(rng.next_u32(), channels, rng.next_u32());
    }
}

/// C18: UINT32_MAX in each argument position independently.
#[test]
fn cfg_c18_u32_max_positions() {
    let mut rng = Rng::new(0xC018);
    for _ in 0..ITERS {
        assert_same(u32::MAX, rng.next_u32(), rng.next_u32());
        assert_same(rng.next_u32(), u32::MAX, rng.next_u32());
        assert_same(rng.next_u32(), rng.next_u32(), u32::MAX);
        assert_same(u32::MAX, 2, rng.next_u32());
        assert_same(u32::MAX, rng.next_u32(), 32);
    }
}

/// C19: powers of two on every axis — full cross-product of `1<<k`.
#[test]
fn cfg_c19_powers_of_two() {
    let mut pows: Vec<u32> = (0..32).map(|k| 1u32 << k).collect();
    pows.push(0);
    pows.push(u32::MAX);
    for &blocksize in &pows {
        for &channels in &pows {
            for &bitdepth in &pows {
                assert_same(blocksize, channels, bitdepth);
                // Also probe one-off-power values, which straddle the
                // truncating division.
                assert_same(
                    blocksize.wrapping_sub(1),
                    channels,
                    bitdepth.wrapping_add(1),
                );
            }
        }
    }
}

/// C20: unconstrained uniform fuzz over the full u32^3 domain.
#[test]
fn cfg_c20_uniform_fuzz_full_domain() {
    let mut rng = Rng::new(0xC020);
    for _ in 0..(ITERS * 10) {
        assert_same(rng.next_u32(), rng.next_u32(), rng.next_u32());
    }
}

/// C21: stereo-forced uniform fuzz (stereo is vanishingly rare under C20).
#[test]
fn cfg_c21_uniform_fuzz_stereo() {
    let mut rng = Rng::new(0xC021);
    for _ in 0..(ITERS * 10) {
        assert_same(rng.next_u32(), 2, rng.next_u32());
    }
}

/// C22: exhaustive small cube — no randomness, dense coverage of all three
/// predicates including both sides of the 32 boundary.
#[test]
fn cfg_c22_exhaustive_small_cube() {
    for blocksize in 0u32..=63 {
        for channels in 0u32..=15 {
            for bitdepth in 0u32..=39 {
                assert_same(blocksize, channels, bitdepth);
            }
        }
    }
}

/// Heavy full-domain fuzz. Runs a large default count, overridable via
/// `HEAVY_FUZZ_ITERS` for an extended soak.
#[test]
fn heavy_fuzz_full_domain() {
    let iters: u64 = std::env::var("HEAVY_FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000_000);

    let mut rng = Rng::new(0xDEAD_BEEF);
    for _ in 0..iters {
        // Mix of fully-random and structurally-interesting draws so the heavy
        // run does not degenerate into "three huge non-stereo numbers".
        let blocksize = rng.next_u32();
        let bitdepth = rng.next_u32();
        let channels = rng.next_u32();
        assert_same(blocksize, channels, bitdepth);

        // Force the branch-relevant states periodically.
        let small_c = rng.range(0, 8);
        let small_d = rng.range(0, 40);
        let small_b = rng.range(0, 8192);
        assert_same(small_b, small_c, small_d);
        assert_same(blocksize, 2, small_d);
        assert_same(small_b, 2, bitdepth);
        assert_same(blocksize, small_c, 32);
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
//
// The C function has no rejection path (see ERRORS.md): it is total. So each
// row asserts that BOTH libraries agree on the same well-defined wrapped
// result for the invalid/edge input, i.e. neither rejects and the Rust does not
// panic, abort, or saturate where the C wraps.
// ===========================================================================

/// E1: blocksize == 0.
#[test]
fn err_e1_blocksize_zero() {
    for &channels in &[0u32, 1, 2, 3, 8, 255, u32::MAX] {
        for &bitdepth in &[0u32, 1, 8, 16, 24, 31, 32, 33, u32::MAX] {
            assert_same(0, channels, bitdepth);
        }
    }
    // Pin the documented C value: 18 + channels when blocksize == 0.
    for &channels in &[0u32, 1, 3, 8, 255] {
        assert_eq!(call_c(0, channels, 16), 18u32.wrapping_add(channels));
        assert_eq!(call_rust(0, channels, 16), 18u32.wrapping_add(channels));
    }
}

/// E2: channels == 0 — `channels * (channels != 2)` is 0, so term1 vanishes.
#[test]
fn err_e2_channels_zero() {
    for &blocksize in &[0u32, 1, 4096, u32::MAX] {
        for &bitdepth in &[0u32, 1, 16, 31, 32, 33, u32::MAX] {
            assert_same(blocksize, 0, bitdepth);
            // Documented C value: exactly 18, for every blocksize/bitdepth.
            assert_eq!(
                call_c(blocksize, 0, bitdepth),
                18,
                "channels==0 must give 18"
            );
        }
    }
    let mut rng = Rng::new(0xE002);
    for _ in 0..ITERS {
        assert_same(rng.next_u32(), 0, rng.next_u32());
    }
}

/// E3: bitdepth == 0. Note the stereo case still contributes, because
/// `(bitdepth != 32)` is 1.
#[test]
fn err_e3_bitdepth_zero() {
    for &blocksize in &[0u32, 1, 2, 7, 8, 9, 4096, u32::MAX] {
        for &channels in &[0u32, 1, 2, 3, 8, u32::MAX] {
            assert_same(blocksize, channels, 0);
        }
        // Documented C values.
        assert_eq!(
            call_c(blocksize, 3, 0),
            18u32.wrapping_add(3).wrapping_add(7 / 8)
        );
        assert_eq!(
            call_c(blocksize, 2, 0),
            18u32
                .wrapping_add(2)
                .wrapping_add(blocksize.wrapping_add(7) / 8)
        );
    }
    let mut rng = Rng::new(0xE003);
    for _ in 0..ITERS {
        assert_same(rng.next_u32(), rng.next_u32(), 0);
    }
}

/// E4: all zero.
#[test]
fn err_e4_all_zero() {
    assert_same(0, 0, 0);
    assert_eq!(call_c(0, 0, 0), 18);
    assert_eq!(call_rust(0, 0, 0), 18);
}

/// E5: blocksize == UINT32_MAX.
#[test]
fn err_e5_blocksize_max() {
    for &channels in &[0u32, 1, 2, 3, 8, 255, u32::MAX] {
        for &bitdepth in &[0u32, 1, 8, 16, 24, 31, 32, 33, u32::MAX] {
            assert_same(u32::MAX, channels, bitdepth);
        }
    }
}

/// E6: channels == UINT32_MAX (one past every valid range).
#[test]
fn err_e6_channels_max() {
    for &blocksize in &[0u32, 1, 2, 4096, u32::MAX] {
        for &bitdepth in &[0u32, 1, 8, 16, 24, 31, 32, 33, u32::MAX] {
            assert_same(blocksize, u32::MAX, bitdepth);
        }
    }
}

/// E7: bitdepth == UINT32_MAX — `bitdepth + 1` wraps to 0 inside term3.
#[test]
fn err_e7_bitdepth_max() {
    for &blocksize in &[0u32, 1, 2, 4096, u32::MAX] {
        for &channels in &[0u32, 1, 2, 3, 8, u32::MAX] {
            assert_same(blocksize, channels, u32::MAX);
        }
    }
    // Stereo + UINT32_MAX depth is the case where `bitdepth + (bitdepth != 32)`
    // wraps to zero, killing term3 entirely. Verify both agree.
    assert_same(4096, 2, u32::MAX);
    assert_same(u32::MAX, 2, u32::MAX);
}

/// E8: all three arguments UINT32_MAX.
#[test]
fn err_e8_all_max() {
    assert_same(u32::MAX, u32::MAX, u32::MAX);
}

/// E9: bitdepth == 32 exactly, stereo — the `+1` correction is suppressed.
#[test]
fn err_e9_bitdepth_32_boundary() {
    let mut rng = Rng::new(0xE009);
    for _ in 0..ITERS {
        assert_same(rng.next_u32(), 2, 32);
    }
    for &blocksize in &[0u32, 1, 2, 4096, u32::MAX] {
        assert_same(blocksize, 2, 32);
        // Documented: term3 uses bitdepth + 0, so inner sum is 2*32*blocksize.
        let expect = 18u32
            .wrapping_add(2)
            .wrapping_add(
                blocksize
                    .wrapping_mul(32)
                    .wrapping_add(blocksize.wrapping_mul(32))
                    .wrapping_add(7)
                    / 8,
            );
        assert_eq!(call_c(blocksize, 2, 32), expect);
        assert_eq!(call_rust(blocksize, 2, 32), expect);
    }
}

/// E10: bitdepth 31 / 33 with stereo — the `+1` correction IS applied on both
/// sides of the boundary.
#[test]
fn err_e10_bitdepth_off_by_one() {
    for &bitdepth in &[31u32, 33] {
        let mut rng = Rng::new(0xE010 ^ bitdepth as u64);
        for _ in 0..ITERS {
            assert_same(rng.next_u32(), 2, bitdepth);
        }
        for &blocksize in &[0u32, 1, 2, 4096, u32::MAX] {
            assert_same(blocksize, 2, bitdepth);
            let expect = 18u32.wrapping_add(2).wrapping_add(
                blocksize
                    .wrapping_mul(bitdepth)
                    .wrapping_add(blocksize.wrapping_mul(bitdepth + 1))
                    .wrapping_add(7)
                    / 8,
            );
            assert_eq!(call_c(blocksize, 2, bitdepth), expect);
            assert_eq!(call_rust(blocksize, 2, bitdepth), expect);
        }
    }
}

/// E11: channels == 2 exactly — term1 suppressed, terms 2+3 active.
#[test]
fn err_e11_channels_2_boundary() {
    let mut rng = Rng::new(0xE011);
    for _ in 0..ITERS {
        assert_same(rng.next_u32(), 2, rng.next_u32());
    }
    for &blocksize in &[0u32, 1, 2, 4096, u32::MAX] {
        for &bitdepth in &[0u32, 1, 8, 16, 24, 31, 32, 33, u32::MAX] {
            assert_same(blocksize, 2, bitdepth);
        }
    }
}

/// E12: channels 1 / 3 — term1 active, terms 2+3 suppressed.
#[test]
fn err_e12_channels_off_by_one() {
    for &channels in &[1u32, 3] {
        let mut rng = Rng::new(0xE012 ^ channels as u64);
        for _ in 0..ITERS {
            assert_same(rng.next_u32(), channels, rng.next_u32());
        }
        for &blocksize in &[0u32, 1, 2, 4096, u32::MAX] {
            for &bitdepth in &[0u32, 1, 8, 16, 24, 31, 32, 33, u32::MAX] {
                assert_same(blocksize, channels, bitdepth);
                let expect = 18u32.wrapping_add(channels).wrapping_add(
                    blocksize
                        .wrapping_mul(bitdepth)
                        .wrapping_mul(channels)
                        .wrapping_add(7)
                        / 8,
                );
                assert_eq!(call_c(blocksize, channels, bitdepth), expect);
                assert_eq!(call_rust(blocksize, channels, bitdepth), expect);
            }
        }
    }
}

/// E13: inner bit-count sum overflows 2^32 — result must be the WRAPPED sum
/// divided by 8, not a saturated value.
#[test]
fn err_e13_inner_overflow() {
    // The canonical documented triple.
    assert_same(0x1_0000, 0x100, 0x100);
    // Verify the C really wrapped: the un-wrapped value would be enormous.
    let got = call_c(0x1_0000, 0x100, 0x100);
    let wide = (0x1_0000u128 * 0x100 * 0x100 + 7) / 8;
    assert_ne!(
        got as u128,
        wide + 18 + 0x100,
        "expected wrapping, not widening"
    );

    let mut rng = Rng::new(0xE013);
    let mut overflows = 0usize;
    for _ in 0..ITERS {
        let blocksize = rng.range(1 << 12, u32::MAX);
        let bitdepth = rng.range(1 << 4, 1 << 20);
        let channels = {
            let c = rng.range(1, 1 << 10);
            if c == 2 { 1 } else { c }
        };
        if (blocksize as u128) * (bitdepth as u128) * (channels as u128) > u32::MAX as u128 {
            overflows += 1;
        }
        assert_same(blocksize, channels, bitdepth);
    }
    assert!(
        overflows > ITERS / 2,
        "row E13 should be dominated by overflowing inputs, got {overflows}"
    );
}

/// E14: the `+7` overflows — inner sum in `UINT32_MAX-6 ..= UINT32_MAX`.
#[test]
fn err_e14_plus7_overflow() {
    // channels = 1, bitdepth = 1 makes the inner sum exactly `blocksize`.
    for target in 0xFFFF_FFF9u32..=0xFFFF_FFFF {
        assert_same(target, 1, 1);
        let got = call_c(target, 1, 1);
        // (target + 7) wraps into 0..=6, so bytes == 0 and result == 18 + 1.
        assert_eq!(got, 19, "expected +7 to wrap to zero bytes for {target}");
        assert_eq!(call_rust(target, 1, 1), 19);
    }
    // One below the wrap point must NOT wrap: 0xFFFF_FFF8 + 7 = 0xFFFF_FFFF.
    assert_same(0xFFFF_FFF8, 1, 1);
    assert_eq!(call_c(0xFFFF_FFF8, 1, 1), 19u32.wrapping_add(0x1FFF_FFFF));
    assert_eq!(call_rust(0xFFFF_FFF8, 1, 1), 19u32.wrapping_add(0x1FFF_FFFF));
}

/// E15: the final `18 + channels + bytes` addition overflows.
#[test]
fn err_e15_final_add_overflow() {
    // channels == UINT32_MAX - 17 makes 18 + channels wrap to exactly 0
    // (0xFFFF_FFEE + 18 == 2^32), and blocksize == 0 makes bytes == 7/8 == 0.
    let channels = u32::MAX - 17;
    assert_same(0, channels, 16);
    assert_eq!(call_c(0, channels, 16), 0);
    assert_eq!(call_rust(0, channels, 16), 0);

    // One more channel wraps to exactly 1.
    let channels = u32::MAX - 16;
    assert_same(0, channels, 16);
    assert_eq!(call_c(0, channels, 16), 1);
    assert_eq!(call_rust(0, channels, 16), 1);

    let mut rng = Rng::new(0xE015);
    for _ in 0..ITERS {
        let c = rng.range(0xFFFF_FF00, u32::MAX);
        assert_same(rng.next_u32(), c, rng.next_u32());
    }
}

/// E16: out-of-range "enum-like" values for `channels`. A C enum/int parameter
/// accepts any `uint32_t`, so values with no valid audio interpretation are
/// real inputs the C handles and the Rust must handle identically.
#[test]
fn err_e16_out_of_range_channel_values() {
    let weird: &[u32] = &[
        0,
        1,
        2,
        3,
        4,
        8,
        9,
        16,
        0xFF,
        0x100,
        0xFFFF,
        0x1_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFE,
        u32::MAX,
    ];
    for &channels in weird {
        for &bitdepth in &[0u32, 1, 4, 8, 12, 16, 20, 24, 31, 32, 33, 64, u32::MAX] {
            for &blocksize in &[0u32, 1, 2, 7, 8, 4096, 65_535, 0x8000_0000, u32::MAX] {
                assert_same(blocksize, channels, bitdepth);
            }
        }
    }
}

/// E17: out-of-range "enum-like" values for `bitdepth`.
#[test]
fn err_e17_out_of_range_bitdepth_values() {
    let weird: &[u32] = &[
        0,
        1,
        2,
        3,
        7,
        31,
        32,
        33,
        63,
        64,
        0xFF,
        0xFFFF,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFE,
        u32::MAX,
    ];
    for &bitdepth in weird {
        for &channels in &[0u32, 1, 2, 3, 4, 8, 0xFF, u32::MAX] {
            for &blocksize in &[0u32, 1, 2, 7, 8, 4096, 65_535, 0x8000_0000, u32::MAX] {
                assert_same(blocksize, channels, bitdepth);
            }
        }
    }
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite too.
// ===========================================================================

/// Both `.so`s must export `max_size_frame` under that exact name, and the
/// symbol must be callable through `dlsym` (already proven by every test above,
/// but asserted explicitly here so a missing export fails loudly).
#[test]
fn symbol_parity_max_size_frame() {
    let l = libs();
    // Resolving again by name is the real check.
    // SAFETY: correct signature for both libraries.
    unsafe {
        let c: Symbol<MaxSizeFrameFn> = l._c_lib.get(b"max_size_frame\0").unwrap();
        let r: Symbol<MaxSizeFrameFn> = l._rust_lib.get(b"max_size_frame\0").unwrap();
        assert_eq!(c(4096, 2, 16), r(4096, 2, 16));
    }
}

/// Cross-check the C `.so`'s dynamic symbol table against the Rust `.so`'s.
/// Every symbol the C exports must be exported by Rust with the same name.
#[test]
fn symbol_parity_nm_diff() {
    fn dynamic_defined(path: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(path)
            .output();
        let out = match out {
            Ok(o) if o.status.success() => o,
            // If `nm` is unavailable in the environment, skip rather than fail.
            _ => return Vec::new(),
        };
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next().map(str::to_string))
            .collect()
    }

    let c_syms = dynamic_defined(&find_c_so());
    if c_syms.is_empty() {
        eprintln!("nm unavailable or produced no output; skipping nm diff");
        return;
    }
    let rust_syms = dynamic_defined(&find_rust_so());

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
    assert!(
        c_syms.iter().any(|s| s == "max_size_frame"),
        "sanity: C .so should export max_size_frame, got {c_syms:?}"
    );
}
