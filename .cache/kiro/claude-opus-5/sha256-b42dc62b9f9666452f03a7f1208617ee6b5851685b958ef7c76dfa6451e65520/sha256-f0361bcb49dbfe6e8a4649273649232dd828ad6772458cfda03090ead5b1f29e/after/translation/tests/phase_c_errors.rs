// Phase C -- error-path differential tests. One test per ERRORS.md row
// (E1..E18), plus the generic FFI boundaries: null pointers, zero/oversized
// lengths, and values one step past every documented valid range including
// out-of-range "enum" ints.

mod common;
use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

fn libc_free(p: *mut std::os::raw::c_void) {
    unsafe extern "C" {
        fn free(p: *mut std::os::raw::c_void);
    }
    unsafe { free(p) }
}

// ===========================================================================
// E1 -- create_result_string: malloc(64) returns NULL.
//
// Not reachable at runtime on this platform (a 64-byte request does not fail).
// Verified structurally -- both sources have the same null check and the same
// early `return NULL` -- and behaviourally: for every reachable input both
// libraries hand back a non-NULL, free-able 64-byte buffer.
// ===========================================================================
#[test]
fn e1_create_result_string_malloc_failure_guard() {
    let c = read_src("c_src/src/lib.c");
    let r = read_src("translation/src/lib.rs");
    assert!(c.contains("if (str == NULL) {\n        return NULL;"), "C guard moved");
    assert!(
        r.contains("if str_.is_null() {") && r.contains("return core::ptr::null_mut();"),
        "Rust create_result_string lost its malloc-failure guard"
    );
    // and the reachable half of the row
    let ops: [&[u8]; 3] = [b"", b"multiply", b"a-very-long-operation-name-indeed-yes"];
    for op in ops {
        let cs = CString::new(op).unwrap();
        let (c, r) = each::<FnCreateResultString, _>("create_result_string", |f| {
            capture(|| unsafe {
                let p = f(cs.as_ptr(), 1);
                let nonnull = !p.is_null();
                let bytes = read_c_buf(p, 64);
                if nonnull {
                    libc_free(p as *mut _);
                }
                (nonnull, bytes)
            })
        });
        assert_eq!(c.0 .0, true, "C returned NULL for a 64-byte request");
        same(&format!("E1 reachable {:?}", show(op)), c, r);
    }
}

// ===========================================================================
// E2 -- create_result_string(NULL, val): the C passes NULL straight to
//       snprintf's %s (no null check). glibc renders "(null)".
// ===========================================================================
#[test]
fn e2_create_result_string_null_op() {
    let mut rng = Rng::with_seed(102);
    let mut vals: Vec<i32> = BOUNDS.to_vec();
    for _ in 0..ITERS {
        vals.push(rng.i32_mixed());
    }
    for val in vals {
        let (c, r) = each::<FnCreateResultString, _>("create_result_string", |f| {
            capture(|| unsafe {
                let p = f(std::ptr::null(), val);
                let bytes = read_c_buf(p, 64);
                if !p.is_null() {
                    libc_free(p as *mut _);
                }
                bytes
            })
        });
        same(&format!("E2 create_result_string(NULL,{val})"), c, r);
    }
    // non-vacuous: confirm the C really formats "(null)"
    let (c, _) = each::<FnCreateResultString, _>("create_result_string", |f| {
        capture(|| unsafe {
            let p = f(std::ptr::null(), 7);
            let b = read_c_buf(p, 64);
            if !p.is_null() {
                libc_free(p as *mut _);
            }
            b
        })
    });
    assert_eq!(
        c.0.unwrap(),
        b"Operation: (null), Value: 7\0".to_vec(),
        "E2: unexpected C rendering of a NULL %s"
    );
}

