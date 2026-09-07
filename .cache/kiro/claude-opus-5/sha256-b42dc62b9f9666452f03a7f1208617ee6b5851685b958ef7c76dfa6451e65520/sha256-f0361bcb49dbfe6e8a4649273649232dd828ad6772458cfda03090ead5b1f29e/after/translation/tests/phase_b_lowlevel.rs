// Phase B -- valid-path differential tests for the LOW-LEVEL entry points.
// CONFIGS.md rows C1..C24.

mod common;
use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

// ===========================================================================
// check_permissions -- C1..C4
// ===========================================================================

fn cp_pairs(pairs: &[(i32, i32)], what: &str) {
    for &(perms, required) in pairs {
        let (c, r) = each::<FnCheckPermissions, _>("check_permissions", |f| {
            capture(|| unsafe { f(perms, required) })
        });
        same(&format!("{what} check_permissions({perms:#o},{required:#o})"), c, r);
    }
}

#[test]
fn c1_check_permissions_required_zero() {
    let mut rng = Rng::new();
    let mut v = vec![(0, 0)];
    for &p in BOUNDS {
        v.push((p, 0));
    }
    for _ in 0..ITERS {
        v.push((rng.i32(), 0));
    }
    cp_pairs(&v, "C1");
}

#[test]
fn c2_check_permissions_single_bit_masks() {
    let mut rng = Rng::with_seed(2);
    let mut v = Vec::new();
    for &req in &[0o400, 0o200, 0o100] {
        for &p in BOUNDS {
            v.push((p, req));
        }
        for &p in &[0, 0o100, 0o200, 0o400, 0o600, 0o644, 0o777, 0o7777] {
            v.push((p, req));
        }
        for _ in 0..ITERS {
            v.push((rng.i32(), req));
        }
    }
    cp_pairs(&v, "C2");
}

#[test]
fn c3_check_permissions_rw_mask() {
    let mut rng = Rng::with_seed(3);
    let req = 0o600;
    let mut v = Vec::new();
    for &p in &[0, 0o100, 0o200, 0o400, 0o600, 0o644, 0o777, 0o1600, -1] {
        v.push((p, req));
    }
    for &p in BOUNDS {
        v.push((p, req));
    }
    for _ in 0..ITERS {
        v.push((rng.i32(), req));
        // deliberately bias toward masks that DO satisfy 0600
        v.push((rng.i32() | 0o600, req));
        v.push((rng.i32() & !0o200, req));
    }
    cp_pairs(&v, "C3");
}

#[test]
fn c4_check_permissions_full_width_masks() {
    let mut rng = Rng::with_seed(4);
    let mut v = Vec::new();
    for &a in BOUNDS {
        for &b in BOUNDS {
            v.push((a, b));
        }
    }
    for _ in 0..ITERS * 4 {
        v.push((rng.i32_mixed(), rng.i32_mixed()));
    }
    cp_pairs(&v, "C4");
}

// ===========================================================================
// safe_add -- C5..C7
// ===========================================================================

fn sa(triples: &[(i32, i32, i32)], what: &str) {
    for &(a, b, perms) in triples {
        let (c, r) = each::<FnSafeAdd, _>("safe_add", |f| capture(|| unsafe { f(a, b, perms) }));
        same(&format!("{what} safe_add({a},{b},{perms:#o})"), c, r);
    }
}

#[test]
fn c5_safe_add_permitted() {
    let mut rng = Rng::with_seed(5);
    let perms_ok = [0o600, 0o644, 0o777, 0o7777, -1, i32::MIN | 0o600, i32::MAX];
    let mut v = Vec::new();
    for &p in &perms_ok {
        for _ in 0..ITERS {
            v.push((rng.i32_mixed(), rng.i32_mixed(), p));
        }
        for &a in BOUNDS {
            for &b in BOUNDS {
                v.push((a, b, p));
            }
        }
    }
    sa(&v, "C5");
}

#[test]
fn c6_safe_add_refused() {
    let mut rng = Rng::with_seed(6);
    let perms_bad = [0, 0o100, 0o200, 0o400, 0o500, 0o044, 0o177, 0o7177];
    let mut v = Vec::new();
    for &p in &perms_bad {
        assert_ne!(p & 0o600, 0o600, "test bug: {p:#o} satisfies 0600");
        for _ in 0..ITERS {
            v.push((rng.i32_mixed(), rng.i32_mixed(), p));
        }
        for &a in BOUNDS {
            v.push((a, a, p));
        }
    }
    // plus random perms that happen not to satisfy the mask
    for _ in 0..ITERS {
        let p = rng.i32() & !0o200;
        v.push((rng.i32_mixed(), rng.i32_mixed(), p));
    }
    sa(&v, "C6");

    // and confirm the refusal text really is produced (not a vacuous pass)
    let (c, _r) = each::<FnSafeAdd, _>("safe_add", |f| capture(|| unsafe { f(7, 9, 0) }));
    assert_eq!(c.0, 0);
    assert_eq!(c.1, b"Insufficient permissions for addition\n");
}

