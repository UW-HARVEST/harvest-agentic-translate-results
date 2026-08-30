//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Every function in the library returns `void` and there are no error codes, so
//! the "rejection" is expressed as *the exact bytes written to stdout* (for E1:
//! none at all) plus "returns normally".  Both libraries are exercised through
//! their `.so` exports and compared byte-for-byte, so we assert the *same*
//! rejection, not merely "both did something".

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// E1 / G1 — printLine(NULL)
// ---------------------------------------------------------------------------

#[test]
fn e1_print_line_null() {
    // The single rejection in the whole C library: `if(line != NULL)`.
    let c_out = capture(|| apply(c_api(), &[Op::PrintLineNull]));
    let r_out = capture(|| apply(rust_api(), &[Op::PrintLineNull]));
    assert_eq!(c_out, r_out, "printLine(NULL) diverged");
    assert!(
        c_out.is_empty(),
        "printLine(NULL) must emit nothing, C emitted {:?}",
        String::from_utf8_lossy(&c_out)
    );
}

#[test]
fn g1_print_line_null_interleaved_with_valid_calls() {
    // NULL must be a pure no-op even in the middle of a stream, i.e. it must not
    // emit a stray newline or disturb the buffer.
    let ops = vec![
        Op::PrintLine(b"before".to_vec()),
        Op::PrintLineNull,
        Op::PrintLineNull,
        Op::PrintIntLine(7),
        Op::PrintLineNull,
        Op::PrintLine(b"after".to_vec()),
    ];
    let out = diff("G1 null interleaved", &ops);
    assert_eq!(out, b"before\n7\nafter\n");

    for mode in [
        Buffering::Default,
        Buffering::Unbuffered,
        Buffering::LineBuffered,
        Buffering::FullyBuffered,
    ] {
        let out = diff_with("G1 null interleaved", mode, &ops);
        assert_eq!(out, b"before\n7\nafter\n", "mode {mode:?}");
    }

    // Many NULLs in a row.
    let many: Vec<Op> = (0..1000).map(|_| Op::PrintLineNull).collect();
    let out = diff("G1 1000 nulls", &many);
    assert!(out.is_empty());
}

// ---------------------------------------------------------------------------
// G2 — zero-length string
// ---------------------------------------------------------------------------

#[test]
fn g2_print_line_zero_length() {
    let out = diff("G2 empty", &[Op::PrintLine(Vec::new())]);
    assert_eq!(out, b"\n");

    // repeated empties
    let ops: Vec<Op> = (0..256).map(|_| Op::PrintLine(Vec::new())).collect();
    let out = diff("G2 256 empties", &ops);
    assert_eq!(out.len(), 256);
    assert!(out.iter().all(|&b| b == b'\n'));
}

// ---------------------------------------------------------------------------
// G3 — oversized length
// ---------------------------------------------------------------------------

#[test]
fn g3_print_line_oversized() {
    let mut rng = Rng::new(SEED ^ 0xA3);
    for len in [1usize << 16, 1 << 20] {
        let payload = rng.bytes_any(len);
        let out = diff(&format!("G3 len={len}"), &[Op::PrintLine(payload.clone())]);
        assert_eq!(out.len(), len + 1);
        assert_eq!(&out[..len], &payload[..]);
    }
}

// ---------------------------------------------------------------------------
// G4 — format-specifier-looking payload (must never be interpreted)
// ---------------------------------------------------------------------------

#[test]
fn g4_print_line_format_specifiers_are_data() {
    // `%n` is the dangerous one: if either implementation ever passed `line` as
    // the *format* string, this would write through a bogus pointer / abort.
    let payloads: &[&[u8]] = &[
        b"%n",
        b"%n%n%n%n%n%n%n%n",
        b"%s",
        b"%s%s%s%s%s%s%s%s%s%s%s%s",
        b"%d %i %u %x %X %o %f %e %g %a %c %p %%",
        b"%1$s %2$n",
        b"%2147483647d",
        b"%-2147483648d",
        b"%.2147483647f",
        b"%*d",
        b"%hn %hhn %ln %lln %zn",
        b"AAAA%08x.%08x.%08x.%08x.%n",
    ];
    for p in payloads {
        let out = diff(
            &format!("G4 {:?}", String::from_utf8_lossy(p)),
            &[Op::PrintLine(p.to_vec())],
        );
        let mut want = p.to_vec();
        want.push(b'\n');
        assert_eq!(
            out,
            want,
            "format-looking payload {:?} must be printed verbatim",
            String::from_utf8_lossy(p)
        );
    }
}

// ---------------------------------------------------------------------------
// G5 — all non-NUL byte values, including invalid UTF-8
// ---------------------------------------------------------------------------

