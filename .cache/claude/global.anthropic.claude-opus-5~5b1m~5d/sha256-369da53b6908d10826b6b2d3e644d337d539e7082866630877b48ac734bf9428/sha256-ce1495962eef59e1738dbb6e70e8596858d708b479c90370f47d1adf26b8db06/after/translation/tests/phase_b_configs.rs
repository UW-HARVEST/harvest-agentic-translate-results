//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` via `libloading` and
//! compares the return value plus all six `tflac_bitwriter` fields byte-for-byte
//! over many randomized inputs (fixed seeds -> reproducible).

mod common;

use common::{load_pair, Bitwriter, Checker, Rng};

/// Junk values for the fields the C never touches (`pos`, `len`, `buffer`).
fn junk(r: &mut Rng) -> (u32, u32, usize) {
    (r.next_u32(), r.next_u32(), r.next_u64() as usize)
}

// ---------------------------------------------------------------------------
// C1 — loop never entered: bw->bits + bits < 64
// ---------------------------------------------------------------------------
#[test]
fn c1_loop_not_entered_fast_path() {
    let p = load_pair();
    let mut r = Rng::new(0xC001);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let bwbits = r.range(0, 62);
        let bits = r.range(0, 63 - bwbits);
        assert!(bwbits + bits < 64);
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("C1");
}

// ---------------------------------------------------------------------------
// C2 — loop entered and drains naturally (bw->bits <= 62)
// ---------------------------------------------------------------------------
#[test]
fn c2_loop_drains_naturally() {
    let p = load_pair();
    let mut r = Rng::new(0xC002);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let bwbits = r.range(0, 62);
        // sum >= 64 without overflowing u32
        let bits = r.range(64 - bwbits, 64 - bwbits + 4096);
        assert!(bwbits.wrapping_add(bits) >= 64);
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("C2");
}

// ---------------------------------------------------------------------------
// C3 — loop hits the `i < 100` cap via bw->bits == 63 (b == 0, no progress)
// ---------------------------------------------------------------------------
#[test]
fn c3_iteration_cap_via_bwbits_63() {
    let p = load_pair();
    let mut r = Rng::new(0xC003);
    let mut ck = Checker::new(&p);
    for _ in 0..10_000 {
        let bits = r.range(1, u32::MAX);
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), 63, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    // deterministic edge values of `bits`
    for bits in [1u32, 2, 63, 64, 65, 100, 101, 1000, u32::MAX] {
        let init = Bitwriter::new(0xDEAD_BEEF_CAFE_F00D, 63, 7, 9, 11, 0);
        ck.check(init, bits, 0xFFFF_FFFF_FFFF_FFFF);
    }
    ck.finish("C3");
}

// ---------------------------------------------------------------------------
// C4 — cap via bits == 0 while bw->bits >= 64 (b clamped to 0, no progress)
// ---------------------------------------------------------------------------
#[test]
fn c4_iteration_cap_via_bits_zero() {
    let p = load_pair();
    let mut r = Rng::new(0xC004);
    let mut ck = Checker::new(&p);
    for _ in 0..10_000 {
        let bwbits = r.range(64, u32::MAX);
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
        ck.check(init, 0, r.interesting_u64());
    }
    for bwbits in [64u32, 65, 100, 127, 128, 1000, 0x8000_0000, u32::MAX] {
        let init = Bitwriter::new(0x0123_4567_89AB_CDEF, bwbits, 1, 2, 3, 0);
        ck.check(init, 0, 0xA5A5_A5A5_A5A5_A5A5);
    }
    ck.finish("C4");
}

// ---------------------------------------------------------------------------
// C5 — 32-bit sum WRAPS below 64, so the loop is skipped despite huge operands
// ---------------------------------------------------------------------------
#[test]
fn c5_loop_condition_u32_wraps_below_64() {
    let p = load_pair();
    let mut r = Rng::new(0xC005);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let k = r.range(0, 1_000_000);
        let j = r.range(0, 63);
        let bwbits = u32::MAX - k;
        let bits = k + 1 + j; // bwbits + bits == 2^32 + j  ->  wraps to j (< 64)
        assert_eq!(bwbits.wrapping_add(bits), j);
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    // the canonical case: 0xFFFFFFFF + 1 == 0
    let init = Bitwriter::new(0x1234_5678_9ABC_DEF0, u32::MAX, 5, 6, 7, 0);
    ck.check(init, 1, u64::MAX);
    ck.finish("C5");
}

