//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md` (plus the generic FFI-boundary rows G1..G6).
//!
//! `driver.c` has no error codes and no `assert`s: every function returns `void`.
//! Its entire rejection surface is therefore "reject by silence" plus the
//! truthiness test in `driver`, so each row asserts the exact observable result
//! (bytes on stdout) *and*, for the paths that can fault, the exact termination
//! status — never merely "both failed somehow".

mod common;

use common::*;
use std::ffi::c_char;

fn cstr(payload: &[u8]) -> Vec<u8> {
    let mut v = payload.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// E1 / G1 — printLine(NULL): the only explicit check in the C.
// ---------------------------------------------------------------------------
#[test]
fn err_e1_print_line_null() {
    // Exactly zero bytes must be produced, AND the call must return normally
    // (exit 0, not a fault). Checked in a child process first so that losing the
    // NULL check — which makes glibc's `puts` dereference NULL — is reported as a
    // readable assertion instead of killing the test binary.
    assert_same_and_eq_isolated("E1/null", b"", |api| unsafe {
        api.print_line(std::ptr::null())
    });
    assert_same_and_eq("E1/null-inproc", b"", |api| unsafe {
        api.print_line(std::ptr::null())
    });
    // Repeated, and mixed with a valid call, so a "silent" failure cannot be
    // confused with a swallowed later call.
    let s = cstr(b"after");
    assert_same_and_eq("E1/null-then-valid", b"after\n", |api| unsafe {
        api.print_line(std::ptr::null());
        api.print_line(std::ptr::null());
        api.print_line(s.as_ptr() as *const c_char);
    });
    // And the status must be a clean exit, not a fault.
    assert_same_isolated("E1/null-status", |api| unsafe {
        api.print_line(std::ptr::null())
    });
}

// ---------------------------------------------------------------------------
// E2 / G2 — printLine(""): non-NULL but zero length.
// ---------------------------------------------------------------------------
#[test]
fn err_e2_print_line_empty() {
    let s = cstr(b"");
    assert_same_and_eq("E2/empty", b"\n", |api| unsafe {
        api.print_line(s.as_ptr() as *const c_char)
    });
}

// ---------------------------------------------------------------------------
// E3 / G3 — oversized length: the NUL terminator is the only bound.
// ---------------------------------------------------------------------------
#[test]
fn err_e3_print_line_oversized() {
    let payload = vec![b'A'; 1 << 20];
    let s = cstr(&payload);
    let mut exp = payload.clone();
    exp.push(b'\n');
    assert_same_and_eq("E3/1MiB", &exp, |api| unsafe {
        api.print_line(s.as_ptr() as *const c_char)
    });
}

// ---------------------------------------------------------------------------
// E4 — embedded NUL: everything past it must be dropped.
// ---------------------------------------------------------------------------
#[test]
fn err_e4_print_line_embedded_nul() {
    for (payload, exp) in [
        (&b"\0hidden"[..], &b"\n"[..]),
        (b"a\0hidden", b"a\n"),
        (b"visible\0hidden\0more", b"visible\n"),
        (b"\0\0\0", b"\n"),
    ] {
        let s = cstr(payload);
        assert_same_and_eq(&format!("E4/{payload:?}"), exp, |api| unsafe {
            api.print_line(s.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// E5 — format characters in the *data*: must NOT be interpreted.
// ---------------------------------------------------------------------------
#[test]
fn err_e5_print_line_percent() {
    // `%n` is the classic format-string exploit primitive; because `line` is the
    // argument and not the format, it must be printed literally by both.
    for payload in [
        &b"%n"[..],
        b"%s%s%s%s%s%s%s%s%s%s%s%s",
        b"%n%n%n%n%n%n%n%n",
        b"%1000000000d",
        b"%*d",
        b"%",
        b"%%n",
    ] {
        let s = cstr(payload);
        let mut exp = payload.to_vec();
        exp.push(b'\n');
        assert_same_and_eq(&format!("E5/{}", String::from_utf8_lossy(payload)), &exp, |api| unsafe {
            api.print_line(s.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// E6 — invalid UTF-8 / high bytes: valid C strings, must pass through verbatim.
// ---------------------------------------------------------------------------
#[test]
fn err_e6_print_line_invalid_utf8() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0xff],
        vec![0x80],
        vec![0xc0, 0x80],           // overlong encoding of NUL
        vec![0xed, 0xa0, 0x80],     // UTF-16 surrogate half
        vec![0xf5, 0x80, 0x80, 0x80], // beyond U+10FFFF
        vec![0xfe, 0xff],           // BOM-ish garbage
        (0x80u8..=0xff).collect(),  // every high byte
    ];
    for (i, payload) in cases.iter().enumerate() {
        let s = cstr(payload);
        let mut exp = payload.clone();
        exp.push(b'\n');
        assert_same_and_eq(&format!("E6/#{i}"), &exp, |api| unsafe {
            api.print_line(s.as_ptr() as *const c_char)
        });
    }
}

// ---------------------------------------------------------------------------
// E7 — driver(0): selects the defective branch.
// ---------------------------------------------------------------------------
#[test]
fn err_e7_driver_zero() {
    // Must agree on bytes AND on how the process ends.
    assert_same_isolated("E7/driver(0)", |api| unsafe { api.driver(0) });
    // Also from a stack state where the C is known to fault, so that "both
    // crash the same way" is genuinely exercised rather than assumed.
    assert_same_isolated("E7/driver(0)-dirty", |api| {
        dirty_stack(0x0102_0304_0506_0708, 2);
        unsafe { api.driver(0) }
    });
}

// ---------------------------------------------------------------------------
// E8 / G4 / G5 — driver with out-of-range "enum" ints.
// ---------------------------------------------------------------------------
#[test]
fn err_e8_driver_out_of_range_enum() {
    // A C `enum`/bool parameter accepts any `int` across the FFI boundary. The C
    // only tests truthiness, so every one of these must take the `good()` branch
    // and print exactly "string\n" — including the values one step outside the
    // documented 0/1 range (-1 and 2).
    for v in [
        -1i32,
        2,
        3,
        -2,
        i32::MIN,
        i32::MAX,
        0x100,
        0xffff,
        0x7fff_ffff,
        u32::MAX as i32,          // -1 reinterpreted
        0xffff_ff00u32 as i32,
        0x0000_0100,
        i32::MIN + 1,
    ] {
        assert_same_and_eq(&format!("E8/v={v}"), b"string\n", |api| unsafe {
            api.driver(v)
        });
    }
    // ...and only exactly zero takes the other branch.
    assert_same_isolated("E8/v=0", |api| unsafe { api.driver(0) });
}

// ---------------------------------------------------------------------------
// E9 — the parameter is an `int`: wider values are truncated to 32 bits.
// ---------------------------------------------------------------------------
#[test]
fn err_e9_driver_int_truncation() {
    // Call through a 64-bit-typed function pointer so the full register is set,
    // then confirm both callees agree on the truncated `int` they observe.
    // 0x1_0000_0000 truncates to 0  -> the bad() branch (isolated).
    assert_same_isolated("E9/0x100000000 -> 0", |api| unsafe {
        api.driver_wide(0x1_0000_0000u64)
    });
    // 0x1_0000_0001 truncates to 1  -> the good() branch.
    assert_same_and_eq("E9/0x100000001 -> 1", b"string\n", |api| unsafe {
        api.driver_wide(0x1_0000_0001u64)
    });
    // 0xFFFF_FFFF_0000_0000 truncates to 0 -> the bad() branch.
    assert_same_isolated("E9/0xffffffff00000000 -> 0", |api| unsafe {
        api.driver_wide(0xffff_ffff_0000_0000u64)
    });
}

// ---------------------------------------------------------------------------
// E10 — bad(): the uninitialized read itself (CWE-457).
// ---------------------------------------------------------------------------
#[test]
fn err_e10_bad_uninitialized_read() {
    // Same caller, same stack => the indeterminate slot both libraries read is
    // the same memory with the same contents, so bytes *and* status must match.
    assert_same_isolated("E10/bad", |api| unsafe { api.bad() });
    for fill in [0u64, u64::MAX, 0x4141_4141_4141_4141, 0xdead_beef_dead_beef] {
        for depth in 0..3u32 {
            assert_same_isolated(&format!("E10/bad f={fill:#x} d={depth}"), |api| {
                dirty_stack(fill, depth);
                unsafe { api.bad() }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — good(): no invalid input is representable.
// ---------------------------------------------------------------------------
#[test]
fn err_e11_good_no_args() {
    assert_same_and_eq("E11/good", b"string\n", |api| unsafe { api.good() });
    assert_same_isolated("E11/good-status", |api| unsafe { api.good() });
}

// ---------------------------------------------------------------------------
// G6 — no hidden global state: the Nth call equals the 1st.
// ---------------------------------------------------------------------------
#[test]
fn err_g6_no_hidden_state() {
    let s = cstr(b"probe");
    // 1st vs 50th call of every well-defined entry point, in the same capture,
    // must produce a perfectly periodic stream.
    let mut exp = Vec::new();
    for _ in 0..50 {
        exp.extend_from_slice(b"probe\n");
        exp.extend_from_slice(b"string\n");
        exp.extend_from_slice(b"string\n");
    }
    assert_same_and_eq("G6/periodic", &exp, |api| unsafe {
        for _ in 0..50 {
            api.print_line(s.as_ptr() as *const c_char);
            api.good();
            api.driver(7);
            api.print_line(std::ptr::null()); // contributes nothing
        }
    });
}

// ---------------------------------------------------------------------------
// Extra boundary coverage: unaligned and one-past-end pointers that are still
// valid C strings, plus a pointer to a read-only mapping.
// ---------------------------------------------------------------------------
#[test]
fn err_extra_pointer_shapes() {
    // Pointer to the terminating NUL of a buffer (i.e. an empty string at the
    // very end of an allocation) — a classic off-by-one shape.
    let buf = cstr(b"abcdef");
    let last = buf.len() - 1;
    assert_same_and_eq("X/one-past-content", b"\n", |api| unsafe {
        api.print_line(buf.as_ptr().add(last) as *const c_char)
    });

    // Deliberately unaligned starts (odd offsets) — `char*` has no alignment
    // requirement, so every offset must behave.
    let long = cstr(b"0123456789abcdef0123456789abcdef");
    for off in [1usize, 3, 5, 7, 9, 15, 17, 31] {
        let exp = {
            let mut v = long[off..long.len() - 1].to_vec();
            v.push(b'\n');
            v
        };
        assert_same_and_eq(&format!("X/unaligned+{off}"), &exp, |api| unsafe {
            api.print_line(long.as_ptr().add(off) as *const c_char)
        });
    }

    // A string in a read-only static (like the C's own literal).
    static RO: &[u8] = b"readonly\0";
    assert_same_and_eq("X/static-ro", b"readonly\n", |api| unsafe {
        api.print_line(RO.as_ptr() as *const c_char)
    });
}

// ---------------------------------------------------------------------------
// Extra: a wild (non-NULL, unmapped) pointer. The C has no way to reject it, so
// both libraries must fault identically. This is the observable consequence of
// the missing validation in `printLine`.
// ---------------------------------------------------------------------------
#[test]
fn err_extra_wild_pointer() {
    for addr in [1usize, 8, 0x1000, 0xdead_beef, usize::MAX & !0xfff] {
        assert_same_isolated(&format!("X/wild={addr:#x}"), |api| unsafe {
            api.print_line(addr as *const c_char)
        });
    }
}
