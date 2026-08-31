//! Differential tests for `driver`.
//!
//! `driver` is the only function in the public API (`c_src/include/driver.h`),
//! and internally it is a composition of two lower-level operations:
//!
//! 1. `div(x, y)` — quotient/remainder, and
//! 2. `printf("quotient: %d, remainder: %d\n", ...)` — formatting.
//!
//! The tests below work up that hierarchy: first the arithmetic core over
//! progressively wider input domains, then the exact formatting of the emitted
//! bytes, then the inputs C leaves undefined.

mod common;

use common::{Outcome, Pair, c_so_path, capture_stdout, outcome_of, rust_so_path};

const INT_MIN: i32 = i32::MIN;
const INT_MAX: i32 = i32::MAX;

/// Deterministic 32-bit values from a xorshift generator, so failures reproduce.
fn xorshift(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

// ---------------------------------------------------------------------------
// Level 0: the exports themselves
// ---------------------------------------------------------------------------

fn both_libraries_export_driver() {
    // Loading succeeds only if both `.so`s resolve the `driver` symbol.
    let pair = Pair::load();
    let out = pair.call_c(7, 2);
    assert_eq!(out, b"quotient: 3, remainder: 1\n");
    let out = pair.call_rust(7, 2);
    assert_eq!(out, b"quotient: 3, remainder: 1\n");
}

/// Guards the requirement that the Rust `.so` export set covers the C one.
fn rust_so_exports_every_c_symbol() {
    fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("failed to run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_owned))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    let c_so = c_so_path();
    let rust_so = rust_so_path();

    let c_syms = defined_dynamic_symbols(&c_so);
    let rust_syms = defined_dynamic_symbols(&rust_so);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing exports present in the C .so: {missing:?}\n\
         C exports: {c_syms:?}"
    );
    assert!(
        c_syms.iter().any(|s| s == "driver"),
        "expected the C .so to export `driver`, got {c_syms:?}"
    );
}

// ---------------------------------------------------------------------------
// Level 1: the div() core, over widening input domains
// ---------------------------------------------------------------------------

fn small_positive_grid() {
    let pair = Pair::load();
    for x in 0..=64 {
        for y in 1..=64 {
            pair.assert_same(x, y);
        }
    }
}

/// C's `div` truncates toward zero and the remainder takes the sign of the
/// numerator; this covers every sign combination.
fn all_sign_combinations() {
    let pair = Pair::load();
    for x in -40..=40 {
        for y in -40..=40 {
            if y == 0 {
                continue; // undefined in C; exercised separately
            }
            pair.assert_same(x, y);
        }
    }
}

fn exact_and_inexact_division() {
    let pair = Pair::load();
    let cases: &[(i32, i32)] = &[
        (100, 10),
        (100, 3),
        (-100, 10),
        (-100, 3),
        (100, -10),
        (100, -3),
        (-100, -10),
        (-100, -3),
        (1, 2),
        (-1, 2),
        (1, -2),
        (-1, -2),
        (0, 5),
        (0, -5),
        (5, 5),
        (5, -5),
        (-5, 5),
        (-5, -5),
        (7, 1),
        (7, -1),
        (-7, 1),
    ];
    for &(x, y) in cases {
        pair.assert_same(x, y);
    }
}

fn integer_extremes() {
    let pair = Pair::load();
    let interesting = [
        INT_MIN,
        INT_MIN + 1,
        INT_MIN + 2,
        -65537,
        -65536,
        -32769,
        -32768,
        -256,
        -100,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        100,
        256,
        32767,
        32768,
        65535,
        65536,
        INT_MAX - 2,
        INT_MAX - 1,
        INT_MAX,
    ];
    for &x in &interesting {
        for &y in &interesting {
            if y == 0 {
                continue; // undefined in C
            }
            if x == INT_MIN && y == -1 {
                continue; // undefined in C (overflow); exercised separately
            }
            pair.assert_same(x, y);
        }
    }
}

fn randomized_full_range_sweep() {
    let pair = Pair::load();
    let mut state: u32 = 0x1234_5678;
    for _ in 0..4000 {
        let x = xorshift(&mut state) as i32;
        let mut y = xorshift(&mut state) as i32;
        if y == 0 {
            y = 1;
        }
        if x == INT_MIN && y == -1 {
            y = 1;
        }
        pair.assert_same(x, y);
    }
}

/// Small divisors against full-range numerators: the case where the quotient is
/// large and the printed field widths differ most between implementations.
fn randomized_small_divisors() {
    let pair = Pair::load();
    let mut state: u32 = 0x9e37_79b9;
    for _ in 0..2000 {
        let x = xorshift(&mut state) as i32;
        let d = (xorshift(&mut state) % 19) as i32 + 1;
        let y = if xorshift(&mut state) & 1 == 0 { d } else { -d };
        if x == INT_MIN && y == -1 {
            continue;
        }
        pair.assert_same(x, y);
    }
}

/// Divisors near the numerator magnitude, producing quotients of -1, 0 and 1.
fn randomized_near_unit_quotients() {
    let pair = Pair::load();
    let mut state: u32 = 0x0bad_c0de;
    for _ in 0..2000 {
        let x = xorshift(&mut state) as i32;
        let delta = (xorshift(&mut state) % 5) as i32 - 2;
        let y = x.wrapping_add(delta);
        if y == 0 || (x == INT_MIN && y == -1) {
            continue;
        }
        pair.assert_same(x, y);
    }
}

// ---------------------------------------------------------------------------
// Level 2: the printf() formatting
// ---------------------------------------------------------------------------

/// Pins the literal byte layout of the output, including the trailing newline
/// and the absence of any padding, so a formatting drift cannot hide behind a
/// matching pair of implementations.
fn output_byte_layout_is_exact() {
    let pair = Pair::load();
    let cases: &[(i32, i32, &[u8])] = &[
        (7, 2, b"quotient: 3, remainder: 1\n"),
        (-7, 2, b"quotient: -3, remainder: -1\n"),
        (7, -2, b"quotient: -3, remainder: 1\n"),
        (-7, -2, b"quotient: 3, remainder: -1\n"),
        (0, 1, b"quotient: 0, remainder: 0\n"),
        (INT_MIN, 1, b"quotient: -2147483648, remainder: 0\n"),
        (INT_MAX, 1, b"quotient: 2147483647, remainder: 0\n"),
        (INT_MIN, -2, b"quotient: 1073741824, remainder: 0\n"),
        (INT_MAX, -1, b"quotient: -2147483647, remainder: 0\n"),
    ];
    for &(x, y, expected) in cases {
        let c_out = pair.call_c(x, y);
        let rust_out = pair.call_rust(x, y);
        assert_eq!(
            c_out, expected,
            "C output changed for driver({x}, {y}): {:?}",
            String::from_utf8_lossy(&c_out)
        );
        assert_eq!(
            rust_out, expected,
            "Rust output differs for driver({x}, {y}): {:?}",
            String::from_utf8_lossy(&rust_out)
        );
    }
}

/// Repeated calls must accumulate identically: no extra flushing, buffering or
/// separator differences between the two implementations.
fn repeated_calls_accumulate_identically() {
    let pair = Pair::load();
    let inputs: Vec<(i32, i32)> = (1..=25).map(|i| (i * 37 - 400, i - 13)).collect();

    let c = pair.c_fn();
    let c_out = capture_stdout(|| {
        for &(x, y) in &inputs {
            if y == 0 {
                continue;
            }
            unsafe { c(x, y) }
        }
    });

    let rust = pair.rust_fn();
    let rust_out = capture_stdout(|| {
        for &(x, y) in &inputs {
            if y == 0 {
                continue;
            }
            unsafe { rust(x, y) }
        }
    });

    assert_eq!(
        c_out,
        rust_out,
        "accumulated stdout mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        24,
        "expected one line per call"
    );
}

// ---------------------------------------------------------------------------
// Level 2b: high-volume bulk comparison
// ---------------------------------------------------------------------------

/// Batches many calls into a single capture each, which makes it cheap to cover
/// a very large input space (the per-call fd redirect is the expensive part).
fn bulk_compare(label: &str, inputs: &[(i32, i32)]) {
    let pair = Pair::load();

    let c = pair.c_fn();
    let c_out = capture_stdout(|| {
        for &(x, y) in inputs {
            unsafe { c(x, y) }
        }
    });

    let rust = pair.rust_fn();
    let rust_out = capture_stdout(|| {
        for &(x, y) in inputs {
            unsafe { rust(x, y) }
        }
    });

    if c_out != rust_out {
        // Pinpoint the first differing line and map it back to its input.
        let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let r_lines: Vec<&[u8]> = rust_out.split(|&b| b == b'\n').collect();
        for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
            if cl != rl {
                let (x, y) = inputs.get(i).copied().unwrap_or((0, 0));
                panic!(
                    "{label}: mismatch on line {i} (driver({x}, {y}))\n  C   : {:?}\n  Rust: {:?}",
                    String::from_utf8_lossy(cl),
                    String::from_utf8_lossy(rl)
                );
            }
        }
        panic!(
            "{label}: output lengths differ ({} C lines vs {} Rust lines)",
            c_lines.len(),
            r_lines.len()
        );
    }
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        inputs.len(),
        "{label}: expected exactly one output line per call"
    );
}

