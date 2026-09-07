//! Phase C — error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md`. The library's only rejection mechanism is
//! the suppression of output (every public function returns `void`), so the
//! asserted "same error" is "the same exact byte stream, and specifically zero
//! bytes where the C's `if (line != NULL)` guard rejects".

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------------------
// Harness self-check (negative control).
//
// Guards against the whole suite passing vacuously because `capture` returns
// nothing for both implementations. Pins the exact expected byte stream.
// ---------------------------------------------------------------------------

const DRIVER_EXPECTED: &[u8] = b"Calling good()...\ngood()\nhelperGood()\nFinished good()\n\
Calling bad()...\nbad()\nFinished bad()\n";

#[test]
fn harness_sanity_capture_is_not_vacuous() {
    for which in BOTH {
        let (_, out) = capture(|| {
            let f = sym_void(which, "driver");
            unsafe { f() };
        });
        assert!(
            !out.is_empty(),
            "{} lib: capture returned zero bytes — the harness is not observing stdout",
            which.name()
        );
        assert_eq!(
            out,
            DRIVER_EXPECTED,
            "{} lib: driver() produced unexpected bytes: {:?}",
            which.name(),
            String::from_utf8_lossy(&out)
        );
    }

    // And confirm a real divergence WOULD be caught.
    let (_, good_out) = capture(|| {
        let f = sym_void(Impl::C, "good");
        unsafe { f() };
    });
    assert_eq!(good_out, b"good()\nhelperGood()\n");
    assert!(
        std::panic::catch_unwind(|| {
            assert_same("negative control", b"a", b"b");
        })
        .is_err(),
        "assert_same failed to detect a divergence"
    );
}

// ---------------------------------------------------------------------------
// Row 1 — printLine(NULL): the one derived rejection in the C source.
// ---------------------------------------------------------------------------

