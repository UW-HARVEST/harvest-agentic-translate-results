//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`.  The C library has no error *returns* (both
//! functions are `void` and branch-free), so its rejection surface consists of
//! silent bit-field truncation and undefined behaviour on a bad pointer.  Each
//! test asserts C and Rust produce the *same* observable result: identical
//! stdout bytes, or — for the UB rows — the identical fatal signal / exit
//! status, not merely "both failed somehow".

mod common;

use common::*;

/// The exact line the C `printf("%u %u %d %d\n", ...)` must produce.
fn expected_line(x: u32, y: u32, b: u8, z: i32) -> Vec<u8> {
    format!("{} {} {} {}\n", x & 0x3, y & 0x7, b & 0x1, z).into_bytes()
}

/// Assert C == Rust *and* that both equal the value derived from the C source.
#[track_caller]
fn assert_driver_exact(x: u32, y: u32, b: u8, z: i32) {
    let p = pair();
    let c = run_driver(&p.c, x, y, b, z);
    let r = run_driver(&p.rust, x, y, b, z);
    assert_eq!(
        c,
        r,
        "driver(x={x:#x}, y={y:#x}, b={b:#x}, z={z}): C and Rust diverged\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    let want = expected_line(x, y, b, z);
    assert_eq!(
        c,
        want,
        "C output does not match the documented truncation semantics for \
         (x={x:#x}, y={y:#x}, b={b:#x}, z={z}): got {:?}, want {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&want)
    );
}

// ---------------------------------------------------------------------------
// Row 1 — `x` out of range of the 2-bit field: silently truncated to x & 3.
// ---------------------------------------------------------------------------
#[test]
fn err_x_out_of_range() {
    let mut rng = Rng::new(0xE001);
    for x in [4u32, 5, 6, 7, 8, 15, 16, 255, 256, 0xFFFF, 0xFFFF_FFFF] {
        assert_driver_exact(x, 3, 1, -42);
    }
    for _ in 0..200 {
        let x = rng.next_u32() | 4; // always out of range
        assert_driver_exact(x, rng.below(8), rng.next_u8() & 1, rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 2 — `y` out of range of the 3-bit field: silently truncated to y & 7.
// ---------------------------------------------------------------------------
#[test]
fn err_y_out_of_range() {
    let mut rng = Rng::new(0xE002);
    for y in [8u32, 9, 10, 15, 16, 31, 32, 255, 256, 0xFFFF, 0xFFFF_FFFF] {
        assert_driver_exact(1, y, 0, 7);
    }
    for _ in 0..200 {
        let y = rng.next_u32() | 8; // always out of range
        assert_driver_exact(rng.below(4), y, rng.next_u8() & 1, rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 3 — `b` is a non-canonical `_Bool` byte: only bit 0 survives.
// ---------------------------------------------------------------------------
#[test]
fn err_b_non_canonical_byte() {
    for b in [2u8, 3, 4, 0x7F, 0x80, 0xFE, 0xFF] {
        assert_driver_exact(2, 5, b, 99);
        // The documented semantics: prints 1 iff the low bit is set.
        let p = pair();
        let got = run_driver(&p.c, 2, 5, b, 99);
        let expect_b = if b & 1 == 1 { b'1' } else { b'0' };
        assert_eq!(
            got[4], expect_b,
            "b={b:#04x}: expected the `b` column to be {} (low bit only)",
            expect_b as char
        );
    }
}

// ---------------------------------------------------------------------------
// Row 15 (enum-equivalent) — exhaustively every one of the 256 possible bytes
// a caller can put in the `_Bool` parameter slot across the FFI boundary.
// `_Bool` has exactly two valid "variants" (0 and 1); the other 254 bytes are
// the out-of-range values the C code must, and does, still handle.
// ---------------------------------------------------------------------------
#[test]
fn err_b_all_256_bytes_exhaustive() {
    let mut rng = Rng::new(0xE015);
    for b in 0u8..=255 {
        assert_driver_exact(rng.below(4), rng.below(8), b, rng.next_i32());
        // and against the extremes of the other axes
        assert_driver_exact(u32::MAX, u32::MAX, b, i32::MIN);
        assert_driver_exact(0, 0, b, 0);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — `x` exactly one step past the valid 2-bit range.
// ---------------------------------------------------------------------------
#[test]
fn err_boundary_one_past_x() {
    assert_driver_exact(3, 0, 0, 0); // last valid
    assert_driver_exact(4, 0, 0, 0); // one past -> 0
    let p = pair();
    let got = run_driver(&p.c, 4, 0, 0, 0);
    assert_eq!(&got[..], b"0 0 0 0\n", "x=4 must wrap to 0");
}

// ---------------------------------------------------------------------------
// Row 5 — `y` exactly one step past the valid 3-bit range.
// ---------------------------------------------------------------------------
#[test]
fn err_boundary_one_past_y() {
    assert_driver_exact(0, 7, 0, 0); // last valid
    assert_driver_exact(0, 8, 0, 0); // one past -> 0
    let p = pair();
    let got = run_driver(&p.c, 0, 8, 0, 0);
    assert_eq!(&got[..], b"0 0 0 0\n", "y=8 must wrap to 0");
}

// ---------------------------------------------------------------------------
// Row 6 — `x`/`y` at UINT_MAX, far past their field ranges.
// ---------------------------------------------------------------------------
#[test]
fn err_uint_max_args() {
    assert_driver_exact(u32::MAX, u32::MAX, 1, 0);
    let p = pair();
    let got = run_driver(&p.c, u32::MAX, u32::MAX, 1, 0);
    assert_eq!(
        &got[..], b"3 7 1 0\n",
        "UINT_MAX must saturate the fields to 3 and 7"
    );
}

// ---------------------------------------------------------------------------
// Row 7 — z == INT_MIN.
// ---------------------------------------------------------------------------
#[test]
fn err_z_int_min() {
    assert_driver_exact(0, 0, 0, i32::MIN);
    let p = pair();
    let got = run_driver(&p.c, 0, 0, 0, i32::MIN);
    assert_eq!(&got[..], b"0 0 0 -2147483648\n");
    // also through the low-level entry point
    assert_print_foo_eq(0x00, [0, 0, 0], i32::MIN);
    assert_print_foo_eq(0xFF, [0xFF; 3], i32::MIN);
}

// ---------------------------------------------------------------------------
// Row 8 — z == INT_MAX.
// ---------------------------------------------------------------------------
#[test]
fn err_z_int_max() {
    assert_driver_exact(0, 0, 0, i32::MAX);
    let p = pair();
    let got = run_driver(&p.c, 0, 0, 0, i32::MAX);
    assert_eq!(&got[..], b"0 0 0 2147483647\n");
    assert_print_foo_eq(0x00, [0, 0, 0], i32::MAX);
    assert_print_foo_eq(0xFF, [0xFF; 3], i32::MAX);
}

// ---------------------------------------------------------------------------
// Row 9 — `z` negative in general: no truncation, printed verbatim via %d.
// ---------------------------------------------------------------------------
#[test]
fn err_z_negative() {
    let mut rng = Rng::new(0xE009);
    for z in [-1i32, -2, -9, -10, -99, -100, -1_000_000_000, i32::MIN + 1] {
        assert_driver_exact(1, 1, 1, z);
    }
    for _ in 0..200 {
        // force the sign bit so every iteration is negative
        let z = (rng.next_u32() | 0x8000_0000) as i32;
        assert_driver_exact(rng.next_u32(), rng.next_u32(), rng.next_u8(), z);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — print_foo: padding bits 6..7 of the storage byte must be ignored.
// ---------------------------------------------------------------------------
#[test]
fn err_padding_bits_ignored() {
    let p = pair();
    let mut rng = Rng::new(0xE010);
    for significant in 0x00u8..=0x3F {
        let base = run_print_foo(&p.c, &foo_image(significant, [0, 0, 0], 0));
        for pad_bits in [0x00u8, 0x40, 0x80, 0xC0] {
            let storage = significant | pad_bits;
            assert_print_foo_eq(storage, [0, 0, 0], 0);
            let got = run_print_foo(&p.c, &foo_image(storage, [0, 0, 0], 0));
            assert_eq!(
                got, base,
                "C: padding bits {pad_bits:#04x} changed the output for \
                 significant bits {significant:#04x}"
            );
            let got_rust = run_print_foo(&p.rust, &foo_image(storage, [0, 0, 0], 0));
            assert_eq!(
                got_rust, base,
                "Rust: padding bits {pad_bits:#04x} leaked into the output for \
                 significant bits {significant:#04x}"
            );
        }
    }
    for _ in 0..200 {
        let significant = rng.next_u8() & 0x3F;
        let z = rng.next_i32();
        let base_c = run_print_foo(&p.c, &foo_image(significant, [0, 0, 0], z));
        let storage = significant | (rng.next_u8() & 0xC0);
        assert_eq!(run_print_foo(&p.c, &foo_image(storage, [0, 0, 0], z)), base_c);
        assert_eq!(
            run_print_foo(&p.rust, &foo_image(storage, [0, 0, 0], z)),
            base_c
        );
    }
}

// ---------------------------------------------------------------------------
// Row 11 — print_foo: storage byte 0xFF (every bit saturated).
// ---------------------------------------------------------------------------
#[test]
fn err_all_bits_set() {
    let p = pair();
    let mut rng = Rng::new(0xE011);
    for z in Z_BOUNDARIES {
        assert_print_foo_eq(0xFF, [0xFF; 3], z);
        let got = run_print_foo(&p.c, &foo_image(0xFF, [0xFF; 3], z));
        assert_eq!(
            got,
            format!("3 7 1 {z}\n").into_bytes(),
            "storage=0xFF must print `3 7 1 {z}`"
        );
    }
    for _ in 0..100 {
        assert_print_foo_eq(0xFF, [0xFF; 3], rng.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Row 12 — print_foo: inter-field padding bytes 1..3 must be ignored.
// ---------------------------------------------------------------------------
#[test]
fn err_inter_field_padding_ignored() {
    let p = pair();
    let mut rng = Rng::new(0xE012);
    for _ in 0..300 {
        let storage = rng.next_u8();
        let z = rng.next_i32();
        let base = run_print_foo(&p.c, &foo_image(storage, [0, 0, 0], z));
        let pad = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        assert_eq!(
            run_print_foo(&p.c, &foo_image(storage, pad, z)),
            base,
            "C: inter-field padding {pad:02x?} changed the output"
        );
        assert_eq!(
            run_print_foo(&p.rust, &foo_image(storage, pad, z)),
            base,
            "Rust: inter-field padding {pad:02x?} leaked into the output"
        );
    }
    // explicit all-ones padding
    assert_print_foo_eq(0x2A, [0xFF, 0xFF, 0xFF], 12345);
}

// ---------------------------------------------------------------------------
// Row 16 — the "zero / oversized length" analogue: all-zero and all-max input.
// ---------------------------------------------------------------------------
#[test]
fn err_all_zero_and_all_max() {
    let p = pair();
    assert_driver_exact(0, 0, 0, 0);
    assert_eq!(&run_driver(&p.c, 0, 0, 0, 0)[..], b"0 0 0 0\n");

    assert_driver_exact(u32::MAX, u32::MAX, 0xFF, i32::MAX);
    assert_eq!(
        &run_driver(&p.c, u32::MAX, u32::MAX, 0xFF, i32::MAX)[..],
        b"3 7 1 2147483647\n"
    );

    // low-level entry point, same two extremes
    assert_print_foo_eq(0x00, [0, 0, 0], 0);
    assert_print_foo_eq(0xFF, [0xFF; 3], i32::MAX);
}

// ---------------------------------------------------------------------------
// Rows 13 & 14 — bad pointers into `print_foo`.
//
// The C code dereferences `foo` with no null check, so this is undefined
// behaviour.  Both implementations are run in forked children and their exit
// statuses are compared, asserting the *same* fatal signal — not just that
// both "failed somehow".
// ---------------------------------------------------------------------------

/// How a child terminated.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Run `f` in a forked child and report how the child terminated.
fn outcome_of<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: silence stdout/stderr so a crash message cannot pollute
            // the test log, then perform the UB.
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 1);
                libc::dup2(devnull, 2);
            }
            f();
            // If we get here the call did not crash.
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else {
            Outcome::Exited(libc::WEXITSTATUS(status))
        }
    }
}

#[test]
fn err_null_pointer_both_crash() {
    let p = pair();
    let c = outcome_of(|| unsafe { (p.c.print_foo)(std::ptr::null()) });
    let r = outcome_of(|| unsafe { (p.rust.print_foo)(std::ptr::null()) });
    assert_eq!(
        c, r,
        "print_foo(NULL): C and Rust terminated differently (C={c:?}, Rust={r:?})"
    );
    assert_eq!(
        c,
        Outcome::Signaled(libc::SIGSEGV),
        "expected the C code to fault with SIGSEGV on a NULL dereference"
    );
}

#[test]
fn err_wild_pointer_both_crash() {
    let p = pair();
    // A non-null, deliberately unmapped, correctly-aligned address.
    let wild = 0x1_0000_0000_0000usize as *const u8;
    let c = outcome_of(|| unsafe { (p.c.print_foo)(wild) });
    let r = outcome_of(|| unsafe { (p.rust.print_foo)(wild) });
    assert_eq!(
        c, r,
        "print_foo(wild pointer): C and Rust terminated differently (C={c:?}, Rust={r:?})"
    );
    assert_eq!(
        c,
        Outcome::Signaled(libc::SIGSEGV),
        "expected the C code to fault with SIGSEGV on a wild dereference"
    );
}

/// Sanity check on the fork harness itself: a *valid* pointer must exit 0 for
/// both implementations, proving the crash tests above detect a real
/// difference rather than always reporting a signal.
#[test]
fn err_pointer_harness_sanity_valid_pointer_exits_cleanly() {
    let p = pair();
    let img = foo_image(0x2A, [0, 0, 0], -1);
    let c = outcome_of(|| unsafe { (p.c.print_foo)(img.as_ptr()) });
    let r = outcome_of(|| unsafe { (p.rust.print_foo)(img.as_ptr()) });
    assert_eq!(c, Outcome::Exited(0), "C should not crash on a valid pointer");
    assert_eq!(
        r,
        Outcome::Exited(0),
        "Rust should not crash on a valid pointer"
    );
}
