//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! The C library has no error-return surface (its only `return` is an
//! unconditional `return 0`), so these tests assert that Rust reproduces the
//! *same* non-rejection: identical `0` return AND identical mutated state, for
//! every degenerate / out-of-range / one-past-the-end input the C accepts.

mod common;
use common::*;

const SEED: u64 = 0xE770_0000_D1FF_0001;

// ---------------------------------------------------------------- E1
/// The sole `return` is unconditional `return 0`; nothing can change it.
#[test]
fn e1_return_value_is_always_zero() {
    let p = pair();
    let rng = Rng::new(SEED ^ 1);
    for _ in 0..100_000 {
        let s = Bw::from_bytes(rng.bytes32());
        let bits = rng.interesting_bits();
        let val = rng.interesting_u64();
        let (rc, _) = p.c.add(s, bits, val);
        let (rr, _) = p.rust.add(s, bits, val);
        assert_eq!(rc, 0, "C returned {rc} for bits={bits} val=0x{val:016x}");
        assert_eq!(rr, 0, "Rust returned {rr} for bits={bits} val=0x{val:016x}");
        assert_same(p, "E1", s, bits, val);
    }
}

// ---------------------------------------------------------------- E2
/// `bits == 0`: `val <<= (64 - 0)`, an out-of-range shift count.
#[test]
fn e2_zero_bits() {
    let p = pair();
    let rng = Rng::new(SEED ^ 2);
    for bb in 0u32..=64 {
        for _ in 0..64 {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "E2 bits=0", s, 0, rng.interesting_u64());
        }
    }
    for _ in 0..2000 {
        let s = Bw::from_bytes(rng.bytes32());
        assert_same(p, "E2 bits=0 random state", s, 0, rng.interesting_u64());
    }
    // zero bits must never change `tot`
    let s = Bw::zeroed();
    let (_, a) = p.c.add(s, 0, u64::MAX);
    let (_, b) = p.rust.add(s, 0, u64::MAX);
    assert_eq!(a.tot(), 0);
    assert_eq!(b.tot(), 0);
}

