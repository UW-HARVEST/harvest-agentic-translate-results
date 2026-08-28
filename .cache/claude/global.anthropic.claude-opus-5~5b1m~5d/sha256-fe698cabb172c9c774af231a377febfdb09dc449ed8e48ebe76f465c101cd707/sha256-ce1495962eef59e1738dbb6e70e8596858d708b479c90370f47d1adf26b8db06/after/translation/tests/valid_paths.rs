//! Phase B - valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md` (C1..C24). Every test drives BOTH the C
//! `.so` and the Rust `.so` through `libloading` and asserts byte-identical
//! results. Randomised rows use a fixed seed so failures are reproducible.

mod common;

use common::{assert_same, assert_same_all, reverse16, Rng};

/// The eight literal masks that appear in the C source, in source order.
const MASKS: [u32; 8] = [
    0xAAAA, 0x5555, 0xCCCC, 0x3333, 0xF0F0, 0x0F0F, 0xFF00, 0x00FF,
];

/// Low halves that are worth pinning while the high half is swept (row C15).
const PINNED_LOW: [u32; 6] = [0x0000, 0x0001, 0x8000, 0xFFFF, 0xAAAA, 0x5555];

fn compose(high: u32, low: u32) -> u32 {
    ((high & 0xFFFF) << 16) | (low & 0xFFFF)
}

// ---------------------------------------------------------------------------
// C1 - the empty input.
// ---------------------------------------------------------------------------
#[test]
fn c1_zero_input() {
    assert_same("C1", 0x0000_0000);
}

// ---------------------------------------------------------------------------
// C2 - single bit in the low half, exhaustive over all 16 positions.
// ---------------------------------------------------------------------------
#[test]
fn c2_single_low_bit_exhaustive() {
    let n = assert_same_all("C2", (0..16).map(|k| 1u32 << k));
    assert_eq!(n, 16);
}

// ---------------------------------------------------------------------------
// C3 - single bit in the high half, exhaustive over all 16 positions.
// ---------------------------------------------------------------------------
#[test]
fn c3_single_high_bit_exhaustive() {
    let n = assert_same_all("C3", (16..32).map(|k| 1u32 << k));
    assert_eq!(n, 16);
}

// ---------------------------------------------------------------------------
// C4 - full 16x16 cross-product of one low bit and one high bit.
// ---------------------------------------------------------------------------
#[test]
fn c4_low_bit_cross_high_bit() {
    let mut args = Vec::with_capacity(256);
    for i in 0..16 {
        for j in 0..16 {
            args.push((1u32 << i) | (1u32 << (16 + j)));
        }
    }
    let n = assert_same_all("C4", args);
    assert_eq!(n, 256);
}

// ---------------------------------------------------------------------------
// C5 / C6 - all-ones low half, with and without an all-ones high half.
// ---------------------------------------------------------------------------
#[test]
fn c5_all_ones_low_half() {
    assert_same("C5", 0x0000_FFFF);
}

#[test]
fn c6_numeric_maximum() {
    assert_same("C6", 0xFFFF_FFFF);
}

// ---------------------------------------------------------------------------
// C7..C10 - each statement's own masks used verbatim as the input.
// ---------------------------------------------------------------------------
#[test]
fn c7_statement1_masks() {
    assert_same_all("C7", [0xAAAAu32, 0x5555]);
}

#[test]
fn c8_statement2_masks() {
    assert_same_all("C8", [0xCCCCu32, 0x3333]);
}

#[test]
fn c9_statement3_masks() {
    assert_same_all("C9", [0xF0F0u32, 0x0F0F]);
}

#[test]
fn c10_statement4_masks() {
    assert_same_all("C10", [0xFF00u32, 0x00FF]);
}

// ---------------------------------------------------------------------------
// C11 - each mask in the low half, its complement in the (discarded) high half.
// ---------------------------------------------------------------------------
#[test]
fn c11_mask_low_with_complement_high() {
    let args: Vec<u32> = MASKS
        .iter()
        .map(|&m| compose(!m & 0xFFFF, m))
        .collect();
    let n = assert_same_all("C11", args);
    assert_eq!(n, 8);
}

// ---------------------------------------------------------------------------
// C12 - exhaustive low half, zero high half. All 65 536 values.
// ---------------------------------------------------------------------------
#[test]
fn c12_exhaustive_low_half_zero_high() {
    let n = assert_same_all("C12", 0u32..=0xFFFF);
    assert_eq!(n, 65_536);
}

// ---------------------------------------------------------------------------
// C13 - exhaustive low half with a saturated high half. Proves the high half
//       cannot influence the result.
// ---------------------------------------------------------------------------
#[test]
fn c13_exhaustive_low_half_ones_high() {
    let n = assert_same_all("C13", (0u32..=0xFFFF).map(|low| compose(0xFFFF, low)));
    assert_eq!(n, 65_536);

    // Cross-check the invariant through the .so boundary: for every low half the
    // result must be the same whether the high half is 0x0000 or 0xFFFF.
    for low in 0u32..=0xFFFF {
        let bare = assert_same("C13", low);
        let dirty = assert_same("C13", compose(0xFFFF, low));
        assert_eq!(
            bare, dirty,
            "high half changed the result for low=0x{low:04X}"
        );
    }
}

