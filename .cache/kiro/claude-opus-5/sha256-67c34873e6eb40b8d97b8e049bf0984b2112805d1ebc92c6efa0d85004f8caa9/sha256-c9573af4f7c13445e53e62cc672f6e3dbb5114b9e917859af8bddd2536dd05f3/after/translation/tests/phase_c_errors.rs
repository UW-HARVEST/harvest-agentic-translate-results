//! Phase C — error / rejection-path differential tests.
//! One test (or one clearly-labelled block) per row of ERRORS.md, E1-E38.
//!
//! Because this library has no error enum and no NULL checks (see the preamble
//! of ERRORS.md), "same error" is asserted as "same returned sentinel value /
//! same bytes written", compared bit-for-bit — never merely "both failed".

mod common;

use common::*;
use std::ffi::c_void;

macro_rules! bind {
    ($l:expr, $name:expr, $ty:ty) => {{
        let c: libloading::Symbol<$ty> = $l.c.get($name);
        let r: libloading::Symbol<$ty> = $l.r.get($name);
        (c, r)
    }};
}

/// Every `int` bit pattern that is NOT a valid `C2_TYPE` variant.
/// C enums accept any `int`, so these are real inputs the C handles.
const BAD_ENUMS: &[u32] = &[
    2,
    3,
    4,
    7,
    100,
    255,
    256,
    0x7FFF_FFFF,
    0x8000_0000, // INT_MIN as unsigned
    0x8000_0001,
    0xFFFF_FFFE,
    0xFFFF_FFFF, // -1 as int
    0xDEAD_BEEF,
];

const VALID_ENUMS: &[u32] = &[C2_TYPE_CIRCLE, C2_TYPE_AABB];

fn some_circle() -> C2Circle {
    C2Circle {
        p: C2v { x: 1.5, y: -2.25 },
        r: 3.0,
    }
}

fn some_aabb() -> C2Aabb {
    C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 4.0, y: 4.0 },
    }
}

// ===========================================================================
// E1, E2, E3 — f2: out-of-range enum values across the FFI boundary
// ===========================================================================

#[test]
fn e1_f2_typea_circle_typeb_out_of_range() {
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    let a = some_circle();
    let b = some_circle();
    let pa = &a as *const C2Circle as *const c_void;
    let pb = &b as *const C2Circle as *const c_void;
    for &bad in BAD_ENUMS {
        unsafe {
            let vc = c(pa, C2_TYPE_CIRCLE, pb, bad);
            let vr = r(pa, C2_TYPE_CIRCLE, pb, bad);
            eq_i32(&format!("E1 f2(CIRCLE, typeB=0x{bad:08x})"), vc, vr);
            assert_eq!(vc, 0, "E1 C must return the 0 sentinel for typeB=0x{bad:08x}");
        }
    }
}

#[test]
fn e2_f2_typea_aabb_typeb_out_of_range() {
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    let a = some_aabb();
    let b = some_aabb();
    let pa = &a as *const C2Aabb as *const c_void;
    let pb = &b as *const C2Aabb as *const c_void;
    for &bad in BAD_ENUMS {
        unsafe {
            let vc = c(pa, C2_TYPE_AABB, pb, bad);
            let vr = r(pa, C2_TYPE_AABB, pb, bad);
            eq_i32(&format!("E2 f2(AABB, typeB=0x{bad:08x})"), vc, vr);
            assert_eq!(vc, 0, "E2 C must return the 0 sentinel for typeB=0x{bad:08x}");
        }
    }
}

#[test]
fn e3_f2_typea_out_of_range_never_dereferences() {
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    // Because the C never dereferences either pointer on the outer `default:`
    // path, NULL is a *legal* input here, and passing it proves the Rust takes
    // the same early exit instead of reading through the pointer.
    for &bad in BAD_ENUMS {
        for &tb in VALID_ENUMS.iter().chain(BAD_ENUMS.iter()) {
            unsafe {
                let vc = c(std::ptr::null(), bad, std::ptr::null(), tb);
                let vr = r(std::ptr::null(), bad, std::ptr::null(), tb);
                eq_i32(&format!("E3 f2(typeA=0x{bad:08x}, typeB=0x{tb:08x}, NULL)"), vc, vr);
                assert_eq!(vc, 0, "E3 C must return the 0 sentinel");
            }
        }
    }
    // Same, with real pointers, so a hypothetical Rust impl that dereferenced
    // before matching would still be caught by the value.
    let a = some_aabb();
    let p = &a as *const C2Aabb as *const c_void;
    for &bad in BAD_ENUMS {
        for &tb in VALID_ENUMS.iter().chain(BAD_ENUMS.iter()) {
            unsafe {
                eq_i32(
                    &format!("E3 f2(typeA=0x{bad:08x}, typeB=0x{tb:08x}, ptr)"),
                    c(p, bad, p, tb),
                    r(p, bad, p, tb),
                );
            }
        }
    }
}

#[test]
fn e1_e3_f2_null_second_pointer_on_typeb_default() {
    // With typeA valid and typeB invalid, the C dereferences neither B nor A
    // (the `default:` arm returns before any cast is evaluated). NULL for both
    // must therefore be safe and return 0 in both implementations.
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    for &ta in VALID_ENUMS {
        for &bad in BAD_ENUMS {
            unsafe {
                let vc = c(std::ptr::null(), ta, std::ptr::null(), bad);
                let vr = r(std::ptr::null(), ta, std::ptr::null(), bad);
                eq_i32(&format!("E1/E3 f2(typeA={ta}, typeB=0x{bad:08x}, NULL)"), vc, vr);
                assert_eq!(vc, 0);
            }
        }
    }
}

// ===========================================================================
// E4-E10 — f3 guards
// ===========================================================================

const IMIN: i32 = i32::MIN;
const IMAX: i32 = i32::MAX;

