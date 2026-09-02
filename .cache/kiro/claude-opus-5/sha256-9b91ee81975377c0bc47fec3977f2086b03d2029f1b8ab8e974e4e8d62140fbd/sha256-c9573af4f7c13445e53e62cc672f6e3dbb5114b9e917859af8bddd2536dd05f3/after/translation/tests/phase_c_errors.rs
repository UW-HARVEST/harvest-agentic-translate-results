//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Rows that make the C library *fault* (rows 3, 4, 10) are run in a forked
//! child so the harness survives, and assert that both libraries die from the
//! SAME signal.

mod common;

use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Fault-comparison plumbing (rows 3, 4, 10)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// How a child process terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Outcome {
    Exited(i32),
    Signalled(i32),
}

/// Runs `f` in a forked child and reports how the child ended.
fn outcome_of<F: FnOnce()>(f: F) -> Outcome {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: only computation, then _exit (never returns to the harness).
        f();
        unsafe { _exit(0) };
    }
    let mut status: i32 = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    let termsig = status & 0x7f;
    if termsig != 0 {
        Outcome::Signalled(termsig)
    } else {
        Outcome::Exited((status >> 8) & 0xff)
    }
}

// ---------------------------------------------------------------------------
// Row 1 — divide_op with b == 0
// ---------------------------------------------------------------------------

#[test]
fn err_01_divide_by_zero() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 101);
    for a in EDGE_I32 {
        let cv = p.c.divide_op(a, 0, 0, 0);
        assert_eq!(cv, p.r.divide_op(a, 0, 0, 0), "divide_op({a}, 0)");
        assert_eq!(cv, 0, "C divide_op({a}, 0) must return 0");
    }
    for _ in 0..20_000 {
        let a = rng.next_i32_mixed();
        let (u1, u2) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        let cv = p.c.divide_op(a, 0, u1, u2);
        assert_eq!(cv, p.r.divide_op(a, 0, u1, u2), "divide_op({a}, 0)");
        assert_eq!(cv, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — modulo_op with b == 0
// ---------------------------------------------------------------------------

#[test]
fn err_02_modulo_by_zero() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 102);
    for a in EDGE_I32 {
        let cv = p.c.modulo_op(a, 0, 0, 0);
        assert_eq!(cv, p.r.modulo_op(a, 0, 0, 0), "modulo_op({a}, 0)");
        assert_eq!(cv, 0, "C modulo_op({a}, 0) must return 0");
    }
    for _ in 0..20_000 {
        let a = rng.next_i32_mixed();
        let (u1, u2) = (rng.next_i32_mixed(), rng.next_i32_mixed());
        let cv = p.c.modulo_op(a, 0, u1, u2);
        assert_eq!(cv, p.r.modulo_op(a, 0, u1, u2), "modulo_op({a}, 0)");
        assert_eq!(cv, 0);
    }
}

// ---------------------------------------------------------------------------
// Rows 3-4 — signed division overflow faults (SIGFPE in C)
// ---------------------------------------------------------------------------

const SIGFPE: i32 = 8;

#[test]
fn err_03_divide_intmin_neg1_faults() {
    let p = Pair::open();
    // Resolve the entry points BEFORE forking: the child must not re-enter the
    // dynamic loader.
    let cf = p.c.raw_op("divide_op");
    let rf = p.r.raw_op("divide_op");
    let co = outcome_of(|| unsafe {
        std::hint::black_box(cf(i32::MIN, -1, 0, 0));
    });
    let ro = outcome_of(|| unsafe {
        std::hint::black_box(rf(i32::MIN, -1, 0, 0));
    });
    assert_eq!(
        co,
        Outcome::Signalled(SIGFPE),
        "C divide_op(INT_MIN, -1) is expected to raise SIGFPE"
    );
    assert_eq!(co, ro, "divide_op(INT_MIN, -1): C and Rust must terminate identically");
}

