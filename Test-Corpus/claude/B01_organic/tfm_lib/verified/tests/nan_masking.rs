//! Structural facts about the C reference that explain why four of the
//! operand-order mutants in `mutation_check.sh` are *semantically equivalent*
//! rather than uncaught bugs.
//!
//! Each fact is asserted against the **C `.so`** (the ground truth), and the
//! Rust `.so` is then required to agree bit-for-bit.

mod common;
use common::*;

/// If either `src[0]` or `src[1]` is NaN, `comiss` is unordered, so the C takes
/// the `else` arm, where:
///
/// * `dest[0] = dxy`   — a plain `movss` of `src[2]`, so **bit-identical**
///   to the input, with no quieting;
/// * `dest[1] = dx2 - lambda` — a `subss` whose *destination* operand is
///   `dx2 = src[1]`.
///
/// Hence when `src[1]` is NaN, `dest[1] == src[1] | 0x00400000` **regardless of
/// `lambda`**: every intermediate NaN payload inside `lambda` is masked out.
/// That is why commuting `dy2 + dx2`, `0.5f * (...)` or `2.0f*dx2*dy2` cannot be
/// observed.
#[test]
fn src1_nan_masks_every_intermediate_payload() {
    let im = impls();
    let c = im.c();
    let r = im.rust();

    let nans: Vec<u32> = {
        let mut v = vec![
            0x7fc0_0000u32,
            0xffc0_0000,
            0x7fc0_0001,
            0xffff_ffff,
            0x7f80_0001,
            0xffbf_ffff,
            0x7fab_cdef,
            0xff87_6543,
            0x7fff_ffff,
            0xff80_0001,
            0x7fbf_ffff,
            0xffc0_dead,
        ];
        let mut rng = Rng::new(SEED ^ 0x2001);
        for _ in 0..64 {
            let sign = (rng.next_u64() & 1) as u32;
            let payload = (rng.next_u32() & 0x007f_ffff).max(1);
            v.push((sign << 31) | 0x7f80_0000 | payload);
        }
        v
    };

    let mut rng = Rng::new(SEED ^ 0x2002);
    let mut checked = 0usize;
    for &s1 in &nans {
        // Anything at all in slot 0 and slot 2.
        let mut slot0: Vec<u32> = SPECIALS.to_vec();
        slot0.extend(nans.iter().copied());
        for &s0 in &slot0 {
            for &s2 in SPECIALS {
                let src = [s0, s1, s2];
                let mut dc = [0u32; 2];
                let mut dr = [0u32; 2];
                unsafe {
                    c(dc.as_mut_ptr(), src.as_ptr(), 1);
                    r(dr.as_mut_ptr(), src.as_ptr(), 1);
                }
                assert_eq!(dc, dr, "C/Rust divergence for src = {src:#010x?}");
                assert_eq!(
                    dc[0], s2,
                    "dest[0] must be a verbatim copy of src[2] (movss, no quieting), \
                     src = {src:#010x?}"
                );
                assert_eq!(
                    dc[1],
                    s1 | 0x0040_0000,
                    "dest[1] must be src[1] quieted, independent of lambda, \
                     src = {src:#010x?}"
                );
                checked += 1;
            }
        }
        for _ in 0..64 {
            let src = [rng.next_u32(), s1, rng.next_u32()];
            let mut dc = [0u32; 2];
            let mut dr = [0u32; 2];
            unsafe {
                c(dc.as_mut_ptr(), src.as_ptr(), 1);
                r(dr.as_mut_ptr(), src.as_ptr(), 1);
            }
            assert_eq!(dc, dr, "C/Rust divergence for src = {src:#010x?}");
            assert_eq!(dc[0], src[2]);
            assert_eq!(dc[1], s1 | 0x0040_0000);
            checked += 1;
        }
    }
    println!("src1_nan_masking: {checked} cases");
    assert!(checked > 10_000);
}

