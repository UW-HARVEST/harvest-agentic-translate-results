// Phase C — one differential test per ERRORS.md row (E1..E16) plus the generic
// FFI boundary cases (G1..G7).
//
// Each test constructs the exact invalid input/condition, calls BOTH libraries,
// and asserts they reject it the SAME way: the same sentinel return value AND
// the same diagnostic bytes on stdout -- not merely "both failed somehow".

mod common;
use common::*;

use std::ffi::CString;

const S: u64 = 0xE770_0000_5555_AAAA;

// ===========================================================================
// E1 — get_operation(opcode < 0) -> NULL
// ===========================================================================

#[test]
fn err_e1_get_operation_negative_opcode() {
    let (c, r) = libs();
    let mut cases: Vec<i32> = vec![-1, -2, -3, -4, -5, -100, -0xABCD, i32::MIN, i32::MIN + 1];
    let mut rng = Rng::new(S ^ 1);
    for _ in 0..500 {
        // Any negative value.
        let v = rng.i32();
        cases.push(if v == i32::MIN { i32::MIN } else { -(v.abs()) - 1 });
    }

    for &opcode in &cases {
        let (cv, co) = capture(|| unsafe { (c.get_operation)(opcode) });
        let (rv, ro) = capture(|| unsafe { (r.get_operation)(opcode) });
        assert!(
            cv.is_none(),
            "E1: C get_operation({opcode}) must return NULL"
        );
        assert!(
            rv.is_none(),
            "E1: Rust get_operation({opcode}) must return NULL (C returned NULL)"
        );
        assert_stdout_eq(&format!("E1 get_operation({opcode})"), &co, &ro);
        assert!(co.is_empty(), "E1: C printed {:?}", show(&co));
    }
}

// ===========================================================================
// E2 — get_operation(opcode >= 4) -> NULL
// ===========================================================================

#[test]
fn err_e2_get_operation_opcode_ge_4() {
    let (c, r) = libs();
    let mut cases: Vec<i32> = vec![4, 5, 6, 7, 8, 100, 0xABCD, i32::MAX, i32::MAX - 1];
    let mut rng = Rng::new(S ^ 2);
    for _ in 0..500 {
        cases.push(4i32.saturating_add((rng.next_u32() % (i32::MAX as u32 - 4)) as i32));
    }

    for &opcode in &cases {
        let (cv, co) = capture(|| unsafe { (c.get_operation)(opcode) });
        let (rv, ro) = capture(|| unsafe { (r.get_operation)(opcode) });
        assert!(cv.is_none(), "E2: C get_operation({opcode}) must return NULL");
        assert!(
            rv.is_none(),
            "E2: Rust get_operation({opcode}) must return NULL (C returned NULL)"
        );
        assert_stdout_eq(&format!("E2 get_operation({opcode})"), &co, &ro);
        assert!(co.is_empty(), "E2: C printed {:?}", show(&co));
    }
}

// ===========================================================================
// E3 — the OP_* macros are 1..4 but the accepted index range is 0..3, so
//      OP_SHIFT (== 4) is OUT OF RANGE. Out-of-range enum-like values crossing
//      the FFI boundary must be handled identically.
// ===========================================================================

#[test]
fn err_e3_get_operation_op_macro_values() {
    let (c, r) = libs();
    // #define OP_ADD 0x01 / OP_MULTIPLY 0x02 / OP_XOR 0x03 / OP_SHIFT 0x04
    const OP_ADD: i32 = 0x01;
    const OP_MULTIPLY: i32 = 0x02;
    const OP_XOR: i32 = 0x03;
    const OP_SHIFT: i32 = 0x04;

    for (name, opcode, expect_null) in [
        ("OP_ADD", OP_ADD, false),
        ("OP_MULTIPLY", OP_MULTIPLY, false),
        ("OP_XOR", OP_XOR, false),
        ("OP_SHIFT", OP_SHIFT, true), // 4 is past the end of ops[4]
    ] {
        let (cv, co) = capture(|| unsafe { (c.get_operation)(opcode) });
        let (rv, ro) = capture(|| unsafe { (r.get_operation)(opcode) });
        assert_eq!(
            cv.is_none(),
            expect_null,
            "E3: C get_operation({name} = {opcode}) NULL-ness"
        );
        assert_eq!(
            cv.is_none(),
            rv.is_none(),
            "E3: get_operation({name} = {opcode}) NULL-ness must agree"
        );
        assert_stdout_eq(&format!("E3 get_operation({name})"), &co, &ro);
    }
}

