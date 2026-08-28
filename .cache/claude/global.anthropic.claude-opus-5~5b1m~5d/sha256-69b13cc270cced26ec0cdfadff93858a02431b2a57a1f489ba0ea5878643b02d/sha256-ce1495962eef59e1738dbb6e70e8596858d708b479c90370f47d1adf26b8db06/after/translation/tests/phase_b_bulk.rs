// Phase B — high-volume sweeps (CONFIGS.md rows C35..C39).
//
// These use `bulk_charinbuf`, which replays a whole batch through each library
// with fd 1 redirected once, so tens of thousands of inputs per row (including
// their full stdout) can be compared in well under a second.

mod support;

use support::*;

// C35 — dense sweep of `charinbuf` mode 0 across (and well past) the uint16
// boundary: every value in -70000..=70000.
#[test]
fn c35_mode0_dense_value_sweep() {
    let cases: Vec<(i32, i32, i32, i32)> = (-70000..=70000).map(|v| (0, v, 0, 0)).collect();
    assert_eq!(cases.len(), 140_001);
    bulk_charinbuf(&cases);
}

// C36 — strided sweep of mode 0 over the entire i32 domain.
#[test]
fn c36_mode0_strided_full_domain() {
    let mut cases = Vec::new();
    let mut v: i64 = i32::MIN as i64;
    while v <= i32::MAX as i64 {
        cases.push((0, v as i32, 0, 0));
        v += 104_729; // prime stride -> 41k samples covering the whole range
    }
    cases.push((0, i32::MAX, 0, 0));
    assert!(cases.len() > 40_000);
    bulk_charinbuf(&cases);
}

// C37 — mode 3 (the arithmetic pipeline) with 50k randomized triples,
// deliberately biased towards overflow-inducing operands.
#[test]
fn c37_mode3_bulk_random() {
    let mut rng = Rng::new(SEED ^ 31);
    let cases: Vec<(i32, i32, i32, i32)> = (0..50_000)
        .map(|_| {
            (
                3,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            )
        })
        .collect();
    bulk_charinbuf(&cases);
}

// C38 — every mode (valid and invalid) with 50k randomized argument triples.
#[test]
fn c38_all_modes_bulk_random() {
    let mut rng = Rng::new(SEED ^ 32);
    let cases: Vec<(i32, i32, i32, i32)> = (0..50_000)
        .map(|_| {
            let mode = match rng.below(3) {
                0 => rng.range_i32(-2, 6),
                1 => rng.next_i32(),
                _ => rng.range_i32(0, 4),
            };
            (
                mode,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            )
        })
        .collect();
    bulk_charinbuf(&cases);
}

// C39 — mode 3 sequences: because `charinbuf` resets the counter on entry each
// call is independent, but interleaving the other modes (which do *not* touch
// the counter after the reset) plus direct op calls is not, so run a long mixed
// batch as a single stream.
#[test]
fn c39_mixed_mode_stream() {
    let mut rng = Rng::new(SEED ^ 33);
    let mut cases = Vec::new();
    for _ in 0..20_000 {
        let mode = rng.range_i32(0, 4);
        cases.push((
            mode,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ));
    }
    bulk_charinbuf(&cases);

    // Followed by direct counter operations, so the state left behind by the
    // last `charinbuf` call is observed too.
    let (c, r) = both();
    for _ in 0..1000 {
        diff_op(rng.below(4), rng.interesting_i32());
    }
    unsafe {
        assert_eq!(
            (c.increment_counter)(0),
            (r.increment_counter)(0),
            "counter diverged after the mixed stream"
        );
    }
}

// C40 — validate_uint16_range: exhaustive over the boundary region and a
// strided sweep over the whole i32 domain (no output, so it is cheap).
#[test]
fn c40_validate_exhaustive_boundary_and_strided() {
    for v in -80_000i32..=80_000 {
        diff_validate(v);
    }
    let mut v: i64 = i32::MIN as i64;
    while v <= i32::MAX as i64 {
        diff_validate(v as i32);
        v += 3_571; // ~1.2M samples
    }
    diff_validate(i32::MAX);
}

// C41 — find_char_in_buffer: exhaustive (buffer position x target) matrix.
#[test]
fn c41_find_exhaustive_matrix() {
    // For every position p and every byte value b: buffer with b at p only.
    let mut buf = vec![b'.'; 64];
    for p in 0..64usize {
        for b in 0u16..256 {
            buf.iter_mut().for_each(|x| *x = b'.');
            buf[p] = b as u8;
            for size in [0usize, 1, p, p + 1, 64] {
                diff_find(&buf, size, b as u8);
            }
        }
    }
}

// C42 — counter operations: exhaustive over a dense operand grid from several
// seed states.
#[test]
fn c42_counter_operand_grid() {
    for seed in [0i32, 1, -1, 7, -7, 65535, i32::MAX, i32::MIN, i32::MAX / 2] {
        for op in 0..4usize {
            for delta in -600i32..=600 {
                seed_counters(seed);
                diff_op(op, delta);
                diff_op(op, delta.wrapping_mul(7));
            }
        }
    }
}
