// Phase B -- valid-path differential tests, one test per CONFIGS.md row.
//
// Everything is driven through the `.so` exports of BOTH libraries: the
// `array` data symbol is written/read via `dlsym`, and
// `perform_expensive_operations` / `long_exec` are called via `dlsym`.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Row 1 -- the `array` data symbol's ABI shape.
// ---------------------------------------------------------------------------
#[test]
fn row01_array_symbol_abi_shape() {
    let h = harness();
    for t in h.all() {
        let base = t.array_ptr();
        assert!(!base.is_null(), "[{}] array is NULL", t.name);
        assert_eq!(
            base as usize % std::mem::align_of::<c_int>(),
            0,
            "[{}] array is misaligned for int",
            t.name
        );
        // Element stride must be sizeof(int) == 4: writing element i must not
        // disturb element i+1.
        unsafe {
            for i in [0usize, 1, 7, 8, N / 2, N - 2, N - 1] {
                std::ptr::write(base.add(i), 0);
            }
            std::ptr::write(base.add(N - 2), -1);
            assert_eq!(std::ptr::read(base.add(N - 1)), 0, "[{}] stride", t.name);
            std::ptr::write(base.add(N - 2), 0);
        }
    }
    // Both libraries must expose *independent* storage.
    assert_ne!(
        h.c.array_ptr(),
        h.rust[0].array_ptr(),
        "the two libraries share one `array`; the differential test would be vacuous"
    );
}

// ---------------------------------------------------------------------------
// Row 2 -- pristine load state: array untouched (all zeros) -> 1 call.
// ---------------------------------------------------------------------------
#[test]
fn row02_pristine_zero_array() {
    let h = harness();
    let input = vec![0 as c_int; N];
    diff_peo(&h, "row02 all-zero (.bss) state", &input, 1);
    // Ground truth from gcc: 0 is NOT a fixed point (step(0) == -3); one call
    // maps every zero element to CHURN_OF_ZERO.
    assert!(
        h.c.read_array().iter().all(|&v| v == CHURN_OF_ZERO),
        "expectation about the C behaviour on an all-zero array is stale"
    );
}