#[test]
fn err_row1_print_line_null() {
    // Differential: both must behave identically.
    diff_print_line_null("row1 printLine(NULL)");

    // And pin the absolute expectation: exactly zero bytes, from BOTH libs.
    for which in BOTH {
        let (_, out) = capture(|| {
            let f = sym_print_line(which);
            unsafe { f(std::ptr::null()) };
        });
        assert!(
            out.is_empty(),
            "{} lib: printLine(NULL) wrote {} byte(s) ({:?}); the C guard requires none",
            which.name(),
            out.len(),
            String::from_utf8_lossy(&out)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 2 — repeated rejection is not sticky and leaves no state behind.
// ---------------------------------------------------------------------------

#[test]
fn err_row2_print_line_null_repeated() {
    diff("row2 printLine(NULL) x1000", |which| {
        let f = sym_print_line(which);
        for _ in 0..1000 {
            unsafe { f(std::ptr::null()) };
        }
    });

    for which in BOTH {
        let (_, out) = capture(|| {
            let f = sym_print_line(which);
            for _ in 0..1000 {
                unsafe { f(std::ptr::null()) };
            }
        });
        assert!(
            out.is_empty(),
            "{} lib: 1000x printLine(NULL) wrote {} byte(s)",
            which.name(),
            out.len()
        );
    }

    // A rejected call must not disturb the following valid one.
    diff("row2 NULL then valid", |which| {
        let f = sym_print_line(which);
        unsafe {
            f(std::ptr::null());
            f(b"still works\0".as_ptr() as *const c_char);
        }
    });
}

// ---------------------------------------------------------------------------
// Row 3 — the zero-length string: one step inside the valid range.
// ---------------------------------------------------------------------------

#[test]
fn err_row3_print_line_empty() {
    diff_print_line_raw("row3 empty string", b"\0");

    for which in BOTH {
        let (_, out) = capture(|| {
            let f = sym_print_line(which);
            unsafe { f(b"\0".as_ptr() as *const c_char) };
        });
        assert_eq!(
            out,
            b"\n",
            "{} lib: printLine(\"\") must emit exactly one newline, got {:?}",
            which.name(),
            String::from_utf8_lossy(&out)
        );
    }

    // Empty strings repeated, and mixed with NULL, must stay distinguishable.
    diff("row3 empty vs null mixed", |which| {
        let f = sym_print_line(which);
        unsafe {
            f(b"\0".as_ptr() as *const c_char);
            f(std::ptr::null());
            f(b"\0".as_ptr() as *const c_char);
            f(std::ptr::null());
            f(b"\0".as_ptr() as *const c_char);
        }
    });
}

// ---------------------------------------------------------------------------
// Row 4 — a rejection between two accepted calls.
// ---------------------------------------------------------------------------

#[test]
fn err_row4_null_interleaved_with_valid() {
    diff("row4 null interleaved", |which| {
        let f = sym_print_line(which);
        unsafe {
            f(std::ptr::null());
            f(b"middle\0".as_ptr() as *const c_char);
            f(std::ptr::null());
        }
    });

    for which in BOTH {
        let (_, out) = capture(|| {
            let f = sym_print_line(which);
            unsafe {
                f(std::ptr::null());
                f(b"middle\0".as_ptr() as *const c_char);
                f(std::ptr::null());
            }
        });
        assert_eq!(
            out,
            b"middle\n",
            "{} lib: NULL calls must contribute nothing, got {:?}",
            which.name(),
            String::from_utf8_lossy(&out)
        );
    }

    // Randomized NULL/valid interleavings.
    let mut rng = Rng::new(0xE770_0004);
    for i in 0..128 {
        let script: Vec<Option<Vec<u8>>> = (0..24)
            .map(|_| {
                if rng.below(2) == 0 {
                    None
                } else {
                    Some(rng.ascii_len(0, 40))
                }
            })
            .collect();
        diff(&format!("row4 randomized iter={i}"), |which| {
            let f = sym_print_line(which);
            for item in &script {
                match item {
                    None => unsafe { f(std::ptr::null()) },
                    Some(p) => {
                        let mut buf = p.clone();
                        buf.push(0);
                        unsafe { f(buf.as_ptr() as *const c_char) };
                    }
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Row 5 — data must never be interpreted as a format string.
// ---------------------------------------------------------------------------

#[test]
fn err_row5_format_specifiers_not_interpreted() {
    // `%n` is the dangerous one: if either side passed the payload as the
    // format string, this would write through a garbage pointer or abort.
    for payload in [
        &b"%s"[..],
        b"%d",
        b"%n",
        b"%%",
        b"%s %d %n %%",
        b"%n%n%n%n%n%n%n%n",
        b"%99999999d",
        b"%*s",
        b"%hn",
        b"%lln",
    ] {
        diff_print_line(
            &format!("row5 {}", String::from_utf8_lossy(payload)),
            payload,
        );
        for which in BOTH {
            let mut buf = payload.to_vec();
            buf.push(0);
            let (_, out) = capture(|| {
                let f = sym_print_line(which);
                unsafe { f(buf.as_ptr() as *const c_char) };
            });
            let mut expected = payload.to_vec();
            expected.push(b'\n');
            assert_eq!(
                out,
                expected,
                "{} lib: format specifiers must be emitted literally, got {:?}",
                which.name(),
                String::from_utf8_lossy(&out)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — an embedded NUL truncates the output.
// ---------------------------------------------------------------------------

#[test]
fn err_row6_embedded_nul_truncates() {
    diff_print_line_raw("row6 nul in the middle", b"visible\0hidden\0");
    diff_print_line_raw("row6 nul first", b"\0hidden\0");
    diff_print_line_raw("row6 many nuls", b"a\0b\0c\0d\0");

    for which in BOTH {
        let (_, out) = capture(|| {
            let f = sym_print_line(which);
            unsafe { f(b"visible\0hidden\0".as_ptr() as *const c_char) };
        });
        assert_eq!(
            out,
            b"visible\n",
            "{} lib: output must stop at the embedded NUL, got {:?}",
            which.name(),
            String::from_utf8_lossy(&out)
        );
    }

    // Randomized: a payload with a NUL at a random offset.
    let mut rng = Rng::new(0xE770_0006);
    for i in 0..128 {
        let head = rng.ascii_len(0, 40);
        let tail = rng.ascii_len(0, 40);
        let mut buf = head.clone();
        buf.push(0);
        buf.extend_from_slice(&tail);
        buf.push(0);
        diff_print_line_raw(&format!("row6 randomized iter={i}"), &buf);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — every single non-NUL byte value, including invalid UTF-8.
// ---------------------------------------------------------------------------

#[test]
fn err_row7_all_single_byte_values() {
    for b in 1u8..=255 {
        let buf = [b, 0];
        diff_print_line_raw(&format!("row7 byte 0x{b:02x}"), &buf);
        for which in BOTH {
            let (_, out) = capture(|| {
                let f = sym_print_line(which);
                unsafe { f(buf.as_ptr() as *const c_char) };
            });
            assert_eq!(
                out,
                [b, b'\n'],
                "{} lib: byte 0x{b:02x} must pass through verbatim, got {out:?}",
                which.name()
            );
        }
    }

    // Deliberately malformed UTF-8 sequences: lone continuation bytes, truncated
    // multi-byte sequences, overlong encodings, surrogates, and 0xFE/0xFF.
    for seq in [
        &b"\x80"[..],
        b"\xbf",
        b"\xc0",
        b"\xc0\x80",
        b"\xc2",
        b"\xe0\x80",
        b"\xed\xa0\x80",
        b"\xf0\x82\x82\xac",
        b"\xf4\x90\x80\x80",
        b"\xf8\x88\x80\x80\x80",
        b"\xfe\xff",
        b"\xff\xfe\xfd",
        b"valid\xc3\xa9then\xffbroken",
    ] {
        diff_print_line("row7 malformed utf8", seq);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — oversized input (1 MiB).
// ---------------------------------------------------------------------------

#[test]
fn err_row8_oversized_input() {
    let payload = vec![b'Z'; 1024 * 1024];
    diff_print_line("row8 1MiB", &payload);

    for which in BOTH {
        let mut buf = payload.clone();
        buf.push(0);
        let (_, out) = capture(|| {
            let f = sym_print_line(which);
            unsafe { f(buf.as_ptr() as *const c_char) };
        });
        assert_eq!(
            out.len(),
            payload.len() + 1,
            "{} lib: 1MiB payload produced {} bytes",
            which.name(),
            out.len()
        );
        assert_eq!(out[..payload.len()], payload[..]);
        assert_eq!(out[payload.len()], b'\n');
    }
}

// ---------------------------------------------------------------------------
// Row 9 — the C's `static` helpers must not be resolvable in either library.
// ---------------------------------------------------------------------------

#[test]
fn err_row9_static_helpers_not_resolvable() {
    for name in ["helperBad", "helperGood"] {
        for which in BOTH {
            assert!(
                !has_symbol(which, name),
                "{} lib exports `{name}`, but the C declares it `static` — \
                 the symbol surfaces must match",
                which.name()
            );
        }
    }
    // Sanity: the four real exports ARE resolvable in both.
    for name in ["printLine", "bad", "good", "driver"] {
        for which in BOTH {
            assert!(
                has_symbol(which, name),
                "{} lib does not export `{name}`",
                which.name()
            );
        }
    }
    // A symbol that exists in neither.
    for which in BOTH {
        assert!(!has_symbol(which, "definitely_not_a_symbol_xyz"));
    }
}

// ---------------------------------------------------------------------------
// Row 10 — the void-parameter boundary: extra args across the C ABI.
//
// The C functions are declared `void f()` / `void driver(void)`. Calling them
// through a wider prototype leaves garbage in the argument registers, which a
// conforming callee ignores. Both sides must ignore it identically. This stands
// in for the "out-of-range enum value" class of test: the library has no enum
// parameter, so the analogous untrusted-value-across-FFI input is a bogus
// argument passed to a no-parameter function.
// ---------------------------------------------------------------------------

type FnManyArgs = unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> i64;

#[test]
fn err_row10_void_functions_ignore_extra_args() {
    for name in ["bad", "good", "driver"] {
        diff(&format!("row10 {name} with garbage args"), |which| {
            let f = sym_void(which, name);
            // SAFETY: x86-64 SysV / AArch64 AAPCS both let a callee that takes
            // no parameters ignore argument registers. The fn pointer itself is
            // unchanged; only the call-site prototype is widened.
            let wide: FnManyArgs = unsafe { std::mem::transmute(*f) };
            unsafe {
                wide(-1, i64::MIN, i64::MAX, 0x7fff_ffff, -0x8000_0000, 0xdead_beef);
            }
        });
    }

    // And the widened call must produce the same bytes as the plain call.
    for name in ["bad", "good", "driver"] {
        for which in BOTH {
            let (_, plain) = capture(|| {
                let f = sym_void(which, name);
                unsafe { f() };
            });
            let (_, widened) = capture(|| {
                let f = sym_void(which, name);
                let wide: FnManyArgs = unsafe { std::mem::transmute(*f) };
                unsafe { wide(-1, i64::MIN, i64::MAX, 7, -7, 0xdead_beef) };
            });
            assert_eq!(
                plain,
                widened,
                "{} lib: {name}() changed behaviour when passed extra arguments",
                which.name()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Extra generic boundary: an out-of-range "enum-like" int reinterpreted as the
// printLine pointer. Small non-zero integers are not valid pointers, so those
// would fault in BOTH implementations (undefined behaviour, excluded). What IS
// well-defined and worth pinning is that only the exact value 0 is treated as
// the rejection sentinel — verified by checking a pointer that is non-NULL but
// points at a NUL byte located at a high address inside a large allocation.
// ---------------------------------------------------------------------------

#[test]
fn err_extra_only_exact_null_is_the_sentinel() {
    // A valid pointer deep inside a big allocation: non-zero, high, unaligned.
    let mut big = vec![b'q'; 1 << 20];
    let idx = (1 << 20) - 1;
    big[idx] = 0;
    let interior = unsafe { big.as_ptr().add(idx - 3) } as *const c_char;

    diff("row-extra interior pointer", |which| {
        let f = sym_print_line(which);
        unsafe { f(interior) };
    });

    for which in BOTH {
        let (_, out) = capture(|| {
            let f = sym_print_line(which);
            unsafe { f(interior) };
        });
        assert_eq!(
            out,
            b"qqq\n",
            "{} lib: interior non-NULL pointer must be accepted, got {:?}",
            which.name(),
            String::from_utf8_lossy(&out)
        );
    }
}
