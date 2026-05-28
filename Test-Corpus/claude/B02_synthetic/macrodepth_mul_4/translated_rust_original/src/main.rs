// SPDX-License-Identifier: MIT
// Rust translation of c_src/src/mdmain.c
//
// Mirrors the C `main`, including parsing two integers from argv and printing
// the same lines in the same order so stdout is byte-identical.

use std::process::ExitCode;

use driver::driver_support;

/// Replicates the behavior of `atoi(3)`: skip leading whitespace, optional
/// sign, then consume decimal digits. Stops at the first non-digit. Numbers
/// outside the i32 range silently overflow (matching the C contract that
/// `atoi` returns int and the driver passes plain `int`s through).
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;

    // Skip whitespace as `isspace(3)` would.
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }

    let mut neg = false;
    if i < bytes.len() {
        match bytes[i] {
            b'+' => {
                i += 1;
            }
            b'-' => {
                neg = true;
                i += 1;
            }
            _ => {}
        }
    }

    let mut acc: i32 = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as i32;
        // `atoi` is allowed to wrap on overflow (undefined behavior in C, but
        // glibc just truncates).
        acc = acc.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }

    if neg {
        acc.wrapping_neg()
    } else {
        acc
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        // Mirror `fprintf(stderr, "usage: %s A B\n", argv[0])`.
        eprintln!("usage: {} A B", args[0]);
        return ExitCode::from(2);
    }

    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    // r_call = OP(a, b)
    let r_call = driver_support::op_call(a, b);

    // acc starts at INIT_<OP> and runs the unrolled REPEAT-step loop.
    let mut acc: i32 = driver_support::op_init();
    driver_support::run_loop(&mut acc);

    // helper_call/helper_ptr/use_generated print their own lines.
    let x1 = driver::helper_call(a, b);
    let x2 = driver::helper_ptr(a, b);
    let x3 = driver::use_generated(repeat_value());

    // g = G_OP(a, b)
    let g = unsafe { (driver::G_OP)(a, b) };

    println!(
        "op={} call={} acc={} g.call={}",
        driver_support::op_name_cstr(),
        r_call,
        acc,
        g
    );
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    println!("summary={}", summary);

    ExitCode::from(0)
}

// Mirror the compile-time selection of REPEAT for the `use_generated(REPEAT)` call.
fn repeat_value() -> i32 {
    #[cfg(feature = "repeat_0")]
    {
        return 0;
    }
    #[cfg(all(feature = "repeat_1", not(feature = "repeat_0")))]
    {
        return 1;
    }
    #[cfg(all(feature = "repeat_2", not(feature = "repeat_0"), not(feature = "repeat_1")))]
    {
        return 2;
    }
    #[cfg(all(
        feature = "repeat_3",
        not(feature = "repeat_0"),
        not(feature = "repeat_1"),
        not(feature = "repeat_2")
    ))]
    {
        return 3;
    }
    #[cfg(all(
        feature = "repeat_4",
        not(feature = "repeat_0"),
        not(feature = "repeat_1"),
        not(feature = "repeat_2"),
        not(feature = "repeat_3")
    ))]
    {
        return 4;
    }
    #[cfg(all(
        feature = "repeat_6",
        not(feature = "repeat_0"),
        not(feature = "repeat_1"),
        not(feature = "repeat_2"),
        not(feature = "repeat_3"),
        not(feature = "repeat_4"),
        not(feature = "repeat_5")
    ))]
    {
        return 6;
    }
    #[cfg(all(
        feature = "repeat_7",
        not(feature = "repeat_0"),
        not(feature = "repeat_1"),
        not(feature = "repeat_2"),
        not(feature = "repeat_3"),
        not(feature = "repeat_4"),
        not(feature = "repeat_5"),
        not(feature = "repeat_6")
    ))]
    {
        return 7;
    }
    #[cfg(all(
        not(feature = "repeat_0"),
        not(feature = "repeat_1"),
        not(feature = "repeat_2"),
        not(feature = "repeat_3"),
        not(feature = "repeat_4"),
        not(feature = "repeat_6"),
        not(feature = "repeat_7"),
    ))]
    {
        return 5;
    }
}
