//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1 … E35). Each constructs the exact
//! invalid input / rejection condition, calls BOTH `.so`s, and asserts they
//! return the SAME sentinel (not merely "both failed").
//!
//! Row E36 (a NULL pointer that IS dereferenced) is genuine UB in the C ground
//! truth — `SIGSEGV` in both implementations — and is deliberately not
//! exercised.

mod common;

use common::*;
use std::ffi::{c_int, c_uint, c_void};
use std::ptr;

const N: usize = 20_000;

/// Every out-of-range `C2_TYPE` value a caller can smuggle across the FFI
/// boundary. A C enum accepts any `int`, so all of these are real inputs.
const BAD_TYPES: &[c_uint] = &[
    2,
    3,
    4,
    5,
    127,
    128,
    255,
    256,
    0xFFFF,
    0x7FFF_FFFF,          // INT_MAX
    0x8000_0000,          // INT_MIN reinterpreted
    0xFFFF_FFFE,
    0xFFFF_FFFF,          // -1
];

#[repr(C, align(16))]
#[derive(Copy, Clone)]
struct ShapeBuf([f32; 4]);

impl ShapeBuf {
    fn ptr(&self) -> *const c_void {
        self as *const ShapeBuf as *const c_void
    }
}

fn chk_f2(p: &Pair, a: *const c_void, ta: c_uint, b: *const c_void, tb: c_uint, tag: &str) -> c_int {
    let cv = unsafe { (p.c.f2)(a, ta, b, tb) };
    let rv = unsafe { (p.rs.f2)(a, ta, b, tb) };
    same(tag, (ta, tb), cv, rv);
    cv
}

// ---------------------------------------------------------------------------
// E1 — f2: typeA out of range -> outer switch default -> return 0
// ---------------------------------------------------------------------------

#[test]
fn e1_f2_typea_out_of_range() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xE1);
    let buf = ShapeBuf([1.0, 2.0, 3.0, 4.0]);
    for &ta in BAD_TYPES {
        // typeB spans valid AND invalid values; the outer default wins either way
        for &tb in [0u32, 1, 2, 7, 0xFFFF_FFFF].iter().chain(BAD_TYPES) {
            let got = chk_f2(p, buf.ptr(), ta, buf.ptr(), tb, "E1 f2 typeA invalid");
            assert_eq!(got, 0, "C must reject typeA={ta} with 0, got {got}");
        }
    }
    for _ in 0..N {
        let ta = r.next_u32();
        if ta <= 1 {
            continue;
        }
        let tb = r.next_u32();
        let got = chk_f2(p, buf.ptr(), ta, buf.ptr(), tb, "E1 f2 typeA random-invalid");
        assert_eq!(got, 0);
    }
}

// ---------------------------------------------------------------------------
// E2 — f2: typeA == CIRCLE, typeB out of range -> inner default -> 0
// ---------------------------------------------------------------------------

#[test]
fn e2_f2_circle_typeb_out_of_range() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xE2);
    let buf = ShapeBuf([1.0, 2.0, 3.0, 4.0]);
    for &tb in BAD_TYPES {
        let got = chk_f2(p, buf.ptr(), C2_TYPE_CIRCLE, buf.ptr(), tb, "E2 f2 CIRCLE/bad");
        assert_eq!(got, 0, "C must reject typeB={tb} with 0, got {got}");
    }
    for _ in 0..N {
        let tb = r.next_u32();
        if tb <= 1 {
            continue;
        }
        let got = chk_f2(p, buf.ptr(), C2_TYPE_CIRCLE, buf.ptr(), tb, "E2 f2 CIRCLE/bad-rand");
        assert_eq!(got, 0);
    }
}

// ---------------------------------------------------------------------------
// E3 — f2: typeA == AABB, typeB out of range -> inner default -> 0
// ---------------------------------------------------------------------------

#[test]
fn e3_f2_aabb_typeb_out_of_range() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xE3);
    let buf = ShapeBuf([1.0, 2.0, 3.0, 4.0]);
    for &tb in BAD_TYPES {
        let got = chk_f2(p, buf.ptr(), C2_TYPE_AABB, buf.ptr(), tb, "E3 f2 AABB/bad");
        assert_eq!(got, 0, "C must reject typeB={tb} with 0, got {got}");
    }
    for _ in 0..N {
        let tb = r.next_u32();
        if tb <= 1 {
            continue;
        }
        let got = chk_f2(p, buf.ptr(), C2_TYPE_AABB, buf.ptr(), tb, "E3 f2 AABB/bad-rand");
        assert_eq!(got, 0);
    }
}

// ---------------------------------------------------------------------------
// E4 — f2: NULL pointers on the paths where they are NOT dereferenced
// ---------------------------------------------------------------------------

#[test]
fn e4_f2_null_pointers_on_default_arms() {
    let p = pair();
    let nul: *const c_void = ptr::null();
    let buf = ShapeBuf([1.0, 2.0, 3.0, 4.0]);

    // outer default: neither A nor B is read
    for &ta in BAD_TYPES {
        for &tb in [0u32, 1, 2, 0xFFFF_FFFF].iter() {
            let got = chk_f2(p, nul, ta, nul, tb, "E4 f2 NULL/outer-default");
            assert_eq!(got, 0);
        }
    }
    // inner defaults: still no dereference
    for &tb in BAD_TYPES {
        for ta in [C2_TYPE_CIRCLE, C2_TYPE_AABB] {
            let got = chk_f2(p, nul, ta, nul, tb, "E4 f2 NULL/inner-default");
            assert_eq!(got, 0);
            // mixed: one NULL, one valid
            let got = chk_f2(p, buf.ptr(), ta, nul, tb, "E4 f2 NULL-B/inner-default");
            assert_eq!(got, 0);
            let got = chk_f2(p, nul, ta, buf.ptr(), tb, "E4 f2 NULL-A/inner-default");
            assert_eq!(got, 0);
        }
    }
    // also a misaligned / dangling-but-unread pointer
    let bogus = 0x1usize as *const c_void;
    for &ta in BAD_TYPES {
        let got = chk_f2(p, bogus, ta, bogus, 2, "E4 f2 bogus-ptr/unread");
        assert_eq!(got, 0);
    }
}

// ---------------------------------------------------------------------------
// E5 — f3: v2 == 0 -> return 0
// ---------------------------------------------------------------------------