#[test]
fn e4_f3_divide_by_zero_returns_zero() {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    let mut g = Rng::seeded();
    let mut v1s: Vec<i32> = vec![0, 1, -1, 2, -2, IMAX, IMIN, IMIN + 1, IMAX - 1];
    for _ in 0..2000 {
        v1s.push(g.next_i32());
    }
    for v1 in v1s {
        unsafe {
            let vc = c(v1, 0);
            let vr = r(v1, 0);
            eq_i32(&format!("E4 f3({v1}, 0)"), vc, vr);
            assert_eq!(vc, 0, "E4 C must return the 0 sentinel for v2 == 0");
        }
    }
}

#[test]
fn e5_f3_v1_nonneg_v2_intmin() {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    let mut g = Rng::seeded();
    let mut v1s: Vec<i32> = vec![0, 1, 2, 3, IMAX, IMAX - 1, 1 << 30];
    for _ in 0..2000 {
        v1s.push((g.next_u32() >> 1) as i32);
    }
    for v1 in v1s {
        unsafe { eq_i32(&format!("E5 f3({v1}, IMIN)"), c(v1, IMIN), r(v1, IMIN)) }
    }
}

#[test]
fn e6_f3_v1_intmin_guard() {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    let mut g = Rng::seeded();
    let mut v2s: Vec<i32> = vec![1, -1, 2, -2, 3, -3, IMAX, IMIN, IMIN + 1, IMAX - 1];
    for _ in 0..4000 {
        let v = g.next_i32();
        if v != 0 {
            v2s.push(v);
        }
    }
    for v2 in v2s {
        unsafe { eq_i32(&format!("E6 f3(IMIN, {v2})"), c(IMIN, v2), r(IMIN, v2)) }
    }
}

#[test]
fn e7_f3_v1_neg_not_intmin_v2_intmin() {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    let mut g = Rng::seeded();
    let mut v1s: Vec<i32> = vec![-1, -2, -3, IMIN + 1, IMIN + 2, -(1 << 30)];
    for _ in 0..2000 {
        let v = -(((g.next_u32() >> 1) as i32).max(1));
        if v != IMIN {
            v1s.push(v);
        }
    }
    for v1 in v1s {
        unsafe { eq_i32(&format!("E7 f3({v1}, IMIN)"), c(v1, IMIN), r(v1, IMIN)) }
    }
}

#[test]
fn e8_f3_both_intmin() {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    unsafe {
        let vc = c(IMIN, IMIN);
        let vr = r(IMIN, IMIN);
        eq_i32("E8 f3(IMIN, IMIN)", vc, vr);
        assert_eq!(vc, 1, "E8 C takes the `q = 1, r = 0` branch");
    }
}

#[test]
fn e9_f3_v1_intmin_v2_positive_overflow_path() {
    // `-(v1 + v2)` overflows for v1 == INT_MIN and v2 >= 1: UB in C, wrapping
    // in the emitted -O0 code. The Rust must wrap identically.
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    for v2 in 1i32..=2000 {
        unsafe { eq_i32(&format!("E9 f3(IMIN, {v2})"), c(IMIN, v2), r(IMIN, v2)) }
    }
    let mut g = Rng::seeded();
    for _ in 0..4000 {
        let v2 = ((g.next_u32() >> 1) as i32).max(1);
        unsafe { eq_i32(&format!("E9 f3(IMIN, {v2})"), c(IMIN, v2), r(IMIN, v2)) }
    }
    for &v2 in &[IMAX, IMAX - 1, 1 << 30, (1 << 30) + 1] {
        unsafe { eq_i32(&format!("E9 f3(IMIN, {v2})"), c(IMIN, v2), r(IMIN, v2)) }
    }
    // v2 == 0 with v1 == INT_MIN is caught earlier by E4
    unsafe { eq_i32("E9 f3(IMIN, 0)", c(IMIN, 0), r(IMIN, 0)) }
}

#[test]
fn e10_f3_v1_intmin_v2_negative_not_intmin() {
    let l = libs();
    let (c, r) = bind!(l, "f3", FnF3);
    for v2 in -2000i32..0 {
        unsafe { eq_i32(&format!("E10 f3(IMIN, {v2})"), c(IMIN, v2), r(IMIN, v2)) }
    }
    let mut g = Rng::seeded();
    for _ in 0..4000 {
        let v2 = -(((g.next_u32() >> 1) as i32).max(1));
        if v2 != IMIN {
            unsafe { eq_i32(&format!("E10 f3(IMIN, {v2})"), c(IMIN, v2), r(IMIN, v2)) }
        }
    }
    for &v2 in &[IMIN + 1, IMIN + 2, -(1 << 30)] {
        unsafe { eq_i32(&format!("E10 f3(IMIN, {v2})"), c(IMIN, v2), r(IMIN, v2)) }
    }
}

// ===========================================================================
// E11, E12 — f4
// ===========================================================================

#[test]
fn e11_f4_zero_state_is_a_fixed_point() {
    let l = libs();
    let (c, r) = bind!(l, "f4", FnF4);
    let mut sc = CnRnd { state: [0, 0] };
    let mut sr = CnRnd { state: [0, 0] };
    for step in 0..256 {
        unsafe {
            let vc = c(&mut sc);
            let vr = r(&mut sr);
            eq_f64(&format!("E11 f4 zero-state step{step}"), vc, vr);
            assert_eq!(vc.to_bits(), 0.0f64.to_bits(), "E11 must be exactly +0.0");
        }
        eq_rnd(&format!("E11 f4 zero-state state step{step}"), sc, sr);
        assert_eq!(sc.state, [0, 0], "E11 zero state must be a fixed point");
    }
    // NOTE: `rnd == NULL` is unchecked in the C (it dereferences immediately),
    // so it faults in both implementations and cannot be compared
    // differentially. Documented as E11 in ERRORS.md, deliberately not called.
}

#[test]
fn e12_f4_result_never_nan_or_inf() {
    let l = libs();
    let (c, r) = bind!(l, "f4", FnF4);
    let mut g = Rng::seeded();
    for i in 0..20000 {
        let st = [g.next_u64(), g.next_u64()];
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        unsafe {
            let vc = c(&mut sc);
            let vr = r(&mut sr);
            eq_f64(&format!("E12 f4 #{i}"), vc, vr);
            assert!(
                !vc.is_nan() && vc.is_finite() && (0.0..1.0).contains(&vc),
                "E12 f4 must stay in [0,1): got {vc} for state {st:016x?}"
            );
        }
        eq_rnd(&format!("E12 f4 state #{i}"), sc, sr);
    }
}

