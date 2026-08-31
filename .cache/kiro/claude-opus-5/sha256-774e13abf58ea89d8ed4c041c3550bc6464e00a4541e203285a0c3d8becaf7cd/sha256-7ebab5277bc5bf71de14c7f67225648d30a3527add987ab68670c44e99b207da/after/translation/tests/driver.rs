//! Level 2: `void driver(const int *data, int len)`
//!
//! `driver` copies its input into a local buffer and calls the internal static
//! `inner`, which self-applies `fma_array` and then prints one decimal value per
//! line via `printf`. Since `inner` is `static` in C it has no exported symbol;
//! it is covered here through `driver`'s observable behaviour: the bytes written
//! to stdout, and the guarantee that the caller's input buffer is untouched.
//!
//! Capturing stdout means retargeting the process-wide fd 1, so this binary
//! deliberately exposes a single `#[test]` entry point: with more than one test
//! running concurrently, libtest's own progress output would be captured too.

mod common;

use common::{EDGE_VALUES, IMPLS, Impl, Rng, capture_stdout, driver, show};

/// Calls `driver` from one implementation and returns `(stdout bytes, input
/// buffer afterwards)`.
fn run(which: Impl, data: &[i32], len: i32) -> (Vec<u8>, Vec<i32>) {
    let f = driver(which);
    let buf = data.to_vec();
    let out = capture_stdout(|| unsafe { f(buf.as_ptr(), len) });
    // Guard against foreign writers landing in the capture: `inner` only ever
    // emits decimal digits, minus signs and newlines.
    assert!(
        out.iter()
            .all(|&b| b.is_ascii_digit() || b == b'-' || b == b'\n'),
        "{which:?} capture contains unexpected bytes (stdout contamination): {:?}",
        String::from_utf8_lossy(&out)
    );
    (out, buf)
}

fn check(data: &[i32], len: i32) {
    let (c_out, c_in) = run(Impl::C, data, len);
    let (rust_out, rust_in) = run(Impl::Rust, data, len);

    assert_eq!(
        c_in,
        data,
        "C driver modified its const input: data={} len={len}",
        show(data)
    );
    assert_eq!(
        rust_in,
        data,
        "Rust driver modified its const input: data={} len={len}",
        show(data)
    );
    if c_out != rust_out {
        panic!(
            "driver stdout mismatch: len={len} data={}\n  C   ={:?}\n  Rust={:?}",
            show(data),
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out)
        );
    }
}

fn empty_input_prints_nothing() {
    let (c_out, _) = run(Impl::C, &[], 0);
    let (rust_out, _) = run(Impl::Rust, &[], 0);
    assert_eq!(c_out, b"", "C driver printed output for len=0");
    assert_eq!(rust_out, c_out, "driver stdout mismatch for len=0");
}

fn single_element() {
    for &v in EDGE_VALUES.iter() {
        check(&[v], 1);
    }
    let mut rng = Rng::new(0xd0_0001);
    for _ in 0..200 {
        check(&[rng.next_i32()], 1);
    }
}

fn small_lengths_small_values() {
    let mut rng = Rng::new(0xd0_0002);
    for len in 0..=20i32 {
        for _ in 0..20 {
            let data: Vec<i32> = (0..len as usize).map(|_| rng.next_small()).collect();
            check(&data, len);
        }
    }
}

fn small_lengths_full_range_values() {
    let mut rng = Rng::new(0xd0_0003);
    for len in 0..=20i32 {
        for _ in 0..20 {
            let data: Vec<i32> = (0..len as usize).map(|_| rng.next_i32()).collect();
            check(&data, len);
        }
    }
}

/// `x * x + x` for each interesting value, verifying the overflow wrap and the
/// exact `%d` rendering (including `-2147483648`).
fn edge_values_as_one_array() {
    check(&EDGE_VALUES, EDGE_VALUES.len() as i32);
}

