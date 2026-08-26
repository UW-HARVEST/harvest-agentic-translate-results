// Phase B — differential tests for the lowest-level exported entry point,
// `int *static_alias(int *outer)`.
//
// Covers CONFIGS.md rows 1-9 and ERRORS.md rows 14-16.
//
// Every call goes through `dlsym` on the C `.so` and on the Rust `.so`; the two
// images are loaded from private copies so each scenario starts with a pristine
// `static int inner = 1;`.
//
// Observables compared after every single call:
//   * whether the returned pointer is the caller's object or the hidden static
//     (pointer identity, both against the argument and against the caller cell),
//   * the value the returned pointer points at,
//   * the caller's object after the call.

mod common;

use common::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    /// Store `v` into the caller's cell and call `static_alias(&cell)`.
    Fresh(i32),
    /// Call `static_alias(p)` with the pointer the previous call returned
    /// (`outer == &inner` once the static has been returned once).
    Chain,
    /// Non-destructive probe of `inner`: `*outer = INT_MIN` takes the else
    /// branch for every `inner > INT_MIN`, revealing `inner` in the cell without
    /// modifying it.
    Probe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Obs {
    ret_is_arg: bool,
    ret_is_cell: bool,
    ret_val: i32,
    cell: i32,
}

fn run_ops(imp: &Impl, ops: &[Op]) -> Vec<Obs> {
    let mut cell: Box<i32> = Box::new(0);
    let mut last: *mut i32 = std::ptr::null_mut();
    let mut out = Vec::with_capacity(ops.len());

    for op in ops {
        let cell_ptr: *mut i32 = &mut *cell;
        let arg: *mut i32 = match *op {
            Op::Fresh(v) => {
                *cell = v;
                cell_ptr
            }
            Op::Chain => {
                if last.is_null() {
                    cell_ptr
                } else {
                    last
                }
            }
            Op::Probe => {
                *cell = i32::MIN;
                cell_ptr
            }
        };
        let ret = unsafe { (imp.static_alias)(arg) };
        out.push(Obs {
            ret_is_arg: ret == arg,
            ret_is_cell: ret == cell_ptr,
            ret_val: unsafe { *ret },
            cell: *cell,
        });
        last = ret;
    }
    out
}

fn assert_same(tag: &str, ops: &[Op]) {
    let p = load_pair(tag);
    let c = run_ops(&p.c, ops);
    let r = run_ops(&p.rust, ops);
    for (i, (co, ro)) in c.iter().zip(r.iter()).enumerate() {
        assert_eq!(
            co, ro,
            "[{tag}] divergence at step {i} (op {:?})\n  C   : {co:?}\n  Rust: {ro:?}\n  ops so far: {:?}",
            ops[i],
            &ops[..=i.min(20)]
        );
    }
    assert_eq!(c.len(), r.len(), "[{tag}] observation count");
}

/// Prefix that drives the hidden `inner` into a particular state.
/// The very same prefix is replayed on both images, so the tests never need to
/// know the resulting value.
fn prefix_for_state(state: &str) -> Vec<Op> {
    match state {
        // inner == 1 (pristine)
        "initial" => vec![],
        // inner == 101
        "grown" => vec![Op::Fresh(100)],
        // 1 -> 2 -> 4 ... -> 2^31 == INT_MIN (signed overflow of the doubling)
        "int_min" => {
            let mut v = vec![Op::Fresh(1)];
            v.extend(std::iter::repeat(Op::Chain).take(30));
            v
        }
        // one more doubling: 2^32 == 0
        "zero" => {
            let mut v = vec![Op::Fresh(1)];
            v.extend(std::iter::repeat(Op::Chain).take(31));
            v
        }
        // 1 -> 2147483001 -> wraps to a "generic" negative value (-648)
        "negative" => vec![Op::Fresh(2_147_483_000), Op::Fresh(i32::MAX)],
        other => panic!("unknown state {other}"),
    }
}

// ------------------------------------------------------------------ CONFIGS #1

#[test]
fn alias_single_call_random() {
    // Pristine image, exactly one call, random value: hits both branches.
    let mut rng = Rng::new(0x5EED_0001);
    let mut values: Vec<i32> = vec![i32::MIN, -1, 0, 1, i32::MAX, 2, -2];
    for _ in 0..25 {
        values.push(rng.next_i32());
    }
    for (i, v) in values.iter().enumerate() {
        assert_same(&format!("single{i}"), &[Op::Fresh(*v)]);
    }
}

