//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Neither `driver` nor `print_foo` can return an error (both are `void` and the
//! C contains zero `return`/`assert`/null-check/range-check statements), so the
//! comparable "result" for each rejection is the pair
//!
//!   (bytes written to stdout, how the call terminated)
//!
//! Every test below asserts BOTH halves match between the C and the Rust `.so`,
//! so "both crashed somehow" is not accepted — the terminating signal itself is
//! compared.

mod harness;

use harness::*;

/// Run one `driver` call in isolation and report output + termination.
fn driver_once(f: &DriverFn, x: u32, y: u32, b: u32, z: i32) -> (Vec<u8>, ChildOutcome) {
    capture_with_outcome(|| unsafe { f(x, y, b, z) })
}

/// Compare a batch of `driver` cases on output *and* termination status.
fn diff_driver(row: &str, cases: &[(u32, u32, u32, i32)]) {
    let (c, rs) = (c_driver(), rs_driver());
    let c_res = capture_with_outcome(|| {
        for &(x, y, b, z) in cases {
            unsafe { c(x, y, b, z) };
        }
    });
    let rs_res = capture_with_outcome(|| {
        for &(x, y, b, z) in cases {
            unsafe { rs(x, y, b, z) };
        }
    });
    assert_eq!(
        c_res.1, rs_res.1,
        "{row}: C and Rust terminated differently ({:?} vs {:?})",
        c_res.1, rs_res.1
    );
    assert_same(row, cases, &c_res.0, &rs_res.0);
}

// ===========================================================================
// Row 1 — print_foo(NULL): no null check in C, so an unchecked dereference.
// ===========================================================================

#[test]
fn err01_print_foo_null_segv_both() {
    let c = c_print_foo();
    let rs = rs_print_foo();

    let (c_out, c_end) = capture_with_outcome(|| unsafe { c(std::ptr::null()) });
    let (rs_out, rs_end) = capture_with_outcome(|| unsafe { rs(std::ptr::null()) });

    // The C library faults rather than reporting an error; the Rust must fault
    // the same way, with the same signal, and must not print anything first.
    assert_eq!(
        c_end,
        ChildOutcome::Signaled(11),
        "expected the C print_foo(NULL) to raise SIGSEGV, got {c_end:?}"
    );
    assert_eq!(
        rs_end, c_end,
        "print_foo(NULL): C ended {c_end:?} but Rust ended {rs_end:?}"
    );
    assert_eq!(
        c_out, rs_out,
        "print_foo(NULL): output differs (C {c_out:?}, Rust {rs_out:?})"
    );
    assert!(
        c_out.is_empty(),
        "expected no output before the fault, got {c_out:?}"
    );
}

// ===========================================================================
// Rows 2-5 — values one step past, and far past, each bit-field's range.
// The C silently truncates; the Rust must truncate identically.
// ===========================================================================

#[test]
fn err_driver_truncation_boundaries() {
    let mut cases = Vec::new();

    // Row 2: x = 4 (one past the `x : 2` max of 3) and the rest of the
    // immediate neighbourhood; Row 3: x = UINT_MAX.
    for x in [3u32, 4, 5, 6, 7, 8, u32::MAX, u32::MAX - 1, 0x8000_0000] {
        cases.push((x, 0, 0, 0));
    }
    // Row 4: y = 8 (one past the `y : 3` max of 7); Row 5: y = UINT_MAX.
    for y in [7u32, 8, 9, 15, 16, u32::MAX, u32::MAX - 1, 0x8000_0000] {
        cases.push((0, y, 0, 0));
    }
    // Both out of range simultaneously.
    for x in [4u32, u32::MAX] {
        for y in [8u32, u32::MAX] {
            cases.push((x, y, 0, 0));
        }
    }

    diff_driver("err rows 2-5: x/y truncation boundaries", &cases);

    // Pin the documented expectations from ERRORS.md against the C itself, so
    // the table is validated and not merely asserted to be self-consistent.
    let c = c_driver();
    let expect = |x: u32, y: u32, want: &str| {
        let (out, end) = driver_once(&c, x, y, 0, 0);
        assert_eq!(end, ChildOutcome::Exited(0));
        assert_eq!(
            String::from_utf8_lossy(&out).trim_end(),
            want,
            "C driver({x}, {y}, 0, 0)"
        );
    };
    expect(4, 0, "0 0 0 0"); // row 2: 4 & 3 == 0
    expect(u32::MAX, 0, "3 0 0 0"); // row 3: 0xFFFFFFFF & 3 == 3
    expect(0, 8, "0 0 0 0"); // row 4: 8 & 7 == 0
    expect(0, u32::MAX, "0 7 0 0"); // row 5: 0xFFFFFFFF & 7 == 7
}

