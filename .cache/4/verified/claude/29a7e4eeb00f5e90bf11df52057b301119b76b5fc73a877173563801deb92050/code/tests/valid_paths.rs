//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1–C19). Both implementations are loaded
//! from their `.so` via `libloading` and driven exclusively through their
//! exported C symbols.

mod common;

use common::{assert_same, cstr, pair, Rng, CONTROL, FORMAT_DIRECTIVES, PRINTABLE};
use std::ffi::c_char;

/// Helper: one `printLine` call with the given payload, differentially compared.
fn check_line(label: &str, payload: &[u8]) {
    let s = cstr(payload);
    assert_same(label, |imp| unsafe { imp.print_line_raw(s.as_ptr()) });
}

// ---------------------------------------------------------------- C1
fn c1_print_line_empty_string() {
    check_line("printLine(\"\")", b"");
    // And the exact expected bytes, so the row is anchored to the C semantics
    // (`printf("%s\n", "")` -> a single newline), not just to agreement.
    let s = cstr(b"");
    let out = common::capture(|| unsafe { pair().c.print_line_raw(s.as_ptr()) });
    assert_eq!(out, b"\n", "C reference for empty string");
}

// ---------------------------------------------------------------- C2
fn c2_print_line_every_single_byte_value() {
    for b in 1u8..=255 {
        check_line(&format!("printLine(single byte 0x{b:02x})"), &[b]);
    }
}

// ---------------------------------------------------------------- C3
fn c3_print_line_random_short_ascii() {
    let mut rng = Rng::new();
    for i in 0..512 {
        let len = rng.range(1, 32);
        let payload = rng.bytes_from(PRINTABLE, len);
        check_line(&format!("printLine(random ascii #{i}, len {len})"), &payload);
    }
}

// ---------------------------------------------------------------- C4
fn c4_print_line_random_full_byte_alphabet() {
    let mut rng = Rng::with_seed(0xC4C4_C4C4_C4C4_C4C4);
    for i in 0..512 {
        let len = rng.range(1, 256);
        let payload = rng.nonzero_bytes(len);
        check_line(
            &format!("printLine(random 0x01..0xff #{i}, len {len})"),
            &payload,
        );
    }
}

// ---------------------------------------------------------------- C5
fn c5_print_line_random_control_bytes() {
    let mut rng = Rng::with_seed(0xC5C5_C5C5_C5C5_C5C5);
    for i in 0..256 {
        let len = rng.range(1, 64);
        let payload = rng.bytes_from(CONTROL, len);
        check_line(&format!("printLine(control bytes #{i}, len {len})"), &payload);
    }
}

// ---------------------------------------------------------------- C6
fn c6_print_line_format_directives() {
    // Each directive alone ...
    for d in FORMAT_DIRECTIVES {
        check_line("printLine(format directive)", d);
    }
    // ... and randomized mixtures of directives and literal text.
    let mut rng = Rng::with_seed(0xC6C6_C6C6_C6C6_C6C6);
    for i in 0..256 {
        let pieces = rng.range(1, 8);
        let mut payload = Vec::new();
        for _ in 0..pieces {
            payload.extend_from_slice(FORMAT_DIRECTIVES[rng.range(0, FORMAT_DIRECTIVES.len() - 1)]);
            let text_len = rng.range(0, 6);
            payload.extend_from_slice(&rng.bytes_from(PRINTABLE, text_len));
        }
        check_line(&format!("printLine(format mix #{i})"), &payload);
    }
}

// ---------------------------------------------------------------- C7
fn c7_print_line_embedded_newlines_and_tabs() {
    check_line("printLine(\"\\n\")", b"\n");
    check_line("printLine(\"a\\nb\")", b"a\nb");
    check_line("printLine(\"\\r\\n\")", b"\r\n");
    check_line("printLine(trailing newline)", b"line\n");
    check_line("printLine(leading newline)", b"\nline");

    let alphabet = b"ab\n\t\r ";
    let mut rng = Rng::with_seed(0xC7C7_C7C7_C7C7_C7C7);
    for i in 0..256 {
        let len = rng.range(1, 48);
        let payload = rng.bytes_from(alphabet, len);
        check_line(&format!("printLine(whitespace mix #{i})"), &payload);
    }
}

// ---------------------------------------------------------------- C8
fn c8_print_line_boundary_lengths() {
    let mut rng = Rng::with_seed(0xC8C8_C8C8_C8C8_C8C8);
    for len in [
        1usize, 2, 3, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 257, 511, 512, 1023, 1024,
        1025, 4095, 4096, 4097, 8191, 8192, 65535, 65536, 65537, 1 << 20,
    ] {
        let payload = rng.bytes_from(PRINTABLE, len);
        check_line(&format!("printLine(len {len})"), &payload);
        // Same length, but over the full byte alphabet.
        let payload = rng.nonzero_bytes(len);
        check_line(&format!("printLine(len {len}, raw bytes)"), &payload);
    }
}

