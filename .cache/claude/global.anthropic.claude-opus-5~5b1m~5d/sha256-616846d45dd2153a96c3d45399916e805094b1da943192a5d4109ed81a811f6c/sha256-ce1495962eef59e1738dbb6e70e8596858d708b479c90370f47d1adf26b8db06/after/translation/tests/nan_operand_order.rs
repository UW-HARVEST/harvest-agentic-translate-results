//! Pins the ONE aspect of this library that IEEE-754 leaves unspecified: which
//! NaN a binary SSE op propagates when *both* operands are NaNs with different
//! payloads/signs.
//!
//! SSE rule (Intel SDM, `ADDSS`/`MULSS`/`SUBSS`/`DIVSS`): if the **destination**
//! operand is a NaN, the destination is returned (quieted if signalling);
//! otherwise, if the source operand is a NaN, the source is returned. So the
//! observable NaN identifies which operand the compiler put in the destination
//! register — an instruction-scheduling detail, not a language-level one.
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `-O` flag, so the
//! documented build
//!
//! ```text
//! cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
//! ```
//!
//! produces an unoptimised (`-O0`) shared object. The Rust translation pins the
//! operand orders GCC emits at `-O0`:
//!
//! | C expression | GCC `-O0` instruction | destination |
//! |---|---|---|
//! | `a.x * b.x` in `c2Dot`   | `mulss %xmm0,%xmm1` (`xmm1=a.x`, `xmm0=b.x`) | `a.x` |
//! | `a.y * b.y` in `c2Dot`   | `mulss %xmm2,%xmm0` (`xmm0=b.y`, `xmm2=a.y`) | `b.y` |
//! | `px + py`   in `c2Dot`   | `addss %xmm1,%xmm0` (`xmm0=py`, `xmm1=px`)   | `py`  |
//! | `a.x *= b`  in `c2Mulvs` | `mulss -0xc(%rbp),%xmm0` (`xmm0=a.x`)        | `a.x` |
//! | `a.x -= b.x` in `c2Sub`  | `subss %xmm1,%xmm0` (`xmm0=a.x`)            | `a.x` |
//! | `da / c2Dot(n,n)`        | `divss %xmm0,%xmm1` (`xmm1=da`)             | `da`  |
//!
//! (The last two are non-commutative, so they are fixed at every `-O` level.)
//!
//! These tests assert the *C's* observed convention directly. If the C `.so` is
//! ever rebuilt with different optimisation flags, GCC re-associates the
//! commutative `fmul`/`fadd` operands and these tests fail with a message
//! telling you exactly which pin to move in `translation/src/lib.rs`.

mod common;
use common::*;

/// The C `.so`'s observed operand-order convention for `c2Dot`.
#[test]
fn c_dot_operand_order_is_the_documented_o0_one() {
    let n1 = f32::from_bits(0x7FC0_0001);
    let n2 = f32::from_bits(0xFFC0_0002);
    let n3 = f32::from_bits(0x7FC0_0003);
    let n4 = f32::from_bits(0xFFC0_0004);

    let probe = |a: C2v, b: C2v| -> u32 { (c().c2Dot)(a, b).to_bits() };

    let observed = [
        probe(C2v { x: n1, y: 1.0 }, C2v { x: n2, y: 1.0 }),
        probe(C2v { x: 1.0, y: n1 }, C2v { x: 1.0, y: n2 }),
        probe(C2v { x: n1, y: n3 }, C2v { x: n2, y: n4 }),
        probe(C2v { x: n1, y: n3 }, C2v { x: 1.0, y: 1.0 }),
    ];
    // px = mulss(dst=a.x, src=b.x); py = mulss(dst=b.y, src=a.y);
    // res = addss(dst=py, src=px)
    let expected_o0 = [0x7FC0_0001u32, 0xFFC0_0002, 0xFFC0_0004, 0x7FC0_0003];
    // What GCC -O1/-O2/-Os/-O3 emit instead (all four collapse to a.x's NaN).
    let expected_optimised = [0x7FC0_0001u32, 0x7FC0_0001, 0x7FC0_0001, 0x7FC0_0001];

    assert_eq!(
        observed,
        expected_o0,
        "\nThe C .so's c2Dot NaN operand order changed.\n\
         observed        = {observed:08x?}\n\
         expected (-O0)  = {expected_o0:08x?}\n\
         gcc -O1/-O2/-Os = {expected_optimised:08x?}\n\
         The documented build (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)\n\
         has no CMAKE_BUILD_TYPE and therefore compiles at -O0. If you built the\n\
         C with optimisation, rebuild it as documented, or re-pin `c2Dot` in\n\
         translation/src/lib.rs to:\n  \
           add_keep_lhs_nan(mul_keep_lhs_nan(a.x, b.x), mul_keep_lhs_nan(a.y, b.y))\n"
    );
}

