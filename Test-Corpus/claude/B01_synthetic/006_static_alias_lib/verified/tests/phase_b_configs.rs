// Phase B — valid-path differential tests, one test per CONFIGS.md row.
//
// Every test calls BOTH .so's through their exported symbols and compares all
// observables (returned value, returned-pointer aliasing class, the caller's
// buffer after the call, the hidden `inner`, and for `driver` the exact stdout
// bytes). Rows are driven with randomized inputs from a fixed seed.
//
// Run single-threaded (fd 1 is redirected to capture `driver`'s printf):
//     cargo test -- --test-threads=1

mod common;

use common::*;
use std::ffi::c_int;

const MIN: c_int = c_int::MIN;
const MAX: c_int = c_int::MAX;

// ===========================================================================
// C1 — then branch, distinct pointer, random *outer > inner
// ===========================================================================
#[test]
fn cfg_c1_then_branch_distinct_random() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..200 {
        let base = rng.i32_in(1, 1 << 20);
        h.set_inner(base);
        let delta = rng.i32_in(1, 1 << 20);
        let val = base + delta; // > inner, no overflow at these magnitudes
        let o = h.sa(val);
        assert_eq!(o.cls, Cls::Inner, "then branch returns &inner");
        assert_eq!(o.ret_val, base.wrapping_add(val), "inner += *outer");
        assert_eq!(o.buf_after, val, "then branch must not touch *outer");
        assert_eq!(h.probe(), base.wrapping_add(val));
    }
}

// ===========================================================================
// C2 — then branch at the exact `>=` equality boundary (*outer == inner)
// ===========================================================================
#[test]
fn cfg_c2_then_branch_equality_boundary() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 2);
    let mut cases: Vec<c_int> = vec![1, 0, -1, 2, -2, MAX, MIN + 1, 1 << 30, -(1 << 30)];
    for _ in 0..100 {
        cases.push(rng.i32_any());
    }
    for base in cases {
        if base == MIN {
            continue; // covered by C10 (needs the no-probe path)
        }
        h.set_inner(base);
        let o = h.sa(base); // *outer == inner  =>  `>=` holds
        assert_eq!(o.cls, Cls::Inner, "equality takes the then branch");
        assert_eq!(o.ret_val, base.wrapping_add(base));
        assert_eq!(o.buf_after, base);
    }
}

// ===========================================================================
// C3 — else branch, distinct pointer, random *outer < inner
// ===========================================================================
#[test]
fn cfg_c3_else_branch_distinct_random() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..200 {
        let base = rng.i32_in(-(1 << 20), 1 << 20);
        h.set_inner(base);
        let delta = rng.i32_in(1, 1 << 20);
        let val = base - delta; // < inner, no overflow at these magnitudes
        let o = h.sa(val);
        assert_eq!(o.cls, Cls::Outer, "else branch returns the caller's pointer");
        assert_eq!(o.buf_after, val.wrapping_add(base), "*outer += inner");
        assert_eq!(o.ret_val, o.buf_after, "ret aliases the caller's buffer");
        assert_eq!(h.probe(), base, "else branch must leave inner alone");
    }
}

// ===========================================================================
// C4 — else branch one step below the boundary (*outer == inner - 1)
// ===========================================================================
#[test]
fn cfg_c4_else_branch_one_below() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 4);
    let mut cases: Vec<c_int> = vec![1, 0, -1, 2, MAX, MIN + 1, 1 << 30];
    for _ in 0..100 {
        cases.push(rng.i32_any());
    }
    for base in cases {
        if base == MIN {
            continue; // inner-1 wraps to MAX, which is >= MIN: not the else branch
        }
        h.set_inner(base);
        let val = base - 1;
        let o = h.sa(val);
        assert_eq!(o.cls, Cls::Outer, "one below boundary => else branch");
        assert_eq!(o.buf_after, val.wrapping_add(base));
        assert_eq!(h.probe(), base);
    }
}