// ---------------------------------------------------------------- C9
fn c9_print_line_interior_unaligned_pointers() {
    let mut rng = Rng::with_seed(0xC9C9_C9C9_C9C9_C9C9);
    // One big buffer; hand the library interior pointers at every alignment.
    let backing = cstr(&rng.bytes_from(PRINTABLE, 300));
    for off in 0..64usize {
        let p = unsafe { backing.as_ptr().add(off) };
        assert_same(&format!("printLine(interior offset {off})"), |imp| unsafe {
            imp.print_line_raw(p)
        });
    }
    // Pointer directly at the terminating NUL (empty tail).
    let p = unsafe { backing.as_ptr().add(300) };
    assert_same("printLine(pointer at terminator)", |imp| unsafe {
        imp.print_line_raw(p)
    });
}

// ---------------------------------------------------------------- C10
fn c10_print_line_many_calls_in_one_capture() {
    let mut rng = Rng::with_seed(0x1010_1010_1010_1010);
    let lines: Vec<Vec<c_char>> = (0..256)
        .map(|_| {
            let len = rng.range(0, 40);
            cstr(&rng.bytes_from(PRINTABLE, len))
        })
        .collect();
    assert_same("256 printLine calls, one capture", |imp| unsafe {
        for l in &lines {
            imp.print_line_raw(l.as_ptr());
        }
    });
}

// ---------------------------------------------------------------- C11
fn c11_bad_single_call_and_dead_helper_stays_dead() {
    assert_same("bad()", |imp| unsafe { imp.bad() });

    // Anchor to the C ground truth: `bad()` prints only its own banner; the
    // `static helperBad()` in driver.c is dead code and must never run.
    let p = pair();
    let c_out = common::capture(|| unsafe { p.c.bad() });
    let rs_out = common::capture(|| unsafe { p.rust.bad() });
    assert_eq!(c_out, b"bad()\n", "C reference output of bad()");
    assert_eq!(rs_out, b"bad()\n", "Rust output of bad()");
    for out in [&c_out, &rs_out] {
        assert!(
            !out.windows(11).any(|w| w == b"helperBad()"),
            "helperBad() must never be called"
        );
    }
}

// ---------------------------------------------------------------- C12
fn c12_bad_repeated() {
    assert_same("bad() x64", |imp| unsafe {
        for _ in 0..64 {
            imp.bad();
        }
    });
}

// ---------------------------------------------------------------- C13
fn c13_good_single_call_uses_static_helper() {
    assert_same("good()", |imp| unsafe { imp.good() });

    let p = pair();
    let c_out = common::capture(|| unsafe { p.c.good() });
    assert_eq!(
        c_out, b"good()\nhelperGood()\n",
        "C reference output of good()"
    );
}

// ---------------------------------------------------------------- C14
fn c14_good_repeated() {
    assert_same("good() x64", |imp| unsafe {
        for _ in 0..64 {
            imp.good();
        }
    });
}

// ---------------------------------------------------------------- C15
fn c15_driver_end_to_end() {
    assert_same("driver()", |imp| unsafe { imp.driver() });

    let p = pair();
    let c_out = common::capture(|| unsafe { p.c.driver() });
    let expected: &[u8] = b"Calling good()...\ngood()\nhelperGood()\nFinished good()\n\
Calling bad()...\nbad()\nFinished bad()\n";
    assert_eq!(c_out, expected, "C reference output of driver()");
}

// ---------------------------------------------------------------- C16
fn c16_driver_repeated_no_state_leak() {
    assert_same("driver() x16", |imp| unsafe {
        for _ in 0..16 {
            imp.driver();
        }
    });
    // Each invocation must be identical to the first (no hidden state).
    let p = pair();
    let one = common::capture(|| unsafe { p.rust.driver() });
    let sixteen = common::capture(|| unsafe {
        for _ in 0..16 {
            p.rust.driver();
        }
    });
    assert_eq!(sixteen, one.repeat(16), "driver() is not idempotent");
}