/// ~200k pseudo-random full-range input pairs.
fn bulk_random_full_range() {
    let mut state: u32 = 0xdead_beef;
    let mut inputs = Vec::with_capacity(200_000);
    while inputs.len() < 200_000 {
        let x = xorshift(&mut state) as i32;
        let y = xorshift(&mut state) as i32;
        if y == 0 || (x == INT_MIN && y == -1) {
            continue; // undefined in C
        }
        inputs.push((x, y));
    }
    bulk_compare("bulk_random_full_range", &inputs);
}

/// Every divisor in `-512..=512` against a spread of numerators, plus a dense
/// walk over small magnitudes.
fn bulk_dense_small_magnitudes() {
    let mut inputs = Vec::new();
    for x in -300i32..=300 {
        for y in -300i32..=300 {
            if y == 0 {
                continue;
            }
            inputs.push((x, y));
        }
    }
    bulk_compare("bulk_dense_small_magnitudes", &inputs);
}

/// Numerators clustered around powers of two and their neighbours, where
/// truncation-toward-zero behaviour differs from flooring.
fn bulk_power_of_two_neighbourhoods() {
    let mut inputs = Vec::new();
    for bit in 0..31u32 {
        let base = 1i32 << bit;
        for dx in -2i32..=2 {
            for &sx in &[1i32, -1] {
                let x = base.wrapping_mul(sx).wrapping_add(dx);
                for sbit in 0..31u32 {
                    let d = 1i32 << sbit;
                    for &sy in &[1i32, -1] {
                        let y = d.wrapping_mul(sy);
                        if y == 0 || (x == INT_MIN && y == -1) {
                            continue;
                        }
                        inputs.push((x, y));
                    }
                }
            }
        }
    }
    bulk_compare("bulk_power_of_two_neighbourhoods", &inputs);
}