// ---------------------------------------------------------------------------
// C6 — ternary takes `bits` (b clamped): only reachable with bw->bits >= 64,
//      where `63 - bw->bits` wraps to a huge u32.
// ---------------------------------------------------------------------------
#[test]
fn c6_ternary_clamps_b_to_bits() {
    let p = load_pair();
    let mut r = Rng::new(0xC006);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let bwbits = r.range(64, 100_000);
        let bits = r.range(1, 4096); // b = 63-bwbits wraps huge  =>  b > bits
        assert!(63u32.wrapping_sub(bwbits) > bits);
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("C6");
}

// ---------------------------------------------------------------------------
// C7 — ternary keeps `b` (no clamp): bw->bits <= 62 with the loop entered
// ---------------------------------------------------------------------------
#[test]
fn c7_ternary_keeps_b() {
    let p = load_pair();
    let mut r = Rng::new(0xC007);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let bwbits = r.range(0, 62);
        let bits = r.range(64 - bwbits, 64 - bwbits + 512);
        assert!(63u32.wrapping_sub(bwbits) <= bits); // b kept
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("C7");
}

// ---------------------------------------------------------------------------
// C8 — bits == 0 (line-8 shift count is 64: out of range, mod-64 -> 0)
// ---------------------------------------------------------------------------
#[test]
fn c8_bits_zero() {
    let p = load_pair();
    let mut r = Rng::new(0xC008);
    let mut ck = Checker::new(&p);
    for bwbits in 0u32..=63 {
        for _ in 0..200 {
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
            ck.check(init, 0, r.interesting_u64());
        }
    }
    ck.finish("C8");
}

// ---------------------------------------------------------------------------
// C9 — bits == 64 (shift count exactly 0)
// ---------------------------------------------------------------------------
#[test]
fn c9_bits_eq_64() {
    let p = load_pair();
    let mut r = Rng::new(0xC009);
    let mut ck = Checker::new(&p);
    for bwbits in [0u32, 1, 32, 62, 63, 64, 65, 127, 128, u32::MAX] {
        for _ in 0..500 {
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
            ck.check(init, 64, r.interesting_u64());
        }
    }
    ck.finish("C9");
}

// ---------------------------------------------------------------------------
// C10 — bits in 1..=63 (the meaningful range), exhaustive over `bits`
// ---------------------------------------------------------------------------
#[test]
fn c10_bits_1_to_63_exhaustive() {
    let p = load_pair();
    let mut r = Rng::new(0xC010);
    let mut ck = Checker::new(&p);
    for bits in 1u32..=63 {
        for _ in 0..300 {
            let bwbits = r.range(0, 63);
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
            ck.check(init, bits, r.interesting_u64());
        }
    }
    ck.finish("C10");
}

// ---------------------------------------------------------------------------
// C11 — bits in 65..=127 (64 - bits wraps)
// ---------------------------------------------------------------------------
#[test]
fn c11_bits_65_to_127() {
    let p = load_pair();
    let mut r = Rng::new(0xC011);
    let mut ck = Checker::new(&p);
    for bits in 65u32..=127 {
        for _ in 0..300 {
            let bwbits = r.interesting_bits();
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
            ck.check(init, bits, r.interesting_u64());
        }
    }
    ck.finish("C11");
}

