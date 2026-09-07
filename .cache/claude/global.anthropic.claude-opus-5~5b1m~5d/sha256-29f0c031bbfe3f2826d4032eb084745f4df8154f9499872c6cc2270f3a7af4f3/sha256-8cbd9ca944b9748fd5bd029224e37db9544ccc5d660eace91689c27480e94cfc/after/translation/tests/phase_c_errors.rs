//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Every test constructs the exact invalid
//! input the C rejects, calls BOTH `.so`s, and asserts they return the SAME
//! sentinel (this library's only rejection signal is the integer `0` from the
//! three `default:` arms of `collided`), or — for the total functions — the same
//! bit pattern.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

const SEED: u64 = 0xDEAD_BEEF_0BAD_F00D;

/// Every `C2_TYPE` value that has NO valid variant. `C2_TYPE` is a C enum, so
/// any `int` can cross the FFI boundary in that slot.
const BAD_TAGS: &[C2_TYPE] = &[
    2,
    3,
    4,
    7,
    16,
    100,
    255,
    256,
    1000,
    0x7FFF_FFFF,          // INT_MAX
    0x8000_0000,          // INT_MIN reinterpreted
    0xFFFF_FFFF,          // -1
    0xFFFF_FFFE,          // -2
    0xFFFF_FFFF - 1000,
    u32::MAX / 2,
];

const GOOD_TAGS: &[C2_TYPE] = &[C2_TYPE_CIRCLE, C2_TYPE_AABB];

/// A live 16-byte payload usable as either a `c2Circle` or a `c2AABB`.
fn payload() -> [u8; 16] {
    let mut b = [0u8; 16];
    for (k, f) in [1.0f32, 2.0, 3.0, 4.0].iter().enumerate() {
        b[k * 4..k * 4 + 4].copy_from_slice(&f.to_bits().to_ne_bytes());
    }
    b
}

fn both(l: &Pair, a: *const c_void, ta: C2_TYPE, b: *const c_void, tb: C2_TYPE) -> (c_int, c_int) {
    (l.c.collided_raw(a, ta, b, tb), l.rs.collided_raw(a, ta, b, tb))
}

// ===========================================================================
// Row 1 — typeA out of range (outer `default:` at lib.c:96)
// ===========================================================================

#[test]
fn err01_typeA_out_of_range() {
    let l = libs();
    let buf = payload();
    let p = buf.as_ptr() as *const c_void;
    for &ta in BAD_TAGS {
        for &tb in GOOD_TAGS.iter().chain(BAD_TAGS.iter()) {
            let (cv, rv) = both(&l, p, ta, p, tb);
            assert_eq!(
                cv, rv,
                "typeA={ta} typeB={tb}: C returned {cv}, Rust returned {rv}"
            );
            assert_eq!(cv, 0, "C must reject typeA={ta} with the 0 sentinel");
            assert_eq!(rv, 0, "Rust must reject typeA={ta} with the 0 sentinel");
        }
    }
    // Also sweep a wide swath of tag values exhaustively-by-sampling.
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..20_000 {
        let ta = rng.next_u32();
        let tb = rng.next_u32();
        let (cv, rv) = both(&l, p, ta, p, tb);
        assert_eq!(cv, rv, "typeA={ta} typeB={tb}");
        if ta > 1 {
            assert_eq!(cv, 0, "out-of-range typeA={ta} must yield 0");
        }
    }
}

// ===========================================================================
// Row 2 — typeA == CIRCLE, typeB out of range (inner `default:` at lib.c:82)
// ===========================================================================

#[test]
fn err02_circle_with_typeB_out_of_range() {
    let l = libs();
    let buf = payload();
    let p = buf.as_ptr() as *const c_void;
    for &tb in BAD_TAGS {
        let (cv, rv) = both(&l, p, C2_TYPE_CIRCLE, p, tb);
        assert_eq!(cv, rv, "typeA=CIRCLE typeB={tb}");
        assert_eq!(cv, 0, "C: CIRCLE vs bad tag {tb} must be 0");
        assert_eq!(rv, 0, "Rust: CIRCLE vs bad tag {tb} must be 0");
    }
}

// ===========================================================================
// Row 3 — typeA == AABB, typeB out of range (inner `default:` at lib.c:92)
// ===========================================================================