// ---------------------------------------------------------------- E3
/// `bits == 64`: exactly the word width, the largest sane value.
#[test]
fn e3_bits_equals_word_width() {
    let p = pair();
    let rng = Rng::new(SEED ^ 3);
    for bb in 0u32..=64 {
        for _ in 0..64 {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "E3 bits=64", s, 64, rng.interesting_u64());
        }
    }
    for _ in 0..2000 {
        let s = Bw::from_bytes(rng.bytes32());
        assert_same(p, "E3 bits=64 random state", s, 64, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- E4
/// `bits == 65`: exactly one step past the valid `[0, 64]` range.
#[test]
fn e4_bits_one_past_word_width() {
    let p = pair();
    let rng = Rng::new(SEED ^ 4);
    for bb in 0u32..=70 {
        for _ in 0..64 {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "E4 bits=65", s, 65, rng.interesting_u64());
        }
    }
    for _ in 0..2000 {
        let s = Bw::from_bytes(rng.bytes32());
        assert_same(p, "E4 bits=65 random state", s, 65, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- E5
/// Oversized `bits`: no upper bound is ever checked.
#[test]
fn e5_oversized_bits() {
    let p = pair();
    let rng = Rng::new(SEED ^ 5);
    const BITS: &[u32] = &[
        66,
        100,
        127,
        128,
        255,
        256,
        1000,
        0x1_0000,
        0x0FFF_FFFF,
        0x1000_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ];
    for &bits in BITS {
        for bb in [0u32, 1, 31, 32, 62, 63, 64, 65, 100, 0xFFFF_FFFF] {
            for _ in 0..48 {
                let s = state_with(&rng, bb, rng.interesting_u64());
                assert_same(p, "E5 oversized bits", s, bits, rng.interesting_u64());
            }
        }
        for _ in 0..200 {
            let s = Bw::from_bytes(rng.bytes32());
            assert_same(p, "E5 oversized bits random state", s, bits, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- E6
/// `bw->bits == 63` with `bits >= 1`: `b == 0`, so only the `i < 100` cap
/// terminates the loop.
#[test]
fn e6_b_zero_hits_iteration_cap() {
    let p = pair();
    let rng = Rng::new(SEED ^ 6);
    for bits in 1u32..=130 {
        for _ in 0..24 {
            let s = state_with(&rng, 63, rng.interesting_u64());
            assert_same(p, "E6 bw.bits=63 b==0", s, bits, rng.interesting_u64());
        }
    }
    for &bits in &[0x8000_0000u32, 0xFFFF_FFFF, 1000, 0x1_0000] {
        for _ in 0..200 {
            let s = state_with(&rng, 63, rng.interesting_u64());
            assert_same(p, "E6 bw.bits=63 huge bits", s, bits, rng.interesting_u64());
        }
    }
    // The cap really is what stops it: bw.bits stays 63, and bits is unchanged
    // by 100 iterations of `bits -= 0`, so the tail adds all of it.
    let mut s = Bw::zeroed();
    s.set_bits(63);
    let (_, a) = p.c.add(s, 5, 0xFFFF_FFFF_FFFF_FFFF);
    let (_, b) = p.rust.add(s, 5, 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(a, b, "E6 C={a:?} Rust={b:?}");
    assert_eq!(a.bits(), 68, "expected 63 + 5 after the capped loop");
}

// ---------------------------------------------------------------- E7
/// `bw->bits > 63` on entry: `b = (u32)(63 - bw->bits)` underflows.
#[test]
fn e7_out_of_range_bw_bits() {
    let p = pair();
    let rng = Rng::new(SEED ^ 7);
    const BB: &[u32] = &[
        64,
        65,
        66,
        70,
        99,
        100,
        127,
        128,
        255,
        256,
        0xFFFF,
        0x1_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ];
    for &bb in BB {
        for bits in 0u32..=68 {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "E7 bw.bits>63", s, bits, rng.interesting_u64());
        }
        for _ in 0..400 {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "E7 bw.bits>63 random bits", s, rng.interesting_bits(), rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- E8
/// `bw->bits + bits` wraps `u32`, so the loop guard is false despite both
/// operands being huge.
#[test]
fn e8_loop_condition_u32_wrap() {
    let p = pair();
    let rng = Rng::new(SEED ^ 8);
    let mut checked_wrap = 0usize;
    for &bb in &[0xFFFF_FFFFu32, 0xFFFF_FFFE, 0xFFFF_F000, 0x8000_0000] {
        let deficit = 0u32.wrapping_sub(bb);
        for k in 0u32..64 {
            let bits = deficit.wrapping_add(k);
            assert!(bb.wrapping_add(bits) < 64, "test setup: sum must wrap below 64");
            checked_wrap += 1;
            for _ in 0..24 {
                let s = state_with(&rng, bb, rng.interesting_u64());
                assert_same(p, "E8 u32 wrap skips loop", s, bits, rng.interesting_u64());
            }
            // Confirm the loop truly did not run: with b never applied, the
            // observable effect is a single OR + a single bits += bits.
            let mut s = Bw::zeroed();
            s.set_bits(bb);
            let (_, a) = p.c.add(s, bits, u64::MAX);
            let (_, b) = p.rust.add(s, bits, u64::MAX);
            assert_eq!(a, b, "E8 C={a:?} Rust={b:?}");
            assert_eq!(a.bits(), bb.wrapping_add(bits));
        }
    }
    assert_eq!(checked_wrap, 4 * 64);
}

// ---------------------------------------------------------------- E9
/// `bw->tot += bits` unsigned wrap-around, unchecked.
#[test]
fn e9_tot_wraps() {
    let p = pair();
    let rng = Rng::new(SEED ^ 9);
    for &tot in &[0xFFFF_FFFFu32, 0xFFFF_FFFE, 0xFFFF_FF00, 0x8000_0000, 0x7FFF_FFFF] {
        for &bits in &[0u32, 1, 2, 64, 65, 100, 0x8000_0000, 0xFFFF_FFFF] {
            for _ in 0..64 {
                let mut s = state_with(&rng, rng.interesting_bits(), rng.interesting_u64());
                s.set_tot(tot);
                assert_same(p, "E9 tot wrap", s, bits, rng.interesting_u64());
                let (_, a) = p.c.add(s, bits, 0);
                assert_eq!(a.tot(), tot.wrapping_add(bits), "C tot is not a plain u32 wrap");
            }
        }
    }
}

// ---------------------------------------------------------------- E10
/// `bw->bits` addition overflow, unchecked.
#[test]
fn e10_bw_bits_wraps() {
    let p = pair();
    let rng = Rng::new(SEED ^ 10);
    for &bb in &[0xFFFF_FFFFu32, 0xFFFF_FFFE, 0xFFFF_FF00, 0xFFFF_0000] {
        for &bits in &[0u32, 1, 2, 63, 64, 65, 100, 0x8000_0000, 0xFFFF_FFFF] {
            for _ in 0..64 {
                let s = state_with(&rng, bb, rng.interesting_u64());
                assert_same(p, "E10 bw.bits wrap", s, bits, rng.interesting_u64());
            }
        }
    }
    // Sweep values that land exactly on / just past the u32 boundary.
    for k in 0u32..200 {
        let bb = 0xFFFF_FFFFu32 - k;
        for bits in [k, k + 1, k + 2, 64, 65] {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "E10 boundary sweep", s, bits, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- E11
/// `bits` is a bare `tflac_u32` across the FFI boundary — there is no valid
/// variant set, so every one of the 2^32 values is a real input. Sweep the
/// whole low range plus every bit-boundary neighbourhood.
#[test]
fn e11_exhaustive_bits_sweep() {
    let p = pair();
    let rng = Rng::new(SEED ^ 11);

    // Contiguous sweep of the low range against several incoming states.
    for bits in 0u32..=512 {
        for &bb in &[0u32, 1, 32, 62, 63, 64, 65, 127, 0xFFFF_FFFF] {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "E11 low sweep", s, bits, rng.interesting_u64());
        }
    }

    // Every power-of-two boundary and its neighbours.
    for shift in 0u32..32 {
        let base = 1u32 << shift;
        for bits in [base.wrapping_sub(1), base, base.wrapping_add(1)] {
            for &bb in &[0u32, 63, 64, 0xFFFF_FFFF] {
                for _ in 0..8 {
                    let s = state_with(&rng, bb, rng.interesting_u64());
                    assert_same(p, "E11 pow2 boundary", s, bits, rng.interesting_u64());
                }
            }
        }
    }

    // Every multiple of 64 (and +/-1) up to 64*64 — the natural period of the
    // `& 63` shift masking.
    for m in 0u32..=64 {
        let base = m * 64;
        for bits in [base.saturating_sub(1), base, base + 1] {
            for &bb in &[0u32, 63, 64, 65] {
                for _ in 0..8 {
                    let s = state_with(&rng, bb, rng.interesting_u64());
                    assert_same(p, "E11 mod-64 boundary", s, bits, rng.interesting_u64());
                }
            }
        }
    }
}

// ---------------------------------------------------------------- E12
/// `buffer` null/dangling and `pos`/`len` inconsistent: never read, never
/// written, no check exists.
#[test]
fn e12_buffer_pos_len_untouched() {
    let p = pair();
    let rng = Rng::new(SEED ^ 12);
    let cases: [(u32, u32, u64); 6] = [
        (0, 0, 0),                       // null buffer, empty
        (u32::MAX, 0, 0),                // pos > len, null buffer
        (u32::MAX, u32::MAX, u64::MAX),  // saturated, bogus non-null buffer
        (0, u32::MAX, 0xDEAD_BEEF),      // bogus non-null buffer
        (12345, 7, 0x1),                 // pos > len, unaligned bogus pointer
        (7, 12345, 0xFFFF_FFFF_FFFF_FFF8),
    ];
    for (pos, len, buffer) in cases {
        for _ in 0..600 {
            let mut s = Bw::from_bytes(rng.bytes32());
            s.set_pos(pos);
            s.set_len(len);
            s.set_buffer(buffer);
            s.set_bits(rng.interesting_bits());
            let bits = rng.interesting_bits();
            let val = rng.interesting_u64();
            assert_same(p, "E12 buffer/pos/len", s, bits, val);
            let (_, a) = p.c.add(s, bits, val);
            assert_eq!((a.pos(), a.len(), a.buffer()), (pos, len, buffer));
            let (_, b) = p.rust.add(s, bits, val);
            assert_eq!((b.pos(), b.len(), b.buffer()), (pos, len, buffer));
        }
    }
}

// ---------------------------------------------------------------- E13
/// `bw == NULL`: dereferenced with no null check, so BOTH libraries fault.
/// Asserted by calling each in a child process and comparing the outcome.
#[test]
fn e13_null_bw_documented_only() {
    // Child mode: perform the null call for one implementation and let it fault.
    if let Ok(which) = std::env::var("E13_NULL_CALL") {
        let p = pair();
        let imp = if which == "c" { &p.c } else { &p.rust };
        let r = imp.add_raw_null(1, 0);
        println!("{} unexpectedly returned {r}", imp.name);
        std::process::exit(0);
    }

    let exe = std::env::current_exe().unwrap();
    let mut outcomes = Vec::new();
    for which in ["c", "rust"] {
        let out = std::process::Command::new(&exe)
            .args(["e13_null_bw_documented_only", "--exact", "--nocapture"])
            .env("E13_NULL_CALL", which)
            .output()
            .expect("spawn child");
        use std::os::unix::process::ExitStatusExt;
        let o = (out.status.code(), out.status.signal());
        println!("{which}: code={:?} signal={:?}", o.0, o.1);
        outcomes.push(o);
    }
    assert_eq!(
        outcomes[0], outcomes[1],
        "C and Rust disagree on the NULL-pointer outcome: C={:?} Rust={:?}",
        outcomes[0], outcomes[1]
    );
    // Neither may quietly succeed: the C has no null check, so it must fault.
    assert!(
        outcomes[0].1.is_some(),
        "expected both to terminate by signal (no null check exists in the C), got {:?}",
        outcomes[0]
    );
}