// ---------------------------------------------------------------------------
// C12 — oversized `bits`: >= 128, 0x80000000, u32::MAX
// ---------------------------------------------------------------------------
#[test]
fn c12_bits_oversized() {
    let p = load_pair();
    let mut r = Rng::new(0xC012);
    let mut ck = Checker::new(&p);
    let edges = [
        128u32, 129, 191, 192, 255, 256, 1000, 65535, 65536, 0x00FF_FFFF, 0x7FFF_FFFE,
        0x7FFF_FFFF, 0x8000_0000, 0x8000_0001, 0xFFFF_FFFD, 0xFFFF_FFFE, 0xFFFF_FFFF,
    ];
    for bits in edges {
        for _ in 0..400 {
            let bwbits = r.interesting_bits();
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
            ck.check(init, bits, r.interesting_u64());
        }
    }
    for _ in 0..5_000 {
        let bits = r.range(128, u32::MAX);
        let bwbits = r.interesting_bits();
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("C12");
}

// ---------------------------------------------------------------------------
// C13 — bw->bits == 0 (empty accumulator), `bits` swept 0..=64
// ---------------------------------------------------------------------------
#[test]
fn c13_bwbits_zero() {
    let p = load_pair();
    let mut r = Rng::new(0xC013);
    let mut ck = Checker::new(&p);
    for bits in 0u32..=64 {
        for _ in 0..300 {
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(r.interesting_u64(), 0, pos, len, r.next_u32(), buf);
            ck.check(init, bits, r.interesting_u64());
        }
    }
    ck.finish("C13");
}

// ---------------------------------------------------------------------------
// C14 — bw->bits in 1..=62 (partially filled), exhaustive over bw->bits
// ---------------------------------------------------------------------------
#[test]
fn c14_bwbits_1_to_62_exhaustive() {
    let p = load_pair();
    let mut r = Rng::new(0xC014);
    let mut ck = Checker::new(&p);
    for bwbits in 1u32..=62 {
        for _ in 0..400 {
            let bits = r.interesting_bits();
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
            ck.check(init, bits, r.interesting_u64());
        }
    }
    ck.finish("C14");
}

// ---------------------------------------------------------------------------
// C15 — bw->bits == 63, the stall value (bits == 0 skips, bits >= 1 stalls)
// ---------------------------------------------------------------------------
#[test]
fn c15_bwbits_63_stall_value() {
    let p = load_pair();
    let mut r = Rng::new(0xC015);
    let mut ck = Checker::new(&p);
    // bits == 0 -> 63 + 0 < 64 -> loop skipped entirely
    for _ in 0..2_000 {
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), 63, pos, len, r.next_u32(), buf);
        ck.check(init, 0, r.interesting_u64());
    }
    // bits == 1 -> 63 + 1 >= 64, b == 0 -> 100-iteration stall
    for _ in 0..2_000 {
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), 63, pos, len, r.next_u32(), buf);
        ck.check(init, 1, r.interesting_u64());
    }
    for _ in 0..6_000 {
        let bits = r.interesting_bits();
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), 63, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("C15");
}

// ---------------------------------------------------------------------------
// C16 — bw->bits >= 64: invalid internal state, out-of-range `>>` counts
// ---------------------------------------------------------------------------
#[test]
fn c16_bwbits_out_of_range() {
    let p = load_pair();
    let mut r = Rng::new(0xC016);
    let mut ck = Checker::new(&p);
    let edges = [
        64u32, 65, 66, 95, 96, 127, 128, 129, 191, 255, 256, 1000, 65536, 0x7FFF_FFFF,
        0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF,
    ];
    for bwbits in edges {
        for _ in 0..500 {
            let bits = r.interesting_bits();
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
            ck.check(init, bits, r.interesting_u64());
        }
    }
    for _ in 0..10_000 {
        let bwbits = r.range(64, u32::MAX);
        let bits = r.interesting_bits();
        let (pos, len, buf) = junk(&mut r);
        let init = Bitwriter::new(r.interesting_u64(), bwbits, pos, len, r.next_u32(), buf);
        ck.check(init, bits, r.interesting_u64());
    }
    ck.finish("C16");
}

