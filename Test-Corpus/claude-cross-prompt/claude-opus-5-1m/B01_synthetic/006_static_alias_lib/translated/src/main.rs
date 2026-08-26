// Rust translation of c_src/src/staticalias.c
// Preserves the exact behavior, including the static variable in static_alias
// and the pointer aliasing pattern used by driver.

use std::io::{self, Read, Write, BufWriter};

static mut INNER: i32 = 1;

/// Translation of:
///
/// ```c
/// int *static_alias(int *outer) {
///     static int inner = 1;
///     if (*outer >= inner) {
///         inner += *outer;
///         return &inner;
///     } else {
///         *outer += inner;
///         return outer;
///     }
/// }
/// ```
unsafe fn static_alias(outer: *mut i32) -> *mut i32 {
    unsafe {
        if *outer >= INNER {
            INNER += *outer;
            std::ptr::addr_of_mut!(INNER)
        } else {
            *outer += INNER;
            outer
        }
    }
}

/// Translation of:
///
/// ```c
/// void driver(int initial_value, int iterations) {
///     int *running_sum = &initial_value;
///     for (int i = 0; i < iterations; i++) {
///         running_sum = static_alias(running_sum);
///         printf("%d\n", *running_sum);
///     }
/// }
/// ```
fn driver<W: Write>(initial_value: i32, iterations: i32, out: &mut W) {
    // `initial_value` is a parameter (a local in the C version);
    // we take its address and let static_alias possibly modify it.
    let mut local = initial_value;
    let mut running_sum: *mut i32 = &mut local as *mut i32;
    for _ in 0..iterations {
        unsafe {
            running_sum = static_alias(running_sum);
            writeln!(out, "{}", *running_sum).unwrap();
        }
    }
}

fn main() {
    // The original C is a library; this driver mirrors a typical usage
    // by reading `initial_value` and `iterations` from stdin (scanf-style:
    // whitespace-separated, including across newlines).
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut iter = input.split_ascii_whitespace();

    let initial_value: i32 = match iter.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => 0,
    };
    let iterations: i32 = match iter.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => 0,
    };

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    driver(initial_value, iterations, &mut out);
    out.flush().unwrap();
}