// ===========================================================================
// E13 — f5 silently discards bits above 15
// ===========================================================================

#[test]
fn e13_f5_high_bits_discarded() {
    let l = libs();
    let (c, r) = bind!(l, "f5", FnF5);
    let mut g = Rng::seeded();
    for i in 0..20000 {
        let low = g.next_u32() & 0xFFFF;
        let high = g.next_u32() & 0xFFFF_0000;
        let a = low | high;
        unsafe {
            let vc = c(a);
            let vr = r(a);
            eq_u32(&format!("E13 f5(0x{a:08x}) #{i}"), vc, vr);
            // the C's contract: high bits are dropped, result fits in 16 bits
            assert!(vc <= 0xFFFF, "E13 f5 result must fit in 16 bits");
            eq_u32(&format!("E13 f5 ignores high bits #{i}"), vc, c(low));
        }
    }
    for &a in &[0xFFFF_0000u32, 0xFFFF_FFFF, 0x8000_0000, 0x1234_0000] {
        unsafe { eq_u32(&format!("E13 f5(0x{a:08x})"), c(a), r(a)) }
    }
}

// ===========================================================================
// E14, E15 — f7
// ===========================================================================

#[test]
fn e14_f7_unsigned_overflow_wraps_identically() {
    let l = libs();
    let (c, r) = bind!(l, "f7", FnF7);
    let big = [
        0xFFFF_FFFFu32,
        0xFFFF_FFFE,
        0x8000_0000,
        0x7FFF_FFFF,
        0x1_0000,
        0xFFFF,
        65536 * 512,
    ];
    for &bs in &big {
        for &ch in &big {
            for &bd in &big {
                unsafe {
                    eq_u32(&format!("E14 f7({bs},{ch},{bd})"), c(bs, ch, bd), r(bs, ch, bd))
                }
            }
        }
    }
    // channels == 2 and bitdepth == 32 combined with wrapping magnitudes
    for &bs in &big {
        unsafe {
            eq_u32(&format!("E14 f7({bs},2,32)"), c(bs, 2, 32), r(bs, 2, 32));
            eq_u32(&format!("E14 f7({bs},2,31)"), c(bs, 2, 31), r(bs, 2, 31));
            eq_u32(
                &format!("E14 f7({bs},2,0xFFFFFFFF)"),
                c(bs, 2, 0xFFFF_FFFF),
                r(bs, 2, 0xFFFF_FFFF),
            );
        }
    }
    let mut g = Rng::seeded();
    for _ in 0..20000 {
        let (bs, ch, bd) = (g.next_u32(), g.next_u32(), g.next_u32());
        unsafe { eq_u32(&format!("E14 f7({bs},{ch},{bd})"), c(bs, ch, bd), r(bs, ch, bd)) }
    }
}

#[test]
fn e15_f7_zero_channels() {
    let l = libs();
    let (c, r) = bind!(l, "f7", FnF7);
    let mut g = Rng::seeded();
    // channels == 0 zeroes every summand -> 18 + 0 + (0+7)/8 == 18
    for &bd in &[0u32, 1, 8, 16, 31, 32, 33, 0xFFFF_FFFF] {
        for &bs in &[0u32, 1, 4096, 0xFFFF_FFFF] {
            unsafe {
                let vc = c(bs, 0, bd);
                let vr = r(bs, 0, bd);
                eq_u32(&format!("E15 f7({bs},0,{bd})"), vc, vr);
                assert_eq!(vc, 18, "E15 channels==0 must give 18");
            }
        }
    }
    for _ in 0..5000 {
        let (bs, bd) = (g.next_u32(), g.next_u32());
        unsafe { eq_u32(&format!("E15 f7({bs},0,{bd})"), c(bs, 0, bd), r(bs, 0, bd)) }
    }
}

// ===========================================================================
// E16, E17 — f9 degenerate / NaN
// ===========================================================================

#[test]
fn e16_f9_zero_denominator() {
    let l = libs();
    let (c, r) = bind!(l, "f9", FnF9);
    let v = |x, y| LmVec2 { x, y };
    let mut g = Rng::seeded();
    for i in 0..5000 {
        let a = v(g.finite_f32(10.0), g.finite_f32(10.0));
        let p = v(g.finite_f32(10.0), g.finite_f32(10.0));
        unsafe {
            let rc = c(a, a, a, p);
            let rr = r(a, a, a, p);
            eq_lmvec2(&format!("E16 f9 coincident #{i}"), rc, rr);
            // C contract: 1.0f/0.0f -> inf, then 0*inf -> NaN
            assert!(
                rc.x.is_nan() || rc.x.is_infinite(),
                "E16 expected non-finite u, got {}",
                rc.x
            );
        }
        // collinear
        let b = v(g.finite_f32(10.0), g.finite_f32(10.0));
        let t = g.range_f32(-3.0, 3.0);
        let col = v(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
        unsafe {
            eq_lmvec2(&format!("E16 f9 collinear #{i}"), c(a, b, col, p), r(a, b, col, p))
        }
    }
    // exact zero triangle
    let z = v(0.0, 0.0);
    for &p in &[z, v(1.0, 1.0), v(-0.0, -0.0), v(f32::INFINITY, 0.0)] {
        unsafe { eq_lmvec2("E16 f9 all-zero", c(z, z, z, p), r(z, z, z, p)) }
    }
}

#[test]
fn e17_f9_nan_propagation() {
    let l = libs();
    let (c, r) = bind!(l, "f9", FnF9);
    let v = |x, y| LmVec2 { x, y };
    let nans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFF80_0002),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0xFFD5_5555),
    ];
    for &n in &nans {
        for &m in &nans {
            let combos = [
                [v(n, 0.0), v(3.0, 0.0), v(0.0, 4.0), v(1.0, 1.0)],
                [v(0.0, n), v(3.0, m), v(0.0, 4.0), v(1.0, 1.0)],
                [v(0.0, 0.0), v(n, m), v(0.0, 4.0), v(1.0, 1.0)],
                [v(0.0, 0.0), v(3.0, 0.0), v(n, m), v(1.0, 1.0)],
                [v(0.0, 0.0), v(3.0, 0.0), v(0.0, 4.0), v(n, m)],
                [v(n, m), v(m, n), v(n, n), v(m, m)],
            ];
            for (k, pts) in combos.iter().enumerate() {
                unsafe {
                    eq_lmvec2(
                        &format!(
                            "E17 f9 combo{k} n=0x{:08x} m=0x{:08x}",
                            n.to_bits(),
                            m.to_bits()
                        ),
                        c(pts[0], pts[1], pts[2], pts[3]),
                        r(pts[0], pts[1], pts[2], pts[3]),
                    )
                }
            }
        }
    }
}