// ===========================================================================
// E3 -- create_result_string: formatted text exceeds the 64-byte buffer, so
//       snprintf truncates at 63 chars + NUL.
// ===========================================================================
#[test]
fn e3_create_result_string_truncation() {
    for n in [31usize, 32, 33, 43, 44, 45, 52, 53, 54, 63, 64, 65, 200, 1000] {
        let op: Vec<u8> = std::iter::repeat(b'Z').take(n).collect();
        let cs = CString::new(op).unwrap();
        for &val in &[0i32, -1, i32::MIN, i32::MAX] {
            let (c, r) = each::<FnCreateResultString, _>("create_result_string", |f| {
                capture(|| unsafe {
                    let p = f(cs.as_ptr(), val);
                    let bytes = read_c_buf(p, 64);
                    if !p.is_null() {
                        libc_free(p as *mut _);
                    }
                    bytes
                })
            });
            let len = c.0.as_ref().unwrap().len();
            assert!(len <= 64, "E3: C wrote past the 64-byte buffer ({len})");
            same(&format!("E3 truncation n={n} val={val}"), c, r);
        }
    }
    // non-vacuous: a case that definitely truncates
    let op: Vec<u8> = std::iter::repeat(b'Z').take(200).collect();
    let cs = CString::new(op).unwrap();
    let (c, _) = each::<FnCreateResultString, _>("create_result_string", |f| {
        capture(|| unsafe {
            let p = f(cs.as_ptr(), 0);
            let b = read_c_buf(p, 64);
            if !p.is_null() {
                libc_free(p as *mut _);
            }
            b
        })
    });
    let b = c.0.unwrap();
    assert_eq!(b.len(), 64, "E3: expected a full 63-char + NUL truncation");
    assert_eq!(*b.last().unwrap(), 0);
}

// ===========================================================================
// E4 -- safe_add: (perms & 0600) != 0600 -> refusal message, return 0.
// ===========================================================================
#[test]
fn e4_safe_add_insufficient_permissions() {
    let mut rng = Rng::with_seed(104);
    // every perms value that fails the 0600 mask, plus randoms
    let mut bad: Vec<i32> = vec![0, 0o100, 0o200, 0o400, 0o500, 0o044, 0o177, 0o1400, 0o5577];
    for &b in BOUNDS {
        if b & 0o600 != 0o600 {
            bad.push(b);
        }
    }
    for _ in 0..ITERS {
        let p = rng.i32();
        if p & 0o600 != 0o600 {
            bad.push(p);
        }
    }
    for p in bad {
        for &(a, b) in &[(1i32, 2i32), (0, 0), (i32::MAX, 1), (i32::MIN, -1), (-5, -6)] {
            let (c, r) = each::<FnSafeAdd, _>("safe_add", |f| capture(|| unsafe { f(a, b, p) }));
            assert_eq!(c.0, 0, "E4: C safe_add({a},{b},{p:#o}) should return 0");
            assert_eq!(
                c.1, b"Insufficient permissions for addition\n",
                "E4: unexpected C refusal text"
            );
            same(&format!("E4 safe_add({a},{b},{p:#o})"), c, r);
        }
    }
    // one step past the boundary in both directions: 0577 fails, 0600 passes
    for &(p, refuses) in &[(0o577, true), (0o600, false), (0o601, false), (0o400, true), (0o200, true)] {
        let (c, r) = each::<FnSafeAdd, _>("safe_add", |f| capture(|| unsafe { f(3, 4, p) }));
        assert_eq!(c.1.is_empty(), !refuses, "E4: boundary {p:#o} misclassified");
        same(&format!("E4 boundary safe_add(3,4,{p:#o})"), c, r);
    }
}

