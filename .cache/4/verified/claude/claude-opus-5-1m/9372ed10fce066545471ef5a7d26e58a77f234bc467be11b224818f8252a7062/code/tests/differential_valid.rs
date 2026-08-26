// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH shared objects through
// their exported C symbols and compares the bytes they write to stdout.
//
// Tests start at the lowest-level entry point (`printHexCharLine`, the
// primitive that is not even declared in driver.h but is exported by the .so)
// and work up to the wrapper (`driver`) and to the composed pipeline.

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------------------
// Harness self-check.
//
// Without this, a broken capture would make every differential assertion
// compare "" with "" and the whole suite would pass vacuously.
// ---------------------------------------------------------------------------

#[test]
fn sanity_capture_actually_observes_output() {
    let c = capture(|| c_api().print_hex_char_line(0x41));
    assert_eq!(
        c, b"41\n",
        "capture harness is not observing the C library's stdout (got {})",
        show(&c)
    );

    let r = capture(|| rust_api().print_hex_char_line(0x41));
    assert_eq!(r, b"41\n", "capture harness did not observe Rust's stdout");

    // And a value that must NOT be empty and must differ from the above, so a
    // stuck/cached capture cannot masquerade as success.
    let c2 = capture(|| c_api().driver(0x7f));
    assert_eq!(
        c2, b"ffffff80\n",
        "C ground truth for driver(0x7f) is not what the source implies"
    );
    let r2 = capture(|| rust_api().driver(0x7f));
    assert_eq!(r2, b"ffffff80\n");
}

// ---------------------------------------------------------------------------
// C1–C5 : printHexCharLine, the low-level entry point
// ---------------------------------------------------------------------------

/// C1 — `%02x` zero-padding path, exhaustive over `0x00..=0x0F`.
#[test]
fn c1_print_hex_padding_class() {
    let vals: Vec<c_char> = chars_in(0x00, 0x0f).collect();
    assert_same_over_values("C1 printHexCharLine", &vals, 1, |api, v| {
        api.print_hex_char_line(v)
    });
}

/// C2 — positive, exactly two digits, exhaustive over `0x10..=0x7F`.
#[test]
fn c2_print_hex_two_digit_class() {
    let vals: Vec<c_char> = chars_in(0x10, 0x7f).collect();
    assert_same_over_values("C2 printHexCharLine", &vals, 1, |api, v| {
        api.print_hex_char_line(v)
    });
}

/// C3 — negative `char`, sign-extended to eight digits, exhaustive `0x80..=0xFF`.
#[test]
fn c3_print_hex_negative_sign_extended_class() {
    let vals: Vec<c_char> = chars_in(0x80, 0xff).collect();
    assert_same_over_values("C3 printHexCharLine", &vals, 1, |api, v| {
        api.print_hex_char_line(v)
    });
}

/// C4 — exhaustive over the entire `char` domain, one call per capture.
#[test]
fn c4_print_hex_exhaustive_domain() {
    let vals: Vec<c_char> = all_chars().collect();
    assert_same_over_values("C4 printHexCharLine exhaustive", &vals, 1, |api, v| {
        api.print_hex_char_line(v)
    });
}

/// C5 — the class boundaries on their own.
#[test]
fn c5_print_hex_boundaries() {
    let vals: Vec<c_char> = [0x00u8, 0x0f, 0x10, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff]
        .iter()
        .map(|&b| b as c_char)
        .collect();
    assert_same_over_values("C5 printHexCharLine boundaries", &vals, 1, |api, v| {
        api.print_hex_char_line(v)
    });
}

// ---------------------------------------------------------------------------
// C6–C12 : driver, the wrapper (data + 1, then the primitive)
// ---------------------------------------------------------------------------

/// C6 — `data ∈ 0x00..=0x0E`, result stays inside the padding class.
#[test]
fn c6_driver_result_padding_class() {
    let vals: Vec<c_char> = chars_in(0x00, 0x0e).collect();
    assert_same_over_values("C6 driver", &vals, 1, |api, v| api.driver(v));
}

/// C7 — `data == 0x0F`, result `0x10` crosses the padding boundary.
#[test]
fn c7_driver_crosses_padding_boundary() {
    assert_same("C7 driver(0x0f)", |api| api.driver(0x0f));
}

/// C8 — `data ∈ 0x10..=0x7E`, positive two-digit result.
#[test]
fn c8_driver_result_two_digit_class() {
    let vals: Vec<c_char> = chars_in(0x10, 0x7e).collect();
    assert_same_over_values("C8 driver", &vals, 1, |api, v| api.driver(v));
}

