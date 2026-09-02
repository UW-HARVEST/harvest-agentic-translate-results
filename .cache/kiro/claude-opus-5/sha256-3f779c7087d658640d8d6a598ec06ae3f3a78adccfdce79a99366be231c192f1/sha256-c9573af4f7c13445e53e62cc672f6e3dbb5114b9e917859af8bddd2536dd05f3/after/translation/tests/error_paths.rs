// Phase C — error / rejection-path differential tests.
//
// One test per row of ERRORS.md, plus the generic FFI-boundary boundaries the
// task mandates (null pointers, zero/oversized lengths, values one step past a
// documented range, and out-of-range enum-like values crossing the boundary).
//
// Each test asserts the SAME rejection on both sides, identified by its
// specific observable sentinel (for this all-`void` API that is the exact
// captured `stdout` byte sequence -- e.g. "exactly zero bytes written" for the
// NULL guard), not merely "both did something".

#![allow(non_snake_case)]

mod harness;
use harness::{Api, Rng, cstr, diff};

// ---------------------------------------------------------------------------
// ERRORS row 1 -- the library's one and only input-rejection guard.
// c_src/src/driver.c:32   if(line != NULL)
// ---------------------------------------------------------------------------

#[test]
fn err_01_print_line_null_is_silent_noop() {
    let out = diff("printLine(NULL)", |a: &Api| a.print_line(std::ptr::null()));
    // Specific sentinel: the guard suppresses the printf entirely.
    assert_eq!(
        out.len(),
        0,
        "the NULL guard must write ZERO bytes (not an error string, not a \
         newline); got {}",
        harness::show(&out)
    );

    // Repeated and interleaved with valid calls: the rejection must not consume
    // or emit anything, so the surrounding output stays exactly as it would be
    // without the NULL calls.
    let s = cstr(b"x");
    let with_nulls = diff("null interleaved", |a: &Api| {
        for _ in 0..64 {
            a.print_line(std::ptr::null());
            a.print_line(s.as_ptr());
            a.print_line(std::ptr::null());
        }
    });
    assert_eq!(with_nulls, b"x\n".repeat(64));
}

// ---------------------------------------------------------------------------
// ERRORS row 2 -- zero length: empty string passes the guard with no payload.
// ---------------------------------------------------------------------------

#[test]
fn err_02_print_line_empty_string() {
    let s = cstr(b"");
    let out = diff("printLine(\"\")", |a: &Api| a.print_line(s.as_ptr()));
    assert_eq!(out, b"\n", "empty payload must still emit the trailing newline");
}

// ---------------------------------------------------------------------------
// ERRORS row 3 -- format specifiers in a DATA argument must not be evaluated.
// A divergence here (e.g. a Rust translation that used the payload as the
// format string) would be a format-string vulnerability, not just a mismatch.
// ---------------------------------------------------------------------------