#[test]
fn err_04_modulo_intmin_neg1_faults() {
    let p = Pair::open();
    let cf = p.c.raw_op("modulo_op");
    let rf = p.r.raw_op("modulo_op");
    let co = outcome_of(|| unsafe {
        std::hint::black_box(cf(i32::MIN, -1, 0, 0));
    });
    let ro = outcome_of(|| unsafe {
        std::hint::black_box(rf(i32::MIN, -1, 0, 0));
    });
    assert_eq!(
        co,
        Outcome::Signalled(SIGFPE),
        "C modulo_op(INT_MIN, -1) is expected to raise SIGFPE"
    );
    assert_eq!(co, ro, "modulo_op(INT_MIN, -1): C and Rust must terminate identically");
}

// ---------------------------------------------------------------------------
// Rows 5-7 — find_node_by_id rejections
// ---------------------------------------------------------------------------

#[test]
fn err_05_find_missing_id_null() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 105);
    for _ in 0..2_000 {
        p.reset_both();
        // ids 1..=n present; probe with ids that cannot be there
        let n = (rng.below(20) + 1) as c_int;
        let l = cstr(b"x");
        for id in 1..=n {
            let v = rng.next_i32_mixed();
            assert_eq!(
                p.c.add_tree_node(id, v, -1, &l),
                p.r.add_tree_node(id, v, -1, &l)
            );
        }
        p.c.set_node_count(n);
        p.r.set_node_count(n);
        for miss in [0, -1, n + 1, 10_000, i32::MIN, i32::MAX] {
            assert_eq!(p.c.find_node_by_id(miss), None, "C must return NULL for {miss}");
            assert_eq!(
                p.c.find_node_by_id(miss),
                p.r.find_node_by_id(miss),
                "find_node_by_id({miss})"
            );
        }
    }
}

#[test]
fn err_06_find_empty_table_null() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 106);
    // fill the table with real data, then declare it empty
    p.reset_both();
    let l = cstr(b"filled");
    for id in 1..=(MAX_NODES as c_int) {
        let v = rng.next_i32_mixed();
        assert_eq!(
            p.c.add_tree_node(id, v, -1, &l),
            p.r.add_tree_node(id, v, -1, &l)
        );
    }
    p.c.set_node_count(0);
    p.r.set_node_count(0);
    for id in 1..=(MAX_NODES as c_int) {
        assert_eq!(p.c.find_node_by_id(id), None, "C must return NULL when count==0");
        assert_eq!(p.c.find_node_by_id(id), p.r.find_node_by_id(id));
    }
    for _ in 0..20_000 {
        let id = rng.next_i32_mixed();
        assert_eq!(p.c.find_node_by_id(id), p.r.find_node_by_id(id));
    }
    // and calculate_tree_sum inherits the rejection
    for id in [1, 25, 50, -1, 0] {
        assert_eq!(p.c.calculate_tree_sum(id), 0);
        assert_eq!(p.c.calculate_tree_sum(id), p.r.calculate_tree_sum(id));
    }
}

#[test]
fn err_07_find_negative_count_null() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 107);
    p.reset_both();
    let l = cstr(b"neg");
    for id in 1..=10 {
        let v = rng.next_i32_mixed();
        assert_eq!(
            p.c.add_tree_node(id, v, -1, &l),
            p.r.add_tree_node(id, v, -1, &l)
        );
    }
    for count in [-1, -2, -100, i32::MIN, i32::MIN + 1] {
        p.c.set_node_count(count);
        p.r.set_node_count(count);
        for id in [1, 5, 10, 0, -1, i32::MAX] {
            assert_eq!(
                p.c.find_node_by_id(id),
                None,
                "C must return NULL with node_count={count}"
            );
            assert_eq!(
                p.c.find_node_by_id(id),
                p.r.find_node_by_id(id),
                "find_node_by_id({id}) with node_count={count}"
            );
            assert_eq!(
                p.c.calculate_tree_sum(id),
                0,
                "C calculate_tree_sum({id}) must be 0 with node_count={count}"
            );
            assert_eq!(
                p.c.calculate_tree_sum(id),
                p.r.calculate_tree_sum(id),
                "calculate_tree_sum({id}) with node_count={count}"
            );
        }
    }
    // NOTE: `add_tree_node` is deliberately NOT called with a negative
    // node_count. The C would evaluate `&node_table[node_count]`, i.e. write
    // BEFORE the array, corrupting whatever the linker placed there — different
    // memory in each library, so there is nothing meaningful to compare and the
    // write could take out the test process itself.
    p.c.set_node_count(0);
    p.r.set_node_count(0);
}

