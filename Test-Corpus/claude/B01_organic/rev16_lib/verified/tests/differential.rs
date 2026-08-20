//! Differential tests: C reference `.so` vs Rust translation `.so`.
//!
//! Both libraries are loaded at runtime with `libloading` and every call goes
//! through the exported `rev16` symbol. The Rust implementation is **never**
//! called directly as a Rust function — it is always reached through the
//! `cdylib`'s `#[no_mangle] extern "C"` export, exactly as an external C caller
//! would, so the export wrapper and its ABI are under test too.
//!
//! Layout:
//!   * `Libs` — loads both `.so` files and exposes raw `extern "C"` fn pointers.
//!   * `Rng`  — deterministic SplitMix64, fixed seed, for reproducible
//!              property-style testing.
//!   * `config_*` tests — Phase B, one per row of `CONFIGS.md`.
//!   * `error_*`  tests — Phase C, one per applicable row of `ERRORS.md`.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

/// The C ABI of the one and only public entry point:
/// `uint32_t rev16(uint32_t a);`
type Rev16Fn = unsafe extern "C" fn(u32) -> u32;

struct Libs {
    // Kept alive so the raw fn pointers below stay valid for the whole test.
    _c_lib: libloading::Library,
    _rust_lib: libloading::Library,
    c_rev16: Rev16Fn,
    rust_rev16: Rev16Fn,
}

/// Directory containing the built Rust `cdylib`.
///
/// The test executable lives at `target/<profile>/deps/<name>-<hash>`, so the
/// profile directory is two levels up. This makes the tests work unchanged
/// under both `cargo test` and `cargo test --release`.
fn rust_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile> dir")
        .to_path_buf()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let candidates = [
        manifest_dir().join("c_src/build/libtranslated_rust.so"),
        manifest_dir().join("c_src/build/libc_src.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    // Fall back to any .so produced by the CMake build.
    let build = manifest_dir().join("c_src/build");
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                return p;
            }
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\nSearched: {candidates:?} and {build:?}"
    );
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    // `[lib] name = "rev16_lib"` => librev16_lib.so
    let p = rust_profile_dir().join("librev16_lib.so");
    if p.is_file() {
        return p;
    }
    panic!(
        "Rust shared library not found at {p:?}. Build it with:\n  cargo build --no-default-features"
    );
}

impl Libs {
    fn load() -> Self {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c_lib = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("failed to dlopen C lib {c_path:?}: {e}"));
            let rust_lib = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("failed to dlopen Rust lib {rust_path:?}: {e}"));

            // Resolve by exact symbol name from the C header. If the Rust
            // `.so` were missing the `#[no_mangle]` export, this would fail.
            let c_sym: libloading::Symbol<Rev16Fn> = c_lib
                .get(b"rev16\0")
                .expect("C .so does not export `rev16`");
            let rust_sym: libloading::Symbol<Rev16Fn> = rust_lib
                .get(b"rev16\0")
                .expect("Rust .so does not export `rev16` (missing #[no_mangle]?)");

            let c_rev16 = *c_sym;
            let rust_rev16 = *rust_sym;

            Self {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_rev16,
                rust_rev16,
            }
        }
    }

    #[inline(always)]
    fn c(&self, a: u32) -> u32 {
        unsafe { (self.c_rev16)(a) }
    }

    #[inline(always)]
    fn rust(&self, a: u32) -> u32 {
        unsafe { (self.rust_rev16)(a) }
    }

    /// Call both implementations through the FFI boundary and assert the
    /// returned bytes are identical.
    #[inline(always)]
    fn assert_same(&self, a: u32, ctx: &str) -> u32 {
        let c = self.c(a);
        let r = self.rust(a);
        assert_eq!(
            c, r,
            "DIVERGENCE [{ctx}]: rev16(0x{a:08X}) -> C=0x{c:08X} Rust=0x{r:08X}"
        );
        // Byte-for-byte comparison of the raw return value, not just numeric
        // equality (guards against any width/endianness discrepancy).
        assert_eq!(
            c.to_ne_bytes(),
            r.to_ne_bytes(),
            "BYTE DIVERGENCE [{ctx}]: rev16(0x{a:08X})"
        );
        c
    }

    /// Assert equality over a whole set of inputs.
    fn assert_same_all<I: IntoIterator<Item = u32>>(&self, inputs: I, ctx: &str) -> usize {
        let mut n = 0usize;
        for a in inputs {
            self.assert_same(a, ctx);
            n += 1;
        }
        assert!(n > 0, "test [{ctx}] executed zero inputs");
        n
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed => reproducible)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    #[inline(always)]
    fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 32) as u16
    }
}

/// Number of randomized inputs per property-style row.
const N_RANDOM: usize = 20_000;

