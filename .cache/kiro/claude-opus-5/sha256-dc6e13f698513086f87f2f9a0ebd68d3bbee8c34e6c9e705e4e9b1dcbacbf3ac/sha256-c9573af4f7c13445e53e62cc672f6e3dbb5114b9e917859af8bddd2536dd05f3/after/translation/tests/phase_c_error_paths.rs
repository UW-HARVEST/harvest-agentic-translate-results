//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Both libraries are driven only through
//! their `.so` exports.
//!
//! The C library has no error *returns* (both functions are `void`), so
//! "returns the same error/rejection" means one of two concrete things,
//! depending on the row:
//!
//! * an **implicit rejection**: the invalid input is silently masked, and the
//!   observable rejection is the exact byte string printed. Asserting the two
//!   libraries print the same bytes is asserting they reject identically. Each
//!   such test additionally pins the *expected* C behaviour independently (the
//!   masking rule read off `objdump`), so a test cannot pass by both sides
//!   being wrong in the same way.
//! * a **fatal rejection**: the process dies. Asserted as an identical
//!   termination signal, observed in a forked child so the runner survives.

mod common;

use common::*;

const SEED: u64 = 0xE770_0000_0BAD_0001;

fn libs() -> Libs {
    Libs::load()
}

/// The byte string the C is *specified* (by its compiled masking) to print.
fn expected_line(x: u32, y: u32, b: u8, z: i32) -> Vec<u8> {
    format!("{} {} {} {}\n", x & 3, y & 7, b & 1, z).into_bytes()
}