// ---------------------------------------------------------------------------
// C14 - exhaustive low half, randomised high half.
// ---------------------------------------------------------------------------
#[test]
fn c14_exhaustive_low_half_random_high() {
    let mut rng = Rng::new(0xC14_5EED);
    let n = assert_same_all(
        "C14",
        (0u32..=0xFFFF).map(|low| compose(rng.next_u16() as u32, low)),
    );
    assert_eq!(n, 65_536);
}

// ---------------------------------------------------------------------------
// C15 - exhaustive high half for each pinned low half.
// ---------------------------------------------------------------------------
#[test]
fn c15_exhaustive_high_half_per_pinned_low() {
    for &low in &PINNED_LOW {
        let n = assert_same_all("C15", (0u32..=0xFFFF).map(|high| compose(high, low)));
        assert_eq!(n, 65_536);
    }
}

// ---------------------------------------------------------------------------
// C16 - byte-aligned shapes: 0x00XY and 0xXY00 for every byte value.
// ---------------------------------------------------------------------------
#[test]
fn c16_byte_aligned_shapes() {
    let mut args = Vec::with_capacity(512);
    for b in 0u32..=0xFF {
        args.push(b); // 0x00XY
        args.push(b << 8); // 0xXY00
    }
    let n = assert_same_all("C16", args);
    assert_eq!(n, 512);
}

// ---------------------------------------------------------------------------
// C17 - one nibble populated at each of the four nibble slots, random high half.
// ---------------------------------------------------------------------------
#[test]
fn c17_nibble_aligned_shapes() {
    let mut rng = Rng::new(0xC17_5EED);
    let mut args = Vec::with_capacity(4 * 16);
    for slot in 0..4 {
        for v in 0u32..16 {
            args.push(compose(rng.next_u16() as u32, v << (slot * 4)));
        }
    }
    let n = assert_same_all("C17", args);
    assert_eq!(n, 64);
}

// ---------------------------------------------------------------------------
// C18 - palindromic low halves (x == reverse16(x)), random high half.
// ---------------------------------------------------------------------------
#[test]
fn c18_palindromic_low_halves() {
    let mut rng = Rng::new(0xC18_5EED);
    let mut count = 0usize;
    for low in 0u32..=0xFFFF {
        if reverse16(low as u16) as u32 != low {
            continue;
        }
        let arg = compose(rng.next_u16() as u32, low);
        let got = assert_same("C18", arg);
        // A 16-bit palindrome must survive the permutation unchanged; confirm
        // both libraries agree on that, using the C result as the oracle.
        assert_eq!(
            got, low,
            "palindrome 0x{low:04X} was not preserved (got 0x{got:08X})"
        );
        count += 1;
    }
    // There are exactly 2^8 = 256 self-reverse 16-bit values.
    assert_eq!(count, 256);
}

// ---------------------------------------------------------------------------
// C19 - large uniform random sweep across the full 32-bit domain.
// ---------------------------------------------------------------------------
#[test]
fn c19_full_range_random_sweep() {
    const SAMPLES: usize = 4_000_000;
    let mut rng = Rng::new(0xC19_5EED);
    for _ in 0..SAMPLES {
        assert_same("C19", rng.next_u32());
    }
}

// ---------------------------------------------------------------------------
// C20 - sparse and dense Hamming-weight distributions.
// ---------------------------------------------------------------------------
#[test]
fn c20_sparse_and_dense_shapes() {
    let mut rng = Rng::new(0xC20_5EED);
    let mut n = 0usize;
    // Every achievable Hamming weight, many samples each.
    for weight in 0u32..=32 {
        for _ in 0..2_000 {
            let v = if weight == 0 {
                0
            } else if weight == 32 {
                u32::MAX
            } else {
                rng.with_weight(weight)
            };
            assert_same("C20", v);
            n += 1;
        }
    }
    assert_eq!(n, 33 * 2_000);
}

// ---------------------------------------------------------------------------
// C21 - composed pipeline: rev16 applied twice, each hop crossing the FFI
//       boundary, over the exhaustive low half with random high halves.
// ---------------------------------------------------------------------------
#[test]
fn c21_double_application_pipeline() {
    let mut rng = Rng::new(0xC21_5EED);
    for low in 0u32..=0xFFFF {
        let arg = compose(rng.next_u16() as u32, low);

        // Run the two-stage pipeline independently in each library, then compare
        // the final results. This catches divergence that only appears when one
        // call's output feeds the next.
        let c1 = unsafe { common::c_rev16()(arg) };
        let c2 = unsafe { common::c_rev16()(c1) };
        let r1 = unsafe { common::rust_rev16()(arg) };
        let r2 = unsafe { common::rust_rev16()(r1) };

        assert_eq!(c1, r1, "[C21] stage 1 diverged for 0x{arg:08X}");
        assert_eq!(c2, r2, "[C21] stage 2 diverged for 0x{arg:08X}");

        // Cross-feed: Rust's stage-1 output into C's stage 2 and vice versa.
        let cross_a = unsafe { common::c_rev16()(r1) };
        let cross_b = unsafe { common::rust_rev16()(c1) };
        assert_eq!(cross_a, cross_b, "[C21] cross-fed pipeline diverged");
        assert_eq!(cross_a, c2, "[C21] cross-fed pipeline left the C result");

        // rev16 is an involution on the low 16 bits.
        assert_eq!(c2, low, "[C21] rev16 was not self-inverse on 0x{low:04X}");
    }
}