#[test]
fn err03_aabb_with_typeB_out_of_range() {
    let l = libs();
    let buf = payload();
    let p = buf.as_ptr() as *const c_void;
    for &tb in BAD_TAGS {
        let (cv, rv) = both(&l, p, C2_TYPE_AABB, p, tb);
        assert_eq!(cv, rv, "typeA=AABB typeB={tb}");
        assert_eq!(cv, 0, "C: AABB vs bad tag {tb} must be 0");
        assert_eq!(rv, 0, "Rust: AABB vs bad tag {tb} must be 0");
    }
}

// ===========================================================================
// Row 4 — both tags out of range (outer default wins first)
// ===========================================================================

#[test]
fn err04_both_tags_out_of_range() {
    let l = libs();
    let buf = payload();
    let p = buf.as_ptr() as *const c_void;
    for &ta in BAD_TAGS {
        for &tb in BAD_TAGS {
            let (cv, rv) = both(&l, p, ta, p, tb);
            assert_eq!(cv, rv, "typeA={ta} typeB={tb}");
            assert_eq!(cv, 0);
            assert_eq!(rv, 0);
        }
    }
}

// ===========================================================================
// Rows 5–7 — NULL pointers on paths where the C never dereferences
// ===========================================================================

#[test]
fn err05_null_A_with_bad_typeA() {
    let l = libs();
    let buf = payload();
    let p = buf.as_ptr() as *const c_void;
    for &ta in BAD_TAGS {
        for &tb in GOOD_TAGS {
            // A is NULL, but the outer `default:` returns before touching it.
            let (cv, rv) = both(&l, ptr::null(), ta, p, tb);
            assert_eq!(cv, rv, "A=NULL typeA={ta} typeB={tb}");
            assert_eq!(cv, 0);
        }
    }
}

#[test]
fn err06_null_B_with_bad_typeB() {
    let l = libs();
    let buf = payload();
    let p = buf.as_ptr() as *const c_void;
    for &ta in GOOD_TAGS {
        for &tb in BAD_TAGS {
            // B is NULL; the inner `default:` returns before touching it.
            let (cv, rv) = both(&l, p, ta, ptr::null(), tb);
            assert_eq!(cv, rv, "typeA={ta} B=NULL typeB={tb}");
            assert_eq!(cv, 0);
        }
    }
}

#[test]
fn err07_both_null_both_bad() {
    let l = libs();
    for &ta in BAD_TAGS {
        for &tb in BAD_TAGS {
            let (cv, rv) = both(&l, ptr::null(), ta, ptr::null(), tb);
            assert_eq!(cv, rv, "A=B=NULL typeA={ta} typeB={tb}");
            assert_eq!(cv, 0);
        }
    }
    // A NULL `A` with a *valid* typeA but bad typeB also never dereferences A
    // in the C (the inner switch is evaluated first on typeB's default arm)…
    // …except for the CIRCLE/CIRCLE and CIRCLE/AABB arms, which do. Those are
    // not exercised here; only the reject arms are.
    for &tb in BAD_TAGS {
        for &ta in GOOD_TAGS {
            let (cv, rv) = both(&l, ptr::null(), ta, ptr::null(), tb);
            assert_eq!(cv, rv);
            assert_eq!(cv, 0);
        }
    }
}

// ===========================================================================
// Rows 8–9 — tag/pointee mismatch (NOT an error in C: bytes are reinterpreted)
// ===========================================================================

