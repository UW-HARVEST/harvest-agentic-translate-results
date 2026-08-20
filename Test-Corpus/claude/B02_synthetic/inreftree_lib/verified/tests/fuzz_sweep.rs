//! High-volume randomized differential sweep. `#[ignore]`d by default (it is
//! slow); run it explicitly, optionally with a different seed:
//!
//!   FUZZ_SEED=12345 FUZZ_ITERS=2000000 cargo test --release --test fuzz_sweep -- --ignored --nocapture
//!
//! This is the property-style safety net behind the per-row tests: it drives the
//! WHOLE public surface in randomized interleavings and compares the full
//! observable state after every single call.

mod common;
use common::*;
use std::ffi::c_int;

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

#[test]
#[ignore = "slow high-volume sweep; run explicitly"]
fn fuzz_inreftree() {
    let seed = env_u64("FUZZ_SEED", SEED);
    let iters = env_u64("FUZZ_ITERS", 500_000);
    with_libs(|p| {
        let mut rng = Rng::new(seed);
        for i in 0..iters {
            let (a, b, c, d) = match i % 4 {
                0 => (rng.i32(), rng.i32(), rng.i32(), rng.i32()),
                1 => (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()),
                2 => (rng.small(), rng.small(), rng.small(), rng.small()),
                _ => (rng.i32(), if rng.below(3) == 0 { 0 } else { rng.i32() }, rng.small(), rng.spicy_i32()),
            };
            let cv = (p.c.inreftree)(a, b, c, d);
            let rv = (p.rust.inreftree)(a, b, c, d);
            assert_eq!(cv, rv, "iter {i}: inreftree({a},{b},{c},{d}) C={cv} Rust={rv}");
            if i % 1024 == 0 {
                assert_state_eq(p, &format!("iter {i}"));
            }
        }
        println!("fuzz_inreftree: {iters} iterations matched (seed {seed})");
    });
}

/// Randomized interleavings of the WHOLE public API. Every argument is drawn
/// ONCE and then handed to both libraries, so the two are always driven with
/// byte-identical inputs.
#[test]
#[ignore = "slow high-volume sweep; run explicitly"]
fn fuzz_api_sequences() {
    let seed = env_u64("FUZZ_SEED", SEED);
    let rounds = env_u64("FUZZ_ROUNDS", 3000);
    with_libs(|p| {
        let mut rng = Rng::new(seed ^ 0xABCD);
        let mut calls = 0u64;
        for round in 0..rounds {
            p.c.reset();
            p.rust.reset();
            // Unique, strictly increasing ids with parents drawn from the
            // already-inserted ids keep the graph a forest, so
            // calculate_tree_sum always terminates (see ERRORS.md U5).
            let mut next_id: c_int = 1;
            for step in 0..150 {
                let ctx = format!("round {round} step {step}");
                match rng.below(10) {
                    // add_tree_node
                    0..=4 => {
                        let parent = if next_id == 1 || rng.below(5) == 0 {
                            -1
                        } else if rng.below(6) == 0 {
                            rng.spicy_i32() // usually absent -> rejection path
                        } else {
                            (rng.below(next_id as u64 - 1) + 1) as c_int
                        };
                        let value = rng.spicy_i32();
                        let label: Vec<u8> = (0..rng.below(50))
                            .map(|_| (rng.below(255) + 1) as u8)
                            .collect();
                        let cv = p.c.add_node(next_id, value, parent, &label);
                        let rv = p.rust.add_node(next_id, value, parent, &label);
                        assert_ret_eq(cv, rv, &format!("{ctx}: add_tree_node"));
                        assert_state_eq(p, &format!("{ctx}: add_tree_node"));
                        if cv >= 0 {
                            next_id += 1;
                        }
                    }
                    // find_node_by_id
                    5 => {
                        let id = if rng.below(2) == 0 {
                            rng.spicy_i32()
                        } else {
                            (rng.below(next_id as u64 + 2)) as c_int
                        };
                        assert_eq!(
                            p.c.find_index(id),
                            p.rust.find_index(id),
                            "{ctx}: find_node_by_id({id})"
                        );
                    }
                    // calculate_tree_sum
                    6 => {
                        let id = if rng.below(2) == 0 {
                            rng.spicy_i32()
                        } else {
                            (rng.below(next_id as u64 + 2)) as c_int
                        };
                        assert_ret_eq(
                            (p.c.calculate_tree_sum)(id),
                            (p.rust.calculate_tree_sum)(id),
                            &format!("{ctx}: calculate_tree_sum({id})"),
                        );
                    }
                    // parse_operation (incl. NULL)
                    7 => {
                        if rng.below(20) == 0 {
                            assert_ret_eq(p.c.parse_op_null(), p.rust.parse_op_null(), &format!("{ctx}: parse(NULL)"));
                        } else {
                            let s: Vec<u8> = (0..rng.below(12))
                                .map(|_| b"+*-/%abz"[rng.below(8) as usize])
                                .collect();
                            assert_ret_eq(p.c.parse_op(&s), p.rust.parse_op(&s), &format!("{ctx}: parse"));
                        }
                    }
                    // get_operation_func + call through the pointer
                    8 => {
                        let op = if rng.below(2) == 0 {
                            (rng.below(5) + 1) as c_int
                        } else {
                            rng.spicy_i32()
                        };
                        let cp = (p.c.get_operation_func)(op) as usize;
                        let rp = (p.rust.get_operation_func)(op) as usize;
                        assert_ne!(cp, 0, "{ctx}: C get_operation_func({op})");
                        assert_ne!(rp, 0, "{ctx}: Rust get_operation_func({op})");
                        let which = if (1..=5).contains(&op) { op } else { OP_ADD };
                        assert_eq!(cp, p.c.op_addr(which), "{ctx}: C op {op}");
                        assert_eq!(rp, p.rust.op_addr(which), "{ctx}: Rust op {op}");
                        let cf: OpFn = unsafe { std::mem::transmute(cp) };
                        let rf: OpFn = unsafe { std::mem::transmute(rp) };
                        let (a, mut b) = (rng.spicy_i32(), rng.spicy_i32());
                        if (which == OP_DIVIDE || which == OP_MODULO) && a == i32::MIN && b == -1 {
                            b = 1; // ERRORS.md U1
                        }
                        assert_ret_eq(cf(a, b, 0, 0), rf(a, b, 0, 0), &format!("{ctx}: op {op}({a},{b})"));
                    }
                    // inreftree in the middle of an unrelated sequence
                    _ => {
                        let (a, b, c, d) =
                            (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
                        assert_ret_eq(
                            (p.c.inreftree)(a, b, c, d),
                            (p.rust.inreftree)(a, b, c, d),
                            &format!("{ctx}: inreftree({a},{b},{c},{d})"),
                        );
                        assert_state_eq(p, &format!("{ctx}: inreftree"));
                        next_id = 5; // inreftree rebuilt ids 1..4
                    }
                }
                calls += 1;
            }
        }
        println!("fuzz_api_sequences: {rounds} rounds / {calls} calls matched (seed {seed})");
    });
}
