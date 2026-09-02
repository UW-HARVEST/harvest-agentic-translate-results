//! Negative controls — proof that the differential suite can actually DETECT a
//! divergence.
//!
//! A green Phase B/C run is only meaningful if the harness and the input
//! generators are sensitive enough to catch a wrong translation. Each mutant
//! below is a *plausible* mistranslation of `src/lib.c`; the test asserts that
//! the same generators used in Phases B and C flag it as different from the C
//! `.so`. If any mutant were to survive, the corresponding real bug could also
//! survive, and that row's coverage would be worthless.

mod common;
use common::*;

/// The mistranslations we guard against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mutant {
    /// Faithful reference — must NOT diverge. Sanity anchor for the others.
    Faithful,
    /// Forget `bw->val &= mask;` inside the loop.
    NoMask,
    /// Compute the loop guard in 64-bit instead of letting `bw->bits + bits`
    /// wrap as `u32` (the `cmp $0x3f` the C compiler emits is 32-bit).
    LoopGuardIn64Bit,
    /// Clamp `b` at 0 instead of letting `63 - bw->bits` underflow as `u32`.
    ClampBInsteadOfUnderflow,
    /// Treat an out-of-range shift count as producing 0 (a common "safe" fix)
    /// instead of reproducing the hardware's mask-to-6-bits.
    CheckedShiftYieldsZero,
    /// Saturate `bw->tot += bits` instead of wrapping.
    SaturatingTot,
    /// Saturate `bw->bits += bits` instead of wrapping.
    SaturatingBwBits,
    /// Use a signed comparison for `b > bits ? bits : b`.
    SignedTernary,
    /// Skip the loop entirely (cap 0). This one IS observable and must be
    /// detected: it proves the loop body is genuinely exercised.
    IterationCapZero,
    /// Caps that are provably UNOBSERVABLE for this function (see
    /// `n4_iteration_cap_at_least_one_is_unobservable`): every iteration after
    /// the first is a no-op, so any cap >= 1 gives identical results. These are
    /// equivalent mutants, not coverage gaps, so they are asserted to be
    /// equivalent rather than required to be detected.
    IterationCapOne,
    IterationCapOneOhOne,
    IterationCapTenThousand,
    /// Apply the final `bw->val |= val >> bw->bits` before `bw->bits += bits`
    /// in the wrong order (use the updated `bits`).
    WrongTailOrder,
}

const TFLAC_UINT_BITS: u64 = 64;
const MASK: u64 = 18446744073709551615u64 << 1;

fn shl(m: Mutant, v: u64, count: u64) -> u64 {
    if m == Mutant::CheckedShiftYieldsZero {
        if count >= 64 { 0 } else { v << count }
    } else {
        v << (count & 63)
    }
}
fn shr(m: Mutant, v: u64, count: u64) -> u64 {
    if m == Mutant::CheckedShiftYieldsZero {
        if count >= 64 { 0 } else { v >> count }
    } else {
        v >> (count & 63)
    }
}

/// A Rust model of `bitwriter_add` with one mutation applied.
fn model(m: Mutant, state: Bw, bits_in: u32, val_in: u64) -> (i32, Bw) {
    let mut s = state;
    let mut bits = bits_in;
    let mut val = val_in;

    let mut bw_val = s.val();
    let mut bw_bits = s.bits();
    let mut bw_tot = s.tot();

    val = shl(m, val, TFLAC_UINT_BITS.wrapping_sub(bits as u64));

    bw_tot = if m == Mutant::SaturatingTot {
        bw_tot.saturating_add(bits)
    } else {
        bw_tot.wrapping_add(bits)
    };

    let cap: i32 = match m {
        Mutant::IterationCapZero => 0,
        Mutant::IterationCapOne => 1,
        Mutant::IterationCapOneOhOne => 101,
        Mutant::IterationCapTenThousand => 10_000,
        _ => 100,
    };
    let mut i: i32 = 0;
    loop {
        let guard = if m == Mutant::LoopGuardIn64Bit {
            (bw_bits as u64 + bits as u64) >= TFLAC_UINT_BITS
        } else {
            (bw_bits.wrapping_add(bits) as u64) >= TFLAC_UINT_BITS
        };
        if !(guard && i < cap) {
            break;
        }

        let mut b: u32 = if m == Mutant::ClampBInsteadOfUnderflow {
            if bw_bits >= 63 { 0 } else { 63 - bw_bits }
        } else {
            TFLAC_UINT_BITS
                .wrapping_sub(bw_bits as u64)
                .wrapping_sub(1) as u32
        };

        b = if m == Mutant::SignedTernary {
            if (b as i32) > (bits as i32) { bits } else { b }
        } else if b > bits {
            bits
        } else {
            b
        };

        bw_val |= shr(m, val, bw_bits as u64);
        bw_bits = bw_bits.wrapping_add(b);
        if m != Mutant::NoMask {
            bw_val &= MASK;
        }
        val = shl(m, val, b as u64);
        bits = bits.wrapping_sub(b);
        i = i.wrapping_add(1);
    }

    if m == Mutant::WrongTailOrder {
        bw_bits = bw_bits.wrapping_add(bits);
        bw_val |= shr(m, val, bw_bits as u64);
    } else {
        bw_val |= shr(m, val, bw_bits as u64);
        bw_bits = if m == Mutant::SaturatingBwBits {
            bw_bits.saturating_add(bits)
        } else {
            bw_bits.wrapping_add(bits)
        };
    }

    s.set_val(bw_val);
    s.set_bits(bw_bits);
    s.set_tot(bw_tot);
    (0, s)
}

