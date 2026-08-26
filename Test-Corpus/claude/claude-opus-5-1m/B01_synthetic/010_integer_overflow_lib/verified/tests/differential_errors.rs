// Phase C — error/boundary-path differential tests.
//
// One test (or one clearly-labelled sub-case) per row of ERRORS.md.
//
// The C library has no error returns, no asserts, no range checks and no
// pointer/length/enum parameters (see ERRORS.md for the mechanical derivation),
// so "the same error/rejection" here means:
//   * for value boundaries: the same exact bytes on stdout, and additionally the
//     exact byte string the C source implies (so the table itself is validated,
//     not just C-vs-Rust agreement);
//   * for I/O failure injection: the same termination status and the same
//     (absence of) output, since `printf`'s return value is discarded by the C.

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------------------
// E1–E8 — value boundaries of the narrow-integer domain
// ---------------------------------------------------------------------------

/// Rows E1–E8.
///
/// `expected` is derived from the C source by hand (signed `char` promotion,
/// `%02x` semantics, `char` wraparound in `data + 1`) and is asserted against
/// the real C library first, so a wrong expectation fails loudly instead of
/// silently agreeing with a wrong Rust translation.
#[test]
fn err_e1_e6_boundary_char_values() {
    // (row, entry point, raw byte, expected stdout)
    let cases: &[(&str, Entry, u8, &[u8])] = &[
        // E1: minimum value / NUL, smallest %02x input -> zero padded.
        ("E1", Entry::PrintHex, 0x00, b"00\n"),
        // E2: CHAR_MAX, last non-negative char.
        ("E2", Entry::PrintHex, 0x7f, b"7f\n"),
        // E3: CHAR_MIN (-128) -> sign extended, 8 digits, width inert.
        ("E3", Entry::PrintHex, 0x80, b"ffffff80\n"),
        // E4: -1, maximal sign extension.
        ("E4", Entry::PrintHex, 0xff, b"ffffffff\n"),
        // E5: driver(CHAR_MAX): data+1 overflows char, wraps to -128.
        ("E5", Entry::Driver, 0x7f, b"ffffff80\n"),
        // E6: driver(-1): wraps the other way, to 0.
        ("E6", Entry::Driver, 0xff, b"00\n"),
        // E7: driver(CHAR_MIN): result -127, still negative.
        ("E7", Entry::Driver, 0x80, b"ffffff81\n"),
        // E8: the %02x padding-width boundary, both sides.
        ("E8", Entry::PrintHex, 0x0f, b"0f\n"),
        ("E8", Entry::PrintHex, 0x10, b"10\n"),
        ("E8", Entry::Driver, 0x0f, b"10\n"),
        ("E8", Entry::Driver, 0x0e, b"0f\n"),
    ];

    for &(row, entry, raw, expected) in cases {
        let v = raw as c_char;
        let label = format!("{row} {entry:?}({raw:#04x})");

        let c = capture(|| entry.call(c_api(), v));
        assert_eq!(
            c,
            expected,
            "{label}: ERRORS.md expectation disagrees with the C ground truth \
             (C produced {})",
            show(&c)
        );

        let r = capture(|| entry.call(rust_api(), v));
        assert_bytes_eq(&label, &c, &r);
    }
}

/// Both entry points, addressed uniformly.
#[derive(Copy, Clone, Debug)]
enum Entry {
    PrintHex,
    Driver,
}

impl Entry {
    fn call(self, api: &'static Api, v: c_char) {
        match self {
            Entry::PrintHex => api.print_hex_char_line(v),
            Entry::Driver => api.driver(v),
        }
    }
    fn call_int(self, api: &'static Api, v: i32) {
        match self {
            Entry::PrintHex => api.print_hex_char_line_int(v),
            Entry::Driver => api.driver_int(v),
        }
    }
}

// ---------------------------------------------------------------------------
// E9–E10 — out-of-range integer through the narrow-integer parameter
// ---------------------------------------------------------------------------

