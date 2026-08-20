//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads both `c_build/libcdecisions.so` and `target/*/libdriver.so`
//! and compares the exported `process_decisions` symbol's return value *and* the
//! post-call buffer bytes.

mod common;

use common::{
    assert_same, assert_same_full, pattern_to_bytes, Rng, INT_EDGE_VALUES, YN_BYTES,
};

// ===========================================================================
// Operation 0 — apply_permissions
// ===========================================================================

/// C1: exhaustive over all 8 `y`/`n` triples.
#[test]
fn c1_op0_all_eight_triples() {
    let mut seen = Vec::new();
    for bits in 0u32..8 {
        let input = pattern_to_bytes(bits, 3);
        seen.push(assert_same_full(&input, 0, 0, "C1"));
    }
    // Every rwx combination must map to a distinct documented outcome; make sure
    // the differential test actually walked distinct branches.
    let mut sorted = seen.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted,
        vec![-20, -10, 0, 14, 23, 35, 56, 107],
        "C1 must walk all 8 documented apply_permissions outcomes"
    );
}

/// C2: exhaustive over all 4^3 case combinations of `{y,Y,n,N}`.
#[test]
fn c2_op0_case_combinations() {
    for a in YN_BYTES {
        for b in YN_BYTES {
            for c in YN_BYTES {
                assert_same_full(&[a, b, c], 0, 0, "C2");
            }
        }
    }
}

/// C3: random arbitrary bytes in all three positions.
#[test]
fn c3_op0_random_bytes() {
    let mut rng = Rng::new(0xC3_0000);
    for _ in 0..20_000 {
        let input = [rng.byte(), rng.byte(), rng.byte()];
        assert_same_full(&input, 0, 0, "C3");
    }
    // Also exhaustively over one position at a time with the others pinned.
    for b in 0u16..=255 {
        let b = b as u8;
        assert_same_full(&[b, b'y', b'n'], 0, 0, "C3-pos0");
        assert_same_full(&[b'y', b, b'n'], 0, 0, "C3-pos1");
        assert_same_full(&[b'y', b'n', b], 0, 0, "C3-pos2");
    }
}

/// C4: lengths 3..=64 — bytes beyond index 2 must be ignored.
#[test]
fn c4_op0_longer_inputs_ignore_tail() {
    let mut rng = Rng::new(0xC4_0000);
    for len in 3..=64usize {
        for _ in 0..200 {
            let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
            assert_same_full(&input, 0, 0, "C4");
        }
        for _ in 0..200 {
            let input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            assert_same_full(&input, 0, 0, "C4-raw");
        }
    }
}

/// C5: `param` is ignored by operation 0.
#[test]
fn c5_op0_param_ignored() {
    for bits in 0u32..8 {
        let input = pattern_to_bytes(bits, 3);
        for param in INT_EDGE_VALUES {
            assert_same_full(&input, 0, param, "C5");
        }
    }
}

// ===========================================================================
// Operation 1 — evaluate_conditions
// ===========================================================================

fn op1_exhaustive_triples(param: i32, ctx: &str) -> Vec<i32> {
    (0u32..8)
        .map(|bits| assert_same_full(&pattern_to_bytes(bits, 3), 1, param, ctx))
        .collect()
}

/// C6: AND.
#[test]
fn c6_op1_and() {
    let r = op1_exhaustive_triples(0, "C6");
    assert!(r.contains(&100) && r.contains(&0), "C6 results {r:?}");
}

/// C7: OR.
#[test]
fn c7_op1_or() {
    let r = op1_exhaustive_triples(1, "C7");
    assert!(r.contains(&0) && r.contains(&103), "C7 results {r:?}");
}

/// C8: XOR.
#[test]
fn c8_op1_xor() {
    let r = op1_exhaustive_triples(2, "C8");
    assert!(r.contains(&7) && r.contains(&0), "C8 results {r:?}");
}

/// C9: NAND.
#[test]
fn c9_op1_nand() {
    let r = op1_exhaustive_triples(3, "C9");
    assert!(r.contains(&200) && r.contains(&0), "C9 results {r:?}");
}

/// C10: all four logic ops crossed with all 4^3 case combinations.
#[test]
fn c10_op1_case_combinations() {
    for param in 0..4 {
        for a in YN_BYTES {
            for b in YN_BYTES {
                for c in YN_BYTES {
                    assert_same_full(&[a, b, c], 1, param, "C10");
                }
            }
        }
    }
}