#[test]
fn c7_safe_add_overflow_boundaries() {
    let cases = [
        (i32::MAX, 1),
        (1, i32::MAX),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN),
        (0, 0),
        (i32::MAX - 1, 2),
        (i32::MIN + 1, -2),
    ];
    let mut v = Vec::new();
    for &(a, b) in &cases {
        for &p in &[0o644, 0o600, 0o777, 0, 0o400] {
            v.push((a, b, p));
        }
    }
    sa(&v, "C7");
}

// ===========================================================================
// create_result_string -- C8..C11
// ===========================================================================

/// Calls `create_result_string` in both libs, comparing the returned buffer
/// (up to and including the NUL; the 64-byte allocation's tail is
/// uninitialised heap) plus stdout, then frees each buffer.
fn crs(op: Option<&[u8]>, val: i32, what: &str) {
    let cs = op.map(|b| CString::new(b).unwrap());
    let p: *const c_char = match &cs {
        Some(s) => s.as_ptr(),
        None => std::ptr::null(),
    };
    let (c, r) = each::<FnCreateResultString, _>("create_result_string", |f| {
        capture(|| unsafe {
            let out = f(p, val);
            let bytes = read_c_buf(out, 64);
            if !out.is_null() {
                libc_free(out as *mut _);
            }
            bytes
        })
    });
    assert!(c.0.is_some(), "{what}: C returned NULL unexpectedly");
    same(what, c, r);
}

// tiny shim so tests can free what the libraries malloc'ed
fn libc_free(p: *mut std::os::raw::c_void) {
    unsafe extern "C" {
        fn free(p: *mut std::os::raw::c_void);
    }
    unsafe { free(p) }
}

#[test]
fn c8_create_result_string_short_ops() {
    let mut rng = Rng::with_seed(8);
    let ops: [&[u8]; 6] = [b"multiply", b"add", b"x", b"array_sum", b"complex", b"none"];
    for op in ops {
        for &val in BOUNDS {
            crs(Some(op), val, &format!("C8 crs({:?},{val})", show(op)));
        }
        for _ in 0..ITERS {
            let val = rng.i32_mixed();
            crs(Some(op), val, &format!("C8 crs({:?},{val})", show(op)));
        }
    }
}

#[test]
fn c9_create_result_string_empty_op() {
    let mut rng = Rng::with_seed(9);
    for &val in BOUNDS {
        crs(Some(b""), val, &format!("C9 crs(\"\",{val})"));
    }
    for _ in 0..ITERS {
        let val = rng.i32_mixed();
        crs(Some(b""), val, &format!("C9 crs(\"\",{val})"));
    }
}

#[test]
fn c10_create_result_string_truncation_sweep() {
    // "Operation: " = 11 bytes, ", Value: " = 9 bytes, INT_MIN = 11 chars.
    // 11 + n + 9 + 11 crosses 63 at n == 32, so sweeping 0..=80 walks the
    // snprintf truncation boundary from both sides.
    for n in 0..=80usize {
        let op: Vec<u8> = std::iter::repeat(b'A').take(n).collect();
        for &val in &[i32::MIN, i32::MAX, 0, -1, 1234567890] {
            crs(Some(&op), val, &format!("C10 crs(A*{n},{val})"));
        }
    }
    // mixed-width filler too, so it is not just one repeated byte
    let mut rng = Rng::with_seed(10);
    for n in 20..=60usize {
        let op: Vec<u8> = (0..n).map(|_| rng.byte_nonzero()).collect();
        crs(Some(&op), rng.i32_mixed(), &format!("C10 crs(rand*{n})"));
    }
}

#[test]
fn c11_create_result_string_high_bit_bytes() {
    let mut rng = Rng::with_seed(11);
    for _ in 0..ITERS {
        let n = rng.range(1, 40) as usize;
        let op: Vec<u8> = (0..n)
            .map(|_| {
                let b = (rng.next_u64() & 0xFF) as u8;
                if b == 0 { 0x80 } else { b }
            })
            .collect();
        let val = rng.i32_mixed();
        crs(Some(&op), val, &format!("C11 crs(hi*{n},{val})"));
    }
}

// ===========================================================================
// multiply_with_log -- C12..C13
// ===========================================================================