// ---------------------------------------------------------------------------
// C17 — value shapes for `val` and `bw->val` (single bits, halves, extremes)
// ---------------------------------------------------------------------------
#[test]
fn c17_value_shapes() {
    let p = load_pair();
    let mut r = Rng::new(0xC017);
    let mut ck = Checker::new(&p);

    let mut shapes: Vec<u64> = vec![
        0,
        u64::MAX,
        1,
        0x8000_0000_0000_0000,
        0x0000_0000_FFFF_FFFF,
        0xFFFF_FFFF_0000_0000,
        0xAAAA_AAAA_AAAA_AAAA,
        0x5555_5555_5555_5555,
    ];
    for i in 0..64 {
        shapes.push(1u64 << i); // single bit at every position
    }

    for &val in &shapes {
        for &bwval in &shapes {
            let bits = r.interesting_bits();
            let bwbits = r.interesting_bits();
            let (pos, len, buf) = junk(&mut r);
            let init = Bitwriter::new(bwval, bwbits, pos, len, r.next_u32(), buf);
            ck.check(init, bits, val);
        }
    }
    // and again with in-range bits/bw->bits so the meaningful paths get every shape
    for &val in &shapes {
        for &bwval in &shapes {
            let bwbits = r.range(0, 63);
            let bits = r.range(0, 64);
            let init = Bitwriter::new(bwval, bwbits, 0, 0, 0, 0);
            ck.check(init, bits, val);
        }
    }
    ck.finish("C17");
}

// ---------------------------------------------------------------------------
// C18 — bit-0 / `mask` (0xFFFFFFFFFFFFFFFE) interaction
// ---------------------------------------------------------------------------
#[test]
fn c18_mask_bit0_interaction() {
    let p = load_pair();
    let mut r = Rng::new(0xC018);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        // bw->val always has bit 0 set
        let bwval = r.interesting_u64() | 1;
        // val shapes that can land a 1 in bit 0 of the result
        let val = match r.next_u64() % 4 {
            0 => u64::MAX,
            1 => 1,
            2 => 0x8000_0000_0000_0000,
            _ => r.next_u64() | 1,
        };
        // mix of loop-skipped and loop-entered configurations
        let (bwbits, bits) = match r.next_u64() % 4 {
            0 => (r.range(0, 62), r.range(0, 1)),      // skipped
            1 => (r.range(0, 62), r.range(64, 128)),   // entered, drains
            2 => (63, r.range(1, 64)),                 // entered, stalls
            _ => (r.range(64, 200), r.range(0, 64)),   // invalid state
        };
        let init = Bitwriter::new(bwval, bwbits, r.next_u32(), r.next_u32(), r.next_u32(), 0);
        ck.check(init, bits, val);
    }
    ck.finish("C18");
}

// ---------------------------------------------------------------------------
// C19 — bw->tot accumulation and wraparound
// ---------------------------------------------------------------------------
#[test]
fn c19_tot_wraparound() {
    let p = load_pair();
    let mut r = Rng::new(0xC019);
    let mut ck = Checker::new(&p);
    for tot in [0u32, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF] {
        for _ in 0..1_000 {
            let bits = r.interesting_bits();
            let bwbits = r.interesting_bits();
            let init = Bitwriter::new(r.interesting_u64(), bwbits, 0, 0, tot, 0);
            ck.check(init, bits, r.interesting_u64());
        }
    }
    for _ in 0..10_000 {
        let init =
            Bitwriter::new(r.interesting_u64(), r.interesting_bits(), 0, 0, r.next_u32(), 0);
        ck.check(init, r.interesting_bits(), r.interesting_u64());
    }
    ck.finish("C19");
}

// ---------------------------------------------------------------------------
// C20 — untouched fields (pos, len, buffer) + struct ABI
// ---------------------------------------------------------------------------
#[test]
fn c20_untouched_fields_and_abi() {
    let p = load_pair();

    // The Rust #[repr(C)] mirror must match the C ABI exactly.
    assert_eq!(std::mem::size_of::<Bitwriter>(), 32, "sizeof(tflac_bitwriter)");
    assert_eq!(std::mem::align_of::<Bitwriter>(), 8, "alignof(tflac_bitwriter)");

    let mut r = Rng::new(0xC020);
    let mut ck = Checker::new(&p);
    for _ in 0..20_000 {
        let pos = r.next_u32();
        let len = r.next_u32();
        // non-null bogus pointer: the C must never dereference `buffer`
        let buf = (r.next_u64() | 1) as usize;
        let init = Bitwriter::new(r.interesting_u64(), r.interesting_bits(), pos, len, r.next_u32(), buf);
        let bits = r.interesting_bits();
        let val = r.interesting_u64();

        // and confirm they really are unchanged by the C itself
        let mut cs = init;
        p.c.call(&mut cs, bits, val);
        assert_eq!(cs.pos, pos, "C modified pos");
        assert_eq!(cs.len, len, "C modified len");
        assert_eq!(cs.buffer as usize, buf, "C modified buffer");

        ck.check(init, bits, val);
    }
    ck.finish("C20");
}