/// C11: all four logic ops crossed with random arbitrary bytes.
#[test]
fn c11_op1_random_bytes() {
    let mut rng = Rng::new(0xC11_0000);
    for param in 0..4 {
        for _ in 0..20_000 {
            let input = [rng.byte(), rng.byte(), rng.byte()];
            assert_same_full(&input, 1, param, "C11");
        }
    }
}

/// C12: lengths 3..=64 — bytes beyond index 2 must be ignored.
#[test]
fn c12_op1_longer_inputs_ignore_tail() {
    let mut rng = Rng::new(0xC12_0000);
    for len in 3..=64usize {
        for param in 0..4 {
            for _ in 0..40 {
                let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
                assert_same_full(&input, 1, param, "C12");
            }
        }
    }
}

// ===========================================================================
// Operation 2 — configure_flags
// ===========================================================================

/// C13: `count == 1` boundary.
#[test]
fn c13_op2_single_byte() {
    for b in YN_BYTES {
        assert_same_full(&[b], 2, 0, "C13");
    }
    assert_eq!(assert_same_full(b"n", 2, 0, "C13-false"), 0);
    assert_eq!(assert_same_full(b"y", 2, 0, "C13-true"), 1001);
}

/// C14: all-false inputs, every length 1..=40.
#[test]
fn c14_op2_all_false() {
    for len in 1..=40usize {
        assert_same_full(&vec![b'n'; len], 2, 0, "C14");
        assert_same_full(&vec![b'N'; len], 2, 0, "C14-upper");
        // Bytes that are not y/Y/n/N also parse as false.
        assert_same_full(&vec![b'?'; len], 2, 0, "C14-other");
        assert_same_full(&vec![0u8; len], 2, 0, "C14-nul");
    }
}

/// C15: all-true inputs, every length 1..=40 (crosses the 32 clamp).
#[test]
fn c15_op2_all_true() {
    for len in 1..=40usize {
        let r = assert_same_full(&vec![b'y'; len], 2, 0, "C15");
        assert_same_full(&vec![b'Y'; len], 2, 0, "C15-upper");
        assert_eq!(r, 1000 + len.min(32) as i32, "C15 len={len}");
    }
}

/// C16: exactly one true, at every index, for every length.
#[test]
fn c16_op2_exactly_one_true() {
    for len in 1..=40usize {
        for i in 0..len {
            let mut input = vec![b'n'; len];
            input[i] = b'y';
            assert_same_full(&input, 2, 0, "C16");
        }
    }
}

/// C17: exactly one false, at every index, for every length.
#[test]
fn c17_op2_exactly_one_false() {
    for len in 1..=40usize {
        for i in 0..len {
            let mut input = vec![b'y'; len];
            input[i] = b'n';
            assert_same_full(&input, 2, 0, "C17");
        }
    }
}

/// C18: strictly alternating patterns from both phases.
#[test]
fn c18_op2_alternating() {
    for len in 1..=40usize {
        for start in [true, false] {
            let input: Vec<u8> = (0..len)
                .map(|i| {
                    if (i % 2 == 0) == start {
                        b'y'
                    } else {
                        b'n'
                    }
                })
                .collect();
            assert_same_full(&input, 2, 0, "C18");
        }
    }
}

/// C19: maximal true-run of exactly `k`.
#[test]
fn c19_op2_consecutive_runs() {
    for k in 1..=10usize {
        for lead in 0..4usize {
            for trail in 0..4usize {
                let mut input = vec![b'n'; lead];
                input.extend(std::iter::repeat(b'y').take(k));
                input.push(b'n');
                // A shorter second run so the maximum stays `k`.
                input.extend(std::iter::repeat(b'y').take(k.saturating_sub(1).min(trail)));
                input.extend(std::iter::repeat(b'n').take(trail));
                if input.is_empty() {
                    continue;
                }
                assert_same_full(&input, 2, 0, "C19");
            }
        }
    }
}

/// C20: the 31 / 32 / 33 clamp boundary.
#[test]
fn c20_op2_clamp_boundary() {
    let mut rng = Rng::new(0xC20_0000);
    for len in [31usize, 32, 33] {
        for _ in 0..5_000 {
            let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
            assert_same_full(&input, 2, 0, "C20");
        }
    }
}

