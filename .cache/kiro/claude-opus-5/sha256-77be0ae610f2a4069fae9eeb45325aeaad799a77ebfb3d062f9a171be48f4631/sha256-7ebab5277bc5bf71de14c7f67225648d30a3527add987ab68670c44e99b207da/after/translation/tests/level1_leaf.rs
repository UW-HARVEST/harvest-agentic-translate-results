//! Level 1: leaf functions with no dependency on global state.

mod common;

use common::*;
use std::ffi::c_int;

/// `divide_op`/`modulo_op` with these operands execute `idiv INT_MIN, -1`,
/// which raises SIGFPE in the C build. A trap has no comparable return value,
/// so the pair is excluded from the differential comparison.
fn traps_in_c(symbol: &str, a: c_int, b: c_int) -> bool {
    matches!(symbol, "divide_op" | "modulo_op") && a == c_int::MIN && b == -1
}

#[test]
fn arithmetic_ops_match() {
    let p = load();
    let vals = interesting_ints();
    let mut checked = 0usize;

    for symbol in ["add_op", "multiply_op", "subtract_op", "divide_op", "modulo_op"] {
        for &a in &vals {
            for &b in &vals {
                if traps_in_c(symbol, a, b) {
                    continue;
                }
                let c = p.c.call_op(symbol, a, b, 0, 0);
                let r = p.rs.call_op(symbol, a, b, 0, 0);
                assert_eq!(c, r, "{symbol}({a}, {b}, 0, 0): C={c} Rust={r}");
                checked += 1;
            }
        }
    }
    assert!(checked > 5000, "expected a broad sweep, got {checked}");
}

#[test]
fn arithmetic_ops_ignore_unused_params() {
    let p = load();
    // The trailing two parameters are unused in C; confirm both agree that
    // varying them changes nothing.
    for symbol in ["add_op", "multiply_op", "subtract_op", "divide_op", "modulo_op"] {
        for &(a, b) in &[(7, 3), (-7, 3), (7, -3), (-7, -3), (0, 0), (5, 0), (0, 5)] {
            for &(u1, u2) in &[(0, 0), (1, -1), (c_int::MAX, c_int::MIN), (-99, 42)] {
                let c = p.c.call_op(symbol, a, b, u1, u2);
                let r = p.rs.call_op(symbol, a, b, u1, u2);
                assert_eq!(c, r, "{symbol}({a}, {b}, {u1}, {u2})");
            }
        }
    }
}

#[test]
fn division_by_zero_matches() {
    let p = load();
    for &a in &interesting_ints() {
        for symbol in ["divide_op", "modulo_op"] {
            let c = p.c.call_op(symbol, a, 0, 0, 0);
            let r = p.rs.call_op(symbol, a, 0, 0, 0);
            assert_eq!(c, r, "{symbol}({a}, 0)");
            assert_eq!(c, 0, "{symbol} must guard against b == 0");
        }
    }
}

#[test]
fn parse_operation_matches() {
    let p = load();

    // NULL is special-cased by the C code (it returns OP_ADD before any
    // dereference, thanks to `||` short-circuiting).
    assert_eq!(p.c.parse_operation(None), p.rs.parse_operation(None));
    assert_eq!(p.c.parse_operation(None), 1);

    let mut cases: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"+\0".to_vec(),
        b"*\0".to_vec(),
        b"-\0".to_vec(),
        b"/\0".to_vec(),
        b"%\0".to_vec(),
        // Precedence: the first matching test in the C chain wins.
        b"%/-*+\0".to_vec(),
        b"/-*\0".to_vec(),
        b"-%\0".to_vec(),
        b"%-\0".to_vec(),
        b"/%\0".to_vec(),
        b"%/\0".to_vec(),
        b"abc\0".to_vec(),
        b"root\0".to_vec(),
        b"left-left\0".to_vec(),
        b"a+b\0".to_vec(),
        b"12*34\0".to_vec(),
        b"   \0".to_vec(),
        b"\x7f\xff\xfe\0".to_vec(),
        b"++++\0".to_vec(),
        b"very long string with no operators at all in it whatsoever\0".to_vec(),
    ];
    // Every single byte value 1..=255 as a one-character string.
    for b in 1u8..=255 {
        cases.push(vec![b, 0]);
    }
    // All two-byte combinations drawn from the operator alphabet plus filler.
    let alphabet = [b'+', b'*', b'-', b'/', b'%', b'x', b'0'];
    for &x in &alphabet {
        for &y in &alphabet {
            cases.push(vec![x, y, 0]);
        }
    }

    for case in &cases {
        let c = p.c.parse_operation(Some(case));
        let r = p.rs.parse_operation(Some(case));
        assert_eq!(c, r, "parse_operation({case:?}): C={c} Rust={r}");
        assert!((1..=5).contains(&c), "unexpected op {c} for {case:?}");
    }
}

#[test]
fn get_operation_func_dispatches_identically() {
    let p = load();

    // Each library must return a pointer to *its own* corresponding function,
    // so compare by resolving the expected symbol in the same library.
    let expected = |op: c_int| -> &'static str {
        match op {
            2 => "multiply_op",
            3 => "subtract_op",
            4 => "divide_op",
            5 => "modulo_op",
            _ => "add_op", // 1 and the default branch
        }
    };

    let mut ops: Vec<c_int> = (-5..=10).collect();
    ops.extend([c_int::MIN, c_int::MAX, 1000, -1000, 6, 0]);

    for op in ops {
        let want = expected(op);
        let c_got = p.c.get_operation_func(op);
        let r_got = p.rs.get_operation_func(op);

        assert_eq!(
            c_got,
            p.c.op_ptr(want),
            "C get_operation_func({op}) should be {want}"
        );
        assert_eq!(
            r_got,
            p.rs.op_ptr(want),
            "Rust get_operation_func({op}) should be {want}"
        );
        assert!(!c_got.is_null() && !r_got.is_null(), "op {op} returned NULL");

        // And behaviourally: the returned function pointers agree on results.
        for &(a, b) in &[(12, 5), (-12, 5), (12, -5), (0, 3), (7, 0)] {
            let cf: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int =
                unsafe { std::mem::transmute(c_got) };
            let rf: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int =
                unsafe { std::mem::transmute(r_got) };
            let cv = unsafe { cf(a, b, 0, 0) };
            let rv = unsafe { rf(a, b, 0, 0) };
            assert_eq!(cv, rv, "dispatched op {op} on ({a}, {b})");
        }
    }
}