// ===========================================================================
// E5 -- multiply_with_log: create_result_string returned NULL -> return 0.
//       Unreachable (see E1); verified structurally on both sides.
// ===========================================================================
#[test]
fn e5_multiply_with_log_null_buffer_guard() {
    let c = read_src("c_src/src/lib.c");
    let r = read_src("translation/src/lib.rs");
    assert!(c.contains("if (*log_msg == NULL) {\n        return 0;"), "C guard moved");
    assert!(
        (r.contains("if log_msg.read_volatile().is_null() {")
            || r.contains("if (*log_msg).is_null() {"))
            && r.contains("return 0;"),
        "Rust multiply_with_log lost its NULL-buffer guard"
    );
    // reachable half: log_msg is always populated and the return is a*b
    let mut rng = Rng::with_seed(105);
    for _ in 0..64 {
        let (a, b) = (rng.i32_mixed(), rng.i32_mixed());
        let (c, r) = each::<FnMultiplyWithLog, _>("multiply_with_log", |f| {
            capture(|| unsafe {
                let mut out: *mut c_char = std::ptr::null_mut();
                let rv = f(a, b, &mut out);
                let ok = !out.is_null();
                let bytes = read_c_buf(out, 64);
                if ok {
                    libc_free(out as *mut _);
                }
                (rv, ok, bytes)
            })
        });
        assert!(c.0 .1, "E5: C left log_msg NULL for ({a},{b})");
        same(&format!("E5 reachable mwl({a},{b})"), c, r);
    }
}

// ===========================================================================
// E6 -- multiply_with_log(a, b, NULL): the C stores through the out-param with
//       no null check. Both libraries must fault identically. Run in a forked
//       child so the crash is an observation rather than a dead test runner.
// ===========================================================================
#[test]
fn e6_multiply_with_log_null_out_param() {
    let c = read_src("c_src/src/lib.c");
    let r = read_src("translation/src/lib.rs");
    assert!(
        !c.contains("log_msg == NULL) {\n        return -1"),
        "C has no null check on log_msg itself"
    );
    assert!(
        r.contains("log_msg.write_volatile(create_result_string(")
            || r.contains("*log_msg = create_result_string("),
        "Rust must reproduce the unchecked store through log_msg"
    );
    assert!(
        !r.contains("if log_msg.is_null()"),
        "Rust must NOT add a null check on log_msg that the C does not have"
    );
    for &(a, b) in &[(3i32, 4i32), (0, 0), (i32::MIN, -1)] {
        let res = fork_same::<FnMultiplyWithLog>(
            &format!("E6 mwl({a},{b},NULL)"),
            "multiply_with_log",
            |f| unsafe { f(a, b, std::ptr::null_mut()) as i64 },
        );
        eprintln!(
            "E6 mwl({a},{b},NULL): value={:?} signal={} stdout={:?}",
            res.value,
            res.signal,
            show(&res.stdout)
        );
        // The C stores through a NULL pointer, so it must die on a signal; the
        // Rust must die on the SAME signal (checked inside fork_same).
        assert_ne!(
            res.signal, 0,
            "E6: expected the unchecked NULL store to fault, got a normal return"
        );
    }
}

// ===========================================================================
// E7 -- copy_and_sum(NULL, count): message + -1, for EVERY count including 0
//       (the null check precedes the count use).
// ===========================================================================
#[test]
fn e7_copy_and_sum_null_src() {
    let mut counts: Vec<i32> = vec![0, 1, 2, 3, -1, i32::MAX, i32::MIN, 1024];
    let mut rng = Rng::with_seed(107);
    for _ in 0..ITERS {
        counts.push(rng.i32_mixed());
    }
    for count in counts {
        let (c, r) = each::<FnCopyAndSum, _>("copy_and_sum", |f| {
            capture(|| unsafe { f(std::ptr::null_mut(), count) })
        });
        assert_eq!(c.0, -1, "E7: C copy_and_sum(NULL,{count}) should be -1");
        assert_eq!(c.1, b"Source pointer is NULL\n", "E7: unexpected C text");
        same(&format!("E7 copy_and_sum(NULL,{count})"), c, r);
    }
}

