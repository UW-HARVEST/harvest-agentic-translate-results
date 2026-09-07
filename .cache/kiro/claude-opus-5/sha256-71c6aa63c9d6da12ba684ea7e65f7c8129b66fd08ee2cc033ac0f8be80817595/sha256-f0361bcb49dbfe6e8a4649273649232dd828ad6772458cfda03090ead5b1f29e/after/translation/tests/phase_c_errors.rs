//! Phase C — error-path differential tests, one test per row of ERRORS.md,
//! plus the generic FFI boundary cases.

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};

// ---------------------------------------------------------------------------
// Sanity: we really did load two DIFFERENT libraries.
// ---------------------------------------------------------------------------
#[test]
fn sanity_two_distinct_libraries_loaded() {
    let p = pair();
    for i in 0..4usize {
        assert_ne!(
            p.c.op_by_index(i) as usize,
            p.rs.op_by_index(i) as usize,
            "C and Rust {} resolved to the same address — only one .so loaded",
            Impl::op_name(i)
        );
    }
    assert_ne!(p.c.checkshift as usize, p.rs.checkshift as usize);
}

// ===========================================================================
// Rows 1-5: get_operation out-of-range opcodes (incl. out-of-range "enum")
// ===========================================================================

fn diff_get_operation_null(row: &str, opcodes: &[c_int]) {
    let p = pair();
    let (c_res, c_out) = capture_stdout(|| {
        opcodes
            .iter()
            .map(|&o| unsafe { (p.c.get_operation)(o) }.map(|f| f as usize))
            .collect::<Vec<_>>()
    });
    let (r_res, r_out) = capture_stdout(|| {
        opcodes
            .iter()
            .map(|&o| unsafe { (p.rs.get_operation)(o) }.map(|f| f as usize))
            .collect::<Vec<_>>()
    });
    for (k, &o) in opcodes.iter().enumerate() {
        assert!(c_res[k].is_none(), "[{row}] C get_operation({o}) was NOT NULL");
        assert!(r_res[k].is_none(), "[{row}] Rust get_operation({o}) was NOT NULL");
    }
    assert_stdout_eq(row, &c_out, &r_out);
    assert!(c_out.is_empty(), "[{row}] get_operation printed something");
}

#[test]
fn err01_get_operation_minus_one() {
    diff_get_operation_null("err01 opcode=-1", &[-1]);
}

#[test]
fn err02_get_operation_int_min() {
    diff_get_operation_null("err02 opcode=INT_MIN", &[i32::MIN]);
}

#[test]
fn err03_get_operation_four() {
    diff_get_operation_null("err03 opcode=4", &[4]);
}

#[test]
fn err04_get_operation_int_max() {
    diff_get_operation_null("err04 opcode=INT_MAX", &[i32::MAX]);
}

#[test]
fn err05_get_operation_arbitrary_out_of_range_enum_values() {
    // C "enums"/opcodes accept any int across FFI; every one of these must be
    // rejected identically by both implementations.
    let mut bad: Vec<c_int> = vec![
        5,
        6,
        7,
        8,
        0x7FFF,
        0xDEAD,
        0xDEADBEEFu32 as i32,
        -2,
        -3,
        -100,
        -0x7FFF,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0x0100_0000,
        0x4,
    ];
    let mut rng = Rng::new(SEED ^ 0xE5);
    while bad.len() < 5_000 {
        let v = rng.next_i32();
        if !(0..4).contains(&v) {
            bad.push(v);
        }
    }
    diff_get_operation_null("err05 arbitrary out-of-range opcodes", &bad);

    // The in-range boundary must still be accepted by BOTH (asymmetric
    // acceptance would be just as much a bug).
    let p = pair();
    for o in [0, 1, 2, 3] {
        assert!(unsafe { (p.c.get_operation)(o) }.is_some(), "C rejected opcode {o}");
        assert!(unsafe { (p.rs.get_operation)(o) }.is_some(), "Rust rejected opcode {o}");
    }
}

// ===========================================================================
// Rows 6-9: execute_operation with a NULL function pointer
// ===========================================================================