/// The exact same input distribution Phases B and C draw from.
fn generated_inputs(n: usize, seed: u64) -> Vec<(Bw, u32, u64)> {
    let rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    // structured rows (mirrors CONFIGS.md sweeps)
    for &bb in &[0u32, 1, 31, 32, 62, 63, 64, 65, 100, 0xFFFF_FFFF] {
        for &bits in &[0u32, 1, 2, 32, 63, 64, 65, 100, 0x8000_0000, 0xFFFF_FFFF] {
            for &val in &[0u64, 1, u64::MAX, 0xAAAA_AAAA_AAAA_AAAA] {
                let mut s = state_with(&rng, bb, rng.interesting_u64());
                s.set_tot(rng.next_u32());
                out.push((s, bits, val));
            }
        }
    }
    // saturated-tot / saturated-bits rows
    for &bb in &[0xFFFF_FFFFu32, 0xFFFF_FF00] {
        for &bits in &[1u32, 64, 0xFFFF_FFFF] {
            let mut s = state_with(&rng, bb, 0);
            s.set_tot(0xFFFF_FFFF);
            out.push((s, bits, u64::MAX));
        }
    }
    // random fuzz rows
    while out.len() < n {
        out.push((
            Bw::from_bytes(rng.bytes32()),
            rng.interesting_bits(),
            rng.interesting_u64(),
        ));
    }
    out
}

#[test]
fn n1_harness_loads_two_distinct_shared_objects() {
    let p = pair();
    assert_ne!(p.c.path, p.rust.path, "harness loaded the same .so twice");
    println!("C    .so: {}", p.c.path.display());
    println!("Rust .so: {}", p.rust.path.display());
    assert!(p.c.path.exists() && p.rust.path.exists());
    // Both must independently answer a trivial call.
    let mut s = Bw::zeroed();
    s.set_bits(3);
    assert_eq!(p.c.add(s, 4, 0xA).0, 0);
    assert_eq!(p.rust.add(s, 4, 0xA).0, 0);
}

#[test]
fn n2_faithful_model_agrees_with_c() {
    // Anchors the mutation test: the unmutated model matches the C .so, so a
    // detected divergence below is attributable to the mutation alone.
    let p = pair();
    for (s, bits, val) in generated_inputs(50_000, 0xA11CE) {
        let (rc, sc) = p.c.add(s, bits, val);
        let (rm, sm) = model(Mutant::Faithful, s, bits, val);
        assert_eq!(
            (rc, sc),
            (rm, sm),
            "faithful model diverged from C: in={s:?} bits={bits} val=0x{val:016x}"
        );
    }
}

#[test]
fn n3_every_mutant_is_detected() {
    let p = pair();
    const MUTANTS: &[Mutant] = &[
        Mutant::NoMask,
        Mutant::LoopGuardIn64Bit,
        Mutant::ClampBInsteadOfUnderflow,
        Mutant::CheckedShiftYieldsZero,
        Mutant::SaturatingTot,
        Mutant::SaturatingBwBits,
        Mutant::SignedTernary,
        Mutant::IterationCapZero,
        Mutant::WrongTailOrder,
    ];
    let inputs = generated_inputs(50_000, 0xA11CE);
    let mut survivors = Vec::new();
    for &m in MUTANTS {
        let mut first: Option<(Bw, u32, u64)> = None;
        let mut hits = 0usize;
        for &(s, bits, val) in &inputs {
            let (rc, sc) = p.c.add(s, bits, val);
            let (rm, sm) = model(m, s, bits, val);
            if (rc, sc) != (rm, sm) {
                hits += 1;
                if first.is_none() {
                    first = Some((s, bits, val));
                }
            }
        }
        match first {
            Some((s, bits, val)) => println!(
                "{m:?}: DETECTED ({hits}/{} inputs); first at bits={bits} val=0x{val:016x} \
                 bw.bits={} bw.tot={}",
                inputs.len(),
                s.bits(),
                s.tot()
            ),
            None => survivors.push(m),
        }
    }
    assert!(
        survivors.is_empty(),
        "these mistranslations SURVIVED the test inputs, so the suite is blind to \
         that class of bug: {survivors:?}"
    );
}