/// Rows E9 and E10.
///
/// The C prototype says `char`, but the SysV ABI passes it in a 32-bit register
/// and a C caller may legally hand over any `int` — this is exactly the
/// situation of an `enum` parameter receiving a value with no valid variant.
/// The callee must look only at the low 8 bits.
///
/// Asserted three ways: C == Rust, and each == the corresponding `char` call.
#[test]
fn err_e9_out_of_range_int_arg_via_ffi() {
    let mut inputs: Vec<i32> = Vec::new();

    // E10: exhaustive over the low byte with deliberately dirty high bits.
    for b in 0..=255u32 {
        inputs.push((0xDEAD_BE00u32 | b) as i32);
        inputs.push((0xFFFF_FF00u32 | b) as i32);
    }
    // E9: hand-picked values one (and many) steps past the `char` range.
    inputs.extend_from_slice(&[
        0x80,
        0xff,
        0x100,
        0x101,
        0x17f,
        0x180,
        0x1ff,
        -129,
        -128,
        -1,
        128,
        129,
        255,
        256,
        -1000,
        1000,
        65535,
        65536,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ]);
    let mut rng = Rng::new(SEED ^ 0x0E9E_0E9E);
    for _ in 0..1024 {
        inputs.push(rng.next_i32());
    }

    // Batched into one capture per (entry point, library, argument form) so the
    // whole 1500+ input set costs a handful of forks instead of thousands.
    for entry in [Entry::PrintHex, Entry::Driver] {
        // (a) C vs Rust, both called with the full-width `int` argument.
        assert_same_over_values(
            &format!("E9/E10 {entry:?}(int) C vs Rust"),
            &inputs,
            1,
            |api, v| entry.call_int(api, v),
        );

        // (b) each library must be self-consistent: passing the raw `int` must
        //     be indistinguishable from passing only its low byte as a `char`.
        let truncated: Vec<c_char> = inputs
            .iter()
            .map(|&v| (v as u32 & 0xff) as u8 as c_char)
            .collect();

        for (name, api) in [("C", c_api()), ("Rust", rust_api())] {
            let as_int = capture(|| {
                for &v in &inputs {
                    entry.call_int(api, v);
                }
            });
            let as_char = capture(|| {
                for &v in &truncated {
                    entry.call(api, v);
                }
            });
            assert_bytes_eq(
                &format!("E9/E10 {entry:?} {name}: int arg vs low-byte char arg"),
                &as_int,
                &as_char,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E11–E13 — stdout I/O failure injection
//
// `printf`'s return value is discarded by the C, so a failing write must be
// tolerated silently and identically by both implementations. These run in a
// forked child so a poisoned stdout error flag cannot leak into other tests.
// ---------------------------------------------------------------------------

fn assert_same_under_failing_stdout(what: &str, sink: Sink) {
    let vals: Vec<c_char> = [0x00u8, 0x01, 0x0f, 0x41, 0x7f, 0x80, 0xfe, 0xff]
        .iter()
        .map(|&b| b as c_char)
        .collect();

    let run = |api: &'static Api| {
        let vals = vals.clone();
        move || {
            for v in vals {
                api.print_hex_char_line(v);
                api.driver(v);
            }
        }
    };

    let c = capture_forked(BufMode::Default, sink, run(c_api()));
    let r = capture_forked(BufMode::Default, sink, run(rust_api()));

    assert_eq!(
        c.exited_with,
        Some(0),
        "{what}: the C library must tolerate the write failure, but the C child \
         did not exit 0: {c:?}"
    );
    assert_eq!(
        r.exited_with, c.exited_with,
        "{what}: exit status differs — C {:?} vs Rust {:?}",
        c.exited_with, r.exited_with
    );
    assert_eq!(
        r.signalled_with, c.signalled_with,
        "{what}: termination signal differs — C {:?} vs Rust {:?}",
        c.signalled_with, r.signalled_with
    );
    assert_eq!(c.signalled_with, None, "{what}: C child was signalled");
    assert_eq!(r.signalled_with, None, "{what}: Rust child was signalled");
    assert_bytes_eq(what, &c.out, &r.out);
}

/// Row E11 — `stdout` points at `/dev/full`: every `write()` fails `ENOSPC`.
#[test]
fn err_e11_stdout_write_fails_dev_full() {
    assert_same_under_failing_stdout("E11 /dev/full (ENOSPC)", Sink::DevFull);
}

/// Row E12 — fd 1 is a read-only descriptor: every `write()` fails `EBADF`.
#[test]
fn err_e12_stdout_fd_not_writable() {
    assert_same_under_failing_stdout("E12 read-only fd 1 (EBADF)", Sink::ReadOnly);
}

/// Row E13 — fd 1 closed outright before the call.
#[test]
fn err_e13_stdout_closed() {
    assert_same_under_failing_stdout("E13 fd 1 closed", Sink::Closed);
}

// ---------------------------------------------------------------------------
// Structural boundaries that are N/A for this API — asserted, not assumed.
//
// The generic C-API boundary checklist names null pointers, zero/oversized
// lengths, and out-of-range enum values. This test pins down mechanically that
// the public surface has no parameter of any such kind, so those rows are
// inapplicable by construction rather than merely untested. If a future change
// to the C introduces a pointer/length/enum parameter, this test starts failing
// and ERRORS.md must be revisited.
// ---------------------------------------------------------------------------

#[test]
fn err_na_rows_public_surface_has_no_pointer_length_or_enum_params() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src");
    let header = std::fs::read_to_string(root.join("include/driver.h")).expect("read driver.h");
    let source = std::fs::read_to_string(root.join("src/driver.c")).expect("read driver.c");

    // Strip // comments so the licence header cannot influence the checks.
    let strip = |s: &str| -> String {
        s.lines()
            .map(|l| match l.find("//") {
                Some(i) => &l[..i],
                None => l,
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let header = strip(&header);
    let source = strip(&source);
    let code = format!("{header}\n{source}");

    assert!(
        !code.contains('*'),
        "the C surface gained a pointer — the null-pointer rows in ERRORS.md are \
         no longer N/A"
    );
    assert!(
        !code.contains("enum"),
        "the C surface gained an enum — the out-of-range-enum rows in ERRORS.md \
         are no longer N/A"
    );
    for t in ["size_t", "ssize_t", "len", "count", "[]"] {
        assert!(
            !code.contains(t),
            "the C surface gained `{t}` — the length/size rows in ERRORS.md are \
             no longer N/A"
        );
    }
    // And the only two prototypes really are `void f(char)`.
    assert!(
        source.contains("void printHexCharLine (char charHex)"),
        "printHexCharLine signature changed"
    );
    assert!(
        source.contains("void driver(char data)"),
        "driver signature changed"
    );
    assert!(
        code.contains("return") == false,
        "the C gained a `return` statement — re-derive the error surface"
    );
    for kw in ["assert", "errno", "abort(", "exit("] {
        assert!(
            !code.contains(kw),
            "the C gained `{kw}` — re-derive the error surface in ERRORS.md"
        );
    }
}