// ===========================================================================
// E8 -- copy_and_sum: malloc(count * sizeof(int)) fails.
//
//  * negative `count` promotes to a near-SIZE_MAX byte request -> malloc always
//    fails -> "Memory allocation failed" + -1. Tested in-process.
//  * very large positive `count` may instead have malloc SUCCEED, after which
//    the C memcpy's past the caller's buffer. That is still a real input the C
//    handles some way, so it is compared in a forked child (crash-or-not,
//    signal and partial stdout must match).
// ===========================================================================
#[test]
fn e8_copy_and_sum_allocation_failure_negative_counts() {
    let mut buf: Vec<c_int> = (0..16).map(|i| i as c_int).collect();
    let mut counts: Vec<i32> = vec![-1, -2, -3, -4, -16, -1024, i32::MIN, i32::MIN + 1, -0x4000_0000];
    let mut rng = Rng::with_seed(108);
    for _ in 0..ITERS {
        let v = rng.i32();
        if v < 0 {
            counts.push(v);
        }
    }
    for count in counts {
        let l = libs();
        let cf: libloading::Symbol<FnCopyAndSum> = sym(&l.c, "copy_and_sum");
        let rf: libloading::Symbol<FnCopyAndSum> = sym(&l.r, "copy_and_sum");
        let c = capture(|| unsafe { cf(buf.as_mut_ptr(), count) });
        let r = capture(|| unsafe { rf(buf.as_mut_ptr(), count) });
        assert_eq!(c.0, -1, "E8: C copy_and_sum(buf,{count}) should be -1");
        assert_eq!(c.1, b"Memory allocation failed\n", "E8: unexpected C text for count={count}");
        same(&format!("E8 copy_and_sum(buf,{count})"), c, r);
    }
}

#[test]
fn e8b_copy_and_sum_oversized_counts() {
    // Oversized but positive: whichever way the C goes, the Rust must match.
    for count in [0x2000_0000i32, 0x4000_0000, 0x7FFF_FFFF, 0x7FFF_FFFE, 1 << 24] {
        let res = fork_same::<FnCopyAndSum>(
            &format!("E8b copy_and_sum(buf,{count})"),
            "copy_and_sum",
            |f| {
                let mut buf: Vec<c_int> = (0..16).map(|i| i as c_int).collect();
                unsafe { f(buf.as_mut_ptr(), count) as i64 }
            },
        );
        eprintln!(
            "E8b count={count}: value={:?} signal={} stdout={:?}",
            res.value,
            res.signal,
            show(&res.stdout)
        );
    }
}

// ===========================================================================
// E9 -- copy_and_sum(valid, 0): NOT an error. malloc(0) succeeds, the loop body
//       never runs, result 0, no output. (One step below the E8 range.)
// ===========================================================================
#[test]
fn e9_copy_and_sum_count_zero_is_not_an_error() {
    let mut rng = Rng::with_seed(109);
    for _ in 0..ITERS {
        let mut buf: Vec<c_int> = (0..8).map(|_| rng.i32_mixed()).collect();
        let l = libs();
        let cf: libloading::Symbol<FnCopyAndSum> = sym(&l.c, "copy_and_sum");
        let rf: libloading::Symbol<FnCopyAndSum> = sym(&l.r, "copy_and_sum");
        let mut b2 = buf.clone();
        let c = capture(|| unsafe { cf(buf.as_mut_ptr(), 0) });
        let r = capture(|| unsafe { rf(b2.as_mut_ptr(), 0) });
        assert_eq!(c.0, 0, "E9: C copy_and_sum(buf,0) should be 0");
        assert!(c.1.is_empty(), "E9: C should print nothing for count=0");
        same("E9 copy_and_sum(buf,0)", c, r);
    }
}

// ===========================================================================
// E10/E11/E12 -- compare_operations null-pointer rejections.
// ===========================================================================
fn cmpop_raw(a: *const c_char, b: *const c_char, what: &str) {
    let (c, r) = each::<FnCompareOperations, _>("compare_operations", |f| {
        capture(|| unsafe { f(a, b) })
    });
    assert_eq!(c.0, -1, "{what}: C should return -1");
    assert_eq!(
        c.1, b"One or both operation strings are NULL\n",
        "{what}: unexpected C text"
    );
    same(what, c, r);
}