/// Run `driver` in both libraries and additionally check the output against the
/// independently-derived expected line, so a shared bug cannot hide.
#[track_caller]
fn check_driver(l: &Libs, x: u32, y: u32, b: u8, z: i32) {
    let (c_out, r_out) = diff_driver(l, x, y, b, z);
    let want = expected_line(x, y, b, z);
    assert_eq!(
        c_out,
        want,
        "C driver({x},{y},{b},{z}) printed {:?}, masking rule says {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&want)
    );
    assert_eq!(
        c_out,
        r_out,
        "driver({x},{y},{b},{z}): C {:?} vs Rust {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

// === Row 1: x > 3 =========================================================

#[test]
fn err01_x_out_of_range_is_silently_truncated() {
    let l = libs();
    // The row's named trigger.
    check_driver(&l, 4, 0, false as u8, 0);
    // No error channel exists: the call still prints exactly one line.
    let (c_out, r_out) = diff_driver(&l, 4, 0, 0, 0);
    assert_eq!(c_out, b"0 0 0 0\n");
    assert_eq!(r_out, b"0 0 0 0\n");
    // Every residue class, and one step past the boundary in both directions.
    for x in [3u32, 4, 5, 6, 7, 8, 9, 10, 11] {
        check_driver(&l, x, 1, 1, -1);
    }
}

// === Row 2: x == UINT_MAX =================================================

#[test]
fn err02_x_uint_max() {
    let l = libs();
    check_driver(&l, u32::MAX, 0, 0, 0);
    let (c_out, r_out) = diff_driver(&l, u32::MAX, 0, 0, 0);
    assert_eq!(c_out, b"3 0 0 0\n", "UINT_MAX & 3 == 3");
    assert_eq!(c_out, r_out);
    for x in [u32::MAX, u32::MAX - 1, u32::MAX - 2, u32::MAX - 3, 1u32 << 31] {
        check_driver(&l, x, 7, 1, i32::MIN);
    }
}

// === Row 3: y > 7 =========================================================

#[test]
fn err03_y_out_of_range_is_silently_truncated() {
    let l = libs();
    check_driver(&l, 0, 8, 0, 0);
    let (c_out, r_out) = diff_driver(&l, 0, 8, 0, 0);
    assert_eq!(c_out, b"0 0 0 0\n");
    assert_eq!(c_out, r_out);
    for y in 7u32..=24 {
        check_driver(&l, 2, y, 1, 99);
    }
}

// === Row 4: y == UINT_MAX =================================================

#[test]
fn err04_y_uint_max() {
    let l = libs();
    check_driver(&l, 0, u32::MAX, 0, 0);
    let (c_out, r_out) = diff_driver(&l, 0, u32::MAX, 0, 0);
    assert_eq!(c_out, b"0 7 0 0\n", "UINT_MAX & 7 == 7");
    assert_eq!(c_out, r_out);
    for y in [u32::MAX, u32::MAX - 1, u32::MAX - 7, u32::MAX - 8, 1u32 << 31] {
        check_driver(&l, 3, y, 1, i32::MAX);
    }
}

// === Row 5: non-canonical `_Bool` byte ====================================

#[test]
fn err05_noncanonical_bool_byte() {
    let l = libs();
    // The row's two named cases: 2 must read as false, 3 as true.
    let (c2, r2) = diff_driver(&l, 0, 0, 2, 0);
    assert_eq!(c2, b"0 0 0 0\n", "b=2 masks to 0, it is NOT normalised to 1");
    assert_eq!(c2, r2);

    let (c3, r3) = diff_driver(&l, 0, 0, 3, 0);
    assert_eq!(c3, b"0 0 1 0\n", "b=3 masks to 1");
    assert_eq!(c3, r3);

    // Exhaustive over every byte a foreign caller can put in the slot.
    for b in 0u8..=255 {
        check_driver(&l, 1, 5, b, -12345);
    }
}

// === Row 6: b == 0xFF =====================================================

#[test]
fn err06_bool_all_bits_set() {
    let l = libs();
    let (c_out, r_out) = diff_driver(&l, 0, 0, 0xFF, 0);
    assert_eq!(c_out, b"0 0 1 0\n", "0xFF & 1 == 1");
    assert_eq!(c_out, r_out);
    check_driver(&l, 0xFFFF_FFFF, 0xFFFF_FFFF, 0xFF, -1);
}

// === Row 7: dirty upper bits in the bool argument register ================

#[test]
fn err07_bool_dirty_upper_argument_bits_ignored() {
    let l = libs();
    type DriverWide = unsafe extern "C" fn(u32, u32, u32, i32);
    let c_wide: libloading::Symbol<DriverWide> = unsafe { l.c.get(b"driver\0").unwrap() };
    let r_wide: libloading::Symbol<DriverWide> = unsafe { l.rust.get(b"driver\0").unwrap() };

    // Values whose low byte is 0 or 1 but whose upper 24 bits are garbage.
    for wide in [
        0xDEAD_BE00u32,
        0xDEAD_BE01,
        0xFFFF_FF00,
        0xFFFF_FF01,
        0x0000_0100,
        0x0000_0101,
        0x8000_0002,
    ] {
        let c_out = capture_stdout(|| unsafe { c_wide(0, 0, wide, 0) });
        let r_out = capture_stdout(|| unsafe { r_wide(0, 0, wide, 0) });
        let want = format!("0 0 {} 0\n", (wide as u8) & 1).into_bytes();
        assert_eq!(
            c_out,
            want,
            "C used more than the low byte of the bool argument for {wide:#x}: {:?}",
            String::from_utf8_lossy(&c_out)
        );
        assert_eq!(c_out, r_out, "wide bool {wide:#x} diverged");
    }
}

// === Row 8: z == INT_MIN ==================================================

#[test]
fn err08_z_int_min() {
    let l = libs();
    let (c_out, r_out) = diff_driver(&l, 0, 0, 0, i32::MIN);
    assert_eq!(c_out, b"0 0 0 -2147483648\n", "INT_MIN stored verbatim");
    assert_eq!(c_out, r_out);
    check_driver(&l, 3, 7, 1, i32::MIN);
    check_driver(&l, 3, 7, 1, i32::MIN + 1);
}

// === Row 9: z == INT_MAX ==================================================

#[test]
fn err09_z_int_max() {
    let l = libs();
    let (c_out, r_out) = diff_driver(&l, 0, 0, 0, i32::MAX);
    assert_eq!(c_out, b"0 0 0 2147483647\n");
    assert_eq!(c_out, r_out);
    check_driver(&l, 3, 7, 1, i32::MAX);
    check_driver(&l, 3, 7, 1, i32::MAX - 1);
}

// === Row 10: negative z printed signed, unlike the unsigned bit-fields ====

#[test]
fn err10_negative_z_is_printed_signed() {
    let l = libs();
    let (c_out, r_out) = diff_driver(&l, 0, 0, 0, -1);
    assert_eq!(
        c_out, b"0 0 0 -1\n",
        "`%d` must print -1, not 4294967295"
    );
    assert_eq!(c_out, r_out);
    for z in [-1i32, -2, -100, -1000000, -2147483647] {
        check_driver(&l, 1, 2, 1, z);
    }
}

// === Row 11: print_foo(NULL) ==============================================

#[test]
fn err11_print_foo_null_pointer_same_fatal_signal() {
    let l = libs();
    let c = l.c_print_foo();
    let r = l.rust_print_foo();

    let c_outcome = run_in_child(|| unsafe { c(std::ptr::null()) });
    let r_outcome = run_in_child(|| unsafe { r(std::ptr::null()) });

    // The C has no null check and returns void, so the only observable
    // "rejection" is the fatal signal. Pin it explicitly rather than accepting
    // "both died somehow".
    assert_eq!(
        c_outcome,
        Outcome::Signalled(11),
        "expected C print_foo(NULL) to raise SIGSEGV, got {c_outcome:?}"
    );
    assert_eq!(
        r_outcome, c_outcome,
        "print_foo(NULL): C {c_outcome:?} vs Rust {r_outcome:?}"
    );
}

/// Also check `driver` itself never faults for any argument combination: it
/// takes no pointers, so there is no null-pointer path into it. A control that
/// the child-process machinery reports success correctly.
#[test]
fn err11b_driver_takes_no_pointer_so_never_faults() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    for &(x, y, b, z) in &[
        (0u32, 0u32, 0u8, 0i32),
        (u32::MAX, u32::MAX, 0xFF, i32::MIN),
        (u32::MAX, u32::MAX, 0xFF, i32::MAX),
    ] {
        assert_eq!(
            run_in_child(|| unsafe { c(x, y, b, z) }),
            Outcome::Exited(0),
            "C driver({x},{y},{b},{z}) did not exit cleanly"
        );
        assert_eq!(
            run_in_child(|| unsafe { r(x, y, b, z) }),
            Outcome::Exited(0),
            "Rust driver({x},{y},{b},{z}) did not exit cleanly"
        );
    }
}

// === Row 12: garbage padding bits must not be rejected or observed ========

#[test]
fn err12_garbage_padding_is_ignored_by_both() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..2000 {
        let byte0 = rng.next_u8();
        let z = rng.next_i32();
        let mut pad_a = [0u8; 3];
        let mut pad_b = [0u8; 3];
        rng.fill(&mut pad_a);
        rng.fill(&mut pad_b);

        let a = foo_bytes(byte0, pad_a, z);
        let b = foo_bytes(byte0, pad_b, z);

        let (c_a, r_a) = diff_print_foo(&l, &a);
        let (c_b, r_b) = diff_print_foo(&l, &b);

        // Independently derived expectation.
        let want = format!(
            "{} {} {} {}\n",
            byte0 & 3,
            (byte0 >> 2) & 7,
            (byte0 >> 5) & 1,
            z
        )
        .into_bytes();
        assert_eq!(c_a, want, "C print_foo decode for byte0={byte0:#02x}");
        assert_eq!(c_a, c_b, "C output changed with padding {pad_a:?} vs {pad_b:?}");
        assert_eq!(c_a, r_a);
        assert_eq!(c_b, r_b);
    }

    // Bits 6-7 of byte 0 are padding too: setting them must change nothing.
    for low6 in 0u8..64 {
        let base = foo_bytes(low6, [0, 0, 0], 0x1234_5678);
        let want = diff_print_foo(&l, &base);
        assert_eq!(want.0, want.1);
        for high in [0x40u8, 0x80, 0xC0] {
            let raw = foo_bytes(low6 | high, [0, 0, 0], 0x1234_5678);
            let got = diff_print_foo(&l, &raw);
            assert_eq!(
                got.0, want.0,
                "C: padding bits 6-7 ({high:#02x}) changed the output"
            );
            assert_eq!(got.0, got.1, "print_foo({raw:02x?}) C vs Rust");
        }
    }
}