/// Count the loop iterations the C algorithm performs, to prove the `i < 100`
/// cap is live code rather than dead defensive cruft.
fn loop_iterations(state: Bw, bits_in: u32) -> i32 {
    let mut bits = bits_in;
    let mut bw_bits = state.bits();
    let mut i: i32 = 0;
    while (bw_bits.wrapping_add(bits) as u64) >= 64 && i < 100 {
        let b0 = 64u64.wrapping_sub(bw_bits as u64).wrapping_sub(1) as u32;
        let b = if b0 > bits { bits } else { b0 };
        bw_bits = bw_bits.wrapping_add(b);
        bits = bits.wrapping_sub(b);
        i += 1;
    }
    i
}

#[test]
fn n4_iteration_cap_at_least_one_is_unobservable() {
    // Finding: for this function every loop iteration AFTER the first is a
    // no-op, so all caps >= 1 are behaviourally identical.
    //
    // Why: `b = (tflac_u32)(63 - bw->bits)`, so iteration 1 either
    //   (a) leaves `bw->bits == 63` exactly (unclamped case, incl. the
    //       `bw->bits > 63` case where the u32 underflow makes
    //       `bw->bits + b` wrap back to 63), or
    //   (b) clamps `b` to `bits`, driving `bits` to 0.
    // From iteration 2 on, `b` is therefore 0, so `bw->bits`, `bits` and `val`
    // all stop changing and the only new contribution is
    // `bw->val |= (val >> 63)` -- at most bit 0 -- which the very next
    // `bw->val &= mask` clears again. In case (b) the two shifts of `val`
    // sum to 64 (or to 0 when `bits % 64 == 0`), so the re-OR is either 0 or
    // exactly the value already OR'd in.
    //
    // This is asserted, not assumed: if the argument were wrong, this test
    // fails and the cap would need direct differential coverage.
    let p = pair();
    let inputs = generated_inputs(50_000, 0xA11CE);
    for m in [
        Mutant::IterationCapOne,
        Mutant::IterationCapOneOhOne,
        Mutant::IterationCapTenThousand,
    ] {
        for &(s, bits, val) in &inputs {
            let (rc, sc) = p.c.add(s, bits, val);
            let (rm, sm) = model(m, s, bits, val);
            assert_eq!(
                (rc, sc),
                (rm, sm),
                "{m:?} is NOT equivalent to the real cap for in={s:?} bits={bits} \
                 val=0x{val:016x} -- the equivalence argument is wrong and the cap \
                 needs its own differential coverage"
            );
        }
    }
}

#[test]
fn n5_iteration_cap_is_actually_reached() {
    // The 100-iteration cap must be live code, otherwise n4's equivalence
    // argument would be vacuous and IterationCapOne could not be detected.
    let p = pair();
    let mut s = Bw::zeroed();
    s.set_bits(63);
    assert_eq!(loop_iterations(s, 1), 100, "expected the cap to terminate the loop");

    // ... and multi-iteration (>1) runs must occur, which is what makes
    // IterationCapOne observable.
    let mut multi = 0usize;
    let mut capped = 0usize;
    let inputs = generated_inputs(50_000, 0xA11CE);
    for &(st, bits, _) in &inputs {
        let n = loop_iterations(st, bits);
        if n > 1 {
            multi += 1;
        }
        if n == 100 {
            capped += 1;
        }
    }
    println!("inputs with >1 loop iteration: {multi}; inputs hitting the 100 cap: {capped}");
    assert!(multi > 0, "no input ever ran the loop more than once");
    assert!(capped > 0, "no input ever reached the i<100 cap");

    // And the real Rust .so must agree with C on the capped case specifically.
    assert_same(p, "n5 capped case", s, 1, u64::MAX);
}
