//! Phase C — error-path differential tests.
//! One test per row of ERRORS.md (rows 20 and 21 live in `phase_c_trap.rs`
//! because the C ground truth kills the process and must be forked off).
//!
//! Every test asserts the two libraries produce the SAME rejection value /
//! sentinel — not merely "both failed somehow".

mod common;
use common::*;
use std::os::raw::{c_char, c_int};

const SEED: u64 = 0x5EED_1234_ABCD_0004;

// ===========================================================================
// Row 1 — is_valid_operation(0): the `op_char &&` short-circuit rejects NUL
// ===========================================================================
#[test]
fn err_01_is_valid_nul() {
    let p = pair();
    let c = p.c.is_valid_operation_raw(0);
    let r = p.rs.is_valid_operation_raw(0);
    assert_eq!(c, r, "is_valid_operation(0)");
    assert_eq!(c, 0, "NUL must be rejected");
}

// ===========================================================================
// Row 2 — op_char < '1'
// ===========================================================================
#[test]
fn err_02_is_valid_below_range() {
    let p = pair();
    for ch in 1i32..=0x30 {
        let ch = ch as c_char;
        let c = p.c.is_valid_operation_raw(ch);
        let r = p.rs.is_valid_operation_raw(ch);
        assert_eq!(c, r, "is_valid_operation({ch})");
        assert_eq!(c, 0, "{ch} < '1' must be rejected");
    }
    // '0' is the interesting boundary: exactly one below '1'.
    assert_eq!(p.c.is_valid_operation_raw(b'0' as c_char), 0);
    assert_eq!(p.rs.is_valid_operation_raw(b'0' as c_char), 0);
}

// ===========================================================================
// Row 3 — op_char > '5'
// ===========================================================================
#[test]
fn err_03_is_valid_above_range() {
    let p = pair();
    for ch in 0x36i32..=0x7f {
        let ch = ch as c_char;
        let c = p.c.is_valid_operation_raw(ch);
        let r = p.rs.is_valid_operation_raw(ch);
        assert_eq!(c, r, "is_valid_operation({ch})");
        assert_eq!(c, 0, "{ch} > '5' must be rejected");
    }
    // '6' is exactly one past the valid range.
    assert_eq!(p.c.is_valid_operation_raw(b'6' as c_char), 0);
    assert_eq!(p.rs.is_valid_operation_raw(b'6' as c_char), 0);
    // ...and '5' is the last accepted one.
    assert_eq!(p.c.is_valid_operation_raw(b'5' as c_char), 1);
    assert_eq!(p.rs.is_valid_operation_raw(b'5' as c_char), 1);
}

// ===========================================================================
// Row 4 — negative char (high bit set); `char` is SIGNED on x86-64
// ===========================================================================
#[test]
fn err_04_is_valid_negative_char() {
    let p = pair();
    for byte in 0x80u16..=0xff {
        let ch = byte as u8 as i8 as c_char;
        assert!(ch < 0, "0x{byte:02x} should be negative as a signed char");
        let c = p.c.is_valid_operation_raw(ch);
        let r = p.rs.is_valid_operation_raw(ch);
        assert_eq!(c, r, "is_valid_operation(0x{byte:02x} = {ch})");
        assert_eq!(c, 0, "negative char {ch} must be rejected");
    }
}

// ===========================================================================
// Row 5 — divide_operation guard: b == 0 returns 0, never traps
// ===========================================================================
#[test]
fn err_05_divide_by_zero() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 5);
    let mut a_vals = vec![0i32, 1, -1, 7, -7, i32::MAX, i32::MIN];
    for _ in 0..500 {
        a_vals.push(rng.i32_interesting());
    }
    for a in a_vals {
        for u in [0i32, 1, -1, i32::MAX] {
            let c = p.c.divide_operation(a, 0, u);
            let r = p.rs.divide_operation(a, 0, u);
            assert_eq!(c, r, "divide_operation({a}, 0, {u})");
            assert_eq!(c, 0, "the b == 0 guard must return exactly 0");
        }
    }
}