/// Fixed seed so every run is reproducible.
const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// The distinct classes of value for a 16-bit half that the four swap stages
/// of `rev16` are sensitive to (stage granularity 1, 2, 4 and 8 bits).
fn high_half_classes() -> Vec<u32> {
    vec![
        0x0000, // empty
        0xFFFF, // all ones
        0xAAAA, 0x5555, // stage-1 granularity
        0xCCCC, 0x3333, // stage-2 granularity
        0xF0F0, 0x0F0F, // stage-3 granularity
        0xFF00, 0x00FF, // stage-4 granularity
        0x0001, 0x8000, // single bit at each end
        0x1234, 0xDEAD, 0xBEEF, // arbitrary
    ]
}

/// Compose a full 32-bit argument from a high and a low 16-bit half.
#[inline(always)]
fn compose(hi: u32, lo: u32) -> u32 {
    ((hi & 0xFFFF) << 16) | (lo & 0xFFFF)
}

// ===========================================================================
// Phase B — valid-path differential tests (one per CONFIGS.md row)
// ===========================================================================

/// C1 — low half zero, crossed with every high-half class.
#[test]
fn config_c1_low_zero_all_high_classes() {
    let libs = Libs::load();
    let inputs = high_half_classes().into_iter().map(|hi| compose(hi, 0x0000));
    let n = libs.assert_same_all(inputs, "C1 low=0 x high classes");
    assert_eq!(n, high_half_classes().len());
}

/// C2 — low half all-ones, crossed with every high-half class.
#[test]
fn config_c2_low_all_ones_all_high_classes() {
    let libs = Libs::load();
    let inputs = high_half_classes().into_iter().map(|hi| compose(hi, 0xFFFF));
    libs.assert_same_all(inputs, "C2 low=0xFFFF x high classes");
}

/// C3 — single bit set at each of the 16 low positions, crossed with every
/// high-half class. A single set bit isolates exactly one path through the
/// four swap stages, which is where an off-by-one shift or a mistyped mask
/// would show up.
#[test]
fn config_c3_single_low_bit_x_high_classes() {
    let libs = Libs::load();
    let mut n = 0;
    for hi in high_half_classes() {
        for bit in 0..16u32 {
            libs.assert_same(compose(hi, 1u32 << bit), "C3 single low bit x high");
            n += 1;
        }
    }
    assert_eq!(n, high_half_classes().len() * 16);
}

/// C4 — single bit set at each of the 32 positions of the full argument.
/// Positions 16..31 must have no effect on the result at all.
#[test]
fn config_c4_single_bit_full_32_positions() {
    let libs = Libs::load();
    for bit in 0..32u32 {
        let a = 1u32 << bit;
        let got = libs.assert_same(a, "C4 single bit over full 32 positions");
        if bit >= 16 {
            assert_eq!(
                got, 0,
                "bit {bit} is above the 16-bit mask width; C discards it so result must be 0"
            );
        }
    }
}

/// C5 — stage-1 (1-bit swap) mask values `0xAAAA` / `0x5555` in the low half,
/// crossed with every high-half class.
#[test]
fn config_c5_stage1_mask_values() {
    let libs = Libs::load();
    for hi in high_half_classes() {
        for lo in [0xAAAAu32, 0x5555] {
            libs.assert_same(compose(hi, lo), "C5 stage-1 masks");
        }
    }
}

/// C6 — stage-2 (2-bit swap) mask values `0xCCCC` / `0x3333`.
#[test]
fn config_c6_stage2_mask_values() {
    let libs = Libs::load();
    for hi in high_half_classes() {
        for lo in [0xCCCCu32, 0x3333] {
            libs.assert_same(compose(hi, lo), "C6 stage-2 masks");
        }
    }
}

/// C7 — stage-3 (4-bit / nibble swap) mask values `0xF0F0` / `0x0F0F`.
#[test]
fn config_c7_stage3_mask_values() {
    let libs = Libs::load();
    for hi in high_half_classes() {
        for lo in [0xF0F0u32, 0x0F0F] {
            libs.assert_same(compose(hi, lo), "C7 stage-3 masks");
        }
    }
}

/// C8 — stage-4 (8-bit / byte swap) mask values `0xFF00` / `0x00FF`.
#[test]
fn config_c8_stage4_mask_values() {
    let libs = Libs::load();
    for hi in high_half_classes() {
        for lo in [0xFF00u32, 0x00FF] {
            libs.assert_same(compose(hi, lo), "C8 stage-4 masks");
        }
    }
}

