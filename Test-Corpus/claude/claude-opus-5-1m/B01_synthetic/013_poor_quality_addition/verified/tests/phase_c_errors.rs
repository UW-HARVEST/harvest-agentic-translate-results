//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Each test constructs the exact invalid
//! input/condition, calls BOTH `.so` files, and asserts they reject (or accept)
//! it identically — same bytes on stdout, same return value, no panic/abort on
//! either side.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// ERRORS row 1 — printLine(NULL): `if (line != NULL)` is false -> no output
// ---------------------------------------------------------------------------
fn err01_print_line_null_pointer() {
    // identical behaviour...
    diff("err01 printLine(NULL)", |api| api.print_line_null());

    // ...and the rejection is specifically "zero bytes written, returns cleanly"
    for api in [c_api(), rust_api()] {
        let out = capture(|| api.print_line_null());
        assert!(
            out.is_empty(),
            "{} printLine(NULL) must write nothing, wrote {:?}",
            api.name,
            out
        );
    }

    // repeated NULL calls, and NULL interleaved with valid calls, stay in sync
    diff("err01 NULL x8", |api| {
        for _ in 0..8 {
            api.print_line_null();
        }
    });
    diff("err01 NULL interleaved", |api| {
        api.print_line_bytes(b"before");
        api.print_line_null();
        api.print_line_bytes(b"after");
        api.print_line_null();
        unsafe { (api.print_int_line)(7) };
    });
}

// ---------------------------------------------------------------------------
// ERRORS row 2 — zero length: printLine("") writes exactly "\n"
// ---------------------------------------------------------------------------
fn err02_print_line_zero_length() {
    diff("err02 printLine(\"\")", |api| api.print_line_bytes(b""));
    for api in [c_api(), rust_api()] {
        let out = capture(|| api.print_line_bytes(b""));
        assert_eq!(out, b"\n", "{} printLine(\"\")", api.name);
    }
    // empty string repeated, and mixed with NULL (accept vs reject boundary)
    diff("err02 empty/NULL mix", |api| {
        api.print_line_bytes(b"");
        api.print_line_null();
        api.print_line_bytes(b"");
    });
}

// ---------------------------------------------------------------------------
// ERRORS row 3 — oversized lengths (past BUFSIZ and past LineWriter capacity)
// ---------------------------------------------------------------------------
fn err03_print_line_oversized() {
    for &len in [65_536usize, 1 << 20].iter() {
        let s = vec![b'X'; len];
        diff(&format!("err03 len={len}"), |api| api.print_line_bytes(&s));
        for api in [c_api(), rust_api()] {
            let out = capture(|| api.print_line_bytes(&s));
            assert_eq!(
                out.len(),
                len + 1,
                "{} truncated a {len}-byte string",
                api.name
            );
        }
    }
    // 1 MiB of high bytes (also not valid UTF-8)
    let s = vec![0xffu8; 1 << 20];
    diff("err03 1MiB of 0xff", |api| api.print_line_bytes(&s));
}