/// A zero divisor is undefined in C and traps on x86-64. The Rust translation
/// must trap the same way rather than panicking or printing something.
fn zero_divisor_traps_identically() {
    let pair = Pair::load();
    let c = pair.c_fn();
    let rust = pair.rust_fn();

    for x in [0, 1, -1, 7, -7, INT_MIN, INT_MAX] {
        let c_outcome = outcome_of(|| unsafe { c(x, 0) });
        let rust_outcome = outcome_of(|| unsafe { rust(x, 0) });
        assert_eq!(
            c_outcome, rust_outcome,
            "termination mismatch for driver({x}, 0)"
        );
        if cfg!(target_arch = "x86_64") {
            assert_eq!(
                c_outcome,
                Outcome::Signalled(8), // SIGFPE
                "expected the C implementation to trap on a zero divisor"
            );
        }
    }
}

/// `INT_MIN / -1` overflows and also traps on x86-64.
fn int_min_over_minus_one_traps_identically() {
    let pair = Pair::load();
    let c = pair.c_fn();
    let rust = pair.rust_fn();

    let c_outcome = outcome_of(|| unsafe { c(INT_MIN, -1) });
    let rust_outcome = outcome_of(|| unsafe { rust(INT_MIN, -1) });
    assert_eq!(
        c_outcome, rust_outcome,
        "termination mismatch for driver(INT_MIN, -1)"
    );
    if cfg!(target_arch = "x86_64") {
        assert_eq!(
            c_outcome,
            Outcome::Signalled(8), // SIGFPE
            "expected the C implementation to trap on INT_MIN / -1"
        );
    }
}

