// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md.  Every call is made twice (C `.so`, Rust
// `.so`) through dlsym'ed function pointers; the return value *and* the bytes
// the call writes to fd 1 must be identical.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// helpers that normalise the pointer-returning entry points into comparable
// plain data
// ---------------------------------------------------------------------------

/// `create_result_string`: returns `(was_null, string_bytes)`.
fn call_crs(api: &Api, op: Option<&[u8]>, val: c_int) -> (bool, Vec<u8>) {
    unsafe {
        let p = match op {
            Some(b) => {
                assert_eq!(*b.last().unwrap(), 0, "test string must be NUL terminated");
                b.as_ptr() as *const c_char
            }
            None => std::ptr::null(),
        };
        let out = (api.create_result_string)(p, val);
        if out.is_null() {
            (true, Vec::new())
        } else {
            let s = cstr_bytes(out);
            libc_free(out);
            (false, s)
        }
    }
}

/// `multiply_with_log`: returns `(ret, out_ptr_was_null, log_bytes)`.
fn call_mwl(api: &Api, a: c_int, b: c_int) -> (c_int, bool, Vec<u8>) {
    unsafe {
        // Poison the out-parameter so we can tell it really was written.
        let mut log: *mut c_char = 0x1 as *mut c_char;
        let ret = (api.multiply_with_log)(a, b, &mut log);
        if log.is_null() {
            (ret, true, Vec::new())
        } else {
            assert_ne!(log as usize, 1, "out-parameter was not written");
            let s = cstr_bytes(log);
            libc_free(log);
            (ret, false, s)
        }
    }
}

fn call_cas(api: &Api, data: &[c_int], count: c_int) -> c_int {
    let mut buf = data.to_vec();
    unsafe { (api.copy_and_sum)(buf.as_mut_ptr(), count) }
}

fn call_cmp(api: &Api, a: &[u8], b: &[u8]) -> c_int {
    unsafe {
        (api.compare_operations)(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char)
    }
}

// ===========================================================================
// harness self-test: prove the comparison is not vacuous — the capture really
// observes library output, and a difference really does fail the comparison.
// ===========================================================================

#[test]
fn harness_detects_differences() {
    let (c, r) = both();

    // Two different C calls must produce different captured stdout, otherwise
    // `capture()` is not observing the library's printf at all.
    let (_, out1) = capture(|| unsafe { (c.complexmode)(1, 3, 5, 7) });
    let (_, out3) = capture(|| unsafe { (c.complexmode)(3, 3, 5, 7) });
    assert_ne!(out1, out3, "capture() is not observing library output");
    assert_eq!(out1, b"Mode 1: Addition\nResult: 8\nOperation performed: addition\n");
    assert_eq!(
        out3,
        b"Mode 3: Array Sum\nResult: 15\nOperation performed: array_sum\n"
    );

    // The same call through the Rust .so must reproduce those exact bytes.
    let (_, rout1) = capture(|| unsafe { (r.complexmode)(1, 3, 5, 7) });
    assert_eq!(out1, rout1);

    // And a deliberately mismatched comparison must panic.
    let bad = std::panic::catch_unwind(|| {
        let (c, r) = both();
        let (cv, cout) = capture(|| unsafe { (c.complexmode)(1, 3, 5, 7) });
        let (rv, rout) = capture(|| unsafe { (r.complexmode)(2, 3, 5, 7) });
        assert_eq!(cv, rv, "control");
        assert_eq!(cout, rout, "control");
    });
    assert!(bad.is_err(), "the differential assertions never fail => vacuous");
}

// ===========================================================================
// check_permissions — CONFIGS rows 1..7
// ===========================================================================

#[test]
fn row01_check_permissions_required_zero() {
    let mut rng = Rng::new(0x1001);
    for _ in 0..500 {
        let perms = rng.i32();
        diff(&format!("check_permissions({perms}, 0)"), |a| unsafe {
            (a.check_permissions)(perms, 0)
        });
    }
}

#[test]
fn row02_check_permissions_exact_match() {
    let mut rng = Rng::new(0x1002);
    for _ in 0..500 {
        let p = rng.i32();
        diff(&format!("check_permissions({p}, {p})"), |a| unsafe {
            (a.check_permissions)(p, p)
        });
    }
}

#[test]
fn row03_check_permissions_required_subset() {
    let mut rng = Rng::new(0x1003);
    for _ in 0..500 {
        let perms = rng.i32();
        // clear a random subset of bits from `perms` to build a strict subset
        let required = perms & rng.i32();
        diff(
            &format!("check_permissions({perms}, {required}) subset"),
            |a| unsafe { (a.check_permissions)(perms, required) },
        );
    }
}