// ===========================================================================
// C5 — aliased calls (outer == &inner): inner += inner doubling to a 0 fixpoint
// ===========================================================================
#[test]
fn cfg_c5_aliased_doubling() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 5);
    let mut starts: Vec<c_int> = vec![1, 3, -7, MAX, MIN + 1, 1 << 29];
    for _ in 0..20 {
        starts.push(rng.i32_any());
    }
    for start in starts {
        if start == MIN {
            continue;
        }
        h.set_inner(start);
        let mut expect = start;
        for step in 0..40 {
            let o = h.sa_aliased();
            expect = expect.wrapping_add(expect);
            assert_eq!(o.cls, Cls::Inner, "aliased call returns &inner");
            assert_eq!(o.ret_val, expect, "step {step}: inner += inner");
        }
        // 40 doublings of any 32-bit value reach the 0 fixpoint.
        assert_eq!(expect, 0);
        assert_eq!(h.probe(), 0, "doubling reaches a 0 fixpoint");
        let o = h.sa_aliased();
        assert_eq!(o.ret_val, 0, "0 is a fixpoint");
    }
}

// ===========================================================================
// C6 — chained: feed the returned pointer back in, as `driver` composes it
// ===========================================================================
#[test]
fn cfg_c6_chained_returned_pointer() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..64 {
        let base = rng.i32_any();
        if base == MIN {
            continue;
        }
        h.set_inner(base);
        let initial = rng.i32_any();
        let got = h.chain(initial, 40); // asserts C == Rust
        let (want, final_inner) = model_chain(base, initial, 40);
        assert_eq!(got, want, "chain(inner={base}, initial={initial}) vs model");
        if final_inner != MIN {
            assert_eq!(h.probe(), final_inner, "final inner vs model");
        }
    }
}

// ===========================================================================
// C7 — inner == 0
// ===========================================================================
#[test]
fn cfg_c7_inner_zero_state() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 7);
    let mut vals: Vec<c_int> = vec![0, 1, -1, MAX, MIN, 2, -2, 1 << 30];
    for _ in 0..100 {
        vals.push(rng.i32_any());
    }
    for val in vals {
        h.set_inner(0);
        let o = h.sa(val);
        if val >= 0 {
            assert_eq!(o.cls, Cls::Inner);
            assert_eq!(o.ret_val, val, "inner = 0 + *outer");
            assert_eq!(o.buf_after, val);
        } else {
            assert_eq!(o.cls, Cls::Outer);
            assert_eq!(o.buf_after, val, "*outer += 0");
            assert_eq!(o.ret_val, val);
        }
    }
}

// ===========================================================================
// C8 — inner negative
// ===========================================================================
#[test]
fn cfg_c8_inner_negative_state() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 8);
    let bases: Vec<c_int> = vec![-1, -2, -1000, -(1 << 30), MIN + 1];
    for base in bases {
        let mut vals: Vec<c_int> = vec![base, base - 1, base + 1, 0, MAX, MIN, -1];
        for _ in 0..40 {
            vals.push(rng.i32_any());
        }
        for val in vals {
            h.set_inner(base);
            let o = h.sa(val);
            if val >= base {
                assert_eq!(o.cls, Cls::Inner);
                assert_eq!(o.ret_val, base.wrapping_add(val));
                assert_eq!(o.buf_after, val);
            } else {
                assert_eq!(o.cls, Cls::Outer);
                assert_eq!(o.buf_after, val.wrapping_add(base));
                assert_eq!(h.probe(), base);
            }
        }
    }
}

// ===========================================================================
// C9 — inner == INT_MAX (then branch overflows)
// ===========================================================================
#[test]
fn cfg_c9_inner_intmax_state() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 9);
    let mut vals: Vec<c_int> = vec![MAX, MAX - 1, 0, -1, MIN, 1];
    for _ in 0..60 {
        vals.push(rng.i32_any());
    }
    for val in vals {
        h.set_inner(MAX);
        let o = h.sa(val);
        if val >= MAX {
            // only val == MAX
            assert_eq!(o.cls, Cls::Inner);
            assert_eq!(o.ret_val, MAX.wrapping_add(MAX), "INT_MAX+INT_MAX wraps to -2");
            assert_eq!(o.ret_val, -2);
        } else {
            assert_eq!(o.cls, Cls::Outer);
            assert_eq!(o.buf_after, val.wrapping_add(MAX));
            assert_eq!(h.probe(), MAX);
        }
    }
}