#[test]
fn err_e3b_get_operation_exhaustive_boundary_sweep() {
    let (c, r) = libs();
    // One step past the range on both sides, and a dense sweep around it.
    for opcode in -260..260i32 {
        let cv = unsafe { (c.get_operation)(opcode) };
        let rv = unsafe { (r.get_operation)(opcode) };
        assert_eq!(
            cv.is_none(),
            rv.is_none(),
            "E3b get_operation({opcode}) NULL-ness must agree"
        );
        let want_valid = (0..4).contains(&opcode);
        assert_eq!(
            cv.is_some(),
            want_valid,
            "E3b: C get_operation({opcode}) validity"
        );
    }
    // And a randomized sweep over the whole i32 domain.
    let mut rng = Rng::new(S ^ 3);
    for _ in 0..5000 {
        let opcode = rng.interesting_i32();
        let cv = unsafe { (c.get_operation)(opcode) };
        let rv = unsafe { (r.get_operation)(opcode) };
        assert_eq!(
            cv.is_none(),
            rv.is_none(),
            "E3b random get_operation({opcode}) NULL-ness"
        );
    }
}

// ===========================================================================
// E4 — execute_operation(NULL func, ..., valid name) -> prints error, returns 0
// ===========================================================================

#[test]
fn err_e4_execute_operation_null_func() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 4);

    for name_str in ["XOR", "SHIFT", "MyOperation", "x"] {
        let name = CString::new(name_str).unwrap();
        for i in 0..50 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let (cv, co) = capture(|| unsafe { (c.execute_operation)(None, a, b, name.as_ptr()) });
            let (rv, ro) = capture(|| unsafe { (r.execute_operation)(None, a, b, name.as_ptr()) });

            assert_eq!(cv, 0, "E4: C must return the 0 sentinel");
            assert_eq!(rv, 0, "E4: Rust must return the 0 sentinel");
            assert_stdout_eq(&format!("E4 name={name_str} iter {i}"), &co, &ro);
            assert_eq!(
                String::from_utf8_lossy(&co),
                format!("Error: Operation function pointer is NULL for {name_str}\n"),
                "E4: exact C diagnostic"
            );
            // The a/b LOG_VALUE lines must NOT be printed on this path.
            assert!(
                !String::from_utf8_lossy(&co).contains("Variable a"),
                "E4: must return before logging"
            );
        }
    }
}

// ===========================================================================
// E5 — execute_operation(NULL func, NULL op_name): %s with a NULL pointer
// ===========================================================================

#[test]
fn err_e5_execute_operation_null_func_null_name() {
    let (c, r) = libs();
    for (a, b) in [(0i32, 0i32), (1, -1), (i32::MAX, i32::MIN), (12345, -6789)] {
        let (cv, co) =
            capture(|| unsafe { (c.execute_operation)(None, a, b, std::ptr::null()) });
        let (rv, ro) =
            capture(|| unsafe { (r.execute_operation)(None, a, b, std::ptr::null()) });

        assert_eq!(cv, 0, "E5: C sentinel");
        assert_eq!(rv, 0, "E5: Rust sentinel");
        assert_eq!(cv, rv, "E5: return values");
        assert_stdout_eq(&format!("E5 a={a} b={b}"), &co, &ro);
        // glibc renders a NULL %s argument as "(null)".
        assert_eq!(
            String::from_utf8_lossy(&co),
            "Error: Operation function pointer is NULL for (null)\n",
            "E5: glibc NULL-%s rendering"
        );
    }
}