#[test]
fn e5_f3_divisor_zero() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xE5);
    for &v1 in SPECIAL_I32 {
        let cv = unsafe { (p.c.f3)(v1, 0) };
        let rv = unsafe { (p.rs.f3)(v1, 0) };
        same("E5 f3 v2==0", (v1, 0), cv, rv);
        assert_eq!(cv, 0, "C must return 0 for v2==0 (v1={v1})");
    }
    for _ in 0..N {
        let v1 = r.edgy_i32();
        let cv = unsafe { (p.c.f3)(v1, 0) };
        let rv = unsafe { (p.rs.f3)(v1, 0) };
        same("E5 f3 v2==0/rand", (v1, 0), cv, rv);
        assert_eq!(cv, 0);
    }
}

// ---------------------------------------------------------------------------
// E6 — f3: v1 >= 0 and v2 == INT_MIN (the `-v2` overflow guard)
// ---------------------------------------------------------------------------

#[test]
fn e6_f3_pos_v1_intmin_v2() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xE6);
    let mut checked = 0;
    for &v1 in SPECIAL_I32 {
        if v1 < 0 {
            continue;
        }
        let cv = unsafe { (p.c.f3)(v1, i32::MIN) };
        let rv = unsafe { (p.rs.f3)(v1, i32::MIN) };
        same("E6 f3 (+, INT_MIN)", (v1, i32::MIN), cv, rv);
        // q = 0 and r = v1 >= 0, so the fix-up is skipped and the answer is 0
        assert_eq!(cv, 0, "expected 0 for f3({v1}, INT_MIN), got {cv}");
        checked += 1;
    }
    assert!(checked > 0);
    for _ in 0..N {
        let v1 = r.next_u32() as i32 & i32::MAX;
        let cv = unsafe { (p.c.f3)(v1, i32::MIN) };
        let rv = unsafe { (p.rs.f3)(v1, i32::MIN) };
        same("E6 f3 (+, INT_MIN)/rand", (v1, i32::MIN), cv, rv);
        assert_eq!(cv, 0);
    }
}

// ---------------------------------------------------------------------------
// E7 — f3: v1 < 0 (but != INT_MIN) and v2 == INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn e7_f3_neg_v1_intmin_v2() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xE7);
    for &v1 in SPECIAL_I32 {
        if v1 >= 0 || v1 == i32::MIN {
            continue;
        }
        let cv = unsafe { (p.c.f3)(v1, i32::MIN) };
        let rv = unsafe { (p.rs.f3)(v1, i32::MIN) };
        same("E7 f3 (-, INT_MIN)", (v1, i32::MIN), cv, rv);
        // q = 1, r = v1 - INT_MIN (wrapping) which is >= 0, so result is 1
        assert_eq!(cv, 1, "expected 1 for f3({v1}, INT_MIN), got {cv}");
    }
    for _ in 0..N {
        let v1 = -((r.next_u32() as i32 & i32::MAX).max(1));
        let cv = unsafe { (p.c.f3)(v1, i32::MIN) };
        let rv = unsafe { (p.rs.f3)(v1, i32::MIN) };
        same("E7 f3 (-, INT_MIN)/rand", (v1, i32::MIN), cv, rv);
        assert_eq!(cv, 1);
    }
    // and INT_MIN + 1, the extreme of this branch
    let cv = unsafe { (p.c.f3)(i32::MIN + 1, i32::MIN) };
    same("E7 f3 (INT_MIN+1, INT_MIN)", (), cv, unsafe {
        (p.rs.f3)(i32::MIN + 1, i32::MIN)
    });
    assert_eq!(cv, 1);
}

// ---------------------------------------------------------------------------
// E8 — f3: v1 == INT_MIN and v2 >= 1
// ---------------------------------------------------------------------------

#[test]
fn e8_f3_intmin_v1_pos_v2() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xE8);
    for v2 in 1..=2000i32 {
        let cv = unsafe { (p.c.f3)(i32::MIN, v2) };
        let rv = unsafe { (p.rs.f3)(i32::MIN, v2) };
        same("E8 f3 (INT_MIN, +)", (i32::MIN, v2), cv, rv);
    }
    for &v2 in SPECIAL_I32 {
        if v2 < 1 {
            continue;
        }
        let cv = unsafe { (p.c.f3)(i32::MIN, v2) };
        let rv = unsafe { (p.rs.f3)(i32::MIN, v2) };
        same("E8 f3 (INT_MIN, +)/special", (i32::MIN, v2), cv, rv);
    }
    // documented values
    assert_eq!(unsafe { (p.c.f3)(i32::MIN, 1) }, i32::MIN);
    assert_eq!(unsafe { (p.rs.f3)(i32::MIN, 1) }, i32::MIN);
    assert_eq!(unsafe { (p.c.f3)(i32::MIN, 2) }, -1073741824);
    assert_eq!(unsafe { (p.rs.f3)(i32::MIN, 2) }, -1073741824);
    for _ in 0..N {
        let v2 = (r.next_u32() as i32 & i32::MAX).max(1);
        let cv = unsafe { (p.c.f3)(i32::MIN, v2) };
        let rv = unsafe { (p.rs.f3)(i32::MIN, v2) };
        same("E8 f3 (INT_MIN, +)/rand", (i32::MIN, v2), cv, rv);
    }
}

// ---------------------------------------------------------------------------
// E9 — f3: v1 == INT_MIN, v2 < 0, v2 != INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn e9_f3_intmin_v1_neg_v2() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xE9);
    for v2 in -2000..=-1i32 {
        let cv = unsafe { (p.c.f3)(i32::MIN, v2) };
        let rv = unsafe { (p.rs.f3)(i32::MIN, v2) };
        same("E9 f3 (INT_MIN, -)", (i32::MIN, v2), cv, rv);
    }
    for &v2 in SPECIAL_I32 {
        if v2 >= 0 || v2 == i32::MIN {
            continue;
        }
        let cv = unsafe { (p.c.f3)(i32::MIN, v2) };
        let rv = unsafe { (p.rs.f3)(i32::MIN, v2) };
        same("E9 f3 (INT_MIN, -)/special", (i32::MIN, v2), cv, rv);
    }
    // f3(INT_MIN, -1): q = ((-(INT_MIN+1))/1) + 1 = INT_MAX + 1, which
    // overflows and wraps to INT_MIN; r == 0 so the fix-up is skipped.
    assert_eq!(unsafe { (p.c.f3)(i32::MIN, -1) }, i32::MIN);
    assert_eq!(unsafe { (p.rs.f3)(i32::MIN, -1) }, i32::MIN);
    assert_eq!(unsafe { (p.c.f3)(i32::MIN, -2) }, 1073741824);
    assert_eq!(unsafe { (p.rs.f3)(i32::MIN, -2) }, 1073741824);
    for _ in 0..N {
        let v2 = -((r.next_u32() as i32 & i32::MAX).max(1));
        let cv = unsafe { (p.c.f3)(i32::MIN, v2) };
        let rv = unsafe { (p.rs.f3)(i32::MIN, v2) };
        same("E9 f3 (INT_MIN, -)/rand", (i32::MIN, v2), cv, rv);
    }
    // v2 == INT_MIN + 1, the extreme of this branch
    let cv = unsafe { (p.c.f3)(i32::MIN, i32::MIN + 1) };
    same("E9 f3 (INT_MIN, INT_MIN+1)", (), cv, unsafe {
        (p.rs.f3)(i32::MIN, i32::MIN + 1)
    });
}

