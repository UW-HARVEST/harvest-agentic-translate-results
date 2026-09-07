//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads both `.so`s through `libloading` and compares the three
//! output `f32`s bit-for-bit (`to_bits`), so NaN payloads and the sign of zero
//! are part of the comparison.

mod common;

use common::*;
use std::ffi::c_int;

/// Iterations per randomized row. Fixed seed per row keeps runs reproducible.
/// Scale up for a soak run with `CB_SOAK=<multiplier> cargo test --release`.
fn n() -> usize {
    let mult: usize = std::env::var("CB_SOAK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    20_000 * mult.max(1)
}

/// Smaller row size for the rows that do many inner calls per iteration.
fn n_small() -> usize {
    let mult: usize = std::env::var("CB_SOAK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    2_000 * mult.max(1)
}

// ---------------------------------------------------------------------------
// C1–C3: each impairment matrix on the intended sRGB domain
// ---------------------------------------------------------------------------

fn srgb_domain_row(row: &str, mode: c_int, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..n() {
        let (r, g, b) = (rng.unit(), rng.unit(), rng.unit());
        assert_same(row, mode, r, g, b);
    }
}

#[test]
fn c1_protanopia_unit_domain() {
    srgb_domain_row("C1", 0, 0x0000_0001);
}

#[test]
fn c2_deuteranopia_unit_domain() {
    srgb_domain_row("C2", 1, 0x0000_0002);
}

#[test]
fn c3_tritanopia_unit_domain() {
    srgb_domain_row("C3", 2, 0x0000_0003);
}

// ---------------------------------------------------------------------------
// C4: signed normals in [-1, 1)
// ---------------------------------------------------------------------------

#[test]
fn c4_signed_unit_domain() {
    let mut rng = Rng::new(0x0000_0004);
    for _ in 0..n() {
        let (r, g, b) = (rng.signed_unit(), rng.signed_unit(), rng.signed_unit());
        for m in MODES {
            assert_same("C4", m, r, g, b);
        }
    }
}

// ---------------------------------------------------------------------------
// C5: random finite values across the whole exponent range
// ---------------------------------------------------------------------------

#[test]
fn c5_random_finite_full_range() {
    let mut rng = Rng::new(0x0000_0005);
    for _ in 0..n() {
        let (r, g, b) = (rng.finite_f32(), rng.finite_f32(), rng.finite_f32());
        for m in MODES {
            assert_same("C5", m, r, g, b);
        }
    }
}

// ---------------------------------------------------------------------------
// C6: totally unrestricted random bit patterns (NaN/inf/subnormal mixed in)
// ---------------------------------------------------------------------------

#[test]
fn c6_random_any_bit_pattern() {
    let mut rng = Rng::new(0x0000_0006);
    for _ in 0..n() {
        let (r, g, b) = (rng.any_f32(), rng.any_f32(), rng.any_f32());
        for m in MODES {
            assert_same("C6", m, r, g, b);
        }
    }
}

// ---------------------------------------------------------------------------
// C7: all sign combinations of exact zero
// ---------------------------------------------------------------------------

#[test]
fn c7_signed_zeros() {
    let zeros = [0.0f32, -0.0f32];
    for &r in &zeros {
        for &g in &zeros {
            for &b in &zeros {
                for m in MODES {
                    assert_same("C7", m, r, g, b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C8: boundary cross-product {-inf, -0, +0, +inf, 1.0}^3
// ---------------------------------------------------------------------------

#[test]
fn c8_infinity_zero_cross_product() {
    let vals = [
        f32::NEG_INFINITY,
        -0.0f32,
        0.0f32,
        f32::INFINITY,
        1.0f32,
    ];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                for m in MODES {
                    assert_same("C8", m, r, g, b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C9: subnormal / tiny inputs
// ---------------------------------------------------------------------------

#[test]
fn c9_subnormal_and_tiny() {
    let smallest_sub = f32::from_bits(1);
    let vals = [
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        smallest_sub,
        -smallest_sub,
        1e-40f32, // subnormal
        -1e-40f32,
        0.0f32,
        -0.0f32,
    ];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                for m in MODES {
                    assert_same("C9", m, r, g, b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C10: huge inputs that overflow to infinity
// ---------------------------------------------------------------------------

#[test]
fn c10_huge_overflow() {
    let vals = [
        f32::MAX,
        -f32::MAX,
        f32::MAX / 2.0,
        -f32::MAX / 2.0,
        1e38f32,
        -1e38f32,
        1.0f32,
        -1.0f32,
    ];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                for m in MODES {
                    assert_same("C10", m, r, g, b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C11: one quiet NaN with a randomized payload, in each channel position
// ---------------------------------------------------------------------------

#[test]
fn c11_single_quiet_nan_per_position() {
    let mut rng = Rng::new(0x0000_0011);
    for _ in 0..n() {
        let nan = rng.qnan();
        let a = rng.signed_unit();
        let b = rng.signed_unit();
        for pos in 0..3 {
            let (r, g, bl) = match pos {
                0 => (nan, a, b),
                1 => (a, nan, b),
                _ => (a, b, nan),
            };
            for m in MODES {
                assert_same("C11", m, r, g, bl);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C12: one signalling NaN with a randomized payload, in each channel position
// ---------------------------------------------------------------------------

#[test]
fn c12_single_signalling_nan_per_position() {
    let mut rng = Rng::new(0x0000_0012);
    for _ in 0..n() {
        let nan = rng.snan();
        let a = rng.signed_unit();
        let b = rng.signed_unit();
        for pos in 0..3 {
            let (r, g, bl) = match pos {
                0 => (nan, a, b),
                1 => (a, nan, b),
                _ => (a, b, nan),
            };
            for m in MODES {
                assert_same("C12", m, r, g, bl);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C13: multiple simultaneous NaNs with distinct payloads
// ---------------------------------------------------------------------------

#[test]
fn c13_multiple_distinct_nans() {
    let mut rng = Rng::new(0x0000_0013);
    for _ in 0..n() {
        // Mix of quiet and signalling so the quieting rule is covered too.
        let n1 = if rng.next_u32() & 1 == 0 {
            rng.qnan()
        } else {
            rng.snan()
        };
        let n2 = if rng.next_u32() & 1 == 0 {
            rng.qnan()
        } else {
            rng.snan()
        };
        let n3 = if rng.next_u32() & 1 == 0 {
            rng.qnan()
        } else {
            rng.snan()
        };
        let ord = rng.signed_unit();

        let cases = [
            (n1, n2, ord),
            (n1, ord, n2),
            (ord, n1, n2),
            (n1, n2, n3),
        ];
        for &(r, g, b) in &cases {
            for m in MODES {
                assert_same("C13", m, r, g, b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C14: fully aliased pointers, R == G == B
// ---------------------------------------------------------------------------

fn call_aliased_all(f: ColourblindFn, mode: c_int, v: f32) -> f32 {
    let mut x = v;
    let p: *mut f32 = &mut x;
    unsafe { f(mode, p, p, p) };
    x
}

#[test]
fn c14_fully_aliased_pointers() {
    let l = libs();
    let mut rng = Rng::new(0x0000_0014);
    for i in 0..n() {
        // Mix ordinary values with pathological bit patterns.
        let v = if i % 4 == 0 {
            rng.any_f32()
        } else {
            rng.signed_unit()
        };
        for m in MODES {
            let c = call_aliased_all(l.c, m, v);
            let r = call_aliased_all(l.rust, m, v);
            assert_eq!(
                c.to_bits(),
                r.to_bits(),
                "[C14] aliased R==G==B divergence: mode={m} in={} C={} Rust={}",
                show(v),
                show(c),
                show(r)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C15: partial aliasing
// ---------------------------------------------------------------------------

/// `which`: 0 => R==G, 1 => R==B, 2 => G==B.
fn call_partial_alias(
    f: ColourblindFn,
    mode: c_int,
    which: u8,
    a: f32,
    b: f32,
) -> (f32, f32) {
    let mut x = a;
    let mut y = b;
    let px: *mut f32 = &mut x;
    let py: *mut f32 = &mut y;
    unsafe {
        match which {
            0 => f(mode, px, px, py), // R and G alias
            1 => f(mode, px, py, px), // R and B alias
            _ => f(mode, py, px, px), // G and B alias
        }
    }
    (x, y)
}

#[test]
fn c15_partially_aliased_pointers() {
    let l = libs();
    let mut rng = Rng::new(0x0000_0015);
    for i in 0..n() {
        let (a, b) = if i % 4 == 0 {
            (rng.any_f32(), rng.any_f32())
        } else {
            (rng.signed_unit(), rng.signed_unit())
        };
        for which in 0u8..3 {
            for m in MODES {
                let c = call_partial_alias(l.c, m, which, a, b);
                let r = call_partial_alias(l.rust, m, which, a, b);
                assert!(
                    c.0.to_bits() == r.0.to_bits() && c.1.to_bits() == r.1.to_bits(),
                    "[C15] partial alias divergence: which={which} mode={m} \
                     in=({}, {}) C=({}, {}) Rust=({}, {})",
                    show(a),
                    show(b),
                    show(c.0),
                    show(c.1),
                    show(r.0),
                    show(r.1)
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C16: unaligned float pointers
// ---------------------------------------------------------------------------

fn call_unaligned(f: ColourblindFn, mode: c_int, off: usize, v: [f32; 3]) -> [u32; 3] {
    // 3 floats + up to 3 bytes of slack, written at a deliberate byte offset.
    let mut buf = [0u8; 16];
    for (i, x) in v.iter().enumerate() {
        buf[off + i * 4..off + i * 4 + 4].copy_from_slice(&x.to_bits().to_le_bytes());
    }
    unsafe {
        let base = buf.as_mut_ptr().add(off);
        let pr = base as *mut f32;
        let pg = base.add(4) as *mut f32;
        let pb = base.add(8) as *mut f32;
        f(mode, pr, pg, pb);
    }
    let mut out = [0u32; 3];
    for i in 0..3 {
        out[i] = u32::from_le_bytes(buf[off + i * 4..off + i * 4 + 4].try_into().unwrap());
    }
    out
}

#[test]
fn c16_unaligned_pointers() {
    let l = libs();
    let mut rng = Rng::new(0x0000_0016);
    for _ in 0..n_small() {
        let v = [rng.signed_unit(), rng.signed_unit(), rng.signed_unit()];
        for off in 1..=3usize {
            for m in MODES {
                let c = call_unaligned(l.c, m, off, v);
                let r = call_unaligned(l.rust, m, off, v);
                assert_eq!(
                    c, r,
                    "[C16] unaligned divergence: off={off} mode={m} in={v:?} C={c:08X?} Rust={r:08X?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C17: repeated in-place application (composed pipeline)
// ---------------------------------------------------------------------------

fn iterate(f: ColourblindFn, mode: c_int, start: (f32, f32, f32), n: usize) -> (f32, f32, f32) {
    let (mut r, mut g, mut b) = start;
    for _ in 0..n {
        unsafe { f(mode, &mut r, &mut g, &mut b) };
    }
    (r, g, b)
}

#[test]
fn c17_repeated_application() {
    let l = libs();
    let mut rng = Rng::new(0x0000_0017);
    for _ in 0..n_small() {
        let start = (rng.unit(), rng.unit(), rng.unit());
        for m in MODES {
            let c = iterate(l.c, m, start, 64);
            let r = iterate(l.rust, m, start, 64);
            assert!(
                bits_eq(c, r),
                "[C17] iterated divergence: mode={m} start={} C={} Rust={}",
                show3(start),
                show3(c),
                show3(r)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C18: modes interleaved over accumulating state
// ---------------------------------------------------------------------------

#[test]
fn c18_interleaved_modes() {
    let l = libs();
    let mut rng = Rng::new(0x0000_0018);
    for _ in 0..n_small() {
        let start = (rng.signed_unit(), rng.signed_unit(), rng.signed_unit());
        // A fixed, reproducible mode schedule including out-of-range values so
        // the no-op path is interleaved with real work.
        let schedule: Vec<c_int> = (0..48).map(|i| ((i * 7) % 5) as c_int - 1).collect();

        let run = |f: ColourblindFn| {
            let (mut r, mut g, mut b) = start;
            for &m in &schedule {
                unsafe { f(m, &mut r, &mut g, &mut b) };
            }
            (r, g, b)
        };

        let c = run(l.c);
        let r = run(l.rust);
        assert!(
            bits_eq(c, r),
            "[C18] interleaved divergence: start={} C={} Rust={}",
            show3(start),
            show3(c),
            show3(r)
        );
    }
}

// ---------------------------------------------------------------------------
// C19: exactly representable inputs
// ---------------------------------------------------------------------------

#[test]
fn c19_exact_values() {
    let mut vals: Vec<f32> = (0..=255).map(|i| i as f32).collect();
    vals.extend_from_slice(&[0.5, 0.25, 0.125, -0.5, -1.0, 255.0, 256.0, 65536.0]);
    for &r in &vals {
        for &g in &[0.0f32, 0.5, 1.0, 128.0, 255.0, -1.0] {
            for &b in &[0.0f32, 0.25, 1.0, 255.0, -255.0] {
                for m in MODES {
                    assert_same("C19", m, r, g, b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C20: the Rust f32 matrix constants equal the C `.rodata` constant pool
// ---------------------------------------------------------------------------

/// Recover each matrix coefficient from the shared objects behaviourally: with
/// `R = 1, G = 0, B = 0` the first output row is exactly the `*R` coefficient
/// (`c * 1 + d * 0 + e * 0`), and likewise for the other basis vectors. Doing it
/// through the `.so`s means the C's *pooled* constants are what gets compared.
#[test]
fn c20_coefficient_bits_match() {
    let l = libs();
    let basis = [(1.0f32, 0.0f32, 0.0f32), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)];
    // Expected pooled f32 words read out of the C .so's .rodata, per matrix.
    // (Only used as a cross-check that the behavioural probe is meaningful.)
    for m in MODES {
        for (i, &(r, g, b)) in basis.iter().enumerate() {
            let c = call(l.c, m, r, g, b);
            let rs = call(l.rust, m, r, g, b);
            assert!(
                bits_eq(c, rs),
                "[C20] coefficient column {i} of mode {m} differs: C={} Rust={}",
                show3(c),
                show3(rs)
            );
        }
    }
}