// ===========================================================================
// E6 — execute_operation(NULL func, "" op_name)
// ===========================================================================

#[test]
fn err_e6_execute_operation_null_func_empty_name() {
    let (c, r) = libs();
    let empty = CString::new("").unwrap();
    let (cv, co) = capture(|| unsafe { (c.execute_operation)(None, 7, 9, empty.as_ptr()) });
    let (rv, ro) = capture(|| unsafe { (r.execute_operation)(None, 7, 9, empty.as_ptr()) });

    assert_eq!(cv, 0, "E6: C sentinel");
    assert_eq!(rv, 0, "E6: Rust sentinel");
    assert_stdout_eq("E6 empty name", &co, &ro);
    assert_eq!(
        String::from_utf8_lossy(&co),
        "Error: Operation function pointer is NULL for \n",
        "E6: exact diagnostic with empty name"
    );
}

// ===========================================================================
// E7 — compute_checksum(NULL values, any count) -> 0
// ===========================================================================

#[test]
fn err_e7_compute_checksum_null_values() {
    let (c, r) = libs();
    for count in [1i32, 2, 3, 4, 5, 16, 1000, i32::MAX, 0, -1, i32::MIN] {
        let (cv, co) = capture(|| unsafe { (c.compute_checksum)(std::ptr::null_mut(), count) });
        let (rv, ro) = capture(|| unsafe { (r.compute_checksum)(std::ptr::null_mut(), count) });
        assert_eq!(cv, 0, "E7: C compute_checksum(NULL, {count}) must be 0");
        assert_eq!(rv, 0, "E7: Rust compute_checksum(NULL, {count}) must be 0");
        assert_stdout_eq(&format!("E7 count={count}"), &co, &ro);
        assert!(co.is_empty(), "E7: C printed {:?}", show(&co));
    }
}

// ===========================================================================
// E8 — compute_checksum(valid values, count == 0) -> 0
// ===========================================================================

#[test]
fn err_e8_compute_checksum_zero_count() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 8);
    for i in 0..200 {
        let values: Vec<i32> = (0..4).map(|_| rng.interesting_i32()).collect();
        let mut cvv = values.clone();
        let mut rvv = values.clone();
        let (cv, co) = capture(|| unsafe { (c.compute_checksum)(cvv.as_mut_ptr(), 0) });
        let (rv, ro) = capture(|| unsafe { (r.compute_checksum)(rvv.as_mut_ptr(), 0) });
        assert_eq!(cv, 0, "E8: C iter {i} must be 0 for count=0");
        assert_eq!(rv, 0, "E8: Rust iter {i} must be 0 for count=0");
        assert_stdout_eq(&format!("E8 iter {i}"), &co, &ro);
        assert_eq!(cvv, values, "E8: C must not touch the array");
        assert_eq!(rvv, values, "E8: Rust must not touch the array");
    }
}

// ===========================================================================
// E9 — compute_checksum(valid values, count < 0) -> 0
// ===========================================================================

#[test]
fn err_e9_compute_checksum_negative_count() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 9);
    let counts = [-1i32, -2, -3, -4, -5, -16, -1000, i32::MIN, i32::MIN + 1, -0xABCD];
    for &count in &counts {
        for i in 0..25 {
            let values: Vec<i32> = (0..4).map(|_| rng.interesting_i32()).collect();
            let mut cvv = values.clone();
            let mut rvv = values.clone();
            let (cv, co) = capture(|| unsafe { (c.compute_checksum)(cvv.as_mut_ptr(), count) });
            let (rv, ro) = capture(|| unsafe { (r.compute_checksum)(rvv.as_mut_ptr(), count) });
            assert_eq!(cv, 0, "E9: C count={count} iter {i} must be 0");
            assert_eq!(rv, 0, "E9: Rust count={count} iter {i} must be 0");
            assert_stdout_eq(&format!("E9 count={count} iter {i}"), &co, &ro);
        }
    }
}

// ===========================================================================
// E10 — compute_checksum(count > 4): oversized length clamped to 4
// ===========================================================================