#[test]
fn err08_09_tag_pointee_mismatch() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..20_000 {
        // 16 bytes of arbitrary content, tagged both ways.
        let mut buf = [0u8; 16];
        for k in 0..4 {
            buf[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
        }
        let mut other = [0u8; 16];
        for k in 0..4 {
            other[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
        }
        let pa = buf.as_ptr() as *const c_void;
        let pb = other.as_ptr() as *const c_void;
        for &ta in GOOD_TAGS {
            for &tb in GOOD_TAGS {
                let (cv, rv) = both(&l, pa, ta, pb, tb);
                assert_eq!(
                    cv, rv,
                    "mismatch ta={ta} tb={tb} A={buf:02x?} B={other:02x?}"
                );
            }
        }
    }
}

// ===========================================================================
// Row 10 — unaligned pointers
// ===========================================================================

#[test]
fn err10_unaligned_pointers() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..5_000 {
        let mut backing = vec![0u8; 32];
        for k in 0..8 {
            backing[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
        }
        for off in 1usize..8 {
            let pa = unsafe { backing.as_ptr().add(off) } as *const c_void;
            let pb = unsafe { backing.as_ptr().add(off + 16 - off) } as *const c_void;
            for &ta in GOOD_TAGS {
                for &tb in GOOD_TAGS {
                    let (cv, rv) = both(&l, pa, ta, pb, tb);
                    assert_eq!(cv, rv, "unaligned off={off} ta={ta} tb={tb}");
                }
            }
        }
    }
}

// ===========================================================================
// Rows 11–12 — c2Dot with quiet and signalling NaNs
// ===========================================================================

#[test]
fn err11_12_dot_quiet_and_signalling_nans() {
    let l = libs();
    let qnans = [0x7FC0_0001u32, 0xFFC0_0001, 0x7FFF_FFFF, 0x7FC0_0000];
    let snans = [0x7F80_0001u32, 0xFF80_0001, 0x7FBF_FFFF, 0xFF80_4321];
    let base = [1.0f32, 2.0, 3.0, 4.0];
    for pats in [&qnans, &snans] {
        for &pat in pats.iter() {
            let n = f32::from_bits(pat);
            for mask in 1u32..16 {
                let mut c = base;
                for k in 0..4 {
                    if mask & (1 << k) != 0 {
                        c[k] = n;
                    }
                }
                let a = v(c[0], c[1]);
                let b = v(c[2], c[3]);
                assert_f32_bits_eq!(
                    "err11+12",
                    l.c.c2Dot(a, b),
                    l.rs.c2Dot(a, b),
                    "c2Dot with NaN 0x{pat:08x} mask {mask:04b}"
                );
            }
        }
    }
}

// ===========================================================================
// Rows 13–16 — c2Dot IEEE exceptional results
// ===========================================================================

#[test]
fn err13_dot_zero_times_infinity() {
    let l = libs();
    for &z in &[0.0f32, -0.0f32] {
        for &i in &[f32::INFINITY, f32::NEG_INFINITY] {
            for &(a, b) in &[
                (v(z, z), v(i, i)),
                (v(i, i), v(z, z)),
                (v(z, 1.0), v(i, 1.0)),
                (v(1.0, z), v(1.0, i)),
            ] {
                assert_f32_bits_eq!(
                    "err13",
                    l.c.c2Dot(a, b),
                    l.rs.c2Dot(a, b),
                    "0*inf: {} . {}",
                    showv(a),
                    showv(b)
                );
            }
        }
    }
}

#[test]
fn err14_dot_inf_minus_inf() {
    let l = libs();
    let cases: &[(c2v, c2v)] = &[
        (v(1.0, 1.0), v(f32::INFINITY, f32::NEG_INFINITY)),
        (v(1.0, 1.0), v(f32::NEG_INFINITY, f32::INFINITY)),
        (v(f32::INFINITY, f32::INFINITY), v(1.0, -1.0)),
        (v(f32::INFINITY, f32::NEG_INFINITY), v(1.0, 1.0)),
        (
            v(f32::MAX, f32::MAX),
            v(f32::MAX, -f32::MAX), // overflow both ways then cancel
        ),
    ];
    for &(a, b) in cases {
        assert_f32_bits_eq!(
            "err14",
            l.c.c2Dot(a, b),
            l.rs.c2Dot(a, b),
            "inf-inf: {} . {}",
            showv(a),
            showv(b)
        );
    }
}

#[test]
fn err15_dot_overflow_to_infinity() {
    let l = libs();
    let cases: &[(c2v, c2v)] = &[
        (v(3.4e38, 0.0), v(3.4e38, 0.0)),
        (v(f32::MAX, f32::MAX), v(f32::MAX, f32::MAX)),
        (v(-f32::MAX, 0.0), v(f32::MAX, 0.0)),
        (v(1e30, 1e30), v(1e30, 1e30)),
        (v(f32::MAX, f32::MAX), v(2.0, 2.0)),
    ];
    for &(a, b) in cases {
        assert_f32_bits_eq!(
            "err15",
            l.c.c2Dot(a, b),
            l.rs.c2Dot(a, b),
            "overflow: {} . {}",
            showv(a),
            showv(b)
        );
    }
}

#[test]
fn err16_dot_underflow_to_zero() {
    let l = libs();
    let tiny = [
        1.0e-30f32,
        -1.0e-30,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-45,
        -1e-45,
        1e-22,
    ];
    for &p in &tiny {
        for &q in &tiny {
            let (a, b) = (v(p, q), v(q, p));
            assert_f32_bits_eq!(
                "err16",
                l.c.c2Dot(a, b),
                l.rs.c2Dot(a, b),
                "underflow: {} . {}",
                showv(a),
                showv(b)
            );
            let (a, b) = (v(p, p), v(q, q));
            assert_f32_bits_eq!("err16", l.c.c2Dot(a, b), l.rs.c2Dot(a, b), "underflow2");
        }
    }
}

// ===========================================================================
// Rows 17–18 — c2Maxv/c2Minv non-NaN-suppressing ternary, signed zeros
// ===========================================================================

#[test]
fn err17_minmax_nan_is_not_suppressed() {
    let l = libs();
    // The C is `a > b ? a : b`, which yields **b** whenever the comparison is
    // unordered. A NaN-suppressing f32::max would yield `a` here — this test
    // pins the C behaviour.
    for ni in 0..NAN_F32_BITS.len() {
        let n = nan(ni);
        for &f in &[1.0f32, -1.0, 0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
            // NaN in a: C must return b's component (the finite one).
            let (a, b) = (v(n, n), v(f, f));
            let cmin = l.c.c2Minv(a, b);
            let cmax = l.c.c2Maxv(a, b);
            assert_v_bits_eq!("err17/nan-in-a", cmin, l.rs.c2Minv(a, b), "min");
            assert_v_bits_eq!("err17/nan-in-a", cmax, l.rs.c2Maxv(a, b), "max");
            assert_eq!(cmin.x.to_bits(), f.to_bits(), "C min must yield b when a is NaN");
            assert_eq!(cmax.x.to_bits(), f.to_bits(), "C max must yield b when a is NaN");

            // NaN in b: C must return b's NaN (still the second operand),
            // preserving the exact payload (no quieting: it is a pure copy).
            let (a, b) = (v(f, f), v(n, n));
            let cmin = l.c.c2Minv(a, b);
            let cmax = l.c.c2Maxv(a, b);
            assert_v_bits_eq!("err17/nan-in-b", cmin, l.rs.c2Minv(a, b), "min");
            assert_v_bits_eq!("err17/nan-in-b", cmax, l.rs.c2Maxv(a, b), "max");
            assert_eq!(cmin.x.to_bits(), n.to_bits(), "C min must yield b's NaN");
            assert_eq!(cmax.x.to_bits(), n.to_bits(), "C max must yield b's NaN");
        }
    }
}

#[test]
fn err18_minmax_signed_zero_returns_b() {
    let l = libs();
    for &az in &[0.0f32, -0.0f32] {
        for &bz in &[0.0f32, -0.0f32] {
            let (a, b) = (v(az, az), v(bz, bz));
            let cmin = l.c.c2Minv(a, b);
            let cmax = l.c.c2Maxv(a, b);
            assert_v_bits_eq!("err18", cmin, l.rs.c2Minv(a, b), "min ±0");
            assert_v_bits_eq!("err18", cmax, l.rs.c2Maxv(a, b), "max ±0");
            // C's `<`/`>` are false for equal values, so b (sign included) wins.
            assert_eq!(cmin.x.to_bits(), bz.to_bits());
            assert_eq!(cmax.x.to_bits(), bz.to_bits());
        }
    }
}

// ===========================================================================
// Rows 19–20 — c2Clampv with no validation
// ===========================================================================

#[test]
fn err19_clamp_inverted_interval() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..20_000 {
        let hi = v(rng.signed(100.0), rng.signed(100.0));
        let lo = v(hi.x + 1.0 + rng.signed(50.0).abs(), hi.y + 1.0);
        let a = v(rng.signed(300.0), rng.signed(300.0));
        assert_v_bits_eq!(
            "err19",
            l.c.c2Clampv(a, lo, hi),
            l.rs.c2Clampv(a, lo, hi),
            "inverted clamp a={} lo={} hi={}",
            showv(a),
            showv(lo),
            showv(hi)
        );
    }
}

