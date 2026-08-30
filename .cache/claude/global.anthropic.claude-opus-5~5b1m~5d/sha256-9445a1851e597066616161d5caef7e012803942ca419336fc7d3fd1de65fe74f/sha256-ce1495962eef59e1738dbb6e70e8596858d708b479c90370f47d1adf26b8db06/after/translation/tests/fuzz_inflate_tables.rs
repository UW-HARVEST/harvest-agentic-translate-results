//! Randomized differential fuzzing of `cp_inflate` with the exported lookup
//! tables retuned on every call. This is what reaches rows A4/A7/A8/A9 of
//! `ERRORS.md`.

mod fuzzcommon;
mod harness;

use fuzzcommon::*;
use harness::*;

/// Rounds of `250` cases each. Override with `FUZZ_ROUNDS=<n>` for a longer run.
fn rounds() -> u32 {
    std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
}

/// The same, but with a randomly retuned table on every call. This is what
/// reaches rows A4/A7/A8/A9 of `ERRORS.md`.
#[test]
fn fuzz_cp_inflate_with_table_mutations() {
    let pair = load_pair();
    let mut rng = Rng::new(0xBEEF);
    let base = corpus(&mut rng);
    let mut total_compared = 0usize;
    let mut total_dropped = 0usize;

    for round in 0..rounds() {
        let mut cases = Vec::new();
        for i in 0..250 {
            let b = &base[rng.below(base.len() as u32) as usize];
            let m = if rng.bool() {
                b.clone()
            } else {
                mutate(&mut rng, b)
            };
            let nmut = rng.range(1, 3) as usize;
            let muts: Vec<Mutation> = (0..nmut)
                .map(|_| {
                    let table = Table::ALL[rng.below(6) as usize];
                    let mut off = rng.below(table.byte_len() as u32) as usize;
                    // `cp_len_base`/`cp_dist_base` are uint32; setting a high
                    // byte makes `int length` / `int backwards_distance`
                    // negative, and the C's `while (length--)` then writes
                    // gigabytes before faulting (or `memset` gets a huge
                    // size_t). That is undefined *and* untestable, so only the
                    // low byte of those entries is retuned.
                    if matches!(table, Table::LenBase | Table::DistBase) {
                        off &= !3;
                    }
                    // `cp_permutation_order[i] >= 64` makes the C write past
                    // `rbp` in `cp_dynamic` (saved frame pointer / return
                    // address / its caller's frame), which is undefined and
                    // layout-dependent; 0..=63 stays inside the frame and is
                    // modelled exactly.
                    let val = if table == Table::PermutationOrder {
                        rng.below(64) as u8
                    } else {
                        rng.byte()
                    };
                    Mutation { table, off, val }
                })
                .collect();
            let out_bytes = match rng.below(3) {
                0 => rng.range(1, 40) as i32,
                1 => rng.range(1, 500) as i32,
                _ => 4096,
            };
            cases.push(
                fuzz_inflate(
                    format!("mut r{round} #{i}"),
                    m,
                    rng.below(4) as usize,
                    out_bytes,
                )
                .with_mutations(muts),
            );
        }
        let rep = fuzz_same(&pair, &cases);
        eprintln!("  round {round}: compared {} dropped {}", rep.compared, rep.dropped);
        total_compared += rep.compared;
        total_dropped += rep.dropped;
    }
    eprintln!(
        "fuzz_cp_inflate_with_table_mutations: compared {total_compared}, dropped {total_dropped}"
    );
    assert!(total_compared > 125, "too few deterministic cases");
}