// ===========================================================================
// Row 6 — modulo_operation guard: b == 0 returns 0, never traps
// ===========================================================================
#[test]
fn err_06_modulo_by_zero() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 6);
    let mut a_vals = vec![0i32, 1, -1, 7, -7, i32::MAX, i32::MIN];
    for _ in 0..500 {
        a_vals.push(rng.i32_interesting());
    }
    for a in a_vals {
        for u in [0i32, 1, -1, i32::MAX] {
            let c = p.c.modulo_operation(a, 0, u);
            let r = p.rs.modulo_operation(a, 0, u);
            assert_eq!(c, r, "modulo_operation({a}, 0, {u})");
            assert_eq!(c, 0, "the b == 0 guard must return exactly 0");
        }
    }
}

// ===========================================================================
// Row 7 — select_operation with an OUT-OF-RANGE ENUM VALUE.
// A C enum accepts any int, so these are real inputs across the FFI boundary.
// The `default:` branch must hand back the `add_operation` pointer.
// ===========================================================================
#[test]
fn err_07_select_operation_out_of_range_enum() {
    let p = pair();
    let mut ops = vec![
        0i32, 6, 7, 8, 9, 10, 100, 1000, -1, -2, -3, -4, -5, -6, -100,
        i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1,
    ];
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..1000 {
        let v = rng.i32_interesting();
        if !(1..=5).contains(&v) {
            ops.push(v);
        }
    }
    let c_add = p.c.op_addr("add_operation");
    let rs_add = p.rs.op_addr("add_operation");
    for op in ops {
        let fc = p.c.select_operation(op);
        let fr = p.rs.select_operation(op);
        assert_eq!(
            fc as usize, c_add,
            "C select_operation({op}) must fall through to &add_operation"
        );
        assert_eq!(
            fr as usize, rs_add,
            "Rust select_operation({op}) must fall through to &add_operation"
        );
        // And behaviourally: addition.
        for (a, b) in [(0, 0), (5, 7), (-5, 7), (i32::MAX, 1), (i32::MIN, -1)] {
            let vc = unsafe { fc(a, b, 0) };
            let vr = unsafe { fr(a, b, 0) };
            assert_eq!(vc, vr, "select_operation({op})({a},{b},0)");
            assert_eq!(vc, a.wrapping_add(b));
        }
    }
}

// ===========================================================================
// Row 8 — the same out-of-range enum reached INDIRECTLY, through
// perform_computation_with_history (must silently behave as addition)
// ===========================================================================
#[test]
fn err_08_pcwh_out_of_range_enum_adds() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 8);
    let mut ops = vec![0i32, 6, -1, -5, 42, i32::MAX, i32::MIN];
    for _ in 0..200 {
        let v = rng.i32_interesting();
        if !(1..=5).contains(&v) {
            ops.push(v);
        }
    }
    for op in ops {
        for _ in 0..5 {
            let a = rng.i32_interesting();
            let b = rng.i32_interesting();
            let mut hc = p.c.allocate_results(10);
            let mut hr = p.rs.allocate_results(10);
            let mut cc: c_int = 0;
            let mut cr: c_int = 0;
            let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
            let vr = unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };
            assert_eq!(vc, vr, "pcwh op={op} a={a} b={b}");
            assert_eq!(vc, a.wrapping_add(b), "out-of-enum op must add");
            assert_eq!(cc, cr, "history_count for op={op}");
            assert_eq!(cc, 1);
        }
    }
}

// ===========================================================================
// Row 9 — allocate_results(0): calloc(0, 24) is NOT an error in glibc
// ===========================================================================
#[test]
fn err_09_allocate_zero_count() {
    let p = pair();
    let pc = p.c.allocate_results(0);
    let pr = p.rs.allocate_results(0);
    assert_eq!(
        pc.is_null(),
        pr.is_null(),
        "allocate_results(0): C null={} Rust null={}",
        pc.is_null(),
        pr.is_null()
    );
    assert!(!pc.is_null(), "glibc calloc(0, n) returns a unique non-NULL pointer");
}

// ===========================================================================
// Row 10 — allocate_results(negative): sign-extends to a huge size_t -> NULL
// ===========================================================================
#[test]
fn err_10_allocate_negative_count() {
    let p = pair();
    for count in [-1i32, -2, -10, -1000, i32::MIN, i32::MIN + 1] {
        let pc = p.c.allocate_results(count);
        let pr = p.rs.allocate_results(count);
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "allocate_results({count}): C null={} Rust null={}",
            pc.is_null(),
            pr.is_null()
        );
        assert!(pc.is_null(), "allocate_results({count}) must fail with NULL");
    }
}