// ---------------------------------------------------------------- C17
fn c17_random_interleaved_sequences() {
    let mut rng = Rng::with_seed(0x1717_1717_1717_1717);
    for seq in 0..64 {
        let n = rng.range(1, 12);
        // Pre-generate the plan so both implementations run the same one.
        let plan: Vec<(usize, Vec<c_char>)> = (0..n)
            .map(|_| {
                let which = rng.range(0, 3);
                let len = rng.range(0, 24);
                (which, cstr(&rng.bytes_from(PRINTABLE, len)))
            })
            .collect();
        assert_same(&format!("interleaved sequence #{seq}"), |imp| unsafe {
            for (which, payload) in &plan {
                match which {
                    0 => imp.print_line_raw(payload.as_ptr()),
                    1 => imp.good(),
                    2 => imp.bad(),
                    _ => imp.driver(),
                }
            }
        });
    }
}

// ---------------------------------------------------------------- C18
fn c18_null_interleaved_with_valid_calls() {
    let mut rng = Rng::with_seed(0x1818_1818_1818_1818);
    for seq in 0..64 {
        let n = rng.range(1, 10);
        let plan: Vec<(usize, Vec<c_char>)> = (0..n)
            .map(|_| {
                let which = rng.range(0, 4);
                let len = rng.range(0, 16);
                (which, cstr(&rng.bytes_from(PRINTABLE, len)))
            })
            .collect();
        assert_same(&format!("null-interleaved sequence #{seq}"), |imp| unsafe {
            for (which, payload) in &plan {
                match which {
                    0 => imp.print_line_raw(std::ptr::null()),
                    1 => imp.print_line_raw(payload.as_ptr()),
                    2 => imp.good(),
                    3 => imp.bad(),
                    _ => imp.driver(),
                }
            }
        });
    }
}

// ---------------------------------------------------------------- C19
fn c19_flush_ordering_one_capture_vs_many() {
    let p = pair();
    // Many small captures concatenated must equal one big capture: proves both
    // libraries flush the shared, fully buffered stdout the same way and leave
    // nothing stranded in the FILE buffer between calls.
    for imp in [&p.c, &p.rust] {
        let split: Vec<u8> = [
            common::capture(|| unsafe { imp.driver() }),
            common::capture(|| unsafe { imp.good() }),
            common::capture(|| unsafe { imp.bad() }),
            common::capture(|| unsafe {
                let s = cstr(b"tail");
                imp.print_line_raw(s.as_ptr())
            }),
        ]
        .concat();
        let joined = common::capture(|| unsafe {
            imp.driver();
            imp.good();
            imp.bad();
            let s = cstr(b"tail");
            imp.print_line_raw(s.as_ptr());
        });
        assert_eq!(
            split,
            joined,
            "{}: buffering/flush ordering differs between split and joined captures",
            imp.name
        );
    }
    // And the two implementations agree on the joined form.
    assert_same("joined multi-entry-point capture", |imp| unsafe {
        imp.driver();
        imp.good();
        imp.bad();
        let s = cstr(b"tail");
        imp.print_line_raw(s.as_ptr());
    });
}

fn main() {
    let mut r = common::Runner::new("valid_paths (Phase B / CONFIGS.md)");
    r.case("c1_print_line_empty_string", c1_print_line_empty_string);
    r.case("c2_print_line_every_single_byte_value", c2_print_line_every_single_byte_value);
    r.case("c3_print_line_random_short_ascii", c3_print_line_random_short_ascii);
    r.case("c4_print_line_random_full_byte_alphabet", c4_print_line_random_full_byte_alphabet);
    r.case("c5_print_line_random_control_bytes", c5_print_line_random_control_bytes);
    r.case("c6_print_line_format_directives", c6_print_line_format_directives);
    r.case("c7_print_line_embedded_newlines_and_tabs", c7_print_line_embedded_newlines_and_tabs);
    r.case("c8_print_line_boundary_lengths", c8_print_line_boundary_lengths);
    r.case("c9_print_line_interior_unaligned_pointers", c9_print_line_interior_unaligned_pointers);
    r.case("c10_print_line_many_calls_in_one_capture", c10_print_line_many_calls_in_one_capture);
    r.case("c11_bad_single_call_and_dead_helper_stays_dead", c11_bad_single_call_and_dead_helper_stays_dead);
    r.case("c12_bad_repeated", c12_bad_repeated);
    r.case("c13_good_single_call_uses_static_helper", c13_good_single_call_uses_static_helper);
    r.case("c14_good_repeated", c14_good_repeated);
    r.case("c15_driver_end_to_end", c15_driver_end_to_end);
    r.case("c16_driver_repeated_no_state_leak", c16_driver_repeated_no_state_leak);
    r.case("c17_random_interleaved_sequences", c17_random_interleaved_sequences);
    r.case("c18_null_interleaved_with_valid_calls", c18_null_interleaved_with_valid_calls);
    r.case("c19_flush_ordering_one_capture_vs_many", c19_flush_ordering_one_capture_vs_many);
    r.finish();
}