/// C21: lengths 33..=64 must behave exactly like the 32-byte prefix.
#[test]
fn c21_op2_tail_beyond_32_ignored() {
    let mut rng = Rng::new(0xC21_0000);
    for len in 33..=64usize {
        for _ in 0..200 {
            let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
            let full = assert_same_full(&input, 2, 0, "C21");
            let prefix = assert_same_full(&input[..32], 2, 0, "C21-prefix");
            assert_eq!(full, prefix, "C21: tail past 32 changed the result (len={len})");
        }
    }
}

/// C22: `length == 1023`, the largest value `main` can produce.
#[test]
fn c22_op2_max_length() {
    let mut rng = Rng::new(0xC22_0000);
    for _ in 0..500 {
        let input: Vec<u8> = (0..1023).map(|_| rng.yn_byte()).collect();
        assert_same_full(&input, 2, 0, "C22");
    }
    assert_same_full(&vec![b'y'; 1023], 2, 0, "C22-all-true");
    assert_same_full(&vec![b'n'; 1023], 2, 0, "C22-all-false");
}

/// C23: exhaustive over every `y`/`n` pattern for lengths 1..=12.
#[test]
fn c23_op2_exhaustive_small_patterns() {
    for len in 1..=12usize {
        for bits in 0u32..(1u32 << len) {
            assert_same_full(&pattern_to_bytes(bits, len), 2, 0, "C23");
        }
    }
}

/// C24: random arbitrary bytes at random lengths.
#[test]
fn c24_op2_random_bytes() {
    let mut rng = Rng::new(0xC24_0000);
    for _ in 0..50_000 {
        let len = 1 + rng.below(64);
        let input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        assert_same_full(&input, 2, 0, "C24");
    }
}

/// C25: `param` is ignored by operation 2.
#[test]
fn c25_op2_param_ignored() {
    let mut rng = Rng::new(0xC25_0000);
    for _ in 0..2_000 {
        let len = 1 + rng.below(40);
        let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
        for param in INT_EDGE_VALUES {
            assert_same_full(&input, 2, param, "C25");
        }
    }
}

// ===========================================================================
// Operation 3 — validate_sequence
// ===========================================================================

/// C26: single-byte sequences.
#[test]
fn c26_op3_single_byte() {
    for b in YN_BYTES {
        assert_same_full(&[b], 3, 0, "C26");
    }
    assert_eq!(assert_same_full(b"y", 3, 0, "C26-y"), 1);
    assert_eq!(assert_same_full(b"n", 3, 0, "C26-n"), -10);
}

/// C27: exhaustive over every `y`/`n` pattern for lengths 1..=14.
#[test]
fn c27_op3_exhaustive_small_patterns() {
    let mut results = std::collections::BTreeSet::new();
    for len in 1..=14usize {
        for bits in 0u32..(1u32 << len) {
            results.insert(assert_same_full(&pattern_to_bytes(bits, len), 3, 0, "C27"));
        }
    }
    // Sanity: the sweep must have reached every *reachable* return value tier.
    for expected in [-12, -11, -10, 1, 2, 11, 20, 25, 30, 45, 50] {
        assert!(
            results.contains(&expected),
            "C27 never produced {expected}; saw {results:?}"
        );
    }
    // `return 40` (long tier, `transitions < 3`) is dead code in the C: rule 3
    // caps every run at 3 equal values, so a sequence of length >= 11 has at
    // least ceil(11/3) - 1 == 3 transitions.  Neither implementation may return
    // it.
    assert!(
        !results.contains(&40),
        "C27 produced the unreachable value 40; saw {results:?}"
    );
}

/// C28: the `len <= 3` / `len <= 10` tier boundary.
#[test]
fn c28_op3_tier_boundary_3_4() {
    for len in [3usize, 4] {
        for bits in 0u32..(1u32 << len) {
            assert_same_full(&pattern_to_bytes(bits, len), 3, 0, "C28");
        }
    }
    for a in YN_BYTES {
        for b in YN_BYTES {
            for c in YN_BYTES {
                assert_same_full(&[a, b, c], 3, 0, "C28-case");
                for d in YN_BYTES {
                    assert_same_full(&[a, b, c, d], 3, 0, "C28-case4");
                }
            }
        }
    }
}

/// C29: the `len <= 10` / long tier boundary.
#[test]
fn c29_op3_tier_boundary_10_11() {
    for len in [10usize, 11] {
        for bits in 0u32..(1u32 << len) {
            assert_same_full(&pattern_to_bytes(bits, len), 3, 0, "C29");
        }
    }
}