#[test]
fn err_e10_compute_checksum_count_clamped_to_4() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 10);
    for i in 0..200 {
        // Exactly 4 valid elements; the clamp must stop the read at 4 even for
        // absurd counts, so this must not read out of bounds.
        let values: Vec<i32> = (0..4).map(|_| rng.interesting_i32()).collect();

        let mut base = values.clone();
        let expect_c = unsafe { (c.compute_checksum)(base.as_mut_ptr(), 4) };
        let mut base_r = values.clone();
        let expect_r = unsafe { (r.compute_checksum)(base_r.as_mut_ptr(), 4) };
        assert_eq!(expect_c, expect_r, "E10 iter {i}: count=4 baseline");

        for &count in &[5i32, 6, 8, 100, 65536, i32::MAX - 1, i32::MAX] {
            let mut cvv = values.clone();
            let mut rvv = values.clone();
            let (cv, co) = capture(|| unsafe { (c.compute_checksum)(cvv.as_mut_ptr(), count) });
            let (rv, ro) = capture(|| unsafe { (r.compute_checksum)(rvv.as_mut_ptr(), count) });
            assert_eq!(cv, rv, "E10 iter {i} count={count}: C vs Rust");
            assert_eq!(cv, expect_c, "E10 iter {i} count={count}: must equal count=4");
            assert_stdout_eq(&format!("E10 iter {i} count={count}"), &co, &ro);
        }
    }
}

// ===========================================================================
// E11 — compute_checksum(NULL, count <= 0): both guard operands false
// ===========================================================================

#[test]
fn err_e11_compute_checksum_null_and_nonpositive() {
    let (c, r) = libs();
    for count in [0i32, -1, -2, -1000, i32::MIN] {
        let (cv, co) = capture(|| unsafe { (c.compute_checksum)(std::ptr::null_mut(), count) });
        let (rv, ro) = capture(|| unsafe { (r.compute_checksum)(std::ptr::null_mut(), count) });
        assert_eq!(cv, 0, "E11: C compute_checksum(NULL, {count})");
        assert_eq!(rv, 0, "E11: Rust compute_checksum(NULL, {count})");
        assert_stdout_eq(&format!("E11 count={count}"), &co, &ro);
        assert!(co.is_empty(), "E11: C printed {:?}", show(&co));
    }
}

// ===========================================================================
// E12 — init_state(NULL, v) -> prints error, returns without writing
// ===========================================================================

#[test]
fn err_e12_init_state_null_state() {
    let (c, r) = libs();
    for v in [0i32, 1, -1, i32::MAX, i32::MIN, 12345] {
        let (_, co) = capture(|| unsafe { (c.init_state)(std::ptr::null_mut(), v) });
        let (_, ro) = capture(|| unsafe { (r.init_state)(std::ptr::null_mut(), v) });
        assert_stdout_eq(&format!("E12 init_state(NULL, {v})"), &co, &ro);
        assert_eq!(
            String::from_utf8_lossy(&co),
            "Error: state pointer is NULL in init_state\n",
            "E12: exact C diagnostic"
        );
        // The success message must NOT appear.
        assert!(
            !String::from_utf8_lossy(&co).contains("State initialized"),
            "E12: must return before the success message"
        );
    }
}

// ===========================================================================
// E13 — apply_operation(NULL state, v, valid func): func must NOT be called
// ===========================================================================

#[test]
fn err_e13_apply_operation_null_state() {
    let (c, r) = libs();
    for opcode in 0..4i32 {
        let cf = unsafe { (c.get_operation)(opcode) };
        let rf = unsafe { (r.get_operation)(opcode) };
        for v in [0i32, 1, -1, i32::MAX, i32::MIN] {
            let (_, co) =
                capture(|| unsafe { (c.apply_operation)(std::ptr::null_mut(), v, cf) });
            let (_, ro) =
                capture(|| unsafe { (r.apply_operation)(std::ptr::null_mut(), v, rf) });
            assert_stdout_eq(&format!("E13 op={opcode} v={v}"), &co, &ro);
            assert_eq!(
                String::from_utf8_lossy(&co),
                "Error: state pointer is NULL in apply_operation\n",
                "E13: exact C diagnostic"
            );
        }
    }
}

