//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI-boundary boundaries.
//!
//! Every public function of this library returns `void`, so "same error result"
//! means: the same bytes written to `stdout` (in particular *zero* bytes for a
//! rejected call) and the same non-crashing return. There is no error code or
//! sentinel to compare, because the C defines none.

mod common;

use common::{Impl, Pcg32, SEED, api, assert_same, capture_stdout, cstr, render};

// ---------------------------------------------------------------------------
// Row 1 — printLine(NULL): the library's one and only input rejection
//         (`if(line != NULL)`, driver.c:31) → writes nothing, returns.
// ---------------------------------------------------------------------------
#[test]
fn err_row1_print_line_null() {
    assert_same("err1/null", |a| unsafe {
        (a.print_line)(std::ptr::null());
    });

    // Assert the *specific* rejection behaviour, not just "both agree".
    for which in [Impl::C, Impl::Rust] {
        let out = capture_stdout(|| unsafe { (api(which).print_line)(std::ptr::null()) });
        assert!(
            out.is_empty(),
            "{}: printLine(NULL) must write 0 bytes, wrote {}",
            which.name(),
            render(&out)
        );
    }

    // Repeated rejection must stay silent.
    assert_same("err1/null x64", |a| unsafe {
        for _ in 0..64 {
            (a.print_line)(std::ptr::null());
        }
    });
}