#[test]
fn g5_print_line_all_byte_values() {
    // one call per byte
    for b in 1u8..=255 {
        let out = diff(&format!("G5 single {b:#04x}"), &[Op::PrintLine(vec![b])]);
        assert_eq!(out, vec![b, b'\n']);
    }
    // one call containing every non-NUL byte, i.e. wildly invalid UTF-8
    let all: Vec<u8> = (1u8..=255).collect();
    let out = diff("G5 all bytes", &[Op::PrintLine(all.clone())]);
    let mut want = all.clone();
    want.push(b'\n');
    assert_eq!(out, want);

    // lone UTF-8 continuation bytes / truncated sequences
    let broken: &[&[u8]] = &[
        &[0x80],
        &[0xC3],
        &[0xE2, 0x82],
        &[0xF0, 0x9F, 0x98],
        &[0xFF, 0xFE],
        &[0xED, 0xA0, 0x80], // surrogate
        &[0xC0, 0xAF],       // overlong
    ];
    for b in broken {
        let out = diff("G5 broken utf8", &[Op::PrintLine(b.to_vec())]);
        let mut want = b.to_vec();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// ---------------------------------------------------------------------------
// G6 — embedded control characters
// ---------------------------------------------------------------------------

#[test]
fn g6_print_line_embedded_control_chars() {
    let payloads: &[&[u8]] = &[b"\n", b"\r", b"\t", b"a\nb\nc", b"\r\n\r\n", b"\x7f\x1b"];
    for p in payloads {
        let out = diff("G6 control", &[Op::PrintLine(p.to_vec())]);
        let mut want = p.to_vec();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// ---------------------------------------------------------------------------
// G7 / G8 / G9 — int boundary values
// ---------------------------------------------------------------------------

#[test]
fn g7_print_int_line_int_min() {
    let out = diff("G7 INT_MIN", &[Op::PrintIntLine(i32::MIN)]);
    assert_eq!(out, b"-2147483648\n");
}

#[test]
fn g8_print_int_line_int_max() {
    let out = diff("G8 INT_MAX", &[Op::PrintIntLine(i32::MAX)]);
    assert_eq!(out, b"2147483647\n");
}

#[test]
fn g9_print_int_line_boundaries() {
    let mut vals: Vec<i32> = vec![0, -1, 1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        for d in [-1i64, 0, 1] {
            vals.push((p + d) as i32);
            vals.push(-(p + d) as i32);
        }
        p *= 10;
    }
    for k in 0..32u32 {
        let q = 1i64 << k;
        for d in [-1i64, 0, 1] {
            vals.push((q + d) as i32);
            vals.push((-(q + d)) as i32);
        }
    }
    for v in vals {
        let out = diff(&format!("G9 {v}"), &[Op::PrintIntLine(v)]);
        let want = format!("{v}\n").into_bytes();
        assert_eq!(out, want, "printIntLine({v})");
    }
}

// ---------------------------------------------------------------------------
// G10 — wider-than-`int` value pushed through the `int` ABI slot
// ---------------------------------------------------------------------------

#[test]
fn g10_print_int_line_wide_arg_truncation() {
    let mut rng = Rng::new(SEED ^ 0xB0);
    let mut vals: Vec<i64> = vec![
        i32::MAX as i64 + 1, // one step past the valid range of `int`
        i32::MIN as i64 - 1,
        i64::MAX,
        i64::MIN,
        0x0000_0001_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        -1,
        0xFFFF_FFFF,
        0x1_2345_6789,
    ];
    for _ in 0..256 {
        vals.push(rng.next_u64() as i64);
    }
    for v in vals {
        let out = diff(&format!("G10 {v:#x}"), &[Op::PrintIntLineWide(v)]);
        // Both must truncate to the same low 32 bits.
        let want = format!("{}\n", v as i32).into_bytes();
        assert_eq!(out, want, "printIntLine(wide {v:#x})");
    }
}

// ---------------------------------------------------------------------------
// G11 — extra register arguments across the FFI boundary to `void (void)`
// ---------------------------------------------------------------------------

#[test]
fn g11_void_functions_ignore_extra_args() {
    let mut rng = Rng::new(SEED ^ 0xB1);
    for _ in 0..64 {
        let (a, b, c) = (
            rng.next_i32() as c_int,
            rng.next_i32() as c_int,
            rng.next_i32() as c_int,
        );
        let out = diff("G11 bad", &[Op::BadExtraArgs(a, b, c)]);
        assert_eq!(out, b"0\n0\n");
        let out = diff("G11 good", &[Op::GoodExtraArgs(a, b, c)]);
        assert_eq!(out, b"0\n2\n");
        let out = diff("G11 driver", &[Op::DriverExtraArgs(a, b, c)]);
        assert_eq!(
            out,
            b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n"
        );
    }
}

// ---------------------------------------------------------------------------
// G12 — the out-of-range-enum class, explicitly discharged
// ---------------------------------------------------------------------------

#[test]
fn g12_no_enum_or_flag_parameters_exist() {
    // The C API has no enum / flag / mode parameter (see CONFIGS.md), so the
    // "int with no valid variant" class degenerates into "any i32", which is
    // fully valid input.  We nevertheless push the *entire* i32 space at the
    // only int-taking entry point, including values that would be nonsense for
    // any hypothetical enum, and require exact agreement.
    let mut rng = Rng::new(SEED ^ 0xB2);
    let mut vals: Vec<i32> = vec![-2, -1, 0, 1, 2, 3, 4, 5, 42, 255, 256, 65535, -65536];
    vals.extend((0..1024).map(|_| rng.next_i32()));
    for chunk in vals.chunks(64) {
        let ops: Vec<Op> = chunk.iter().copied().map(Op::PrintIntLine).collect();
        let out = diff("G12 int sweep", &ops);
        let want: Vec<u8> = chunk.iter().flat_map(|v| format!("{v}\n").into_bytes()).collect();
        assert_eq!(out, want);
    }
}

// ---------------------------------------------------------------------------
// G13 — repeated / interleaved invocation across buffering modes
// ---------------------------------------------------------------------------

#[test]
fn g13_repeated_and_interleaved_invocation() {
    let ops = vec![
        Op::PrintLineNull,
        Op::PrintLine(Vec::new()),
        Op::PrintIntLine(i32::MIN),
        Op::Bad,
        Op::PrintLineNull,
        Op::Good,
        Op::Driver,
        Op::PrintIntLine(i32::MAX),
        Op::PrintLine(b"%n".to_vec()),
        Op::PrintLineNull,
    ];
    let mut expect: Vec<u8> = Vec::new();
    expect.extend_from_slice(b"\n");
    expect.extend_from_slice(b"-2147483648\n");
    expect.extend_from_slice(b"0\n0\n");
    expect.extend_from_slice(b"0\n2\n");
    expect.extend_from_slice(
        b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n",
    );
    expect.extend_from_slice(b"2147483647\n");
    expect.extend_from_slice(b"%n\n");

    for mode in [
        Buffering::Default,
        Buffering::Unbuffered,
        Buffering::LineBuffered,
        Buffering::FullyBuffered,
    ] {
        // repeat the script 20× within a single capture
        let repeated: Vec<Op> = (0..20).flat_map(|_| ops.clone()).collect();
        let out = diff_with(&format!("G13 {mode:?}"), mode, &repeated);
        let want: Vec<u8> = (0..20).flat_map(|_| expect.clone()).collect();
        assert_eq!(out, want, "mode {mode:?}");
    }
}

// ---------------------------------------------------------------------------
// G14 — the output stream itself fails (printf returns < 0)
// ---------------------------------------------------------------------------

#[test]
fn g14_write_failure_is_ignored_identically() {
    // Neither implementation inspects the return value of printf/puts, so a
    // stream that cannot be written to must be silently tolerated by both.
    let ops = vec![
        Op::PrintLine(b"unwritable".to_vec()),
        Op::PrintLineNull,
        Op::PrintIntLine(i32::MIN),
        Op::Bad,
        Op::Good,
        Op::Driver,
    ];
    let (c_bytes, c_err) = run_with_write_failing_stdout(|| apply(c_api(), &ops));
    let (r_bytes, r_err) = run_with_write_failing_stdout(|| apply(rust_api(), &ops));

    assert!(c_bytes.is_empty(), "C wrote to a read-only stream: {c_bytes:?}");
    assert_eq!(
        c_bytes, r_bytes,
        "byte output on a failing stream diverged"
    );
    assert_eq!(
        c_err, r_err,
        "the stream error flag differs after a failing write (C={c_err}, Rust={r_err})"
    );
    assert!(c_err, "expected the stream error flag to be set");

    // Both must still return normally afterwards, on a healthy stream.
    let out = diff("G14 recovery", &[Op::Driver]);
    assert_eq!(
        out,
        b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n"
    );
}

// ---------------------------------------------------------------------------
// Extra: pointer edge cases that are *not* NULL but are unusual
// ---------------------------------------------------------------------------

#[test]
fn extra_print_line_unaligned_and_interior_pointer() {
    // A pointer into the middle of a buffer, and an odd (unaligned) address.
    let buf = b"XXXXhello world\0".to_vec();
    for skip in 0..4usize {
        let p = unsafe { buf.as_ptr().add(skip) } as *const c_char;
        let c_out = capture(|| unsafe { (c_api().print_line)(p) });
        let r_out = capture(|| unsafe { (rust_api().print_line)(p) });
        assert_eq!(c_out, r_out, "interior pointer skip={skip}");
        let mut want = buf[skip..buf.len() - 1].to_vec();
        want.push(b'\n');
        assert_eq!(c_out, want);
    }
}