// ===========================================================================
// C10 — inner == INT_MIN (the then branch is ALWAYS taken: x >= INT_MIN)
// ===========================================================================
#[test]
fn cfg_c10_inner_intmin_state() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 10);
    let mut vals: Vec<c_int> = vec![MIN, MIN + 1, 0, 1, -1, MAX];
    for _ in 0..60 {
        vals.push(rng.i32_any());
    }
    for val in vals {
        h.set_inner(MIN);
        // sa_np: no probe first — probing would itself take the then branch.
        let o = h.sa_np(val);
        assert_eq!(o.cls, Cls::Inner, "every value is >= INT_MIN");
        assert_eq!(o.ret_val, MIN.wrapping_add(val));
        assert_eq!(o.buf_after, val, "then branch must not touch *outer");
        let after = MIN.wrapping_add(val);
        if after != MIN {
            assert_eq!(h.probe(), after);
        }
    }
}

// ===========================================================================
// C11 — extreme input values against a large positive inner
// ===========================================================================
#[test]
fn cfg_c11_extreme_input_values() {
    let mut h = harness();
    for base in [1 << 30, MAX - 1, 1_000_000_000] {
        for val in [MAX, MIN, 0, 1, -1, MIN + 1, MAX - 1] {
            h.set_inner(base);
            let o = h.sa(val);
            if val >= base {
                assert_eq!(o.cls, Cls::Inner);
                assert_eq!(o.ret_val, base.wrapping_add(val));
            } else {
                assert_eq!(o.cls, Cls::Outer);
                assert_eq!(o.buf_after, val.wrapping_add(base));
            }
        }
    }
}

// ===========================================================================
// C12 — long randomized state machine, mixing aliased and distinct calls
// ===========================================================================
#[test]
fn cfg_c12_random_state_machine() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 12);

    let start = 1;
    h.set_inner(start);
    let mut inner = start; // independent model of the hidden static

    for i in 0..4000 {
        if rng.bool() {
            // aliased: inner += inner
            let o = h.sa_aliased();
            inner = inner.wrapping_add(inner);
            assert_eq!(o.cls, Cls::Inner, "op {i}");
            assert_eq!(o.ret_val, inner, "op {i}: aliased inner += inner");
        } else {
            let val = if rng.bool() {
                rng.i32_any()
            } else {
                // bias towards the branch boundary
                inner.wrapping_add(rng.i32_in(-2, 2))
            };
            let o = h.sa_np(val);
            if val >= inner {
                inner = inner.wrapping_add(val);
                assert_eq!(o.cls, Cls::Inner, "op {i}: val={val} inner={inner}");
                assert_eq!(o.ret_val, inner, "op {i}");
                assert_eq!(o.buf_after, val, "op {i}");
            } else {
                assert_eq!(o.cls, Cls::Outer, "op {i}: val={val} inner={inner}");
                assert_eq!(o.buf_after, val.wrapping_add(inner), "op {i}");
                assert_eq!(o.ret_val, o.buf_after, "op {i}");
            }
        }
    }
    if inner != MIN {
        assert_eq!(h.probe(), inner, "model vs libraries after 4000 ops");
    }
}

// ===========================================================================
// C13 — driver, 1 iteration, initial_value >= inner (then branch)
// ===========================================================================
#[test]
fn cfg_c13_driver_one_iteration_then() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..60 {
        let base = rng.i32_in(1, 1 << 20);
        h.set_inner(base);
        let initial = base + rng.i32_in(0, 1 << 20);
        let out = h.driver(initial, 1);
        assert_eq!(out, model_driver_bytes(base, initial, 1));
        assert_eq!(out, expect_lines(&[base.wrapping_add(initial)]));
        assert_eq!(h.probe(), base.wrapping_add(initial));
    }
}

// ===========================================================================
// C14 — driver, 1 iteration, initial_value < inner (else branch)
// ===========================================================================
#[test]
fn cfg_c14_driver_one_iteration_else() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..60 {
        let base = rng.i32_in(-(1 << 20), 1 << 20);
        h.set_inner(base);
        let initial = base - rng.i32_in(1, 1 << 20);
        let out = h.driver(initial, 1);
        assert_eq!(out, model_driver_bytes(base, initial, 1));
        assert_eq!(out, expect_lines(&[initial.wrapping_add(base)]));
        assert_eq!(h.probe(), base, "else branch leaves inner alone");
    }
}

