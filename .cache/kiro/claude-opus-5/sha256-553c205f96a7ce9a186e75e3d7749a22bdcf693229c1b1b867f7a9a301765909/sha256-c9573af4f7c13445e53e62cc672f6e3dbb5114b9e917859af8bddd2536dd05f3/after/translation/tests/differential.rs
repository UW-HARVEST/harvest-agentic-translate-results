//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! The Rust implementation is NEVER called directly from this crate — it is
//! loaded from `librev16_lib.so` through the dynamic loader and invoked through
//! its `#[no_mangle] extern "C"` export, exactly as an external C consumer
//! would. That way the export wrapper itself is part of what is under test.
//!
//! Phase B rows come from `CONFIGS.md`, Phase C rows from `ERRORS.md`.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

type Rev16 = unsafe extern "C" fn(u32) -> u32;

/// Repository root (parent of the crate directory).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

/// Locate the C shared object produced by `c_src/build`.
///
/// The CMake project name is derived from the containing directory name, so the
/// file name is not fixed; find the single `lib*.so` in the build directory.
fn c_so_path() -> PathBuf {
    let build_dir = repo_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\n\
                 build the C library first:\n  cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with("lib") && name.ends_with(".so")
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

/// Locate the Rust cdylib. Prefers the profile the tests were built with, but
/// accepts either `debug` or `release` so the suite works both ways.
fn rust_so_path() -> PathBuf {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in preferred {
        let p = target.join(profile).join("librev16_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "librev16_lib.so not found under {}; run `cargo build` (and/or \
         `cargo build --release`) in translation/ first",
        target.display()
    );
}

/// Both implementations, loaded through the dynamic loader.
struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: Rev16,
    rust: Rev16,
}

impl Pair {
    fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("dlopen C .so");
            let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust .so");
            let c: Symbol<Rev16> = c_lib.get(b"rev16\0").expect("C .so exports rev16");
            let rust: Symbol<Rev16> = rust_lib.get(b"rev16\0").expect("Rust .so exports rev16");
            let (c, rust) = (*c, *rust);
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Call both exports and assert the returned 32-bit words are identical.
    #[track_caller]
    fn check(&self, input: u32, row: &str) -> u32 {
        let got_c = unsafe { (self.c)(input) };
        let got_rust = unsafe { (self.rust)(input) };
        assert_eq!(
            got_c, got_rust,
            "[{row}] divergence for input {input:#010x}: C = {got_c:#010x}, Rust = {got_rust:#010x}"
        );
        got_c
    }

    #[track_caller]
    fn check_all<I: IntoIterator<Item = u32>>(&self, inputs: I, row: &str) {
        for input in inputs {
            self.check(input, row);
        }
    }
}

/// Seeded xorshift64* — reproducible across runs and platforms.
struct Rng(u64);

impl Rng {
    const SEED: u64 = 0x2545_F491_4F6C_DD1D;

    fn new() -> Self {
        Rng(Self::SEED)
    }

    fn next_u64(&mut self) -> u64 {
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

    fn next_u16(&mut self) -> u32 {
        (self.next_u64() >> 48) as u32
    }
}

// ---------------------------------------------------------------------------
// Sanity: the symbol is reachable in both objects at all.
// ---------------------------------------------------------------------------

#[test]
fn both_objects_export_rev16() {
    let p = Pair::load();
    // A trivial call proves the export wrapper is callable across the FFI
    // boundary in both objects.
    p.check(0, "smoke");
}

// ---------------------------------------------------------------------------
// Phase B — valid-path rows from CONFIGS.md
// ---------------------------------------------------------------------------

/// CONFIGS.md row 1 — empty low half (minimum input).
#[test]
fn phase_b_row01_low_half_zero() {
    let p = Pair::load();
    assert_eq!(p.check(0x0000_0000, "row01"), 0x0000_0000);
}

/// CONFIGS.md row 2 — saturated low half.
#[test]
fn phase_b_row02_low_half_all_ones() {
    let p = Pair::load();
    assert_eq!(p.check(0x0000_FFFF, "row02"), 0x0000_FFFF);
}

/// CONFIGS.md row 3 — cardinality "one": every single-bit low half.
#[test]
fn phase_b_row03_single_bit_low_half() {
    let p = Pair::load();
    for k in 0..16u32 {
        let out = p.check(1u32 << k, "row03");
        assert_eq!(out, 1u32 << (15 - k), "row03: bit {k} did not land at {}", 15 - k);
    }
}

/// CONFIGS.md row 4 — cardinality "many", minimal: every unordered bit pair.
#[test]
fn phase_b_row04_bit_pairs_low_half() {
    let p = Pair::load();
    let mut pairs = 0usize;
    for i in 0..16u32 {
        for j in (i + 1)..16u32 {
            p.check((1u32 << i) | (1u32 << j), "row04");
            pairs += 1;
        }
    }
    assert_eq!(pairs, 120);
}

/// CONFIGS.md row 5 — randomized 16-bit inputs.
#[test]
fn phase_b_row05_random_low_half() {
    let p = Pair::load();
    let mut rng = Rng::new();
    for _ in 0..20_000 {
        p.check(rng.next_u16(), "row05");
    }
}

/// CONFIGS.md row 6 — the exact mask literals the C code uses.
#[test]
fn phase_b_row06_mask_literals() {
    let p = Pair::load();
    p.check_all(
        [
            0xAAAA, 0x5555, 0xCCCC, 0x3333, 0xF0F0, 0x0F0F, 0xFF00, 0x00FF,
        ],
        "row06",
    );
}

/// CONFIGS.md row 7 — byte-shaped inputs (stage-4 byte swap axis).
#[test]
fn phase_b_row07_byte_shapes() {
    let p = Pair::load();
    for b in 0..=0xFFu32 {
        p.check(b, "row07-low-byte");
        p.check(b << 8, "row07-high-byte");
        p.check((b << 8) | b, "row07-both-bytes");
    }
}

/// CONFIGS.md row 8 — nibble-shaped inputs (stage-3 nibble swap axis).
#[test]
fn phase_b_row08_nibble_shapes() {
    let p = Pair::load();
    for n in 0..=0xFu32 {
        for shift in [0u32, 4, 8, 12] {
            p.check(n << shift, "row08");
        }
    }
}

/// CONFIGS.md row 9 — every bit-reversal palindrome of the low half.
#[test]
fn phase_b_row09_palindromes() {
    let p = Pair::load();
    let mut count = 0usize;
    for x in 0u32..=0xFFFF {
        if (x as u16).reverse_bits() as u32 == x {
            let out = p.check(x, "row09");
            assert_eq!(out, x, "row09: palindrome {x:#06x} must map to itself");
            count += 1;
        }
    }
    // 8 independent bit pairs => 2^8 palindromes.
    assert_eq!(count, 256);
}

/// CONFIGS.md row 10 — exhaustive sweep of the significant half.
#[test]
fn phase_b_row10_exhaustive_low_half() {
    let p = Pair::load();
    for x in 0u32..=0xFFFF {
        let out = p.check(x, "row10");
        // Independent reference, not derived from either implementation.
        assert_eq!(out, (x as u16).reverse_bits() as u32, "row10: {x:#06x}");
    }
}

/// CONFIGS.md row 11 — all-ones discarded half with random low halves.
#[test]
fn phase_b_row11_high_half_all_ones() {
    let p = Pair::load();
    let mut rng = Rng::new();
    for _ in 0..20_000 {
        let low = rng.next_u16();
        let out = p.check(0xFFFF_0000 | low, "row11");
        assert_eq!(
            out,
            p.check(low, "row11-baseline"),
            "row11: high half must not affect the result"
        );
    }
}

/// CONFIGS.md row 12 — only the discarded bits vary.
#[test]
fn phase_b_row12_random_high_half_zero_low() {
    let p = Pair::load();
    let mut rng = Rng::new();
    for _ in 0..20_000 {
        let hi = rng.next_u16() << 16;
        assert_eq!(p.check(hi, "row12"), 0);
    }
}

/// CONFIGS.md row 13 — full 32-bit randomized sweep.
#[test]
fn phase_b_row13_random_full_word() {
    let p = Pair::load();
    let mut rng = Rng::new();
    for _ in 0..50_000 {
        p.check(rng.next_u32(), "row13");
    }
}

/// CONFIGS.md row 14 — whole-word boundary interpretations.
#[test]
fn phase_b_row14_word_boundaries() {
    let p = Pair::load();
    p.check_all(
        [
            0x0000_0000,
            0x0000_0001,
            0x0000_FFFF,
            0x0001_0000,
            0x7FFF_FFFF,
            0x8000_0000,
            0x8000_0001,
            0xFFFF_0000,
            0xFFFF_FFFF,
        ],
        "row14",
    );
}

/// CONFIGS.md row 15 — walking bit across the whole 32-bit word.
#[test]
fn phase_b_row15_walking_bit_full_word() {
    let p = Pair::load();
    for k in 0..32u32 {
        let out = p.check(1u32 << k, "row15");
        let expected = if k < 16 { 1u32 << (15 - k) } else { 0 };
        assert_eq!(out, expected, "row15: bit {k}");
    }
}

/// CONFIGS.md row 16 — statelessness across repeated / interleaved calls.
#[test]
fn phase_b_row16_stateless() {
    let p = Pair::load();
    let mut rng = Rng::new();
    let probes: Vec<u32> = (0..512).map(|_| rng.next_u32()).collect();
    let first: Vec<u32> = probes.iter().map(|&x| p.check(x, "row16-first")).collect();

    // Repeat each input many times.
    for (i, &x) in probes.iter().enumerate() {
        for _ in 0..8 {
            assert_eq!(p.check(x, "row16-repeat"), first[i]);
        }
    }
    // Interleave in a different (pseudo-random) order.
    for _ in 0..4_000 {
        let i = (rng.next_u32() as usize) % probes.len();
        assert_eq!(p.check(probes[i], "row16-interleaved"), first[i]);
    }
}

/// CONFIGS.md row 17 — composed double application through the `.so`.
#[test]
fn phase_b_row17_double_application() {
    let p = Pair::load();
    let mut rng = Rng::new();
    for _ in 0..20_000 {
        let x = rng.next_u32();
        let c_twice = unsafe { (p.c)((p.c)(x)) };
        let rust_twice = unsafe { (p.rust)((p.rust)(x)) };
        assert_eq!(
            c_twice, rust_twice,
            "row17: rev16(rev16({x:#010x})) diverged: C {c_twice:#010x} vs Rust {rust_twice:#010x}"
        );
        // Cross-linked composition: C then Rust must equal Rust then C.
        let mixed_a = unsafe { (p.rust)((p.c)(x)) };
        let mixed_b = unsafe { (p.c)((p.rust)(x)) };
        assert_eq!(mixed_a, mixed_b, "row17: mixed composition diverged");
        assert_eq!(mixed_a, c_twice, "row17: mixed composition != C composition");
    }
}

// ---------------------------------------------------------------------------
// Phase C — error / boundary rows from ERRORS.md
//
// The C function is total: it has no rejection path, no error sentinel and no
// errno channel (see ERRORS.md for the mechanical derivation). These tests
// therefore verify the dual property — that the Rust export likewise accepts
// every bit pattern, returns the identical word, and never panics or aborts.
// ---------------------------------------------------------------------------

/// ERRORS.md C1 / C2 / C3 — minimum, maximum, saturated-low.
#[test]
fn phase_c_c1_c2_c3_min_max_saturated() {
    let p = Pair::load();
    assert_eq!(p.check(0x0000_0000, "C1"), 0x0000_0000);
    assert_eq!(p.check(0xFFFF_FFFF, "C2"), 0x0000_FFFF);
    assert_eq!(p.check(0x0000_FFFF, "C3"), 0x0000_FFFF);
}

/// ERRORS.md C4 — one step past the 16-bit range that the masks imply.
#[test]
fn phase_c_c4_one_past_range() {
    let p = Pair::load();
    assert_eq!(p.check(0x0001_0000, "C4"), 0x0000_0000);
    // and one step below the boundary, for symmetry
    assert_eq!(p.check(0x0000_FFFF, "C4"), 0x0000_FFFF);
}

/// ERRORS.md C5 / C6 — sign-bit and `INT32_MAX` patterns.
#[test]
fn phase_c_c5_c6_sign_boundaries() {
    let p = Pair::load();
    assert_eq!(p.check(0x8000_0000, "C5"), 0x0000_0000);
    assert_eq!(p.check(0x7FFF_FFFF, "C6"), 0x0000_FFFF);
}

/// ERRORS.md C7 — values that are negative if misread as signed.
#[test]
fn phase_c_c7_negative_if_signed() {
    let p = Pair::load();
    let baseline = p.check(0x0000_0001, "C7-baseline");
    assert_eq!(baseline, 0x0000_8000);
    for x in [0x8000_0001u32, 0xFFFF_0001, 0xDEAD_0001, 0x8001_0001] {
        assert_eq!(p.check(x, "C7"), baseline);
    }
}

/// ERRORS.md C8 — out-of-range / "no valid variant" integers pushed across the
/// FFI boundary. C `uint32_t` accepts any bit pattern; so must the Rust export.
#[test]
fn phase_c_c8_out_of_range_ints_across_ffi() {
    let p = Pair::load();
    let raw: [i64; 8] = [
        -1,
        -2,
        -32_768,
        -2_147_483_648,
        2_147_483_647,
        0xDEAD_BEEF,
        0xFFFF_FFFF,
        0x1_0000_0000_u64 as i64 - 1,
    ];
    for r in raw {
        let as_u32 = r as u32;
        p.check(as_u32, "C8");
    }
    // Also sweep every i32 value in a coarse grid, reinterpreted as c_uint.
    let mut v: i64 = i32::MIN as i64;
    while v <= i32::MAX as i64 {
        p.check(v as i32 as u32, "C8-grid");
        v += 0x0010_0001;
    }
}

/// ERRORS.md C9 — walking one over all 32 positions, including dropped bits.
#[test]
fn phase_c_c9_walking_one_including_dropped_bits() {
    let p = Pair::load();
    for k in 0..32u32 {
        let out = p.check(1u32 << k, "C9");
        let expected = if k < 16 { 1u32 << (15 - k) } else { 0 };
        assert_eq!(out, expected, "C9: bit {k}");
    }
}

/// ERRORS.md C10 — only the discarded half populated ("oversized" analogue).
#[test]
fn phase_c_c10_only_discarded_half() {
    let p = Pair::load();
    assert_eq!(p.check(0xFFFF_0000, "C10"), 0x0000_0000);
    assert_eq!(p.check(0x0001_0000, "C10"), 0x0000_0000);
    assert_eq!(p.check(0xABCD_0000, "C10"), 0x0000_0000);
}

/// ERRORS.md C11 — no hidden state / no sticky error condition. Feeding the
/// "worst" inputs first must not change subsequent results in either object.
#[test]
fn phase_c_c11_no_sticky_state() {
    let p = Pair::load();
    let poison = [0xFFFF_FFFFu32, 0x8000_0000, 0xDEAD_BEEF, 0x0000_0000];
    let mut rng = Rng::new();
    for _ in 0..5_000 {
        let x = rng.next_u32();
        let clean = p.check(x, "C11-clean");
        for &bad in &poison {
            p.check(bad, "C11-poison");
        }
        assert_eq!(p.check(x, "C11-after-poison"), clean);
    }
}