// ------------------------------------------------------------------ CONFIGS #2

#[test]
fn alias_else_branch_shapes() {
    for (i, v) in [i32::MIN, -1_000_000, -1, 0, i32::MIN + 1]
        .iter()
        .enumerate()
    {
        // inner == 1, so all of these take the else branch.
        assert_same(&format!("else{i}"), &[Op::Fresh(*v), Op::Fresh(*v)]);
    }
    // randomized values that all take the else branch from a pristine image
    let mut rng = Rng::new(0xE15E_0001);
    for i in 0..20 {
        let v = rng.range_i32(i32::MIN, 0);
        assert_same(&format!("else_rand{i}"), &[Op::Fresh(v), Op::Fresh(v), Op::Chain]);
    }
}

// ------------------------------------------------------------------ CONFIGS #3

#[test]
fn alias_then_branch_shapes() {
    for (i, v) in [1, 2, 1000, i32::MAX, i32::MAX - 1].iter().enumerate() {
        assert_same(&format!("then{i}"), &[Op::Fresh(*v)]);
    }
    // randomized values that all take the then branch from a pristine image
    let mut rng = Rng::new(0x7E17_0001);
    for i in 0..20 {
        let v = rng.range_i32(1, i32::MAX);
        assert_same(&format!("then_rand{i}"), &[Op::Fresh(v), Op::Chain]);
    }
}

// ------------------------------------------------------------------ CONFIGS #4

#[test]
fn alias_equality_edge() {
    // *outer == inner exactly, with inner == 1 ...
    assert_same("eq_initial", &[Op::Fresh(1)]);
    // ... and with a grown inner (Fresh(100) -> inner == 101, then *outer == 101).
    assert_same("eq_grown", &[Op::Fresh(100), Op::Fresh(101), Op::Fresh(202)]);
    // one below / one above the equality edge
    assert_same("eq_below", &[Op::Fresh(100), Op::Fresh(100)]);
    assert_same("eq_above", &[Op::Fresh(100), Op::Fresh(102)]);

    // Randomized equality edges: from a pristine image `inner == 1`, so
    // Fresh(k) with k >= 1 leaves inner == k + 1, and the following values sit
    // exactly on / just below / just above the next `>=` boundaries.
    let mut rng = Rng::new(0xE00E_0001);
    for i in 0..20 {
        let k = rng.range_i32(1, 1 << 20);
        assert_same(
            &format!("eq_rand{i}"),
            &[
                Op::Fresh(k),
                Op::Fresh(k + 1),         // exactly inner
                Op::Fresh(2 * (k + 1)),   // exactly inner again
                Op::Fresh(4 * (k + 1) - 1), // one below
                Op::Fresh(4 * (k + 1)),   // and one above the previous inner
                Op::Chain,
            ],
        );
    }
}

// ------------------------------------------------------------------ CONFIGS #5

#[test]
fn alias_self_aliasing_chain() {
    // outer == &inner, chained: doubling until the signed overflow wraps and
    // beyond (inner becomes INT_MIN, then 0, then stays 0).
    for start in [1, 5, 0, -1, i32::MAX, i32::MIN, 1 << 29] {
        let mut ops = vec![Op::Fresh(start)];
        ops.extend(std::iter::repeat(Op::Chain).take(40));
        assert_same(&format!("chain{start}"), &ops);
    }
    // Interleave probes so the hidden state is observed at every step, too.
    let mut ops = vec![Op::Fresh(1)];
    for _ in 0..35 {
        ops.push(Op::Chain);
        ops.push(Op::Probe);
    }
    assert_same("chain_probed", &ops);
}

// ------------------------------------------------------------------ CONFIGS #6