#[test]
fn e10_compare_operations_null_first() {
    let mut rng = Rng::with_seed(110);
    for _ in 0..ITERS {
        let n = rng.range(0, 24) as usize;
        let s: Vec<u8> = (0..n).map(|_| rng.byte_nonzero()).collect();
        let cs = CString::new(s).unwrap();
        cmpop_raw(std::ptr::null(), cs.as_ptr(), "E10 compare_operations(NULL, s)");
    }
    let e = CString::new("").unwrap();
    cmpop_raw(std::ptr::null(), e.as_ptr(), "E10 compare_operations(NULL, \"\")");
}

#[test]
fn e11_compare_operations_null_second() {
    let mut rng = Rng::with_seed(111);
    for _ in 0..ITERS {
        let n = rng.range(0, 24) as usize;
        let s: Vec<u8> = (0..n).map(|_| rng.byte_nonzero()).collect();
        let cs = CString::new(s).unwrap();
        cmpop_raw(cs.as_ptr(), std::ptr::null(), "E11 compare_operations(s, NULL)");
    }
    let e = CString::new("").unwrap();
    cmpop_raw(e.as_ptr(), std::ptr::null(), "E11 compare_operations(\"\", NULL)");
}

#[test]
fn e12_compare_operations_both_null() {
    for _ in 0..8 {
        cmpop_raw(
            std::ptr::null(),
            std::ptr::null(),
            "E12 compare_operations(NULL, NULL)",
        );
    }
}

// ===========================================================================
// E13 -- compare_operations: the exact strcmp magnitude, not just its sign.
// ===========================================================================
#[test]
fn e13_compare_operations_exact_magnitude() {
    let mut rng = Rng::with_seed(113);
    let mut checked_nonunit = false;
    for _ in 0..ITERS * 4 {
        let n = rng.range(1, 20) as usize;
        let a: Vec<u8> = (0..n).map(|_| rng.byte_nonzero()).collect();
        let mut b = a.clone();
        let i = rng.range(0, n as u64 - 1) as usize;
        b[i] = rng.byte_nonzero();
        let ca = CString::new(a.clone()).unwrap();
        let cb = CString::new(b.clone()).unwrap();
        let (c, r) = each::<FnCompareOperations, _>("compare_operations", |f| {
            capture(|| unsafe { f(ca.as_ptr(), cb.as_ptr()) })
        });
        if c.0.abs() > 1 {
            checked_nonunit = true;
        }
        same("E13 exact strcmp magnitude", c, r);
    }
    // widest possible byte gap, both directions
    for &(x, y) in &[(1u8, 255u8), (255, 1), (0x01, 0x80), (0x80, 0x01)] {
        let ca = CString::new(vec![x]).unwrap();
        let cb = CString::new(vec![y]).unwrap();
        let (c, r) = each::<FnCompareOperations, _>("compare_operations", |f| {
            capture(|| unsafe { f(ca.as_ptr(), cb.as_ptr()) })
        });
        if c.0.abs() > 1 {
            checked_nonunit = true;
        }
        same(&format!("E13 [{x:#x}] vs [{y:#x}]"), c, r);
    }
    assert!(
        checked_nonunit,
        "E13: never observed a |strcmp| > 1, so magnitude was not actually compared"
    );
}

// ===========================================================================
// E14 -- complexmode: malloc(sizeof(Result)) returns NULL -> message + -1.
//        Unreachable (40-byte request); verified structurally.
// ===========================================================================
#[test]
fn e14_complexmode_tracker_allocation_guard() {
    let c = read_src("c_src/src/lib.c");
    let r = read_src("translation/src/lib.rs");
    assert!(
        c.contains("if (res_tracker == NULL) {")
            && c.contains("Failed to allocate result tracker"),
        "C guard moved"
    );
    assert!(
        r.contains("if res_tracker.is_null() {")
            && r.contains("Failed to allocate result tracker"),
        "Rust complexmode lost its tracker-allocation guard"
    );
    // reachable half: every mode returns without hitting the guard
    for mode in [1i32, 2, 3, 4, 0] {
        let (c, r) = each::<FnComplexmode, _>("complexmode", |f| {
            capture(|| unsafe { f(mode, 1, 2, 3) })
        });
        assert!(
            !c.1.starts_with(b"Failed to allocate"),
            "E14: C hit the unreachable guard"
        );
        same(&format!("E14 reachable complexmode({mode},1,2,3)"), c, r);
    }
}

