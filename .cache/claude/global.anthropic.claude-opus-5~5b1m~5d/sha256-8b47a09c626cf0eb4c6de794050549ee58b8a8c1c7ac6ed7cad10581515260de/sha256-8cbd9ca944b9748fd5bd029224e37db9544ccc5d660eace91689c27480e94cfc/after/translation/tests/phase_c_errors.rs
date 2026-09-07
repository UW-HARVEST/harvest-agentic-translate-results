// Phase C — error-path differential tests.
// One test per row of ERRORS.md, plus the generic FFI boundaries.

mod common;

use common::*;
use std::ffi::CString;
use std::ffi::c_char;

// ---------------------------------------------------------------- ERRORS row 1
#[test]
fn err_row1_validation_branch_unreachable() {
    // C: if (strncmp("VALID", "VALID", strlen("VALID")) != 0) { print; goto cleanup; }
    // Both operands are the same literal, so the branch can never be taken for
    // ANY input. Assert that neither library ever emits the failure message and
    // that neither ever short-circuits to a bare 0 when the sum is non-zero.
    let l = libs();
    let cf = l.c_cleanup();
    let rf = l.rust_cleanup();
    let probes: [[i32; 4]; 8] = [
        [0, 0, 0, 0],
        [1, 2, 3, 4],
        [10, 20, 30, 40],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [-1, -1, -1, -1],
        [9, 41, -30, 7],
        [40, 40, 40, 40],
    ];
    for t in probes {
        let mut c_ret = 0;
        let c_out = capture_stdout(|| c_ret = unsafe { cf(t[0], t[1], t[2], t[3]) });
        let mut r_ret = 0;
        let r_out = capture_stdout(|| r_ret = unsafe { rf(t[0], t[1], t[2], t[3]) });

        assert_eq!(c_ret, r_ret, "return mismatch for {t:?}");
        assert_eq!(c_out, r_out, "stdout mismatch for {t:?}");

        let text = String::from_utf8_lossy(&c_out);
        assert!(
            !text.contains("Input string validation failed"),
            "C unexpectedly took the validation-failure branch for {t:?}"
        );
        assert!(
            !String::from_utf8_lossy(&r_out).contains("Input string validation failed"),
            "Rust took the validation-failure branch for {t:?} but C did not"
        );
        // Reaching the tail means the success message was printed.
        assert_eq!(
            c_out.as_slice(),
            b"Processed numbers: numbers\n",
            "C did not reach the tail for {t:?}"
        );
        assert_eq!(c_ret, model_cleanup(t[0], t[1], t[2], t[3]));
    }
}

// ---------------------------------------------------------------- ERRORS row 2
#[test]
fn err_row2_malloc_branch_parity() {
    // C: dynamic_str = malloc(50); if (!dynamic_str) { print; goto cleanup; }
    // A 50-byte allocation cannot be forced to fail through the FFI boundary,
    // so verify the observable contract: on a healthy allocator BOTH libraries
    // take the success branch (identical message, identical return), and the
    // return value on that branch is the accumulated result — the C code has no
    // distinct error sentinel, so the error branch would return the same
    // accumulated value, which we assert equals the model in both cases.
    let l = libs();
    let cf = l.c_cleanup();
    let rf = l.rust_cleanup();
    let mut rng = Rng::new(0xE22_0002);
    for _ in 0..256 {
        let t = [
            rng.next_i32(),
            rng.range_i32(-50, 50),
            rng.pick(&[10, 20, 30, 40, 0]),
            rng.next_i32(),
        ];
        let mut c_ret = 0;
        let c_out = capture_stdout(|| c_ret = unsafe { cf(t[0], t[1], t[2], t[3]) });
        let mut r_ret = 0;
        let r_out = capture_stdout(|| r_ret = unsafe { rf(t[0], t[1], t[2], t[3]) });
        assert_eq!(c_ret, r_ret, "return mismatch for {t:?}");
        assert_eq!(c_out, r_out, "stdout mismatch for {t:?}");
        assert!(
            !String::from_utf8_lossy(&c_out).contains("Memory allocation failed"),
            "C hit the malloc-failure branch unexpectedly"
        );
        assert!(
            !String::from_utf8_lossy(&r_out).contains("Memory allocation failed"),
            "Rust hit the malloc-failure branch but C did not"
        );
        // Whichever branch is taken, the return value is the accumulated result.
        assert_eq!(c_ret, model_cleanup(t[0], t[1], t[2], t[3]));
    }
}