#[test]
fn row04_check_permissions_partial_overlap() {
    let mut rng = Rng::new(0x1004);
    for _ in 0..500 {
        let perms = rng.i32();
        // guarantee at least one required bit is missing from perms
        let missing_bit = 1i32 << (rng.below(31) as u32);
        let required = (perms & rng.i32()) | missing_bit;
        let perms = perms & !missing_bit;
        diff(
            &format!("check_permissions({perms}, {required}) partial"),
            |a| unsafe { (a.check_permissions)(perms, required) },
        );
    }
}

#[test]
fn row05_check_permissions_disjoint() {
    let mut rng = Rng::new(0x1005);
    for _ in 0..500 {
        let perms = rng.i32();
        let required = !perms & rng.i32();
        diff(
            &format!("check_permissions({perms}, {required}) disjoint"),
            |a| unsafe { (a.check_permissions)(perms, required) },
        );
    }
}

#[test]
fn row06_check_permissions_boundary_words() {
    for &p in EDGE_I32 {
        for &q in EDGE_I32 {
            diff(&format!("check_permissions({p}, {q}) edge"), |a| unsafe {
                (a.check_permissions)(p, q)
            });
        }
    }
}

#[test]
fn row07_check_permissions_macro_values() {
    for &p in EDGE_PERMS {
        for &q in EDGE_PERMS {
            diff(&format!("check_permissions({p:o}, {q:o}) perms"), |a| unsafe {
                (a.check_permissions)(p, q)
            });
        }
    }
}

// ===========================================================================
// safe_add — CONFIGS rows 8..12
// ===========================================================================

fn safe_add_sweep(seed: u64, perms_set: &[c_int], label: &str) {
    let mut rng = Rng::new(seed);
    for &perms in perms_set {
        for _ in 0..120 {
            let a0 = rng.i32();
            let b0 = rng.i32();
            diff(
                &format!("{label} safe_add({a0}, {b0}, {perms:o})"),
                |api| unsafe { (api.safe_add)(a0, b0, perms) },
            );
        }
        // plus the small-value neighbourhood
        for _ in 0..60 {
            let a0 = rng.small(1000);
            let b0 = rng.small(1000);
            diff(
                &format!("{label} safe_add small({a0}, {b0}, {perms:o})"),
                |api| unsafe { (api.safe_add)(a0, b0, perms) },
            );
        }
    }
}

#[test]
fn row08_safe_add_granted() {
    safe_add_sweep(0x2001, &[0o600, 0o644, 0o777, -1, 0o606, 0o1600], "granted");
}

#[test]
fn row09_safe_add_missing_write() {
    safe_add_sweep(0x2002, &[0o400, 0o500, 0o444], "no-write");
}

#[test]
fn row10_safe_add_missing_read() {
    safe_add_sweep(0x2003, &[0o200, 0o300, 0o222], "no-read");
}

#[test]
fn row11_safe_add_missing_both() {
    safe_add_sweep(0x2004, &[0, 0o100, 0o077, 0o111], "no-rw");
}

#[test]
fn row12_safe_add_wrapping_sums() {
    let perms_all = [0o600, 0o644, -1, i32::MAX, 0o400, 0, i32::MIN];
    for &perms in &perms_all {
        for &a0 in EDGE_I32 {
            for &b0 in EDGE_I32 {
                diff(
                    &format!("safe_add wrap({a0}, {b0}, {perms})"),
                    |api| unsafe { (api.safe_add)(a0, b0, perms) },
                );
            }
        }
    }
}

// ===========================================================================
// copy_and_sum — CONFIGS rows 13..17
// ===========================================================================

#[test]
fn row13_copy_and_sum_count_one() {
    let mut rng = Rng::new(0x3001);
    for _ in 0..300 {
        let v = vec![rng.i32()];
        diff(&format!("copy_and_sum({v:?}, 1)"), |api| {
            call_cas(api, &v, 1)
        });
    }
    for &e in EDGE_I32 {
        let v = vec![e];
        diff(&format!("copy_and_sum([{e}], 1)"), |api| call_cas(api, &v, 1));
    }
}

#[test]
fn row14_copy_and_sum_count_three() {
    let mut rng = Rng::new(0x3002);
    for _ in 0..400 {
        let v = vec![rng.i32(), rng.i32(), rng.i32()];
        diff(&format!("copy_and_sum({v:?}, 3)"), |api| {
            call_cas(api, &v, 3)
        });
    }
}