// ===========================================================================
// E14 — apply_operation(valid state, v, NULL func): state left UNMODIFIED
// ===========================================================================

#[test]
fn err_e14_apply_operation_null_func() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 14);

    for i in 0..200 {
        let seed_acc = rng.interesting_i32();
        let v = rng.interesting_i32();

        let mut cb = StateBuf::new();
        let mut rb = StateBuf::new();
        let _ = capture(|| unsafe { (c.init_state)(cb.as_ptr(), seed_acc) });
        let _ = capture(|| unsafe { (r.init_state)(rb.as_ptr(), seed_acc) });
        let before_c = cb.state();
        let before_r = rb.state();

        let (_, co) = capture(|| unsafe { (c.apply_operation)(cb.as_ptr(), v, None) });
        let (_, ro) = capture(|| unsafe { (r.apply_operation)(rb.as_ptr(), v, None) });

        assert_stdout_eq(&format!("E14 iter {i}"), &co, &ro);
        assert_eq!(
            String::from_utf8_lossy(&co),
            "Error: operation function pointer is NULL in apply_operation\n",
            "E14: exact C diagnostic"
        );
        // Crucially: operation_count must NOT be incremented.
        assert_eq!(cb.state(), before_c, "E14: C must not modify the state");
        assert_eq!(rb.state(), before_r, "E14: Rust must not modify the state");
        assert_eq!(cb.bytes(), rb.bytes(), "E14: state bytes must match");
        assert_eq!(cb.state().operation_count, 0, "E14: operation_count unchanged");
    }
}

// ===========================================================================
// E15 — apply_operation(NULL state, v, NULL func): CHECK ORDER
//        `state` is tested before `func`, so only the state message is printed.
// ===========================================================================

#[test]
fn err_e15_apply_operation_both_null_order() {
    let (c, r) = libs();
    for v in [0i32, 1, -1, i32::MAX, i32::MIN, 999] {
        let (_, co) =
            capture(|| unsafe { (c.apply_operation)(std::ptr::null_mut(), v, None) });
        let (_, ro) =
            capture(|| unsafe { (r.apply_operation)(std::ptr::null_mut(), v, None) });
        assert_stdout_eq(&format!("E15 v={v}"), &co, &ro);
        let text = String::from_utf8_lossy(&co);
        assert_eq!(
            text, "Error: state pointer is NULL in apply_operation\n",
            "E15: only the STATE diagnostic (state is checked first)"
        );
        assert!(
            !text.contains("operation function pointer is NULL"),
            "E15: the func diagnostic must NOT be reached"
        );
    }
}

// ===========================================================================
// E16 — checkshift's malloc-failure guard (`return -1`)
//
// A 12-byte allocation does not fail in practice, so this path is not reachable
// by passing arguments. It IS covered by a dedicated out-of-process test using a
// malloc interposer -- see tests/phase_c_malloc_failure.rs. Here we assert the
// complementary invariant: on the SUCCESS path neither library ever produces the
// -1 sentinel spuriously, and the failure diagnostic never appears.
// ===========================================================================

#[test]
fn err_e16_checkshift_never_reports_alloc_failure_on_success_path() {
    let (c, r) = libs();
    let mut rng = Rng::new(S ^ 16);
    for i in 0..500 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let (cv, co) = capture(|| unsafe { (c.checkshift)(p[0], p[1], p[2], p[3]) });
        let (rv, ro) = capture(|| unsafe { (r.checkshift)(p[0], p[1], p[2], p[3]) });
        assert_eq!(cv, rv, "E16 iter {i}: checkshift{p:?}");
        assert_stdout_eq(&format!("E16 iter {i}"), &co, &ro);
        for (who, out) in [("C", &co), ("Rust", &ro)] {
            let text = String::from_utf8_lossy(out);
            assert!(
                !text.contains("Failed to allocate memory"),
                "E16: {who} reported an allocation failure on the success path"
            );
            assert!(
                text.contains("=== Ending foo function ===") ,
                "E16: {who} must run to completion"
            );
        }
    }
}