/// C30: medium tier (4..=10) transition-count buckets `20` / `25` / `30`.
#[test]
fn c30_op3_medium_tier_buckets() {
    let mut buckets = std::collections::BTreeSet::new();
    for len in 4..=10usize {
        for bits in 0u32..(1u32 << len) {
            let r = assert_same_full(&pattern_to_bytes(bits, len), 3, 0, "C30");
            if r > 0 {
                buckets.insert(r);
            }
        }
    }
    for expected in [20, 25, 30] {
        assert!(buckets.contains(&expected), "C30 missing {expected}: {buckets:?}");
    }
}

/// C31: long tier (11..=64) transition-count buckets `40` / `45` / `50`.
#[test]
fn c31_op3_long_tier_buckets() {
    let mut rng = Rng::new(0xC31_0000);
    let mut buckets = std::collections::BTreeSet::new();

    for len in 11..=64usize {
        // Few transitions: "y" + "n"*(len-1) has exactly one transition.
        let mut few = vec![b'n'; len];
        few[0] = b'y';
        buckets.insert(assert_same_full(&few, 3, 0, "C31-few"));

        // Many transitions: strict alternation starting with y and ending n.
        let alt: Vec<u8> = (0..len)
            .map(|i| if i % 2 == 0 { b'y' } else { b'n' })
            .collect();
        buckets.insert(assert_same_full(&alt, 3, 0, "C31-many"));

        // Mid-range: blocks of two.
        let blocks: Vec<u8> = (0..len)
            .map(|i| if (i / 2) % 2 == 0 { b'y' } else { b'n' })
            .collect();
        buckets.insert(assert_same_full(&blocks, 3, 0, "C31-mid"));

        for _ in 0..300 {
            let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
            buckets.insert(assert_same_full(&input, 3, 0, "C31-rand"));
        }
    }

    for expected in [45, 50] {
        assert!(buckets.contains(&expected), "C31 missing {expected}: {buckets:?}");
    }
    // `return 40` is unreachable (see `c27_op3_exhaustive_small_patterns`): with
    // rule 3 capping runs at 3, any surviving sequence of length >= 11 has at
    // least 3 transitions.  Exhaustively confirm both implementations agree.
    assert!(
        !buckets.contains(&40),
        "C31 produced the unreachable value 40: {buckets:?}"
    );
    for len in 11..=20usize {
        for bits in 0u32..(1u32 << 12.min(len)) {
            let mut input = pattern_to_bytes(bits, len);
            input[0] = b'y';
            input[len - 1] = b'n';
            assert_ne!(
                assert_same_full(&input, 3, 0, "C31-no-40"),
                40,
                "C31: len={len} bits={bits} reached the dead branch"
            );
        }
    }
}

/// C32: case-mixed sequences at random lengths.
#[test]
fn c32_op3_case_mixed() {
    let mut rng = Rng::new(0xC32_0000);
    for _ in 0..50_000 {
        let len = 1 + rng.below(40);
        let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
        assert_same_full(&input, 3, 0, "C32");
    }
}

/// C33: random arbitrary bytes at random lengths.
#[test]
fn c33_op3_random_bytes() {
    let mut rng = Rng::new(0xC33_0000);
    for _ in 0..50_000 {
        let len = 1 + rng.below(64);
        let input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        assert_same_full(&input, 3, 0, "C33");
    }
    // Biased mix so the interesting rules are reachable with random bytes too.
    for _ in 0..50_000 {
        let len = 1 + rng.below(64);
        let input: Vec<u8> = (0..len)
            .map(|_| if rng.below(4) == 0 { rng.byte() } else { rng.yn_byte() })
            .collect();
        assert_same_full(&input, 3, 0, "C33-biased");
    }
}

/// C34: `length == 1023`.
#[test]
fn c34_op3_max_length() {
    let mut rng = Rng::new(0xC34_0000);
    for _ in 0..300 {
        let input: Vec<u8> = (0..1023).map(|_| rng.yn_byte()).collect();
        assert_same_full(&input, 3, 0, "C34");
    }
    // A 1023-byte sequence that survives all three rules: y n y n ... n.
    let alt: Vec<u8> = (0..1023)
        .map(|i| if i % 2 == 0 { b'y' } else { b'n' })
        .collect();
    assert_same_full(&alt, 3, 0, "C34-alt");

    // Well past anything `main` can produce, to stress the long-tier
    // `len - 3` / `size_t` arithmetic at scale.
    for len in [1024usize, 4096, 10_000] {
        let alt: Vec<u8> = (0..len)
            .map(|i| if i % 2 == 0 { b'y' } else { b'n' })
            .collect();
        assert_same_full(&alt, 3, 0, "C34-huge-alt");

        let blocks: Vec<u8> = (0..len)
            .map(|i| if (i / 3) % 2 == 0 { b'y' } else { b'n' })
            .collect();
        assert_same_full(&blocks, 3, 0, "C34-huge-blocks");

        let mut few = vec![b'n'; len];
        few[0] = b'y';
        assert_same_full(&few, 3, 0, "C34-huge-few");

        for _ in 0..20 {
            let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
            assert_same_full(&input, 3, 0, "C34-huge-rand");
        }
        // The other operations at the same scale.
        for op in [0, 1, 2] {
            for param in 0..4 {
                assert_same_full(&alt, op, param, "C34-huge-other-ops");
            }
        }
    }
}

