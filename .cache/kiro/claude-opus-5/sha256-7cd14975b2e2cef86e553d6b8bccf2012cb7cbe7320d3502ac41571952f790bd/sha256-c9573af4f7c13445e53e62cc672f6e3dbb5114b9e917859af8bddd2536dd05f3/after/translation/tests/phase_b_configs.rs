//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH `.so`s through `libloading` and compares the full
//! 32-byte `tflac_bitwriter` image plus the `int` return, byte for byte.
//! Randomized axes use a fixed seed so failures reproduce exactly.

mod common;
use common::*;

/// Fixed base seed; each row derives its own stream from it.
const SEED: u64 = 0x5EED_0000_C0FF_EE01;

/// Every `bw->bits` class worth sweeping, including out-of-range states.
const BW_BITS_SWEEP: &[u32] = &[
    0, 1, 2, 3, 7, 8, 15, 16, 31, 32, 33, 47, 61, 62, 63, 64, 65, 66, 100, 127, 128, 255, 256,
    0xFFFF, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF,
];

const ITERS: usize = 400;

// ---------------------------------------------------------------- C1
#[test]
fn c1_bits_zero_all_bw_bits() {
    let p = pair();
    let rng = Rng::new(SEED ^ 1);
    for &bb in BW_BITS_SWEEP {
        for _ in 0..ITERS {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "C1 bits=0", s, 0, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- C2
#[test]
fn c2_single_bit_into_empty() {
    let p = pair();
    let rng = Rng::new(SEED ^ 2);
    for _ in 0..(ITERS * 8) {
        let s = state_with(&rng, 0, rng.interesting_u64());
        assert_same(p, "C2 bits=1 bw.bits=0", s, 1, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- C3
#[test]
fn c3_b_zero_iteration_cap() {
    let p = pair();
    let rng = Rng::new(SEED ^ 3);
    for _ in 0..(ITERS * 4) {
        let s = state_with(&rng, 63, rng.interesting_u64());
        assert_same(p, "C3 bits=1 bw.bits=63 (b==0, 100 iters)", s, 1, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- C4
#[test]
fn c4_mid_bits_empty_writer() {
    let p = pair();
    let rng = Rng::new(SEED ^ 4);
    for _ in 0..(ITERS * 8) {
        let bits = rng.range_u32(2, 62);
        let s = state_with(&rng, 0, rng.interesting_u64());
        assert_same(p, "C4 mid bits, bw.bits=0", s, bits, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- C5
#[test]
fn c5_accumulate_no_loop() {
    let p = pair();
    let rng = Rng::new(SEED ^ 5);
    for _ in 0..(ITERS * 8) {
        // bw.bits + bits < 64  =>  loop body never runs
        let bb = rng.range_u32(1, 62);
        let bits = rng.range_u32(0, 63 - bb);
        let s = state_with(&rng, bb, rng.interesting_u64());
        assert_same(p, "C5 sum<64, zero iterations", s, bits, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- C6
#[test]
fn c6_single_loop_iteration_both_ternary_arms() {
    let p = pair();
    let rng = Rng::new(SEED ^ 6);
    for _ in 0..(ITERS * 8) {
        // 64 <= bw.bits + bits < 128
        let bb = rng.range_u32(1, 63);
        let bits = rng.range_u32(64 - bb, 127 - bb);
        let s = state_with(&rng, bb, rng.interesting_u64());
        assert_same(p, "C6 64<=sum<128", s, bits, rng.interesting_u64());
    }
    // Explicitly force each ternary arm: b = 63-bw.bits vs bits.
    for bb in 0u32..=63 {
        let b = 63 - bb;
        for bits in [b.saturating_sub(1), b, b + 1, b + 2, 64, 65] {
            for _ in 0..16 {
                let s = state_with(&rng, bb, rng.interesting_u64());
                assert_same(p, "C6 ternary arms", s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------- C7
#[test]
fn c7_bits_63() {
    let p = pair();
    let rng = Rng::new(SEED ^ 7);
    for &bb in BW_BITS_SWEEP {
        for _ in 0..ITERS {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "C7 bits=63", s, 63, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- C8
#[test]
fn c8_bits_64_empty() {
    let p = pair();
    let rng = Rng::new(SEED ^ 8);
    for _ in 0..(ITERS * 8) {
        let s = state_with(&rng, 0, rng.interesting_u64());
        assert_same(p, "C8 bits=64 bw.bits=0", s, 64, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- C9
#[test]
fn c9_bits_64_all_bw_bits() {
    let p = pair();
    let rng = Rng::new(SEED ^ 9);
    for &bb in BW_BITS_SWEEP {
        for _ in 0..ITERS {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "C9 bits=64", s, 64, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- C10
#[test]
fn c10_bits_65_one_past_range() {
    let p = pair();
    let rng = Rng::new(SEED ^ 10);
    for bb in 0u32..=64 {
        for _ in 0..64 {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "C10 bits=65", s, 65, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- C11
#[test]
fn c11_bits_above_word_width() {
    let p = pair();
    let rng = Rng::new(SEED ^ 11);
    for &bits in &[66u32, 70, 100, 127, 128, 199, 255, 256, 1000, 4096] {
        for &bb in BW_BITS_SWEEP {
            for _ in 0..24 {
                let s = state_with(&rng, bb, rng.interesting_u64());
                assert_same(p, "C11 bits>64", s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------- C12
#[test]
fn c12_huge_bits() {
    let p = pair();
    let rng = Rng::new(SEED ^ 12);
    for &bits in &[0x8000_0000u32, 0xFFFF_FFFF, 0xC000_0000, 0x9ABC_DEF0] {
        for &bb in BW_BITS_SWEEP {
            for _ in 0..24 {
                let s = state_with(&rng, bb, rng.interesting_u64());
                assert_same(p, "C12 huge bits", s, bits, rng.interesting_u64());
            }
        }
    }
    for _ in 0..(ITERS * 2) {
        let bits = rng.next_u32() | 0x8000_0000;
        let s = state_with(&rng, 0, rng.interesting_u64());
        assert_same(p, "C12 random huge bits", s, bits, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- C13
#[test]
fn c13_bw_bits_exactly_64() {
    let p = pair();
    let rng = Rng::new(SEED ^ 13);
    for bits in 0u32..=65 {
        for _ in 0..64 {
            let s = state_with(&rng, 64, rng.interesting_u64());
            assert_same(p, "C13 bw.bits=64", s, bits, rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- C14
#[test]
fn c14_bw_bits_above_63() {
    let p = pair();
    let rng = Rng::new(SEED ^ 14);
    for &bb in &[65u32, 66, 70, 100, 0xFF, 0x100, 0xFFFF, 0x1_0000, 0x7FFF_FFFF] {
        for _ in 0..(ITERS * 2) {
            let s = state_with(&rng, bb, rng.interesting_u64());
            assert_same(p, "C14 bw.bits>64", s, rng.interesting_bits(), rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- C15
#[test]
fn c15_loop_condition_u32_wrap_skips_loop() {
    let p = pair();
    let rng = Rng::new(SEED ^ 15);
    // bw.bits = 0xFFFFFFFF, bits chosen so (u32)(bw.bits + bits) < 64
    // i.e. bits in 1..=64  =>  wrapped sum = bits - 1  in 0..=63.
    for bb in [0xFFFF_FFFFu32, 0xFFFF_FFFE, 0xFFFF_FF00] {
        let deficit = (0u32).wrapping_sub(bb); // bits that make the sum wrap to 0
        for k in 0u32..64 {
            let bits = deficit.wrapping_add(k);
            for _ in 0..16 {
                let s = state_with(&rng, bb, rng.interesting_u64());
                let (_, sc) = p.c.add(s, bits, 0);
                // sanity: the wrapped sum really is < 64 (loop skipped)
                debug_assert!(bb.wrapping_add(bits) < 64);
                let _ = sc;
                assert_same(p, "C15 u32 wrap skips loop", s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------- C16
#[test]
fn c16_loop_condition_u32_wrap_enters_loop() {
    let p = pair();
    let rng = Rng::new(SEED ^ 16);
    for bb in [0xFFFF_FFFFu32, 0xFFFF_FFF0, 0x8000_0000] {
        let deficit = (0u32).wrapping_sub(bb);
        for k in [64u32, 65, 100, 1000, 0x1000, 0x8000_0000] {
            let bits = deficit.wrapping_add(k);
            for _ in 0..24 {
                let s = state_with(&rng, bb, rng.interesting_u64());
                assert_same(p, "C16 wrapped sum >= 64", s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------- C17
#[test]
fn c17_val_boundary_patterns() {
    let p = pair();
    let rng = Rng::new(SEED ^ 17);
    const VALS: &[u64] = &[
        0,
        1,
        2,
        3,
        0x8000_0000_0000_0000,
        0x4000_0000_0000_0000,
        0xFFFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFE,
        0xAAAA_AAAA_AAAA_AAAA,
        0x5555_5555_5555_5555,
        0x0000_0000_FFFF_FFFF,
        0xFFFF_FFFF_0000_0000,
        0x0123_4567_89AB_CDEF,
    ];
    for &val in VALS {
        for bits in [0u32, 1, 2, 31, 32, 33, 62, 63, 64, 65, 100, 0xFFFF_FFFF] {
            for &bb in BW_BITS_SWEEP {
                let s = state_with(&rng, bb, rng.interesting_u64());
                assert_same(p, "C17 val boundary patterns", s, bits, val);
            }
        }
    }
}

// ---------------------------------------------------------------- C18
#[test]
fn c18_dirty_incoming_bw_val() {
    let p = pair();
    let rng = Rng::new(SEED ^ 18);
    for &bwval in &[
        0u64,
        1,
        0xFFFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFE,
        0xAAAA_AAAA_AAAA_AAAA,
        0x5555_5555_5555_5555,
    ] {
        for _ in 0..(ITERS * 4) {
            let bb = rng.interesting_bits();
            let s = state_with(&rng, bb, bwval);
            assert_same(p, "C18 dirty bw.val", s, rng.interesting_bits(), rng.interesting_u64());
        }
    }
}

// ---------------------------------------------------------------- C19
#[test]
fn c19_tot_wraps() {
    let p = pair();
    let rng = Rng::new(SEED ^ 19);
    for &tot in &[
        0u32,
        1,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FF00,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ] {
        for &bits in &[0u32, 1, 63, 64, 65, 100, 0x8000_0000, 0xFFFF_FFFF] {
            for _ in 0..32 {
                let mut s = state_with(&rng, rng.interesting_bits(), rng.interesting_u64());
                s.set_tot(tot);
                assert_same(p, "C19 tot wrap", s, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------- C20
#[test]
fn c20_untouched_fields_preserved() {
    let p = pair();
    let rng = Rng::new(SEED ^ 20);
    for _ in 0..(ITERS * 8) {
        let mut s = Bw::from_bytes(rng.bytes32());
        s.set_bits(rng.interesting_bits());
        // deliberately inconsistent: pos > len, bogus non-null buffer
        s.set_pos(rng.next_u32());
        s.set_len(rng.next_u32());
        s.set_buffer(rng.next_u64());
        let bits = rng.interesting_bits();
        let val = rng.interesting_u64();
        assert_same(p, "C20 garbage pos/len/buffer", s, bits, val);

        // and explicitly confirm C leaves them alone, so Rust must too
        let (_, after) = p.c.add(s, bits, val);
        assert_eq!(after.pos(), s.pos(), "C mutated pos");
        assert_eq!(after.len(), s.len(), "C mutated len");
        assert_eq!(after.buffer(), s.buffer(), "C mutated buffer");
        let (_, after_r) = p.rust.add(s, bits, val);
        assert_eq!(after_r.pos(), s.pos(), "Rust mutated pos");
        assert_eq!(after_r.len(), s.len(), "Rust mutated len");
        assert_eq!(after_r.buffer(), s.buffer(), "Rust mutated buffer");
    }
    // null buffer variant
    for _ in 0..ITERS {
        let mut s = Bw::from_bytes(rng.bytes32());
        s.set_buffer(0);
        s.set_bits(rng.interesting_bits());
        assert_same(p, "C20 null buffer", s, rng.interesting_bits(), rng.interesting_u64());
    }
}

// ---------------------------------------------------------------- C21
#[test]
fn c21_chained_realistic_bit_packing() {
    let p = pair();
    let rng = Rng::new(SEED ^ 21);
    for _ in 0..200 {
        let mut start = Bw::zeroed();
        start.set_buffer(rng.next_u64());
        start.set_len(rng.next_u32());
        let ops: Vec<(u32, u64)> = (0..64)
            .map(|_| (rng.range_u32(1, 32), rng.interesting_u64()))
            .collect();
        assert_same_chain(p, "C21 chained bit packing", start, &ops);
    }
}

// ---------------------------------------------------------------- C22
#[test]
fn c22_chained_fully_random() {
    let p = pair();
    let rng = Rng::new(SEED ^ 22);
    for _ in 0..200 {
        let start = Bw::from_bytes(rng.bytes32());
        let ops: Vec<(u32, u64)> = (0..64)
            .map(|_| (rng.interesting_bits(), rng.interesting_u64()))
            .collect();
        assert_same_chain(p, "C22 chained random", start, &ops);
    }
}

// ---------------------------------------------------------------- C23
#[test]
fn c23_chained_repeated_iteration_cap() {
    let p = pair();
    let rng = Rng::new(SEED ^ 23);
    for _ in 0..100 {
        let mut start = Bw::zeroed();
        start.set_bits(63);
        start.set_val(rng.interesting_u64());
        let ops: Vec<(u32, u64)> = (0..40)
            .map(|i| {
                let bits = if i % 2 == 0 { 63 } else { 1 };
                (bits, rng.interesting_u64())
            })
            .collect();
        assert_same_chain(p, "C23 chained cap", start, &ops);
    }
}

// ---------------------------------------------------------------- C24
#[test]
fn c24_unconstrained_fuzz() {
    let p = pair();
    let rng = Rng::new(SEED ^ 24);
    // Every struct byte random + random bits/val. Keeps the loop-heavy cases
    // in play while covering combinations not individually named above.
    for _ in 0..200_000 {
        let s = Bw::from_bytes(rng.bytes32());
        assert_same(p, "C24 fuzz", s, rng.interesting_bits(), rng.interesting_u64());
    }
}