// ---------------------------------------------------------------------------
// C21 — realistic sequential pipeline, bits in 1..=32
// ---------------------------------------------------------------------------
#[test]
fn c21_sequential_pipeline_realistic() {
    let p = load_pair();
    let mut r = Rng::new(0xC021);
    let mut ck = Checker::new(&p);
    for _ in 0..20 {
        let steps: Vec<(u32, u64)> = (0..2_000)
            .map(|_| {
                let bits = r.range(1, 32);
                let val = r.next_u64() & ((1u64 << bits) - 1); // a real bit-packer masks first
                (bits, val)
            })
            .collect();
        ck.check_sequence(Bitwriter::zeroed(), &steps);
    }
    ck.finish("C21");
}

// ---------------------------------------------------------------------------
// C22 — sequential pipeline, unconstrained (drives state into invalid ranges)
// ---------------------------------------------------------------------------
#[test]
fn c22_sequential_pipeline_unconstrained() {
    let p = load_pair();
    let mut r = Rng::new(0xC022);
    let mut ck = Checker::new(&p);
    for _ in 0..20 {
        let steps: Vec<(u32, u64)> =
            (0..2_000).map(|_| (r.interesting_bits(), r.interesting_u64())).collect();
        ck.check_sequence(Bitwriter::zeroed(), &steps);
    }
    // also start from junk states
    for _ in 0..20 {
        let start = Bitwriter::new(
            r.interesting_u64(),
            r.interesting_bits(),
            r.next_u32(),
            r.next_u32(),
            r.next_u32(),
            0,
        );
        let steps: Vec<(u32, u64)> =
            (0..500).map(|_| (r.interesting_bits(), r.interesting_u64())).collect();
        ck.check_sequence(start, &steps);
    }
    ck.finish("C22");
}

// ---------------------------------------------------------------------------
// C23 — full-random fuzz over all six fields + both arguments
// ---------------------------------------------------------------------------
#[test]
fn c23_full_random_fuzz() {
    let p = load_pair();
    let mut r = Rng::new(0xC023);
    let mut ck = Checker::new(&p);
    for _ in 0..100_000 {
        let init = Bitwriter::new(
            r.interesting_u64(),
            r.interesting_bits(),
            r.next_u32(),
            r.next_u32(),
            r.next_u32(),
            r.next_u64() as usize,
        );
        ck.check(init, r.interesting_bits(), r.interesting_u64());
    }
    ck.finish("C23");
}

// ---------------------------------------------------------------------------
// C24 — exhaustive small grid: bw->bits 0..=70 x bits 0..=70
// ---------------------------------------------------------------------------
#[test]
fn c24_exhaustive_small_grid() {
    let p = load_pair();
    let mut ck = Checker::new(&p);
    for bwbits in 0u32..=70 {
        for bits in 0u32..=70 {
            for &(bwval, val) in &[
                (0u64, 0u64),
                (0u64, u64::MAX),
                (u64::MAX, 0u64),
                (u64::MAX, u64::MAX),
                (0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210),
                (0xAAAA_AAAA_AAAA_AAAB, 0x5555_5555_5555_5555),
            ] {
                let init = Bitwriter::new(bwval, bwbits, 0xAAAA, 0xBBBB, 0xFFFF_FF00, 0);
                ck.check(init, bits, val);
            }
        }
    }
    ck.finish("C24");
}
