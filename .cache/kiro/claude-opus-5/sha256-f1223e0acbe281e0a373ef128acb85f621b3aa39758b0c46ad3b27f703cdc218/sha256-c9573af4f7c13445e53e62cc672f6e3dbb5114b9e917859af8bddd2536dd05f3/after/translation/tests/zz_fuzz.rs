//! Heavy randomized fuzz across the whole exported surface, run against both
//! `.so`s in lock-step. This is the independent cross-check on top of the
//! row-by-row Phases B and C: it does not know which configurations "matter",
//! it just hammers everything.
//!
//! Ignored by default (it runs for tens of seconds). Run with:
//! `cargo test --release --test zz_fuzz -- --ignored --nocapture`

mod common;
use common::*;

#[test]
#[ignore = "long-running stress test"]
fn fuzz_everything() {
    let iters: u64 = std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);

    // A pool that mixes ordinary values with every awkward FP bit pattern.
    let mut pool: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, 1e300, -1e300, 1e-300,
        f64::MAX, f64::MIN, f64::MIN_POSITIVE, f64::INFINITY, f64::NEG_INFINITY,
        f64::NAN, -f64::NAN, f64::from_bits(1),
    ];
    for payload in [1u64, 7, 0x0000_0000_00FF_FFFF, 0x0007_FFFF_FFFF_FFFF] {
        for q in [0u64, 0x0008_0000_0000_0000] {
            for s in [0u64, 1 << 63] {
                let d = f64::from_bits(s | 0x7FF0_0000_0000_0000 | q | payload);
                if d.is_nan() {
                    pool.push(d);
                }
            }
        }
    }

    let mut rng = Rng::new(SEED ^ 0xF0F0);
    let mut calls: u64 = 0;

    for round in 0..iters {
        let p = Pair::fresh();
        // `next_id` keeps parent_id < id, so the id-graph stays acyclic and
        // calculate_subtree_sum terminates in both libraries.
        let mut next_id: i32 = 1;

        for _ in 0..600 {
            match rng.below(9) {
                0 | 1 => {
                    let parent = if next_id == 1 { -1 } else { rng.range_i32(-2, next_id - 1) };
                    let nlen = rng.below(70) as usize;
                    let name = rng.bytes(nlen, 0x00, 0xFF);
                    let value = if rng.below(3) == 0 {
                        pool[rng.below(pool.len() as u64) as usize]
                    } else {
                        rng.next_f64_bits()
                    };
                    if p.add_node(next_id, parent, &name, value) >= 0 {
                        next_id += 1;
                    }
                }
                2 => {
                    p.find_node_by_id(if rng.below(2) == 0 {
                        rng.range_i32(-3, next_id + 3)
                    } else {
                        rng.next_i32()
                    });
                }
                3 => {
                    p.get_children_count(if rng.below(2) == 0 {
                        rng.range_i32(-3, next_id + 3)
                    } else {
                        rng.next_i32()
                    });
                }
                4 => {
                    p.calculate_subtree_sum(rng.range_i32(-3, next_id + 3));
                }
                5 => {
                    let nlen = rng.below(120) as usize;
                    let s = rng.bytes(nlen, 0x01, 0xFF);
                    p.process_string(&s);
                }
                6 => {
                    let d = match rng.below(3) {
                        0 => pool[rng.below(pool.len() as u64) as usize],
                        1 => rng.next_f64_bits(),
                        _ => rng.next_in_int_range(),
                    };
                    p.safe_double_to_int(d);
                }
                7 => {
                    let id = rng.range_i32(1, next_id.max(1));
                    if p.find_node_by_id(id).is_some() {
                        match rng.below(3) {
                            0 => p.set_active(id, 0),
                            1 => p.set_active(id, rng.next_i32()),
                            _ => p.set_value(id, pool[rng.below(pool.len() as u64) as usize]),
                        }
                        // set_active(.., 0) makes the node invisible again
                        if p.find_node_by_id(id).is_some() {
                            p.process_node_name(id);
                        }
                    }
                }
                _ => {
                    p.maxnmin(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
                    next_id = 7;
                }
            }
            calls += 1;
        }
        if round % 100 == 0 {
            eprintln!("round {round}/{iters}, {calls} differential calls, all matching");
        }
    }
    eprintln!("fuzz complete: {calls} differential calls, 0 divergences");
}