/// C9 — `data == 0x7F`: `data + 1` overflows the `char` range and wraps to -128.
#[test]
fn c9_driver_signed_overflow_wrap() {
    assert_same("C9 driver(0x7f)", |api| api.driver(0x7f));
}

/// C10 — `data ∈ 0x80..=0xFE`, negative result, eight digits.
#[test]
fn c10_driver_result_negative_class() {
    let vals: Vec<c_char> = chars_in(0x80, 0xfe).collect();
    assert_same_over_values("C10 driver", &vals, 1, |api, v| api.driver(v));
}

/// C11 — `data == 0xFF` (-1): wraps back to `0x00`.
#[test]
fn c11_driver_wraps_to_zero() {
    assert_same("C11 driver(0xff)", |api| api.driver(-1));
}

/// C12 — exhaustive over the entire `char` domain, one call per capture.
#[test]
fn c12_driver_exhaustive_domain() {
    let vals: Vec<c_char> = all_chars().collect();
    assert_same_over_values("C12 driver exhaustive", &vals, 1, |api, v| api.driver(v));
}

// ---------------------------------------------------------------------------
// C13 : both entry points on the same randomized value
// ---------------------------------------------------------------------------

/// C13 — 4096 seeded-random values; each value goes through the primitive and
/// the wrapper inside one capture, so the relationship between the two entry
/// points is compared, not just each one in isolation.
#[test]
fn c13_both_entry_points_randomized() {
    let mut rng = Rng::new(SEED);
    let vals: Vec<c_char> = (0..4096).map(|_| rng.next_i8()).collect();
    // 2 output lines per input: the primitive's, then the wrapper's.
    assert_same_over_values("C13 both entry points", &vals, 2, |api, v| {
        api.print_hex_char_line(v);
        api.driver(v);
    });
}

// ---------------------------------------------------------------------------
// C14–C15 : Axis C — out-of-range `int` in the narrow-integer parameter
// ---------------------------------------------------------------------------

fn dirty_int_inputs() -> Vec<i32> {
    let mut v: Vec<i32> = Vec::new();
    // Every low byte with non-zero high bits.
    for b in 0..=255u32 {
        v.push((0xDEAD_BE00u32 | b) as i32);
    }
    // Hand-picked out-of-range values.
    v.extend_from_slice(&[
        0x100,
        0x1ff,
        0x180,
        0x17f,
        256,
        257,
        -1000,
        1000,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        0x7fff_ff80u32 as i32,
        0x0000_ff00,
    ]);
    // Plus a seeded-random spread.
    let mut rng = Rng::new(SEED ^ 0xA5A5_A5A5);
    for _ in 0..512 {
        v.push(rng.next_i32());
    }
    v
}

/// C14 — `printHexCharLine` called with a full-width `int` argument.
#[test]
fn c14_print_hex_dirty_int_argument() {
    let vals = dirty_int_inputs();
    assert_same_over_values("C14 printHexCharLine(int)", &vals, 1, |api, v| {
        api.print_hex_char_line_int(v)
    });
}

/// C15 — `driver` called with a full-width `int` argument.
#[test]
fn c15_driver_dirty_int_argument() {
    let vals = dirty_int_inputs();
    assert_same_over_values("C15 driver(int)", &vals, 1, |api, v| api.driver_int(v));
}

// ---------------------------------------------------------------------------
// C16–C19 : Axis D — call sequencing and buffer accumulation
// ---------------------------------------------------------------------------

/// C16 — 1000 random calls to the primitive accumulated in one capture.
#[test]
fn c16_print_hex_sequence_accumulated() {
    let vals: Vec<c_char> = {
        let mut rng = Rng::new(SEED ^ 16);
        (0..1000).map(|_| rng.next_i8()).collect()
    };
    assert_same("C16 printHexCharLine x1000", |api| {
        for &v in &vals {
            api.print_hex_char_line(v);
        }
    });
}

/// C17 — 1000 random calls to the wrapper accumulated in one capture.
#[test]
fn c17_driver_sequence_accumulated() {
    let vals: Vec<c_char> = {
        let mut rng = Rng::new(SEED ^ 17);
        (0..1000).map(|_| rng.next_i8()).collect()
    };
    assert_same("C17 driver x1000", |api| {
        for &v in &vals {
            api.driver(v);
        }
    });
}

/// C18 — the two entry points interleaved in one capture: exercises the
/// composed pipeline and the shared stdio buffer together.
#[test]
fn c18_interleaved_entry_points() {
    let vals: Vec<c_char> = {
        let mut rng = Rng::new(SEED ^ 18);
        (0..1000).map(|_| rng.next_i8()).collect()
    };
    assert_same("C18 interleaved x1000", |api| {
        for (i, &v) in vals.iter().enumerate() {
            if i % 2 == 0 {
                api.driver(v);
            } else {
                api.print_hex_char_line(v);
            }
        }
    });
}

