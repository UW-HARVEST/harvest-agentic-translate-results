//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundary rows G1-G10.
//! Both `.so` files are loaded via `libloading`; the assertion is always that
//! the two agree on the *same* outcome (return value **and** stdout), never
//! merely that "both failed somehow".

mod common;

use common::*;

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — the `strncmp` validation rejection at lib.c:42.
//
// Both operands are the in-function literal "VALID", so no FFI argument can
// make the branch fire. The verifiable property is therefore: for *every*
// input, both libraries take the success path and neither ever emits the
// diagnostic, and the return value is the accumulator (not the 0 the rejection
// path would produce).
// ---------------------------------------------------------------------------

#[test]
fn err01_string_validation_branch_unreachable_in_both() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 101);
    let _g = stdout_guard();

    for _ in 0..3_000 {
        let (a, b, c, d) = (
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
        );
        let (c_ret, c_out) = capture(|| unsafe { (p.c.cleanup)(a, b, c, d) });
        let (r_ret, r_out) = capture(|| unsafe { (p.rust.cleanup)(a, b, c, d) });
        assert_eq!(c_ret, r_ret, "cleanup({a},{b},{c},{d}) return differs");
        assert_eq!(c_out, r_out, "cleanup({a},{b},{c},{d}) stdout differs");
        assert!(
            !contains(&c_out, b"Input string validation failed."),
            "C unexpectedly took the validation-rejection branch"
        );
        assert!(
            !contains(&r_out, b"Input string validation failed."),
            "Rust took the validation-rejection branch where C did not"
        );
    }

    // The rejection path would `return 0` after skipping the switch. A case
    // whose accumulator is provably non-zero therefore also proves the branch
    // was not taken in either library.
    let (c_ret, _) = capture(|| unsafe { (p.c.cleanup)(10, 10, 10, 10) });
    let (r_ret, _) = capture(|| unsafe { (p.rust.cleanup)(10, 10, 10, 10) });
    assert_eq!(c_ret, r_ret);
    assert_ne!(c_ret, 0, "success path must not yield the rejection's 0");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — the malloc-failure rejection at lib.c:66.
//
// `malloc(50)` cannot be made to fail by any argument, so as with row 1 the
// checkable property is that neither library ever takes the branch, and that
// on the taken path the accumulator is still returned.
// ---------------------------------------------------------------------------