// ---------------------------------------------------------------------------
// E10 — f3: both INT_MIN -> q = 1, r = 0 -> return 1
// ---------------------------------------------------------------------------

#[test]
fn e10_f3_both_intmin() {
    let p = pair();
    let cv = unsafe { (p.c.f3)(i32::MIN, i32::MIN) };
    let rv = unsafe { (p.rs.f3)(i32::MIN, i32::MIN) };
    same("E10 f3 (INT_MIN, INT_MIN)", (), cv, rv);
    assert_eq!(cv, 1, "C returns 1 for f3(INT_MIN, INT_MIN), got {cv}");
}

// ---------------------------------------------------------------------------
// E11 — f3: the `r < 0` fix-up path
// ---------------------------------------------------------------------------

#[test]
fn e11_f3_negative_remainder_fixup() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xEB);
    // hand-picked cases where the remainder is negative
    for &(v1, v2) in &[
        (-1i32, 3i32),
        (1, -3),
        (-5, 3),
        (5, -3),
        (-1, i32::MAX),
        (1, -i32::MAX),
        (i32::MIN + 1, 3),
        (i32::MIN + 1, -3),
        (i32::MAX, -2),
        (-i32::MAX, 2),
        (i32::MIN, 3),
        (i32::MIN, -3),
    ] {
        let cv = unsafe { (p.c.f3)(v1, v2) };
        let rv = unsafe { (p.rs.f3)(v1, v2) };
        same("E11 f3 fix-up", (v1, v2), cv, rv);
    }
    // Randomized differential sweep of the whole fix-up region, plus a
    // characterisation of the C's ACTUAL (quirky) rounding rule so a future
    // "cleanup" of the Rust cannot silently change it:
    //
    //   * v1 < 0, v2 > 0 : r = -((-v1) % v2) <= 0  -> fix-up fires -> floors
    //   * v1 < 0, v2 < 0 : r = -((-v1)%(-v2)) <= 0 -> fix-up fires
    //   * v1 >= 0, v2 > 0: early `return v1/v2`    -> truncates (== floor)
    //   * v1 >= 0, v2 < 0: r = v1 % (-v2) >= 0     -> fix-up NEVER fires, so
    //                      the C TRUNCATES toward zero instead of flooring.
    //
    // That last quadrant is a real bug in the C, and it is ground truth.
    let mut fixup_hits = 0usize;
    for _ in 0..N {
        let v1 = r.next_i32() % 100_000;
        let v2 = r.next_i32() % 977;
        let cv = unsafe { (p.c.f3)(v1, v2) };
        let rv = unsafe { (p.rs.f3)(v1, v2) };
        same("E11 f3 fix-up/rand", (v1, v2), cv, rv);
        if v2 == 0 {
            continue;
        }
        let trunc = (v1 as i64) / (v2 as i64);
        let floor = (v1 as i64).div_euclid(v2 as i64);
        let expected = if v1 >= 0 && v2 < 0 {
            trunc // the quirk: fix-up cannot fire here
        } else {
            floor
        };
        assert_eq!(
            cv as i64, expected,
            "f3({v1}, {v2}): C's rounding rule changed"
        );
        if trunc != floor && v1 < 0 {
            fixup_hits += 1;
        }
    }
    assert!(
        fixup_hits > 1000,
        "only {fixup_hits} samples actually exercised the r < 0 fix-up"
    );
    // the fix-up can itself wrap: q + (-1) at INT_MIN
    for &(v1, v2) in &[(i32::MIN, 3i32), (i32::MIN, 7), (i32::MIN + 1, 3)] {
        let cv = unsafe { (p.c.f3)(v1, v2) };
        same("E11 f3 fix-up wrap", (v1, v2), cv, unsafe {
            (p.rs.f3)(v1, v2)
        });
    }
}

// ---------------------------------------------------------------------------
// E12 / E13 — f4 degenerate and wrapping states
// ---------------------------------------------------------------------------

#[test]
fn e12_f4_zero_state() {
    let p = pair();
    let mut sc = CnRnd { state: [0, 0] };
    let mut sr = CnRnd { state: [0, 0] };
    for i in 0..64 {
        let cv = unsafe { (p.c.f4)(&mut sc) };
        let rv = unsafe { (p.rs.f4)(&mut sr) };
        same("E12 f4 zero-state", i, cv, rv);
        same("E12 f4 zero-state/arr", i, sc.state, sr.state);
        assert_eq!(
            cv.to_bits(),
            0,
            "C f4 with an all-zero state must return exactly +0.0"
        );
        assert_eq!(sc.state, [0, 0], "state must stay zero");
    }
}

#[test]
fn e13_f4_wrapping_states() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xED);
    for st in [
        [u64::MAX, u64::MAX],
        [0, u64::MAX],
        [u64::MAX, 0],
        [1, u64::MAX],
        [u64::MAX, 1],
        [1u64 << 63, 1u64 << 63],
        [u64::MAX >> 1, (u64::MAX >> 1) + 1],
    ] {
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        for i in 0..32 {
            let cv = unsafe { (p.c.f4)(&mut sc) };
            let rv = unsafe { (p.rs.f4)(&mut sr) };
            same("E13 f4 wrapping", (st, i), cv, rv);
            same("E13 f4 wrapping/arr", (st, i), sc.state, sr.state);
            assert!(!cv.is_nan(), "f4 must never return NaN");
            assert!((0.0..1.0).contains(&cv), "f4 must stay in [0,1): {cv}");
        }
    }
    for _ in 0..N {
        let st = [r.edgy_u64(), r.edgy_u64()];
        let mut sc = CnRnd { state: st };
        let mut sr = CnRnd { state: st };
        let cv = unsafe { (p.c.f4)(&mut sc) };
        let rv = unsafe { (p.rs.f4)(&mut sr) };
        same("E13 f4 wrapping/rand", st, cv, rv);
        same("E13 f4 wrapping/rand-arr", st, sc.state, sr.state);
    }
}