// ---------------------------------------------------------------- ERRORS row 3
#[test]
fn err_row3_cleanup_resources_null() {
    let l = libs();
    let cf = l.c_cleanup_resources();
    let rf = l.rust_cleanup_resources();
    for _ in 0..64 {
        let c_out = capture_stdout(|| unsafe { cf(std::ptr::null_mut()) });
        let r_out = capture_stdout(|| unsafe { rf(std::ptr::null_mut()) });
        assert!(c_out.is_empty(), "C printed something on NULL");
        assert!(r_out.is_empty(), "Rust printed something on NULL");
        assert_eq!(c_out, r_out);
    }
}

// ---------------------------------------------------------------- ERRORS row 4
#[test]
fn err_row4_cleanup_resources_frees() {
    // Free a live libc block through each .so; then immediately allocate the
    // same size again. If either implementation failed to free (or corrupted the
    // heap) this loop would leak unboundedly or abort.
    let l = libs();
    let cf = l.c_cleanup_resources();
    let rf = l.rust_cleanup_resources();
    for size in [1usize, 8, 50, 1024] {
        for _ in 0..128 {
            let p = unsafe { malloc(size) } as *mut c_char;
            assert!(!p.is_null());
            unsafe { cf(p) };
            let q = unsafe { malloc(size) } as *mut c_char;
            assert!(!q.is_null());
            unsafe { rf(q) };
        }
    }
}

// ---------------------------------------------------------------- ERRORS row 5
#[test]
fn err_row5_print_result_null_label() {
    // glibc printf renders a NULL %s as "(null)". Both libraries forward the
    // same NULL to the same libc printf, so the bytes must agree exactly.
    let l = libs();
    let cf = l.c_print_result();
    let rf = l.rust_print_result();
    for &r in &[0i32, 42, -7, i32::MIN, i32::MAX] {
        let c_out = capture_stdout(|| unsafe { cf(std::ptr::null(), r) });
        let r_out = capture_stdout(|| unsafe { rf(std::ptr::null(), r) });
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
            "NULL-label stdout mismatch for result {r}"
        );
        assert_eq!(c_out, r_out, "NULL-label byte mismatch for result {r}");
        assert!(!c_out.is_empty(), "C produced no output for NULL label");
    }
}

// ---------------------------------------------------------------- ERRORS row 6
#[test]
fn err_row6_print_result_empty_label() {
    let cs = CString::new("").unwrap();
    let l = libs();
    let cf = l.c_print_result();
    let rf = l.rust_print_result();
    for &r in &[0i32, -1, i32::MIN, i32::MAX] {
        let c_out = capture_stdout(|| unsafe { cf(cs.as_ptr(), r) });
        let r_out = capture_stdout(|| unsafe { rf(cs.as_ptr(), r) });
        assert_eq!(c_out, r_out, "empty-label mismatch for {r}");
        assert_eq!(c_out, format!(": {r}\n").into_bytes());
    }
}

// ---------------------------------------------------------------- ERRORS row 7
#[test]
fn err_row7_print_result_long_label() {
    let l = libs();
    let cf = l.c_print_result();
    let rf = l.rust_print_result();
    for len in [255usize, 256, 1023, 1024, 4095, 65536] {
        let lab = "Z".repeat(len);
        let cs = CString::new(lab.clone()).unwrap();
        let c_out = capture_stdout(|| unsafe { cf(cs.as_ptr(), 1) });
        let r_out = capture_stdout(|| unsafe { rf(cs.as_ptr(), 1) });
        assert_eq!(c_out, r_out, "long-label ({len}) mismatch");
        assert_eq!(
            c_out,
            format!("{lab}: 1\n").into_bytes(),
            "long label ({len}) truncated"
        );
    }
}