// ===========================================================================
// E15 -- complexmode default arm. This is the out-of-range-enum row: `mode` is
//        an int across the FFI boundary, so every value with no matching case
//        is a real input. Also covers "one step past" 4 and below 1.
// ===========================================================================
#[test]
fn e15_complexmode_out_of_range_mode() {
    let mut modes: Vec<i32> = vec![
        0, 5, 6, 7, 8, -1, -2, -4, 100, 1000, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1,
    ];
    let mut rng = Rng::with_seed(115);
    for _ in 0..ITERS * 2 {
        let m = rng.i32();
        if !(1..=4).contains(&m) {
            modes.push(m);
        }
    }
    // wide-int aliasing: 0x1_0000_0001 truncates to 1 in C's int parameter.
    // Passing an i64 through an int parameter is not expressible here, but the
    // full i32 range is, and it is swept above.
    for m in modes {
        let (c, r) = each::<FnComplexmode, _>("complexmode", |f| {
            capture(|| unsafe { f(m, 11, 22, 33) })
        });
        assert_eq!(c.0, -1, "E15: C complexmode({m},..) should be -1");
        assert_eq!(c.1, b"Invalid mode\n", "E15: unexpected C transcript for {m}");
        same(&format!("E15 complexmode({m},11,22,33)"), c, r);
    }
    // one step INSIDE the range must NOT take the default arm
    for m in [1i32, 2, 3, 4] {
        let (c, r) = each::<FnComplexmode, _>("complexmode", |f| {
            capture(|| unsafe { f(m, 11, 22, 33) })
        });
        assert_ne!(c.1, b"Invalid mode\n", "E15: mode {m} must be valid");
        assert!(
            c.1.ends_with(b"\n") && c.1.windows(19).any(|w| w == b"Operation performed"),
            "E15: mode {m} must print the trailer"
        );
        same(&format!("E15 in-range complexmode({m},11,22,33)"), c, r);
    }
}

// ===========================================================================
// E16 -- complexmode mode 2: log_message NULL or empty -> "Log message
//        creation failed" and NO free. Unreachable because
//        create_result_string always writes a non-empty prefix; verified
//        structurally that the Rust has the same condition and the same
//        no-free branch.
// ===========================================================================
#[test]
fn e16_complexmode_mode2_log_failure_branch() {
    let c = read_src("c_src/src/lib.c");
    let r = read_src("translation/src/lib.rs");
    assert!(
        c.contains("if (log_message == NULL || strcmp(log_message, \"\") == 0) {"),
        "C condition moved"
    );
    assert!(
        r.contains("if log_message.is_null() || c_strcmp(log_message, cstr!(\"\")) == 0 {"),
        "Rust mode-2 log-failure condition diverges from the C"
    );
    assert!(
        c.contains("Log message creation failed") && r.contains("Log message creation failed"),
        "both must carry the same failure text"
    );
    // reachable half: the branch is never taken, for any (v1,v2)
    let mut rng = Rng::with_seed(116);
    let mut pairs: Vec<(i32, i32)> = vec![(0, 0), (i32::MIN, -1), (i32::MAX, i32::MAX)];
    for _ in 0..ITERS {
        pairs.push((rng.i32_mixed(), rng.i32_mixed()));
    }
    for (a, b) in pairs {
        let (c, r) = each::<FnComplexmode, _>("complexmode", |f| {
            capture(|| unsafe { f(2, a, b, 0) })
        });
        assert!(
            !c.1.starts_with(b"Log message creation failed"),
            "E16: C hit the unreachable branch for ({a},{b})"
        );
        same(&format!("E16 complexmode(2,{a},{b},0)"), c, r);
    }
}