// ---------------------------------------------------------------------------
// Row 3 -- uniform array, swept over every magnitude class.
// ---------------------------------------------------------------------------
#[test]
fn row03_uniform_magnitude_classes() {
    let h = harness();
    let values: &[c_int] = &[
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        8,
        -8,
        1 << 30,
        -(1 << 30),
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MIN,
        c_int::MIN + 1,
    ];
    let mut input = vec![0 as c_int; N];
    for &v in values {
        fill_uniform(&mut input, v);
        diff_peo(&h, &format!("row03 uniform value {v}"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- a single non-zero element at index 0.
// ---------------------------------------------------------------------------
#[test]
fn row04_single_element_at_zero() {
    let h = harness();
    let mut input = vec![0 as c_int; N];
    input[0] = c_int::MIN;
    diff_peo(&h, "row04 single INT_MIN at index 0", &input, 1);
}

// ---------------------------------------------------------------------------
// Row 5 -- single non-zero element at every 8-lane position.
//
// The Rust worker batches `LANES = 8` elements per inner-loop trip while the C
// walks one element at a time. Placing the only interesting value at each lane
// offset proves the batching is a faithful re-association.
// ---------------------------------------------------------------------------
#[test]
fn row05_single_element_every_lane_offset() {
    let h = harness();
    let mut rng = Rng::new(0x05_05_05);
    let mut positions: Vec<usize> = (0..8).collect();
    positions.push(N - 1);
    for pos in positions {
        let mut input = vec![0 as c_int; N];
        input[pos] = rng.next_i32() | 1;
        diff_peo(
            &h,
            &format!("row05 single value at index {pos} (lane {})", pos % 8),
            &input,
            1,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 6 -- randomised non-negative inputs (the shape `rand()` produces).
// ---------------------------------------------------------------------------
#[test]
fn row06_random_nonnegative() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0006);
    let mut input = vec![0 as c_int; N];
    for trial in 0..6 {
        for slot in input.iter_mut() {
            *slot = rng.next_nonneg();
        }
        assert!(input.iter().all(|&v| v >= 0));
        diff_peo(&h, &format!("row06 non-negative trial {trial}"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 7 -- randomised negative inputs (sar / negative div / negative rem).
// ---------------------------------------------------------------------------
#[test]
fn row07_random_negative() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0007);
    let mut input = vec![0 as c_int; N];
    for trial in 0..6 {
        for slot in input.iter_mut() {
            *slot = rng.next_neg();
        }
        assert!(input.iter().all(|&v| v < 0));
        diff_peo(&h, &format!("row07 negative trial {trial}"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 8 -- randomised full-range inputs, mixed signs within each 8-lane chunk.
// ---------------------------------------------------------------------------
#[test]
fn row08_random_full_range() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0008);
    let mut input = vec![0 as c_int; N];
    for trial in 0..8 {
        for slot in input.iter_mut() {
            *slot = rng.next_i32();
        }
        diff_peo(&h, &format!("row08 full-range trial {trial}"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 9 -- inputs restricted to the 13 residue classes of `% 7`.
// ---------------------------------------------------------------------------
#[test]
fn row09_residue_classes_mod_7() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0009);
    let mut input = vec![0 as c_int; N];
    for trial in 0..4 {
        for slot in input.iter_mut() {
            // Build a value with a chosen remainder, keeping both signs.
            let want_rem = (rng.below(13) as c_int) - 6; // -6 ..= 6
            let mut q = (rng.next_u32() >> 4) as c_int;
            if want_rem < 0 {
                q = -q;
            }
            let v = q.wrapping_mul(7).wrapping_add(want_rem);
            *slot = v;
        }
        // Sanity: every residue class actually shows up.
        let mut seen = [false; 13];
        for &v in input.iter() {
            seen[(v % 7 + 6) as usize] = true;
        }
        assert!(
            seen.iter().all(|&s| s),
            "row09 trial {trial} did not cover all 13 residue classes"
        );
        diff_peo(&h, &format!("row09 residue trial {trial}"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 10 -- overflow-triggering extremes at random positions.
// ---------------------------------------------------------------------------
#[test]
fn row10_overflow_extremes() {
    let h = harness();
    let extremes: &[c_int] = &[
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MAX - 2,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN + 2,
        1 << 30,
        -(1 << 30),
        (1 << 30) + 1,
        -((1 << 30) + 1),
        c_int::MAX / 3,
        c_int::MAX / 3 + 1,
        c_int::MAX / 3 - 1,
        c_int::MIN / 3,
        c_int::MIN / 3 - 1,
        c_int::MIN / 3 + 1,
        1 << 29,
        -(1 << 29),
    ];
    let h_ref = &h;
    let mut rng = Rng::new(0xA110_0010);
    let mut input = vec![0 as c_int; N];
    for trial in 0..4 {
        for slot in input.iter_mut() {
            *slot = extremes[rng.below(extremes.len())];
        }
        diff_peo(h_ref, &format!("row10 extremes trial {trial}"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 11 -- boundary-value stripe cycling the full edge-case table.
// ---------------------------------------------------------------------------
#[test]
fn row11_edge_value_stripe() {
    let h = harness();
    let mut input = vec![0 as c_int; N];
    // Two different phase offsets so the 8-lane groups get different mixes.
    // EDGE_VALUES.len() is deliberately not a multiple of 8.
    assert_ne!(EDGE_VALUES.len() % 8, 0);
    for offset in [0usize, 3] {
        fill_cycle(&mut input, EDGE_VALUES, offset);
        diff_peo(&h, &format!("row11 edge stripe offset {offset}"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 12 -- composition: compare after EVERY one of n back-to-back calls.
// ---------------------------------------------------------------------------
#[test]
fn row12_composition_short() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0012);
    let mut input = vec![0 as c_int; N];
    for &calls in &[0usize, 1, 2, 3, 5, 8, 13] {
        for slot in input.iter_mut() {
            *slot = rng.next_i32();
        }
        diff_peo(&h, &format!("row12 composition of {calls} call(s)"), &input, calls);
    }
}

// ---------------------------------------------------------------------------
// Row 13 -- long composition (40 calls == f^4000), compared after each call.
// ---------------------------------------------------------------------------
#[test]
fn row13_composition_long() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0013);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32();
    }
    diff_peo(&h, "row13 long composition", &input, 40);
    // Report whether the orbit has settled -- this is the regime the real
    // `ITERATIONS = 2000` run spends essentially all of its time in.
    let before = h.c.read_array();
    h.c.peo();
    let after = h.c.read_array();
    println!(
        "row13: after 40 calls, {} / {N} elements still change on the next call",
        (0..N).filter(|&i| before[i] != after[i]).count()
    );
}

// ---------------------------------------------------------------------------
// Row 14 -- each `.so` owns a private `array`; neither reads the other's.
// ---------------------------------------------------------------------------
#[test]
fn row14_cross_library_state_independence() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0014);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32() | 1; // never 0, so "untouched" is distinguishable
    }
    let zeros = vec![0 as c_int; N];

    // Give C real data, leave every Rust array zeroed, then run only C.
    h.c.write_array(&input);
    for t in &h.rust {
        t.write_array(&zeros);
    }
    h.c.peo();
    for t in &h.rust {
        assert!(
            t.read_array().iter().all(|&v| v == 0),
            "[{}] array changed although only the C worker ran -- the two \
             libraries are sharing state",
            t.name
        );
    }

    // Mirror image: give Rust the data, leave C zeroed, run only Rust.
    let c_expected = h.c.read_array();
    for t in &h.rust {
        t.write_array(&input);
        t.peo();
    }
    assert_eq!(
        h.c.read_array(),
        c_expected,
        "the C array changed although only a Rust worker ran"
    );
    // And the results must still agree with what C produced from `input`.
    for t in &h.rust {
        assert!(
            t.read_array() == c_expected,
            "[{}] independent run over the same input diverged from C",
            t.name
        );
    }
}

// ---------------------------------------------------------------------------
// Row 15 -- one-past-the-end guard: nothing outside [0, ARRAY_SIZE) is written.
// ---------------------------------------------------------------------------
#[test]
fn row15_no_write_past_end() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0015);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32();
    }
    // Snapshot the 256 bytes that follow `array` in each library, run the
    // worker, and require them to be unchanged.
    let mut snaps = Vec::new();
    for t in h.all() {
        t.write_array(&input);
        let tail = unsafe {
            let past = t.array_ptr().add(N) as *const u8;
            std::slice::from_raw_parts(past, 256).to_vec()
        };
        snaps.push(tail);
    }
    for t in h.all() {
        t.peo();
    }
    for (t, before) in h.all().zip(snaps.iter()) {
        let after = unsafe {
            let past = t.array_ptr().add(N) as *const u8;
            std::slice::from_raw_parts(past, 256).to_vec()
        };
        assert_eq!(
            before, &after,
            "[{}] perform_expensive_operations wrote past the end of `array`",
            t.name
        );
    }
    // Also check the last in-bounds element really was processed.
    let out = h.c.read_array();
    assert_ne!(
        out[N - 1],
        input[N - 1],
        "expectation stale: element ARRAY_SIZE-1 was not transformed"
    );
}

// ---------------------------------------------------------------------------
// Row 16 -- the seeding step's distribution, for many seeds.
//
// `long_exec` internally does `srand(seed); for(i) array[i] = rand();`. That
// exact stream (produced here by the very same libc both `.so`s import -- see
// `symbols.rs::both_libraries_import_the_same_libc_prng`) is fed to both
// workers, so the seed-dependent input shape is covered without paying for a
// 2000-iteration run.
// ---------------------------------------------------------------------------
#[test]
fn row16_seed_derived_inputs() {
    let h = harness();
    let mut input = vec![0 as c_int; N];
    for seed in [0u32, 1, 2, 42, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF] {
        libc_rand_fill(seed, &mut input);
        assert!(
            input.iter().all(|&v| v >= 0),
            "glibc rand() returned a negative value; assumption stale"
        );
        diff_peo(&h, &format!("row16 seed {seed:#010x} stream"), &input, 1);
    }
}

// ---------------------------------------------------------------------------
// Row 19 -- the wrapper's own glue: xor fold order and `%d` formatting.
//
// `long_exec` folds with `xor_result ^= array[i]` and prints with
// `printf("%d\n", xor_result)`. Both are reproduced here against real worker
// output (including a negative fold result, where `%d` formatting matters).
// ---------------------------------------------------------------------------
#[test]
fn row19_fold_and_format_glue() {
    let h = harness();
    let mut rng = Rng::new(0xA110_0019);
    let mut input = vec![0 as c_int; N];
    for slot in input.iter_mut() {
        *slot = rng.next_i32();
    }
    diff_peo(&h, "row19 fold input", &input, 2);

    let c = h.c.read_array();
    let fold = xor_fold(&c);
    for t in &h.rust {
        assert_eq!(
            xor_fold(&t.read_array()),
            fold,
            "[{}] xor fold differs from C",
            t.name
        );
    }
    // `%d` of an int is Rust's Display for i32, including for negatives.
    let printed = format!("{fold}\n");
    assert!(printed.ends_with('\n'));
    println!("row19: fold = {fold} (negative: {})", fold < 0);

    // Exercise the formatting of a deliberately negative fold too.
    let mut neg = vec![0 as c_int; N];
    neg[0] = c_int::MIN;
    assert_eq!(xor_fold(&neg), c_int::MIN);
    assert_eq!(format!("{}", xor_fold(&neg)), "-2147483648");
}

// ---------------------------------------------------------------------------
// Extended soak: many more randomised batches across CONFIGS.md rows 6-11.
//
// Each `perform_expensive_operations` call transforms 262144 independent
// starting values, so N batches cover N * 262144 distinct inputs. This is the
// property-style breadth backing the row check-offs; it is `#[ignore]`d only
// because of its runtime (~0.26 s per batch per library), not because it is
// optional. Run with:
//     cargo test --offline --test valid_paths -- --ignored soak
// ---------------------------------------------------------------------------
#[test]
#[ignore = "extended soak (~2 minutes); breadth backup for rows 6-11"]
fn soak_randomised_batches() {
    let h = harness();
    let mut rng = Rng::new(0x50AC_5EED);
    let mut input = vec![0 as c_int; N];
    let batches: usize = std::env::var("LONG_SOAK_BATCHES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);

    for batch in 0..batches {
        // Rotate through the distribution families so every shape gets many
        // independent trials, mixing families inside single 8-lane chunks.
        match batch % 6 {
            0 => input.iter_mut().for_each(|s| *s = rng.next_i32()),
            1 => input.iter_mut().for_each(|s| *s = rng.next_nonneg()),
            2 => input.iter_mut().for_each(|s| *s = rng.next_neg()),
            3 => input
                .iter_mut()
                .for_each(|s| *s = EDGE_VALUES[rng.below(EDGE_VALUES.len())]),
            4 => input.iter_mut().for_each(|s| {
                // small magnitudes, both signs -- exercises x/2 truncation and
                // x%7 sign near zero
                let v = (rng.below(64) as c_int) - 32;
                *s = v;
            }),
            _ => input.iter_mut().for_each(|s| {
                // half edge cases, half random: mixed lanes
                *s = if rng.next_u32() & 1 == 0 {
                    EDGE_VALUES[rng.below(EDGE_VALUES.len())]
                } else {
                    rng.next_i32()
                }
            }),
        }
        diff_peo(&h, &format!("soak batch {batch} (family {})", batch % 6), &input, 1);
    }
    println!(
        "soak: {batches} batches x {N} independent starting values = {} inputs compared",
        batches * N
    );
}