// ---------------------------------------------------------------------------
// E14 — f5 silently discards bits above bit 15
// ---------------------------------------------------------------------------

#[test]
fn e14_f5_high_bits_discarded() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xEE);
    for &a in &[0xFFFF_0000u32, 0xDEAD_BEEF, u32::MAX, 0x1_0000, 0x8000_0000] {
        let cv = unsafe { (p.c.f5)(a) };
        let rv = unsafe { (p.rs.f5)(a) };
        same("E14 f5 high-bits", a, cv, rv);
        assert!(cv <= 0xFFFF, "f5 must produce a 16-bit value, got {cv:#x}");
        assert_eq!(cv, unsafe { (p.c.f5)(a & 0xFFFF) });
    }
    for _ in 0..N {
        let a = r.next_u32() | 0xFFFF_0000;
        let cv = unsafe { (p.c.f5)(a) };
        let rv = unsafe { (p.rs.f5)(a) };
        same("E14 f5 high-bits/rand", a, cv, rv);
        assert!(cv <= 0xFFFF);
    }
}

// ---------------------------------------------------------------------------
// E15 / E16 / E17 — f7 overflow and mode boundaries
// ---------------------------------------------------------------------------

#[test]
fn e15_f7_unsigned_overflow() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xEF);
    for &(bs, ch, bd) in &[
        (u32::MAX, u32::MAX, u32::MAX),
        (u32::MAX, 2, u32::MAX),
        (u32::MAX, 1, u32::MAX),
        (0x8000_0000, 0x8000_0000, 0x8000_0000),
        (1, u32::MAX, 1),
        (u32::MAX, u32::MAX - 17, 1),
        (0xFFFF_FFFF, 0, 0xFFFF_FFFF),
        (0x1_0000, 3, 0x1_0000),
    ] {
        same(
            "E15 f7 overflow",
            (bs, ch, bd),
            unsafe { (p.c.f7)(bs, ch, bd) },
            unsafe { (p.rs.f7)(bs, ch, bd) },
        );
    }
    // `18 + channels` overflow
    for ch in [u32::MAX - 18, u32::MAX - 17, u32::MAX - 1, u32::MAX] {
        for &bd in &[0u32, 8, 32] {
            same(
                "E15 f7 channels-overflow",
                (0u32, ch, bd),
                unsafe { (p.c.f7)(0, ch, bd) },
                unsafe { (p.rs.f7)(0, ch, bd) },
            );
        }
    }
    for _ in 0..N {
        let (bs, ch, bd) = (r.next_u32(), r.next_u32(), r.next_u32());
        same(
            "E15 f7 overflow/rand",
            (bs, ch, bd),
            unsafe { (p.c.f7)(bs, ch, bd) },
            unsafe { (p.rs.f7)(bs, ch, bd) },
        );
    }
}

#[test]
fn e16_f7_channels_equals_2_mode() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF0);
    // channels == 2 is a distinct mode: the `channels != 2` term vanishes
    for &bd in &[0u32, 1, 8, 16, 24, 31, 32, 33, u32::MAX] {
        for &bs in &[0u32, 1, 4096, 65535, u32::MAX] {
            same(
                "E16 f7 ch==2",
                (bs, 2u32, bd),
                unsafe { (p.c.f7)(bs, 2, bd) },
                unsafe { (p.rs.f7)(bs, 2, bd) },
            );
            // and the immediate neighbours, so the predicate boundary is pinned
            for ch in [1u32, 3] {
                same(
                    "E16 f7 ch near 2",
                    (bs, ch, bd),
                    unsafe { (p.c.f7)(bs, ch, bd) },
                    unsafe { (p.rs.f7)(bs, ch, bd) },
                );
            }
        }
    }
    for _ in 0..N {
        let bs = r.next_u32();
        let bd = r.next_u32();
        same(
            "E16 f7 ch==2/rand",
            (bs, 2u32, bd),
            unsafe { (p.c.f7)(bs, 2, bd) },
            unsafe { (p.rs.f7)(bs, 2, bd) },
        );
    }
}

#[test]
fn e17_f7_bitdepth_equals_32_mode() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF1);
    for &ch in &[0u32, 1, 2, 3, u32::MAX] {
        for &bs in &[0u32, 1, 4096, 65535, u32::MAX] {
            for bd in [31u32, 32, 33] {
                same(
                    "E17 f7 bd near 32",
                    (bs, ch, bd),
                    unsafe { (p.c.f7)(bs, ch, bd) },
                    unsafe { (p.rs.f7)(bs, ch, bd) },
                );
            }
        }
    }
    for _ in 0..N {
        let bs = r.next_u32();
        same(
            "E17 f7 ch==2,bd==32/rand",
            (bs, 2u32, 32u32),
            unsafe { (p.c.f7)(bs, 2, 32) },
            unsafe { (p.rs.f7)(bs, 2, 32) },
        );
    }
}

// ---------------------------------------------------------------------------
// E18 / E19 — f9 division by zero and NaN/Inf propagation
// ---------------------------------------------------------------------------

fn chk_f9(p: &Pair, a: LmVec2, b: LmVec2, c: LmVec2, q: LmVec2, tag: &str) -> LmVec2 {
    let cv = unsafe { (p.c.f9)(a, b, c, q) };
    let rv = unsafe { (p.rs.f9)(a, b, c, q) };
    same(
        tag,
        (
            (a.x.to_bits(), a.y.to_bits()),
            (b.x.to_bits(), b.y.to_bits()),
            (c.x.to_bits(), c.y.to_bits()),
            (q.x.to_bits(), q.y.to_bits()),
        ),
        cv,
        rv,
    );
    cv
}