fn values_whose_square_straddles_the_i32_boundary() {
    let data: Vec<i32> = (46330..46350).chain(-46350..-46330).collect();
    check(&data, data.len() as i32);
}

/// `len` smaller than the buffer: only the first `len` elements may be read or
/// printed.
fn len_shorter_than_buffer() {
    let mut rng = Rng::new(0xd0_0004);
    for _ in 0..50 {
        let data: Vec<i32> = (0..32).map(|_| rng.next_small()).collect();
        for len in [0i32, 1, 5, 16, 31, 32] {
            check(&data, len);
        }
    }
}

fn larger_arrays() {
    let mut rng = Rng::new(0xd0_0005);
    for len in [64i32, 255, 256, 257, 1000, 4096] {
        let data: Vec<i32> = (0..len as usize).map(|_| rng.next_small()).collect();
        check(&data, len);
        let data: Vec<i32> = (0..len as usize).map(|_| rng.next_i32()).collect();
        check(&data, len);
    }
}

/// Repeated calls must not accumulate state between invocations.
fn repeated_calls_are_independent() {
    let data = [1i32, 2, 3, 4, 5];
    let mut c_runs = Vec::new();
    let mut rust_runs = Vec::new();
    for _ in 0..5 {
        c_runs.push(run(Impl::C, &data, data.len() as i32).0);
        rust_runs.push(run(Impl::Rust, &data, data.len() as i32).0);
    }
    for w in c_runs.windows(2) {
        assert_eq!(w[0], w[1], "C driver is not stateless");
    }
    assert_eq!(c_runs, rust_runs, "driver stdout mismatch across repeats");
}

/// The C and Rust libraries share the process's `stdout`, so their output must
/// interleave in call order without one buffering behind the other.
fn interleaved_c_and_rust_calls_share_stdout_ordering() {
    let a = [1i32, 2];
    let b = [3i32, 4];
    let c_f = driver(Impl::C);
    let rust_f = driver(Impl::Rust);

    let interleaved = capture_stdout(|| unsafe {
        c_f(a.as_ptr(), 2);
        rust_f(b.as_ptr(), 2);
        c_f(b.as_ptr(), 2);
        rust_f(a.as_ptr(), 2);
    });
    let all_c = capture_stdout(|| unsafe {
        c_f(a.as_ptr(), 2);
        c_f(b.as_ptr(), 2);
        c_f(b.as_ptr(), 2);
        c_f(a.as_ptr(), 2);
    });
    assert_eq!(
        interleaved,
        all_c,
        "interleaved output differs from all-C output\n  mixed={:?}\n  allC ={:?}",
        String::from_utf8_lossy(&interleaved),
        String::from_utf8_lossy(&all_c)
    );
}

/// Independent cross-check of the expected formatting, derived directly from
/// the C source: `out[i] = data[i] * data[i] + data[i]`, then `printf("%d\n")`.
fn matches_reference_formatting() {
    let mut rng = Rng::new(0xd0_0006);
    for _ in 0..100 {
        let len = rng.range(20);
        let data: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        let expected: String = data
            .iter()
            .map(|&v| format!("{}\n", v.wrapping_mul(v).wrapping_add(v)))
            .collect();
        for which in IMPLS {
            let (out, _) = run(which, &data, len as i32);
            assert_eq!(
                String::from_utf8_lossy(&out),
                expected,
                "{which:?} formatting differs from reference for data={}",
                show(&data)
            );
        }
    }
}

#[test]
fn driver_matches_c() {
    empty_input_prints_nothing();
    single_element();
    small_lengths_small_values();
    small_lengths_full_range_values();
    edge_values_as_one_array();
    values_whose_square_straddles_the_i32_boundary();
    len_shorter_than_buffer();
    larger_arrays();
    repeated_calls_are_independent();
    interleaved_c_and_rust_calls_share_stdout_ordering();
    matches_reference_formatting();
}