/// The C `.so`'s observed operand-order convention for `c2Mulvs`.
#[test]
fn c_mulvs_operand_order_is_the_documented_o0_one() {
    let n1 = f32::from_bits(0x7FC0_0001);
    let n2 = f32::from_bits(0xFFC0_0002);
    let n3 = f32::from_bits(0x7FC0_0003);

    let observed = vbits((c().c2Mulvs)(C2v { x: n1, y: n3 }, n2));
    // mulss(dst = a.x, src = b) and mulss(dst = a.y, src = b)
    let expected_o0 = (0x7FC0_0001u32, 0x7FC0_0003u32);

    assert_eq!(
        observed,
        expected_o0,
        "\nThe C .so's c2Mulvs NaN operand order changed.\n\
         observed       = {observed:08x?}\n\
         expected (-O0) = {expected_o0:08x?}\n\
         gcc -O1/-O2/-Os = (0x7fc00001, 0xffc00002)  [second lane vectorised]\n\
         gcc -O3/-Ofast  = (0xffc00002, 0xffc00002)  [mulps, scalar broadcast is dst]\n\
         Rebuild the C as documented (-O0), or re-pin `c2Mulvs` in\n\
         translation/src/lib.rs accordingly.\n"
    );
}

/// The non-commutative ops are pinned by the language, not the scheduler — this
/// documents that they need no `asm!` and are stable at every `-O` level.
#[test]
fn noncommutative_ops_need_no_pinning() {
    let n1 = f32::from_bits(0x7FC0_0001);
    let n2 = f32::from_bits(0xFFC0_0002);
    let n3 = f32::from_bits(0x7FC0_0003);
    let n4 = f32::from_bits(0xFFC0_0004);

    // c2Sub: subss(dst = a.x, src = b.x) -> a's NaN wins in both lanes
    let sub = vbits((c().c2Sub)(C2v { x: n1, y: n3 }, C2v { x: n2, y: n4 }));
    assert_eq!(sub, (0x7FC0_0001, 0x7FC0_0003), "c2Sub must keep `a`'s NaN");

    // c2V: pure copy, no arithmetic, so even SNaN survives un-quieted
    let v = vbits((c().c2V)(f32::from_bits(0x7F80_0001), -0.0));
    assert_eq!(v, (0x7F80_0001, 0x8000_0000));

    // and the Rust agrees, through the .so
    diff(
        || "c2Sub NaN order".to_string(),
        |api| vbits((api.c2Sub)(C2v { x: n1, y: n3 }, C2v { x: n2, y: n4 })),
    );
    diff(
        || "c2V SNaN passthrough".to_string(),
        |api| vbits((api.c2V)(f32::from_bits(0x7F80_0001), -0.0)),
    );
}

/// `c2Maxv`/`c2Minv` do no arithmetic at all — they `movss` one operand or the
/// other — so their NaN behaviour is fixed by the C ternary's semantics
/// (`unordered ⇒ false ⇒ b`) at every optimisation level.
#[test]
fn ternary_minmax_semantics_are_opt_level_independent() {
    let na = f32::from_bits(0x7F80_0001); // SNaN, must NOT be quieted
    let nb = f32::from_bits(0xFFC0_ABCD);
    for (a, b) in [
        (C2v { x: na, y: nb }, C2v { x: nb, y: na }),
        (C2v { x: 1.0, y: na }, C2v { x: nb, y: 2.0 }),
    ] {
        let mx = diff(
            || format!("c2Maxv({a:?}, {b:?})"),
            |api| vbits((api.c2Maxv)(a, b)),
        );
        let mn = diff(
            || format!("c2Minv({a:?}, {b:?})"),
            |api| vbits((api.c2Minv)(a, b)),
        );
        if a.x.is_nan() || b.x.is_nan() {
            assert_eq!(mx.0, b.x.to_bits(), "max: unordered must select b.x verbatim");
            assert_eq!(mn.0, b.x.to_bits(), "min: unordered must select b.x verbatim");
        }
    }
}
