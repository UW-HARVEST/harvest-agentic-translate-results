//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Every symbol in this library returns `void`, so the "error code / sentinel"
//! being compared is the library's only observable result channel: the exact
//! bytes it writes to stdout (zero bytes = rejected).

mod common;

use common::*;
use std::ffi::c_char;

// ---------------------------------------------------------------- row 1
/// ERRORS row 1 — `printLine(NULL)`. `if (line != NULL)` takes its false
/// branch: no `puts`, zero bytes, no crash. Repeated to catch a Rust
/// implementation that only survives the first NULL.
#[test]
fn err_1_print_line_null() {
    let c_fn = print_line(c_lib());
    let r_fn = print_line(rust_lib());

    let c_out = capture(|| unsafe { c_fn(std::ptr::null()) });
    let r_out = capture(|| unsafe { r_fn(std::ptr::null()) });

    assert_same("err_1[printLine(NULL)]", &c_out, &r_out);
    assert_same("err_1 sentinel: rejected == zero bytes", b"", &c_out);

    let c_many = capture(|| {
        for _ in 0..100 {
            unsafe { c_fn(std::ptr::null()) }
        }
    });
    let r_many = capture(|| {
        for _ in 0..100 {
            unsafe { r_fn(std::ptr::null()) }
        }
    });
    assert_same("err_1[printLine(NULL) x100]", &c_many, &r_many);
    assert_same("err_1[x100] sentinel", b"", &c_many);

    // A NULL rejection must not disturb the following valid call in either lib.
    let ok = b"after-null\0";
    let c_after = capture(|| unsafe {
        c_fn(std::ptr::null());
        c_fn(ok.as_ptr() as *const c_char);
    });
    let r_after = capture(|| unsafe {
        r_fn(std::ptr::null());
        r_fn(ok.as_ptr() as *const c_char);
    });
    assert_same("err_1[NULL then valid]", &c_after, &r_after);
    assert_same("err_1[NULL then valid] literal", b"after-null\n", &c_after);
}

// ---------------------------------------------------------------- row 2
/// ERRORS row 2 — a lone `'\0'`: the degenerate boundary of the accepting
/// branch. `puts("")` emits exactly one newline.
#[test]
fn err_2_print_line_empty_string() {
    let empty: [u8; 1] = [0];
    let c_fn = print_line(c_lib());
    let r_fn = print_line(rust_lib());
    let c_out = capture(|| unsafe { c_fn(empty.as_ptr() as *const c_char) });
    let r_out = capture(|| unsafe { r_fn(empty.as_ptr() as *const c_char) });
    assert_same("err_2[\"\"]", &c_out, &r_out);
    assert_same("err_2 literal", b"\n", &c_out);
}

// ---------------------------------------------------------------- row 3
/// ERRORS row 3 — embedded NUL / "oversized length": the buffer holds bytes
/// after its terminator. Both must stop at the first `'\0'` and ignore the tail.
#[test]
fn err_3_print_line_embedded_nul() {
    let cases: [&[u8]; 4] = [
        b"\0trailing garbage\0more",
        b"visible\0hidden",
        b"a\0\0\0\0\0\0\0\0",
        b"\0",
    ];
    let c_fn = print_line(c_lib());
    let r_fn = print_line(rust_lib());

    for (i, buf) in cases.iter().enumerate() {
        let p = buf.as_ptr() as *const c_char;
        let c_out = capture(|| unsafe { c_fn(p) });
        let r_out = capture(|| unsafe { r_fn(p) });
        assert_same(&format!("err_3[case {i}]"), &c_out, &r_out);

        // Expected: bytes up to the first NUL, plus '\n'.
        let head = &buf[..buf.iter().position(|&b| b == 0).unwrap()];
        let mut expected = head.to_vec();
        expected.push(b'\n');
        assert_same(&format!("err_3[case {i}] literal"), &expected, &c_out);
    }
}

// ---------------------------------------------------------------- row 4
/// ERRORS row 4 — a non-NULL pointer that is an arbitrary interior offset into
/// a buffer (odd address, "looks bogus"). The guard tests only against NULL, so
/// both libraries must print from that offset.
#[test]
fn err_4_print_line_unaligned_offset() {
    let buf: Vec<u8> = b"0123456789abcdef\0".to_vec();
    let c_fn = print_line(c_lib());
    let r_fn = print_line(rust_lib());

    for off in 0..16usize {
        let p = unsafe { buf.as_ptr().add(off) } as *const c_char;
        let c_out = capture(|| unsafe { c_fn(p) });
        let r_out = capture(|| unsafe { r_fn(p) });
        assert_same(&format!("err_4[offset {off}]"), &c_out, &r_out);

        let mut expected = buf[off..16].to_vec();
        expected.push(b'\n');
        assert_same(&format!("err_4[offset {off}] literal"), &expected, &c_out);
    }
}