// ---------------------------------------------------------------------------
// Rows 8, 21 — add_tree_node capacity limit
// ---------------------------------------------------------------------------

#[test]
fn err_08_add_node_table_full() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 108);
    p.reset_both();
    let l = cstr(b"full");
    for id in 1..=(MAX_NODES as c_int) {
        let v = rng.next_i32_mixed();
        assert_eq!(
            p.c.add_tree_node(id, v, -1, &l),
            p.r.add_tree_node(id, v, -1, &l)
        );
    }
    assert_eq!(p.c.get_node_count(), MAX_NODES as c_int);
    let before_c = p.c.node_table_image();
    let before_r = p.r.node_table_image();
    for _ in 0..2_000 {
        let id = rng.next_i32_mixed();
        let v = rng.next_i32_mixed();
        let par = if rng.below(2) == 0 { -1 } else { (rng.below(50) as c_int) + 1 };
        let lab = cstr(b"overflowing-label-that-should-never-be-written");
        let cv = p.c.add_tree_node(id, v, par, &lab);
        let rv = p.r.add_tree_node(id, v, par, &lab);
        assert_eq!(cv, -1, "C must reject with -1 when the table is full");
        assert_eq!(cv, rv, "add_tree_node when full");
        assert_eq!(p.c.get_node_count(), MAX_NODES as c_int, "count must not move");
        assert_eq!(p.r.get_node_count(), MAX_NODES as c_int);
        assert_eq!(p.c.node_table_image(), before_c, "C table must be untouched");
        assert_eq!(p.r.node_table_image(), before_r, "Rust table must be untouched");
    }
    // counts strictly above MAX_NODES are rejected too
    for count in [50, 51, 100, i32::MAX] {
        p.c.set_node_count(count);
        p.r.set_node_count(count);
        let lab = cstr(b"x");
        assert_eq!(p.c.add_tree_node(1, 1, -1, &lab), -1);
        assert_eq!(
            p.c.add_tree_node(1, 1, -1, &lab),
            p.r.add_tree_node(1, 1, -1, &lab),
            "add_tree_node with node_count={count}"
        );
    }
}

#[test]
fn err_21_add_node_boundary_49_50() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 121);
    for _ in 0..500 {
        p.reset_both();
        let l = cstr(b"b");
        for id in 1..=49 {
            let v = rng.next_i32_mixed();
            assert_eq!(
                p.c.add_tree_node(id, v, -1, &l),
                p.r.add_tree_node(id, v, -1, &l)
            );
        }
        assert_eq!(p.c.get_node_count(), 49);
        // index 49 (the last slot) must still succeed and return 49
        let v = rng.next_i32_mixed();
        let cv = p.c.add_tree_node(50, v, -1, &l);
        let rv = p.r.add_tree_node(50, v, -1, &l);
        assert_eq!(cv, 49, "the 50th add must return index 49");
        assert_eq!(cv, rv);
        p.assert_state_eq("boundary 49");
        // one step past: rejected
        let cv = p.c.add_tree_node(51, v, -1, &l);
        let rv = p.r.add_tree_node(51, v, -1, &l);
        assert_eq!(cv, -1, "the 51st add must be rejected");
        assert_eq!(cv, rv);
        p.assert_state_eq("boundary 50");
    }
}

// ---------------------------------------------------------------------------
// Row 9 — add_tree_node with an unresolvable parent
// ---------------------------------------------------------------------------