// === Row 13: misaligned pointer is accepted, not rejected =================

#[test]
fn err13_misaligned_pointer_accepted_not_rejected() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);

    // Must not fault in either library, and must print the same thing.
    let c = l.c_print_foo();
    let r = l.rust_print_foo();
    #[repr(align(8))]
    struct Aligned([u8; 16]);

    for _ in 0..500 {
        let mut raw = [0u8; FOO_SIZE];
        rng.fill(&mut raw);
        for offset in 1usize..=3 {
            let mut buf = Aligned([0u8; 16]);
            buf.0[offset..offset + 8].copy_from_slice(&raw);
            let p = unsafe { buf.0.as_ptr().add(offset) };
            let c_out = capture_stdout(|| unsafe { c(p) });
            let r_out = capture_stdout(|| unsafe { r(p) });
            let want = format!(
                "{} {} {} {}\n",
                raw[0] & 3,
                (raw[0] >> 2) & 7,
                (raw[0] >> 5) & 1,
                i32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]])
            )
            .into_bytes();
            assert_eq!(
                c_out, want,
                "C print_foo at offset {offset} on {raw:02x?} printed {:?}",
                String::from_utf8_lossy(&c_out)
            );
            assert_eq!(
                c_out, r_out,
                "misaligned print_foo(offset {offset}, {raw:02x?}) diverged"
            );
        }
    }

    // And the misaligned call must not be turned into a fault by the Rust.
    let raw = foo_bytes(0xFF, [0xAA, 0xBB, 0xCC], i32::MIN);
    let outcome_c = run_in_child(|| {
        let mut buf = Aligned([0u8; 16]);
        buf.0[1..9].copy_from_slice(&raw);
        unsafe { c(buf.0.as_ptr().add(1)) };
    });
    let outcome_r = run_in_child(|| {
        let mut buf = Aligned([0u8; 16]);
        buf.0[1..9].copy_from_slice(&raw);
        unsafe { r(buf.0.as_ptr().add(1)) };
    });
    assert_eq!(outcome_c, Outcome::Exited(0), "C faulted on misaligned input");
    assert_eq!(
        outcome_r, outcome_c,
        "misaligned print_foo termination: C {outcome_c:?} vs Rust {outcome_r:?}"
    );
}