// ---------------------------------------------------------------- row 5
/// ERRORS row 5 — oversized length: a 4095-byte string with no interior NUL.
#[test]
fn err_5_print_line_oversized() {
    for &len in &[4095usize, 8192, 65535] {
        let mut buf = vec![b'Z'; len];
        buf.push(0);
        let p = buf.as_ptr() as *const c_char;
        let c_fn = print_line(c_lib());
        let r_fn = print_line(rust_lib());
        let c_out = capture(|| unsafe { c_fn(p) });
        let r_out = capture(|| unsafe { r_fn(p) });
        assert_same(&format!("err_5[len={len}]"), &c_out, &r_out);
        assert_eq!(c_out.len(), len + 1, "err_5[len={len}]: byte count");
        assert_eq!(*c_out.last().unwrap(), b'\n');
    }
}

// ---------------------------------------------------------------- row 6
/// ERRORS row 6 — the nullary exports are declared `void(void)`; calling them
/// through a wider FFI signature (extra register arguments, including values
/// that would be out-of-range for any enum) must be ignored identically by both
/// libraries. This is the closest analogue to "out-of-range enum across FFI" in
/// an API that declares no enum and takes no integer parameters.
#[test]
fn err_6_nullary_extra_args_ignored() {
    type WideFn = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64);

    const ARGS: [[u64; 6]; 4] = [
        [0, 0, 0, 0, 0, 0],
        [u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX],
        [0xDEAD_BEEF, 1, 2, 3, 4, 5],
        // Values with no valid variant for any plausible enum, plus negatives.
        [
            (-1i64) as u64,
            (-2147483648i64) as u64,
            999_999,
            0x7FFF_FFFF,
            0x8000_0000,
            42,
        ],
    ];

    for name in ["bad", "good", "driver"] {
        let mut sym = name.as_bytes().to_vec();
        sym.push(0);
        let c_fn = unsafe { c_lib().get::<WideFn>(&sym) }.unwrap();
        let r_fn = unsafe { rust_lib().get::<WideFn>(&sym) }.unwrap();

        // Baseline: the same symbol called with the correct nullary signature.
        let baseline = diff_nullary(name);

        for (i, a) in ARGS.iter().enumerate() {
            let c_out = capture(|| unsafe { c_fn(a[0], a[1], a[2], a[3], a[4], a[5]) });
            let r_out = capture(|| unsafe { r_fn(a[0], a[1], a[2], a[3], a[4], a[5]) });
            assert_same(&format!("err_6[{name} args {i}]"), &c_out, &r_out);
            assert_same(
                &format!("err_6[{name} args {i}] == nullary baseline"),
                &baseline,
                &c_out,
            );
        }
    }
}

// -------------------------------------------------- generic FFI boundaries

/// `printLine` is the only entry point with a pointer parameter, so the generic
/// "null pointer" boundary for the whole API surface is exactly row 1. This test
/// additionally confirms the nullary exports are unaffected by a preceding
/// rejected call, i.e. the NULL path leaves no residue in shared `stdout`.
#[test]
fn err_generic_null_does_not_poison_other_entry_points() {
    let pl_c = print_line(c_lib());
    let pl_r = print_line(rust_lib());
    let d_c = nullary(c_lib(), "driver");
    let d_r = nullary(rust_lib(), "driver");

    let c_out = capture(|| unsafe {
        pl_c(std::ptr::null());
        d_c();
        pl_c(std::ptr::null());
    });
    let r_out = capture(|| unsafe {
        pl_r(std::ptr::null());
        d_r();
        pl_r(std::ptr::null());
    });
    assert_same("err_generic[null around driver]", &c_out, &r_out);
    assert_same(
        "err_generic literal",
        b"Calling good()...\ngood()\nhelperGood()\nFinished good()\nCalling bad()...\nbad()\nFinished bad()\n",
        &c_out,
    );
}

/// The two `static` C helpers have internal linkage and must NOT be reachable
/// via `dlsym` in either library — an over-eager `#[no_mangle]` would be a real
/// (if benign-looking) divergence in the exported surface.
#[test]
fn err_generic_static_helpers_not_dlsym_reachable() {
    for name in ["helperGood", "helperBad"] {
        let mut sym = name.as_bytes().to_vec();
        sym.push(0);
        let in_c = unsafe { c_lib().get::<NullaryFn>(&sym) }.is_ok();
        let in_rust = unsafe { rust_lib().get::<NullaryFn>(&sym) }.is_ok();
        assert!(!in_c, "C unexpectedly exports `{name}`");
        assert_eq!(
            in_c, in_rust,
            "`{name}`: C exports={in_c} but Rust exports={in_rust}"
        );
    }
}