#[test]
fn err20_clamp_nan_endpoints() {
    let l = libs();
    let base = [1.0f32, 2.0, -3.0, -4.0, 5.0, 6.0];
    for mask in 1u32..64 {
        for ni in 0..NAN_F32_BITS.len() {
            let mut c = base;
            for k in 0..6 {
                if mask & (1 << k) != 0 {
                    c[k] = nan(ni + k);
                }
            }
            let (a, lo, hi) = (v(c[0], c[1]), v(c[2], c[3]), v(c[4], c[5]));
            assert_v_bits_eq!(
                "err20",
                l.c.c2Clampv(a, lo, hi),
                l.rs.c2Clampv(a, lo, hi),
                "nan clamp mask={mask:06b}"
            );
        }
    }
}

// ===========================================================================
// Rows 21–24 — c2CircletoCircle degenerate/exceptional inputs
// ===========================================================================

#[test]
fn err21_cc_negative_radius() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..20_000 {
        let ra = -(rng.signed(10.0).abs());
        let rb = if rng.below(2) == 0 { -(rng.signed(10.0).abs()) } else { rng.signed(10.0).abs() };
        let a = circle(rng.signed(20.0), rng.signed(20.0), ra);
        let b = circle(rng.signed(20.0), rng.signed(20.0), rb);
        assert_int_eq!(
            "err21",
            l.c.c2CircletoCircle(a, b),
            l.rs.c2CircletoCircle(a, b),
            "negative radii {} {}",
            show(ra),
            show(rb)
        );
    }
}