fn lv(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

#[test]
fn e18_f9_degenerate_division_by_zero() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF2);
    // all three points coincident -> denom == 0 -> 1.0f / 0.0f
    let got = chk_f9(
        p,
        lv(0.0, 0.0),
        lv(0.0, 0.0),
        lv(0.0, 0.0),
        lv(1.0, 1.0),
        "E18 f9 coincident",
    );
    assert!(
        got.x.is_nan() || got.x.is_infinite() || got.x == 0.0,
        "unguarded 1/0 must yield NaN/Inf/0, got {}",
        got.x
    );
    for _ in 0..N {
        let a = lv(r.finite_f32(10.0), r.finite_f32(10.0));
        let b = lv(r.finite_f32(10.0), r.finite_f32(10.0));
        let q = lv(r.finite_f32(10.0), r.finite_f32(10.0));
        chk_f9(p, a, a, b, q, "E18 f9 p1==p2");
        chk_f9(p, a, b, a, q, "E18 f9 p1==p3");
        chk_f9(p, a, b, b, q, "E18 f9 p2==p3");
        chk_f9(p, a, a, a, q, "E18 f9 all-equal");
        // exactly collinear
        let t = r.finite_f32(4.0);
        let c = lv(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
        chk_f9(p, a, b, c, q, "E18 f9 collinear");
    }
}

#[test]
fn e19_f9_nan_inf_propagation() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF3);
    let specials: Vec<f32> = special_f32_values();
    // every special in every one of the 8 coordinate slots, one at a time
    for &s in &specials {
        for slot in 0..8 {
            let mut c = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.3, 0.3];
            c[slot] = s;
            chk_f9(
                p,
                lv(c[0], c[1]),
                lv(c[2], c[3]),
                lv(c[4], c[5]),
                lv(c[6], c[7]),
                "E19 f9 one-hot special",
            );
        }
    }
    for _ in 0..N {
        chk_f9(
            p,
            r.raw_lmv(),
            r.raw_lmv(),
            r.raw_lmv(),
            r.raw_lmv(),
            "E19 f9 raw",
        );
    }
}

// ---------------------------------------------------------------------------
// E20 — f10 table-index boundaries (exhaustive: no rejection path exists)
// ---------------------------------------------------------------------------

#[test]
fn e20_f10_index_boundaries_exhaustive() {
    let p = pair();
    // last in-bounds element: h = 0xFFFF -> n = 63, offset 0x400, low 0x3FF
    // -> m__mantissa[2047]
    for h in [0u16, 1, 0x3FF, 0x400, 0xFBFF, 0xFC00, 0xFFFE, 0xFFFF] {
        same("E20 f10 boundary", h, unsafe { (p.c.f10)(h) }, unsafe {
            (p.rs.f10)(h)
        });
    }
    // exhaustive: proves no index is ever out of range in either impl
    let mut h: u32 = 0;
    while h <= 0xFFFF {
        let hh = h as u16;
        same("E20 f10 exhaustive", hh, unsafe { (p.c.f10)(hh) }, unsafe {
            (p.rs.f10)(hh)
        });
        h += 1;
    }
    // every bucket boundary (n transitions)
    for n in 0u16..64 {
        for lo in [0u16, 0x3FF] {
            let hh = (n << 10) | lo;
            same("E20 f10 bucket", hh, unsafe { (p.c.f10)(hh) }, unsafe {
                (p.rs.f10)(hh)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// E21 / E22 / E23 — f11 rejection / fall-through arms
// ---------------------------------------------------------------------------

fn chk_f1x(p: &Pair, which: u8, src: [f32; 3], tag: &str) -> [f32; 3] {
    let (cf, rf) = match which {
        11 => (p.c.f11, p.rs.f11),
        12 => (p.c.f12, p.rs.f12),
        _ => (p.c.f13, p.rs.f13),
    };
    let cv = call_f1x(cf, src);
    let rv = call_f1x(rf, src);
    same(tag, src.map(f32::to_bits), cv, rv);
    cv
}

#[test]
fn e21_f11_saturation_zero_early_return() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF4);
    for &s in &[0.0f32, -0.0] {
        for &lb in SPECIAL_F32 {
            let l = f32::from_bits(lb);
            for &hb in SPECIAL_F32 {
                let out = chk_f1x(p, 11, [f32::from_bits(hb), s, l], "E21 f11 s==0");
                // the early return copies `l` into all three slots verbatim
                assert_eq!(out[0].to_bits(), l.to_bits());
                assert_eq!(out[1].to_bits(), l.to_bits());
                assert_eq!(out[2].to_bits(), l.to_bits());
            }
        }
    }
    for _ in 0..N {
        let s = if r.next_u32() & 1 == 0 { 0.0 } else { -0.0 };
        chk_f1x(p, 11, [r.raw_f32(), s, r.raw_f32()], "E21 f11 s==0/rand");
    }
}

#[test]
fn e22_f11_final_else_arm() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF5);
    // reachable only for h NaN or h >= 360
    for &hb in &[
        0x7FC0_0000u32,
        0xFFC0_0000,
        0x7F80_0001,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
    ] {
        for &s in &[1.0f32, 0.5, -1.0, 2.0] {
            for &l in &[0.0f32, 0.5, 1.0, -1.0] {
                let out = chk_f1x(p, 11, [f32::from_bits(hb), s, l], "E22 f11 else/NaN");
                assert_eq!(out[0].to_bits(), out[1].to_bits());
                assert_eq!(out[1].to_bits(), out[2].to_bits());
            }
        }
    }
    for h in [360.0f32, 360.00003, 361.0, 1e30, f32::MAX, f32::INFINITY] {
        for &s in &[1.0f32, 0.5, -1.0] {
            for &l in &[0.0f32, 0.5, 1.0] {
                let out = chk_f1x(p, 11, [h, s, l], "E22 f11 else/h>=360");
                assert_eq!(out[0].to_bits(), out[1].to_bits());
                assert_eq!(out[1].to_bits(), out[2].to_bits());
            }
        }
    }
    for _ in 0..N {
        let h = 360.0 + r.range_f32(0.0, 1e7);
        chk_f1x(p, 11, [h, r.nice_f32(2.0), r.nice_f32(2.0)], "E22 f11 else/rand");
    }
}

#[test]
fn e23_f11_negative_hue_takes_third_arm() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF6);
    // The C's third test is `h < 120 && h < 180`, so h < 0 lands there — NOT in
    // the final `else`. Verify by comparing against an in-sector hue that takes
    // the same arm.
    for h in [-0.00001f32, -1.0, -60.0, -180.0, -1e30, f32::NEG_INFINITY, f32::MIN] {
        for &s in &[1.0f32, 0.5, -1.0] {
            for &l in &[0.25f32, 0.5, 0.75] {
                let out = chk_f1x(p, 11, [h, s, l], "E23 f11 h<0 -> third arm");
                // third arm sets dest[0] = m and dest[1] = c + m, which differ
                // whenever c != 0; the `else` arm would make all three equal.
                let ref_out = chk_f1x(p, 11, [150.0, s, l], "E23 f11 ref [120,180)");
                // both must agree on dest[0] == m
                assert_eq!(
                    out[0].to_bits(),
                    ref_out[0].to_bits(),
                    "h={h} should take the same arm as h=150"
                );
            }
        }
    }
    for _ in 0..N {
        let h = -r.range_f32(0.0, 1e7);
        chk_f1x(p, 11, [h, r.nice_f32(2.0), r.nice_f32(2.0)], "E23 f11 h<0/rand");
    }
}

