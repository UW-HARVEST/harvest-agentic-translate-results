// Phase B — CONFIGS.md rows C10..C14
//
// The dispatch layer: `get_operation` (opcode -> function pointer) and
// `execute_operation` (call a function pointer, logging three lines to stdout).
// Both are driven directly, not just through `checkshift`.

mod common;
use common::*;

use std::ffi::CString;

/// Fixed seed for reproducibility.
const S: u64 = 0x0D15_1A7C_4000_0001;

/// Compare two `operation_func` values behaviourally across `n` random pairs.
fn kernels_behave_identically(
    label: &str,
    cf: OperationFunc,
    rf: OperationFunc,
    n: usize,
    seed: u64,
) {
    let mut rng = Rng::new(seed);
    for i in 0..n {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        assert_eq!(
            unsafe { cf(a, b) },
            unsafe { rf(a, b) },
            "{label}: iteration {i}, a={a} (0x{a:08X}), b={b} (0x{b:08X})"
        );
    }
}

// ---------------------------------------------------------------------------
// C10 — get_operation for every valid opcode 0..3
// ---------------------------------------------------------------------------

#[test]
fn c10_get_operation_valid_opcodes() {
    let (c, r) = libs();
    for opcode in 0..4i32 {
        let cf = unsafe { (c.get_operation)(opcode) };
        let rf = unsafe { (r.get_operation)(opcode) };

        assert!(cf.is_some(), "C get_operation({opcode}) must be non-NULL");
        assert_eq!(
            cf.is_some(),
            rf.is_some(),
            "get_operation({opcode}): NULL-ness must agree (C={}, Rust={})",
            cf.is_some(),
            rf.is_some()
        );

        // The returned pointer must be the kernel the C source maps that opcode
        // to: ops[] = {multiply, add, xor, shift}. Verified behaviourally in
        // BOTH libraries (addresses necessarily differ between the two .so's).
        let expect_c = c.kernel(opcode as usize);
        let expect_r = r.kernel(opcode as usize);
        kernels_behave_identically(
            &format!("C10 get_operation({opcode}) vs C's own kernel"),
            cf.unwrap(),
            expect_c,
            200,
            S ^ opcode as u64,
        );
        kernels_behave_identically(
            &format!("C10 get_operation({opcode}) vs Rust's own kernel"),
            rf.unwrap(),
            expect_r,
            200,
            S ^ opcode as u64,
        );
        // And cross-library: C's dispatched kernel == Rust's dispatched kernel.
        kernels_behave_identically(
            &format!("C10 get_operation({opcode}) C vs Rust"),
            cf.unwrap(),
            rf.unwrap(),
            500,
            S ^ (0x100 + opcode as u64),
        );
    }
}

#[test]
fn c10_get_operation_is_silent() {
    let (c, r) = libs();
    for opcode in -2..6i32 {
        let (cv, co) = capture(|| unsafe { (c.get_operation)(opcode) });
        let (rv, ro) = capture(|| unsafe { (r.get_operation)(opcode) });
        assert_eq!(
            cv.is_some(),
            rv.is_some(),
            "get_operation({opcode}) NULL-ness"
        );
        assert!(co.is_empty(), "C get_operation({opcode}) printed {:?}", show(&co));
        assert_stdout_eq(&format!("get_operation({opcode}) silence"), &co, &ro);
    }
}

// ---------------------------------------------------------------------------
// C11 — repeated calls: exercises the C lazy `static ops[4]` init-on-first-use
//        branch (first call initialises, subsequent calls take the other path)
// ---------------------------------------------------------------------------