// ===========================================================================
// Rows 6-8 — out-of-range `_Bool`, the classic FFI blind spot.
//
// A C enum/_Bool accepts any int across the FFI boundary. GCC compiles the
// 1-bit bit-field store as `and $0x1` — it masks bit 0 rather than testing for
// non-zero — so b = 2 yields 0, not 1. Every byte value plus a range of
// 32-bit values with a zero low byte are checked.
// ===========================================================================

#[test]
fn err_driver_bool_out_of_range_all_ints() {
    // Rows 6 and 7: every possible byte value, in range and out.
    let mut cases: Vec<(u32, u32, u32, i32)> = (0u32..=255).map(|b| (1, 2, b, -7)).collect();

    // Row 8: non-zero values whose low byte is zero, plus assorted wide values.
    for b in [
        0x100u32,
        0x1FE,
        0x200,
        0xFF00,
        0x1_0000,
        0x7FFF_FF00,
        0x8000_0000,
        0xFFFF_FF00,
        0xFFFF_FFFF,
        0xDEAD_BE00,
        0xDEAD_BE01,
    ] {
        cases.push((1, 2, b, -7));
    }

    diff_driver("err rows 6-8: out-of-range _Bool", &cases);

    // Pin the exact documented semantics against the C library.
    let c = c_driver();
    let field_of = |b: u32| -> String {
        let (out, end) = driver_once(&c, 0, 0, b, 0);
        assert_eq!(end, ChildOutcome::Exited(0));
        let s = String::from_utf8_lossy(&out).trim_end().to_string();
        s.split(' ').nth(2).unwrap().to_string()
    };
    assert_eq!(field_of(0), "0");
    assert_eq!(field_of(1), "1");
    // Row 6: the value one past the valid range does NOT become 1.
    assert_eq!(field_of(2), "0", "b=2 must mask to bit 0, not test non-zero");
    assert_eq!(field_of(3), "1");
    assert_eq!(field_of(254), "0");
    assert_eq!(field_of(255), "1");
    // Row 8: only the low byte reaches the callee, and only its bit 0 matters.
    assert_eq!(field_of(0x100), "0");
    assert_eq!(field_of(0xFFFF_FF00), "0");
    assert_eq!(field_of(0xFFFF_FFFF), "1");
}

// ===========================================================================
// Rows 9-10 — the `int z` member's extreme values.
// ===========================================================================

#[test]
fn err_driver_z_extremes() {
    let mut cases = Vec::new();
    for z in [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        -2147483647,
    ] {
        for (x, y, b) in [(0u32, 0u32, 0u32), (3, 7, 1), (u32::MAX, u32::MAX, u32::MAX)] {
            cases.push((x, y, b, z));
        }
    }
    diff_driver("err rows 9-10: z extremes", &cases);

    let c = c_driver();
    let (out, end) = driver_once(&c, 0, 0, 0, i32::MIN);
    assert_eq!(end, ChildOutcome::Exited(0));
    assert_eq!(
        String::from_utf8_lossy(&out).trim_end(),
        "0 0 0 -2147483648",
        "row 9: INT_MIN must print verbatim"
    );
    let (out, _) = driver_once(&c, 0, 0, 0, i32::MAX);
    assert_eq!(
        String::from_utf8_lossy(&out).trim_end(),
        "0 0 0 2147483647",
        "row 10: INT_MAX must print verbatim"
    );
}

// ===========================================================================
// Row 11 — a foo_t image whose padding bits (6..7) are set: bits no field owns.
// ===========================================================================

#[test]
fn err_print_foo_padding_bits_ignored() {
    let cp = c_print_foo();
    let rp = rs_print_foo();

    // Every storage byte, so every padding-bit combination is covered.
    let images: Vec<FooImage> = (0u16..=255)
        .map(|s| FooImage::new(s as u8, 0x5A5A_5A5A))
        .collect();

    let c_res = capture_with_outcome(|| {
        for i in &images {
            unsafe { cp(i.as_ptr()) };
        }
    });
    let rs_res = capture_with_outcome(|| {
        for i in &images {
            unsafe { rp(i.as_ptr()) };
        }
    });
    assert_eq!(c_res.1, rs_res.1, "row 11: termination differs");
    assert_same("err row 11: padding bits", &images, &c_res.0, &rs_res.0);

    // And confirm against the C that padding really is ignored: setting bits
    // 6/7 must not change the output for the same low 6 bits.
    for low in 0u8..64 {
        let base = capture(|| unsafe { cp(FooImage::new(low, 1).as_ptr()) });
        for pad in [0x40u8, 0x80, 0xC0] {
            let with_pad = capture(|| unsafe { cp(FooImage::new(low | pad, 1).as_ptr()) });
            assert_eq!(
                base, with_pad,
                "row 11: C output changed when padding bits {pad:#x} were set (low={low:#x})"
            );
        }
    }
}