#[test]
fn err_09_add_missing_parent() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 109);
    for _ in 0..4_000 {
        p.reset_both();
        let l = cstr(b"root");
        let v0 = rng.next_i32_mixed();
        assert_eq!(
            p.c.add_tree_node(1, v0, -1, &l),
            p.r.add_tree_node(1, v0, -1, &l)
        );
        // parent_id != -1 and absent from the table
        let bad_parent = loop {
            let x = rng.next_i32_mixed();
            if x != -1 && x != 1 {
                break x;
            }
        };
        let id = rng.next_i32_mixed();
        let v = rng.next_i32_mixed();
        let lab = cstr(b"orphan-label");
        let cv = p.c.add_tree_node(id, v, bad_parent, &lab);
        let rv = p.r.add_tree_node(id, v, bad_parent, &lab);
        assert_eq!(cv, -1, "C must reject a missing parent with -1");
        assert_eq!(cv, rv, "add_tree_node(.., parent={bad_parent})");
        assert_eq!(p.c.get_node_count(), 1, "count must not be incremented");
        // the C leaves the half-written slot behind; the Rust must too
        p.assert_state_eq(&format!("missing parent {bad_parent} leaves slot written"));
        let cn = p.c.node(1);
        assert_eq!(cn.id, id, "C wrote the rejected node's id anyway");
        assert_eq!(cn.value, v);
        assert_eq!(cn.parent_id, bad_parent);
        assert_eq!(cn.left_child_id, -1);
        assert_eq!(cn.right_child_id, -1);
        assert_eq!(&cn.label[..12], &lab[..12].iter().map(|&b| b as i8).collect::<Vec<_>>()[..]);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — add_tree_node with a NULL label
// ---------------------------------------------------------------------------

#[test]
fn err_10_add_null_label_faults() {
    let p = Pair::open();
    p.reset_both();
    let cf = p.c.raw_add_tree_node();
    let rf = p.r.raw_add_tree_node();
    let co = outcome_of(|| unsafe {
        std::hint::black_box(cf(1, 2, -1, std::ptr::null()));
    });
    let ro = outcome_of(|| unsafe {
        std::hint::black_box(rf(1, 2, -1, std::ptr::null()));
    });
    assert!(
        matches!(co, Outcome::Signalled(_)),
        "C add_tree_node(.., NULL) is expected to fault, got {co:?}"
    );
    assert_eq!(
        co, ro,
        "add_tree_node(.., NULL): C and Rust must terminate identically"
    );
}

// ---------------------------------------------------------------------------
// Row 11 — calculate_tree_sum on an unknown id
// ---------------------------------------------------------------------------

#[test]
fn err_11_sum_missing_id_zero() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 111);
    for _ in 0..2_000 {
        p.reset_both();
        let n = rng.below(21) as c_int;
        let l = cstr(b"n");
        for id in 1..=n {
            let v = rng.next_i32_mixed();
            assert_eq!(
                p.c.add_tree_node(id, v, -1, &l),
                p.r.add_tree_node(id, v, -1, &l)
            );
        }
        for miss in [0, -1, n + 1, 12_345, i32::MIN, i32::MAX] {
            let cv = p.c.calculate_tree_sum(miss);
            assert_eq!(cv, 0, "C calculate_tree_sum({miss}) must be 0");
            assert_eq!(cv, p.r.calculate_tree_sum(miss), "calculate_tree_sum({miss})");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 12-13 — parse_operation "rejections" that are really OP_ADD
// ---------------------------------------------------------------------------

#[test]
fn err_12_parse_null_is_add() {
    let p = Pair::open();
    for _ in 0..1_000 {
        let cv = p.c.parse_operation_null();
        let rv = p.r.parse_operation_null();
        assert_eq!(cv, 1, "C parse_operation(NULL) must be OP_ADD (1)");
        assert_eq!(cv, rv, "parse_operation(NULL)");
    }
    // and the fn pointer it feeds is add_op in both
    let cop = p.c.parse_operation_null();
    assert_eq!(
        p.c.get_operation_func_identity(cop),
        p.r.get_operation_func_identity(cop)
    );
}

#[test]
fn err_13_parse_no_operator_is_add() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 113);
    let empty = cstr(b"");
    assert_eq!(p.c.parse_operation(&empty), 1, "C parse_operation(\"\") must be OP_ADD");
    assert_eq!(p.c.parse_operation(&empty), p.r.parse_operation(&empty));
    const ALPHA: &[u8] = b"abcDEF012 \t\r\n~!@#$^&()_=[]{}|;:'\",.<>?\\`";
    for _ in 0..20_000 {
        let len = rng.below(30) as usize;
        let body: Vec<u8> = (0..len)
            .map(|_| ALPHA[rng.below(ALPHA.len() as u32) as usize])
            .collect();
        let s = cstr(&body);
        let cv = p.c.parse_operation(&s);
        assert_eq!(cv, 1, "C must fall back to OP_ADD for {:?}", String::from_utf8_lossy(&body));
        assert_eq!(cv, p.r.parse_operation(&s));
    }
    // bytes >= 0x80 (negative char) must not be mistaken for an operator
    for b in 0x80u8..=0xFF {
        let s = cstr(&[b, b, b]);
        assert_eq!(
            p.c.parse_operation(&s),
            p.r.parse_operation(&s),
            "parse_operation with high byte {b:#x}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 14 — out-of-range enum values across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err_14_get_op_func_out_of_range_enum() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 114);
    // one step past each end of the valid 1..=5 range, plus the extremes
    let mut ops: Vec<c_int> = vec![0, -1, 6, 7, 100, -100, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    for _ in 0..2_000 {
        let v = rng.next_i32_mixed();
        if !(1..=5).contains(&v) {
            ops.push(v);
        }
    }
    for op in ops {
        // the default: arm must yield add_op, i.e. a + b
        let cv = p.c.get_operation_func_probe(op, 10, 3);
        let rv = p.r.get_operation_func_probe(op, 10, 3);
        assert_eq!(cv, 13, "C get_operation_func({op}) must fall back to add_op");
        assert_eq!(cv, rv, "get_operation_func({op}) probe");
        assert_eq!(
            p.c.get_operation_func_identity(op),
            "add_op",
            "C get_operation_func({op}) identity"
        );
        assert_eq!(
            p.c.get_operation_func_identity(op),
            p.r.get_operation_func_identity(op),
            "get_operation_func({op}) symbol identity"
        );
        // and over randomized operands
        for _ in 0..5 {
            let (a, b) = (rng.next_i32_mixed(), rng.next_i32_mixed());
            assert_eq!(
                p.c.get_operation_func_probe(op, a, b),
                p.r.get_operation_func_probe(op, a, b),
                "get_operation_func({op})({a},{b})"
            );
        }
    }
    // parse_operation's return value is also just an int; feed it straight back
    for _ in 0..2_000 {
        let len = rng.below(6) as usize;
        let body: Vec<u8> = (0..len).map(|_| b"+*-/%zq"[rng.below(7) as usize]).collect();
        let s = cstr(&body);
        let cop = p.c.parse_operation(&s);
        let rop = p.r.parse_operation(&s);
        assert_eq!(cop, rop);
        assert_eq!(
            p.c.get_operation_func_probe(cop, 10, 3),
            p.r.get_operation_func_probe(rop, 10, 3)
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 15-17 — inreftree's internal rejection / fallback branches
// ---------------------------------------------------------------------------

#[test]
fn err_15_inreftree_param2_zero_retargets() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 115);
    p.reset_both();
    for _ in 0..20_000 {
        let (a, c, d) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        let cv = p.c.inreftree(a, 0, c, d);
        let rv = p.r.inreftree(a, 0, c, d);
        assert_eq!(cv, rv, "inreftree({a}, 0, {c}, {d})");
        // node 2 exists but has value 0, so the target was reset to id 1
        assert_eq!(p.c.node(1).id, 2);
        assert_eq!(p.c.node(1).value, 0);
        // the sum is unaffected by the retarget; only the 2nd operand changes
        let sum = p.c.calculate_tree_sum(1);
        assert_eq!(sum, p.r.calculate_tree_sum(1));
        let op = match sum.rem_euclid(4) {
            0 => 1,
            1 => 2,
            2 => 3,
            _ => 5,
        };
        let expect = if sum < 0 && sum % 4 != 0 {
            sum.wrapping_add(1) // negative index path -> add_op with target 1
        } else {
            match op {
                1 => sum.wrapping_add(1),
                2 => sum.wrapping_mul(1),
                3 => sum.wrapping_sub(1),
                _ => sum.wrapping_rem(1),
            }
        };
        assert_eq!(cv, expect, "retargeted to id 1 for sum={sum}");
    }
}

#[test]
fn err_16_inreftree_target_null_unreachable() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 116);
    p.reset_both();
    // The `target == NULL` half of the check needs target_id == -1, which needs
    // no label in node_table[0..node_count) to contain 'l'. inreftree always
    // installs "left" at index 1, so it cannot happen. Prove the invariant
    // holds in both libraries, and that the *sibling* branch is what fires.
    for _ in 0..20_000 {
        let (a, b, c, d) = (
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
            rng.next_i32_mixed(),
        );
        assert_eq!(p.c.inreftree(a, b, c, d), p.r.inreftree(a, b, c, d));
        for lib in [&p.c, &p.r] {
            assert_eq!(lib.get_node_count(), 4, "{}: node_count", lib.name);
            let n1 = lib.node(1);
            assert_eq!(n1.id, 2, "{}: node 1 id", lib.name);
            let lab: Vec<u8> = n1.label.iter().map(|&x| x as u8).collect();
            assert_eq!(&lab[..5], b"left\0".as_slice(), "{}: node 1 label", lib.name);
            assert!(
                lib.find_node_by_id(2).is_some(),
                "{}: target lookup can never be NULL",
                lib.name
            );
        }
    }
}

#[test]
fn err_17_inreftree_negative_modulo_oob_read() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 117);
    p.reset_both();
    // tree_sum % 4 in {-1, -2, -3} indexes BEFORE the "+*-%" literal.
    for class in [1i32, 2, 3] {
        for _ in 0..10_000 {
            let mag = (rng.next_u32() % 200_000) as i32 * 4;
            let sum = -(mag + class);
            let a = rng.next_i32() >> 3;
            let c = rng.next_i32() >> 3;
            let b = 1; // non-zero so target_id stays 2
            let d = sum.wrapping_sub(a).wrapping_sub(b).wrapping_sub(c);
            let cv = p.c.inreftree(a, b, c, d);
            let rv = p.r.inreftree(a, b, c, d);
            assert_eq!(p.c.calculate_tree_sum(1), sum, "sum setup");
            assert_eq!(cv, rv, "inreftree sum={sum} (sum%4={})", sum % 4);
            // the byte read is not an operator, so parse_operation -> OP_ADD
            assert_eq!(cv, sum.wrapping_add(2), "negative-index path must select add_op");
        }
    }
    // and the same with param2 == 0 (target 1)
    for class in [1i32, 2, 3] {
        for _ in 0..5_000 {
            let mag = (rng.next_u32() % 200_000) as i32 * 4;
            let sum = -(mag + class);
            let a = rng.next_i32() >> 3;
            let c = rng.next_i32() >> 3;
            let d = sum.wrapping_sub(a).wrapping_sub(c);
            let cv = p.c.inreftree(a, 0, c, d);
            let rv = p.r.inreftree(a, 0, c, d);
            assert_eq!(cv, rv, "inreftree({a},0,{c},{d}) sum={sum}");
            assert_eq!(cv, sum.wrapping_add(1));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 18-19 — provably dead re-checks
// ---------------------------------------------------------------------------

#[test]
fn err_18_dead_parent_id_recheck() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 118);
    // `parent->id != parent_id` can never fire: find_node_by_id only returns a
    // node whose id already equals the argument. Assert the invariant directly
    // through the exports, over randomized tables incl. duplicate ids.
    for _ in 0..4_000 {
        p.reset_both();
        let count = (rng.below(MAX_NODES as u32) + 1) as usize;
        let l = cstr(b"l");
        for _ in 0..count {
            let id = (rng.below(6) as c_int) + 1; // heavy duplication
            let v = rng.next_i32_mixed();
            assert_eq!(
                p.c.add_tree_node(id, v, -1, &l),
                p.r.add_tree_node(id, v, -1, &l)
            );
        }
        for id in [1, 2, 3, 4, 5, 6, 7, -1, 0] {
            let idx = p.c.find_node_by_id(id);
            assert_eq!(idx, p.r.find_node_by_id(id));
            if let Some(i) = idx {
                assert_eq!(p.c.node(i as usize).id, id, "C: found node has a different id");
                assert_eq!(p.r.node(i as usize).id, id, "Rust: found node has a different id");
            }
        }
    }
}

#[test]
fn err_19_dead_node_id_recheck() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 119);
    // Same invariant seen from calculate_tree_sum: a found node's id always
    // matches, so the `node->id != node_id` half of the guard is dead and the
    // only way to get 0 is a genuine miss.
    for _ in 0..4_000 {
        p.reset_both();
        let count = (rng.below(MAX_NODES as u32) + 1) as usize;
        let l = cstr(b"l");
        let mut vals = Vec::new();
        for i in 0..count {
            let id = i as c_int + 1;
            let v = rng.next_i32_mixed();
            vals.push(v);
            assert_eq!(
                p.c.add_tree_node(id, v, -1, &l),
                p.r.add_tree_node(id, v, -1, &l)
            );
        }
        for (i, &v) in vals.iter().enumerate() {
            let id = i as c_int + 1;
            let cv = p.c.calculate_tree_sum(id);
            assert_eq!(cv, p.r.calculate_tree_sum(id));
            // leaves (all roots here, no children linked) sum to their own value
            assert_eq!(cv, v, "leaf {id} sum must be its own value, never 0-by-recheck");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — oversized / unterminated buffers
// ---------------------------------------------------------------------------

#[test]
fn err_20_oversized_label_truncates() {
    let p = Pair::open();
    let mut rng = Rng::new(SEED ^ 120);
    for len in [31usize, 32, 33, 40, 63, 64, 200, 1000] {
        for _ in 0..200 {
            p.reset_both();
            let mut body: Vec<u8> = (0..len)
                .map(|_| b"abcdefghijklmnopqrstuvwxyz"[rng.below(26) as usize])
                .collect();
            body.push(0);
            let v = rng.next_i32_mixed();
            let cv = p.c.add_tree_node(9, v, -1, &body);
            let rv = p.r.add_tree_node(9, v, -1, &body);
            assert_eq!(cv, 0, "truncation is not an error");
            assert_eq!(cv, rv, "add_tree_node with a {len}-byte label");
            p.assert_state_eq(&format!("{len}-byte label"));
            let cn = p.c.node(0);
            assert_eq!(cn.label[31], 0, "label[31] must be forced to NUL");
            for k in 0..31 {
                assert_eq!(cn.label[k] as u8, body[k], "byte {k} of a {len}-byte label");
            }
            // no write past the 32-byte label field
            assert_eq!(p.c.node(1).id, 0, "C wrote past node 0");
            assert_eq!(p.r.node(1).id, 0, "Rust wrote past node 0");
        }
    }
    // a buffer whose only NUL is the very last byte -> scanned to the end
    for len in [1usize, 2, 31, 32, 100] {
        let mut body = vec![b'%'; len];
        body.push(0);
        assert_eq!(
            p.c.parse_operation(&body),
            p.r.parse_operation(&body),
            "parse_operation over a {len}-byte run"
        );
    }
}