// ===========================================================================
// E17 -- signed-integer overflow in every arithmetic site.
// ===========================================================================
#[test]
fn e17_signed_overflow_everywhere() {
    let ext = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, -1, 1, 0, 2, -2];

    // safe_add (permitted, so the add is actually reached)
    for &a in &ext {
        for &b in &ext {
            let (c, r) = each::<FnSafeAdd, _>("safe_add", |f| capture(|| unsafe { f(a, b, 0o644) }));
            same(&format!("E17 safe_add({a},{b},0644)"), c, r);
        }
    }
    // multiply_with_log -- note the C computes a*b TWICE (once for the log
    // string, once for the return), so both overflows must agree.
    for &a in &ext {
        for &b in &ext {
            let (c, r) = each::<FnMultiplyWithLog, _>("multiply_with_log", |f| {
                capture(|| unsafe {
                    let mut o: *mut c_char = std::ptr::null_mut();
                    let rv = f(a, b, &mut o);
                    let s = read_c_buf(o, 64);
                    if !o.is_null() {
                        libc_free(o as *mut _);
                    }
                    (rv, s)
                })
            });
            same(&format!("E17 multiply_with_log({a},{b})"), c, r);
        }
    }
    // copy_and_sum accumulator
    for &a in &ext {
        for &b in &ext {
            let mut cb = vec![a, b, a, b, a];
            let mut rb = cb.clone();
            let l = libs();
            let cf: libloading::Symbol<FnCopyAndSum> = sym(&l.c, "copy_and_sum");
            let rf: libloading::Symbol<FnCopyAndSum> = sym(&l.r, "copy_and_sum");
            let c = capture(|| unsafe { cf(cb.as_mut_ptr(), 5) });
            let r = capture(|| unsafe { rf(rb.as_mut_ptr(), 5) });
            same(&format!("E17 copy_and_sum([{a},{b},..],5)"), c, r);
        }
    }
    // complexmode modes 1..4
    for mode in 1..=4 {
        for &a in &ext {
            for &b in &ext {
                for &c3 in &[i32::MAX, i32::MIN, 0, -1] {
                    let (c, r) = each::<FnComplexmode, _>("complexmode", |f| {
                        capture(|| unsafe { f(mode, a, b, c3) })
                    });
                    same(&format!("E17 complexmode({mode},{a},{b},{c3})"), c, r);
                }
            }
        }
    }
}

// ===========================================================================
// E18 -- check_permissions has NO rejection path: required == 0 is vacuously
//        satisfied even for perms == 0. Recorded to prove the Rust adds none.
// ===========================================================================
#[test]
fn e18_check_permissions_has_no_rejection_path() {
    let mut rng = Rng::with_seed(118);
    let mut pairs: Vec<(i32, i32)> = vec![(0, 0), (0, -1), (-1, 0), (-1, -1), (i32::MIN, i32::MIN)];
    for &a in BOUNDS {
        for &b in BOUNDS {
            pairs.push((a, b));
        }
    }
    for _ in 0..ITERS * 4 {
        pairs.push((rng.i32_mixed(), rng.i32_mixed()));
    }
    for (perms, required) in pairs {
        let (c, r) = each::<FnCheckPermissions, _>("check_permissions", |f| {
            capture(|| unsafe { f(perms, required) })
        });
        assert!(c.1.is_empty(), "E18: check_permissions must print nothing");
        assert!(c.0 == 0 || c.0 == 1, "E18: C returned {} (not a 0/1 bool)", c.0);
        same(&format!("E18 check_permissions({perms},{required})"), c, r);
    }
    // the vacuous-mask row itself
    let (c, r) = each::<FnCheckPermissions, _>("check_permissions", |f| {
        capture(|| unsafe { f(0, 0) })
    });
    assert_eq!(c.0, 1, "E18: (0 & 0) == 0 must be true");
    same("E18 check_permissions(0,0)", c, r);
}