// ---------------------------------------------------------------------------
// E24 … E27 — f12 rejection / default arms
// ---------------------------------------------------------------------------

#[test]
fn e24_f12_saturation_zero_early_return() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF7);
    for &s in &[0.0f32, -0.0] {
        for &vb in SPECIAL_F32 {
            let v = f32::from_bits(vb);
            for &hb in SPECIAL_F32 {
                let out = chk_f1x(p, 12, [f32::from_bits(hb), s, v], "E24 f12 s==0");
                assert_eq!(out[0].to_bits(), v.to_bits());
                assert_eq!(out[1].to_bits(), v.to_bits());
                assert_eq!(out[2].to_bits(), v.to_bits());
            }
        }
    }
    for _ in 0..N {
        let s = if r.next_u32() & 1 == 0 { 0.0 } else { -0.0 };
        chk_f1x(p, 12, [r.raw_f32(), s, r.raw_f32()], "E24 f12 s==0/rand");
    }
}

#[test]
fn e25_f12_negative_index_default_arm() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF8);
    // i < 0 -> `cmpl $4, i; ja` (unsigned) -> default
    for h in [
        -1e-30f32, -0.001, -1.0, -59.9, -60.0, -60.1, -120.0, -1e6, -1e30, f32::MIN,
        f32::NEG_INFINITY,
    ] {
        for &s in &[1.0f32, 0.5, -1.0, 2.0] {
            for &v in &[0.0f32, 0.5, 1.0, -1.0] {
                chk_f1x(p, 12, [h, s, v], "E25 f12 i<0");
            }
        }
    }
    for _ in 0..N {
        let h = -r.range_f32(1e-6, 1e7);
        let mut s = r.finite_f32(2.0);
        if s == 0.0 {
            s = 1.0;
        }
        chk_f1x(p, 12, [h, s, r.nice_f32(2.0)], "E25 f12 i<0/rand");
    }
}

#[test]
fn e26_f12_index_ge_5_default_arm() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xF9);
    for h in [300.0f32, 359.9, 360.0, 420.0, 600.0, 1e6, 1e9] {
        for &s in &[1.0f32, 0.5, -1.0, 2.0] {
            for &v in &[0.0f32, 0.5, 1.0, -1.0] {
                chk_f1x(p, 12, [h, s, v], "E26 f12 i>=5");
            }
        }
    }
    for _ in 0..N {
        let h = r.range_f32(300.0, 1e7);
        let mut s = r.finite_f32(2.0);
        if s == 0.0 {
            s = 1.0;
        }
        chk_f1x(p, 12, [h, s, r.nice_f32(2.0)], "E26 f12 i>=5/rand");
    }
}

#[test]
fn e27_f12_unrepresentable_int_conversion() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xFA);
    // (int)floorf(h/60) with h/60 not representable in `int`:
    // C leaves it undefined; x86-64 cvttss2si yields INT_MIN -> default arm.
    let hs: &[f32] = &[
        f32::NAN,
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFFF_FFFF),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        // exactly at the 2^31 conversion boundary, in h/60 terms
        2147483648.0 * 60.0,
        -2147483648.0 * 60.0,
        2147483520.0 * 60.0,
        1.2884902e11,
        -1.2884902e11,
    ];
    for &h in hs {
        for &s in &[1.0f32, 0.5, -1.0, 2.0, f32::INFINITY, f32::NAN] {
            for &v in &[0.0f32, 0.5, 1.0, -1.0, f32::INFINITY, f32::NAN] {
                chk_f1x(p, 12, [h, s, v], "E27 f12 (int) UB");
            }
        }
    }
    // and around the boundary with random perturbations
    for _ in 0..N {
        let base = 2147483648.0f32 * 60.0;
        let h = f32::from_bits(base.to_bits().wrapping_add(r.next_u32() % 64).wrapping_sub(32));
        let sign = if r.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        let mut s = r.finite_f32(2.0);
        if s == 0.0 {
            s = 1.0;
        }
        chk_f1x(p, 12, [h * sign, s, r.nice_f32(2.0)], "E27 f12 (int) UB/rand");
    }
}

// ---------------------------------------------------------------------------
// E28 … E32 — f13 rejection / guard branches
// ---------------------------------------------------------------------------

#[test]
fn e28_f13_delta_zero_early_return() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xFB);
    for &vb in SPECIAL_F32 {
        let v = f32::from_bits(vb);
        let out = chk_f1x(p, 13, [v, v, v], "E28 f13 delta==0");
        if !v.is_nan() && v != 0.0 && v.is_finite() {
            assert_eq!(out[0].to_bits(), 0, "h must be +0.0");
            assert_eq!(out[1].to_bits(), 0, "s must be +0.0");
            assert_eq!(out[2].to_bits(), v.to_bits(), "v must be max");
        }
    }
    for _ in 0..N {
        let v = r.finite_f32(1e6);
        chk_f1x(p, 13, [v, v, v], "E28 f13 delta==0/rand");
    }
}