// ---------------------------------------------------------------------------
// ERRORS row 4 — invalid UTF-8 must pass through verbatim (no panic, no U+FFFD)
// ---------------------------------------------------------------------------
fn err04_print_line_invalid_utf8() {
    let cases: &[&[u8]] = &[
        &[0x80],
        &[0x81, 0x82, 0x83],
        &[0xbf, 0xbf],
        &[0xc0],
        &[0xc0, 0xaf],
        &[0xc1, 0xbf],
        &[0xe0, 0x80],
        &[0xe0, 0x80, 0xaf],
        &[0xed, 0xa0, 0x80],
        &[0xed, 0xbf, 0xbf],
        &[0xf0, 0x82, 0x82, 0xac],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xf5],
        &[0xf8],
        &[0xfc],
        &[0xfe, 0xff],
        &[0xff, 0xff, 0xff, 0xff],
        &[0x41, 0xff, 0x42, 0x80, 0x43],
    ];
    for (i, s) in cases.iter().enumerate() {
        diff(&format!("err04 #{i}"), |api| api.print_line_bytes(s));
        for api in [c_api(), rust_api()] {
            let out = capture(|| api.print_line_bytes(s));
            let mut want = s.to_vec();
            want.push(b'\n');
            assert_eq!(out, want, "{} mangled invalid UTF-8 {s:?}", api.name);
        }
    }

    // exhaustive: every high byte on its own, and every high byte in context
    for b in 0x80u8..=0xff {
        let one = [b];
        diff(&format!("err04 high \\x{b:02x}"), |api| {
            api.print_line_bytes(&one)
        });
        let ctx = [b'a', b, b'b'];
        diff(&format!("err04 ctx \\x{b:02x}"), |api| api.print_line_bytes(&ctx));
    }

    // randomized raw byte soup
    let mut rng = Rng::new(SEED ^ 0x04);
    for i in 0..300 {
        let len = rng.range(1, 300);
        let s: Vec<u8> = (0..len).map(|_| 0x80 + (rng.next_u32() % 0x80) as u8).collect();
        diff(&format!("err04 random #{i}"), |api| api.print_line_bytes(&s));
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 5 — printf directives in the *data* must not be interpreted
// ---------------------------------------------------------------------------
fn err05_print_line_format_string_is_data() {
    let cases: &[&[u8]] = &[
        b"%n",
        b"%n%n%n%n%n%n%n%n",
        b"%s",
        b"%s%s%s%s%s%s%s%s%s%s%s%s",
        b"%99999999999d",
        b"%.2147483647f",
        b"%1$n",
        b"%*d",
        b"%hhn",
        b"%lln",
        b"AAAA%08x.%08x.%08x.%08x.%08x.%08x",
        b"%",
        b"%%",
        b"%%%",
    ];
    for (i, s) in cases.iter().enumerate() {
        diff(&format!("err05 #{i}"), |api| api.print_line_bytes(s));
        for api in [c_api(), rust_api()] {
            let out = capture(|| api.print_line_bytes(s));
            let mut want = s.to_vec();
            want.push(b'\n');
            assert_eq!(
                out, want,
                "{} interpreted the payload as a format string",
                api.name
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 6 — printIntLine at / one step past the int range extremes
// ---------------------------------------------------------------------------
fn err06_print_int_line_range_extremes() {
    let vals = [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MAX - 1, i32::MAX, -1, 0];
    for &v in vals.iter() {
        diff(&format!("err06 v={v}"), |api| unsafe {
            (api.print_int_line)(v as c_int)
        });
    }
    // the two textual extremes, verified against glibc's %d
    for api in [c_api(), rust_api()] {
        let out = capture(|| unsafe { (api.print_int_line)(i32::MIN as c_int) });
        assert_eq!(out, b"-2147483648\n", "{} printIntLine(INT_MIN)", api.name);
        let out = capture(|| unsafe { (api.print_int_line)(i32::MAX as c_int) });
        assert_eq!(out, b"2147483647\n", "{} printIntLine(INT_MAX)", api.name);
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 7 — out-of-range argument across the FFI boundary:
// the caller passes 64 bits where the callee declares `int`.
// (Register-passing ABI: the callee reads only the low 32 bits.)
// ---------------------------------------------------------------------------
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn err07_print_int_line_out_of_range_argument() {
    type WideFn = unsafe extern "C" fn(i64);

    let vals: [i64; 12] = [
        0x0000_0001_0000_0001u64 as i64,
        0x0000_0001_0000_0000u64 as i64,
        0xffff_ffff_ffff_ffffu64 as i64,
        0xffff_ffff_0000_0000u64 as i64,
        0x7fff_ffff_ffff_ffffu64 as i64,
        0x8000_0000_0000_0000u64 as i64,
        0xdead_beef_dead_beefu64 as i64,
        0x0000_0000_8000_0000u64 as i64,
        0x0000_0000_7fff_ffffu64 as i64,
        0x1234_5678_9abc_def0u64 as i64,
        i64::from(i32::MIN) - 1,
        i64::from(i32::MAX) + 1,
    ];

    for &v in vals.iter() {
        let c = capture(|| unsafe {
            let f: WideFn = std::mem::transmute(c_api().print_int_line);
            f(v)
        });
        let r = capture(|| unsafe {
            let f: WideFn = std::mem::transmute(rust_api().print_int_line);
            f(v)
        });
        assert_bytes_eq(&format!("err07 wide arg {v:#018x}"), &c, &r);
        // both must have seen the truncated 32-bit value
        let want = format!("{}\n", v as i32).into_bytes();
        assert_eq!(c, want, "C did not truncate {v:#018x} to int");
    }

    // randomized wide values
    let mut rng = Rng::new(SEED ^ 0x07);
    for i in 0..500 {
        let v = rng.next_u64() as i64;
        let c = capture(|| unsafe {
            let f: WideFn = std::mem::transmute(c_api().print_int_line);
            f(v)
        });
        let r = capture(|| unsafe {
            let f: WideFn = std::mem::transmute(rust_api().print_int_line);
            f(v)
        });
        assert_bytes_eq(&format!("err07 random #{i} {v:#018x}"), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 8 — main with degenerate argc/argv (never dereferenced by C)
// ---------------------------------------------------------------------------
fn err08_main_degenerate_argc_argv() {
    // argv = NULL with assorted argc values, including negative
    for &argc in [0i32, -1, 1, 2, i32::MIN, i32::MAX].iter() {
        diff_with(&format!("err08 main({argc}, NULL)"), |api| {
            api.call_main(argc as c_int, std::ptr::null_mut())
        });
    }

    // argv present but argc lies about its size (C never reads it)
    for &argc in [0i32, 1, 5, -3, i32::MAX].iter() {
        diff_with(&format!("err08 main({argc}, argv[1])"), |api| {
            let argv = Argv::new(&["driver"]);
            api.call_main(argc as c_int, argv.as_ptr())
        });
    }

    // argv array whose first entry is NULL
    diff_with("err08 main(1, [NULL])", |api| {
        let mut arr: [*mut c_char; 2] = [std::ptr::null_mut(), std::ptr::null_mut()];
        api.call_main(1, arr.as_mut_ptr())
    });

    // return value must be exactly 0 on both sides
    for api in [c_api(), rust_api()] {
        let (_, ret) = capture_ret(|| api.call_main(0, std::ptr::null_mut()));
        assert_eq!(ret, 0, "{} main must return 0", api.name);
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundaries not tied to a specific ERRORS row
// ---------------------------------------------------------------------------

/// `printLine` is the only pointer-taking entry point; make sure an
/// *unaligned* / interior pointer into a larger buffer behaves the same
/// (valid C, just not the start of an allocation).
fn err_generic_interior_pointer() {
    let buf = b"0123456789abcdef\0tail".to_vec();
    for off in 0..16usize {
        let c = capture(|| unsafe { (c_api().print_line)(buf.as_ptr().add(off) as *const c_char) });
        let r =
            capture(|| unsafe { (rust_api().print_line)(buf.as_ptr().add(off) as *const c_char) });
        assert_bytes_eq(&format!("interior pointer off={off}"), &c, &r);
    }
}

/// There is no enum in the C API (see ERRORS.md); the closest analogue is
/// feeding every "shape" of 32-bit bit pattern to the only int parameter.
fn err_generic_all_int_bit_patterns() {
    let mut vals: Vec<i32> = Vec::new();
    for bit in 0..32 {
        vals.push(1i32 << bit);
        vals.push(!(1i32 << bit));
        vals.push((1i32 << bit).wrapping_neg());
    }
    vals.extend_from_slice(&[0, -1, i32::MIN, i32::MAX, 0x5555_5555, 0x3333_3333u32 as i32]);
    for (i, &v) in vals.iter().enumerate() {
        diff(&format!("bit pattern #{i} v={v}"), |api| unsafe {
            (api.print_int_line)(v as c_int)
        });
    }
}

/// Every exported symbol must exist in BOTH shared objects (dlsym parity).
fn err_generic_symbol_lookup_parity() {
    // resolving all five symbols in both libraries is what `c_api()`/`rust_api()`
    // do; a missing symbol panics there.
    let c = c_api();
    let r = rust_api();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "Rust");
    // and they must be distinct implementations
    assert_ne!(c.print_line as usize, r.print_line as usize);
    assert_ne!(c.print_int_line as usize, r.print_int_line as usize);
    assert_ne!(c.bad as usize, r.bad as usize);
    assert_ne!(c.good as usize, r.good as usize);
    assert_ne!(c.main as usize, r.main as usize);
}

// ---------------------------------------------------------------------------
// runner — one entry per ERRORS.md row, executed sequentially
// ---------------------------------------------------------------------------
fn main() {
    let mut s = Suite::new("Phase C — error paths (ERRORS.md)");
    s.run("ERRORS row1 printLine(NULL)", err01_print_line_null_pointer);
    s.run("ERRORS row2 printLine zero length", err02_print_line_zero_length);
    s.run("ERRORS row3 printLine oversized length", err03_print_line_oversized);
    s.run("ERRORS row4 printLine invalid UTF-8", err04_print_line_invalid_utf8);
    s.run("ERRORS row5 printLine format string is data", err05_print_line_format_string_is_data);
    s.run("ERRORS row6 printIntLine range extremes", err06_print_int_line_range_extremes);
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    s.run(
        "ERRORS row7 printIntLine out-of-range FFI argument",
        err07_print_int_line_out_of_range_argument,
    );
    s.run("ERRORS row8 main degenerate argc/argv", err08_main_degenerate_argc_argv);
    s.run("generic interior pointer", err_generic_interior_pointer);
    s.run("generic all int bit patterns", err_generic_all_int_bit_patterns);
    s.run("generic dlsym parity", err_generic_symbol_lookup_parity);
    s.finish();
}