// ===========================================================================
// Generic FFI boundaries not tied to a single ERRORS.md row.
// ===========================================================================
#[test]
fn generic_null_pointers_everywhere() {
    // every pointer parameter of every export, set to NULL
    let ok = CString::new("op").unwrap();

    // create_result_string(NULL, .) -- covered by E2, re-asserted as a boundary
    let (c, r) = each::<FnCreateResultString, _>("create_result_string", |f| {
        capture(|| unsafe {
            let p = f(std::ptr::null(), 0);
            let b = read_c_buf(p, 64);
            if !p.is_null() {
                libc_free(p as *mut _);
            }
            b
        })
    });
    same("generic create_result_string(NULL,0)", c, r);

    // copy_and_sum(NULL, .)
    let (c, r) = each::<FnCopyAndSum, _>("copy_and_sum", |f| {
        capture(|| unsafe { f(std::ptr::null_mut(), 3) })
    });
    same("generic copy_and_sum(NULL,3)", c, r);

    // compare_operations with each side NULL
    for (a, b, tag) in [
        (std::ptr::null(), ok.as_ptr(), "NULL,ok"),
        (ok.as_ptr(), std::ptr::null(), "ok,NULL"),
        (std::ptr::null(), std::ptr::null(), "NULL,NULL"),
    ] {
        let (c, r) = each::<FnCompareOperations, _>("compare_operations", |f| {
            capture(|| unsafe { f(a, b) })
        });
        same(&format!("generic compare_operations({tag})"), c, r);
    }
}

#[test]
fn generic_zero_and_one_past_range_lengths() {
    let mut buf: Vec<c_int> = (0..8).map(|i| (i * 7) as c_int).collect();
    // 0 (empty), 1, exactly the buffer length, and one past it
    for count in [0i32, 1, 8, 9] {
        let mut cb = buf.clone();
        let mut rb = buf.clone();
        let l = libs();
        let cf: libloading::Symbol<FnCopyAndSum> = sym(&l.c, "copy_and_sum");
        let rf: libloading::Symbol<FnCopyAndSum> = sym(&l.r, "copy_and_sum");
        let c = capture(|| unsafe { cf(cb.as_mut_ptr(), count) });
        let r = capture(|| unsafe { rf(rb.as_mut_ptr(), count) });
        // count == 9 reads one int past the Vec; both libraries read the same
        // adjacent bytes only if they behave identically, which is the point.
        if count <= 8 {
            same(&format!("generic copy_and_sum(len8,{count})"), c, r);
        } else {
            // one-past read: the value is allocator-dependent, so only the
            // control flow (no message, normal return) is comparable.
            assert_eq!(c.1, r.1, "generic count=9: stdout differs");
        }
    }
    // permission masks one step either side of the documented constants
    for p in [
        0o377, 0o400, 0o401, 0o177, 0o200, 0o201, 0o077, 0o100, 0o101, 0o577, 0o600, 0o601,
    ] {
        let (c, r) = each::<FnSafeAdd, _>("safe_add", |f| capture(|| unsafe { f(1, 1, p) }));
        same(&format!("generic safe_add(1,1,{p:#o})"), c, r);
        for req in [0o377, 0o400, 0o401, 0o600, 0o601, 0o777, 0o1000] {
            let (c, r) = each::<FnCheckPermissions, _>("check_permissions", |f| {
                capture(|| unsafe { f(p, req) })
            });
            same(&format!("generic check_permissions({p:#o},{req:#o})"), c, r);
        }
    }
    // complexmode: one step past each end of the valid mode range
    for m in [0i32, 1, 4, 5] {
        let (c, r) = each::<FnComplexmode, _>("complexmode", |f| {
            capture(|| unsafe { f(m, 1, 2, 3) })
        });
        same(&format!("generic complexmode({m},1,2,3)"), c, r);
    }
}