#[test]
fn alias_overflow_both_branches() {
    // then branch overflow: inner grown to ~2^30, *outer = INT_MAX.
    assert_same(
        "ovf_then",
        &[Op::Fresh(1 << 30), Op::Fresh(i32::MAX), Op::Fresh(i32::MAX)],
    );
    // else branch overflow: inner driven negative, *outer = INT_MIN.
    let mut ops = prefix_for_state("negative");
    ops.push(Op::Fresh(i32::MIN));
    ops.push(Op::Fresh(i32::MIN + 1));
    assert_same("ovf_else", &ops);

    // then branch overflow from INT_MIN state (inner == INT_MIN, *outer == INT_MIN).
    let mut ops = prefix_for_state("int_min");
    ops.push(Op::Fresh(i32::MIN));
    ops.push(Op::Fresh(i32::MAX));
    assert_same("ovf_int_min", &ops);

    // Randomized overflow: two large positive values overflow the then branch,
    // and a negative `inner` with a very negative `*outer` overflows the else
    // branch.
    let mut rng = Rng::new(0x0FF0_0001);
    for i in 0..15 {
        let a = rng.range_i32(1 << 30, i32::MAX);
        let b = rng.range_i32(1 << 30, i32::MAX);
        assert_same(
            &format!("ovf_rand_then{i}"),
            &[Op::Fresh(a), Op::Fresh(b), Op::Chain, Op::Probe],
        );

        let mut ops = prefix_for_state("negative");
        ops.push(Op::Fresh(rng.range_i32(i32::MIN, i32::MIN + 4096)));
        ops.push(Op::Fresh(rng.range_i32(i32::MIN, -(1 << 30))));
        ops.push(Op::Chain);
        assert_same(&format!("ovf_rand_else{i}"), &ops);
    }
}

// ------------------------------------------------------------------ CONFIGS #7

#[test]
fn alias_state_value_matrix() {
    let states = ["initial", "grown", "zero", "negative", "int_min"];
    let values = [i32::MIN, -1, 0, 1, i32::MAX, 7, -7];
    for state in states {
        for (j, v) in values.iter().enumerate() {
            let mut ops = prefix_for_state(state);
            ops.push(Op::Fresh(*v));
            // and once more, so the follow-on state is compared as well
            ops.push(Op::Fresh(*v));
            ops.push(Op::Chain);
            assert_same(&format!("matrix_{state}_{j}"), &ops);
        }
    }
    // same cross-product with randomized values per state
    let mut rng = Rng::new(0x3A7E_0001);
    for state in states {
        for j in 0..8 {
            let mut ops = prefix_for_state(state);
            ops.push(Op::Fresh(rng.next_i32()));
            ops.push(Op::Probe);
            ops.push(Op::Fresh(rng.next_i32()));
            ops.push(Op::Chain);
            assert_same(&format!("matrix_rand_{state}_{j}"), &ops);
        }
    }
}

// ------------------------------------------------------------------ CONFIGS #8

#[test]
fn alias_random_sequences() {
    for seed in 0..6u64 {
        let mut rng = Rng::new(0xA5A5_0000 + seed);
        let mut ops = Vec::with_capacity(500);
        for _ in 0..500 {
            match rng.below(10) {
                0..=4 => {
                    // mix of wild and near-boundary values
                    let v = match rng.below(6) {
                        0 => i32::MIN,
                        1 => i32::MAX,
                        2 => 0,
                        3 => rng.range_i32(-5, 5),
                        4 => rng.range_i32(i32::MIN, i32::MIN + 32),
                        _ => rng.next_i32(),
                    };
                    ops.push(Op::Fresh(v));
                }
                5..=8 => ops.push(Op::Chain),
                _ => ops.push(Op::Probe),
            }
        }
        assert_same(&format!("rand{seed}"), &ops);
    }
}

// ------------------------------------------------------------------ CONFIGS #9