// ===========================================================================
// Row 12 — misaligned foo_t pointer. foo_t requires 4-byte alignment; x86-64
// tolerates unaligned loads and the C performs no check, so the call must
// succeed identically rather than fault or abort.
// ===========================================================================

#[test]
fn err_print_foo_misaligned_pointer() {
    #[repr(C, align(4))]
    struct Buf([u8; 16]);

    let cp = c_print_foo();
    let rp = rs_print_foo();

    for off in 0usize..4 {
        for &(storage, z) in &[
            (0x00u8, 0i32),
            (0xFF, -1),
            (0x2A, i32::MIN),
            (0xC3, i32::MAX),
            (0x15, 123456),
        ] {
            let run = |f: &PrintFooFn| {
                capture_with_outcome(|| {
                    let mut buf = Buf([0u8; 16]);
                    buf.0[off..off + 8].copy_from_slice(&FooImage::new(storage, z).0);
                    unsafe { f(buf.0.as_ptr().add(off)) };
                })
            };
            let (c_out, c_end) = run(&cp);
            let (rs_out, rs_end) = run(&rp);
            assert_eq!(
                c_end, rs_end,
                "row 12: offset {off} storage {storage:#x} z {z}: C ended {c_end:?}, Rust {rs_end:?}"
            );
            assert_eq!(
                c_out, rs_out,
                "row 12: offset {off} storage {storage:#x} z {z}: output differs \
                 (C {:?}, Rust {:?})",
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&rs_out)
            );
            // The misaligned call must actually succeed, matching C.
            assert_eq!(
                c_end,
                ChildOutcome::Exited(0),
                "row 12: expected the C library to tolerate a misaligned pointer"
            );
        }
    }
}

// ===========================================================================
// Generic FFI boundaries beyond the table.
// ===========================================================================

/// A `foo_t *` pointing at unmapped-but-non-NULL memory: another pointer the C
/// dereferences without checking.
#[test]
fn err_print_foo_wild_pointer_faults_identically() {
    let cp = c_print_foo();
    let rp = rs_print_foo();
    for addr in [1usize, 0xF, 0x1000, 0xDEAD_BEEF, usize::MAX & !3] {
        let p = addr as *const u8;
        let c_end = run_in_child(|| unsafe { cp(p) });
        let rs_end = run_in_child(|| unsafe { rp(p) });
        assert_eq!(
            c_end, rs_end,
            "wild pointer {addr:#x}: C ended {c_end:?} but Rust ended {rs_end:?}"
        );
    }
}

/// `driver` has no pointer parameters, so no argument can be "null"; the
/// equivalent boundary is the all-zero and all-ones argument vectors.
#[test]
fn err_driver_extreme_argument_vectors() {
    let cases = [
        (0u32, 0u32, 0u32, 0i32),
        (u32::MAX, u32::MAX, u32::MAX, -1),
        (u32::MAX, u32::MAX, u32::MAX, i32::MIN),
        (u32::MAX, u32::MAX, u32::MAX, i32::MAX),
        (0, 0, 0, i32::MIN),
        (0x8000_0000, 0x8000_0000, 0x8000_0000, i32::MIN),
    ];
    diff_driver("generic: extreme argument vectors", &cases);
}

/// Repeated calls must not accumulate state (the C `driver` builds `foo_t` on
/// the stack and ORs into whatever bits were there, so a stale-padding bug
/// would show up as output drifting between the first and later calls).
#[test]
fn err_driver_no_state_leak_across_repeats() {
    let cases: Vec<(u32, u32, u32, i32)> = std::iter::repeat((3u32, 7u32, 1u32, -1i32))
        .take(200)
        .collect();
    diff_driver("generic: repeated identical calls", &cases);

    // Every line must be identical to the first: no drift.
    let c = c_driver();
    let out = capture(|| {
        for _ in 0..200 {
            unsafe { c(3, 7, 1, -1) };
        }
    });
    let text = String::from_utf8_lossy(&out);
    let mut lines = text.lines();
    let first = lines.next().unwrap();
    assert_eq!(first, "3 7 1 -1");
    for (i, l) in text.lines().enumerate() {
        assert_eq!(l, first, "line {i} drifted from the first line");
    }
}