/// C35: the in-place `bool *` rewrite of the caller's buffer.
///
/// `assert_same` already compares the whole post-call buffer for every single
/// call in this file, but this test pins the behaviour down explicitly,
/// including on the early-return rule-violation paths where C has *already*
/// rewritten every byte before bailing out.
#[test]
fn c35_op3_buffer_rewrite_matches() {
    use std::ffi::c_char;
    let l = common::libs();

    let cases: &[&[u8]] = &[
        b"y",
        b"n",
        b"Y",
        b"N",
        b"yn",
        b"ny",          // -10, buffer still rewritten
        b"yy",          // -11, buffer still rewritten
        b"ynnnn",       // -12, buffer still rewritten
        b"yyyyn",       // -12
        b"y?n",         // '?' -> false
        b"\x00\xffyn",
        b"ynynynynyn",
        b"yynnyynnyynnyynn",
    ];

    for case in cases {
        let mut cbuf = case.to_vec();
        let mut rbuf = case.to_vec();
        let n = case.len();
        let cres = unsafe { (l.c)(cbuf.as_mut_ptr() as *mut c_char, n, 3, 0) };
        let rres = unsafe { (l.rust)(rbuf.as_mut_ptr() as *mut c_char, n, 3, 0) };

        assert_eq!(cres, rres, "C35 return mismatch for {case:02x?}");
        assert_eq!(cbuf, rbuf, "C35 buffer mismatch for {case:02x?}");

        // The C code turns each byte into a raw `_Bool`: 1 for y/Y, else 0.
        let expected: Vec<u8> = case
            .iter()
            .map(|&b| u8::from(b == b'y' || b == b'Y'))
            .collect();
        assert_eq!(
            cbuf, expected,
            "C35: the C reference itself did not rewrite {case:02x?} as expected"
        );
    }

    // And randomized.
    let mut rng = Rng::new(0xC35_0000);
    for _ in 0..20_000 {
        let len = 1 + rng.below(50);
        let input: Vec<u8> = (0..len)
            .map(|_| if rng.below(3) == 0 { rng.byte() } else { rng.yn_byte() })
            .collect();
        assert_same_full(&input, 3, 0, "C35-rand");
    }
}

/// C36: `param` is ignored by operation 3.
#[test]
fn c36_op3_param_ignored() {
    let mut rng = Rng::new(0xC36_0000);
    for _ in 0..2_000 {
        let len = 1 + rng.below(40);
        let input: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
        for param in INT_EDGE_VALUES {
            assert_same_full(&input, 3, param, "C36");
        }
    }
}

// ===========================================================================
// Cross-cutting
// ===========================================================================

/// C37: broad randomized fuzz over the whole parameter space.
#[test]
fn c37_full_random_fuzz() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);

    for _ in 0..200_000 {
        let len = rng.below(81); // 0..=80
        let bytes: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => rng.byte(),
                _ => rng.yn_byte(),
            })
            .collect();

        let op = rng.range(-4, 8) as i32;
        let param = rng.range(-4, 8) as i32;

        // Sometimes claim a shorter length than the buffer actually holds; never
        // a longer one (which the C code would read out of bounds for).
        let claimed = if rng.below(4) == 0 && len > 0 {
            rng.below(len + 1)
        } else {
            len
        };

        assert_same(&bytes, claimed, op, param, "C37");
    }
}

/// C37b: fuzz using the full `INT_EDGE_VALUES` set for op/param.
#[test]
fn c37b_edge_int_fuzz() {
    let mut rng = Rng::new(0xFEED_FACE);
    for _ in 0..3_000 {
        let len = rng.below(20);
        let bytes: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();
        for op in INT_EDGE_VALUES {
            for param in INT_EDGE_VALUES {
                assert_same(&bytes, len, op, param, "C37b");
            }
        }
    }
}