fn diff_execute_null(row: &str, op_name: *const c_char) {
    let p = pair();
    let (c_res, c_out) =
        capture_stdout(|| unsafe { (p.c.execute_operation)(None, 12345, -6789, op_name) });
    let (r_res, r_out) =
        capture_stdout(|| unsafe { (p.rs.execute_operation)(None, 12345, -6789, op_name) });
    assert_eq!(c_res, r_res, "[{row}] return value differs");
    assert_eq!(c_res, 0, "[{row}] C did not return 0");
    assert_stdout_eq(row, &c_out, &r_out);
    assert!(
        String::from_utf8_lossy(&c_out)
            .starts_with("Error: Operation function pointer is NULL for "),
        "[{row}] unexpected C message: {}",
        show(&c_out)
    );
}

#[test]
fn err06_execute_null_func_named() {
    let name = CString::new("XOR").unwrap();
    diff_execute_null("err06 NULL func, name=XOR", name.as_ptr());
    // Also across a range of values: the NULL branch must be value-independent.
    let p = pair();
    let mut rng = Rng::new(SEED ^ 6);
    let cases: Vec<(c_int, c_int)> = (0..500).map(|_| (rng.spicy_i32(), rng.spicy_i32())).collect();
    let (c_res, c_out) = capture_stdout(|| {
        cases
            .iter()
            .map(|&(a, b)| unsafe { (p.c.execute_operation)(None, a, b, name.as_ptr()) })
            .collect::<Vec<_>>()
    });
    let (r_res, r_out) = capture_stdout(|| {
        cases
            .iter()
            .map(|&(a, b)| unsafe { (p.rs.execute_operation)(None, a, b, name.as_ptr()) })
            .collect::<Vec<_>>()
    });
    assert_eq!(c_res, r_res);
    assert!(c_res.iter().all(|&v| v == 0));
    assert_stdout_eq("err06 NULL func, many values", &c_out, &r_out);
}

#[test]
fn err07_execute_null_func_null_name() {
    // glibc's printf renders a NULL "%s" as "(null)"; both sides must match.
    diff_execute_null("err07 NULL func, NULL name", std::ptr::null());
}

#[test]
fn err08_execute_null_func_empty_name() {
    let name = CString::new("").unwrap();
    diff_execute_null("err08 NULL func, empty name", name.as_ptr());
}

#[test]
fn err09_execute_null_func_via_get_operation() {
    // Composed path: the NULL comes out of get_operation, not from the caller.
    let p = pair();
    let name = CString::new("COMPOSED").unwrap();
    for bad in [-1, 4, i32::MIN, i32::MAX, 99] {
        let (c_res, c_out) = capture_stdout(|| unsafe {
            let f = (p.c.get_operation)(bad);
            (p.c.execute_operation)(f, 7, 9, name.as_ptr())
        });
        let (r_res, r_out) = capture_stdout(|| unsafe {
            let f = (p.rs.get_operation)(bad);
            (p.rs.execute_operation)(f, 7, 9, name.as_ptr())
        });
        assert_eq!(c_res, r_res, "[err09 opcode={bad}] return differs");
        assert_eq!(c_res, 0);
        assert_stdout_eq(&format!("err09 opcode={bad}"), &c_out, &r_out);
    }
}

// ===========================================================================
// Rows 10-15: compute_checksum rejection paths
// ===========================================================================

fn diff_checksum_reject(row: &str, use_null: bool, counts: &[c_int]) {
    let p = pair();
    let mut vals: Vec<c_int> = vec![0x1234_5678, -1, i32::MIN, 42];
    let ptr = if use_null { std::ptr::null_mut() } else { vals.as_mut_ptr() };

    let (c_res, c_out) = capture_stdout(|| {
        counts.iter().map(|&n| unsafe { (p.c.compute_checksum)(ptr, n) }).collect::<Vec<c_uint>>()
    });
    let (r_res, r_out) = capture_stdout(|| {
        counts.iter().map(|&n| unsafe { (p.rs.compute_checksum)(ptr, n) }).collect::<Vec<c_uint>>()
    });
    for (k, &n) in counts.iter().enumerate() {
        assert_eq!(
            c_res[k], r_res[k],
            "[{row}] count={n}: C=0x{:08X} Rust=0x{:08X}",
            c_res[k], r_res[k]
        );
        assert_eq!(c_res[k], 0, "[{row}] count={n}: C sentinel was not 0");
    }
    assert_stdout_eq(row, &c_out, &r_out);
    assert!(c_out.is_empty());
}

#[test]
fn err10_checksum_null_values_count4() {
    diff_checksum_reject("err10 NULL values, count=4", true, &[4]);
}

#[test]
fn err11_checksum_null_values_count0() {
    diff_checksum_reject("err11 NULL values, count=0", true, &[0]);
}