/// In the `if` arm the guard `src[0] < src[1]` succeeded, so **neither** `dx2`
/// nor `dy2` can be NaN. Every SSE operand-order choice involving only
/// `dx2`/`dy2` is therefore payload-irrelevant on that arm.
#[test]
fn if_arm_never_sees_nan_in_dx2_or_dy2() {
    let mut rng = Rng::new(SEED ^ 0x2003);
    let mut n = 0usize;
    for _ in 0..500_000 {
        let (a, b, c) = rng.candidate();
        let t = trace(a, b, c);
        if t.arm_if {
            assert!(
                !t.dx2.is_nan() && !t.dy2.is_nan(),
                "if arm reached with a NaN operand: {a:#010x} {b:#010x} {c:#010x}"
            );
            n += 1;
        }
    }
    for &a in SPECIALS {
        for &b in SPECIALS {
            let t = trace(a, b, 0);
            if t.arm_if {
                assert!(!t.dx2.is_nan() && !t.dy2.is_nan());
                n += 1;
            }
        }
    }
    assert!(n > 1000, "only {n} if-arm samples");
}

/// `2.0f * dx2` (a `mulss` against the constant `2.0f`) and `dx2 + dx2` (the
/// `addss %xmm0, %xmm0` the reference build actually emits) are
/// indistinguishable: the values agree exactly (scaling by two is exact, and
/// both overflow to the same infinity) and, since `2.0f` is never NaN, the NaN
/// result of both is `quiet(dx2)`.
#[test]
fn two_times_x_equals_x_plus_x_including_payloads() {
    let mut rng = Rng::new(SEED ^ 0x2004);
    for _ in 0..2_000_000 {
        let b = rng.next_u32();
        let x = f32::from_bits(b);
        let a = 2.0f32 * x;
        let s = x + x;
        assert_eq!(
            a.to_bits(),
            s.to_bits(),
            "2.0*x != x+x for {b:#010x}"
        );
    }
    for &b in SPECIALS {
        let x = f32::from_bits(b);
        assert_eq!((2.0f32 * x).to_bits(), (x + x).to_bits());
    }
}

/// `sqrtf`'s result is consumed by exactly one operation,
/// `fadd(fadd(dy2, dx2), root)`, whose destination operand is the *sum*. So if
/// `root` is NaN the `addss` either discards it (sum already NaN) or returns
/// `quiet(root)`. Since `quiet` is idempotent, quieting `root` inside `fsqrt`
/// is unobservable — which is why "sqrt does not quiet a NaN operand" is an
/// equivalent mutant.
///
/// Asserted here against the C `.so`: the **computed** output slot
/// (`dx2 - lambda`) is *always* a quiet NaN whenever it is NaN, i.e. the
/// downstream `addss`/`mulss`/`subss` chain always applies the quiet bit. Only
/// the verbatim `dxy` slot can carry a signaling NaN through.
#[test]
fn computed_output_slot_is_always_quiet_when_nan() {
    let im = impls();
    let c = im.c();
    let r = im.rust();

    // Idempotence of quieting, which the argument above relies on.
    for k in 0..1000u32 {
        let x = 0x7f80_0000 | (k * 0x0000_2003) & 0x007f_ffff | 1;
        assert_eq!((x | 0x0040_0000) | 0x0040_0000, x | 0x0040_0000);
    }

    let mut rng = Rng::new(SEED ^ 0x2006);
    let mut nan_computed = 0usize;
    let mut snan_dxy = 0usize;
    let mut iter = 0usize;
    while iter < 400_000 {
        let (s0, s1, s2) = if iter % 3 == 0 {
            rng.candidate()
        } else {
            (rng.pool_f32(), rng.pool_f32(), rng.pool_f32())
        };
        iter += 1;
        let src = [s0, s1, s2];
        let mut dc = [0u32; 2];
        let mut dr = [0u32; 2];
        unsafe {
            c(dc.as_mut_ptr(), src.as_ptr(), 1);
            r(dr.as_mut_ptr(), src.as_ptr(), 1);
        }
        assert_eq!(dc, dr, "C/Rust divergence for src = {src:#010x?}");

        let arm_if = trace(s0, s1, s2).arm_if;
        // if arm  -> dest[0] computed, dest[1] = dxy verbatim
        // else arm-> dest[0] = dxy verbatim, dest[1] computed
        let (computed, verbatim) = if arm_if {
            (dc[0], dc[1])
        } else {
            (dc[1], dc[0])
        };
        assert_eq!(verbatim, s2, "the dxy slot must be a verbatim copy of src[2]");
        let is_nan = |b: u32| (b & 0x7f80_0000) == 0x7f80_0000 && (b & 0x007f_ffff) != 0;
        if is_nan(computed) {
            assert_ne!(
                computed & 0x0040_0000,
                0,
                "the computed slot produced a SIGNALING NaN {computed:#010x} for \
                 src = {src:#010x?}"
            );
            nan_computed += 1;
        }
        if is_nan(verbatim) && verbatim & 0x0040_0000 == 0 {
            snan_dxy += 1;
        }
    }
    println!(
        "computed-slot NaNs: {nan_computed}, signaling NaNs passed through the dxy \
         slot: {snan_dxy}"
    );
    assert!(nan_computed > 10_000, "not enough NaN outputs observed");
    assert!(snan_dxy > 0, "no signaling NaN ever reached the verbatim slot");
}

