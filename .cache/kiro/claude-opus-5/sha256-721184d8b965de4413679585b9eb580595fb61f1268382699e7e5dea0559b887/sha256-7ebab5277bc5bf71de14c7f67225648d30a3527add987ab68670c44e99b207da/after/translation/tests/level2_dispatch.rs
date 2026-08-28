//! Level 2: dispatch (`get_operation`) and the printing wrapper
//! (`execute_operation`).

mod common;

use common::*;
use std::ffi::{c_char, c_int};

const OP_NAMES: [&str; 4] = [
    "multiply_with_static",
    "add_with_static",
    "xor_operation",
    "shift_with_static",
];

/// `get_operation` must return NULL outside [0, 4) and, inside that range, the
/// address of the corresponding exported function in the *same* library.
fn get_operation_table_matches() {
    let libs = impls();

    for which in BOTH {
        let get: libloading::Symbol<FnGetOperation> = libs.sym(which, "get_operation");
        let expected: Vec<usize> = OP_NAMES
            .iter()
            .map(|n| {
                let s: libloading::Symbol<FnBinop> = libs.sym(which, n);
                (*s) as usize
            })
            .collect();

        for opcode in 0..4 {
            let (got, out) = capture_stdout(|| unsafe { get(opcode) });
            assert!(out.is_empty(), "{which:?} get_operation printed: {}", show(&out));
            let got = got.unwrap_or_else(|| panic!("{which:?} get_operation({opcode}) == NULL"));
            assert_eq!(
                got as usize, expected[opcode as usize],
                "{which:?} get_operation({opcode}) is not {}",
                OP_NAMES[opcode as usize]
            );
        }
    }
}

fn get_operation_out_of_range_is_null_in_both() {
    let libs = impls();
    let c: libloading::Symbol<FnGetOperation> = libs.sym(Which::C, "get_operation");
    let r: libloading::Symbol<FnGetOperation> = libs.sym(Which::Rust, "get_operation");

    let mut codes: Vec<c_int> = vec![-1, -2, -100, 4, 5, 6, 100, c_int::MIN, c_int::MAX];
    codes.extend(sample_ints());

    for opcode in codes {
        let (cv, cout) = capture_stdout(|| unsafe { c(opcode) });
        let (rv, rout) = capture_stdout(|| unsafe { r(opcode) });
        assert_eq!(
            cv.is_none(),
            rv.is_none(),
            "get_operation({opcode}): C null={} Rust null={}",
            cv.is_none(),
            rv.is_none()
        );
        assert_eq!(cout, rout, "get_operation({opcode}) stdout differs");
        // Where non-NULL, the two must compute the same thing.
        if let (Some(cf), Some(rf)) = (cv, rv) {
            for (a, b) in [(3, 5), (-7, 11), (c_int::MIN, c_int::MAX), (0, 0)] {
                assert_eq!(
                    unsafe { cf(a, b) },
                    unsafe { rf(a, b) },
                    "get_operation({opcode})({a}, {b}) differs"
                );
            }
        }
    }
}

/// Repeated calls must be stable (the C version lazily initialises a static
/// table on first use).
fn get_operation_is_idempotent() {
    let libs = impls();
    for which in BOTH {
        let get: libloading::Symbol<FnGetOperation> = libs.sym(which, "get_operation");
        let first: Vec<Option<usize>> = (-2..6)
            .map(|i| unsafe { get(i) }.map(|f| f as usize))
            .collect();
        for _ in 0..5 {
            let again: Vec<Option<usize>> = (-2..6)
                .map(|i| unsafe { get(i) }.map(|f| f as usize))
                .collect();
            assert_eq!(first, again, "{which:?} get_operation not idempotent");
        }
    }
}

fn cstr(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

/// `execute_operation` for every opcode, comparing return value *and* the exact
/// bytes printed (the `LOG_VALUE` macro stringifies the parameter names `a`/`b`).
fn execute_operation_matches_for_each_op() {
    let libs = impls();
    let c_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::C, "get_operation");
    let r_get: libloading::Symbol<FnGetOperation> = libs.sym(Which::Rust, "get_operation");
    let c_exec: libloading::Symbol<FnExecuteOperation> = libs.sym(Which::C, "execute_operation");
    let r_exec: libloading::Symbol<FnExecuteOperation> = libs.sym(Which::Rust, "execute_operation");

    let names = ["XOR", "SHIFT", "", "op with spaces", "%d%s%%", "MULT"];
    let vals = sample_ints();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);

    for opcode in 0..4 {
        let cf = unsafe { c_get(opcode) };
        let rf = unsafe { r_get(opcode) };
        for name in names {
            let n = cstr(name);
            for _ in 0..60 {
                let a = vals[(rng.next_u64() as usize) % vals.len()];
                let b = vals[(rng.next_u64() as usize) % vals.len()];
                let (cv, cout) = capture_stdout(|| unsafe { c_exec(cf, a, b, n.as_ptr()) });
                let (rv, rout) = capture_stdout(|| unsafe { r_exec(rf, a, b, n.as_ptr()) });
                assert_eq!(cv, rv, "execute_operation(op{opcode}, {a}, {b}, {name:?}) value");
                assert_eq!(
                    cout,
                    rout,
                    "execute_operation(op{opcode}, {a}, {b}, {name:?}) stdout:\nC   ={}\nRust={}",
                    show(&cout),
                    show(&rout)
                );
            }
        }
    }
}

fn execute_operation_null_func_matches() {
    let libs = impls();
    let c_exec: libloading::Symbol<FnExecuteOperation> = libs.sym(Which::C, "execute_operation");
    let r_exec: libloading::Symbol<FnExecuteOperation> = libs.sym(Which::Rust, "execute_operation");

    for name in ["XOR", "SHIFT", "", "weird\tname"] {
        let n = cstr(name);
        let (cv, cout) = capture_stdout(|| unsafe { c_exec(None, 1, 2, n.as_ptr()) });
        let (rv, rout) = capture_stdout(|| unsafe { r_exec(None, 1, 2, n.as_ptr()) });
        assert_eq!(cv, rv, "NULL func return value for {name:?}");
        assert_eq!(
            cout,
            rout,
            "NULL func stdout for {name:?}:\nC   ={}\nRust={}",
            show(&cout),
            show(&rout)
        );
        assert_eq!(cv, 0, "C contract: NULL func returns 0");
    }
}

fn main() {
    let mut r = Runner::new();
    r.case("get_operation_table_matches", get_operation_table_matches);
    r.case("get_operation_out_of_range_is_null_in_both", get_operation_out_of_range_is_null_in_both);
    r.case("get_operation_is_idempotent", get_operation_is_idempotent);
    r.case("execute_operation_matches_for_each_op", execute_operation_matches_for_each_op);
    r.case("execute_operation_null_func_matches", execute_operation_null_func_matches);
    r.finish();
}