#[test]
fn err22_cc_radii_summing_to_nan() {
    let l = libs();
    // +inf + -inf ⇒ r2 = NaN ⇒ `d2 < NaN` is false ⇒ 0.
    for &(ra, rb) in &[
        (f32::INFINITY, f32::NEG_INFINITY),
        (f32::NEG_INFINITY, f32::INFINITY),
    ] {
        for &p in &[v(0.0, 0.0), v(1.0, 1.0), v(1e30, -1e30)] {
            let a = c2Circle { p, r: ra };
            let b = c2Circle { p, r: rb };
            let cv = l.c.c2CircletoCircle(a, b);
            assert_int_eq!("err22", cv, l.rs.c2CircletoCircle(a, b), "inf+(-inf) radii");
            assert_eq!(cv, 0, "d2 < NaN must be false in C");
        }
    }
}

#[test]
fn err23_cc_nan_centre() {
    let l = libs();
    for ni in 0..NAN_F32_BITS.len() {
        let n = nan(ni);
        for &r in &[0.0f32, 1.0, 1e30, f32::INFINITY] {
            for (a, b) in [
                (circle(n, 0.0, r), circle(0.0, 0.0, r)),
                (circle(0.0, n, r), circle(0.0, 0.0, r)),
                (circle(0.0, 0.0, r), circle(n, 0.0, r)),
                (circle(0.0, 0.0, r), circle(0.0, n, r)),
                (circle(n, n, r), circle(n, n, r)),
            ] {
                let cv = l.c.c2CircletoCircle(a, b);
                assert_int_eq!("err23", cv, l.rs.c2CircletoCircle(a, b), "nan centre");
                assert_eq!(cv, 0, "NaN < r2 must be false in C");
            }
        }
    }
}

#[test]
fn err24_cc_zero_radius_both() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..10_000 {
        let p = v(rng.signed(100.0), rng.signed(100.0));
        for &(ra, rb) in &[(0.0f32, 0.0f32), (0.0, -0.0), (-0.0, -0.0)] {
            // Identical centres: d2 == 0, r2 == 0, so `0 < 0` is false.
            let a = c2Circle { p, r: ra };
            let b = c2Circle { p, r: rb };
            let cv = l.c.c2CircletoCircle(a, b);
            assert_int_eq!("err24", cv, l.rs.c2CircletoCircle(a, b), "zero radii");
            assert_eq!(cv, 0, "coincident zero-radius circles must not collide");
        }
    }
}

// ===========================================================================
// Rows 25–28 — c2CircletoAABB degenerate/exceptional inputs
// ===========================================================================

