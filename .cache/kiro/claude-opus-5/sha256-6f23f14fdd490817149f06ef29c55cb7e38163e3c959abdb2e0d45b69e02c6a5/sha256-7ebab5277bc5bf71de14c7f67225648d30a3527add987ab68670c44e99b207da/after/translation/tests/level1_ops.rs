//! Level 1: the three leaf `operation_fn` implementations, called through the
//! `.so` exports of both libraries.

mod common;

use common::*;
use std::ffi::c_int;

const OPS: &[&str] = &["process_value", "double_value", "triple_value"];

fn interesting_values() -> Vec<c_int> {
    let mut v = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        10,
        -10,
        99,
        100,
        999,
        1000,
        1001,
        65534,
        65535,
        65536,
        -65535,
        1_000_000,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        c_int::MAX / 2,
        c_int::MAX / 3,
        c_int::MIN / 2,
        c_int::MIN / 3,
    ];
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..200 {
        v.push(rng.next_u32() as c_int);
    }
    v
}

fn ops_match_on_interesting_values() {
    for name in OPS {
        for &val in &interesting_values() {
            compare_op(name, val, 0, std::ptr::null_mut());
        }
    }
}

fn ops_ignore_extra_arguments() {
    // The C implementations cast both extra parameters to void; make sure the
    // Rust versions do not let them influence the result either.
    let mut sentinel: u64 = 0xDEAD_BEEF;
    let ctx = &mut sentinel as *mut u64 as *mut std::ffi::c_void;
    for name in OPS {
        for &val in &[0, 1, -7, 12345, c_int::MAX, c_int::MIN] {
            for &extra in &[0, 1, -1, c_int::MAX, c_int::MIN] {
                compare_op(name, val, extra, std::ptr::null_mut());
                compare_op(name, val, extra, ctx);
            }
        }
    }
}

fn ops_produce_no_stdout() {
    let l = libs();
    for name in OPS {
        let f = op(&l.rust, name);
        let (_, out) = capture_stdout(|| unsafe { f(5, 0, std::ptr::null_mut()) });
        assert!(out.is_empty(), "{name} unexpectedly wrote to stdout");
    }
}

/// Single entry point: the stdout capture redirects the process-wide fd 1, so
/// no other libtest thread may be writing while a case runs.
#[test]
fn level1_ops_all() {
    eprintln!("  case group: ops_match_on_interesting_values");
    ops_match_on_interesting_values();
    eprintln!("  case group: ops_ignore_extra_arguments");
    ops_ignore_extra_arguments();
    eprintln!("  case group: ops_produce_no_stdout");
    ops_produce_no_stdout();
}
