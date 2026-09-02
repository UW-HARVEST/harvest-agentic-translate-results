//! Phase C — error-path / boundary differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no explicit rejection
//! paths at all (no `return`, no `assert`, no `NULL` check, no range check —
//! see the greps recorded in `ERRORS.md`), so the "error surface" is the set of
//! implicit undefined-behaviour arithmetic conditions plus the generic FFI
//! boundary values. For each, both `.so` files are driven with the exact
//! triggering input and their observable results (the `stdout` bytes — there is
//! no return value) must be identical, and the *specific* wrapped value the C
//! produces is asserted, not merely "both did something".

mod harness;

use harness::{harness, Entry, Harness, Rng};

/// ERRORS row 1 — `run(INT_MAX)`: signed overflow of `bedrooms += extra_bedrooms`.
#[test]
fn err_01_run_int_max() {
    let mut h = harness();
    let before = h.probe_bedrooms() as i32;

    let (c_out, r_out) = h.call_both(Entry::Run, i32::MAX);
    assert_eq!(
        c_out,
        r_out,
        "\nrun(i32::MAX) divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    // The C wraps two's-complement (plain `addl` at -O0); assert the exact value.
    assert_eq!(
        Harness::parse_last_state(&c_out).1,
        before.wrapping_add(i32::MAX) as i64,
        "run(i32::MAX) must wrap bedrooms two's-complement, as the C does"
    );
}

/// ERRORS row 2 — `run(INT_MIN)`: signed underflow of the same `+=`.
#[test]
fn err_02_run_int_min() {
    let mut h = harness();
    let before = h.probe_bedrooms() as i32;

    let (c_out, r_out) = h.call_both(Entry::Run, i32::MIN);
    assert_eq!(
        c_out,
        r_out,
        "\nrun(i32::MIN) divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(
        Harness::parse_last_state(&c_out).1,
        before.wrapping_add(i32::MIN) as i64,
        "run(i32::MIN) must wrap bedrooms two's-complement, as the C does"
    );
}

/// ERRORS row 3 — one step inside each end of the `int` range.
#[test]
fn err_03_run_one_step_inside_range() {
    let mut h = harness();
    for &arg in &[i32::MAX - 1, i32::MIN + 1] {
        let before = h.probe_bedrooms() as i32;
        let (c_out, r_out) = h.call_both(Entry::Run, arg);
        assert_eq!(
            c_out,
            r_out,
            "\nrun({arg}) divergence\n  C:    {:?}\n  Rust: {:?}\n",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        assert_eq!(
            Harness::parse_last_state(&c_out).1,
            before.wrapping_add(arg) as i64
        );
    }
}

/// ERRORS row 4 — negative delta drives `bedrooms` below zero; the C never
/// clamps or validates, so the value must be printed as a negative `%d`.
#[test]
fn err_04_run_negative_drives_bedrooms_below_zero() {
    let mut h = harness();
    h.set_bedrooms(2, "err04");

    for step in 0..6 {
        h.assert_same(Entry::Run, -1, &format!("err04/step{step}"));
    }
    let after = h.probe_bedrooms();
    assert!(
        after < 0,
        "err04: bedrooms should have gone negative, got {after}"
    );

    let (c_out, r_out) = h.call_both(Entry::Run, -1);
    assert_eq!(c_out, r_out);
    let text = String::from_utf8_lossy(&c_out);
    assert!(
        text.contains("-"),
        "err04: expected a negative bedrooms count in the output, got {text:?}"
    );
}

/// ERRORS row 5 — degenerate zero delta: the third and fourth printed lines must
/// be byte-identical to each other (the C calls `add_bedrooms(.., 0)` between
/// them).
#[test]
fn err_05_run_zero_delta() {
    let mut h = harness();
    let (c_out, r_out) = h.call_both(Entry::Run, 0);
    assert_eq!(
        c_out,
        r_out,
        "\nrun(0) divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    let lines: Vec<&str> = std::str::from_utf8(&c_out)
        .expect("utf8")
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(
        lines[2], lines[3],
        "run(0) must print the same line twice at the end"
    );
}

/// ERRORS row 6 — `driver(INT_MAX)`: `run` is called twice, so the overflowing
/// add is applied twice.
#[test]
fn err_06_driver_int_max_double_wrap() {
    let mut h = harness();
    let before = h.probe_bedrooms() as i32;

    let (c_out, r_out) = h.call_both(Entry::Driver, i32::MAX);
    assert_eq!(
        c_out,
        r_out,
        "\ndriver(i32::MAX) divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(
        Harness::parse_last_state(&c_out).1,
        before.wrapping_add(i32::MAX).wrapping_add(i32::MAX) as i64,
        "driver(i32::MAX) must apply the wrapping add twice"
    );
}

/// ERRORS row 7 — `driver(INT_MIN)`: double underflow.
#[test]
fn err_07_driver_int_min_double_wrap() {
    let mut h = harness();
    let before = h.probe_bedrooms() as i32;

    let (c_out, r_out) = h.call_both(Entry::Driver, i32::MIN);
    assert_eq!(
        c_out,
        r_out,
        "\ndriver(i32::MIN) divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(
        Harness::parse_last_state(&c_out).1,
        before.wrapping_add(i32::MIN).wrapping_add(i32::MIN) as i64,
        "driver(i32::MIN) must apply the wrapping add twice"
    );
}

/// ERRORS row 8 — overflow at the exact boundary: `bedrooms == i32::MAX`, then
/// `run(1)`.
#[test]
fn err_08_run_overflow_at_exact_boundary() {
    let mut h = harness();
    h.set_bedrooms(i32::MAX, "err08");

    let (c_out, r_out) = h.call_both(Entry::Run, 1);
    assert_eq!(
        c_out,
        r_out,
        "\nboundary overflow divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(
        Harness::parse_last_state(&c_out).1,
        i32::MIN as i64,
        "i32::MAX + 1 must wrap to i32::MIN, as the C does"
    );
}

/// ERRORS row 9 — underflow at the exact boundary: `bedrooms == i32::MIN`, then
/// `run(-1)`.
#[test]
fn err_09_run_underflow_at_exact_boundary() {
    let mut h = harness();
    h.set_bedrooms(i32::MIN, "err09");

    let (c_out, r_out) = h.call_both(Entry::Run, -1);
    assert_eq!(
        c_out,
        r_out,
        "\nboundary underflow divergence\n  C:    {:?}\n  Rust: {:?}\n",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
    assert_eq!(
        Harness::parse_last_state(&c_out).1,
        i32::MAX as i64,
        "i32::MIN + (-1) must wrap to i32::MAX, as the C does"
    );
}

/// ERRORS row 10 — "out-of-range enum values". Neither entry point takes an
/// `enum`/bitflag/mode, so the valid domain is the entire `int` range and no
/// value can be out of range. Rather than assume that, sweep the extremes, the
/// bit-pattern edges, and a fixed-seed uniform sample of the whole `i32` domain
/// through BOTH entry points and require identical behaviour with no rejection.
#[test]
fn err_10_full_int_domain_no_rejected_values() {
    let mut h = harness();

    // Bit-pattern edges and values that would be "no valid variant" for any
    // plausible enum encoding.
    let edges: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        3,
        4,
        5,
        -5,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x7FFF_FFFFu32 as i32,
        0x8000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x0000_FFFF,
        0x7FFF,
        -0x8000,
        1 << 30,
        -(1 << 30),
        255,
        256,
        -256,
        65_536,
        -65_536,
    ];
    for &arg in &edges {
        h.assert_same(Entry::Run, arg, "err10/edge/run");
        h.assert_same(Entry::Driver, arg, "err10/edge/driver");
    }

    let mut rng = Rng::new(0x0000_000A_5EED_00FF);
    for i in 0..600 {
        let arg = rng.next_i32();
        let entry = if rng.bool() { Entry::Run } else { Entry::Driver };
        h.assert_same(entry, arg, &format!("err10/rand{i}"));
    }
}

/// ERRORS rows 11 & 12 — null pointers and zero/oversized lengths are
/// unreachable across this ABI. This test documents and *mechanically enforces*
/// that: both exported symbols must have the shape `void f(int)`, i.e. the
/// public C surface must contain no pointer, buffer, length or callback
/// parameter that could carry a null or an out-of-bounds size.
#[test]
fn err_11_and_12_no_pointer_or_length_parameters_exist() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("failed to read c_src/include/driver.h");

    // Strip comments so the license text does not produce false hits.
    let code: String = header
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !code.contains('*'),
        "the public header gained a pointer parameter; ERRORS.md rows 11/12 must be revisited:\n{code}"
    );
    assert!(
        code.contains("void driver(int x);"),
        "public header no longer declares `void driver(int x);`:\n{code}"
    );

    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/driver.c"),
    )
    .expect("failed to read c_src/src/driver.c");

    // The only externally linked definitions must still be these two, and both
    // must take a single `int`.
    assert!(source.contains("void run(int extra_bedrooms) {"));
    assert!(source.contains("void driver(int x) {"));

    // And there must still be no rejection paths to test (the premise of
    // ERRORS.md). If any of these ever appear, ERRORS.md needs new rows.
    for forbidden in ["assert(", "return -", "return NULL", "errno", "exit("] {
        assert!(
            !source.contains(forbidden),
            "c_src/src/driver.c now contains `{forbidden}`; ERRORS.md needs a new row for it"
        );
    }

    // Finally: calling both entry points still cannot fail, whatever we pass.
    let mut h = harness();
    h.assert_same(Entry::Run, 0, "err11/smoke");
    h.assert_same(Entry::Driver, 0, "err11/smoke");
}

/// ERRORS row 15 is exercised in its own test binary
/// (`tests/printf_format.rs`) because it needs to observe the `%.1f` field
/// width growing from its pristine 3-character form, which requires a process
/// where no other test has already advanced the persistent global state.
#[test]
fn err_15_bathrooms_width_growth_see_printf_format_rs() {
    // Guard: keep the pointer to the real test honest.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/printf_format.rs");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} missing: {e}", path.display()));
    assert!(
        body.contains("fn err_15_bathrooms_width_growth"),
        "tests/printf_format.rs no longer contains the ERRORS.md row 15 test"
    );
}