/// C19 — output far larger than `BUFSIZ`, forcing many underlying `write()`s
/// and flushes that land in the middle of the stream.
#[test]
fn c19_large_output_many_flushes() {
    let vals: Vec<c_char> = {
        let mut rng = Rng::new(SEED ^ 19);
        (0..20_000).map(|_| rng.next_i8()).collect()
    };
    let c = capture(|| {
        let api = c_api();
        for &v in &vals {
            api.driver(v);
            api.print_hex_char_line(v);
        }
    });
    let r = capture(|| {
        let api = rust_api();
        for &v in &vals {
            api.driver(v);
            api.print_hex_char_line(v);
        }
    });
    assert!(
        c.len() > 8192 * 4,
        "C19 expected to exceed BUFSIZ several times over, got {} bytes",
        c.len()
    );
    assert_bytes_eq("C19 large output", &c, &r);
}

// ---------------------------------------------------------------------------
// C20–C22 : Axis E — stdout buffering mode
// ---------------------------------------------------------------------------

fn assert_same_forked(what: &str, mode: BufMode) {
    let vals: Vec<c_char> = all_chars().collect();

    let run = |api: &'static Api| {
        let vals = vals.clone();
        move || {
            for v in vals {
                api.print_hex_char_line(v);
                api.driver(v);
            }
        }
    };

    let c = capture_forked(mode, Sink::TempFile, run(c_api()));
    let r = capture_forked(mode, Sink::TempFile, run(rust_api()));

    assert_eq!(
        c.exited_with,
        Some(0),
        "{what}: C child terminated abnormally: {c:?}"
    );
    assert_eq!(
        r.exited_with,
        Some(0),
        "{what}: Rust child terminated abnormally: {r:?}"
    );
    assert!(!c.out.is_empty(), "{what}: C child produced no output");
    assert_bytes_eq(what, &c.out, &r.out);
}

/// C20 — `stdout` unbuffered (`_IONBF`): one `write()` per `printf`.
#[test]
fn c20_stdout_unbuffered() {
    assert_same_forked("C20 _IONBF", BufMode::Unbuffered);
}

/// C21 — `stdout` line buffered (`_IOLBF`) with a 1-byte buffer.
#[test]
fn c21_stdout_line_buffered() {
    assert_same_forked("C21 _IOLBF", BufMode::LineBuffered);
}

/// C22 — `stdout` fully buffered with a tiny 8-byte buffer, so flushes land in
/// the middle of individual records.
#[test]
fn c22_stdout_fully_buffered_tiny() {
    assert_same_forked("C22 _IOFBF(8)", BufMode::FullyBufferedTiny);
}

// ---------------------------------------------------------------------------
// C23 : Axis E — pipe instead of a regular file
// ---------------------------------------------------------------------------

/// C23 — fd 1 is a pipe (non-seekable), exhaustive over the domain.
#[test]
fn c23_stdout_is_a_pipe() {
    let vals: Vec<c_char> = all_chars().collect();
    let c = capture_via_pipe(|| {
        let api = c_api();
        for &v in &vals {
            api.print_hex_char_line(v);
        }
    });
    let r = capture_via_pipe(|| {
        let api = rust_api();
        for &v in &vals {
            api.print_hex_char_line(v);
        }
    });
    assert!(!c.is_empty(), "C23 pipe capture saw no C output");
    assert_bytes_eq("C23 pipe", &c, &r);

    let c = capture_via_pipe(|| {
        let api = c_api();
        for &v in &vals {
            api.driver(v);
        }
    });
    let r = capture_via_pipe(|| {
        let api = rust_api();
        for &v in &vals {
            api.driver(v);
        }
    });
    assert_bytes_eq("C23 pipe driver", &c, &r);
}

// ---------------------------------------------------------------------------
// C24 : Axis F — load / call order independence
// ---------------------------------------------------------------------------

/// C24 — the exhaustive domain compared with the Rust library invoked first,
/// then with the C library invoked first. Catches any first-call or lazy-PLT
/// asymmetry between the two objects.
#[test]
fn c24_call_order_independence() {
    let vals: Vec<c_char> = all_chars().collect();

    // Rust invoked first, then C.
    assert_same_rust_first("C24 rust-first, exhaustive", |api| {
        for &v in &vals {
            api.print_hex_char_line(v);
            api.driver(v);
        }
    });

    // C invoked first, then Rust.
    assert_same_over_values("C24 c-first, exhaustive", &vals, 2, |api, v| {
        api.print_hex_char_line(v);
        api.driver(v);
    });
}