// ===========================================================================
// C15 — driver, 2 iterations (step 2 switches to aliased doubling)
// ===========================================================================
#[test]
fn cfg_c15_driver_two_iterations() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 15);
    let mut cases: Vec<(c_int, c_int)> = vec![
        (1, 5),
        (1, 0),
        (1, -5),
        (0, 0),
        (-3, -4),
        (7, 7),
        (MAX, MAX),
        (0, MIN),
    ];
    for _ in 0..60 {
        cases.push((rng.i32_any(), rng.i32_any()));
    }
    for (base, initial) in cases {
        if base == MIN {
            continue;
        }
        h.set_inner(base);
        let out = h.driver(initial, 2);
        assert_eq!(
            out,
            model_driver_bytes(base, initial, 2),
            "driver({initial},2) with inner={base}"
        );
    }
}

// ===========================================================================
// C16 — driver, many iterations: aliased doubling overflows mid-run
// ===========================================================================
#[test]
fn cfg_c16_driver_many_iterations_overflow() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 16);
    // (a) initial >= inner, so step 1 takes the then branch and the remaining 39
    // steps are aliased doublings — enough to wrap all the way to the 0 fixpoint.
    for _ in 0..40 {
        let base = rng.i32_in(1, 1 << 20);
        h.set_inner(base);
        let initial = base + rng.i32_in(0, 1 << 20);
        let out = h.driver(initial, 40);
        assert_eq!(
            out,
            model_driver_bytes(base, initial, 40),
            "driver({initial},40) with inner={base}"
        );
        assert!(
            out.ends_with(b"0\n"),
            "39 doublings must reach the 0 fixpoint, got {:?}",
            String::from_utf8_lossy(&out)
        );
    }

    // (b) unconstrained initial: may linger in the else branch (each step adds
    // `inner` to the caller's local) and never start doubling within 40 steps.
    // No fixpoint claim here — just the byte-exact differential + model check.
    for _ in 0..40 {
        let base = rng.i32_in(1, 1 << 20);
        h.set_inner(base);
        let initial = rng.i32_any();
        let out = h.driver(initial, 40);
        assert_eq!(
            out,
            model_driver_bytes(base, initial, 40),
            "driver({initial},40) with inner={base}"
        );
    }
}

// ===========================================================================
// C17 — driver with negative initial_value against positive inner
// ===========================================================================
#[test]
fn cfg_c17_driver_negative_initial() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..60 {
        let base = rng.i32_in(1, 1 << 20);
        h.set_inner(base);
        let initial = -rng.i32_in(1, 1 << 20);
        let iters = rng.i32_in(1, 12);
        let out = h.driver(initial, iters);
        assert_eq!(out, model_driver_bytes(base, initial, iters));
    }
}

// ===========================================================================
// C18 — driver with inner == 0
// ===========================================================================
#[test]
fn cfg_c18_driver_inner_zero() {
    let mut h = harness();
    for initial in [-1000, -1, 0, 1, 1000, MAX, MIN] {
        for iters in [1, 2, 5] {
            h.set_inner(0);
            let out = h.driver(initial, iters);
            assert_eq!(
                out,
                model_driver_bytes(0, initial, iters),
                "driver({initial},{iters}) with inner=0"
            );
        }
    }
}

// ===========================================================================
// C19 — driver with negative inner
// ===========================================================================
#[test]
fn cfg_c19_driver_inner_negative() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 19);
    for base in [-1, -1000, -(1 << 30), MIN + 1] {
        let mut inits: Vec<c_int> = vec![base, base - 1, base + 1, 0, MAX, MIN];
        for _ in 0..20 {
            inits.push(rng.i32_any());
        }
        for initial in inits {
            let iters = rng.i32_in(1, 8);
            h.set_inner(base);
            let out = h.driver(initial, iters);
            assert_eq!(
                out,
                model_driver_bytes(base, initial, iters),
                "driver({initial},{iters}) with inner={base}"
            );
        }
    }
}

// ===========================================================================
// C20 — driver with extreme initial_value
// ===========================================================================
#[test]
fn cfg_c20_driver_extreme_initial() {
    let mut h = harness();
    for initial in [MIN, MIN + 1, MAX, MAX - 1] {
        for iters in [1, 3, 10] {
            for base in [1, 0, -1, MAX, 1 << 30] {
                h.set_inner(base);
                let out = h.driver(initial, iters);
                assert_eq!(
                    out,
                    model_driver_bytes(base, initial, iters),
                    "driver({initial},{iters}) with inner={base}"
                );
            }
        }
    }
}

