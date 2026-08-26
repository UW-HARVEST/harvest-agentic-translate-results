//! Phase B — differential tests for the lowest-level public entry point,
//! `perform_expensive_operations()` operating on the exported `array` object
//! (CONFIGS.md rows 2–10).
//!
//! The C loop body is elementwise independent, so one call over the 262 144-slot
//! array exercises 262 144 *distinct* inputs at once; the tests exploit that to
//! cover millions of values in a handful of calls.
//!
//! Everything is compared after every call, over the full 1 MB of output, and
//! against the C compiled at both `-O0` and `-O2` (the arithmetic depends on
//! signed overflow, negative `<<`, negative `>>` and truncating `/` and `%`).

mod common;

use common::{diff_perform, pairs, Rng, ARRAY_SIZE};

/// Values that make the C body take its interesting paths:
/// * `x * 3 + 7` overflowing (|x| around `INT_MAX / 3`),
/// * `x << 1` overflowing / shifting a negative,
/// * `x >> 3` on negatives (arithmetic shift),
/// * `x / 2` truncating toward zero in both directions,
/// * every `x % 7` residue and its sign,
/// * the transformation's only fixed point.
fn edge_values() -> Vec<i32> {
    let mut v: Vec<i32> = Vec::new();
    let anchors: [i64; 30] = [
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        8,
        -8,
        i32::MAX as i64,
        i32::MIN as i64,
        (i32::MAX as i64) / 3,
        (i32::MIN as i64) / 3,
        715_827_882, // (INT_MAX - 7) / 3, the x*3+7 overflow threshold
        -715_827_882,
        1 << 30,
        -(1 << 30),
        -848_907_408, // fixed point of the 100-step transformation
        0x5555_5555,
        -0x5555_5555,
        0x3333_3333,
        -0x3333_3333,
        0x0F0F_0F0F,
        -0x0F0F_0F0F,
        0x7FFF_FFF8,
        -0x7FFF_FFF8,
        1_073_741_824, // 2^30, where x*3 overflows hardest
        -1_073_741_824,
        1_431_655_765, // (2^32-1)/3
        -1_431_655_765,
    ];
    for a in anchors {
        for d in -8i64..=8 {
            v.push(a.wrapping_add(d) as i32);
        }
    }
    // Powers of two and their neighbours (every bit position).
    for bit in 0..32u32 {
        let p = 1i64 << bit;
        for d in [-1i64, 0, 1] {
            v.push((p + d) as i32);
            v.push((-p + d) as i32);
        }
    }
    // Every residue class mod 7 and mod 2, positive and negative.
    for r in 0..7i64 {
        for m in [1i64, -1] {
            v.push((m * (r + 700_000)) as i32);
            v.push((m * (r + 700_001)) as i32);
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Tiles `values` across a full-size array (padding with a deterministic
/// pseudo-random tail so the whole array is meaningful input).
fn tile(values: &[i32], seed: u64) -> Vec<i32> {
    let mut rng = Rng::new(seed);
    let mut a = Vec::with_capacity(ARRAY_SIZE);
    while a.len() < ARRAY_SIZE {
        for &v in values {
            if a.len() == ARRAY_SIZE {
                break;
            }
            a.push(v);
        }
        if a.len() < ARRAY_SIZE && values.is_empty() {
            break;
        }
        if a.len() < ARRAY_SIZE {
            // separate repetitions with random values
            for _ in 0..3 {
                if a.len() == ARRAY_SIZE {
                    break;
                }
                a.push(rng.next_i32());
            }
        }
    }
    a
}

/// CONFIGS.md rows 2 & 3 — the `.bss` initial state, and compounding calls.
#[test]
fn all_zeros() {
    let p = pairs();
    let input = vec![0i32; ARRAY_SIZE];
    diff_perform("all zeros (.bss initial state), 10 calls", &p, &input, 10);
}

/// CONFIGS.md row 4 — genuinely uniform arrays (one value everywhere) for the
/// most extreme values, plus a tiled sweep of every edge value.
#[test]
fn uniform_value_sweep() {
    let p = pairs();
    for v in [
        0i32,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        -848_907_408,
        715_827_882,
        -715_827_882,
    ] {
        let input = vec![v; ARRAY_SIZE];
        diff_perform(&format!("uniform array of {v}"), &p, &input, 1);
    }

    let edges = edge_values();
    assert!(edges.len() > 400, "edge value list looks too small");
    let input = tile(&edges, 0x5EED_0001);
    diff_perform(
        &format!("tiled sweep of {} edge values", edges.len()),
        &p,
        &input,
        2,
    );
}

/// CONFIGS.md row 5 — uniform-random `i32` over the full signed range.
#[test]
fn random_full_range() {
    let p = pairs();
    for round in 0..8u64 {
        let mut rng = Rng::new(0x5EED_1000 + round);
        let input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
        diff_perform(
            &format!("uniform random i32, round {round}"),
            &p,
            &input,
            1,
        );
    }
}

/// CONFIGS.md row 6 — random input with state compounding over 3 calls.
#[test]
fn random_full_range_repeated() {
    let p = pairs();
    for round in 0..3u64 {
        let mut rng = Rng::new(0x5EED_2000 + round);
        let input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
        diff_perform(
            &format!("uniform random i32 x3 calls, round {round}"),
            &p,
            &input,
            3,
        );
    }
}

/// CONFIGS.md row 7 — the only shape the real program produces: `rand()` output
/// in `[0, 2^31)`.
#[test]
fn rand_shaped_values() {
    let p = pairs();
    for round in 0..2u64 {
        let mut rng = Rng::new(0x5EED_3000 + round);
        let input: Vec<i32> = (0..ARRAY_SIZE)
            .map(|_| (rng.next_u32() >> 1) as i32)
            .collect();
        diff_perform(&format!("rand()-shaped values, round {round}"), &p, &input, 2);
    }
}

/// CONFIGS.md row 8 — extremes planted at the loop's boundary indices.
#[test]
fn boundary_indices() {
    let p = pairs();
    let extremes = [
        i32::MIN,
        i32::MAX,
        0,
        1,
        -1,
        7,
        -7,
        -848_907_408,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    for (round, chunk) in extremes.chunks(4).enumerate() {
        let mut rng = Rng::new(0x5EED_4000 + round as u64);
        let mut input: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect();
        let idx = [0usize, 1, ARRAY_SIZE - 2, ARRAY_SIZE - 1];
        for (k, &v) in chunk.iter().enumerate() {
            input[idx[k]] = v;
        }
        diff_perform(
            &format!("boundary indices with {chunk:?}"),
            &p,
            &input,
            2,
        );
    }
}

/// CONFIGS.md row 9 — every `/ 2` rounding direction and `% 7` residue/sign.
#[test]
fn division_and_modulo_shapes() {
    let p = pairs();
    let mut values = Vec::new();
    for base in [0i64, 1, 6, 7, 8, 13, 14, 1_000_000, -1, -6, -7, -8, -13, -14, -1_000_000] {
        for k in 0..14i64 {
            values.push((base * 7 + k) as i32);
            values.push((base * 7 - k) as i32);
            values.push((base * 2 + k) as i32);
        }
    }
    // and the same shapes near the extremes, where / and % interact with overflow
    for base in [i32::MIN as i64, i32::MAX as i64] {
        for k in 0..64i64 {
            values.push(base.wrapping_add(k) as i32);
            values.push(base.wrapping_sub(k) as i32);
        }
    }
    values.sort_unstable();
    values.dedup();
    let input = tile(&values, 0x5EED_5000);
    diff_perform(
        &format!("division/modulo shapes ({} distinct values)", values.len()),
        &p,
        &input,
        2,
    );
}

/// CONFIGS.md row 10 — dense contiguous ranges plus a strided sweep of the whole
/// `i32` domain (4 × 262 144 = ~1M distinct values, spaced 16 384 apart).
#[test]
fn exhaustive_low_and_extreme_ranges() {
    let p = pairs();

    // -131072 ..= 131071 : a fully contiguous block, exactly ARRAY_SIZE wide.
    let input: Vec<i32> = (0..ARRAY_SIZE)
        .map(|i| i as i32 - (ARRAY_SIZE as i32) / 2)
        .collect();
    diff_perform("contiguous range -131072..=131071", &p, &input, 2);

    // The two ends of the domain, 131072 values each.
    let mut input: Vec<i32> = Vec::with_capacity(ARRAY_SIZE);
    for i in 0..ARRAY_SIZE / 2 {
        input.push((i32::MIN as i64 + i as i64) as i32);
    }
    for i in 0..ARRAY_SIZE / 2 {
        input.push((i32::MAX as i64 - i as i64) as i32);
    }
    diff_perform("INT_MIN.. and ..INT_MAX blocks", &p, &input, 2);

    // Strided sweep: covers the entire u32 space at stride 16384, 4 offsets.
    for offset in [0u32, 4093, 8191, 12_289] {
        let input: Vec<i32> = (0..ARRAY_SIZE)
            .map(|i| ((i as u32).wrapping_mul(16_384).wrapping_add(offset)) as i32)
            .collect();
        diff_perform(&format!("strided full-domain sweep, offset {offset}"), &p, &input, 1);
    }
}

/// Harness sanity + regression anchors.
///
/// Guards against a false pass in which the tests would compare two buffers that
/// `perform_expensive_operations` never touched (wrong `array` symbol, wrong
/// pointer, no-op call). The expected values were read out of the **C** shared
/// object (both `-O0` and `-O2` agree on them):
///
/// ```text
/// f(          0) =  -626538949      f(-2147483648) =  -756415197
/// f(          1) = -1057168239      f( 2147483647) =  -627633746
/// f(         -1) =  -626500583      f( -848907408) =  -848907408  (fixed point)
/// f(          7) =  -822186310      f(f(0))        =  -890868442
/// f(         -7) =  -626277382      f(f(1))        =  -954282591
/// ```
#[test]
fn transformation_anchors() {
    let p = pairs();
    let _g = common::array_guard();

    let inputs: [i32; 8] = [0, 1, -1, 7, -7, i32::MIN, i32::MAX, -848_907_408];
    let expected: [i32; 8] = [
        -626_538_949,
        -1_057_168_239,
        -626_500_583,
        -822_186_310,
        -626_277_382,
        -756_415_197,
        -627_633_746,
        -848_907_408, // the transformation's fixed point
    ];

    let mut input = vec![0i32; ARRAY_SIZE];
    input[..8].copy_from_slice(&inputs);

    for imp in p.c.iter().chain(std::iter::once(&p.rust)) {
        imp.set_array(&input);
        imp.perform();
        let out = imp.get_array();
        for k in 0..8 {
            assert_eq!(
                out[k], expected[k],
                "{}: f({}) = {} but the C reference says {}",
                imp.name, inputs[k], out[k], expected[k]
            );
        }
        // the array really was modified (element 8.. was 0 -> f(0))
        assert_eq!(out[8], -626_538_949, "{}: tail element untouched?", imp.name);
        assert_ne!(out[0], inputs[0], "{}: perform() did nothing", imp.name);

        // a second application composes
        imp.perform();
        let out2 = imp.get_array();
        assert_eq!(out2[0], -890_868_442, "{}: f(f(0))", imp.name);
        assert_eq!(out2[1], -954_282_591, "{}: f(f(1))", imp.name);
        assert_eq!(
            out2[7], -848_907_408,
            "{}: the fixed point must stay put",
            imp.name
        );
    }
}