#[test]
fn err25_ca_inverted_box() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..20_000 {
        let max = v(rng.signed(50.0), rng.signed(50.0));
        let min = v(max.x + 1.0 + rng.signed(20.0).abs(), max.y + 1.0);
        let a = circle(rng.signed(80.0), rng.signed(80.0), rng.signed(20.0));
        let b = c2AABB { min, max };
        assert_int_eq!(
            "err25",
            l.c.c2CircletoAABB(a, b),
            l.rs.c2CircletoAABB(a, b),
            "inverted box min={} max={}",
            showv(min),
            showv(max)
        );
    }
}

#[test]
fn err26_ca_nan_everywhere() {
    let l = libs();
    let base = [0.5f32, 0.5, 1.0, 0.0, 0.0, 2.0, 2.0];
    for mask in 1u32..128 {
        for ni in 0..NAN_F32_BITS.len() {
            let mut c = base;
            for k in 0..7 {
                if mask & (1 << k) != 0 {
                    c[k] = nan(ni + k);
                }
            }
            let a = circle(c[0], c[1], c[2]);
            let b = aabb(c[3], c[4], c[5], c[6]);
            assert_int_eq!(
                "err26",
                l.c.c2CircletoAABB(a, b),
                l.rs.c2CircletoAABB(a, b),
                "nan mask={mask:07b}"
            );
        }
    }
}

#[test]
fn err27_ca_negative_radius_still_collides() {
    let l = libs();
    let bx = aabb(0.0, 0.0, 1.0, 1.0);
    for &r in &[-0.5f32, -1.0, -1e-20, -1e20, -f32::MIN_POSITIVE] {
        for &p in &[v(0.5, 0.5), v(1.2, 0.5), v(-3.0, 4.0), v(0.0, 0.0)] {
            let a = c2Circle { p, r };
            assert_int_eq!(
                "err27",
                l.c.c2CircletoAABB(a, bx),
                l.rs.c2CircletoAABB(a, bx),
                "negative radius {}",
                show(r)
            );
        }
    }
}

#[test]
fn err28_ca_zero_radius_never_collides() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..10_000 {
        let min = v(rng.signed(50.0), rng.signed(50.0));
        let max = v(min.x + 5.0, min.y + 5.0);
        let p = v(min.x + 2.5, min.y + 2.5); // strictly inside
        for &r in &[0.0f32, -0.0f32] {
            let a = c2Circle { p, r };
            let b = c2AABB { min, max };
            let cv = l.c.c2CircletoAABB(a, b);
            assert_int_eq!("err28", cv, l.rs.c2CircletoAABB(a, b), "zero radius");
            assert_eq!(cv, 0, "d2 < 0 is impossible, so C must return 0");
        }
    }
}

// ===========================================================================
// Rows 29–31 — c2AABBtoAABB degenerate/exceptional inputs
// ===========================================================================

#[test]
fn err29_aa_inverted_boxes() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..20_000 {
        let p = v(rng.signed(50.0), rng.signed(50.0));
        let q = v(rng.signed(50.0), rng.signed(50.0));
        for (a, b) in [
            (aabb(p.x, p.y, p.x - 1.0, p.y - 1.0), aabb(q.x, q.y, q.x + 1.0, q.y + 1.0)),
            (aabb(p.x, p.y, p.x + 1.0, p.y + 1.0), aabb(q.x, q.y, q.x - 1.0, q.y - 1.0)),
            (aabb(p.x, p.y, p.x - 1.0, p.y - 1.0), aabb(q.x, q.y, q.x - 1.0, q.y - 1.0)),
        ] {
            assert_int_eq!(
                "err29",
                l.c.c2AABBtoAABB(a, b),
                l.rs.c2AABBtoAABB(a, b),
                "inverted boxes"
            );
        }
    }
}