// ===========================================================================
// Row 11 — allocate_results(INT_MAX): ~48 GiB request must return NULL
// ===========================================================================
#[test]
fn err_11_allocate_int_max() {
    let p = pair();
    for count in [i32::MAX, i32::MAX - 1, i32::MAX / 2] {
        let pc = p.c.allocate_results(count);
        let pr = p.rs.allocate_results(count);
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "allocate_results({count}): C null={} Rust null={}",
            pc.is_null(),
            pr.is_null()
        );
    }
}

// ===========================================================================
// Row 12 — *history == NULL resets the caller's *history_count to 0
// (the caller's count is DISCARDED - a subtle, easy-to-mistranslate detail)
// ===========================================================================
#[test]
fn err_12_pcwh_null_history_resets_count() {
    let p = pair();
    for garbage in [0i32, 1, 5, 9, 10, 11, 999, -1, -50, i32::MAX, i32::MIN] {
        let mut hc: *mut ComputationResult = std::ptr::null_mut();
        let mut hr: *mut ComputationResult = std::ptr::null_mut();
        let mut cc: c_int = garbage;
        let mut cr: c_int = garbage;
        let vc = unsafe { p.c.perform_computation_with_history(11, 22, 1, &mut hc, &mut cc) };
        let vr = unsafe { p.rs.perform_computation_with_history(11, 22, 1, &mut hr, &mut cr) };
        assert_eq!(vc, vr, "garbage count {garbage}: return");
        assert_eq!(vc, 33);
        assert_eq!(cc, cr, "garbage count {garbage}: count after");
        assert_eq!(cc, 1, "count {garbage} must be reset to 0 then incremented to 1");
        assert!(!hc.is_null() && !hr.is_null());
        let n = 10 * std::mem::size_of::<ComputationResult>();
        let bc = unsafe { std::slice::from_raw_parts(hc as *const u8, n) };
        let br = unsafe { std::slice::from_raw_parts(hr as *const u8, n) };
        assert_eq!(bc, br, "garbage count {garbage}: buffer image");
    }
}

// ===========================================================================
// Row 13 — *history_count >= 10: write AND increment are both skipped,
// but the arithmetic result is still returned
// ===========================================================================
#[test]
fn err_13_pcwh_history_full_skips_write() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 13);
    for full in [10i32, 11, 12, 100, 1000, i32::MAX, i32::MAX - 1] {
        for op in 1..=5i32 {
            for _ in 0..20 {
                let a = rng.i32_interesting();
                let b = rng.nonzero_i32();
                if is_div_trap(a, b) {
                    continue;
                }
                let mut hc = p.c.allocate_results(10);
                let mut hr = p.rs.allocate_results(10);
                let mut cc: c_int = full;
                let mut cr: c_int = full;
                let vc = unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
                let vr =
                    unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };
                assert_eq!(vc, vr, "full={full} op={op} a={a} b={b}: return");
                assert_eq!(vc, c_apply(op, a, b).unwrap(), "result must still be computed");
                assert_eq!(cc, cr, "full={full}: count");
                assert_eq!(cc, full, "count must NOT be incremented when >= 10");
                let n = 10 * std::mem::size_of::<ComputationResult>();
                let bc = unsafe { std::slice::from_raw_parts(hc as *const u8, n) };
                let br = unsafe { std::slice::from_raw_parts(hr as *const u8, n) };
                assert!(bc.iter().all(|&x| x == 0), "nothing must be written");
                assert_eq!(bc, br, "full={full}: buffer image");
            }
        }
    }
}