#[test]
fn row15_copy_and_sum_count_zero() {
    // malloc(0) + empty loop.  Also with a zero-length allocation the source
    // pointer is never read, so pass both a real buffer and a dangling-but-
    // non-null pointer shape (1-element buffer).
    let v: Vec<c_int> = vec![0x7fff_ffff];
    diff("copy_and_sum(buf, 0)", |api| call_cas(api, &v, 0));
    let big = vec![1, 2, 3, 4];
    diff("copy_and_sum(buf4, 0)", |api| call_cas(api, &big, 0));
}

#[test]
fn row16_copy_and_sum_many() {
    let mut rng = Rng::new(0x3003);
    for &count in &[2usize, 7, 64, 255, 256, 1024, 4096, 65536] {
        for _ in 0..4 {
            let v: Vec<c_int> = (0..count).map(|_| rng.i32()).collect();
            diff(&format!("copy_and_sum(rand[{count}], {count})"), |api| {
                call_cas(api, &v, count as c_int)
            });
        }
        // also read a prefix of a longer buffer (count < len)
        let v: Vec<c_int> = (0..count + 16).map(|_| rng.i32()).collect();
        diff(&format!("copy_and_sum(prefix[{count}])"), |api| {
            call_cas(api, &v, count as c_int)
        });
    }
}

#[test]
fn row17_copy_and_sum_wrapping_totals() {
    let cases: Vec<Vec<c_int>> = vec![
        vec![i32::MAX, i32::MAX, i32::MAX],
        vec![i32::MIN, i32::MIN, i32::MIN],
        vec![i32::MAX, 1],
        vec![i32::MIN, -1],
        vec![i32::MAX, i32::MIN],
        vec![i32::MAX; 64],
        vec![i32::MIN; 64],
        vec![1 << 30, 1 << 30, 1 << 30, 1 << 30],
        vec![-(1 << 30), -(1 << 30), -(1 << 30), -(1 << 30)],
    ];
    for v in &cases {
        let n = v.len() as c_int;
        diff(&format!("copy_and_sum(wrap {v:?}, {n})"), |api| {
            call_cas(api, v, n)
        });
    }
    // random extremes
    let mut rng = Rng::new(0x3004);
    for _ in 0..200 {
        let n = 1 + rng.below(32) as usize;
        let v: Vec<c_int> = (0..n)
            .map(|_| if rng.next_u64() & 1 == 0 { i32::MAX } else { i32::MIN })
            .collect();
        diff(&format!("copy_and_sum(extremes {v:?})"), |api| {
            call_cas(api, &v, n as c_int)
        });
    }
}

// ===========================================================================
// multiply_with_log — CONFIGS rows 18..20
// ===========================================================================

#[test]
fn row18_multiply_with_log_random() {
    let mut rng = Rng::new(0x4001);
    for _ in 0..400 {
        let a0 = rng.i32();
        let b0 = rng.i32();
        diff(&format!("multiply_with_log({a0}, {b0})"), |api| {
            call_mwl(api, a0, b0)
        });
    }
    for _ in 0..200 {
        let a0 = rng.small(10_000);
        let b0 = rng.small(10_000);
        diff(&format!("multiply_with_log small({a0}, {b0})"), |api| {
            call_mwl(api, a0, b0)
        });
    }
}

#[test]
fn row19_multiply_with_log_zero_operands() {
    for &(a0, b0) in &[(0, 0), (0, 1), (1, 0), (0, i32::MIN), (i32::MAX, 0), (0, -1)] {
        diff(&format!("multiply_with_log zero({a0}, {b0})"), |api| {
            call_mwl(api, a0, b0)
        });
    }
}

#[test]
fn row20_multiply_with_log_wrapping_products() {
    for &a0 in EDGE_I32 {
        for &b0 in EDGE_I32 {
            diff(&format!("multiply_with_log wrap({a0}, {b0})"), |api| {
                call_mwl(api, a0, b0)
            });
        }
    }
}

// ===========================================================================
// create_result_string — CONFIGS rows 21..25
// ===========================================================================

#[test]
fn row21_create_result_string_basic() {
    let ops: Vec<&[u8]> = vec![b"\0", b"a\0", b"multiply\0", b"addition\0", b"array_sum\0"];
    let mut rng = Rng::new(0x5001);
    for op in &ops {
        for _ in 0..40 {
            let val = rng.i32();
            diff(
                &format!("create_result_string({:?}, {val})", show(op)),
                |api| call_crs(api, Some(op), val),
            );
        }
    }
}