// === Row 14: exhaustive byte 0 — the "out-of-range enum value" analogue ===

#[test]
fn err14_every_byte0_value_accepted_and_decoded_identically() {
    let l = libs();
    // The packed bit-field allocation unit is the closest thing this API has to
    // an enum crossing the FFI boundary: any of the 256 byte values can arrive,
    // none is "valid" or "invalid", and the C decodes all of them by masking.
    for byte0 in 0u16..=255 {
        let byte0 = byte0 as u8;
        for &z in &[0i32, -1, i32::MIN, i32::MAX] {
            let raw = foo_bytes(byte0, [0, 0, 0], z);
            let (c_out, r_out) = diff_print_foo(&l, &raw);
            let want = format!(
                "{} {} {} {}\n",
                byte0 & 3,
                (byte0 >> 2) & 7,
                (byte0 >> 5) & 1,
                z
            )
            .into_bytes();
            assert_eq!(
                c_out,
                want,
                "C decode of byte0={byte0:#02x}: {:?}",
                String::from_utf8_lossy(&c_out)
            );
            assert_eq!(c_out, r_out, "byte0={byte0:#02x} z={z} diverged");
        }
    }
}

// === Generic FFI boundary checks (beyond the table) ======================

/// A `bool` argument is the only enum-like C type in the signature. C `_Bool`
/// nominally has two variants but accepts all 256 byte values across FFI; every
/// one is covered by `err05`. Here we additionally push a *pointer-width* dirty
/// value through the slot.
#[test]
fn generic_bool_slot_pointer_width_garbage() {
    let l = libs();
    type DriverU64 = unsafe extern "C" fn(u32, u32, u64, i32);
    let c: libloading::Symbol<DriverU64> = unsafe { l.c.get(b"driver\0").unwrap() };
    let r: libloading::Symbol<DriverU64> = unsafe { l.rust.get(b"driver\0").unwrap() };
    for wide in [
        0u64,
        1,
        0xFFFF_FFFF_FFFF_FF00,
        0xFFFF_FFFF_FFFF_FF01,
        0xDEAD_BEEF_CAFE_BA02,
        u64::MAX,
    ] {
        let c_out = capture_stdout(|| unsafe { c(0, 0, wide, 0) });
        let r_out = capture_stdout(|| unsafe { r(0, 0, wide, 0) });
        assert_eq!(c_out, r_out, "64-bit-wide bool slot {wide:#x} diverged");
    }
}