// ===========================================================================
// E18, E19 — f10 index bound and half-float specials
// ===========================================================================

#[test]
fn e18_f10_no_reachable_out_of_range_index() {
    // `n = h >> 10` is provably <= 63 for a uint16_t, and
    // (h & 0x3ff) + m__offset[n] is provably <= 2047. If the Rust indexed
    // differently (or panicked), the exhaustive comparison below would catch
    // it — a Rust panic across `extern "C"` aborts the process.
    let l = libs();
    let (c, r) = bind!(l, "f10", FnF10);
    for h in 0u16..=u16::MAX {
        unsafe { eq_f32(&format!("E18 f10(0x{h:04x})"), c(h), r(h)) }
        if h == u16::MAX {
            break;
        }
    }
    // and the top of every exponent class, which is where an off-by-one in
    // the offset table would first read past m__mantissa[2048]
    for n in 0u16..64 {
        let h = (n << 10) | 0x3FF;
        unsafe { eq_f32(&format!("E18 f10 top-of-class n={n}"), c(h), r(h)) }
    }
}

#[test]
fn e19_f10_half_inf_and_nan_encodings() {
    let l = libs();
    let (c, r) = bind!(l, "f10", FnF10);
    // n == 31 -> positive inf/NaN half encodings; n == 63 -> negative
    for n in [31u16, 63] {
        for m in 0u16..=0x3FF {
            let h = (n << 10) | m;
            unsafe {
                let vc = c(h);
                let vr = r(h);
                eq_f32(&format!("E19 f10(0x{h:04x})"), vc, vr);
                assert!(
                    !vc.is_finite(),
                    "E19 f10(0x{h:04x}) should be inf or NaN, got {vc}"
                );
            }
        }
    }
}

// ===========================================================================
// E20-E23 — f11 early return, unreachable branch, NaN, -0.0
// ===========================================================================

fn call3(name: &str, src: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let l = libs();
    let (c, r) = bind!(l, name, FnTriple);
    let mut dc = [-1234.5678f32; 3];
    let mut dr = [-1234.5678f32; 3];
    unsafe {
        c(dc.as_mut_ptr(), src.as_ptr());
        r(dr.as_mut_ptr(), src.as_ptr());
    }
    (dc, dr)
}

#[track_caller]
fn same3(name: &str, tag: &str, src: [f32; 3]) -> [f32; 3] {
    let (dc, dr) = call3(name, src);
    eq_triple(&format!("{tag} {name}({src:?})"), dc, dr);
    dc
}

#[test]
fn e20_f11_saturation_zero_writes_l_three_times() {
    let mut g = Rng::seeded();
    for &s in &[0.0f32, -0.0f32] {
        for i in 0..3000 {
            let h = g.mixed_f32();
            let ll = g.mixed_f32();
            let out = same3("f11", &format!("E20 s={s} #{i}"), [h, s, ll]);
            assert_eq!(out[0].to_bits(), ll.to_bits(), "E20 dest[0] must be l");
            assert_eq!(out[1].to_bits(), ll.to_bits(), "E20 dest[1] must be l");
            assert_eq!(out[2].to_bits(), ll.to_bits(), "E20 dest[2] must be l");
        }
    }
}

#[test]
fn e21_f11_final_else_branch() {
    // Reached ONLY by: h in [120,180) (because the branch above reads
    // `h < 120.0f && h < 180.0f`), h >= 360, and h NaN.
    // Negative h is NOT here — see e21b below.
    let mut g = Rng::seeded();
    let mut triggers: Vec<f32> = vec![
        120.0,
        150.0,
        179.0,
        f32::from_bits(180.0f32.to_bits() - 1),
        360.0,
        361.0,
        1e30,
        f32::INFINITY,
        f32::NAN,
        -f32::NAN,
    ];
    for _ in 0..500 {
        triggers.push(g.range_f32(120.0, 180.0));
        triggers.push(g.range_f32(360.0, 1e6));
    }
    for (i, &h) in triggers.iter().enumerate() {
        for &s in &[0.25f32, 1.0, -1.0, 2.0] {
            for &ll in &[0.0f32, 0.5, 1.0, -0.5] {
                let out = same3("f11", &format!("E21 h={h} #{i}"), [h, s, ll]);
                // all three outputs must equal m == l - 0.5*c
                assert_eq!(
                    out[0].to_bits(),
                    out[1].to_bits(),
                    "E21 final else must write m to all three (h={h}, s={s}, l={ll})"
                );
                assert_eq!(out[1].to_bits(), out[2].to_bits(), "E21 dest[1] != dest[2]");
            }
        }
    }
}

#[test]
fn e21b_f11_negative_hue_hits_the_typo_branch() {
    // Because lib.c:894 reads `h < 120.0f && h < 180.0f`, every h < 0 falls
    // into that branch and gets {m, c+m, x+m} — verified against the compiled
    // C. This is the row that catches a translation that "fixed" the typo to
    // `h >= 120.0f`, which would send negative h to the final else instead.
    let mut g = Rng::seeded();
    let mut triggers: Vec<f32> = vec![-1.0, -0.5, -60.0, -180.0, -1e30, f32::NEG_INFINITY];
    for _ in 0..1000 {
        triggers.push(-g.range_f32(0.0001, 1e6));
    }
    for (i, &h) in triggers.iter().enumerate() {
        for &s in &[0.25f32, 1.0, 2.0] {
            for &ll in &[0.0f32, 0.5, 1.0] {
                let out = same3("f11", &format!("E21b h={h} #{i}"), [h, s, ll]);
                if s != 0.0 && ll == 0.5 && s == 0.25 && h == -1.0 {
                    // sanity anchor against the C probe: dest[0] == m and the
                    // three are NOT all equal, i.e. it is not the final else
                    assert_ne!(
                        out[0].to_bits(),
                        out[1].to_bits(),
                        "E21b h<0 must take the typo branch, not the final else"
                    );
                }
            }
        }
    }
}