#[test]
fn row22_create_result_string_null_op() {
    // glibc renders a NULL `%s` argument as "(null)".
    for &val in EDGE_I32 {
        diff(&format!("create_result_string(NULL, {val})"), |api| {
            call_crs(api, None, val)
        });
    }
}

#[test]
fn row23_create_result_string_truncation_boundary() {
    // "Operation: " (11) + op + ", Value: " (9) + digits.  Sweep op lengths so
    // the formatted text lands on 62/63/64/65+ bytes and snprintf truncates.
    for len in 0..80usize {
        let mut op = vec![b'x'; len];
        op.push(0);
        for &val in &[0, 7, -7, 12345, -12345, i32::MAX, i32::MIN] {
            diff(
                &format!("create_result_string(x*{len}, {val})"),
                |api| call_crs(api, Some(&op), val),
            );
        }
    }
}

#[test]
fn row24_create_result_string_value_boundaries() {
    let ops: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"op\0".to_vec(),
        b"multiply\0".to_vec(),
        {
            let mut v = vec![b'z'; 40];
            v.push(0);
            v
        },
    ];
    for op in &ops {
        for &val in EDGE_I32 {
            diff(
                &format!("create_result_string(len {}, {val})", op.len() - 1),
                |api| call_crs(api, Some(op), val),
            );
        }
    }
}

#[test]
fn row25_create_result_string_odd_bytes() {
    let mut cases: Vec<Vec<u8>> = vec![
        b"%d%s%%\0".to_vec(),
        b"12345\0".to_vec(),
        b"tab\there\0".to_vec(),
        b"nl\nhere\0".to_vec(),
        vec![0x80, 0xff, 0xfe, 0x7f, 0x01, 0],
        vec![0xc3, 0xa9, 0xe2, 0x82, 0xac, 0],
    ];
    let mut rng = Rng::new(0x5002);
    for _ in 0..120 {
        let len = rng.below(70) as usize;
        cases.push(rng.cstring(len));
    }
    for op in &cases {
        for &val in &[0, -1, i32::MIN, i32::MAX, 42] {
            diff(
                &format!("create_result_string(bytes {:?}, {val})", show(op)),
                |api| call_crs(api, Some(op), val),
            );
        }
    }
}

// ===========================================================================
// compare_operations — CONFIGS rows 26..31
// ===========================================================================

#[test]
fn row26_compare_operations_identical() {
    let cases: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"none\0".to_vec(),
        b"multiplication\0".to_vec(),
        vec![0xff, 0x80, 0x01, 0],
    ];
    for s in &cases {
        diff(&format!("compare_operations(eq {:?})", show(s)), |api| {
            call_cmp(api, s, s)
        });
    }
}

#[test]
fn row27_compare_operations_first_byte_differs() {
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (b"a\0".to_vec(), b"b\0".to_vec()),
        (b"b\0".to_vec(), b"a\0".to_vec()),
        (b"A\0".to_vec(), b"a\0".to_vec()),
        (b"\0".to_vec(), b"a\0".to_vec()),
        (b"a\0".to_vec(), b"\0".to_vec()),
        (vec![0x01, 0], vec![0x7f, 0]),
    ];
    for (a, b) in &pairs {
        diff(
            &format!("compare_operations({:?}, {:?})", show(a), show(b)),
            |api| call_cmp(api, a, b),
        );
    }
}

#[test]
fn row28_compare_operations_interior_byte_differs() {
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (b"addition\0".to_vec(), b"additionz\0".to_vec()),
        (b"array_sum\0".to_vec(), b"array_sun\0".to_vec()),
        (b"array_sun\0".to_vec(), b"array_sum\0".to_vec()),
        (b"multiplication\0".to_vec(), b"multiplicatioN\0".to_vec()),
        (b"aaaaaaaaaab\0".to_vec(), b"aaaaaaaaaaa\0".to_vec()),
    ];
    for (a, b) in &pairs {
        diff(
            &format!("compare_operations({:?}, {:?})", show(a), show(b)),
            |api| call_cmp(api, a, b),
        );
    }
}