#[test]
fn err30_aa_nan_collides_with_everything() {
    let l = libs();
    // Every `<` is false when a NaN is involved, so all four dN are 0 and the C
    // returns 1. Verified for all 255 non-empty coordinate subsets.
    let base = [0.0f32, 0.0, 1.0, 1.0, 10.0, 10.0, 11.0, 11.0]; // disjoint boxes
    for mask in 1u32..256 {
        for ni in 0..NAN_F32_BITS.len() {
            let mut c = base;
            for k in 0..8 {
                if mask & (1 << k) != 0 {
                    c[k] = nan(ni + k);
                }
            }
            let a = aabb(c[0], c[1], c[2], c[3]);
            let b = aabb(c[4], c[5], c[6], c[7]);
            let cv = l.c.c2AABBtoAABB(a, b);
            assert_int_eq!("err30", cv, l.rs.c2AABBtoAABB(a, b), "nan mask={mask:08b}");
        }
    }
    // The all-NaN case must be exactly 1.
    let n = nan(0);
    let a = aabb(n, n, n, n);
    let b = aabb(n, n, n, n);
    assert_eq!(l.c.c2AABBtoAABB(a, b), 1, "all-NaN boxes must collide in C");
    assert_eq!(l.rs.c2AABBtoAABB(a, b), 1);
}

#[test]
fn err31_aa_touching_counts_as_collision() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..10_000 {
        let p = v(rng.signed(100.0), rng.signed(100.0));
        let a = aabb(p.x, p.y, p.x + 2.0, p.y + 2.0);
        let b = aabb(a.max.x, a.min.y, a.max.x + 2.0, a.max.y);
        let cv = l.c.c2AABBtoAABB(a, b);
        assert_int_eq!("err31", cv, l.rs.c2AABBtoAABB(a, b), "touching");
        assert_eq!(cv, 1, "strict `<` means touching boxes collide");
    }
}

// ===========================================================================
// Rows 32–34 — c2V / c2Sub bit-level edge cases
// ===========================================================================

#[test]
fn err32_c2v_does_not_quiet_signalling_nans() {
    let l = libs();
    let snans = [0x7F80_0001u32, 0xFF80_0001, 0x7FBF_FFFF, 0xFF80_4321, 0x7F80_0FFF];
    for &pat in &snans {
        let n = f32::from_bits(pat);
        let cv = l.c.c2V(n, n);
        assert_v_bits_eq!("err32", cv, l.rs.c2V(n, n), "c2V sNaN 0x{pat:08x}");
        // The C is a plain field copy, so the sNaN survives unchanged.
        assert_eq!(cv.x.to_bits(), pat, "c2V must not quiet the sNaN");
        assert_eq!(cv.y.to_bits(), pat);
    }
    // Exhaustive over the low bits of the NaN payload space.
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..50_000 {
        let pat = 0x7F80_0000u32 | (rng.next_u32() & 0x007F_FFFF).max(1);
        let n = f32::from_bits(pat);
        assert_v_bits_eq!("err32/rand", l.c.c2V(n, -n), l.rs.c2V(n, -n), "c2V");
    }
}

#[test]
fn err33_sub_inf_minus_inf() {
    let l = libs();
    for &i in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &j in &[f32::INFINITY, f32::NEG_INFINITY] {
            for (a, b) in [
                (v(i, j), v(i, j)),
                (v(i, 1.0), v(j, 1.0)),
                (v(1.0, i), v(1.0, j)),
                (v(i, i), v(j, j)),
            ] {
                assert_v_bits_eq!(
                    "err33",
                    l.c.c2Sub(a, b),
                    l.rs.c2Sub(a, b),
                    "inf-inf: {} - {}",
                    showv(a),
                    showv(b)
                );
            }
        }
    }
}

#[test]
fn err34_sub_nan_payload_propagation() {
    let l = libs();
    let pats = [
        0x7FC0_0001u32,
        0xFFC0_1234,
        0x7F80_0001, // sNaN — SSE quiets it
        0xFF80_4321,
        0x7FFF_FFFF,
    ];
    for &pa in &pats {
        for &pb in &pats {
            let (na, nb) = (f32::from_bits(pa), f32::from_bits(pb));
            for (a, b) in [
                (v(na, na), v(nb, nb)),
                (v(na, 1.0), v(1.0, nb)),
                (v(1.0, na), v(nb, 1.0)),
                (v(na, nb), v(1.0, 1.0)),
                (v(1.0, 1.0), v(na, nb)),
            ] {
                assert_v_bits_eq!(
                    "err34",
                    l.c.c2Sub(a, b),
                    l.rs.c2Sub(a, b),
                    "sub NaN 0x{pa:08x} / 0x{pb:08x}"
                );
            }
        }
    }
}

// ===========================================================================
// Generic FFI-boundary coverage required by Phase C beyond the table
// ===========================================================================