#[test]
fn e22_f11_nan_hue_takes_final_else() {
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFFD5_5555),
    ];
    for &h in &nans {
        for &s in &[0.5f32, 1.0, -1.0, 1e-30, f32::INFINITY] {
            for &ll in &[0.0f32, 0.5, 1.0, -1.0, f32::INFINITY] {
                let out = same3("f11", "E22", [h, s, ll]);
                assert_eq!(out[0].to_bits(), out[1].to_bits(), "E22 not all-m");
                assert_eq!(out[1].to_bits(), out[2].to_bits(), "E22 not all-m");
            }
        }
    }
}

#[test]
fn e23_f11_negative_zero_hue_takes_first_branch() {
    // -0.0f >= 0.0f is TRUE, so h == -0.0 must take the FIRST branch, not the
    // final else. Detected by comparing against h == +0.0 and against a
    // known-else hue.
    for &s in &[0.5f32, 1.0, 0.25] {
        for &ll in &[0.25f32, 0.5, 0.75] {
            let neg = same3("f11", "E23 h=-0.0", [-0.0, s, ll]);
            let pos = same3("f11", "E23 h=+0.0", [0.0, s, ll]);
            eq_triple("E23 -0.0 must behave as +0.0", pos, neg);
            let els = same3("f11", "E23 h=150", [150.0, s, ll]);
            assert_ne!(
                neg[0].to_bits(),
                els[0].to_bits(),
                "E23 h=-0.0 must NOT take the final else (s={s}, l={ll})"
            );
        }
    }
}

#[test]
fn e24_f11_null_pointers_documented_not_called() {
    // The C dereferences `dest`/`src` unconditionally (no NULL check), so a
    // NULL argument faults in BOTH implementations and cannot be compared
    // differentially. Assert instead that a 3-float buffer is the exact
    // extent touched: a 5-float buffer's guard elements must stay untouched.
    let l = libs();
    let (c, r) = bind!(l, "f11", FnTriple);
    let mut g = Rng::seeded();
    for i in 0..2000 {
        let src = [g.mixed_f32(), g.mixed_f32(), g.mixed_f32(), 111.0, 222.0];
        let mut dc = [0.0f32, 0.0, 0.0, 333.0, 444.0];
        let mut dr = [0.0f32, 0.0, 0.0, 333.0, 444.0];
        unsafe {
            c(dc.as_mut_ptr(), src.as_ptr());
            r(dr.as_mut_ptr(), src.as_ptr());
        }
        eq_bits(&format!("E24 f11 buffer #{i}"), dc, dr);
        assert_eq!(
            (dc[3].to_bits(), dc[4].to_bits()),
            (333.0f32.to_bits(), 444.0f32.to_bits()),
            "E24 f11 wrote past dest[2]"
        );
    }
}

// ===========================================================================
// E25-E27 — f12
// ===========================================================================

#[test]
fn e25_f12_saturation_zero_writes_v_three_times() {
    let mut g = Rng::seeded();
    for &s in &[0.0f32, -0.0f32] {
        for i in 0..3000 {
            let h = g.mixed_f32();
            let v = g.mixed_f32();
            let out = same3("f12", &format!("E25 s={s} #{i}"), [h, s, v]);
            for k in 0..3 {
                assert_eq!(out[k].to_bits(), v.to_bits(), "E25 dest[{k}] must be v");
            }
        }
    }
}

#[test]
fn e26_f12_default_switch_arm() {
    // i not in 0..=4 -> r=v, g=p, b=q
    let mut g = Rng::seeded();
    let mut hs: Vec<f32> = vec![
        300.0, 301.0, 359.9, 360.0, 420.0, 1e6, -1.0, -60.0, -1e6,
    ];
    for _ in 0..500 {
        hs.push(g.range_f32(300.0, 1e5));
        hs.push(-g.range_f32(0.0001, 1e5));
    }
    for (i, &h) in hs.iter().enumerate() {
        for &s in &[0.5f32, 1.0, 0.25] {
            for &v in &[0.25f32, 0.5, 1.0] {
                let out = same3("f12", &format!("E26 h={h} #{i}"), [h, s, v]);
                // default arm: r == v exactly
                assert_eq!(
                    out[0].to_bits(),
                    v.to_bits(),
                    "E26 default arm must set r = v (h={h}, s={s}, v={v})"
                );
            }
        }
    }
}

#[test]
fn e27_f12_float_to_int_out_of_range_selects_default() {
    // (int)floorf(h/60) is UB in C; x86 cvttss2si returns INT_MIN, which is
    // `> 4` in the unsigned comparison the C compiles to, so `default:` runs.
    let hs = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        1e30,
        -1e30,
        2147483648.0 * 60.0,
        -2147483648.0 * 60.0,
        1.2884902e11,
        -1.2884902e11,
    ];
    for &h in &hs {
        for &s in &[0.5f32, 1.0, -1.0, 1e-30] {
            for &v in &[0.25f32, 0.5, 1.0, f32::INFINITY] {
                let out = same3("f12", "E27", [h, s, v]);
                if !h.is_nan() {
                    assert_eq!(
                        out[0].to_bits(),
                        v.to_bits(),
                        "E27 must take default arm (h={h}, s={s}, v={v})"
                    );
                }
            }
        }
    }
    // dense sweep straddling the 2^31 cast boundary
    let mut g = Rng::seeded();
    for i in 0..4000 {
        let scale = 2147483648.0f32 * 60.0;
        let h = scale * g.range_f32(0.99, 1.01) * if i % 2 == 0 { 1.0 } else { -1.0 };
        same3("f12", &format!("E27 boundary #{i}"), [h, 0.5, 0.75]);
    }
}