#[test]
fn row29_compare_operations_prefix() {
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (b"none\0".to_vec(), b"none_x\0".to_vec()),
        (b"none_x\0".to_vec(), b"none\0".to_vec()),
        (b"\0".to_vec(), b"\0".to_vec()),
        (b"complex\0".to_vec(), b"complexmode\0".to_vec()),
    ];
    for (a, b) in &pairs {
        diff(
            &format!("compare_operations(prefix {:?}, {:?})", show(a), show(b)),
            |api| call_cmp(api, a, b),
        );
    }
}

#[test]
fn row30_compare_operations_high_bytes() {
    // strcmp compares as unsigned char: 0x80 > 0x7f.
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (vec![0x80, 0], vec![0x7f, 0]),
        (vec![0x7f, 0], vec![0x80, 0]),
        (vec![0xff, 0], vec![0x01, 0]),
        (vec![b'a', 0xff, 0], vec![b'a', 0x01, 0]),
        (vec![0xff; 9], {
            let mut v = vec![0xffu8; 8];
            v.push(0);
            v
        }),
    ];
    for (a, b) in &pairs {
        let a = if a.last() == Some(&0) {
            a.clone()
        } else {
            let mut t = a.clone();
            t.push(0);
            t
        };
        diff(
            &format!("compare_operations(high {a:?}, {b:?})"),
            |api| call_cmp(api, &a, b),
        );
    }
}

#[test]
fn row31_compare_operations_random_pairs() {
    let mut rng = Rng::new(0x6001);
    for _ in 0..1500 {
        let la = rng.below(65) as usize;
        let lb = rng.below(65) as usize;
        let a = rng.cstring(la);
        let mut b = if rng.next_u64() % 3 == 0 {
            // frequently produce near-identical strings so interior mismatch
            // positions get covered
            let mut t = a.clone();
            if t.len() > 1 {
                let i = rng.below((t.len() - 1) as u64) as usize;
                let mut nb = rng.byte();
                if nb == 0 {
                    nb = 1;
                }
                t[i] = nb;
            }
            t
        } else {
            rng.cstring(lb)
        };
        if b.last() != Some(&0) {
            b.push(0);
        }
        diff(
            &format!("compare_operations(rand {:?} vs {:?})", show(&a), show(&b)),
            |api| call_cmp(api, &a, &b),
        );
    }
}

// ===========================================================================
// complexmode — CONFIGS rows 32..41
// ===========================================================================

fn complexmode_sweep(seed: u64, mode: c_int, iters: usize, small: bool) {
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let (v1, v2, v3) = if small {
            (rng.small(10_000), rng.small(10_000), rng.small(10_000))
        } else {
            (rng.i32(), rng.i32(), rng.i32())
        };
        diff(
            &format!("complexmode({mode}, {v1}, {v2}, {v3})"),
            |api| unsafe { (api.complexmode)(mode, v1, v2, v3) },
        );
    }
}

fn complexmode_edges(mode: c_int) {
    for &v1 in EDGE_I32 {
        for &v2 in EDGE_I32 {
            for &v3 in &[0, 1, -1, i32::MAX, i32::MIN] {
                diff(
                    &format!("complexmode edge({mode}, {v1}, {v2}, {v3})"),
                    |api| unsafe { (api.complexmode)(mode, v1, v2, v3) },
                );
            }
        }
    }
}

#[test]
fn row32_complexmode_mode1_random() {
    complexmode_sweep(0x7001, 1, 200, false);
    complexmode_sweep(0x7002, 1, 200, true);
}

#[test]
fn row33_complexmode_mode1_edges() {
    complexmode_edges(1);
}

#[test]
fn row34_complexmode_mode2_random() {
    complexmode_sweep(0x7003, 2, 200, false);
    complexmode_sweep(0x7004, 2, 200, true);
}

#[test]
fn row35_complexmode_mode2_edges() {
    complexmode_edges(2);
}

#[test]
fn row36_complexmode_mode3_random() {
    complexmode_sweep(0x7005, 3, 200, false);
    complexmode_sweep(0x7006, 3, 200, true);
}

#[test]
fn row37_complexmode_mode3_edges() {
    complexmode_edges(3);
}

#[test]
fn row38_complexmode_mode4_random() {
    complexmode_sweep(0x7007, 4, 200, false);
    complexmode_sweep(0x7008, 4, 200, true);
}

#[test]
fn row39_complexmode_mode4_edges() {
    complexmode_edges(4);
}

#[test]
fn row40_complexmode_mode_neighbourhood() {
    let mut rng = Rng::new(0x7009);
    for mode in -2..=7 {
        for _ in 0..40 {
            let v1 = rng.i32();
            let v2 = rng.i32();
            let v3 = rng.i32();
            diff(
                &format!("complexmode neighbourhood({mode}, {v1}, {v2}, {v3})"),
                |api| unsafe { (api.complexmode)(mode, v1, v2, v3) },
            );
        }
    }
}