fn mwl(a: i32, b: i32, what: &str) {
    let (c, r) = each::<FnMultiplyWithLog, _>("multiply_with_log", |f| {
        capture(|| unsafe {
            let mut out: *mut c_char = std::ptr::null_mut();
            let rv = f(a, b, &mut out);
            let bytes = read_c_buf(out, 64);
            if !out.is_null() {
                libc_free(out as *mut _);
            }
            (rv, bytes)
        })
    });
    assert!(c.0 .1.is_some(), "{what}: C left log_msg NULL");
    same(what, c, r);
}

#[test]
fn c12_multiply_with_log_random() {
    let mut rng = Rng::with_seed(12);
    for _ in 0..ITERS * 4 {
        let (a, b) = (rng.i32_mixed(), rng.i32_mixed());
        mwl(a, b, &format!("C12 mwl({a},{b})"));
    }
}

#[test]
fn c13_multiply_with_log_boundaries() {
    for &a in BOUNDS {
        for &b in BOUNDS {
            mwl(a, b, &format!("C13 mwl({a},{b})"));
        }
    }
    for &(a, b) in &[
        (i32::MAX, 2),
        (2, i32::MAX),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (0, i32::MIN),
        (i32::MIN, 0),
        (46341, 46341), // just past sqrt(INT_MAX)
        (-46341, 46341),
    ] {
        mwl(a, b, &format!("C13 mwl({a},{b})"));
    }
}

// ===========================================================================
// copy_and_sum -- C14..C18
// ===========================================================================

fn cas(buf: &mut [c_int], count: c_int, what: &str) {
    let mut cbuf = buf.to_vec();
    let mut rbuf = buf.to_vec();
    let l = libs();
    let cf: libloading::Symbol<FnCopyAndSum> = sym(&l.c, "copy_and_sum");
    let rf: libloading::Symbol<FnCopyAndSum> = sym(&l.r, "copy_and_sum");
    let c = capture(|| unsafe { cf(cbuf.as_mut_ptr(), count) });
    let r = capture(|| unsafe { rf(rbuf.as_mut_ptr(), count) });
    assert_eq!(cbuf, rbuf, "{what}: source buffer mutated differently");
    same(what, c, r);
}

#[test]
fn c14_copy_and_sum_count_zero() {
    let mut rng = Rng::with_seed(14);
    for _ in 0..ITERS {
        let mut buf: Vec<c_int> = (0..8).map(|_| rng.i32_mixed()).collect();
        cas(&mut buf, 0, "C14 cas(count=0)");
    }
}

#[test]
fn c15_copy_and_sum_count_one() {
    let mut rng = Rng::with_seed(15);
    for &v in BOUNDS {
        cas(&mut [v], 1, &format!("C15 cas([{v}],1)"));
    }
    for _ in 0..ITERS {
        let v = rng.i32_mixed();
        cas(&mut [v], 1, &format!("C15 cas([{v}],1)"));
    }
}

#[test]
fn c16_copy_and_sum_count_three() {
    let mut rng = Rng::with_seed(16);
    for _ in 0..ITERS * 4 {
        let mut buf = [rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed()];
        let d = format!("C16 cas({buf:?},3)");
        cas(&mut buf, 3, &d);
    }
    for &a in BOUNDS {
        for &b in BOUNDS {
            let mut buf = [a, b, a];
            let d = format!("C16 cas({buf:?},3)");
            cas(&mut buf, 3, &d);
        }
    }
}

#[test]
fn c17_copy_and_sum_many() {
    let mut rng = Rng::with_seed(17);
    for _ in 0..ITERS {
        let n = rng.range(2, 1024) as usize;
        let mut buf: Vec<c_int> = (0..n).map(|_| rng.i32_mixed()).collect();
        cas(&mut buf, n as c_int, &format!("C17 cas(n={n})"));
    }
    // count < buffer length: only the first `count` elements participate
    for _ in 0..ITERS {
        let n = rng.range(2, 512) as usize;
        let mut buf: Vec<c_int> = (0..n).map(|_| rng.i32_mixed()).collect();
        let k = rng.range(1, n as u64) as c_int;
        cas(&mut buf, k, &format!("C17 cas(n={n},count={k})"));
    }
}

