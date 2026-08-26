// Phase C — error-path differential tests for `mathop` (ERRORS.md E15 .. E19
// and G6).
//
// `mathop` mutates `static` state inside each library, so all of these rows run
// from a SINGLE `#[test]` in this dedicated test binary: that way the process
// starts with both libraries' statics fresh and the two are driven in lockstep.

mod common;

use common::*;
use std::ffi::c_int;

/// An independent oracle: the C algorithm re-derived from `lib.c`, used to prove
/// that the *specific* fallback each row exercises really is what happens (and,
/// for E15, that the `validation_char = '1'` store is dead).
fn oracle(p1: c_int, p2: c_int, p3: c_int, p4: c_int, ts: i64) -> (c_int, c_int) {
    fn apply(op: c_int, a: c_int, b: c_int) -> c_int {
        match op {
            2 => a.wrapping_mul(b),
            3 => a.wrapping_sub(b),
            4 => {
                if b == 0 {
                    0
                } else {
                    a.wrapping_div(b)
                }
            }
            5 => {
                if b == 0 {
                    0
                } else {
                    a.wrapping_rem(b)
                }
            }
            _ => a.wrapping_add(b), // OP_ADD (1) and every `default:` value
        }
    }
    let op1 = p3.wrapping_rem(5).wrapping_add(1);
    let priority = op1.wrapping_mul(10);
    let intermediate = apply(op1, p1, p2);
    let op2 = p4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);
    let mut result = apply(op2, intermediate, p4);
    result = result.wrapping_add(priority);
    result = result.wrapping_add((ts % 100) as c_int);
    (result, priority)
}

struct Call {
    ret: c_int,
    timestamp: i64,
    priority: i64,
    entries: i64,
}

/// One lockstep call pair; also checks both against the independent oracle.
fn diff(p1: c_int, p2: c_int, p3: c_int, p4: c_int, ctx: &str) -> Call {
    let (c, r) = both();
    assert!(
        !mathop_is_ub(p1, p2, p3, p4),
        "{ctx}: mathop({p1},{p2},{p3},{p4}) reaches INT_MIN/-1 division UB (E21)"
    );
    let _g = serial();

    let mut cv: c_int = 0;
    let cout = capture_stdout(|| cv = unsafe { (c.mathop)(p1, p2, p3, p4) });
    let mut rv: c_int = 0;
    let rout = capture_stdout(|| rv = unsafe { (r.mathop)(p1, p2, p3, p4) });

    assert_eq!(cv, rv, "{ctx}: mathop({p1},{p2},{p3},{p4}) return");
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "{ctx}: mathop({p1},{p2},{p3},{p4}) stdout"
    );

    let parse = |label: &str| -> i64 {
        let text = String::from_utf8_lossy(&cout).to_string();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix(label) {
                return rest.trim().parse::<i64>().expect("number");
            }
        }
        panic!("{label:?} missing from {text:?}");
    };
    let call = Call {
        ret: cv,
        timestamp: parse("Computation performed at timestamp: "),
        priority: parse("Operation priority: "),
        entries: parse("History entries: "),
    };
    let (expect, expect_prio) = oracle(p1, p2, p3, p4, call.timestamp);
    assert_eq!(
        cv, expect,
        "{ctx}: mathop({p1},{p2},{p3},{p4}) disagrees with the C-derived oracle"
    );
    assert_eq!(call.priority, expect_prio as i64, "{ctx}: priority");
    assert_eq!(
        parse("Final result: "),
        cv as i64,
        "{ctx}: printed result must equal the returned one"
    );
    call
}

#[test]
fn phase_c_mathop_error_rows() {
    // ---- E19 / C32 first: fresh statics, so saturation is observable -------
    let mut expected = 0i64;
    for i in 0..8 {
        let call = diff(49, 2, i, i, &format!("E19 call={i}"));
        expected = (expected + 2).min(10);
        assert_eq!(
            call.entries, expected,
            "E19: history entries must saturate at 10 and never exceed it"
        );
    }
    // Well past saturation the count must stay pinned at 10.
    for i in 0..4 {
        let call = diff(50, 3, i + 1, i + 2, &format!("E19 extra={i}"));
        assert_eq!(call.entries, 10, "E19: silently dropped past the limit");
    }

    // ---- E15 / G6: invalid validation char is a dead store -----------------
    // Every one of these has `(char)(param1 % 128)` outside '1'..'5', so the
    // `validation_char = '1'` assignment fires; the oracle (which does not model
    // it at all) still predicts the result exactly => the store is dead.
    for &p1 in &[
        0i32,
        128,
        -128,
        256,
        48,
        54,
        -49,
        -1,
        1,
        127,
        i32::MIN,   // INT_MIN % 128 == 0  -> is_valid_operation(0) is false (E1)
        i32::MAX,   // INT_MAX % 128 == 127 -> above '5' (E3)
    ] {
        for &p3 in &[0i32, 1, 2, 3, 4, -1, -2] {
            diff(p1, 6, p3, 4, &format!("E15 p1={p1} p3={p3}"));
        }
    }
    // Contrast: an accepted validation char changes nothing either.
    for &p1 in &[49i32, 50, 51, 52, 53, 177, 181] {
        diff(p1, 6, 0, 4, &format!("E15 valid p1={p1}"));
    }

    // ---- E16: selected_op == 0 (param3 % 5 == -1) --------------------------
    for &p3 in &[-1i32, -6, -11, -101, -2147483646] {
        let call = diff(50, 7, p3, 4, &format!("E16 p3={p3}"));
        assert_eq!(
            call.priority, 0,
            "E16 p3={p3}: get_operation_priority(0) == 0"
        );
        // The default arm was used, i.e. the intermediate is p1 + p2.
        let (expect, _) = oracle(50, 7, p3, 4, call.timestamp);
        assert_eq!(call.ret, expect);
    }

    // ---- E17: negative selected_op -> negative priority --------------------
    for &(p3, op) in &[
        (-2i32, -1i32),
        (-3, -2),
        (-4, -3),
        (-7, -1),
        (-8, -2),
        (-9, -3),
        (i32::MIN, -2),
    ] {
        let call = diff(51, 9, p3, 4, &format!("E17 p3={p3}"));
        assert_eq!(
            call.priority,
            (op as i64) * 10,
            "E17 p3={p3}: negative priority must be added, not clamped"
        );
        assert!(call.priority < 0);
    }

    // ---- E18: param4 == INT_MAX makes `param4 + 1` overflow ----------------
    for &p3 in &[0i32, 1, 2, 3, 4, -1, -3] {
        diff(52, 5, p3, i32::MAX, &format!("E18 p3={p3} p4=INT_MAX"));
    }
    diff(52, 5, 0, i32::MAX - 1, "E18 p4=INT_MAX-1");
    diff(52, 5, 0, i32::MIN, "E18 p4=INT_MIN");
    diff(52, 5, 0, i32::MIN + 1, "E18 p4=INT_MIN+1");

    // ---- Out-of-range enum values reached through mathop (G1) -------------
    // `selected_op` and `second_op` are cast from ints, so they can be any of
    // -3..5; each must take `select_operation`'s `default:` arm identically.
    for p3 in -12..=12 {
        for p4 in -12..=12 {
            if mathop_is_ub(53, 11, p3, p4) {
                continue;
            }
            diff(53, 11, p3, p4, &format!("G1 p3={p3} p4={p4}"));
        }
    }
}
