// Phase C — error-path / rejection differential tests.
//
// One test per row of ERRORS.md. Because both public functions return `void`,
// "same error result" is asserted as: same stdout bytes AND (for the UB rows)
// same terminating signal / exit status of the process.

mod common;

use common::*;

/// E1: `printLine(NULL)` — the only null check in the library. No output at all.
#[test]
fn e1_print_line_null() {
    let c = run_print_line_null(Which::C);
    let rs = run_print_line_null(Which::Rust);
    assert_same("printLine(NULL)", &c, &rs);
    assert!(c.is_empty(), "C should print nothing for NULL, got {:?}", c);

    // Also out of process, so a crash (if any) would be observable.
    if runner_path().is_some() {
        let co = run_subprocess(Which::C, "printLineNull", "-").unwrap();
        let ro = run_subprocess(Which::Rust, "printLineNull", "-").unwrap();
        assert_eq!(co.code, ro.code, "exit code for printLine(NULL)");
        assert_eq!(co.signal, ro.signal, "signal for printLine(NULL)");
        assert_same("printLine(NULL) subprocess", &co.stdout, &ro.stdout);
    }
}

/// E2: `data >= 100` — the `if (data < 100)` guard rejects the copy.
#[test]
fn e2_driver_data_ge_100() {
    for d in [100, 101, 150, 199, 200, 12345] {
        diff_driver(d);
        let c = run_driver(Which::C, d);
        assert_eq!(c, b"\n", "C prints only a newline for driver({d})");
    }
}

/// E3: exactly one step past the valid range.
#[test]
fn e3_driver_boundary_exactly_100() {
    diff_driver(100);
    // and the last accepted value, for contrast
    diff_driver(99);
    assert_ne!(
        run_driver(Which::C, 99),
        run_driver(Which::C, 100),
        "sanity: the C guard must actually change behaviour at 100"
    );
    assert_eq!(run_driver(Which::C, 100), run_driver(Which::Rust, 100));
    assert_eq!(run_driver(Which::C, 99), run_driver(Which::Rust, 99));
}

/// E4: INT_MAX — maximal oversized length.
#[test]
fn e4_driver_int_max() {
    diff_driver(i32::MAX);
    assert_eq!(run_driver(Which::C, i32::MAX), b"\n");
}

/// E5: `data == 99`, the largest in-branch value (off-by-one edge; `strncpy`
/// copies no NUL from `source`, the explicit `dest[99]='\0'` terminates it).
#[test]
fn e5_driver_99_off_by_one() {
    diff_driver(99);
    let c = run_driver(Which::C, 99);
    let mut expect = vec![b'A'; 99];
    expect.push(b'\n');
    assert_eq!(c, expect, "C output for driver(99)");
}

/// E6: zero length.
#[test]
fn e6_driver_zero_length() {
    diff_driver(0);
    assert_eq!(run_driver(Which::C, 0), b"\n");
}

/// E7: negative `data` — undefined behaviour in the C (huge `size_t` for
/// `strncpy`), which must be reproduced: both libraries must die the same way.
#[test]
fn e7_driver_negative_ub_matches() {
    if runner_path().is_none() {
        eprintln!("skipping e7: runner example not built");
        return;
    }
    let mut rng = Rng::new(SEED ^ 0xE7);
    let mut cases: Vec<i32> = vec![-1, -2, -3, -50, -99, -100, -1000, -65536];
    for _ in 0..12 {
        cases.push(rng.range_i32(i32::MIN + 1, -1));
    }
    for d in cases {
        let arg = d.to_string();
        let c = run_subprocess(Which::C, "driver", &arg).unwrap();
        let r = run_subprocess(Which::Rust, "driver", &arg).unwrap();
        assert_eq!(c.signal, r.signal, "terminating signal for driver({d})");
        assert_eq!(c.code, r.code, "exit code for driver({d})");
        assert_same(&format!("stdout of driver({d})"), &c.stdout, &r.stdout);
    }
}

/// E8: INT_MIN.
#[test]
fn e8_driver_int_min() {
    if runner_path().is_none() {
        eprintln!("skipping e8: runner example not built");
        return;
    }
    let arg = i32::MIN.to_string();
    let c = run_subprocess(Which::C, "driver", &arg).unwrap();
    let r = run_subprocess(Which::Rust, "driver", &arg).unwrap();
    assert_eq!(c.signal, r.signal, "terminating signal for driver(INT_MIN)");
    assert_eq!(c.code, r.code, "exit code for driver(INT_MIN)");
    assert_same("stdout of driver(INT_MIN)", &c.stdout, &r.stdout);
}

/// E9: `printLine` performs no length validation — the NUL position alone
/// decides where reading stops.
#[test]
fn e9_print_line_no_length_check() {
    // NUL right at the start of a large buffer full of data.
    let mut buf = vec![0u8; 1];
    buf.extend(vec![b'Z'; 4096]);
    buf.push(0);
    diff_print_line(&buf);

    // NUL at the very end of a large buffer.
    let mut buf2 = vec![b'Z'; 4096];
    buf2.push(0);
    diff_print_line(&buf2);

    // Unaligned / offset pointers into the same buffer.
    let big: Vec<u8> = (0..1024u32).map(|i| ((i % 255) + 1) as u8).collect();
    for off in [0usize, 1, 3, 7, 15, 511, 1023] {
        let mut b = big[off..].to_vec();
        b.push(0);
        diff_print_line(&b);
    }
}

/// E10: empty string.
#[test]
fn e10_print_line_empty_string() {
    diff_print_line(b"\0");
    assert_eq!(run_print_line(Which::C, b"\0"), b"\n");
}

/// Generic FFI-boundary sweep: there are no enums in this API, so the closest
/// analogue is the unconstrained `int` parameter. Sweep every "interesting"
/// integer including all powers of two and their neighbours, skipping the
/// negatives (covered out-of-process by E7/E8).
#[test]
fn generic_int_boundary_sweep() {
    let mut cases: Vec<i32> = Vec::new();
    for bit in 0..31 {
        let v: i64 = 1i64 << bit;
        for delta in [-1i64, 0, 1] {
            let x = v + delta;
            if (0..=i32::MAX as i64).contains(&x) {
                cases.push(x as i32);
            }
        }
    }
    cases.extend([0, 1, 98, 99, 100, 101, i32::MAX - 1, i32::MAX]);
    cases.sort_unstable();
    cases.dedup();
    for d in cases {
        diff_driver(d);
    }
}