// ===========================================================================
// G — generic FFI boundary cases beyond the table
// ===========================================================================

#[test]
fn err_g_all_null_pointer_combinations() {
    let (c, r) = libs();
    let name = CString::new("NAME").unwrap();
    let valid_c = unsafe { (c.get_operation)(0) };
    let valid_r = unsafe { (r.get_operation)(0) };

    // execute_operation: {NULL, valid} func x {NULL, valid, empty} name
    let empty = CString::new("").unwrap();
    for (fi, (cf, rf)) in [(None, None), (valid_c, valid_r)].into_iter().enumerate() {
        for (ni, nptr) in [std::ptr::null(), name.as_ptr(), empty.as_ptr()]
            .into_iter()
            .enumerate()
        {
            let (cv, co) = capture(|| unsafe { (c.execute_operation)(cf, 5, 6, nptr) });
            let (rv, ro) = capture(|| unsafe { (r.execute_operation)(rf, 5, 6, nptr) });
            assert_eq!(cv, rv, "G: execute_operation func#{fi} name#{ni}");
            assert_stdout_eq(&format!("G: execute_operation func#{fi} name#{ni}"), &co, &ro);
        }
    }

    // apply_operation: {NULL, valid} state x {NULL, valid} func
    for (si, use_null_state) in [true, false].into_iter().enumerate() {
        for (fi, (cf, rf)) in [(None, None), (valid_c, valid_r)].into_iter().enumerate() {
            let mut cb = StateBuf::new();
            let mut rb = StateBuf::new();
            let _ = capture(|| unsafe { (c.init_state)(cb.as_ptr(), 42) });
            let _ = capture(|| unsafe { (r.init_state)(rb.as_ptr(), 42) });
            let (cp, rp) = if use_null_state {
                (std::ptr::null_mut(), std::ptr::null_mut())
            } else {
                (cb.as_ptr(), rb.as_ptr())
            };
            let (_, co) = capture(|| unsafe { (c.apply_operation)(cp, 7, cf) });
            let (_, ro) = capture(|| unsafe { (r.apply_operation)(rp, 7, rf) });
            assert_stdout_eq(&format!("G: apply_operation state#{si} func#{fi}"), &co, &ro);
            assert_eq!(
                cb.bytes(),
                rb.bytes(),
                "G: apply_operation state#{si} func#{fi} bytes"
            );
        }
    }

    // init_state / compute_checksum with NULL.
    let (_, co) = capture(|| unsafe { (c.init_state)(std::ptr::null_mut(), 1) });
    let (_, ro) = capture(|| unsafe { (r.init_state)(std::ptr::null_mut(), 1) });
    assert_stdout_eq("G: init_state(NULL)", &co, &ro);

    let (cv, _) = capture(|| unsafe { (c.compute_checksum)(std::ptr::null_mut(), 4) });
    let (rv, _) = capture(|| unsafe { (r.compute_checksum)(std::ptr::null_mut(), 4) });
    assert_eq!(cv, rv, "G: compute_checksum(NULL, 4)");
}

#[test]
fn err_g_extreme_integers_every_entry_point() {
    let (c, r) = libs();
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];

    for &a in &extremes {
        for &b in &extremes {
            for k in 0..4usize {
                assert_eq!(
                    unsafe { (c.kernel(k))(a, b) },
                    unsafe { (r.kernel(k))(a, b) },
                    "G: kernel {k} with extremes a={a}, b={b}"
                );
            }
            // checkshift with extreme params in every position.
            let (cv, co) = capture(|| unsafe { (c.checkshift)(a, b, a, b) });
            let (rv, ro) = capture(|| unsafe { (r.checkshift)(a, b, a, b) });
            assert_eq!(cv, rv, "G: checkshift({a},{b},{a},{b})");
            assert_stdout_eq(&format!("G: checkshift({a},{b},{a},{b})"), &co, &ro);
        }
    }
}