#[test]
fn e29_f13_max_zero_guard() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xFC);
    // max == 0 with a non-zero delta: requires a negative channel
    for &t in &[
        [0.0f32, 0.0, -1.0],
        [0.0, -1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, -2.0],
        [-0.0, 0.0, -1.0],
        [0.0, -f32::MAX, -1.0],
        [-f32::MIN_POSITIVE, 0.0, -1.0],
    ] {
        let out = chk_f1x(p, 13, t, "E29 f13 max==0");
        assert_eq!(out[0].to_bits(), 0, "h must be +0.0 for {t:?}");
        assert_eq!(out[1].to_bits(), 0, "s must be +0.0 for {t:?}");
        assert_eq!(out[2].to_bits(), 0, "v == max == 0 for {t:?}");
    }
    for _ in 0..N {
        // exactly one channel is 0 and it is the maximum
        let neg1 = -r.range_f32(1e-6, 1e6);
        let neg2 = -r.range_f32(1e-6, 1e6);
        for t in [[0.0, neg1, neg2], [neg1, 0.0, neg2], [neg1, neg2, 0.0]] {
            let out = chk_f1x(p, 13, t, "E29 f13 max==0/rand");
            assert_eq!(out[2].to_bits(), 0);
        }
    }
}

#[test]
fn e30_f13_nan_never_displaces_incumbent() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xFD);
    let nans = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFFF_FFFF),
    ];
    // one NaN in each slot, exhaustively over the other two from the corpus
    for &nn in &nans {
        for slot in 0..3usize {
            for &ab in SPECIAL_F32 {
                for &bb in SPECIAL_F32 {
                    let mut t = [f32::from_bits(ab), f32::from_bits(bb), 0.0f32];
                    // rotate so the NaN lands in `slot`
                    let others = [f32::from_bits(ab), f32::from_bits(bb)];
                    let mut k = 0;
                    for i in 0..3 {
                        if i == slot {
                            t[i] = nn;
                        } else {
                            t[i] = others[k];
                            k += 1;
                        }
                    }
                    chk_f1x(p, 13, t, "E30 f13 NaN in min/max");
                }
            }
        }
    }
    // two and three NaNs
    for &a in &nans {
        for &b in &nans {
            chk_f1x(p, 13, [a, b, 1.0], "E30 f13 2 NaN");
            chk_f1x(p, 13, [a, 1.0, b], "E30 f13 2 NaN b");
            chk_f1x(p, 13, [1.0, a, b], "E30 f13 2 NaN c");
            for &c in &nans {
                chk_f1x(p, 13, [a, b, c], "E30 f13 3 NaN");
            }
        }
    }
    // infinities
    for &a in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &b in &[f32::INFINITY, f32::NEG_INFINITY, 0.0, 1.0] {
            for &c in &[f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
                chk_f1x(p, 13, [a, b, c], "E30 f13 Inf");
            }
        }
    }
    for _ in 0..N {
        chk_f1x(p, 13, [r.raw_f32(), r.raw_f32(), r.raw_f32()], "E30 f13 raw");
    }
}

#[test]
fn e31_f13_hue_wrap_fixup() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0xFE);
    // r == max and g < b -> (g-b)/delta < 0 -> h < 0 -> h += 360
    for _ in 0..N {
        let rr = r.range_f32(0.5, 1.0);
        let b = r.range_f32(0.1, 0.5);
        let g = r.range_f32(0.0, b);
        let out = chk_f1x(p, 13, [rr, g, b], "E31 f13 h wrap");
        if g < b && rr > b {
            assert!(
                out[0] >= 0.0 && out[0] < 360.0,
                "hue must be wrapped into [0,360): {} for {:?}",
                out[0],
                [rr, g, b]
            );
        }
    }
    // exactly h == -0.0 boundary: (g - b) == -0.0
    for &t in &[
        [1.0f32, 0.0, 0.0],
        [1.0, -0.0, 0.0],
        [1.0, 0.0, -0.0],
        [1.0, 0.5, 0.5],
    ] {
        chk_f1x(p, 13, t, "E31 f13 h==+-0");
    }
    // and h just barely negative
    for _ in 0..N {
        let rr = 1.0f32;
        let b = r.range_f32(0.0, 1.0);
        let g = f32::from_bits(b.to_bits().wrapping_sub(1));
        chk_f1x(p, 13, [rr, g, b], "E31 f13 h tiny-negative");
    }
}

#[test]
fn e32_f13_negative_max_and_saturation_overflow() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x100);
    // all channels negative -> max < 0 -> negative s
    for &t in &[
        [-1.0f32, -2.0, -3.0],
        [-3.0, -2.0, -1.0],
        [-2.0, -1.0, -3.0],
        [-1e30, -1e-30, -1.0],
        [f32::MIN, -1.0, -2.0],
    ] {
        let out = chk_f1x(p, 13, t, "E32 f13 negative max");
        assert!(out[2] < 0.0, "v = max must be negative for {t:?}");
    }
    for _ in 0..N {
        let t = [
            -r.range_f32(1e-6, 1e6),
            -r.range_f32(1e-6, 1e6),
            -r.range_f32(1e-6, 1e6),
        ];
        chk_f1x(p, 13, t, "E32 f13 negative max/rand");
    }
    // delta / max overflows to +inf: tiny subnormal max, huge delta
    for &t in &[
        [f32::from_bits(1), -f32::MAX, 0.0f32],
        [f32::MIN_POSITIVE, f32::MIN, 0.0],
        [f32::from_bits(1), f32::MIN, f32::MIN],
    ] {
        chk_f1x(p, 13, t, "E32 f13 s overflow");
    }
    for _ in 0..N {
        let m = f32::from_bits(r.next_u32() % 4096 + 1); // subnormal
        let lo = -r.range_f32(1e30, 3e38);
        chk_f1x(p, 13, [m, lo, lo], "E32 f13 s overflow/rand");
    }
}

// ---------------------------------------------------------------------------
// E33 / E34 — agglom's isnan() filters and error rows through the aggregate
// ---------------------------------------------------------------------------

#[rustfmt::skip]
fn agglom_call(f: FnAgglom, v: &[f64; 0], a: &AgglomArgs) -> f64 {
    let _ = v;
    unsafe {
        f(
            a.f2[0], a.f2[1], a.f2[2], a.f2[3], a.f2[4], a.f2[5], a.f2[6],
            a.f3[0], a.f3[1],
            a.f4[0], a.f4[1],
            a.f5,
            a.f7[0], a.f7[1], a.f7[2],
            a.f9[0], a.f9[1], a.f9[2], a.f9[3], a.f9[4], a.f9[5], a.f9[6], a.f9[7],
            a.f10,
            a.f11[0], a.f11[1], a.f11[2],
            a.f12[0], a.f12[1], a.f12[2],
            a.f13[0], a.f13[1], a.f13[2],
        )
    }
}