#[test]
fn row41_complexmode_fully_random() {
    let mut rng = Rng::new(0x700a);
    for _ in 0..600 {
        let mode = 1 + (rng.below(4) as c_int);
        let (v1, v2, v3) = (rng.i32(), rng.i32(), rng.i32());
        diff(
            &format!("complexmode rand-valid({mode}, {v1}, {v2}, {v3})"),
            |api| unsafe { (api.complexmode)(mode, v1, v2, v3) },
        );
    }
    for _ in 0..600 {
        let mode = rng.i32();
        let (v1, v2, v3) = (rng.i32(), rng.i32(), rng.i32());
        diff(
            &format!("complexmode rand-any({mode}, {v1}, {v2}, {v3})"),
            |api| unsafe { (api.complexmode)(mode, v1, v2, v3) },
        );
    }
}

// ===========================================================================
// row 42 — stdout is compared by `diff()` for every row above; this test makes
// the property explicit (the printed text must be non-empty and identical).
// ===========================================================================

#[test]
fn row42_stdout_bytes_are_compared_and_non_empty() {
    let (c, r) = both();
    for mode in 1..=4 {
        let (cv, cout) = capture(|| unsafe { (c.complexmode)(mode, 3, 5, 7) });
        let (rv, rout) = capture(|| unsafe { (r.complexmode)(mode, 3, 5, 7) });
        assert_eq!(cv, rv);
        assert!(!cout.is_empty(), "mode {mode} printed nothing (capture broken?)");
        assert_eq!(cout, rout, "mode {mode}: {} vs {}", show(&cout), show(&rout));
    }
    // and the denial path of safe_add prints too
    let (_, cout) = capture(|| unsafe { (c.safe_add)(1, 2, 0) });
    let (_, rout) = capture(|| unsafe { (r.safe_add)(1, 2, 0) });
    assert_eq!(cout, b"Insufficient permissions for addition\n".to_vec());
    assert_eq!(cout, rout);
}

// ===========================================================================
// row 43 — composed pipeline across low-level entry points
// ===========================================================================

#[test]
fn row43_composed_pipeline() {
    let mut rng = Rng::new(0x8001);
    for _ in 0..200 {
        let a0 = rng.i32();
        let b0 = rng.i32();
        let val = rng.i32();

        // create_result_string on each side, then feed both strings back into
        // compare_operations of *both* libraries (cross product), then release
        // the blocks with the shared C runtime's free().
        let (c, r) = both();
        let (cs, cout) = capture(|| unsafe {
            let p = (c.create_result_string)(b"multiply\0".as_ptr() as *const c_char, val);
            let s = cstr_bytes(p);
            libc_free(p);
            s
        });
        let (rs, rout) = capture(|| unsafe {
            let p = (r.create_result_string)(b"multiply\0".as_ptr() as *const c_char, val);
            let s = cstr_bytes(p);
            libc_free(p);
            s
        });
        assert_eq!(cs, rs, "create_result_string({val})");
        assert_eq!(cout, rout);

        // multiply_with_log's string must equal create_result_string's for the
        // product, in both implementations.
        let expect = {
            let mut v = cs.clone();
            v.push(0);
            v
        };
        let mwl = diff(&format!("pipeline mwl({a0}, {b0})"), |api| {
            call_mwl(api, a0, b0)
        });
        let mut got = mwl.2.clone();
        got.push(0);
        let cmp_c = call_cmp(c, &got, &got);
        let cmp_r = call_cmp(r, &got, &got);
        assert_eq!(cmp_c, 0);
        assert_eq!(cmp_c, cmp_r);
        if a0.wrapping_mul(b0) == val {
            assert_eq!(got, expect);
        }

        // and compare the log string against a freshly built one for the same
        // product through both compare_operations implementations
        let (ref_str, _) = capture(|| unsafe {
            let p = (c.create_result_string)(
                b"multiply\0".as_ptr() as *const c_char,
                a0.wrapping_mul(b0),
            );
            let s = cstr_bytes(p);
            libc_free(p);
            let mut s2 = s;
            s2.push(0);
            s2
        });
        let x = diff(&format!("pipeline cmp({a0}, {b0})"), |api| {
            call_cmp(api, &got, &ref_str)
        });
        assert_eq!(x, 0, "log string differs from reference");
    }
}