// ===========================================================================
// Row 14 — *history_count < 0: the `< 10` guard PASSES, so the C writes
// OUT OF BOUNDS *before* the buffer and increments the count. The C does not
// reject this, and the Rust must be equally unguarded.
//
// Done safely: allocate 20 slots, hand the function a pointer to slot 10, and
// use counts in -10..-1 so the OOB writes land inside the real allocation.
// ===========================================================================
#[test]
fn err_14_pcwh_negative_count_writes_oob() {
    let p = pair();
    const SLACK: isize = 10;
    for neg in [-1i32, -2, -5, -10] {
        for op in 1..=3i32 {
            let base_c = p.c.allocate_results(20);
            let base_r = p.rs.allocate_results(20);
            assert!(!base_c.is_null() && !base_r.is_null());
            let mut hc = unsafe { base_c.offset(SLACK) };
            let mut hr = unsafe { base_r.offset(SLACK) };
            let mut cc: c_int = neg;
            let mut cr: c_int = neg;

            let vc = unsafe { p.c.perform_computation_with_history(31, 7, op, &mut hc, &mut cc) };
            let vr = unsafe { p.rs.perform_computation_with_history(31, 7, op, &mut hr, &mut cr) };

            assert_eq!(vc, vr, "neg={neg} op={op}: return value");
            assert_eq!(cc, cr, "neg={neg} op={op}: count");
            assert_eq!(cc, neg + 1, "the `< 10` guard must pass and increment");

            // The whole 20-slot allocation, including the region before the
            // pointer we handed in, must match byte-for-byte.
            let n = 20 * std::mem::size_of::<ComputationResult>();
            let bc = unsafe { std::slice::from_raw_parts(base_c as *const u8, n) };
            let br = unsafe { std::slice::from_raw_parts(base_r as *const u8, n) };
            assert_eq!(bc, br, "neg={neg} op={op}: OOB write landed differently");
            // And it really did write before the given pointer.
            let touched = unsafe { *base_c.offset(SLACK + neg as isize) };
            assert_eq!(touched.value, c_apply(op, 31, 7).unwrap());
            assert_eq!(touched.status, 0);
        }
    }
}

// ===========================================================================
// Row 15 — status is ALWAYS STATUS_SUCCESS (0); STATUS_ERROR / STATUS_WARNING
// are declared but unreachable. A translation that reported errors here would
// diverge.
// ===========================================================================
#[test]
fn err_15_status_always_success() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 15);
    for op in [i32::MIN, -1, 0, 1, 2, 3, 4, 5, 6, i32::MAX] {
        let mut hc = p.c.allocate_results(10);
        let mut hr = p.rs.allocate_results(10);
        let mut cc: c_int = 0;
        let mut cr: c_int = 0;
        for _ in 0..10 {
            let a = rng.i32_interesting();
            // Include b == 0 so the divide/modulo guards are hit too.
            let b = if rng.next_u64() % 4 == 0 { 0 } else { rng.nonzero_i32() };
            if is_div_trap(a, b) && b != 0 {
                continue;
            }
            unsafe { p.c.perform_computation_with_history(a, b, op, &mut hc, &mut cc) };
            unsafe { p.rs.perform_computation_with_history(a, b, op, &mut hr, &mut cr) };
        }
        assert_eq!(cc, cr);
        let sc = unsafe { std::slice::from_raw_parts(hc as *const ComputationResult, 10) };
        let sr = unsafe { std::slice::from_raw_parts(hr as *const ComputationResult, 10) };
        for i in 0..cc.max(0) as usize {
            assert_eq!(sc[i].status, 0, "C slot {i} status must be STATUS_SUCCESS");
            assert_eq!(sr[i].status, 0, "Rust slot {i} status must be STATUS_SUCCESS");
            assert_eq!(sc[i], sr[i], "slot {i}");
        }
    }
}

// ===========================================================================
// Row 16 — mathop's validation is a DEAD STORE. An invalid validation char
// must have NO observable effect: mathop(p1, ...) depends on p1 only as the
// left operand, never on whether (char)(p1 % 128) was in '1'..'5'.
// ===========================================================================
#[test]
fn err_16_mathop_validation_is_dead_store() {
    let p = pair();
    // Pairs of param1 values that differ ONLY in validity but share the same
    // arithmetic role is impossible (param1 IS the operand), so instead prove
    // the two libraries agree on every validity class, and that the result
    // matches the model, which models NO validation effect at all.
    let mut cases: Vec<i32> = vec![
        0, 48, 49, 50, 51, 52, 53, 54, 127, 128, 176, 177, 181, 182, -49, -53, -1, i32::MIN,
        i32::MAX,
    ];
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..300 {
        cases.push(rng.i32_interesting());
    }
    for p1 in cases {
        let vchar = (p1.wrapping_rem(128)) as i8 as c_char;
        let valid_c = p.c.is_valid_operation_raw(vchar);
        let valid_r = p.rs.is_valid_operation_raw(vchar);
        assert_eq!(valid_c, valid_r, "validation char for param1={p1}");

        if mathop_traps(p1, 3, 1, 1) {
            continue;
        }
        let vc = p.c.mathop(p1, 3, 1, 1);
        let vr = p.rs.mathop(p1, 3, 1, 1);
        assert_eq!(vc, vr, "mathop({p1}, 3, 1, 1) (validity={valid_c})");
        let modelled = mathop_model(p1, 3, 1, 1, p.c.get_computation_timestamp());
        assert_eq!(
            Some(vc),
            modelled,
            "mathop({p1},3,1,1) must be unaffected by the dead validation store"
        );
    }
}