// ---------------------------------------------------------------------------
// Row 2 — printLine(""): zero-length payload, accepted, emits just "\n"
// ---------------------------------------------------------------------------
#[test]
fn err_row2_print_line_empty() {
    assert_same("err2/empty", |a| unsafe {
        let s = cstr(b"");
        (a.print_line)(s.as_ptr());
    });

    for which in [Impl::C, Impl::Rust] {
        let out = capture_stdout(|| unsafe {
            let s = cstr(b"");
            (api(which).print_line)(s.as_ptr());
        });
        assert_eq!(
            out,
            b"\n",
            "{}: printLine(\"\") must emit exactly one newline, got {}",
            which.name(),
            render(&out)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 3 — printLine: oversized lengths, no truncation
// ---------------------------------------------------------------------------
#[test]
fn err_row3_print_line_oversized() {
    let mut rng = Pcg32::new(SEED ^ 0x33);
    for &n in &[1usize, 4096, 4097, 65536, 1_048_576] {
        let payload = rng.bytes_nonzero(n);
        assert_same(&format!("err3/len {n}"), |a| unsafe {
            let s = cstr(&payload);
            (a.print_line)(s.as_ptr());
        });

        // No truncation on either side.
        for which in [Impl::C, Impl::Rust] {
            let out = capture_stdout(|| unsafe {
                let s = cstr(&payload);
                (api(which).print_line)(s.as_ptr());
            });
            assert_eq!(
                out.len(),
                n + 1,
                "{}: printLine truncated a {n}-byte payload to {} bytes",
                which.name(),
                out.len()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — printLine: non-UTF-8 bytes and printf directives pass through
// ---------------------------------------------------------------------------
#[test]
fn err_row4_print_line_non_utf8_and_format_chars() {
    // Every high byte on its own, plus multi-byte invalid sequences.
    for b in 0x80u8..=0xff {
        let payload = [b, b, 0x41, b];
        assert_same(&format!("err4/high 0x{b:02x}"), |a| unsafe {
            let s = cstr(&payload);
            (a.print_line)(s.as_ptr());
        });
    }

    let bad_utf8: &[&[u8]] = &[
        b"\xc3",                 // truncated 2-byte seq
        b"\xe2\x82",             // truncated 3-byte seq
        b"\xf0\x9f\x92",         // truncated 4-byte seq
        b"\xed\xa0\x80",         // UTF-16 surrogate encoded as UTF-8
        b"\xf5\x80\x80\x80",     // above U+10FFFF
        b"\xfe\xff",             // never-valid bytes
        b"\xc0\xaf",             // overlong "/"
        b"%s\x80%d\xff%n",       // directives + invalid bytes together
    ];
    for (i, p) in bad_utf8.iter().enumerate() {
        assert_same(&format!("err4/seq {i}"), |a| unsafe {
            let s = cstr(p);
            (a.print_line)(s.as_ptr());
        });

        // The payload must appear verbatim: %-directives are arguments, not format.
        for which in [Impl::C, Impl::Rust] {
            let out = capture_stdout(|| unsafe {
                let s = cstr(p);
                (api(which).print_line)(s.as_ptr());
            });
            let mut expected = p.to_vec();
            expected.push(b'\n');
            assert_eq!(
                out,
                expected,
                "{}: printLine reinterpreted its payload: {}",
                which.name(),
                render(&out)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — printIntLine: int extremes.
//
// This is also the "out-of-range enum value across the FFI boundary" case: the
// parameter's ABI type is a plain 32-bit `int`, the C library validates nothing,
// so every bit pattern — including ones no sensible caller would pass — is a
// real input that both sides must render identically.
// ---------------------------------------------------------------------------
#[test]
fn err_row5_print_int_line_extremes() {
    let vals: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        // bit patterns that are "out of range" for any plausible enum
        u32::MAX as i32,
        0x8000_0000u32 as i32,
        0x7fff_ffff,
        -12345,
        0x0000_00ff,
        0x0000_ffff,
    ];
    for &v in vals {
        assert_same(&format!("err5/{v}"), |a| unsafe {
            (a.print_int_line)(v);
        });
    }

    // Pin the two hardest values absolutely.
    for which in [Impl::C, Impl::Rust] {
        let out = capture_stdout(|| unsafe { (api(which).print_int_line)(i32::MIN) });
        assert_eq!(
            out,
            b"-2147483648\n",
            "{}: printIntLine(INT_MIN) wrong: {}",
            which.name(),
            render(&out)
        );
        let out = capture_stdout(|| unsafe { (api(which).print_int_line)(i32::MAX) });
        assert_eq!(
            out,
            b"2147483647\n",
            "{}: printIntLine(INT_MAX) wrong: {}",
            which.name(),
            render(&out)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 6 — a rejected call must leave no residue between valid calls
// ---------------------------------------------------------------------------
#[test]
fn err_row6_interleaved_with_rejections() {
    assert_same("err6/valid-null-valid", |a| unsafe {
        let a1 = cstr(b"before");
        let a2 = cstr(b"after");
        (a.print_line)(a1.as_ptr());
        (a.print_line)(std::ptr::null());
        (a.print_int_line)(-7);
        (a.print_line)(std::ptr::null());
        (a.print_line)(a2.as_ptr());
    });

    for which in [Impl::C, Impl::Rust] {
        let out = capture_stdout(|| unsafe {
            let a = api(which);
            let a1 = cstr(b"before");
            let a2 = cstr(b"after");
            (a.print_line)(a1.as_ptr());
            (a.print_line)(std::ptr::null());
            (a.print_int_line)(-7);
            (a.print_line)(std::ptr::null());
            (a.print_line)(a2.as_ptr());
        });
        assert_eq!(
            out,
            b"before\n-7\nafter\n",
            "{}: rejected calls left residue: {}",
            which.name(),
            render(&out)
        );
    }

    // Randomized interleaving of valid payloads, NULLs and random ints.
    let mut rng = Pcg32::new(SEED ^ 0x66);
    for i in 0..128 {
        enum S {
            Null,
            Line(Vec<u8>),
            Int(i32),
        }
        let plan: Vec<S> = (0..24)
            .map(|_| match rng.range(0, 2) {
                0 => S::Null,
                1 => {
                    let n = rng.range(0, 24) as usize;
                    S::Line(rng.bytes_nonzero(n))
                }
                _ => S::Int(rng.next_i32()),
            })
            .collect();
        assert_same(&format!("err6/rand {i}"), |a| unsafe {
            for s in &plan {
                match s {
                    S::Null => (a.print_line)(std::ptr::null()),
                    S::Line(p) => {
                        let c = cstr(p);
                        (a.print_line)(c.as_ptr());
                    }
                    S::Int(v) => (a.print_int_line)(*v),
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Generic boundary: the no-argument functions cannot be given bad input, but
// calling them immediately after a rejected call, and calling them zero times,
// are still boundaries worth pinning.
// ---------------------------------------------------------------------------
#[test]
fn err_generic_void_fns_after_rejection() {
    assert_same("errgen/void after null", |a| unsafe {
        (a.print_line)(std::ptr::null());
        (a.good)();
        (a.print_line)(std::ptr::null());
        (a.bad)();
        (a.print_line)(std::ptr::null());
        (a.driver)();
    });

    // Zero calls → zero bytes, on both sides (validates the harness itself).
    for which in [Impl::C, Impl::Rust] {
        let out = capture_stdout(|| {
            let _ = api(which);
        });
        assert!(out.is_empty(), "{}: unexpected output on load", which.name());
    }
}