// ---------------------------------------------------------------------------
// C22 - 8-deep chain with a random high-half injection between every stage, so
//       the discard path is re-entered mid-pipeline.
// ---------------------------------------------------------------------------
#[test]
fn c22_deep_chain_with_high_half_injection() {
    let mut rng = Rng::new(0xC22_5EED);
    for _ in 0..200_000 {
        let mut c_state = rng.next_u32();
        let mut r_state = c_state;
        for _ in 0..8 {
            let injection = (rng.next_u16() as u32) << 16;
            c_state = unsafe { common::c_rev16()(c_state | injection) };
            r_state = unsafe { common::rust_rev16()(r_state | injection) };
            assert_eq!(
                c_state, r_state,
                "[C22] chain diverged (injection 0x{injection:08X})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C23 - order independence: replay a shuffled argument set, interleaving the two
//       libraries, and confirm results never depend on call order.
// ---------------------------------------------------------------------------
#[test]
fn c23_call_order_independence() {
    let mut rng = Rng::new(0xC23_5EED);

    // Build a pool of arguments and record each library's answer in order.
    let pool: Vec<u32> = (0..20_000).map(|_| rng.next_u32()).collect();
    let expected: Vec<u32> = pool.iter().map(|&a| assert_same("C23", a)).collect();

    // Fisher-Yates shuffle of the indices, then replay in the new order,
    // alternating which library is called first.
    let mut idx: Vec<usize> = (0..pool.len()).collect();
    for i in (1..idx.len()).rev() {
        let j = rng.below(i as u32 + 1) as usize;
        idx.swap(i, j);
    }

    for (k, &i) in idx.iter().enumerate() {
        let arg = pool[i];
        let (c, r) = if k % 2 == 0 {
            let c = unsafe { common::c_rev16()(arg) };
            let r = unsafe { common::rust_rev16()(arg) };
            (c, r)
        } else {
            let r = unsafe { common::rust_rev16()(arg) };
            let c = unsafe { common::c_rev16()(arg) };
            (c, r)
        };
        assert_eq!(c, r, "[C23] divergence on replay of 0x{arg:08X}");
        assert_eq!(c, expected[i], "[C23] result depended on call order");
    }
}

// ---------------------------------------------------------------------------
// C24 - concurrent invocation of both libraries from 8 threads.
// ---------------------------------------------------------------------------
#[test]
fn c24_concurrent_invocation() {
    const THREADS: u64 = 8;
    const PER_THREAD: usize = 300_000;

    // Resolve the symbols on the main thread first so the OnceLocks are warm.
    let c = common::c_rev16();
    let r = common::rust_rev16();

    let mut handles = Vec::new();
    for t in 0..THREADS {
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::new(0xC24_5EED ^ (t.wrapping_mul(0x9E37_79B9)));
            for _ in 0..PER_THREAD {
                let arg = rng.next_u32();
                let cv = unsafe { c(arg) };
                let rv = unsafe { r(arg) };
                assert_eq!(cv, rv, "[C24] divergence for 0x{arg:08X} on thread {t}");
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}

// ---------------------------------------------------------------------------
// EXHAUSTIVE - the input domain of `rev16` is a single u32, i.e. only 2^32
// values, so the translation can be verified *completely* rather than sampled.
// This drives all 4 294 967 296 arguments through BOTH .so exports and requires
// byte-identical results, which subsumes every row of CONFIGS.md and ERRORS.md.
//
// Marked #[ignore] because it takes ~1-2 minutes; run it with:
//     cargo test --offline --release-so -- --ignored --nocapture
// (any invocation with `-- --ignored` works).
// ---------------------------------------------------------------------------
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with -- --ignored"]
fn exhaustive_all_2pow32_arguments() {
    let c = common::c_rev16();
    let r = common::rust_rev16();

    let mut checked: u64 = 0;
    let mut i: u64 = 0;
    while i <= u32::MAX as u64 {
        let a = i as u32;
        let cv = unsafe { c(a) };
        let rv = unsafe { r(a) };
        if cv != rv {
            panic!("[EXHAUSTIVE] divergence at rev16(0x{a:08X}): C=0x{cv:08X} Rust=0x{rv:08X}");
        }
        checked += 1;
        i += 1;
    }
    assert_eq!(checked, 4_294_967_296u64, "did not cover the whole domain");
    println!("[EXHAUSTIVE] verified all {checked} u32 arguments identical");
}