// ===========================================================================
// E28-E32 — f13
// ===========================================================================

#[test]
fn e28_f13_delta_zero_early_return() {
    let mut g = Rng::seeded();
    for i in 0..4000 {
        let v = g.finite_f32(1e6);
        let out = same3("f13", &format!("E28 #{i}"), [v, v, v]);
        assert_eq!(out[0].to_bits(), 0.0f32.to_bits(), "E28 h must be +0.0");
        assert_eq!(out[1].to_bits(), 0.0f32.to_bits(), "E28 s must be +0.0");
        assert_eq!(out[2].to_bits(), v.to_bits(), "E28 v must be max");
    }
    // all-NaN: every compare is false so min == max == r, delta = NaN - NaN
    for &n in &[
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FAB_CDEF),
    ] {
        same3("f13", "E28 all-nan", [n, n, n]);
    }
    for &v in &special_f32s() {
        same3("f13", "E28 special", [v, v, v]);
    }
}

#[test]
fn e29_f13_max_zero_early_return() {
    // max == 0 with delta != 0 (all-negative inputs)
    let cases: &[[f32; 3]] = &[
        [-1.0, -2.0, 0.0],
        [-1.0, -2.0, -0.0],
        [0.0, -1.0, -2.0],
        [-0.0, -1.0, -2.0],
        [-1.0, 0.0, -2.0],
        [-1e30, -1.0, 0.0],
        [-f32::MAX, -1.0, -0.0],
    ];
    for t in cases {
        let out = same3("f13", "E29", *t);
        assert_eq!(out[0].to_bits(), 0.0f32.to_bits(), "E29 h must be +0.0 for {t:?}");
        assert_eq!(out[1].to_bits(), 0.0f32.to_bits(), "E29 s must be +0.0 for {t:?}");
    }
    let mut g = Rng::seeded();
    for i in 0..4000 {
        let a = -g.range_f32(0.0001, 1e4);
        let b = -g.range_f32(0.0001, 1e4);
        for t in [[a, b, 0.0f32], [a, 0.0, b], [0.0, a, b], [a, b, -0.0]] {
            same3("f13", &format!("E29 rnd #{i}"), t);
        }
    }
}

#[test]
fn e30_f13_final_else_branch() {
    // Neither r == max nor g == max -> h = 4 + (r-g)/delta
    let mut g = Rng::seeded();
    for i in 0..4000 {
        let bv = g.range_f32(0.5, 1.0);
        let rv = g.range_f32(0.0, bv * 0.9);
        let gv = g.range_f32(0.0, bv * 0.9);
        // b strictly greatest -> final else
        let out = same3("f13", &format!("E30 #{i}"), [rv, gv, bv]);
        assert_eq!(out[2].to_bits(), bv.to_bits(), "E30 v must be b");
    }
}

#[test]
fn e31_f13_negative_hue_correction() {
    let mut g = Rng::seeded();
    for i in 0..4000 {
        let r = g.range_f32(0.5, 1.0);
        let bv = g.range_f32(0.001, r);
        let gv = g.range_f32(0.0, bv * 0.99);
        same3("f13", &format!("E31 #{i}"), [r, gv, bv]);
    }
    // magnitudes where h += 360 is not enough to bring h back to >= 0
    for t in [
        [1.0f32, -1e30, 0.0],
        [1e-30, -1e30, 0.0],
        [f32::MAX, -f32::MAX, 0.0],
        [1.0, -f32::MAX, 0.5],
        [f32::MIN_POSITIVE, -1e38, 0.0],
    ] {
        same3("f13", "E31 extreme", t);
    }
}

#[test]
fn e32_f13_nan_delta() {
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFFD5_5555),
    ];
    let vals = [0.0f32, -0.0, 0.5, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, 1e30];
    for &n in &nans {
        for &a in &vals {
            for &b in &vals {
                same3("f13", "E32 n0", [n, a, b]);
                same3("f13", "E32 n1", [a, n, b]);
                same3("f13", "E32 n2", [a, b, n]);
            }
        }
    }
}

// ===========================================================================
// E33-E35 — c2* NaN semantics
// ===========================================================================

#[test]
fn e33_c2_minmax_and_dot_nan_selection() {
    let l = libs();
    let (maxc, maxr) = bind!(l, "c2Maxv", FnC2Bin);
    let (minc, minr) = bind!(l, "c2Minv", FnC2Bin);
    let (dotc, dotr) = bind!(l, "c2Dot", FnC2Dot);
    let (subc, subr) = bind!(l, "c2Sub", FnC2Bin);
    let nans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFF80_0002),
        f32::from_bits(0xFFD5_5555),
    ];
    for &n in &nans {
        for &m in &nans {
            let a = C2v { x: n, y: m };
            let b = C2v { x: 1.0, y: -2.0 };
            unsafe {
                // NaN makes `>`/`<` false, so both min and max return `b`
                let mc = maxc(a, b);
                eq_vec2("E33 c2Maxv(nan, b)", mc, maxr(a, b));
                assert_eq!(mc.x.to_bits(), b.x.to_bits(), "E33 c2Maxv must pick b.x");
                let nc = minc(a, b);
                eq_vec2("E33 c2Minv(nan, b)", nc, minr(a, b));
                assert_eq!(nc.x.to_bits(), b.x.to_bits(), "E33 c2Minv must pick b.x");
                // but b as first operand keeps b when b is not NaN
                eq_vec2("E33 c2Maxv(b, nan)", maxc(b, a), maxr(b, a));
                eq_vec2("E33 c2Minv(b, nan)", minc(b, a), minr(b, a));
                eq_vec2("E33 c2Sub(nan, b)", subc(a, b), subr(a, b));
                eq_vec2("E33 c2Sub(b, nan)", subc(b, a), subr(b, a));
                eq_f32("E33 c2Dot(nan, b)", dotc(a, b), dotr(a, b));
                eq_f32("E33 c2Dot(b, nan)", dotc(b, a), dotr(b, a));
                let c2 = C2v { x: m, y: n };
                eq_f32("E33 c2Dot(nan, nan)", dotc(a, c2), dotr(a, c2));
            }
        }
    }
}