// ===========================================================================
// Row 17 — param3 < 0 => (param3 % 5) + 1 yields 0, -1, -2, -3: an
// out-of-range Operation. select_operation falls through to add, and
// get_operation_priority returns a NEGATIVE priority.
// ===========================================================================
#[test]
fn err_17_mathop_negative_param3_bad_enum() {
    let p = pair();
    for p3 in -20i32..0 {
        let op1 = mathop_op1(p3);
        assert!(op1 <= 1, "param3={p3} should give a non-positive-ish op ({op1})");
        // The priority the C computes for this bogus enum.
        let pc = p.c.get_operation_priority(op1);
        let pr = p.rs.get_operation_priority(op1);
        assert_eq!(pc, pr, "priority for bogus op {op1} (param3={p3})");
        assert_eq!(pc, op1 * 10);

        for p4 in [0i32, 1, 2, 3, 4, -1, -2, -7] {
            if mathop_traps(9, 4, p3, p4) {
                continue;
            }
            let vc = p.c.mathop(9, 4, p3, p4);
            let vr = p.rs.mathop(9, 4, p3, p4);
            assert_eq!(vc, vr, "mathop(9, 4, {p3}, {p4}) with bogus first op {op1}");
            assert_eq!(
                Some(vc),
                mathop_model(9, 4, p3, p4, p.c.get_computation_timestamp()),
                "mathop(9,4,{p3},{p4}) vs model"
            );
        }
    }
    // param3 == INT_MIN: INT_MIN % 5 == -3, so op1 == -2.
    assert_eq!(mathop_op1(i32::MIN), -2);
    if !mathop_traps(9, 4, i32::MIN, 1) {
        assert_eq!(p.c.mathop(9, 4, i32::MIN, 1), p.rs.mathop(9, 4, i32::MIN, 1));
    }
}

// ===========================================================================
// Row 18 — the SECOND op enum: ((param4 + 1) % 5) + 1, including the
// param4 == INT_MAX overflow of `param4 + 1`
// ===========================================================================
#[test]
fn err_18_mathop_negative_param4_bad_enum() {
    let p = pair();
    let mut p4s: Vec<i32> = (-20..20).collect();
    p4s.extend([i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1]);
    for p4 in p4s {
        let op2 = mathop_op2(p4);
        // Cross-check the enum the C would compute, including the overflow.
        assert_eq!(op2, p4.wrapping_add(1).wrapping_rem(5).wrapping_add(1));
        let pc = p.c.get_operation_priority(op2);
        let pr = p.rs.get_operation_priority(op2);
        assert_eq!(pc, pr, "priority for second op {op2} (param4={p4})");

        for p3 in [0i32, 1, 2, 3, 4, -1, -3] {
            if mathop_traps(9, 4, p3, p4) {
                continue;
            }
            let vc = p.c.mathop(9, 4, p3, p4);
            let vr = p.rs.mathop(9, 4, p3, p4);
            assert_eq!(vc, vr, "mathop(9, 4, {p3}, {p4}) second op {op2}");
            assert_eq!(
                Some(vc),
                mathop_model(9, 4, p3, p4, p.c.get_computation_timestamp()),
                "mathop(9,4,{p3},{p4}) vs model"
            );
        }
    }
    // param4 == -1 makes `(param4 + 1) % 5 + 1` exactly 1 (OP_ADD).
    assert_eq!(mathop_op2(-1), 1);
    // param4 == INT_MAX overflows: (INT_MAX+1) == INT_MIN, INT_MIN % 5 == -3.
    assert_eq!(mathop_op2(i32::MAX), -2);
}