/// `print_foo` on a struct at the very end of a mapped page: the C reads bytes
/// 0 and 4..=7 only, so an 8-byte object touching the page end is fine, while a
/// pointer *past* the end faults. Both libraries must agree on both.
#[test]
fn generic_print_foo_page_boundary_and_unmapped() {
    let l = libs();
    let c = l.c_print_foo();
    let r = l.rust_print_foo();

    // A valid 8-byte object that ends exactly at a page boundary.
    let page = 4096usize;
    let mut buf = vec![0u8; page * 2];
    let base = buf.as_ptr() as usize;
    let aligned_page_end = ((base + page - 1) / page) * page;
    let off = aligned_page_end - base;
    if off >= 8 {
        let raw = foo_bytes(0x2A, [1, 2, 3], -424242);
        buf[off - 8..off].copy_from_slice(&raw);
        let p = unsafe { buf.as_ptr().add(off - 8) };
        let c_out = capture_stdout(|| unsafe { c(p) });
        let r_out = capture_stdout(|| unsafe { r(p) });
        assert_eq!(c_out, r_out, "page-boundary print_foo diverged");
    }

    // A wildly out-of-range pointer: both must die with the same signal.
    let bad = 0x1usize as *const u8;
    let oc = run_in_child(|| unsafe { c(bad) });
    let or = run_in_child(|| unsafe { r(bad) });
    assert_eq!(oc, Outcome::Signalled(11), "expected SIGSEGV, got {oc:?}");
    assert_eq!(or, oc, "bad pointer: C {oc:?} vs Rust {or:?}");

    // Non-canonical (sign-extended, non-mapped) address.
    let bad2 = 0xFFFF_8000_0000_0000u64 as usize as *const u8;
    let oc2 = run_in_child(|| unsafe { c(bad2) });
    let or2 = run_in_child(|| unsafe { r(bad2) });
    assert_eq!(or2, oc2, "non-canonical pointer: C {oc2:?} vs Rust {or2:?}");
}

/// Zero-length / oversized "length" arguments do not exist in this API (there is
/// no length parameter anywhere — verified against the header). This test
/// documents that and pins the argument count, so if the C ever grew one the
/// symbol lookup below would need updating.
#[test]
fn generic_no_length_parameters_exist() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/driver.h"),
    )
    .expect("read driver.h");
    assert!(
        header.contains("void driver(unsigned int x, unsigned int y, bool b, int z);"),
        "public C API changed; revisit ERRORS.md / CONFIGS.md"
    );
    assert!(
        !header.contains("size_t") && !header.contains("len"),
        "a length parameter appeared in the public header; add rows for zero \
         and oversized lengths"
    );
}