#[test]
fn err02_malloc_failure_branch_unreachable_in_both() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 102);
    let _g = stdout_guard();

    for _ in 0..3_000 {
        let (a, b, c, d) = (
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
        );
        let (c_ret, c_out) = capture(|| unsafe { (p.c.cleanup)(a, b, c, d) });
        let (r_ret, r_out) = capture(|| unsafe { (p.rust.cleanup)(a, b, c, d) });
        assert_eq!(c_ret, r_ret);
        assert_eq!(c_out, r_out);
        assert!(
            !contains(&c_out, b"Memory allocation failed."),
            "C unexpectedly took the allocation-failure branch"
        );
        assert!(
            !contains(&r_out, b"Memory allocation failed."),
            "Rust took the allocation-failure branch where C did not"
        );
        // Success path always prints the formatted line instead.
        assert!(contains(&c_out, b"Processed numbers: numbers"));
        assert!(contains(&r_out, b"Processed numbers: numbers"));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 / G9 — `cleanup_resources` null guard at lib.c:84
// ---------------------------------------------------------------------------

#[test]
fn err03_cleanup_resources_null_is_noop() {
    let p = pair();
    let _g = stdout_guard();
    for _ in 0..1_000 {
        let (_, c_out) = capture(|| unsafe { (p.c.cleanup_resources)(std::ptr::null_mut()) });
        let (_, r_out) = capture(|| unsafe { (p.rust.cleanup_resources)(std::ptr::null_mut()) });
        assert_eq!(c_out, r_out, "cleanup_resources(NULL) stdout differs");
        assert!(
            c_out.is_empty(),
            "cleanup_resources(NULL) must be silent, got {:?}",
            String::from_utf8_lossy(&c_out)
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — accumulator overflow / underflow (`result += numbers[i]`)
// ---------------------------------------------------------------------------

#[test]
fn err04_accumulator_overflow_matches_compiled_c() {
    // Hand-picked wrap-inducing shapes...
    let cases: &[[i32; 4]] = &[
        [i32::MAX, 1, 0, 0],
        [i32::MAX, i32::MAX, 0, 0],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, -1, 0, 0],
        [i32::MIN, i32::MIN, 0, 0],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, 10, 0, 0],  // label contributes +30 on top of INT_MAX
        [i32::MAX, 30, 0, 0],  // label contributes +70 on top of INT_MAX
        [i32::MIN, 10, 0, 0],
        [i32::MIN, 30, 0, 0],
        [i32::MAX, i32::MIN, 0, 0],
        [2_147_483_600, 100, 0, 0],
        [-2_147_483_600, -100, 0, 0],
        [1_073_741_824, 1_073_741_824, 0, 0],
        [1_073_741_824, 1_073_741_824, 1_073_741_824, 1_073_741_824],
        [-1_073_741_824, -1_073_741_824, -1_073_741_824, -1_073_741_824],
    ];
    for &[a, b, c, d] in cases {
        assert_cleanup(a, b, c, d);
    }

    // ...plus randomised large-magnitude values, which overflow constantly.
    let mut rng = Rng::new(SEED ^ 104);
    for _ in 0..10_000 {
        let big = |rng: &mut Rng| {
            if rng.bool() {
                rng.range_i32(i32::MAX - 1_000_000, i32::MAX)
            } else {
                rng.range_i32(i32::MIN, i32::MIN + 1_000_000)
            }
        };
        assert_cleanup(big(&mut rng), big(&mut rng), big(&mut rng), big(&mut rng));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 / G-null — `print_result` with a NULL label
// ---------------------------------------------------------------------------

#[test]
fn err05_print_result_null_label() {
    let p = pair();
    let _g = stdout_guard();
    for r in [0, 1, -1, 42, i32::MAX, i32::MIN] {
        let (_, c_out) = capture(|| unsafe { (p.c.print_result)(std::ptr::null(), r) });
        let (_, r_out) = capture(|| unsafe { (p.rust.print_result)(std::ptr::null(), r) });
        assert_eq!(
            c_out,
            r_out,
            "print_result(NULL, {r}) stdout differs:\n C   : {:?}\n Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        // Document the concrete glibc rendering the C ground truth produces.
        assert_eq!(
            c_out,
            format!("(null): {r}\n").into_bytes(),
            "unexpected C rendering of a null %s argument"
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 — intentionally NOT executed.
//
// `cleanup_resources` only null-checks, so handing it a non-`malloc` pointer
// executes `free()` on it. That is undefined behaviour that aborts the process
// in *both* libraries identically and would prove nothing while destroying the
// test run. Recorded here so the row is accounted for rather than forgotten.
// ---------------------------------------------------------------------------

#[test]
fn err06_non_malloc_pointer_is_out_of_contract() {
    // Assert only the contract boundary we *can* check: the two libraries
    // agree on which pointer values are treated as "nothing to release".
    assert_same("cleanup_resources(NULL) — the only rejectable pointer", |imp| unsafe {
        (imp.cleanup_resources)(std::ptr::null_mut());
    });
}

// ---------------------------------------------------------------------------
// ERRORS.md row 7 — live malloc pointer is the only defined non-null input
// ---------------------------------------------------------------------------

#[test]
fn err07_cleanup_resources_live_pointer() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 107);
    let _g = stdout_guard();
    for _ in 0..1_000 {
        let n = rng.range_usize(1, 8192);
        let pc = c_malloc(n);
        assert!(!pc.is_null());
        let (_, c_out) = capture(|| unsafe { (p.c.cleanup_resources)(pc) });
        let pr = c_malloc(n);
        assert!(!pr.is_null());
        let (_, r_out) = capture(|| unsafe { (p.rust.cleanup_resources)(pr) });
        assert_eq!(c_out, r_out);
        assert!(c_out.is_empty());
    }
    // Zero-size allocation: malloc(0) returns a unique freeable pointer.
    let pc = c_malloc(0);
    let pr = c_malloc(0);
    let (_, c_out) = capture(|| unsafe { (p.c.cleanup_resources)(pc) });
    let (_, r_out) = capture(|| unsafe { (p.rust.cleanup_resources)(pr) });
    assert_eq!(c_out, r_out, "cleanup_resources(malloc(0)) differs");
}

// ---------------------------------------------------------------------------
// Generic FFI boundary rows G1-G8, G10
// ---------------------------------------------------------------------------

#[test]
fn g01_all_zero_arguments() {
    assert_cleanup(0, 0, 0, 0);
    // Zero mixed with each label, in every position.
    for l in LABELS {
        for pos in 0..4usize {
            let mut args = [0i32; 4];
            args[pos] = l;
            assert_cleanup(args[0], args[1], args[2], args[3]);
        }
    }
}

#[test]
fn g02_int_extremes_in_every_position() {
    for extreme in [i32::MIN, i32::MAX, -1, 1] {
        for pos in 0..4usize {
            let mut args = [0i32; 4];
            args[pos] = extreme;
            assert_cleanup(args[0], args[1], args[2], args[3]);
        }
        assert_cleanup(extreme, extreme, extreme, extreme);
    }
    // Every pair of extremes in every ordered pair of positions.
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &x in &extremes {
        for &y in &extremes {
            assert_cleanup(x, y, 0, 0);
            assert_cleanup(0, 0, x, y);
            assert_cleanup(x, 0, y, 0);
        }
    }
}

#[test]
fn g03_one_step_past_each_switch_label() {
    for l in LABELS {
        for delta in [-2i32, -1, 1, 2] {
            let v = l + delta;
            for pos in 0..4usize {
                let mut args = [0i32; 4];
                args[pos] = v;
                assert_cleanup(args[0], args[1], args[2], args[3]);
            }
            assert_cleanup(v, v, v, v);
        }
    }
    // Also the values just outside the whole label span.
    for v in [0, 9, 41, 50, -10, -20, -30, -40] {
        assert_cleanup(v, v, v, v);
    }
}

#[test]
fn g04_out_of_range_enum_variants() {
    // The library declares no enum; `cleanup`'s switch is the equivalent
    // discriminated dispatch (4 named labels + default). Every int with no
    // matching label is an "out-of-range variant" and must go to `default`
    // identically in both libraries. Includes values that look like enum
    // sentinels and values that alias the labels under narrower widths.
    let odd: &[i32] = &[
        -1,
        i32::MIN,
        i32::MAX,
        0x7FFF_FFFF,
        -0x8000_0000,
        0xFFFF,
        0x1_0000,
        266,      // 10 + 256: same low byte as label 10
        276,      // 20 + 256
        286,      // 30 + 256
        296,      // 40 + 256
        -10,
        -20,
        -30,
        -40,
        65546,    // 10 + 65536
        1_000_000,
        -1_000_000,
        11,
        21,
        31,
        41,
        5,
        50,
        100,
    ];
    for &v in odd {
        for pos in 0..4usize {
            let mut args = [0i32; 4];
            args[pos] = v;
            assert_cleanup(args[0], args[1], args[2], args[3]);
        }
        assert_cleanup(v, v, v, v);
        // Mixed with genuine labels so `default` and fall-through interleave.
        for l in LABELS {
            assert_cleanup(v, l, v, l);
            assert_cleanup(l, v, l, v);
        }
    }
}

#[test]
fn g05_print_result_zero_length_label() {
    assert_print_result_raw(b"\0", 0);
    assert_print_result_raw(b"\0", i32::MIN);
    assert_print_result_raw(b"\0", i32::MAX);
}

#[test]
fn g06_print_result_oversized_label() {
    for n in [1usize, 1024, 65_535, 65_536, 65_537, 1 << 20] {
        let mut bytes = vec![b'A'; n];
        bytes.push(0);
        assert_print_result_raw(&bytes, n as i32);
    }
}

#[test]
fn g07_print_result_format_specifier_label() {
    for label in [
        "%s", "%d", "%n", "%%", "%p", "%x", "%1000000d", "%.2147483647f", "%*s", "%hhn",
    ] {
        assert_print_result(label, 0);
        assert_print_result(label, -1);
    }
    // A label that is *only* dangerous if wrongly used as a format string.
    assert_print_result("%s%s%s%s%s%s%s%s%s%s%n%n%n", 5);
}

#[test]
fn g08_print_result_high_byte_label() {
    for b in 0x80u8..=0xFF {
        assert_print_result_raw(&[b, 0], b as i32);
    }
    let mut all: Vec<u8> = (1u8..=255).rev().collect();
    all.push(0);
    assert_print_result_raw(&all, 0);
}

#[test]
fn g10_cleanup_result_fed_into_print_result() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 110);
    let _g = stdout_guard();
    let label = std::ffi::CString::new("result").unwrap();
    for _ in 0..2_000 {
        let (a, b, c, d) = (
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
        );
        let run = |imp: &Impl| unsafe {
            let r = (imp.cleanup)(a, b, c, d);
            (imp.print_result)(label.as_ptr(), r);
            r
        };
        let (c_ret, c_out) = capture(|| run(&p.c));
        let (r_ret, r_out) = capture(|| run(&p.rust));
        assert_eq!(c_ret, r_ret, "({a},{b},{c},{d}) return differs");
        assert_eq!(
            c_out,
            r_out,
            "({a},{b},{c},{d}) stdout differs:\n C   : {:?}\n Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