/// C9 — per-byte-lane boundary values placed in each of the two low bytes.
#[test]
fn config_c9_byte_lane_boundary_values() {
    let libs = Libs::load();
    let bytes = [0x00u32, 0x01, 0x02, 0x7F, 0x80, 0x81, 0xFE, 0xFF];
    let mut n = 0;
    for b0 in bytes {
        for b1 in bytes {
            let lo = (b1 << 8) | b0;
            for hi in [0x0000u32, 0xFFFF, 0xA5A5] {
                libs.assert_same(compose(hi, lo), "C9 byte-lane boundaries");
                n += 1;
            }
        }
    }
    assert_eq!(n, bytes.len() * bytes.len() * 3);
}

/// C10 — bit-palindromes in the low 16 bits (`rev16(a) == a`), the fixed points
/// of the transform. Generated by mirroring a random 8-bit value.
#[test]
fn config_c10_palindromic_low_half() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xA10);
    // All 256 palindromes of the form: low byte b, high byte = bitreverse(b).
    for b in 0..256u32 {
        let rb = (b as u8).reverse_bits() as u32;
        let lo = (rb << 8) | b;
        let got = libs.assert_same(lo, "C10 palindrome");
        assert_eq!(got, lo, "palindrome must be a fixed point of rev16");
        // Same palindrome with random garbage in the ignored high half.
        let hi = rng.next_u16() as u32;
        let got2 = libs.assert_same(compose(hi, lo), "C10 palindrome + high garbage");
        assert_eq!(got2, lo);
    }
}

/// C11 — randomized low half, high half zero.
#[test]
fn config_c11_random_low_high_zero() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xC11);
    let inputs = (0..N_RANDOM).map(|_| rng.next_u16() as u32);
    let n = libs.assert_same_all(inputs, "C11 random low, high=0");
    assert_eq!(n, N_RANDOM);
}

/// C12 — randomized low half, high half all-ones.
#[test]
fn config_c12_random_low_high_ones() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xC12);
    let inputs = (0..N_RANDOM).map(|_| compose(0xFFFF, rng.next_u16() as u32));
    let n = libs.assert_same_all(inputs, "C12 random low, high=0xFFFF");
    assert_eq!(n, N_RANDOM);
}

/// C13 — fully random 32-bit arguments (both halves random).
#[test]
fn config_c13_random_full_32bit() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xC13);
    let inputs = (0..N_RANDOM).map(|_| rng.next_u32());
    let n = libs.assert_same_all(inputs, "C13 random full 32-bit");
    assert_eq!(n, N_RANDOM);
}

/// C14 — exhaustive sweep of all 2^16 high-half values against several fixed
/// low halves; proves the high half is ignored for every possible value.
#[test]
fn config_c14_exhaustive_high_half_sweep() {
    let libs = Libs::load();
    for lo in [0x0000u32, 0x0001, 0x1234, 0xAAAA, 0x8000, 0xFFFF] {
        let expected = libs.assert_same(lo, "C14 baseline");
        for hi in 0..=0xFFFFu32 {
            let got = libs.assert_same(compose(hi, lo), "C14 high-half sweep");
            assert_eq!(
                got, expected,
                "high half 0x{hi:04X} must not affect rev16(low=0x{lo:04X})"
            );
        }
    }
}

/// C15 — involution property: applying `rev16` twice returns the low 16 bits.
/// Checked through both `.so`s and cross-composed (C then Rust, Rust then C)
/// so a divergence in either direction is caught.
#[test]
fn config_c15_involution_property() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xC15);
    for _ in 0..N_RANDOM {
        let a = rng.next_u32();
        let c1 = libs.assert_same(a, "C15 first application");
        let c2 = libs.assert_same(c1, "C15 second application");
        assert_eq!(
            c2,
            a & 0xFFFF,
            "rev16(rev16(0x{a:08X})) must be 0x{:04X}",
            a & 0xFFFF
        );
        // Cross-composition across the two implementations.
        assert_eq!(libs.rust(libs.c(a)), a & 0xFFFF, "cross C->Rust");
        assert_eq!(libs.c(libs.rust(a)), a & 0xFFFF, "cross Rust->C");
    }
}

/// C16 — EXHAUSTIVE over the entire effective input domain: all 2^16 low-half
/// values (the high half is proven irrelevant by C4/C14). Together with C14
/// this is a complete proof of equivalence over all 2^32 inputs.
#[test]
fn config_c16_exhaustive_low_16_bits() {
    let libs = Libs::load();
    let mut checked = 0usize;
    for lo in 0..=0xFFFFu32 {
        let got = libs.assert_same(lo, "C16 exhaustive low 16 bits");
        // Independently cross-check against a reference bit reversal.
        assert_eq!(
            got,
            ((lo as u16).reverse_bits()) as u32,
            "rev16(0x{lo:04X}) must equal a 16-bit bit-reversal"
        );
        checked += 1;
    }
    assert_eq!(checked, 65_536);
}