/// The clamp constant's *sign* is unobservable: `sqrtf(±0.0) == ±0.0` and the
/// only way that sign could survive `(dy2 + dx2) + root` is `dy2 + dx2 == -0.0`,
/// which requires `dy2 == dx2 == -0.0` — and that makes `sqd == +0.0`, so the
/// clamp is not taken. Hence "clamp target becomes -0.0f" is an equivalent
/// mutant. Verified by search here.
#[test]
fn clamp_zero_sign_is_unobservable() {
    assert_unreachable(
        "clamp taken AND (dy2 + dx2) == -0.0",
        SEED ^ 0x2007,
        400_000,
        |t| t.sqd < 0.0 && (t.dy2 + t.dx2).to_bits() == 0x8000_0000,
    );
    // Both halves of the argument, separately.
    let mut rng = Rng::new(SEED ^ 0x2008);
    let mut sum_negzero = 0usize;
    for _ in 0..400_000 {
        let (a, b, c) = rng.candidate();
        let t = trace(a, b, c);
        if (t.dy2 + t.dx2).to_bits() == 0x8000_0000 {
            assert_eq!(
                t.dy2.to_bits(),
                0x8000_0000,
                "dy2 + dx2 == -0.0 requires dy2 == -0.0"
            );
            assert_eq!(t.dx2.to_bits(), 0x8000_0000);
            // With dy2 == dx2 == -0.0 the first three terms cancel to +0.0, so
            // sqd == term4, which is never negative.
            assert!(
                !(t.sqd < 0.0),
                "with dy2 == dx2 == -0.0, sqd must not be negative (got {:e})",
                t.sqd
            );
            sum_negzero += 1;
        }
    }
    // Deterministic witness so the branch is definitely exercised.
    let t = trace(0x8000_0000, 0x8000_0000, 0x8000_0000);
    assert_eq!((t.dy2 + t.dx2).to_bits(), 0x8000_0000);
    assert_eq!(t.sqd.to_bits(), 0);
    assert!(sum_negzero > 0, "the -0.0 sum branch was never exercised");
    diff1("clamp-sign witness", 0x8000_0000, 0x8000_0000, 0x8000_0000);
    println!("clamp_zero_sign: {sum_negzero} random cases with dy2 + dx2 == -0.0");
}

/// Rust's `f32 <` is an ordered comparison (false for NaN), exactly like
/// `comiss` + `jbe`, so the explicit NaN guards in `flt` are belt-and-braces.
#[test]
fn rust_lt_is_ordered_like_comiss() {
    let mut rng = Rng::new(SEED ^ 0x2005);
    for _ in 0..1_000_000 {
        let (a, b) = (rng.next_u32(), rng.next_u32());
        let (x, y) = (f32::from_bits(a), f32::from_bits(b));
        if x.is_nan() || y.is_nan() {
            assert!(!(x < y), "NaN compare must be false: {a:#010x} {b:#010x}");
        }
    }
}
