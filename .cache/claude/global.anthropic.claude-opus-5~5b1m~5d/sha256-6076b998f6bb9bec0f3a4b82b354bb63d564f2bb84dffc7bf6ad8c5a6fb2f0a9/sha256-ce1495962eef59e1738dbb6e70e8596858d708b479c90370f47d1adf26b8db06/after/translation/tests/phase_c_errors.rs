// PHASE C -- error-path differential tests.
//
// One test per row of ERRORS.md. Each constructs that exact invalid input and
// asserts the two shared objects reject it *identically* -- same sentinel value,
// same state mutation, same termination signal -- not merely "both failed".

mod common;

use common::*;
use std::ptr;

// ---------------------------------------------------------------------------
// Rows 1, 2, 3 -- is_valid_operation rejections: NUL, below '1', above '5'
// ---------------------------------------------------------------------------
#[test]
fn err_01_02_03_is_valid_operation_rejections() {
    let l = libs();

    // Row 1: NUL fails the first `&&` operand.
    let c = unsafe { (l.c.is_valid_operation)(0) };
    let r = unsafe { (l.rust.is_valid_operation)(0) };
    assert_same(c, r, "is_valid_operation(0)");
    assert_eq!(c, 0, "row 1: NUL must be rejected");

    // Row 2: strictly below '1' (49).
    for v in 1i8..=48 {
        let c = unsafe { (l.c.is_valid_operation)(v) };
        let r = unsafe { (l.rust.is_valid_operation)(v) };
        assert_same(c, r, &format!("is_valid_operation({v})"));
        assert_eq!(c, 0, "row 2: {v} < '1' must be rejected");
    }

    // Row 3: strictly above '5' (53).
    for v in 54i8..=127 {
        let c = unsafe { (l.c.is_valid_operation)(v) };
        let r = unsafe { (l.rust.is_valid_operation)(v) };
        assert_same(c, r, &format!("is_valid_operation({v})"));
        assert_eq!(c, 0, "row 3: {v} > '5' must be rejected");
    }

    // ...and the accepted band, to prove the rejection is not blanket.
    for v in 49i8..=53 {
        let c = unsafe { (l.c.is_valid_operation)(v) };
        let r = unsafe { (l.rust.is_valid_operation)(v) };
        assert_same(c, r, &format!("is_valid_operation({v})"));
        assert_eq!(c, 1, "'1'..'5' must be accepted");
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- negative (signed) char
// ---------------------------------------------------------------------------
#[test]
fn err_04_is_valid_operation_negative_char() {
    let l = libs();
    for v in i8::MIN..0 {
        let c = unsafe { (l.c.is_valid_operation)(v) };
        let r = unsafe { (l.rust.is_valid_operation)(v) };
        assert_same(c, r, &format!("is_valid_operation({v})"));
        assert_eq!(c, 0, "row 4: negative char {v} must be rejected");
    }
}

// ---------------------------------------------------------------------------
// Row 5 -- divide_operation guards b == 0 by returning 0
// ---------------------------------------------------------------------------
#[test]
fn err_05_divide_by_zero() {
    let l = libs();
    let mut rng = Rng::new();
    let mut cases: Vec<i32> = vec![0, 1, -1, 7, -7, i32::MAX, i32::MIN];
    for _ in 0..500 {
        cases.push(rng.interesting_i32());
    }
    for a in cases {
        for unused in [0, 1, -1, i32::MAX] {
            let c = unsafe { (l.c.divide_operation)(a, 0, unused) };
            let r = unsafe { (l.rust.divide_operation)(a, 0, unused) };
            assert_same(c, r, &format!("divide_operation({a}, 0, {unused})"));
            assert_eq!(c, 0, "row 5: the b==0 guard must return the sentinel 0");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 -- modulo_operation guards b == 0 by returning 0
// ---------------------------------------------------------------------------
#[test]
fn err_06_modulo_by_zero() {
    let l = libs();
    let mut rng = Rng::new();
    let mut cases: Vec<i32> = vec![0, 1, -1, 7, -7, i32::MAX, i32::MIN];
    for _ in 0..500 {
        cases.push(rng.interesting_i32());
    }
    for a in cases {
        for unused in [0, 1, -1, i32::MAX] {
            let c = unsafe { (l.c.modulo_operation)(a, 0, unused) };
            let r = unsafe { (l.rust.modulo_operation)(a, 0, unused) };
            assert_same(c, r, &format!("modulo_operation({a}, 0, {unused})"));
            assert_eq!(c, 0, "row 6: the b==0 guard must return the sentinel 0");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 -- select_operation with out-of-range enum values (incl. one step past
//          each end of the valid 1..5 range) must fall back to add_operation
// ---------------------------------------------------------------------------
#[test]
fn err_07_select_operation_out_of_range_enum() {
    let l = libs();
    let mut bad: Vec<i32> = vec![0, 6, -1, 7, -2, 100, -100, i32::MIN, i32::MAX, i32::MIN + 1];
    let mut rng = Rng::new();
    for _ in 0..500 {
        let v = rng.next_i32();
        if !(1..=5).contains(&v) {
            bad.push(v);
        }
    }

    for op in bad {
        let cf = unsafe { (l.c.select_operation)(op) };
        let rf = unsafe { (l.rust.select_operation)(op) };
        assert!(cf.is_some(), "row 7: C select_operation({op}) must not return NULL");
        assert!(
            rf.is_some(),
            "row 7: Rust select_operation({op}) must not return NULL"
        );
        let (cf, rf) = (cf.unwrap(), rf.unwrap());

        // Same fallback behaviour, and specifically the ADD behaviour.
        let c_add = unsafe { (l.c.add_operation)(0, 0, 0) };
        let _ = c_add;
        for (a, b) in [(3, 4), (-3, 4), (i32::MAX, 1), (i32::MIN, -1), (0, 0), (7, 0)] {
            let c = unsafe { cf(a, b, 0) };
            let r = unsafe { rf(a, b, 0) };
            assert_same(c, r, &format!("select_operation({op})({a}, {b})"));
            assert_eq!(
                c,
                a.wrapping_add(b),
                "row 7: the default arm must be add_operation for op={op}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 -- get_operation_priority has no range check; op*10 overflows silently
// ---------------------------------------------------------------------------
#[test]
fn err_08_get_operation_priority_overflow() {
    let l = libs();
    let mut cases: Vec<i32> = vec![
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x1999_999A,
        -0x1999_999A,
        214_748_365,
        -214_748_365,
        0,
        6,
        -1,
    ];
    let mut rng = Rng::new();
    for _ in 0..1000 {
        cases.push(rng.next_i32());
    }
    for op in cases {
        let c = unsafe { (l.c.get_operation_priority)(op) };
        let r = unsafe { (l.rust.get_operation_priority)(op) };
        assert_same(c, r, &format!("get_operation_priority({op})"));
        assert_eq!(
            c,
            op.wrapping_mul(10),
            "row 8: no check, gcc wraps the signed overflow"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 9 -- allocate_results with a negative count -> NULL
// ---------------------------------------------------------------------------
#[test]
fn err_09_allocate_results_negative_count() {
    let l = libs();
    for &count in &[-1i32, -2, -10, -1000, i32::MIN, i32::MIN + 1, -0x4000_0000] {
        let cp = unsafe { (l.c.allocate_results)(count) };
        let rp = unsafe { (l.rust.allocate_results)(count) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "row 9: allocate_results({count}) null-ness differs (C {cp:?} / Rust {rp:?})"
        );
        assert!(
            cp.is_null(),
            "row 9: allocate_results({count}) must fail (sign-extended huge size_t)"
        );
        if !cp.is_null() {
            unsafe { free(cp as *mut _) };
        }
        if !rp.is_null() {
            unsafe { free(rp as *mut _) };
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 -- allocate_results(0) -> glibc returns a non-NULL unique pointer
// ---------------------------------------------------------------------------
#[test]
fn err_10_allocate_results_zero_count() {
    let l = libs();
    let cp = unsafe { (l.c.allocate_results)(0) };
    let rp = unsafe { (l.rust.allocate_results)(0) };
    assert_eq!(
        cp.is_null(),
        rp.is_null(),
        "row 10: allocate_results(0) null-ness differs (C {cp:?} / Rust {rp:?})"
    );
    assert!(
        !cp.is_null(),
        "row 10: calloc(0, 24) returns a unique non-NULL pointer on glibc"
    );
    unsafe {
        free(cp as *mut _);
        free(rp as *mut _);
    }
}

// ---------------------------------------------------------------------------
// Row 11 -- oversized count -> NULL
// ---------------------------------------------------------------------------
#[test]
fn err_11_allocate_results_oversized_count() {
    let l = libs();
    for &count in &[i32::MAX, i32::MAX - 1, 0x4000_0000, 0x2000_0000] {
        let cp = unsafe { (l.c.allocate_results)(count) };
        let rp = unsafe { (l.rust.allocate_results)(count) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "row 11: allocate_results({count}) null-ness differs (C {cp:?} / Rust {rp:?})"
        );
        if !cp.is_null() {
            unsafe { free(cp as *mut _) };
        }
        if !rp.is_null() {
            unsafe { free(rp as *mut _) };
        }
    }
    // The largest of these (48 GiB) must fail on any sane machine.
    let cp = unsafe { (l.c.allocate_results)(i32::MAX) };
    let rp = unsafe { (l.rust.allocate_results)(i32::MAX) };
    assert!(cp.is_null() && rp.is_null(), "row 11: 48 GiB request should fail");
}

// ---------------------------------------------------------------------------
// Row 12 -- *history == NULL forcibly resets a non-zero *history_count
// ---------------------------------------------------------------------------
#[test]
fn err_12_pcwh_null_history_resets_count() {
    let l = libs();
    for &stale in &[1i32, 5, 9, 10, 11, 99, -1, i32::MAX, i32::MIN] {
        let mut ch: *mut ComputationResult = ptr::null_mut();
        let mut cc = stale;
        let cres = unsafe { (l.c.perform_computation_with_history)(11, 4, OP_ADD, &mut ch, &mut cc) };

        let mut rh: *mut ComputationResult = ptr::null_mut();
        let mut rc = stale;
        let rres =
            unsafe { (l.rust.perform_computation_with_history)(11, 4, OP_ADD, &mut rh, &mut rc) };

        assert_same(cres, rres, &format!("row 12 return (stale={stale})"));
        assert_same(cc, rc, &format!("row 12 count (stale={stale})"));
        assert_eq!(cc, 1, "row 12: the count must be reset to 0 then incremented");
        assert_eq!(
            unsafe { slots(ch, 1) }[0].value,
            15,
            "row 12: the record must land in slot 0"
        );
        assert_eq!(
            unsafe { raw_bytes(ch, 10) },
            unsafe { raw_bytes(rh, 10) },
            "row 12: buffer differs (stale={stale})"
        );
        unsafe {
            free(ch as *mut _);
            free(rh as *mut _);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 -- at capacity (count == 10) the record is silently dropped but the
//           computed value is still returned
// ---------------------------------------------------------------------------
#[test]
fn err_13_pcwh_capacity_reached_silent_drop() {
    let l = libs();
    let mut rng = Rng::new();

    for _ in 0..200 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let op = rng.interesting_op();
        if (op == OP_DIVIDE || op == OP_MODULO) && is_c_div_trap(a, b) {
            continue;
        }
        let mut ch = unsafe { (l.c.allocate_results)(10) };
        let mut rh = unsafe { (l.rust.allocate_results)(10) };
        assert!(!ch.is_null() && !rh.is_null());
        let before_c = unsafe { raw_bytes(ch, 10) };

        let mut cc = HISTORY_CAPACITY;
        let mut rc = HISTORY_CAPACITY;
        let cres = unsafe { (l.c.perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
        let rres = unsafe { (l.rust.perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };

        let ctx = format!("row 13: pcwh(a={a}, b={b}, op={op}, count=10)");
        assert_same(cres, rres, &format!("{ctx} return"));
        assert_same(cc, rc, &format!("{ctx} count"));
        assert_eq!(cc, 10, "{ctx}: count must NOT advance past capacity");
        assert_eq!(
            unsafe { raw_bytes(ch, 10) },
            before_c,
            "{ctx}: nothing may be written at capacity"
        );
        assert_eq!(
            unsafe { raw_bytes(ch, 10) },
            unsafe { raw_bytes(rh, 10) },
            "{ctx}: buffers differ"
        );
        unsafe {
            free(ch as *mut _);
            free(rh as *mut _);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 -- count already ABOVE capacity: same silent drop, no clamping
// ---------------------------------------------------------------------------
#[test]
fn err_14_pcwh_count_above_capacity() {
    let l = libs();
    for &start in &[11i32, 12, 50, 1000, i32::MAX, i32::MAX - 1] {
        let mut ch = unsafe { (l.c.allocate_results)(10) };
        let mut rh = unsafe { (l.rust.allocate_results)(10) };
        assert!(!ch.is_null() && !rh.is_null());
        let before = unsafe { raw_bytes(ch, 10) };

        let mut cc = start;
        let mut rc = start;
        let cres = unsafe { (l.c.perform_computation_with_history)(6, 7, OP_MULTIPLY, &mut ch, &mut cc) };
        let rres =
            unsafe { (l.rust.perform_computation_with_history)(6, 7, OP_MULTIPLY, &mut rh, &mut rc) };

        assert_same(cres, rres, &format!("row 14 return (count={start})"));
        assert_same(cc, rc, &format!("row 14 count (count={start})"));
        assert_eq!(cres, 42, "row 14: the value is still computed and returned");
        assert_eq!(cc, start, "row 14: no clamping, no increment");
        assert_eq!(
            unsafe { raw_bytes(ch, 10) },
            before,
            "row 14: nothing written (count={start})"
        );
        assert_eq!(
            unsafe { raw_bytes(ch, 10) },
            unsafe { raw_bytes(rh, 10) },
            "row 14: buffers differ (count={start})"
        );
        unsafe {
            free(ch as *mut _);
            free(rh as *mut _);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 -- a NEGATIVE count passes the `< 10` guard and writes out of bounds
//           at index -1. To observe that store safely we hand the library a
//           pointer into the MIDDLE of a buffer we own, so index -1 still lands
//           inside our own allocation and can be compared byte-for-byte.
// ---------------------------------------------------------------------------
#[test]
fn err_15_pcwh_negative_count_oob_write() {
    let l = libs();

    for &start in &[-1i32, -2, -5] {
        let n = 12usize;
        let base_c = unsafe { (l.c.allocate_results)(n as i32) };
        let base_r = unsafe { (l.rust.allocate_results)(n as i32) };
        assert!(!base_c.is_null() && !base_r.is_null());

        // Offset the "history" pointer far enough that index `start` stays inside.
        let off = 6isize;
        let mut ch = unsafe { base_c.offset(off) };
        let mut rh = unsafe { base_r.offset(off) };
        let mut cc = start;
        let mut rc = start;

        let cres = unsafe { (l.c.perform_computation_with_history)(20, 5, OP_SUBTRACT, &mut ch, &mut cc) };
        let rres =
            unsafe { (l.rust.perform_computation_with_history)(20, 5, OP_SUBTRACT, &mut rh, &mut rc) };

        let ctx = format!("row 15: pcwh(count={start}) negative-index store");
        assert_same(cres, rres, &format!("{ctx} return"));
        assert_same(cc, rc, &format!("{ctx} count"));
        assert_eq!(cres, 15, "{ctx}: value still computed");
        assert_eq!(cc, start + 1, "{ctx}: the count is incremented, not rejected");

        // The whole 12-slot allocation, including the out-of-bounds slot.
        assert_eq!(
            unsafe { raw_bytes(base_c, n) },
            unsafe { raw_bytes(base_r, n) },
            "{ctx}: the negative-index store must land at the same offset with the same bytes"
        );
        let written = unsafe { *base_c.offset(off + start as isize) };
        assert_eq!(written.value, 15, "{ctx}: expected the store at index {start}");
        assert_eq!(written.status, 0, "{ctx}: STATUS_SUCCESS");

        unsafe {
            free(base_c as *mut _);
            free(base_r as *mut _);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 -- out-of-range enum `op` reaching perform_computation_with_history
// ---------------------------------------------------------------------------
#[test]
fn err_16_pcwh_out_of_range_enum() {
    let l = libs();
    for &op in &[0i32, 6, -1, 7, -100, i32::MIN, i32::MAX] {
        let mut ch: *mut ComputationResult = ptr::null_mut();
        let mut cc = 0;
        let cres = unsafe { (l.c.perform_computation_with_history)(9, 5, op, &mut ch, &mut cc) };

        let mut rh: *mut ComputationResult = ptr::null_mut();
        let mut rc = 0;
        let rres = unsafe { (l.rust.perform_computation_with_history)(9, 5, op, &mut rh, &mut rc) };

        assert_same(cres, rres, &format!("row 16 return (op={op})"));
        assert_same(cc, rc, &format!("row 16 count (op={op})"));
        assert_eq!(cres, 14, "row 16: op={op} must fall back to ADD (9 + 5)");
        assert_eq!(cc, 1, "row 16: the record is still written");
        assert_eq!(
            unsafe { raw_bytes(ch, 10) },
            unsafe { raw_bytes(rh, 10) },
            "row 16: buffer differs (op={op})"
        );
        unsafe {
            free(ch as *mut _);
            free(rh as *mut _);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17/18 -- NULL pointer arguments: the C has no null checks at all, so
//               both implementations must die with the SAME signal. Each call
//               runs in a forked child so the crash is contained.
// ---------------------------------------------------------------------------
#[test]
fn err_17_18_pcwh_null_pointers_crash() {
    let l = libs();
    let _g = global_lock();

    const SIGSEGV: i32 = 11;

    // Row 17: `history` itself is NULL -> dereference of NULL.
    let mut dummy_count: i32 = 0;
    let dummy_ptr: *mut i32 = &mut dummy_count;
    let c_out = run_isolated(|| unsafe {
        (l.c.perform_computation_with_history)(1, 2, OP_ADD, ptr::null_mut(), dummy_ptr);
    });
    let r_out = run_isolated(|| unsafe {
        (l.rust.perform_computation_with_history)(1, 2, OP_ADD, ptr::null_mut(), dummy_ptr);
    });
    assert_eq!(
        c_out, r_out,
        "row 17: history == NULL must terminate identically (C {c_out:?} / Rust {r_out:?})"
    );
    assert_eq!(
        c_out,
        Outcome::Signalled(SIGSEGV),
        "row 17: expected SIGSEGV from the unchecked *history dereference"
    );

    // Row 18: `history_count` is NULL while *history is a live buffer.
    let cbuf = unsafe { (l.c.allocate_results)(10) };
    let rbuf = unsafe { (l.rust.allocate_results)(10) };
    assert!(!cbuf.is_null() && !rbuf.is_null());
    let mut chp = cbuf;
    let mut rhp = rbuf;
    let chp_ptr: *mut *mut ComputationResult = &mut chp;
    let rhp_ptr: *mut *mut ComputationResult = &mut rhp;

    let c_out = run_isolated(|| unsafe {
        (l.c.perform_computation_with_history)(1, 2, OP_ADD, chp_ptr, ptr::null_mut());
    });
    let r_out = run_isolated(|| unsafe {
        (l.rust.perform_computation_with_history)(1, 2, OP_ADD, rhp_ptr, ptr::null_mut());
    });
    assert_eq!(
        c_out, r_out,
        "row 18: history_count == NULL must terminate identically (C {c_out:?} / Rust {r_out:?})"
    );
    assert_eq!(
        c_out,
        Outcome::Signalled(SIGSEGV),
        "row 18: expected SIGSEGV from the unchecked *history_count dereference"
    );

    // And with BOTH NULL, for good measure.
    let c_out = run_isolated(|| unsafe {
        (l.c.perform_computation_with_history)(1, 2, OP_ADD, ptr::null_mut(), ptr::null_mut());
    });
    let r_out = run_isolated(|| unsafe {
        (l.rust.perform_computation_with_history)(1, 2, OP_ADD, ptr::null_mut(), ptr::null_mut());
    });
    assert_eq!(c_out, r_out, "both-NULL must terminate identically");

    unsafe {
        free(cbuf as *mut _);
        free(rbuf as *mut _);
    }
}

// ---------------------------------------------------------------------------
// Rows 21/22 -- STATUS_SUCCESS is the only status ever stored, and no function
//               uses -1/STATUS_ERROR as an error signal
// ---------------------------------------------------------------------------
#[test]
fn err_21_22_status_is_always_success() {
    let l = libs();
    let mut rng = Rng::new();

    let mut ch: *mut ComputationResult = ptr::null_mut();
    let mut cc = 0;
    let mut rh: *mut ComputationResult = ptr::null_mut();
    let mut rc = 0;

    let mut recorded = 0;
    for _ in 0..400 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        let op = rng.interesting_op();
        if (op == OP_DIVIDE || op == OP_MODULO) && is_c_div_trap(a, b) {
            continue;
        }
        // Reset so we keep recording rather than saturating.
        if cc >= HISTORY_CAPACITY {
            cc = 0;
            rc = 0;
        }
        let cres = unsafe { (l.c.perform_computation_with_history)(a, b, op, &mut ch, &mut cc) };
        let rres = unsafe { (l.rust.perform_computation_with_history)(a, b, op, &mut rh, &mut rc) };
        assert_same(cres, rres, "rows 21/22 return");
        assert_same(cc, rc, "rows 21/22 count");
        recorded += 1;

        let idx = (cc - 1) as usize;
        let cslot = unsafe { slots(ch, 10) }[idx];
        let rslot = unsafe { slots(rh, 10) }[idx];
        assert_eq!(cslot, rslot, "rows 21/22: record differs");
        assert_eq!(
            cslot.status, 0,
            "row 21: status must always be STATUS_SUCCESS, never ERROR/WARNING"
        );
    }
    assert!(recorded > 300, "expected most iterations to record");

    // Row 22: -1 is never an error signal -- prove the library returns -1 as a
    // perfectly ordinary arithmetic result and keeps going.
    let mut h: *mut ComputationResult = ptr::null_mut();
    let mut n = 0;
    let c = unsafe { (l.c.perform_computation_with_history)(2, 3, OP_SUBTRACT, &mut h, &mut n) };
    assert_eq!(c, -1, "2 - 3 == -1, a value not an error");
    assert_eq!(n, 1, "row 22: the -1 result was still recorded as SUCCESS");
    assert_eq!(unsafe { slots(h, 1) }[0].status, 0);
    let mut h2: *mut ComputationResult = ptr::null_mut();
    let mut n2 = 0;
    let r = unsafe { (l.rust.perform_computation_with_history)(2, 3, OP_SUBTRACT, &mut h2, &mut n2) };
    assert_same(c, r, "row 22 return");
    assert_same(n, n2, "row 22 count");
    assert_eq!(
        unsafe { raw_bytes(h, 10) },
        unsafe { raw_bytes(h2, 10) },
        "row 22 buffer"
    );
    unsafe {
        free(ch as *mut _);
        free(rh as *mut _);
        free(h as *mut _);
        free(h2 as *mut _);
    }
}

// ---------------------------------------------------------------------------
// Row 25 -- DOCUMENTED C UNDEFINED BEHAVIOUR: INT_MIN / -1 and INT_MIN % -1.
//
// On x86-64 the C code's `idiv` traps and the process is killed by SIGFPE, so
// the C function produces NO return value for the Rust to match. The Rust uses
// wrapping_div / wrapping_rem and returns normally. This test pins both observed
// outcomes so the divergence is proven and cannot change silently.
// ---------------------------------------------------------------------------
#[test]
fn ub_divide_int_min_by_minus_one() {
    let l = libs();
    let _g = global_lock();

    const SIGFPE: i32 = 8;

    for (name, cf, rf) in [
        ("divide_operation", l.c.divide_operation, l.rust.divide_operation),
        ("modulo_operation", l.c.modulo_operation, l.rust.modulo_operation),
    ] {
        let c_out = run_isolated(|| unsafe {
            let v = cf(i32::MIN, -1, 0);
            std::hint::black_box(v);
        });
        let r_out = run_isolated(|| unsafe {
            let v = rf(i32::MIN, -1, 0);
            std::hint::black_box(v);
        });
        assert_eq!(
            c_out,
            Outcome::Signalled(SIGFPE),
            "{name}(INT_MIN, -1): the C is expected to trap (undefined behaviour)"
        );
        assert_eq!(
            r_out,
            Outcome::Exited(0),
            "{name}(INT_MIN, -1): the Rust is expected to return normally"
        );
    }

    // The Rust's defined results, pinned.
    assert_eq!(
        unsafe { (l.rust.divide_operation)(i32::MIN, -1, 0) },
        i32::MIN,
        "wrapping_div(INT_MIN, -1) == INT_MIN"
    );
    assert_eq!(
        unsafe { (l.rust.modulo_operation)(i32::MIN, -1, 0) },
        0,
        "wrapping_rem(INT_MIN, -1) == 0"
    );

    // Everything adjacent to the trap must still match exactly.
    for (a, b) in [
        (i32::MIN, 1),
        (i32::MIN, -2),
        (i32::MIN + 1, -1),
        (i32::MAX, -1),
        (i32::MIN, i32::MIN),
        (-1, -1),
    ] {
        assert_same(
            unsafe { (l.c.divide_operation)(a, b, 0) },
            unsafe { (l.rust.divide_operation)(a, b, 0) },
            &format!("divide_operation({a}, {b})"),
        );
        assert_same(
            unsafe { (l.c.modulo_operation)(a, b, 0) },
            unsafe { (l.rust.modulo_operation)(a, b, 0) },
            &format!("modulo_operation({a}, {b})"),
        );
    }
}
