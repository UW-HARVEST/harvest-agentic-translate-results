// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::env;
use std::process::ExitCode;

/// Represents which logical pointer is currently the "running_sum".
/// In the C code, `running_sum` may point either to the local
/// `initial_value` in `main` or to the function-static `inner` in
/// `static_alias`.
#[derive(Clone, Copy)]
enum SumRef {
    Initial,
    Inner,
}

struct State {
    initial_value: i32,
    inner: i32,
}

/// Mirrors the C function:
///   int *static_alias(int *outer) {
///     static int inner = 1;
///     if (*outer >= inner) { inner += *outer; return &inner; }
///     else                 { *outer += inner; return outer;  }
///   }
///
/// Because in C, `outer` may alias `&inner` (after the first call which
/// returns `&inner` and that pointer is passed back in), we must read
/// `*outer` BEFORE mutating `inner`, just like C's `+=` does.
fn static_alias(state: &mut State, current: SumRef) -> SumRef {
    // Read *outer first.
    let outer_val: i32 = match current {
        SumRef::Initial => state.initial_value,
        SumRef::Inner => state.inner,
    };

    if outer_val >= state.inner {
        // `inner += *outer`
        state.inner = state.inner.wrapping_add(outer_val);
        SumRef::Inner
    } else {
        // `*outer += inner`
        // If `current` were Inner, then outer_val == state.inner, which
        // contradicts the branch condition; so `current` must be Initial.
        match current {
            SumRef::Initial => {
                state.initial_value = state.initial_value.wrapping_add(state.inner);
            }
            SumRef::Inner => unreachable!(),
        }
        current
    }
}

/// A close approximation of C's `strtol(s, &end, 10)` for the purposes
/// of this program. Returns the parsed value (truncated to i32 like the
/// C code which assigns `long` into `int`) and the byte offset where
/// parsing stopped. If no digits were parsed, the returned offset is 0
/// (matching `end == argv[i]`).
fn c_strtol(s: &str) -> (i32, usize) {
    let bytes = s.as_bytes();
    let mut i: usize = 0;

    // C strtol skips leading whitespace per isspace().
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }

    let digit_start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }

    if i == digit_start {
        // No digits consumed -> *endptr = nptr, return 0.
        return (0, 0);
    }

    if negative {
        val = val.wrapping_neg();
    }

    // Mirror the C `int x = strtol(...);` truncation from long to int.
    (val as i32, i)
}

fn run() -> i32 {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 3 {
        println!("Error: should only be two (integer) arguments!");
        return 1;
    }

    let (initial_value, end1) = c_strtol(&args[1]);
    if end1 == 0 {
        println!("Error: first argument must be an integer!");
        return 1;
    }

    let (iterations, end2) = c_strtol(&args[2]);
    if end2 == 0 {
        println!("Error: second argument must be an integer!");
        return 1;
    }

    let mut state = State {
        initial_value,
        inner: 1,
    };
    let mut current = SumRef::Initial;

    let mut i: i32 = 0;
    while i < iterations {
        current = static_alias(&mut state, current);
        let v = match current {
            SumRef::Initial => state.initial_value,
            SumRef::Inner => state.inner,
        };
        println!("{}", v);
        i = i.wrapping_add(1);
    }

    0
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
