//! `parse_val` writes `errno` (it zeroes it, and `strtol` may set `ERANGE`),
//! which is observable by the caller after `driver` returns. The Rust port
//! hand-rolls `strtol` instead of calling libc, so the resulting `errno` must be
//! compared too, not just stdout.

mod common;

use common::{call_driver_errno, show, Impl};

fn check(input: &[u8], seed: i32) {
    let (c_out, c_errno) = call_driver_errno(Impl::C, input, seed);
    let (rust_out, rust_errno) = call_driver_errno(Impl::Rust, input, seed);
    if c_out != rust_out {
        panic!(
            "driver({:?}) output mismatch\n--- C ---\n{}\n--- Rust ---\n{}",
            show(input),
            show(&c_out),
            show(&rust_out)
        );
    }
    if c_errno != rust_errno {
        panic!(
            "driver({:?}) with errno seeded to {seed}: C left errno={c_errno}, Rust left errno={rust_errno}",
            show(input)
        );
    }
}

#[test]
fn errno_after_driver_matches() {
    let inputs: &[&[u8]] = &[
        b"0",
        b"5",
        b"-5",
        b"2147483647",
        b"-2147483648",
        b"2147483648",
        b"-2147483649",
        b"9223372036854775807",
        b"9223372036854775808",   // ERANGE
        b"-9223372036854775809",  // ERANGE
        b"99999999999999999999999999",
        b"-99999999999999999999999999",
        b"",
        b" ",
        b"abc",
        b"+",
        b"-",
        b"12abc",
        b"  -7  ",
        b"0x10",
    ];
    // Seed with 0, with ERANGE, with EINVAL and with an unrelated value so any
    // difference in whether errno is cleared or preserved shows up.
    for &seed in &[0i32, 34 /* ERANGE */, 22 /* EINVAL */, 4242] {
        for input in inputs {
            check(input, seed);
        }
    }
}