#[test]
fn err12_checksum_null_values_negative_count() {
    diff_checksum_reject("err12 NULL values, count<0", true, &[-1, -2, -1000, i32::MIN]);
}

#[test]
fn err13_checksum_valid_ptr_count0() {
    diff_checksum_reject("err13 valid ptr, count=0", false, &[0]);
}

#[test]
fn err14_checksum_valid_ptr_count_minus_one() {
    diff_checksum_reject("err14 valid ptr, count=-1", false, &[-1]);
}

#[test]
fn err15_checksum_valid_ptr_count_int_min() {
    let mut counts: Vec<c_int> = vec![i32::MIN, i32::MIN + 1, -2, -3, -0x4000_0000];
    let mut rng = Rng::new(SEED ^ 15);
    while counts.len() < 2_000 {
        let v = rng.next_i32();
        if v <= 0 {
            counts.push(v);
        }
    }
    diff_checksum_reject("err15 valid ptr, count<=0 sweep", false, &counts);
    // NULL pointer combined with the same non-positive counts.
    diff_checksum_reject("err15 NULL ptr, count<=0 sweep", true, &counts);
    // NULL pointer with POSITIVE counts is also a rejection (values != NULL).
    diff_checksum_reject("err15 NULL ptr, count>0", true, &[1, 2, 3, 4, 5, 1000, i32::MAX]);
}

// ===========================================================================
// Row 16: init_state with a NULL state pointer
// ===========================================================================

#[test]
fn err16_init_state_null_state() {
    let p = pair();
    let vals: Vec<c_int> = vec![0, 1, -1, i32::MAX, i32::MIN, 12345];
    let (_, c_out) = capture_stdout(|| {
        for &v in &vals {
            unsafe { (p.c.init_state)(std::ptr::null_mut(), v) };
        }
    });
    let (_, r_out) = capture_stdout(|| {
        for &v in &vals {
            unsafe { (p.rs.init_state)(std::ptr::null_mut(), v) };
        }
    });
    assert_stdout_eq("err16 init_state NULL", &c_out, &r_out);
    let text = String::from_utf8_lossy(&c_out);
    assert_eq!(
        text.matches("Error: state pointer is NULL in init_state\n").count(),
        vals.len(),
        "[err16] wrong number of error lines: {}",
        show(&c_out)
    );
}

// ===========================================================================
// Rows 17-19: apply_operation rejection paths (order matters)
// ===========================================================================

#[test]
fn err17_apply_operation_null_state_valid_func() {
    let p = pair();
    let (_, c_out) = capture_stdout(|| unsafe {
        for i in 0..4 {
            let f = (p.c.get_operation)(i);
            (p.c.apply_operation)(std::ptr::null_mut(), 99, f);
        }
    });
    let (_, r_out) = capture_stdout(|| unsafe {
        for i in 0..4 {
            let f = (p.rs.get_operation)(i);
            (p.rs.apply_operation)(std::ptr::null_mut(), 99, f);
        }
    });
    assert_stdout_eq("err17 apply NULL state", &c_out, &r_out);
    assert_eq!(
        String::from_utf8_lossy(&c_out)
            .matches("Error: state pointer is NULL in apply_operation\n")
            .count(),
        4
    );
}

#[test]
fn err18_apply_operation_null_state_and_null_func() {
    // The state check comes FIRST in the C source: only the state message
    // must be printed, never the function-pointer message.
    let p = pair();
    let (_, c_out) =
        capture_stdout(|| unsafe { (p.c.apply_operation)(std::ptr::null_mut(), 1, None) });
    let (_, r_out) =
        capture_stdout(|| unsafe { (p.rs.apply_operation)(std::ptr::null_mut(), 1, None) });
    assert_stdout_eq("err18 apply NULL state + NULL func", &c_out, &r_out);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        "Error: state pointer is NULL in apply_operation\n",
        "[err18] check ordering diverged from the C source"
    );
}

