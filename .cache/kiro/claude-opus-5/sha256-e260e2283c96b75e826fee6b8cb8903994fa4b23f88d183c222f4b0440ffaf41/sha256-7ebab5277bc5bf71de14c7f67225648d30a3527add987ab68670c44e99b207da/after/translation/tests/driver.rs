//! Top of the call hierarchy: `void driver(int data)`.
//!
//! The C builds a 99 byte run of `'A'` in `source`, `strncpy`s `data` bytes of
//! it into a zeroed 100 byte `dest` when `data < 100`, writes `dest[data] = 0`,
//! and prints the result. Note the boundary that is deliberately preserved:
//! `data == 99` copies 99 bytes and then terminates at index 99, the last
//! in-bounds byte of `dest`.
//!
//! Negative `data` is not exercised: `strncpy` would receive `(size_t)data`,
//! an enormous count, and `dest[data]` would index before the buffer. The Rust
//! reproduces that undefined behaviour by construction (`data as usize`, then
//! `offset(data as isize)`), but it cannot be observed without crashing the
//! process, so there is nothing to compare.
//!
//! Driven from a single `#[test]` because capturing output redirects the
//! process-wide file descriptor 1; see `print_line.rs` for the rationale.

mod common;

use common::{capture_stdout, driver_fns, show};

/// Calls both `driver`s with `data` and reports any difference.
fn check(data: i32) -> Result<(), String> {
    let (c_fn, rust_fn) = driver_fns();

    let c_out = capture_stdout(|| unsafe { c_fn(data) });
    let rust_out = capture_stdout(|| unsafe { rust_fn(data) });

    if c_out == rust_out {
        Ok(())
    } else {
        Err(format!(
            "driver({data}) mismatch\n    C:    {}\n    Rust: {}",
            show(&c_out),
            show(&rust_out)
        ))
    }
}

/// Pins the expected bytes down independently, so the two libraries cannot
/// agree on the wrong answer.
fn check_against_expected(data: i32, expected: &[u8]) -> Result<(), String> {
    let (c_fn, rust_fn) = driver_fns();

    let c_out = capture_stdout(|| unsafe { c_fn(data) });
    let rust_out = capture_stdout(|| unsafe { rust_fn(data) });

    if c_out != expected {
        return Err(format!(
            "C driver({data}) produced {}, expected {}",
            show(&c_out),
            show(expected)
        ));
    }
    if c_out != rust_out {
        return Err(format!(
            "driver({data}) mismatch\n    C:    {}\n    Rust: {}",
            show(&c_out),
            show(&rust_out)
        ));
    }
    Ok(())
}

/// `driver` uses only stack buffers, so a run of calls must not accumulate
/// state on either side.
fn case_repeated_calls() -> Result<(), String> {
    let (c_fn, rust_fn) = driver_fns();
    let sequence = [99, 0, 5, 100, 5, 0, 99, 1, 100, 42];

    let c_out = capture_stdout(|| {
        for d in sequence {
            unsafe { c_fn(d) };
        }
    });
    let rust_out = capture_stdout(|| {
        for d in sequence {
            unsafe { rust_fn(d) };
        }
    });

    if c_out != rust_out {
        return Err(format!(
            "repeated call sequence mismatch\n    C:    {}\n    Rust: {}",
            show(&c_out),
            show(&rust_out)
        ));
    }
    Ok(())
}

/// Both libraries write through the same libc `stdout`, so alternating between
/// them inside one capture must interleave in call order.
fn case_shared_stdout_ordering() -> Result<(), String> {
    let (c_fn, rust_fn) = driver_fns();

    let out = capture_stdout(|| unsafe {
        c_fn(3);
        rust_fn(3);
        c_fn(100);
        rust_fn(100);
    });

    if out != b"AAA\nAAA\n\n\n" {
        return Err(format!("unexpected interleaving {}", show(&out)));
    }
    Ok(())
}

#[test]
fn driver_matches_c() {
    let mut failures: Vec<String> = Vec::new();
    let mut record = |name: &str, r: Result<(), String>| {
        if let Err(e) = r {
            failures.push(format!("[{name}] {e}"));
        }
    };

    // Exhaustive over the whole `data < 100` branch for non-negative inputs.
    for data in 0..100 {
        record("in_range", check(data));

        let mut expected = vec![b'A'; data as usize];
        expected.push(b'\n');
        record("in_range_expected", check_against_expected(data, &expected));
    }

    // `data >= 100` skips the copy, leaving `dest` zeroed, so an empty line is
    // printed.
    for data in [100, 101, 128, 255, 256, 1000, 65536, i32::MAX - 1, i32::MAX] {
        record("out_of_range", check_against_expected(data, b"\n"));
    }

    record("repeated_calls", case_repeated_calls());
    record("shared_stdout_ordering", case_shared_stdout_ordering());

    assert!(
        failures.is_empty(),
        "{} driver case(s) differ:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