#[derive(Copy, Clone, Debug)]
struct AgglomArgs {
    f2: [f32; 7],
    f3: [c_int; 2],
    f4: [u64; 2],
    f5: u32,
    f7: [u32; 3],
    f9: [f32; 8],
    f10: u16,
    f11: [f32; 3],
    f12: [f32; 3],
    f13: [f32; 3],
}

impl AgglomArgs {
    fn baseline() -> AgglomArgs {
        AgglomArgs {
            f2: [0.5, 0.5, 1.0, 0.0, 0.0, 2.0, 2.0],
            f3: [17, 5],
            f4: [0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210],
            f5: 0xBEEF,
            f7: [4096, 2, 16],
            f9: [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.3, 0.3],
            f10: 0x3C00,
            f11: [30.0, 0.5, 0.5],
            f12: [200.0, 0.5, 0.5],
            f13: [0.2, 0.6, 0.4],
        }
    }
}

#[track_caller]
fn chk_agglom(p: &Pair, a: &AgglomArgs, tag: &str) {
    same(
        tag,
        format!("{a:?}"),
        agglom_call(p.c.agglom, &[], a),
        agglom_call(p.rs.agglom, &[], a),
    );
}

#[test]
fn e33_agglom_isnan_filters() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x101);
    let nans: Vec<f32> = vec![
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFFF_FFFF),
    ];
    // each of the 13 filters, driven one at a time
    for &nn in &nans {
        for slot in 0..8 {
            let mut a = AgglomArgs::baseline();
            a.f9[slot] = nn;
            chk_agglom(p, &a, "E33 agglom f9-NaN");
        }
        for slot in 0..3 {
            let mut a = AgglomArgs::baseline();
            a.f11[slot] = nn;
            chk_agglom(p, &a, "E33 agglom f11-NaN");
            let mut a = AgglomArgs::baseline();
            a.f12[slot] = nn;
            chk_agglom(p, &a, "E33 agglom f12-NaN");
            let mut a = AgglomArgs::baseline();
            a.f13[slot] = nn;
            chk_agglom(p, &a, "E33 agglom f13-NaN");
        }
    }
    // half-float NaN encodings for the f10 filter
    for m in 1u16..=0x3FF {
        for sign in [0u16, 0x8000] {
            let mut a = AgglomArgs::baseline();
            a.f10 = sign | 0x7C00 | m;
            chk_agglom(p, &a, "E33 agglom f10-NaN");
        }
    }
    // all filters firing simultaneously
    for &nn in &nans {
        let mut a = AgglomArgs::baseline();
        a.f9 = [nn; 8];
        a.f11 = [nn; 3];
        a.f12 = [nn; 3];
        a.f13 = [nn; 3];
        a.f10 = 0x7E00;
        chk_agglom(p, &a, "E33 agglom all-NaN");
    }
    for _ in 0..N {
        let mut a = AgglomArgs::baseline();
        for i in 0..8 {
            if r.next_u32() % 3 == 0 {
                a.f9[i] = nans[r.below(4) as usize];
            }
        }
        for i in 0..3 {
            if r.next_u32() % 3 == 0 {
                a.f11[i] = nans[r.below(4) as usize];
            }
            if r.next_u32() % 3 == 0 {
                a.f12[i] = nans[r.below(4) as usize];
            }
            if r.next_u32() % 3 == 0 {
                a.f13[i] = nans[r.below(4) as usize];
            }
        }
        chk_agglom(p, &a, "E33 agglom NaN/rand");
    }
}

#[test]
fn e34_agglom_f3_error_rows() {
    let p = pair();
    // every f3 error row, routed through the aggregate entry point
    for &(v1, v2) in &[
        (17i32, 0i32),
        (0, 0),
        (i32::MIN, 0),
        (i32::MAX, 0),
        (0, i32::MIN),
        (17, i32::MIN),
        (-17, i32::MIN),
        (i32::MIN, i32::MIN),
        (i32::MIN, 1),
        (i32::MIN, -1),
        (i32::MIN, i32::MAX),
        (i32::MIN, i32::MIN + 1),
        (i32::MAX, i32::MIN),
        (i32::MIN + 1, i32::MIN),
        (-1, 3),
        (1, -3),
    ] {
        let mut a = AgglomArgs::baseline();
        a.f3 = [v1, v2];
        chk_agglom(p, &a, "E34 agglom f3 error rows");
    }
    // f7 overflow and f5 high bits through the aggregate too
    for &f7 in &[
        [u32::MAX, u32::MAX, u32::MAX],
        [u32::MAX, 2, 32],
        [0, 2, 32],
        [4096, 0, 0],
    ] {
        let mut a = AgglomArgs::baseline();
        a.f7 = f7;
        chk_agglom(p, &a, "E34 agglom f7 overflow");
    }
    for &f5 in &[u32::MAX, 0xFFFF_0000, 0, 0x8000_0000] {
        let mut a = AgglomArgs::baseline();
        a.f5 = f5;
        chk_agglom(p, &a, "E34 agglom f5 high bits");
    }
    // f4 degenerate state
    for &f4 in &[[0u64, 0], [u64::MAX, u64::MAX]] {
        let mut a = AgglomArgs::baseline();
        a.f4 = f4;
        chk_agglom(p, &a, "E34 agglom f4 degenerate");
    }
}

// ---------------------------------------------------------------------------
// E35 — f11 / f12 / f13 with dest == src
// ---------------------------------------------------------------------------

#[test]
fn e35_f1x_full_aliasing() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x102);
    for which in [11u8, 12, 13] {
        let (cf, rf) = match which {
            11 => (p.c.f11, p.rs.f11),
            12 => (p.c.f12, p.rs.f12),
            _ => (p.c.f13, p.rs.f13),
        };
        for &a in SPECIAL_F32 {
            for &b in SPECIAL_F32 {
                let src = [f32::from_bits(a), f32::from_bits(b), f32::from_bits(a)];
                same(
                    "E35 f1x aliased",
                    (which, src.map(f32::to_bits)),
                    call_f1x_aliased(cf, src),
                    call_f1x_aliased(rf, src),
                );
            }
        }
        for _ in 0..N {
            let src = [r.raw_f32(), r.raw_f32(), r.raw_f32()];
            same(
                "E35 f1x aliased/raw",
                (which, src.map(f32::to_bits)),
                call_f1x_aliased(cf, src),
                call_f1x_aliased(rf, src),
            );
        }
    }
}