#[test]
fn c18_copy_and_sum_repeated_wraparound() {
    let n = 1000usize;
    let all_max: Vec<c_int> = vec![i32::MAX; n];
    let all_min: Vec<c_int> = vec![i32::MIN; n];
    let alt: Vec<c_int> = (0..n)
        .map(|i| if i % 2 == 0 { i32::MAX } else { i32::MIN })
        .collect();
    let big: Vec<c_int> = vec![0x4000_0000; n];
    for (name, v) in [
        ("all_max", all_max),
        ("all_min", all_min),
        ("alternating", alt),
        ("0x40000000", big),
    ] {
        for &count in &[1i32, 2, 3, 7, 100, 999, 1000] {
            let mut b = v.clone();
            cas(&mut b, count, &format!("C18 cas({name},{count})"));
        }
    }
}

// ===========================================================================
// compare_operations -- C19..C24
// ===========================================================================

fn cmpop(a: &[u8], b: &[u8], what: &str) {
    let ca = CString::new(a).unwrap();
    let cb = CString::new(b).unwrap();
    let (c, r) = each::<FnCompareOperations, _>("compare_operations", |f| {
        capture(|| unsafe { f(ca.as_ptr(), cb.as_ptr()) })
    });
    same(what, c, r);
}

#[test]
fn c19_compare_operations_equal() {
    let mut rng = Rng::with_seed(19);
    cmpop(b"", b"", "C19 \"\" vs \"\"");
    for _ in 0..ITERS * 2 {
        let n = rng.range(0, 48) as usize;
        let s: Vec<u8> = (0..n).map(|_| rng.byte_nonzero()).collect();
        cmpop(&s, &s, &format!("C19 equal n={n}"));
    }
}

#[test]
fn c20_compare_operations_differ_first_byte() {
    let mut rng = Rng::with_seed(20);
    for _ in 0..ITERS * 2 {
        let n = rng.range(1, 32) as usize;
        let mut a: Vec<u8> = (0..n).map(|_| rng.byte_nonzero()).collect();
        let mut b = a.clone();
        a[0] = rng.byte_nonzero();
        b[0] = rng.byte_nonzero();
        cmpop(&a, &b, "C20 differ@0");
        cmpop(&b, &a, "C20 differ@0 swapped");
    }
    // exact magnitudes at the extremes of the byte range
    for &(x, y) in &[(1u8, 255u8), (255, 1), (0x7F, 0x80), (0x80, 0x7F), (1, 2), (2, 1)] {
        cmpop(&[x], &[y], &format!("C20 [{x:#x}] vs [{y:#x}]"));
    }
}

#[test]
fn c21_compare_operations_differ_late() {
    let mut rng = Rng::with_seed(21);
    for _ in 0..ITERS * 2 {
        let n = rng.range(2, 40) as usize;
        let base: Vec<u8> = (0..n).map(|_| rng.byte_nonzero()).collect();
        let i = rng.range(1, n as u64 - 1) as usize;
        let mut a = base.clone();
        let mut b = base.clone();
        a[i] = rng.byte_nonzero();
        b[i] = rng.byte_nonzero();
        cmpop(&a, &b, &format!("C21 differ@{i}"));
        cmpop(&b, &a, &format!("C21 differ@{i} swapped"));
    }
}

#[test]
fn c22_compare_operations_prefix() {
    let mut rng = Rng::with_seed(22);
    for _ in 0..ITERS * 2 {
        let n = rng.range(1, 40) as usize;
        let long: Vec<u8> = (0..n).map(|_| rng.byte_nonzero()).collect();
        let k = rng.range(0, n as u64) as usize;
        let short = long[..k].to_vec();
        cmpop(&short, &long, &format!("C22 prefix {k}/{n}"));
        cmpop(&long, &short, &format!("C22 prefix {k}/{n} swapped"));
    }
    cmpop(b"", b"a", "C22 \"\" vs a");
    cmpop(b"a", b"", "C22 a vs \"\"");
}

#[test]
fn c23_compare_operations_high_bit_bytes() {
    let mut rng = Rng::with_seed(23);
    for _ in 0..ITERS * 3 {
        let n = rng.range(1, 24) as usize;
        let mk = |rng: &mut Rng| -> Vec<u8> {
            (0..n)
                .map(|_| {
                    let b = (rng.next_u64() & 0xFF) as u8;
                    if b == 0 { 0x80 } else { b }
                })
                .collect()
        };
        let a = mk(&mut rng);
        let b = mk(&mut rng);
        cmpop(&a, &b, "C23 high-bit");
        cmpop(&b, &a, "C23 high-bit swapped");
    }
}

#[test]
fn c24_compare_operations_library_literals() {
    let names: [&[u8]; 5] = [
        b"none",
        b"addition",
        b"multiplication",
        b"array_sum",
        b"complex",
    ];
    for a in names {
        for b in names {
            cmpop(a, b, &format!("C24 {} vs {}", show(a), show(b)));
        }
    }
}