#[test]
fn c11_get_operation_repeated_calls_lazy_init() {
    let (c, r) = libs();
    // 50 rounds over every opcode, including invalid ones interleaved, so that
    // the "already initialised" branch is taken many times.
    for round in 0..50 {
        for opcode in -1..5i32 {
            let cf = unsafe { (c.get_operation)(opcode) };
            let rf = unsafe { (r.get_operation)(opcode) };
            assert_eq!(
                cf.is_some(),
                rf.is_some(),
                "C11 round {round}, get_operation({opcode}) NULL-ness"
            );
            if let (Some(cfn), Some(rfn)) = (cf, rf) {
                // Same mapping on every repeat call.
                assert_eq!(
                    unsafe { cfn(round, opcode) },
                    unsafe { rfn(round, opcode) },
                    "C11 round {round}, opcode {opcode}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — execute_operation with each opcode, comparing return value AND the
//        three printf lines byte-for-byte
// ---------------------------------------------------------------------------

#[test]
fn c12_execute_operation_all_opcodes_random() {
    let (c, r) = libs();
    let name = CString::new("XOR").unwrap();
    let mut rng = Rng::new(S ^ 0xC12);

    for opcode in 0..4i32 {
        let cf = unsafe { (c.get_operation)(opcode) };
        let rf = unsafe { (r.get_operation)(opcode) };
        for i in 0..300 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();
            let (cv, co) = capture(|| unsafe { (c.execute_operation)(cf, a, b, name.as_ptr()) });
            let (rv, ro) = capture(|| unsafe { (r.execute_operation)(rf, a, b, name.as_ptr()) });
            assert_eq!(
                cv, rv,
                "C12 execute_operation(op={opcode}) iter {i}: a={a}, b={b}"
            );
            assert_stdout_eq(
                &format!("C12 execute_operation(op={opcode}) iter {i}: a={a}, b={b}"),
                &co,
                &ro,
            );
            // Sanity: the transcript really is the three documented lines.
            let text = String::from_utf8_lossy(&co);
            assert!(
                text.starts_with("Variable a = ") && text.contains("\nVariable b = ")
                    && text.contains("\nResult of XOR: "),
                "C12 unexpected transcript shape: {:?}",
                text
            );
        }
    }
}

#[test]
fn c12_execute_operation_boundary_values() {
    let (c, r) = libs();
    let name = CString::new("SHIFT").unwrap();
    for opcode in 0..4i32 {
        let cf = unsafe { (c.get_operation)(opcode) };
        let rf = unsafe { (r.get_operation)(opcode) };
        for &a in INTERESTING {
            for &b in [0i32, 1, -1, i32::MAX, i32::MIN].iter() {
                let (cv, co) =
                    capture(|| unsafe { (c.execute_operation)(cf, a, b, name.as_ptr()) });
                let (rv, ro) =
                    capture(|| unsafe { (r.execute_operation)(rf, a, b, name.as_ptr()) });
                assert_eq!(cv, rv, "C12b op={opcode} a={a} b={b}");
                assert_stdout_eq(&format!("C12b op={opcode} a={a} b={b}"), &co, &ro);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C13 — cross-library function pointers (axis A7)
//
// Hand a function pointer obtained from ONE library to the OTHER library's
// execute_operation. Both directions must produce identical results and output.
// ---------------------------------------------------------------------------

#[test]
fn c13_execute_operation_cross_library_function_pointers() {
    let (c, r) = libs();
    let name = CString::new("CROSS").unwrap();
    let mut rng = Rng::new(S ^ 0xC13);

    for opcode in 0..4i32 {
        let cf = unsafe { (c.get_operation)(opcode) };
        let rf = unsafe { (r.get_operation)(opcode) };

        for i in 0..150 {
            let a = rng.interesting_i32();
            let b = rng.interesting_i32();

            // Rust's execute_operation driving the C kernel ...
            let (v1, o1) = capture(|| unsafe { (r.execute_operation)(cf, a, b, name.as_ptr()) });
            // ... versus C's execute_operation driving the C kernel.
            let (v2, o2) = capture(|| unsafe { (c.execute_operation)(cf, a, b, name.as_ptr()) });
            assert_eq!(v1, v2, "C13 Rust-exec/C-kernel op={opcode} i={i} a={a} b={b}");
            assert_stdout_eq(
                &format!("C13 Rust-exec/C-kernel op={opcode} i={i}"),
                &o2,
                &o1,
            );

            // C's execute_operation driving the Rust kernel ...
            let (v3, o3) = capture(|| unsafe { (c.execute_operation)(rf, a, b, name.as_ptr()) });
            // ... versus Rust's execute_operation driving the Rust kernel.
            let (v4, o4) = capture(|| unsafe { (r.execute_operation)(rf, a, b, name.as_ptr()) });
            assert_eq!(v3, v4, "C13 C-exec/Rust-kernel op={opcode} i={i} a={a} b={b}");
            assert_stdout_eq(&format!("C13 C-exec/Rust-kernel op={opcode} i={i}"), &o3, &o4);

            // All four combinations must agree with each other.
            assert_eq!(v1, v3, "C13 all-combinations op={opcode} i={i}");
            assert_stdout_eq(&format!("C13 all-combinations op={opcode} i={i}"), &o1, &o3);
        }
    }
}

// ---------------------------------------------------------------------------
// C14 — op_name shapes (axis A6): the name is echoed through `%s`
// ---------------------------------------------------------------------------

#[test]
fn c14_execute_operation_op_name_shapes() {
    let (c, r) = libs();
    let long = "N".repeat(200);
    let names: Vec<CString> = [
        "XOR",
        "SHIFT",
        "",
        "a",
        "with space",
        "%d",
        "%s",
        "%%",
        "%n",
        "%99999999d",
        "tab\there",
        "newline\nhere",
        "non-ascii: \u{00e9}\u{4e2d}",
        long.as_str(),
    ]
    .iter()
    .map(|s| CString::new(*s).unwrap())
    .collect();

    for name in &names {
        for opcode in 0..4i32 {
            let cf = unsafe { (c.get_operation)(opcode) };
            let rf = unsafe { (r.get_operation)(opcode) };
            let (a, b) = (0x1234_5678i32, -0x0765_4321i32);

            let (cv, co) = capture(|| unsafe { (c.execute_operation)(cf, a, b, name.as_ptr()) });
            let (rv, ro) = capture(|| unsafe { (r.execute_operation)(rf, a, b, name.as_ptr()) });
            assert_eq!(
                cv, rv,
                "C14 op={opcode} name={:?} return value",
                name.to_string_lossy()
            );
            assert_stdout_eq(
                &format!("C14 op={opcode} name={:?}", name.to_string_lossy()),
                &co,
                &ro,
            );
        }
    }
}