/// C17 — EXHAUSTIVE over all 2^32 inputs. Opt-in (slow); enable with
/// `RUN_EXHAUSTIVE_32=1`. This is the absolute completeness check: the input
/// domain of the library is finite and this visits every point of it.
#[test]
fn config_c17_exhaustive_all_2pow32() {
    if std::env::var("RUN_EXHAUSTIVE_32").as_deref() != Ok("1") {
        eprintln!("C17 skipped (set RUN_EXHAUSTIVE_32=1 to run the full 2^32 sweep)");
        return;
    }
    let libs = Libs::load();
    let mut a: u32 = 0;
    loop {
        let c = libs.c(a);
        let r = libs.rust(a);
        if c != r {
            panic!("DIVERGENCE [C17]: rev16(0x{a:08X}) -> C=0x{c:08X} Rust=0x{r:08X}");
        }
        if a == u32::MAX {
            break;
        }
        a += 1;
    }
    eprintln!("C17: all 2^32 inputs agree");
}

// ===========================================================================
// Phase C — error/boundary-path differential tests (ERRORS.md rows)
// ===========================================================================
//
// The C source contains zero rejection paths (no error returns, asserts, range
// checks, null checks, pointers, lengths or enums -- established mechanically
// in ERRORS.md), so rows G1-G4 are structurally inapplicable and the tests
// below cover the applicable generic FFI boundaries G5-G9.

/// G5 — zero / minimum input.
#[test]
fn error_g5_zero_input() {
    let libs = Libs::load();
    let got = libs.assert_same(0x0000_0000, "G5 zero input");
    assert_eq!(got, 0x0000_0000);
}

/// G6 — maximum input `0xFFFFFFFF`.
#[test]
fn error_g6_max_input() {
    let libs = Libs::load();
    let got = libs.assert_same(0xFFFF_FFFF, "G6 max input");
    assert_eq!(
        got, 0x0000_FFFF,
        "upper 16 bits are discarded by the 16-bit masks"
    );
}

/// G7 — one step past the effective 16-bit range.
#[test]
fn error_g7_one_past_16bit_range() {
    let libs = Libs::load();
    // Exactly one past the 16-bit range.
    let got = libs.assert_same(0x0001_0000, "G7 one past 16-bit range");
    assert_eq!(got, 0x0000_0000, "bit 16 is silently discarded by C");
    // The largest in-range value, and the values either side of the boundary.
    for a in [0x0000_FFFEu32, 0x0000_FFFF, 0x0001_0000, 0x0001_0001] {
        libs.assert_same(a, "G7 around the 16-bit boundary");
    }
    // rev16(0x10001) must equal rev16(0x00001).
    assert_eq!(libs.c(0x0001_0001), libs.c(0x0000_0001));
    assert_eq!(libs.rust(0x0001_0001), libs.rust(0x0000_0001));
}

/// G8 — the upper half is ignored for arbitrary non-zero values.
#[test]
fn error_g8_upper_half_ignored() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x608);
    for _ in 0..N_RANDOM {
        let lo = rng.next_u16() as u32;
        let hi = rng.next_u16() as u32;
        let base = libs.assert_same(lo, "G8 baseline low only");
        let with_hi = libs.assert_same(compose(hi, lo), "G8 with high garbage");
        assert_eq!(
            base, with_hi,
            "high half 0x{hi:04X} must not change rev16(0x{lo:04X})"
        );
    }
}

/// G9 — the result never exceeds 16 bits, for randomized and extreme inputs.
#[test]
fn error_g9_result_fits_16_bits() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x609);
    let extremes = [0x0000_0000u32, 0xFFFF_FFFF, 0x8000_0000, 0x0000_8000];
    for a in extremes.into_iter().chain((0..N_RANDOM).map(|_| rng.next_u32())) {
        let got = libs.assert_same(a, "G9 result width");
        assert!(
            got <= 0xFFFF,
            "rev16(0x{a:08X}) = 0x{got:08X} must fit in 16 bits"
        );
    }
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite
// ===========================================================================

/// Every symbol the C `.so` exports must resolve in the Rust `.so` too.
#[test]
fn symbols_c_exports_all_resolve_in_rust() {
    let libs = Libs::load();
    // The full exported surface of the C library (see SYMBOLS.md).
    for name in [b"rev16\0".as_slice()] {
        unsafe {
            let c: Result<libloading::Symbol<Rev16Fn>, _> = libs._c_lib.get(name);
            assert!(c.is_ok(), "C .so must export {name:?}");
            let r: Result<libloading::Symbol<Rev16Fn>, _> = libs._rust_lib.get(name);
            assert!(
                r.is_ok(),
                "Rust .so is MISSING the exported symbol {name:?}"
            );
        }
    }
}