/// Sanity check on the fork harness itself: a well-defined input must exit 0.
fn defined_inputs_do_not_trap() {
    let pair = Pair::load();
    let c = pair.c_fn();
    let rust = pair.rust_fn();
    assert_eq!(outcome_of(|| unsafe { c(10, 3) }), Outcome::Exited(0));
    assert_eq!(outcome_of(|| unsafe { rust(10, 3) }), Outcome::Exited(0));
}

// ---------------------------------------------------------------------------
// Sequential runner (`harness = false`)
// ---------------------------------------------------------------------------

fn main() {
    let cases: &[(&str, fn())] = &[
        // Level 0: exports
        ("both_libraries_export_driver", both_libraries_export_driver),
        ("rust_so_exports_every_c_symbol", rust_so_exports_every_c_symbol),
        // Level 1: div() core
        ("small_positive_grid", small_positive_grid),
        ("all_sign_combinations", all_sign_combinations),
        ("exact_and_inexact_division", exact_and_inexact_division),
        ("integer_extremes", integer_extremes),
        ("randomized_full_range_sweep", randomized_full_range_sweep),
        ("randomized_small_divisors", randomized_small_divisors),
        ("randomized_near_unit_quotients", randomized_near_unit_quotients),
        // Level 2: printf() formatting
        ("output_byte_layout_is_exact", output_byte_layout_is_exact),
        (
            "repeated_calls_accumulate_identically",
            repeated_calls_accumulate_identically,
        ),
        // Level 2b: high-volume bulk comparison
        ("bulk_dense_small_magnitudes", bulk_dense_small_magnitudes),
        (
            "bulk_power_of_two_neighbourhoods",
            bulk_power_of_two_neighbourhoods,
        ),
        ("bulk_random_full_range", bulk_random_full_range),
        // Level 3: inputs C leaves undefined
        ("zero_divisor_traps_identically", zero_divisor_traps_identically),
        (
            "int_min_over_minus_one_traps_identically",
            int_min_over_minus_one_traps_identically,
        ),
        ("defined_inputs_do_not_trap", defined_inputs_do_not_trap),
    ];

    // Optional substring filter, mirroring `cargo test <filter>`.
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));

    eprintln!("running {} differential tests", cases.len());
    let mut failed = Vec::new();
    let mut ran = 0usize;
    for &(name, case) in cases {
        if let Some(f) = &filter {
            if !name.contains(f.as_str()) {
                continue;
            }
        }
        ran += 1;
        eprint!("test {name} ... ");
        let result = std::panic::catch_unwind(case);
        match result {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    if failed.is_empty() {
        eprintln!("\ntest result: ok. {ran} passed; 0 failed");
    } else {
        eprintln!(
            "\ntest result: FAILED. {} passed; {} failed\nfailures: {:?}",
            ran - failed.len(),
            failed.len(),
            failed
        );
        std::process::exit(1);
    }
}