#[test]
fn err19_apply_operation_valid_state_null_func() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 19);
    let states: Vec<ComputeState> = (0..500)
        .map(|_| ComputeState {
            accumulator: rng.spicy_i32(),
            operation_count: rng.range_i32(-3, 500),
            checksum: rng.next_i32() as c_uint,
        })
        .collect();
    let values: Vec<c_int> = (0..500).map(|_| rng.spicy_i32()).collect();

    let run = |imp: &Impl| {
        states
            .iter()
            .zip(values.iter())
            .map(|(st, &v)| {
                let mut s = *st;
                unsafe { (imp.apply_operation)(&mut s, v, None) };
                s.bytes()
            })
            .collect::<Vec<_>>()
    };
    let (c_states, c_out) = capture_stdout(|| run(&p.c));
    let (r_states, r_out) = capture_stdout(|| run(&p.rs));
    assert_eq!(c_states, r_states, "[err19] state bytes differ");
    assert_stdout_eq("err19 apply NULL func", &c_out, &r_out);
    // The struct must be COMPLETELY untouched.
    for (k, st) in states.iter().enumerate() {
        assert_eq!(c_states[k], st.bytes(), "[err19] case {k}: state was mutated");
    }
    assert_eq!(
        String::from_utf8_lossy(&c_out)
            .matches("Error: operation function pointer is NULL in apply_operation\n")
            .count(),
        states.len()
    );
}

// ===========================================================================
// Row 20: checkshift malloc-failure path (structural coverage)
// ===========================================================================