/// C38: statefulness — run operation 3 first (which rewrites the buffer to raw
/// 0/1 bytes), then feed the *rewritten* buffer back through every operation.
/// The rewritten bytes are `0x00`/`0x01`, neither of which parses as true.
#[test]
fn c38_sequential_calls_on_rewritten_buffer() {
    use std::ffi::c_char;
    let l = common::libs();
    let mut rng = Rng::new(0xC38_0000);

    for _ in 0..5_000 {
        let len = 1 + rng.below(40);
        let original: Vec<u8> = (0..len).map(|_| rng.yn_byte()).collect();

        let mut cbuf = original.clone();
        let mut rbuf = original.clone();

        // First pass: operation 3 rewrites both buffers.
        let c0 = unsafe { (l.c)(cbuf.as_mut_ptr() as *mut c_char, len, 3, 0) };
        let r0 = unsafe { (l.rust)(rbuf.as_mut_ptr() as *mut c_char, len, 3, 0) };
        assert_eq!(c0, r0, "C38 first pass mismatch for {original:02x?}");
        assert_eq!(cbuf, rbuf, "C38 first pass buffer mismatch for {original:02x?}");

        // Second pass: every operation on the rewritten bytes.
        for op in 0..4 {
            for param in 0..4 {
                let mut c2 = cbuf.clone();
                let mut r2 = rbuf.clone();
                let cres = unsafe { (l.c)(c2.as_mut_ptr() as *mut c_char, len, op, param) };
                let rres = unsafe { (l.rust)(r2.as_mut_ptr() as *mut c_char, len, op, param) };
                assert_eq!(cres, rres, "C38 second pass ret mismatch op={op} param={param}");
                assert_eq!(c2, r2, "C38 second pass buffer mismatch op={op} param={param}");
            }
        }
    }
}

/// C42: `length` far larger than the readable buffer, for the operations whose
/// access pattern is provably bounded independently of `length`.
///
/// * operations 0 and 1 only ever read indices 0, 1 and 2, so a 3-byte buffer is
///   enough no matter how huge `length` is.
/// * operation 2 clamps at `min(length, 32)`, so a 32-byte buffer is enough.
///
/// The C handles both perfectly well; a Rust wrapper that eagerly built a
/// `length`-sized slice would be unsound (and, with `length == usize::MAX`,
/// would trip a std debug assertion).  Operation 3 is deliberately excluded: it
/// genuinely reads and writes all `length` bytes, so over-claiming there is
/// out-of-bounds in the C too.
#[test]
fn c42_length_larger_than_buffer() {
    let huge = [
        3usize,
        4,
        32,
        33,
        1024,
        65_536,
        1 << 40,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ];

    // Operations 0 and 1: three readable bytes are all the C can touch.
    for bits in 0u32..8 {
        let three = pattern_to_bytes(bits, 3);
        for length in huge {
            for param in [0, 1, 2, 3, -1, 77] {
                let r0 = unsafe {
                    common::assert_same_overclaimed_length(&three, length, 0, param, "C42-op0")
                };
                let r1 = unsafe {
                    common::assert_same_overclaimed_length(&three, length, 1, param, "C42-op1")
                };
                // And they must agree with the honest, in-bounds call.
                assert_eq!(r0, assert_same_full(&three, 0, param, "C42-op0-ref"));
                assert_eq!(r1, assert_same_full(&three, 1, param, "C42-op1-ref"));
            }
        }
    }

    // Operation 2: thirty-two readable bytes are all the C can touch.
    let mut rng = Rng::new(0xC42_0000);
    for _ in 0..200 {
        let thirty_two: Vec<u8> = (0..32).map(|_| rng.yn_byte()).collect();
        for length in huge.iter().copied().filter(|&l| l >= 32) {
            let r = unsafe {
                common::assert_same_overclaimed_length(&thirty_two, length, 2, 0, "C42-op2")
            };
            assert_eq!(r, assert_same_full(&thirty_two, 2, 0, "C42-op2-ref"));
        }
    }

    // Unknown operations never dereference the pointer at all, so any `length`
    // with any buffer is fine.
    for op in [-1, 4, 5, i32::MIN, i32::MAX] {
        for length in huge {
            let r = unsafe {
                common::assert_same_overclaimed_length(b"", length, op, 0, "C42-bad-op")
            };
            assert_eq!(r, -3, "C42: op={op} length={length}");
        }
    }
}