#[test]
fn alias_caller_storage_shapes() {
    // (a) heap cell
    let p = load_pair("storage_heap");
    let mut cb: Box<i32> = Box::new(-3);
    let mut rb: Box<i32> = Box::new(-3);
    let cret = unsafe { (p.c.static_alias)(&mut *cb) };
    let rret = unsafe { (p.rust.static_alias)(&mut *rb) };
    assert_eq!(unsafe { *cret }, unsafe { *rret }, "heap: *ret");
    assert_eq!(*cb, *rb, "heap: cell");
    assert_eq!(
        cret == &mut *cb as *mut i32,
        rret == &mut *rb as *mut i32,
        "heap: identity"
    );

    // (b) element of an array: neighbours must be untouched in both
    let p = load_pair("storage_array");
    let mut ca: [i32; 5] = [11, 22, -33, 44, 55];
    let mut ra: [i32; 5] = [11, 22, -33, 44, 55];
    let cret = unsafe { (p.c.static_alias)(&mut ca[2]) };
    let rret = unsafe { (p.rust.static_alias)(&mut ra[2]) };
    assert_eq!(unsafe { *cret }, unsafe { *rret }, "array: *ret");
    assert_eq!(ca, ra, "array: whole array (neighbour writes)");
    assert_eq!(
        cret == &mut ca[2] as *mut i32,
        rret == &mut ra[2] as *mut i32,
        "array: identity"
    );
    // second call now takes the then branch and must not write the array at all
    let cret = unsafe { (p.c.static_alias)(&mut ca[3]) };
    let rret = unsafe { (p.rust.static_alias)(&mut ra[3]) };
    assert_eq!(unsafe { *cret }, unsafe { *rret }, "array2: *ret");
    assert_eq!(ca, ra, "array2: whole array");

    // (c) 'static cell
    static mut C_CELL: i32 = 9;
    static mut R_CELL: i32 = 9;
    let p = load_pair("storage_static");
    let cret = unsafe { (p.c.static_alias)(std::ptr::addr_of_mut!(C_CELL)) };
    let rret = unsafe { (p.rust.static_alias)(std::ptr::addr_of_mut!(R_CELL)) };
    assert_eq!(unsafe { *cret }, unsafe { *rret }, "static: *ret");
    assert_eq!(unsafe { C_CELL }, unsafe { R_CELL }, "static: cell");

    // (d) randomized values through heap cells and array elements, several calls
    // per image so both branches and the aliasing case are hit
    let mut rng = Rng::new(0x5709_0001);
    for round in 0..10 {
        let p = load_pair(&format!("storage_rand{round}"));
        let mut carr: [i32; 4] = [0; 4];
        let mut rarr: [i32; 4] = [0; 4];
        let mut cbox: Box<i32> = Box::new(0);
        let mut rbox: Box<i32> = Box::new(0);
        let mut clast: *mut i32 = std::ptr::null_mut();
        let mut rlast: *mut i32 = std::ptr::null_mut();
        for step in 0..12 {
            let v = rng.next_i32();
            let idx = rng.below(4) as usize;
            let (carg, rarg) = match rng.below(3) {
                0 => {
                    carr[idx] = v;
                    rarr[idx] = v;
                    (&mut carr[idx] as *mut i32, &mut rarr[idx] as *mut i32)
                }
                1 => {
                    *cbox = v;
                    *rbox = v;
                    (&mut *cbox as *mut i32, &mut *rbox as *mut i32)
                }
                _ => {
                    if clast.is_null() {
                        *cbox = v;
                        *rbox = v;
                        (&mut *cbox as *mut i32, &mut *rbox as *mut i32)
                    } else {
                        (clast, rlast)
                    }
                }
            };
            let cret = unsafe { (p.c.static_alias)(carg) };
            let rret = unsafe { (p.rust.static_alias)(rarg) };
            assert_eq!(
                (unsafe { *cret }, cret == carg, carr, *cbox),
                (unsafe { *rret }, rret == rarg, rarr, *rbox),
                "storage_rand{round} step {step} (value {v}, index {idx})"
            );
            clast = cret;
            rlast = rret;
        }
    }
}

// ------------------------------------------------------------- ERRORS #14/#15/#16

#[test]
fn alias_boundary_and_enumlike_values() {
    // Every "out of range" integer a caller can pass across the FFI boundary:
    // the C prototype takes an int, so all 2^32 bit patterns are valid inputs.
    // Boundaries plus a random sample.
    let mut rng = Rng::new(0xDEAD_BEEF);
    let mut values = vec![
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
        1 << 30,
        -(1 << 30),
    ];
    for _ in 0..20 {
        values.push(rng.next_i32());
    }
    // All of them in one sequence per image (state accumulates identically) ...
    let ops: Vec<Op> = values.iter().map(|v| Op::Fresh(*v)).collect();
    assert_same("boundary_seq", &ops);
    // ... and each from a pristine image.
    for (i, v) in values.iter().enumerate() {
        assert_same(&format!("boundary{i}"), &[Op::Fresh(*v), Op::Chain]);
    }
}