#[test]
fn err20_checkshift_malloc_failure_path_present_in_both() {
    // A 12-byte malloc cannot be made to fail deterministically from outside
    // the library, and both implementations call the process allocator. Verify
    // structurally that the identical failure path exists in both images.
    let needle = b"Error: Failed to allocate memory for state";
    let c_so = std::fs::read(find_any_so_c()).expect("read C .so");
    let r_so = std::fs::read(find_any_so_rust()).expect("read Rust .so");
    assert!(
        contains(&c_so, needle),
        "[err20] C .so is missing the malloc-failure message (test assumption broken)"
    );
    assert!(
        contains(&r_so, needle),
        "[err20] Rust .so does not contain the malloc-failure message — the \
         allocation-failure branch was not translated"
    );
    // Both must also contain every other diagnostic string. Note: gcc AND
    // rustc both rewrite a newline-terminated `printf` with no conversions
    // into `puts`, which drops the trailing newline from `.rodata` — so search
    // for each message with one trailing newline removed.
    for msg in [
        &b"Error: Operation function pointer is NULL for %s\n"[..],
        &b"Error: state pointer is NULL in init_state\n"[..],
        &b"Error: state pointer is NULL in apply_operation\n"[..],
        &b"Error: operation function pointer is NULL in apply_operation\n"[..],
        &b"Variable a = %d\n"[..],
        &b"Variable b = %d\n"[..],
        &b"Result of %s: %d\n"[..],
        &b"State initialized with accumulator = %d\n"[..],
        &b"\n=== Starting foo function ===\n"[..],
        &b"Parameters: %d, %d, %d, %d\n"[..],
        &b"\n--- Operation 1: Multiply ---\n"[..],
        &b"\n--- Operation 2: Add ---\n"[..],
        &b"\n--- Operation 3: XOR ---\n"[..],
        &b"\n--- Operation 4: Shift ---\n"[..],
        &b"\nComputed checksum: 0x%04X\n"[..],
        &b"\nFinal accumulator: %d\n"[..],
        &b"Operation count: %d\n"[..],
        &b"Final result: %d\n"[..],
        &b"=== Ending foo function ===\n\n"[..],
        &b"XOR\0"[..],
        &b"SHIFT\0"[..],
    ] {
        let trimmed = msg.strip_suffix(b"\n").unwrap_or(msg);
        assert!(
            contains(&c_so, msg) || contains(&c_so, trimmed),
            "C .so missing {:?} (test assumption)",
            show(msg)
        );
        assert!(
            contains(&r_so, msg) || contains(&r_so, trimmed),
            "Rust .so missing format string {:?}",
            show(msg)
        );
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

fn find_any_so_c() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/build");
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .expect("C .so")
}

fn find_any_so_rust() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("CHECKSHIFT_RUST_SO") {
        return std::path::PathBuf::from(p);
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libcheckshift_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust .so not found")
}

// ===========================================================================
// Generic FFI-boundary sweep (beyond the table)
// ===========================================================================

#[test]
fn err_generic_all_null_pointer_combinations() {
    let p = pair();
    let name = CString::new("N").unwrap();
    let mut st = ComputeState::default();

    // Every pointer parameter, individually and combined.
    let (_, c_out) = capture_stdout(|| unsafe {
        (p.c.execute_operation)(None, 1, 2, std::ptr::null());
        (p.c.execute_operation)(None, 1, 2, name.as_ptr());
        (p.c.compute_checksum)(std::ptr::null_mut(), 4);
        (p.c.compute_checksum)(std::ptr::null_mut(), 0);
        (p.c.init_state)(std::ptr::null_mut(), 5);
        (p.c.apply_operation)(std::ptr::null_mut(), 5, None);
        (p.c.apply_operation)(std::ptr::null_mut(), 5, Some(p.c.xor_operation));
        (p.c.apply_operation)(&mut st, 5, None);
    });
    let mut st2 = ComputeState::default();
    let (_, r_out) = capture_stdout(|| unsafe {
        (p.rs.execute_operation)(None, 1, 2, std::ptr::null());
        (p.rs.execute_operation)(None, 1, 2, name.as_ptr());
        (p.rs.compute_checksum)(std::ptr::null_mut(), 4);
        (p.rs.compute_checksum)(std::ptr::null_mut(), 0);
        (p.rs.init_state)(std::ptr::null_mut(), 5);
        (p.rs.apply_operation)(std::ptr::null_mut(), 5, None);
        (p.rs.apply_operation)(std::ptr::null_mut(), 5, Some(p.rs.xor_operation));
        (p.rs.apply_operation)(&mut st2, 5, None);
    });
    assert_stdout_eq("err generic NULL combos", &c_out, &r_out);
    assert_eq!(st.bytes(), st2.bytes(), "state diverged on the NULL-func path");
}

#[test]
fn err_generic_count_boundaries_full_sweep() {
    // Zero, valid, one-past-valid, and oversized lengths on the same data.
    let p = pair();
    let base: Vec<c_int> = vec![0x0BAD_F00Du32 as i32, -1, i32::MIN, 0x7F7F_7F7F];
    let counts: Vec<c_int> = vec![
        i32::MIN,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        100,
        1000,
        0x0100_0000,
        i32::MAX - 1,
        i32::MAX,
    ];
    let (c_res, c_out) = capture_stdout(|| {
        counts
            .iter()
            .map(|&n| {
                let mut v = base.clone();
                unsafe { (p.c.compute_checksum)(v.as_mut_ptr(), n) }
            })
            .collect::<Vec<c_uint>>()
    });
    let (r_res, r_out) = capture_stdout(|| {
        counts
            .iter()
            .map(|&n| {
                let mut v = base.clone();
                unsafe { (p.rs.compute_checksum)(v.as_mut_ptr(), n) }
            })
            .collect::<Vec<c_uint>>()
    });
    for (k, &n) in counts.iter().enumerate() {
        assert_eq!(
            c_res[k], r_res[k],
            "[err generic count sweep] count={n}: C=0x{:08X} Rust=0x{:08X}",
            c_res[k], r_res[k]
        );
    }
    assert_stdout_eq("err generic count sweep", &c_out, &r_out);
}

#[test]
fn err_generic_opcode_one_past_range_both_directions() {
    let p = pair();
    // -1 / 4 are the two "one step past" values; 0 / 3 the two valid ends.
    let probes: [(c_int, bool); 6] =
        [(-2, false), (-1, false), (0, true), (3, true), (4, false), (5, false)];
    for (op, should_be_some) in probes {
        let c = unsafe { (p.c.get_operation)(op) };
        let r = unsafe { (p.rs.get_operation)(op) };
        assert_eq!(
            c.is_some(),
            should_be_some,
            "C get_operation({op}) disagrees with the C source's range check"
        );
        assert_eq!(
            c.is_some(),
            r.is_some(),
            "get_operation({op}): C is_some={} Rust is_some={}",
            c.is_some(),
            r.is_some()
        );
    }
}

#[test]
fn err_generic_execute_with_out_of_range_opcode_and_null_name() {
    // Two error conditions crossing at once, driven through the FFI.
    let p = pair();
    for bad in [-1, 4, i32::MIN, i32::MAX] {
        let (c_res, c_out) = capture_stdout(|| unsafe {
            let f = (p.c.get_operation)(bad);
            (p.c.execute_operation)(f, i32::MIN, i32::MAX, std::ptr::null())
        });
        let (r_res, r_out) = capture_stdout(|| unsafe {
            let f = (p.rs.get_operation)(bad);
            (p.rs.execute_operation)(f, i32::MIN, i32::MAX, std::ptr::null())
        });
        assert_eq!(c_res, r_res, "[err generic opcode={bad}] return differs");
        assert_stdout_eq(&format!("err generic opcode={bad} NULL name"), &c_out, &r_out);
    }
}