#[test]
fn generic_out_of_range_enum_exhaustive_low_range() {
    let l = libs();
    let buf = payload();
    let p = buf.as_ptr() as *const c_void;
    // Every value 0..=4096 in both slots — one step past the valid range and
    // far beyond it.
    for ta in 0u32..=4096 {
        for tb in [0u32, 1, 2, 3, 4095, 4096] {
            let (cv, rv) = both(&l, p, ta, p, tb);
            assert_eq!(cv, rv, "ta={ta} tb={tb}");
            if ta > 1 || tb > 1 {
                assert_eq!(cv, 0, "any out-of-range tag must yield 0 (ta={ta} tb={tb})");
            }
        }
    }
    // And the whole 0..=4096 range in the second slot.
    for tb in 0u32..=4096 {
        for ta in [0u32, 1, 2, 4096] {
            let (cv, rv) = both(&l, p, ta, p, tb);
            assert_eq!(cv, rv, "ta={ta} tb={tb}");
        }
    }
}

#[test]
fn generic_null_and_dangling_on_reject_paths_only() {
    let l = libs();
    // A selection of "impossible" pointer values: the C never dereferences them
    // on the reject paths, so neither may the Rust.
    let ptrs: &[*const c_void] = &[
        ptr::null(),
        1usize as *const c_void,
        usize::MAX as *const c_void,
        0xDEAD_BEEFusize as *const c_void,
        8usize as *const c_void,
    ];
    for &pa in ptrs {
        for &pb in ptrs {
            for &ta in BAD_TAGS.iter().take(6) {
                for &tb in BAD_TAGS.iter().take(6).chain(GOOD_TAGS.iter()) {
                    let (cv, rv) = both(&l, pa, ta, pb, tb);
                    assert_eq!(cv, rv, "bad ptrs, ta={ta} tb={tb}");
                    assert_eq!(cv, 0);
                }
            }
            // Valid typeA with bad typeB: still never dereferences.
            for &ta in GOOD_TAGS {
                for &tb in BAD_TAGS.iter().take(6) {
                    let (cv, rv) = both(&l, pa, ta, pb, tb);
                    assert_eq!(cv, rv, "bad ptrs, ta={ta} tb={tb}");
                    assert_eq!(cv, 0);
                }
            }
        }
    }
}

#[test]
fn generic_zero_and_oversized_payload_regions() {
    let l = libs();
    // A payload buffer sized exactly to each struct (no slack) placed at the end
    // of a page-aligned allocation would fault if the callee over-read. 12 bytes
    // for a circle, 16 for an AABB.
    let mut rng = Rng::new(SEED ^ 99);
    for _ in 0..20_000 {
        let cbuf: Vec<u8> = (0..12).map(|_| rng.next_u32() as u8).collect();
        let abuf: Vec<u8> = (0..16).map(|_| rng.next_u32() as u8).collect();
        let pc = cbuf.as_ptr() as *const c_void;
        let pa = abuf.as_ptr() as *const c_void;
        // Exactly-sized buffers for each declared tag: no over-read allowed.
        for &(x, tx, y, ty) in &[
            (pc, C2_TYPE_CIRCLE, pc, C2_TYPE_CIRCLE),
            (pc, C2_TYPE_CIRCLE, pa, C2_TYPE_AABB),
            (pa, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE),
            (pa, C2_TYPE_AABB, pa, C2_TYPE_AABB),
        ] {
            let (cv, rv) = both(&l, x, tx, y, ty);
            assert_eq!(cv, rv, "exact-size payloads tx={tx} ty={ty}");
        }
    }
    // An oversized buffer: the callee must read only the leading bytes, so the
    // trailing content is irrelevant and both must agree regardless.
    let mut big = vec![0u8; 4096];
    for k in 0..1024 {
        big[k * 4..k * 4 + 4].copy_from_slice(&rng.next_u32().to_ne_bytes());
    }
    for start in [0usize, 16, 64, 1000, 4080 - 16] {
        let p = unsafe { big.as_ptr().add(start) } as *const c_void;
        for &ta in GOOD_TAGS {
            for &tb in GOOD_TAGS {
                let (cv, rv) = both(&l, p, ta, p, tb);
                assert_eq!(cv, rv, "oversized buffer at {start}");
            }
        }
    }
}