#[test]
fn e34_circle_tests_report_no_collision_on_nan() {
    let l = libs();
    let (cc, cr) = bind!(l, "c2CircletoCircle", FnCircleCircle);
    let (ac, ar) = bind!(l, "c2CircletoAABB", FnCircleAabb);
    let n = f32::NAN;
    let circ_nan = C2Circle { p: C2v { x: n, y: n }, r: n };
    let circ_ok = some_circle();
    let box_ok = some_aabb();
    let box_nan = C2Aabb {
        min: C2v { x: n, y: n },
        max: C2v { x: n, y: n },
    };
    unsafe {
        for (tag, vc, vr) in [
            ("nan-nan", cc(circ_nan, circ_nan), cr(circ_nan, circ_nan)),
            ("nan-ok", cc(circ_nan, circ_ok), cr(circ_nan, circ_ok)),
            ("ok-nan", cc(circ_ok, circ_nan), cr(circ_ok, circ_nan)),
        ] {
            eq_i32(&format!("E34 c2CircletoCircle {tag}"), vc, vr);
            assert_eq!(vc, 0, "E34 NaN must report 0 (no collision) for {tag}");
        }
        for (tag, vc, vr) in [
            ("nan-box", ac(circ_nan, box_ok), ar(circ_nan, box_ok)),
            ("circ-nanbox", ac(circ_ok, box_nan), ar(circ_ok, box_nan)),
            ("nan-nanbox", ac(circ_nan, box_nan), ar(circ_nan, box_nan)),
        ] {
            eq_i32(&format!("E34 c2CircletoAABB {tag}"), vc, vr);
            assert_eq!(vc, 0, "E34 NaN must report 0 for {tag}");
        }
    }
}

#[test]
fn e35_aabb_test_reports_collision_on_all_nan() {
    let l = libs();
    let (c, r) = bind!(l, "c2AABBtoAABB", FnAabbAabb);
    let n = f32::NAN;
    let nb = C2Aabb {
        min: C2v { x: n, y: n },
        max: C2v { x: n, y: n },
    };
    let ok = some_aabb();
    unsafe {
        let vc = c(nb, nb);
        eq_i32("E35 c2AABBtoAABB(nan, nan)", vc, r(nb, nb));
        assert_eq!(vc, 1, "E35 all-NaN boxes must report 1 (collide)");
        eq_i32("E35 c2AABBtoAABB(nan, ok)", c(nb, ok), r(nb, ok));
        eq_i32("E35 c2AABBtoAABB(ok, nan)", c(ok, nb), r(ok, nb));
    }
    // one NaN coordinate at a time
    for slot in 0..4 {
        let mut b = ok;
        match slot {
            0 => b.min.x = n,
            1 => b.min.y = n,
            2 => b.max.x = n,
            _ => b.max.y = n,
        }
        unsafe {
            eq_i32(&format!("E35 slot{slot} a"), c(b, ok), r(b, ok));
            eq_i32(&format!("E35 slot{slot} b"), c(ok, b), r(ok, b));
        }
    }
}

// ===========================================================================
// E36-E38 — agglom
// ===========================================================================

#[test]
fn e36_e37_e38_agglom_error_paths() {
    let l = libs();
    let (c, r) = bind!(l, "agglom", FnAgglom);
    let mut g = Rng::seeded();

    #[allow(clippy::too_many_arguments)]
    fn go(
        f: &FnAgglom,
        f2: [f32; 7],
        f3: [i32; 2],
        f4: [u64; 2],
        f5: u32,
        f7: [u32; 3],
        f9: [f32; 8],
        f10: u16,
        f11: [f32; 3],
        f12: [f32; 3],
        f13: [f32; 3],
    ) -> f64 {
        unsafe {
            f(
                f2[0], f2[1], f2[2], f2[3], f2[4], f2[5], f2[6], f3[0], f3[1], f4[0], f4[1], f5,
                f7[0], f7[1], f7[2], f9[0], f9[1], f9[2], f9[3], f9[4], f9[5], f9[6], f9[7], f10,
                f11[0], f11[1], f11[2], f12[0], f12[1], f12[2], f13[0], f13[1], f13[2],
            )
        }
    }

    let base_f2 = [0.0f32, 0.0, 1.0, -1.0, -1.0, 1.0, 1.0];
    let base_f9 = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.25, 0.25];
    let base_f11 = [30.0f32, 0.5, 0.5];
    let base_f13 = [0.25f32, 0.5, 0.75];

    // E37: f3_2 == 0 contributes 0 and surfaces no error
    for f3_1 in [0i32, 1, -1, IMIN, IMAX] {
        let vc = go(&c, base_f2, [f3_1, 0], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, base_f11, base_f11, base_f13);
        let vr = go(&r, base_f2, [f3_1, 0], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, base_f11, base_f11, base_f13);
        eq_f64(&format!("E37 agglom f3=({f3_1},0)"), vc, vr);
    }

    // E36: NaN terms are skipped, inf terms are NOT
    let nan = f32::NAN;
    for slot in 0..3 {
        let mut t = base_f11;
        t[slot] = nan;
        eq_f64(
            &format!("E36 agglom f11[{slot}]=NaN"),
            go(&c, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, t, base_f11, base_f13),
            go(&r, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, t, base_f11, base_f13),
        );
        let mut u = base_f13;
        u[slot] = nan;
        eq_f64(
            &format!("E36 agglom f13[{slot}]=NaN"),
            go(&c, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, base_f11, base_f11, u),
            go(&r, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, base_f11, base_f11, u),
        );
        for &inf in &[f32::INFINITY, f32::NEG_INFINITY] {
            let mut w = base_f11;
            w[slot] = inf;
            let vc = go(&c, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, w, base_f11, base_f13);
            let vr = go(&r, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, w, base_f11, base_f13);
            eq_f64(&format!("E36 agglom f11[{slot}]={inf}"), vc, vr);
        }
    }
    // degenerate f9 -> inf propagates (not filtered)
    let degen = [1.0f32, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0, 3.0];
    eq_f64(
        "E36 agglom degenerate f9",
        go(&c, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], degen, 0x3C00, base_f11, base_f11, base_f13),
        go(&r, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], degen, 0x3C00, base_f11, base_f11, base_f13),
    );
    // f10 inf/NaN half encodings
    for h in [0x7C00u16, 0x7C01, 0x7E00, 0x7FFF, 0xFC00, 0xFC01, 0xFFFF] {
        eq_f64(
            &format!("E36 agglom f10=0x{h:04x}"),
            go(&c, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, h, base_f11, base_f11, base_f13),
            go(&r, base_f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, h, base_f11, base_f11, base_f13),
        );
    }

    // E38: agglom always uses typeA=CIRCLE, typeB=AABB, so f2's `default:`
    // arms are unreachable here. Confirm the value still matches for the
    // shapes agglom does build, including all-NaN and inverted boxes.
    for _ in 0..3000 {
        let f2 = [
            g.mixed_f32(), g.mixed_f32(), g.mixed_f32(), g.mixed_f32(), g.mixed_f32(),
            g.mixed_f32(), g.mixed_f32(),
        ];
        eq_f64(
            "E38 agglom f2 shapes",
            go(&c, f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, base_f11, base_f11, base_f13),
            go(&r, f2, [7, 3], [1, 2], 5, [4096, 2, 16], base_f9, 0x3C00, base_f11, base_f11, base_f13),
        );
    }
}