// ===========================================================================
// C21 — printf("%d\n") formatting shapes: 0, negative, and 11-byte INT_MIN
// ===========================================================================
#[test]
fn cfg_c21_driver_output_formatting() {
    let mut h = harness();

    // exactly "0\n"
    h.set_inner(0);
    assert_eq!(h.driver(0, 1), b"0\n".to_vec());

    // negative: inner=5, initial=-3 -> else -> -3+5=2 ... use a clear case
    h.set_inner(10);
    assert_eq!(h.driver(-100, 1), b"-90\n".to_vec());

    // INT_MIN prints as 11 bytes "-2147483648"
    h.set_inner(1);
    let out = h.driver(MAX, 1); // 1 + INT_MAX wraps to INT_MIN
    assert_eq!(out, b"-2147483648\n".to_vec());
    assert_eq!(out.len(), 12);

    // a run mixing widths
    h.set_inner(1);
    let out = h.driver(0, 4);
    assert_eq!(out, model_driver_bytes(1, 0, 4));
    // 0 < 1 -> else: buf=1, print 1; 1>=1 -> inner=2 print 2; then 4; then 8
    assert_eq!(out, b"1\n2\n4\n8\n".to_vec());
}

// ===========================================================================
// C22 — cross-entry-point state sharing: interleave driver and static_alias
// ===========================================================================
#[test]
fn cfg_c22_interleaved_entry_points() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 22);
    h.set_inner(1);
    let mut inner = 1;

    for i in 0..300 {
        match rng.next_u64() % 3 {
            0 => {
                let val = rng.i32_any();
                let o = h.sa_np(val);
                if val >= inner {
                    inner = inner.wrapping_add(val);
                    assert_eq!(o.cls, Cls::Inner, "op {i}");
                    assert_eq!(o.ret_val, inner, "op {i}");
                } else {
                    assert_eq!(o.cls, Cls::Outer, "op {i}");
                    assert_eq!(o.buf_after, val.wrapping_add(inner), "op {i}");
                }
            }
            1 => {
                let o = h.sa_aliased();
                inner = inner.wrapping_add(inner);
                assert_eq!(o.ret_val, inner, "op {i}");
            }
            _ => {
                let initial = rng.i32_any();
                let iters = rng.i32_in(0, 6);
                let out = h.driver(initial, iters);
                let want = model_driver_bytes(inner, initial, iters);
                assert_eq!(out, want, "op {i}: driver({initial},{iters}) inner={inner}");
                let steps = if iters > 0 { iters as usize } else { 0 };
                inner = model_chain(inner, initial, steps).1;
            }
        }
    }
    if inner != MIN {
        assert_eq!(h.probe(), inner, "shared inner after interleaving");
    }
}

// ===========================================================================
// C23 — driver called repeatedly: state carries from one call to the next
// ===========================================================================
#[test]
fn cfg_c23_driver_called_repeatedly() {
    let mut h = harness();
    h.set_inner(1);
    let mut inner = 1;
    for round in 0..25 {
        let initial = (round as c_int) * 7 - 50;
        let iters = 3;
        let out = h.driver(initial, iters);
        assert_eq!(
            out,
            model_driver_bytes(inner, initial, iters),
            "round {round}: inner={inner}"
        );
        inner = model_chain(inner, initial, iters as usize).1;
    }
    if inner != MIN {
        assert_eq!(h.probe(), inner);
    }
}

// ===========================================================================
// C24 — fully randomized driver calls, full stdout byte-compare each
// ===========================================================================
#[test]
fn cfg_c24_driver_randomized() {
    let mut h = harness();
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..150 {
        let base = rng.i32_any();
        if base == MIN {
            continue;
        }
        h.set_inner(base);
        let initial = rng.i32_any();
        let iters = rng.i32_in(0, 30);
        let out = h.driver(initial, iters);
        assert_eq!(
            out,
            model_driver_bytes(base, initial, iters),
            "case {i}: driver({initial},{iters}) with inner={base}"
        );
    }
}