// ---------------------------------------------------------------- G1
#[test]
fn bound_int_extremes() {
    // Signed overflow in `result += numbers[i]`: must wrap identically.
    const EXT: [i32; 8] = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -1,
        0,
        1,
        i32::MAX / 2,
        i32::MAX,
    ];
    for &a in &EXT {
        for &b in &EXT {
            for &c in &EXT {
                for &d in &EXT {
                    let got = diff_cleanup(a, b, c, d);
                    assert_eq!(got, model_cleanup(a, b, c, d), "at ({a},{b},{c},{d})");
                }
            }
        }
    }
}

// ---------------------------------------------------------------- G2
#[test]
fn bound_switch_label_neighbours() {
    // One step past every distinguished value, plus negative mirrors and zero.
    const V: [i32; 21] = [
        0, 9, 10, 11, 19, 20, 21, 29, 30, 31, 39, 40, 41, -9, -10, -11, -20, -30, -40, -41, 50,
    ];
    for &a in &V {
        for &b in &V {
            let got = diff_cleanup(a, b, 0, 0);
            assert_eq!(got, model_cleanup(a, b, 0, 0), "at ({a},{b},0,0)");
        }
        let got = diff_cleanup(a, a, a, a);
        assert_eq!(got, model_cleanup(a, a, a, a), "at ({a} x4)");
    }
}

// ---------------------------------------------------------------- G3
#[test]
fn bound_no_enum_surface_but_full_int_domain_agrees() {
    // The C API declares no enum type, so there is no invalid discriminant to
    // pass. The equivalent hazard is an arbitrary `int` reaching the switch;
    // sweep a large pseudo-random slice of the full domain, including values
    // adjacent to the labels and huge magnitudes.
    let mut rng = Rng::new(0xE22_0003);
    for _ in 0..2048 {
        let slot = |rng: &mut Rng| match rng.next_u64() % 4 {
            0 => rng.next_i32(),
            1 => rng.range_i32(-64, 64),
            2 => rng.pick(&[10, 20, 30, 40, 9, 11, 21, 31, 41]),
            _ => rng.pick(&[i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1]),
        };
        let t = [
            slot(&mut rng),
            slot(&mut rng),
            slot(&mut rng),
            slot(&mut rng),
        ];
        let got = diff_cleanup(t[0], t[1], t[2], t[3]);
        assert_eq!(got, model_cleanup(t[0], t[1], t[2], t[3]), "at {t:?}");
    }
}

// ---------------------------------------------------------------- G4
#[test]
fn bound_print_result_extremes() {
    let l = libs();
    let cf = l.c_print_result();
    let rf = l.rust_print_result();
    let cs = CString::new("label").unwrap();
    for &r in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        let c_out = capture_stdout(|| unsafe { cf(cs.as_ptr(), r) });
        let r_out = capture_stdout(|| unsafe { rf(cs.as_ptr(), r) });
        assert_eq!(c_out, r_out, "extreme result {r} mismatch");
        assert_eq!(c_out, format!("label: {r}\n").into_bytes());
    }
}

// ---------------------------------------------------------------- symbol parity
#[test]
fn symbol_parity_all_c_exports_resolve_in_rust() {
    let l = libs();
    for name in [
        &b"cleanup\0"[..],
        &b"print_result\0"[..],
        &b"cleanup_resources\0"[..],
    ] {
        let in_c = unsafe { l.c.get::<*const ()>(name).is_ok() };
        let in_rust = unsafe { l.rust.get::<*const ()>(name).is_ok() };
        let pretty = String::from_utf8_lossy(&name[..name.len() - 1]).to_string();
        assert!(in_c, "symbol {pretty} missing from the C .so");
        assert!(in_rust, "symbol {pretty} missing from the Rust .so");
    }
}