// ===========================================================================
// Row 19 — param1 == INT_MIN: INT_MIN % 128 == 0, so the NUL rejection path
// ===========================================================================
#[test]
fn err_19_mathop_int_min_param1() {
    let p = pair();
    assert_eq!(i32::MIN.wrapping_rem(128), 0, "INT_MIN % 128 == 0");
    assert_eq!(p.c.is_valid_operation_raw(0), 0);
    assert_eq!(p.rs.is_valid_operation_raw(0), 0);
    for p2 in [0i32, 1, 2, -2, 7, i32::MAX] {
        for p3 in [0i32, 1, 2, 3, 4, -1] {
            for p4 in [0i32, 1, 2, 3, 4, -1] {
                if mathop_traps(i32::MIN, p2, p3, p4) {
                    continue;
                }
                let vc = p.c.mathop(i32::MIN, p2, p3, p4);
                let vr = p.rs.mathop(i32::MIN, p2, p3, p4);
                assert_eq!(vc, vr, "mathop(INT_MIN, {p2}, {p3}, {p4})");
            }
        }
    }
}

// ===========================================================================
// Row 22 — signed overflow wraps identically in every kernel and in
// get_operation_priority (`op * 10`)
// ===========================================================================
#[test]
fn err_22_signed_overflow_wraps() {
    let p = pair();
    let extremes = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 1 << 30, -(1 << 30)];
    for &a in &extremes {
        for &b in &extremes {
            assert_eq!(
                p.c.add_operation(a, b, 0),
                p.rs.add_operation(a, b, 0),
                "add_operation({a}, {b}) overflow"
            );
            assert_eq!(
                p.c.subtract_operation(a, b, 0),
                p.rs.subtract_operation(a, b, 0),
                "subtract_operation({a}, {b}) overflow"
            );
            assert_eq!(
                p.c.multiply_operation(a, b, 0),
                p.rs.multiply_operation(a, b, 0),
                "multiply_operation({a}, {b}) overflow"
            );
        }
        // `op * 10` overflows for |op| > INT_MAX/10
        assert_eq!(
            p.c.get_operation_priority(a),
            p.rs.get_operation_priority(a),
            "get_operation_priority({a}) overflow"
        );
    }
    for op in [214748365i32, 214748364, -214748365, i32::MAX / 10 + 1] {
        assert_eq!(
            p.c.get_operation_priority(op),
            p.rs.get_operation_priority(op),
            "get_operation_priority({op}) overflow boundary"
        );
    }
}

// ===========================================================================
// Generic FFI boundary sweeps that every C API needs, beyond the table.
// ===========================================================================

/// Every single-int entry point, over the full set of "one step past valid".
#[test]
fn err_generic_one_past_valid_enum_everywhere() {
    let p = pair();
    // The documented valid Operation range is 1..=5; probe 0 and 6 plus the
    // absolute extremes on every function that takes an Operation.
    for op in [0i32, 6, -1, i32::MIN, i32::MAX] {
        assert_eq!(
            p.c.get_operation_priority(op),
            p.rs.get_operation_priority(op),
            "get_operation_priority({op})"
        );
        assert_eq!(
            p.c.select_operation(op) as usize == p.c.op_addr("add_operation"),
            p.rs.select_operation(op) as usize == p.rs.op_addr("add_operation"),
            "select_operation({op}) default-ness"
        );
        let mut hc: *mut ComputationResult = std::ptr::null_mut();
        let mut hr: *mut ComputationResult = std::ptr::null_mut();
        let mut cc: c_int = 0;
        let mut cr: c_int = 0;
        assert_eq!(
            unsafe { p.c.perform_computation_with_history(5, 6, op, &mut hc, &mut cc) },
            unsafe { p.rs.perform_computation_with_history(5, 6, op, &mut hr, &mut cr) },
            "pcwh with op={op}"
        );
        assert_eq!(cc, cr);
    }
}

/// Zero and oversized lengths on the only length-taking function.
#[test]
fn err_generic_zero_and_oversized_lengths() {
    let p = pair();
    for count in [0i32, 1, i32::MAX, i32::MIN, -1] {
        let pc = p.c.allocate_results(count);
        let pr = p.rs.allocate_results(count);
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "allocate_results({count}) NULL-ness: C={} Rust={}",
            pc.is_null(),
            pr.is_null()
        );
    }
}
