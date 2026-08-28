//! Level 1: leaf functions with no global state — `add_three`, `multiply_add`,
//! and `apply_operation` dispatching to them.

mod common;

use common::*;
use std::ffi::c_int;

/// A callback defined in the *test* binary, passed into both libraries to check
/// that the `extern "C"` calling convention of the exported `apply_operation`
/// wrapper is identical on both sides.
unsafe extern "C" fn foreign_op(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(3)
        .wrapping_sub(b.wrapping_mul(5))
        .wrapping_add(c)
}

#[test]
fn pure_leaf_functions_match() {
    let libs = load();

    let (add_c, add_r) = libs.pair::<FnTernary>("add_three");
    let (mul_c, mul_r) = libs.pair::<FnTernary>("multiply_add");
    let (apply_c, apply_r) = libs.pair::<FnApplyOperation>("apply_operation");

    // --- add_three / multiply_add over the cross product of interesting ints
    for &a in INTS {
        for &b in INTS {
            for &c in INTS {
                let (ec, er) = unsafe { (add_c(a, b, c), add_r(a, b, c)) };
                assert_eq!(ec, er, "add_three({a}, {b}, {c})");

                let (ec, er) = unsafe { (mul_c(a, b, c), mul_r(a, b, c)) };
                assert_eq!(ec, er, "multiply_add({a}, {b}, {c})");
            }
        }
    }

    // --- apply_operation, each library dispatching through its *own* op
    for &a in INTS {
        for &b in INTS {
            for &c in INTS {
                let ec = unsafe { apply_c(*add_c, a, b, c) };
                let er = unsafe { apply_r(*add_r, a, b, c) };
                assert_eq!(ec, er, "apply_operation(add_three, {a}, {b}, {c})");

                let ec = unsafe { apply_c(*mul_c, a, b, c) };
                let er = unsafe { apply_r(*mul_r, a, b, c) };
                assert_eq!(ec, er, "apply_operation(multiply_add, {a}, {b}, {c})");
            }
        }
    }

    // --- apply_operation with a callback owned by neither library
    for &a in INTS {
        for &b in INTS {
            for &c in INTS {
                let ec = unsafe { apply_c(foreign_op, a, b, c) };
                let er = unsafe { apply_r(foreign_op, a, b, c) };
                assert_eq!(ec, er, "apply_operation(foreign_op, {a}, {b}, {c})");
                assert_eq!(ec, unsafe { foreign_op(a, b, c) }, "callback passthrough");
            }
        }
    }

    // --- cross-library dispatch: C's apply_operation invoking Rust's op and
    //     vice versa. Both ops are pure, so the results must agree.
    for &a in INTS {
        for &b in INTS {
            let ec = unsafe { apply_c(*add_r, a, b, 42) };
            let er = unsafe { apply_r(*add_c, a, b, 42) };
            assert_eq!(ec, er, "cross-dispatch add_three({a}, {b}, 42)");
        }
    }
}