#[test]
fn err_03_print_line_format_specifiers_not_interpreted() {
    for p in [
        &b"%s"[..],
        &b"%n"[..],
        &b"%s%s%s%s%s%s%s%s"[..],
        &b"%n%n%n%n"[..],
        &b"%1000000000d"[..],
        &b"%*d"[..],
        &b"%hn %hhn %lln"[..],
        &b"%s %d %n %% %p %x %o %e %f %g %c"[..],
    ] {
        let out = {
            let s = cstr(p);
            diff("printLine(format specifier payload)", |a: &Api| {
                a.print_line(s.as_ptr())
            })
        };
        let mut want = p.to_vec();
        want.push(b'\n');
        assert_eq!(
            out, want,
            "payload {:?} must be echoed verbatim; any expansion means the \
             payload was treated as a format string",
            String::from_utf8_lossy(p)
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 4 -- oversized length + high bytes.
// ---------------------------------------------------------------------------

#[test]
fn err_04_print_line_oversized_and_high_bytes() {
    // Oversized: well past every stdio buffer size, so the write is split.
    for len in [65536usize, 100_000, 262_144] {
        let bytes = vec![0xABu8; len];
        let s = cstr(&bytes);
        let out = diff(&format!("printLine(oversized len={len})"), |a: &Api| {
            a.print_line(s.as_ptr())
        });
        assert_eq!(out.len(), len + 1, "no truncation at len={len}");
        assert!(out[..len].iter().all(|&b| b == 0xAB));
        assert_eq!(out[len], b'\n');
    }

    // Every high byte value, one payload each.
    for b in 0x80u8..=0xFF {
        let s = cstr(&[b, b, b]);
        let out = diff(&format!("printLine(high byte {b:#04x})"), |a: &Api| {
            a.print_line(s.as_ptr())
        });
        assert_eq!(out, vec![b, b, b, b'\n']);
    }

    // Randomized oversized payloads with arbitrary high bytes.
    let mut rng = Rng::new(0xBADF00D_01);
    for _ in 0..8 {
        let len = 4096 + rng.below(60_000) as usize;
        let bytes = rng.cstring_bytes(len);
        let s = cstr(&bytes);
        let out = diff("printLine(rand oversized)", |a: &Api| a.print_line(s.as_ptr()));
        assert_eq!(out.len(), len + 1);
        assert_eq!(&out[..len], &bytes[..]);
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 5 -- printIntLine: values one step past the int extremes.
// ---------------------------------------------------------------------------

#[test]
fn err_05_print_int_line_extremes() {
    for v in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, -1, 0] {
        let out = diff(&format!("printIntLine({v}) [extreme]"), |a: &Api| {
            a.print_int_line(v)
        });
        assert_eq!(
            out,
            format!("{v}\n").into_bytes(),
            "two's-complement %d rendering must not clamp or trap"
        );
    }
    // INT_MIN specifically: the one value whose negation overflows.
    let out = diff("printIntLine(INT_MIN)", |a: &Api| a.print_int_line(i32::MIN));
    assert_eq!(out, b"-2147483648\n");
}

// ---------------------------------------------------------------------------
// ERRORS row 6 -- out-of-range "enum" values for driver's int flag.
// A C `int` parameter accepts all 2^32 values; `if (useGood)` is truthiness,
// so there is no invalid value and no rejection. Both sides must agree on
// WHICH branch each out-of-range value selects, which is why every case below
// is cross-checked against a direct bad()/good() call.
// ---------------------------------------------------------------------------

#[test]
fn err_06_driver_out_of_range_flag_values() {
    let bad_out = diff("bad() [reference]", |a: &Api| a.bad());
    let good_out = diff("good() [reference]", |a: &Api| a.good());

    // Zero is the ONLY value that selects bad().
    let z = diff("driver(0)", |a: &Api| a.driver(0));
    assert_eq!(z, bad_out, "0 must select bad()");

    // Everything else -- including values no enum would define -- selects good().
    let truthy: [i32; 16] = [
        1,
        2,
        3,
        -1,
        -2,
        7,
        0x100,
        0xFFFF,
        0x7FFF_FFFF,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        1 << 16,
        1 << 30,
        -(1 << 30),
        123456789,
    ];
    for v in truthy {
        let out = diff(&format!("driver({v}) [out-of-range enum]"), |a: &Api| {
            a.driver(v)
        });
        assert_eq!(
            out, good_out,
            "non-zero flag {v} must select good() via C truthiness, not be \
             rejected and not fall through to bad()"
        );
    }

    // Randomized: force the two sides to agree on branch selection for
    // arbitrary flag values, including the rare random zero.
    let mut rng = Rng::new(0xBADF00D_02);
    for _ in 0..256 {
        let v = rng.next_i32();
        let out = diff(&format!("driver({v}) [rand enum]"), |a: &Api| a.driver(v));
        let want = if v == 0 { &bad_out } else { &good_out };
        assert_eq!(&out, want, "branch selection diverged for flag {v}");
    }
}

// ---------------------------------------------------------------------------
// Generic boundary: printIntLine driven with the full set of power-of-two
// boundaries and their neighbours (one step past each documented width).
// ---------------------------------------------------------------------------

#[test]
fn err_07_print_int_line_power_of_two_boundaries() {
    let mut vals: Vec<i32> = vec![0];
    for bit in 0..31u32 {
        let p = 1i64 << bit;
        for cand in [p - 1, p, p + 1, -p - 1, -p, -p + 1] {
            if cand >= i32::MIN as i64 && cand <= i32::MAX as i64 {
                vals.push(cand as i32);
            }
        }
    }
    vals.sort_unstable();
    vals.dedup();
    for v in vals {
        let out = diff(&format!("printIntLine({v}) [pow2]"), |a: &Api| {
            a.print_int_line(v)
        });
        assert_eq!(out, format!("{v}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Generic boundary: a NUL byte immediately at the start vs. later, i.e. the
// pointer is valid but the C string terminates earlier than the buffer. Both
// sides must stop at the NUL identically.
// ---------------------------------------------------------------------------

#[test]
fn err_08_print_line_early_nul_terminator() {
    // Buffer whose logical content ends before its allocation does.
    let buf: Vec<u8> = b"visible\0hidden-must-not-appear\0".to_vec();
    let out = diff("printLine(early NUL)", |a: &Api| {
        a.print_line(buf.as_ptr() as *const std::ffi::c_char)
    });
    assert_eq!(out, b"visible\n", "must stop at the first NUL");

    // NUL at offset 0 -- non-null pointer, zero-length string.
    let buf0: Vec<u8> = b"\0trailing".to_vec();
    let out0 = diff("printLine(NUL at 0)", |a: &Api| {
        a.print_line(buf0.as_ptr() as *const std::ffi::c_char)
    });
    assert_eq!(out0, b"\n");
}

// ---------------------------------------------------------------------------
// ERRORS rows 6/7 -- a 64-bit value whose low 32 bits are zero is `0` as an
// `int`, so it must select bad() even though the 64-bit pattern is truthy.
// This pins down argument-register width handling across the FFI boundary.
// ---------------------------------------------------------------------------

#[test]
fn err_09_driver_low_byte_zero_is_still_truthy() {
    let bad_out = diff("bad() [ref/width]", |a: &Api| a.bad());
    let good_out = diff("good() [ref/width]", |a: &Api| a.good());

    // Truncating 64-bit patterns to int: low half zero => 0 => bad().
    for hi in [0xFFFF_FFFFu64, 0x1u64, 0xDEAD_BEEFu64] {
        let v = ((hi << 32) as u64) as u32 as i32; // == 0
        assert_eq!(v, 0);
        let out = diff("driver(hi-bits-only => 0)", |a: &Api| a.driver(v));
        assert_eq!(out, bad_out);
    }
    // Low half non-zero => good(), regardless of the high half.
    for lo in [1u32, 0x8000_0000, 0xFFFF_FFFF] {
        let v = lo as i32;
        let out = diff(&format!("driver({v}) [width]"), |a: &Api| a.driver(v));
        assert_eq!(out, good_out);
    }
}

// ---------------------------------------------------------------------------
// Generic boundary: bad()/good() take no arguments, so their "error surface"
// is stack robustness. bad() writes 40 bytes into a 10-byte alloca; hammer it
// hard and interleave it with the other entry points to prove the overrun
// produces no observable divergence and corrupts no subsequent call.
// ---------------------------------------------------------------------------

#[test]
fn err_10_bad_overrun_does_not_corrupt_later_calls() {
    let s = cstr(b"sentinel");
    let out = diff("bad() overrun stress", |a: &Api| {
        for i in 0..512 {
            a.bad();
            a.print_int_line(i);
            a.print_line(s.as_ptr());
            a.good();
            a.driver(0);
            a.driver(1);
        }
    });
    let mut want = Vec::new();
    for i in 0..512 {
        want.extend_from_slice(b"0\n");
        want.extend_from_slice(format!("{i}\n").as_bytes());
        want.extend_from_slice(b"sentinel\n");
        want.extend_from_slice(b"0\n");
        want.extend_from_slice(b"0\n");
        want.extend_from_slice(b"0\n");
    }
    assert_eq!(out, want, "the alloca overrun must stay unobservable on both sides");
}