// ===========================================================================
// Generic C-API boundaries required by Phase C beyond the ERRORS.md table
// ===========================================================================

#[test]
fn generic_out_of_range_enums_exhaustive_pairs() {
    // Every combination of {valid, invalid} x {valid, invalid} enum values,
    // which is the class of bug happy-path tests miss entirely.
    let l = libs();
    let (c, r) = bind!(l, "f2", FnF2);
    let a = some_aabb();
    let b = some_aabb();
    let pa = &a as *const C2Aabb as *const c_void;
    let pb = &b as *const C2Aabb as *const c_void;
    let all: Vec<u32> = VALID_ENUMS.iter().chain(BAD_ENUMS.iter()).copied().collect();
    for &ta in &all {
        for &tb in &all {
            unsafe {
                let vc = c(pa, ta, pb, tb);
                let vr = r(pa, ta, pb, tb);
                eq_i32(&format!("generic f2(0x{ta:08x}, 0x{tb:08x})"), vc, vr);
                if !VALID_ENUMS.contains(&ta) || !VALID_ENUMS.contains(&tb) {
                    assert_eq!(vc, 0, "invalid enum pair must return 0");
                }
            }
        }
    }
    // and a random fuzz over the whole u32 range for both enum slots
    let mut g = Rng::seeded();
    for i in 0..20000 {
        let (ta, tb) = (g.next_u32(), g.next_u32());
        unsafe {
            eq_i32(&format!("generic f2 fuzz #{i} (0x{ta:08x},0x{tb:08x})"), c(pa, ta, pb, tb), r(pa, ta, pb, tb))
        }
    }
}

#[test]
fn generic_integer_boundaries_one_step_past_range() {
    let l = libs();
    let (f3c, f3r) = bind!(l, "f3", FnF3);
    let (f5c, f5r) = bind!(l, "f5", FnF5);
    let (f7c, f7r) = bind!(l, "f7", FnF7);
    let (f10c, f10r) = bind!(l, "f10", FnF10);

    // f3: every neighbour of the guard constants
    let edges = [IMIN, IMIN + 1, IMIN + 2, -2, -1, 0, 1, 2, IMAX - 2, IMAX - 1, IMAX];
    for &v1 in &edges {
        for &v2 in &edges {
            unsafe { eq_i32(&format!("gen f3({v1},{v2})"), f3c(v1, v2), f3r(v1, v2)) }
        }
    }
    // f5: bit 15/16 boundary
    for &a in &[
        0x0000_7FFFu32, 0x0000_8000, 0x0000_FFFF, 0x0001_0000, 0x0001_0001, u32::MAX, 0,
    ] {
        unsafe { eq_u32(&format!("gen f5(0x{a:08x})"), f5c(a), f5r(a)) }
    }
    // f7: one step either side of the two branch constants (2 and 32)
    for ch in [0u32, 1, 2, 3] {
        for bd in [0u32, 31, 32, 33] {
            for bs in [0u32, 1, u32::MAX] {
                unsafe {
                    eq_u32(&format!("gen f7({bs},{ch},{bd})"), f7c(bs, ch, bd), f7r(bs, ch, bd))
                }
            }
        }
    }
    // f10: the extremes of the uint16_t domain (there is no wider input)
    for &h in &[0u16, 1, 0x3FF, 0x400, 0x7BFF, 0x7C00, 0xFBFF, 0xFC00, 0xFFFE, 0xFFFF] {
        unsafe { eq_f32(&format!("gen f10(0x{h:04x})"), f10c(h), f10r(h)) }
    }
}

#[test]
fn generic_zero_and_oversized_lengths() {
    // The three-float writers have a fixed extent; there is no length
    // parameter to abuse. Verify the extent is exactly 3 floats for f12/f13
    // too (f11 covered by E24), using guarded over-allocated buffers.
    let l = libs();
    let mut g = Rng::seeded();
    for name in ["f12", "f13"] {
        let (c, r) = bind!(l, name, FnTriple);
        for i in 0..2000 {
            let src = [g.mixed_f32(), g.mixed_f32(), g.mixed_f32(), 111.0, 222.0];
            let mut dc = [0.0f32, 0.0, 0.0, 333.0, 444.0];
            let mut dr = [0.0f32, 0.0, 0.0, 333.0, 444.0];
            unsafe {
                c(dc.as_mut_ptr(), src.as_ptr());
                r(dr.as_mut_ptr(), src.as_ptr());
            }
            eq_bits(&format!("generic {name} buffer #{i}"), dc, dr);
            assert_eq!(
                (dc[3].to_bits(), dc[4].to_bits()),
                (333.0f32.to_bits(), 444.0f32.to_bits()),
                "generic {name} wrote past dest[2]"
            );
        }
    }
}
