//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported
//! `gaussian_kernel` symbol and compares the destination buffer bit-for-bit
//! (including the guard region that must stay untouched).

mod common;

use common::{Rng, SEED, branches, buffer_len, expect_match, expect_match_fill, garbage_fill};

/// Randomized iteration count per row.
const N: usize = 400;
/// Fewer iterations for the rows that allocate large buffers.
const N_BIG: usize = 60;

fn seeded(row: u64) -> Rng {
    Rng::new(SEED ^ (row.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

/// Log-uniform "ordinary" radius.
fn normal_radius(rng: &mut Rng) -> f32 {
    rng.log_uniform(1e-3, 1e3)
}

// ---------------------------------------------------------------------------
// C1..C5 — small fixed sizes, randomized radius
// ---------------------------------------------------------------------------

fn fixed_size_row(row: u64, size: i32) {
    let mut rng = seeded(row);
    for _ in 0..N {
        let radius = normal_radius(&mut rng);
        expect_match(size, radius);
    }
    // Sanity: the row is not vacuous — the loop really ran.
    let b = branches(size, 1.0);
    assert!(b.stores > 0, "row C{row} (size={size}) performs no stores");
}

#[test]
fn c01_size_1_odd_minimal() {
    fixed_size_row(1, 1);
    // size==1 always normalises to exactly 1.0 for finite non-zero radius.
    let out = expect_match(1, 3.0);
    assert_eq!(out[0].to_bits(), 1.0f32.to_bits(), "size=1 must normalise to 1.0");
}

#[test]
fn c02_size_2_even_minimal_overrun() {
    fixed_size_row(2, 2);
    // 3 stores for size 2: dest[2] is the one-past-the-end element.
    assert_eq!(branches(2, 3.0).stores, 3);
}

#[test]
fn c03_size_3_odd() {
    fixed_size_row(3, 3);
    assert_eq!(branches(3, 3.0).stores, 3);
}

#[test]
fn c04_size_4_even() {
    fixed_size_row(4, 4);
    assert_eq!(branches(4, 3.0).stores, 5);
}

#[test]
fn c05_size_5_odd() {
    fixed_size_row(5, 5);
    assert_eq!(branches(5, 3.0).stores, 5);
}

// ---------------------------------------------------------------------------
// C6..C9 — randomized sizes, mixed clamped/unclamped taps
// ---------------------------------------------------------------------------

#[test]
fn c06_odd_sizes_7_to_33() {
    let mut rng = seeded(6);
    let mut saw_mixed = false;
    for _ in 0..N {
        let size = rng.range_i32(3, 16) * 2 + 1; // 7..=33 odd
        let radius = normal_radius(&mut rng);
        expect_match(size, radius);
        let b = branches(size, radius);
        if b.kept_positive > 0 && b.clamped_zero > 0 {
            saw_mixed = true;
        }
    }
    assert!(saw_mixed, "C6 never exercised a mixed clamped/unclamped kernel");
}

#[test]
fn c07_even_sizes_6_to_32_with_overrun() {
    let mut rng = seeded(7);
    let mut saw_mixed = false;
    for _ in 0..N {
        let size = rng.range_i32(3, 16) * 2; // 6..=32 even
        let radius = normal_radius(&mut rng);
        expect_match(size, radius);
        // The loop writes size+1 elements for even size.
        assert_eq!(branches(size, radius).stores, size as u64 + 1);
        let b = branches(size, radius);
        if b.kept_positive > 0 && b.clamped_zero > 0 {
            saw_mixed = true;
        }
    }
    assert!(saw_mixed, "C7 never exercised a mixed clamped/unclamped kernel");
}

#[test]
fn c08_large_odd_sizes() {
    let mut rng = seeded(8);
    for _ in 0..N_BIG {
        let size = rng.range_i32(50, 512) * 2 + 1; // 101..=1025 odd
        let radius = normal_radius(&mut rng);
        expect_match(size, radius);
    }
}

#[test]
fn c09_large_even_sizes() {
    let mut rng = seeded(9);
    for _ in 0..N_BIG {
        let size = rng.range_i32(50, 512) * 2; // 100..=1024 even
        let radius = normal_radius(&mut rng);
        expect_match(size, radius);
    }
}

// ---------------------------------------------------------------------------
// C10..C19 — radius classes
// ---------------------------------------------------------------------------

/// Drive a fixed radius (or radius generator) across randomized sizes.
fn radius_class_row(row: u64, mut make: impl FnMut(&mut Rng) -> f32) {
    let mut rng = seeded(row);
    for _ in 0..N {
        let size = rng.range_i32(1, 65);
        let radius = make(&mut rng);
        expect_match(size, radius);
    }
}

#[test]
fn c10_negative_radius() {
    radius_class_row(10, |rng| -rng.log_uniform(1e-3, 1e3));
    // Sign of radius must not change the result at all (x*x is even).
    let mut rng = seeded(1010);
    for _ in 0..N {
        let size = rng.range_i32(1, 65);
        let r = normal_radius(&mut rng);
        let pos = expect_match(size, r);
        let neg = expect_match(size, -r);
        assert_eq!(
            pos.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            neg.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "±radius must give identical kernels (size={size}, radius={r:e})"
        );
    }
}

#[test]
fn c11_radius_positive_zero() {
    radius_class_row(11, |_| 0.0f32);
    let mut rng = seeded(1011);
    for _ in 0..64 {
        let size = rng.range_i32(1, 65);
        let out = expect_match(size, 0.0);
        let b = branches(size, 0.0);
        assert!(!b.normalised, "radius=0 must skip normalisation (size={size})");
        for i in 0..common::touched_len(size) {
            assert_eq!(out[i].to_bits(), 0.0f32.to_bits(), "radius=0 tap {i} must be +0.0");
        }
    }
}

#[test]
fn c12_radius_negative_zero() {
    radius_class_row(12, |_| -0.0f32);
    let out = expect_match(9, -0.0);
    for i in 0..common::touched_len(9) {
        assert_eq!(out[i].to_bits(), 0.0f32.to_bits());
    }
}

#[test]
fn c13_radius_positive_infinity() {
    radius_class_row(13, |_| f32::INFINITY);
    // Flat kernel normalised by 1/(2*hsize+1).
    let size = 8; // even -> 9 stores, but only 8 normalised
    let out = expect_match(size, f32::INFINITY);
    let b = branches(size, f32::INFINITY);
    assert!(b.normalised);
    assert_eq!(b.kept_positive, 9, "flat kernel keeps every tap positive");
    assert_ne!(out[size as usize].to_bits(), 0.0f32.to_bits(), "overrun element written");
}

#[test]
fn c14_radius_negative_infinity() {
    radius_class_row(14, |_| f32::NEG_INFINITY);
}

#[test]
fn c15_radius_nan() {
    radius_class_row(15, |_| f32::NAN);
    let mut rng = seeded(1015);
    for _ in 0..64 {
        let size = rng.range_i32(1, 65);
        let out = expect_match(size, f32::NAN);
        let b = branches(size, f32::NAN);
        assert_eq!(b.clamped_from_nan, b.stores, "every tap must come from a NaN v");
        assert!(!b.normalised);
        for i in 0..common::touched_len(size) {
            assert_eq!(out[i].to_bits(), 0.0f32.to_bits(), "NaN radius tap {i} must clamp to +0.0");
        }
    }
    // Non-canonical NaN payloads and negative NaNs too.
    for bits in [0x7FC0_0001u32, 0xFFC0_0000, 0x7F80_0001, 0xFFFF_FFFF] {
        expect_match(7, f32::from_bits(bits));
        expect_match(8, f32::from_bits(bits));
    }
}

#[test]
fn c16_radius_subnormal() {
    radius_class_row(16, |rng| rng.subnormal_f32());
    // Smallest positive subnormal: sigma/radius overflows to +inf.
    let out = expect_match(5, f32::from_bits(1));
    for i in 0..common::touched_len(5) {
        assert_eq!(out[i].to_bits(), 0.0f32.to_bits());
    }
}

#[test]
fn c17_radius_huge_finite() {
    radius_class_row(17, |rng| rng.log_uniform(1e20, 1e38));
    let out = expect_match(5, f32::MAX);
    // rs underflows -> flat kernel -> all taps equal.
    let first = out[0].to_bits();
    for i in 0..common::touched_len(5) {
        assert_eq!(out[i].to_bits(), first, "f32::MAX radius must give a flat kernel");
    }
}

#[test]
fn c18_radius_tiny_normal() {
    radius_class_row(18, |rng| rng.log_uniform(1e-38, 1e-6));
    // Dirac-spike regime: only the centre tap survives.
    let size = 9;
    let out = expect_match(size, 1e-6);
    let b = branches(size, 1e-6);
    assert_eq!(b.kept_positive, 1, "tiny radius must leave exactly the centre tap");
    assert!(b.normalised);
    assert_eq!(out[(size / 2) as usize].to_bits(), 1.0f32.to_bits(), "spike must be exactly 1.0");
}

#[test]
fn c19_radius_on_clamp_boundary() {
    // v == 0 exactly  <=>  x*x == sigma*sigma*tetha == 5.76f  <=>  |x| == 2.4f
    // and  |x| == |r| * (sigma/radius)  =>  radius == |r| * (2/3).
    let mut rng = seeded(19);
    for _ in 0..N {
        let r = rng.range_i32(1, 32);
        let size = rng.range_i32(2 * r + 1, 2 * r + 33).min(1025);
        let radius = (r as f32) * (2.0f32 / 3.0f32);
        expect_match(size, radius);
        expect_match(size, -radius);
        // And one ULP either side of the boundary, where v flips sign.
        expect_match(size, f32::from_bits(radius.to_bits() - 1));
        expect_match(size, f32::from_bits(radius.to_bits() + 1));
    }

    // The exact-boundary tap must be stored as `+0.0` because the clamp is a
    // strict `>`. Not every integer `r` lands exactly on the boundary after
    // rounding, so count the ones that do and require the path to be reached.
    let mut exact_zero_taps = 0usize;
    for r in 1..=32i32 {
        let radius = (r as f32) * (2.0f32 / 3.0f32);
        let size = 2 * r + 1;
        let out = expect_match(size, radius);
        let hsize = size / 2;
        for idx in [(hsize - r) as usize, (hsize + r) as usize] {
            assert_eq!(
                out[idx].to_bits() & 0x8000_0000,
                0,
                "boundary tap {idx} (r={r}, radius={radius:e}) must never be negative or -0.0"
            );
            // Un-normalised value was 0 exactly => normalised value is still 0.
            if out[idx].to_bits() == 0 {
                exact_zero_taps += 1;
            }
        }
    }
    assert!(
        exact_zero_taps >= 8,
        "C19 only reached the exact v==0 clamp boundary {exact_zero_taps} times; \
         the strict-`>` path is not being exercised"
    );
}

#[test]
fn c20_radius_arbitrary_bit_patterns() {
    let mut rng = seeded(20);
    for _ in 0..2000 {
        let size = rng.range_i32(1, 65);
        let radius = rng.any_f32();
        expect_match(size, radius);
    }
}

// ---------------------------------------------------------------------------
// C21..C24 — degenerate / negative sizes on the valid path
// ---------------------------------------------------------------------------

#[test]
fn c21_size_zero() {
    let mut rng = seeded(21);
    for _ in 0..N {
        let radius = rng.any_f32();
        expect_match(0, radius);
    }
    // size==0 still stores one element and never normalises it.
    let b = branches(0, 3.0);
    assert_eq!(b.stores, 1);
    let out = expect_match(0, 3.0);
    assert_eq!(
        out[0].to_bits(),
        (1.0f32 - common::s2()).to_bits(),
        "size=0 leaves dest[0] un-normalised at 1.0 - s2"
    );
}

#[test]
fn c22_size_minus_one_truncating_division() {
    let mut rng = seeded(22);
    for _ in 0..N {
        let radius = rng.any_f32();
        expect_match(-1, radius);
    }
    // hsize = -1/2 = 0 (truncation, not floor) => exactly one store.
    assert_eq!(branches(-1, 3.0).stores, 1);
    let out = expect_match(-1, 3.0);
    assert_eq!(out[0].to_bits(), (1.0f32 - common::s2()).to_bits());
}

#[test]
fn c23_negative_sizes_do_nothing() {
    let mut rng = seeded(23);
    for _ in 0..N {
        let size = rng.range_i32(-4096, -2);
        let radius = rng.any_f32();
        let len = buffer_len(size);
        let fill = garbage_fill(&mut rng, len);
        let out = expect_match_fill(size, radius, &fill);
        assert_eq!(
            out.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            fill.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "size={size} must leave the buffer untouched"
        );
        assert_eq!(branches(size, radius).stores, 0);
    }
}

#[test]
fn c24_extreme_negative_sizes() {
    let mut rng = seeded(24);
    for size in [i32::MIN, i32::MIN + 1, i32::MIN + 2, -2_000_000_000, -1_000_000_001, -3, -2] {
        for _ in 0..32 {
            let radius = rng.any_f32();
            let len = buffer_len(size);
            let fill = garbage_fill(&mut rng, len);
            let out = expect_match_fill(size, radius, &fill);
            assert_eq!(
                out.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                fill.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "size={size} must leave the buffer untouched"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C25..C28 — buffer shapes, statefulness, full sweep
// ---------------------------------------------------------------------------

#[test]
fn c25_garbage_prefilled_buffers() {
    let mut rng = seeded(25);
    for _ in 0..N * 2 {
        let size = rng.range_i32(-4, 65);
        let radius = rng.any_f32();
        let len = buffer_len(size);
        let fill = garbage_fill(&mut rng, len);
        expect_match_fill(size, radius, &fill);
    }
}

#[test]
fn c26_guard_region_untouched() {
    let mut rng = seeded(26);
    for _ in 0..N {
        let size = rng.range_i32(0, 65);
        let radius = normal_radius(&mut rng);
        let len = buffer_len(size);
        let fill = garbage_fill(&mut rng, len);
        let out = expect_match_fill(size, radius, &fill);
        // Anything beyond `touched_len` must be byte-identical to the fill in
        // BOTH implementations (expect_match_fill already proved C == Rust).
        for i in common::touched_len(size)..len {
            assert_eq!(
                out[i].to_bits(),
                fill[i].to_bits(),
                "size={size}: guard element {i} was written (touched_len={})",
                common::touched_len(size)
            );
        }
        // And for even positive size, dest[size] MUST have been written.
        if size > 0 && size % 2 == 0 {
            assert_eq!(
                common::touched_len(size),
                size as usize + 1,
                "even size must write one past the end"
            );
        }
    }
}

#[test]
fn c27_repeated_invocation_no_hidden_state() {
    let mut rng = seeded(27);
    let pair = common::pair();
    for _ in 0..N {
        let s1 = rng.range_i32(-4, 65);
        let s2v = rng.range_i32(-4, 65);
        let r1 = rng.any_f32();
        let r2 = rng.any_f32();
        let len = buffer_len(s1).max(buffer_len(s2v));
        let fill = garbage_fill(&mut rng, len);
        let mut cbuf = fill.clone();
        let mut rbuf = fill.clone();
        unsafe {
            (pair.c.gaussian_kernel)(cbuf.as_mut_ptr(), s1, r1);
            (pair.c.gaussian_kernel)(cbuf.as_mut_ptr(), s2v, r2);
            (pair.rs.gaussian_kernel)(rbuf.as_mut_ptr(), s1, r1);
            (pair.rs.gaussian_kernel)(rbuf.as_mut_ptr(), s2v, r2);
        }
        for i in 0..len {
            assert_eq!(
                cbuf[i].to_bits(),
                rbuf[i].to_bits(),
                "divergence after two calls at {i}: (s1={s1}, r1={r1:e}) then (s2={s2v}, r2={r2:e})\n\
                 C=0x{:08X} Rust=0x{:08X}",
                cbuf[i].to_bits(),
                rbuf[i].to_bits()
            );
        }
    }
}

#[test]
fn c28_full_randomized_sweep() {
    let mut rng = seeded(28);
    for _ in 0..20_000 {
        let size = rng.range_i32(-8, 129);
        // Mix radius classes so every branch gets hit in the sweep.
        let radius = match rng.next_u32() % 8 {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => rng.subnormal_f32(),
            6 => rng.any_f32(),
            _ => {
                let m = rng.log_uniform(1e-6, 1e6);
                if rng.next_u32() & 1 == 0 { m } else { -m }
            }
        };
        expect_match(size, radius);
    }
}
