//! Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//!
//! Every case is driven from one `#[test]` because stdout capture manipulates
//! the process-global file descriptor 1; running cases in parallel would let
//! libtest's own progress output land inside a capture window.
//!
//! Cases are ordered from the lowest-level export (`printLine`) upward through
//! `bad` / `good` to the public entry point `driver`.

mod common;

use std::ffi::{CString, c_char, c_int};

use common::{FnDriver, FnPrintLine, FnVoid, Report, capture_stdout, impls, show, sym};

#[test]
fn rust_so_matches_c_so_for_every_export() {
    let mut report = Report::new();

    print_line_cases(&mut report);
    bad_and_good_cases(&mut report);
    driver_cases(&mut report);
    cross_level_consistency(&mut report);

    report.finish();
}

// ---------------------------------------------------------------------------
// Level 0: printLine(const char *)
// ---------------------------------------------------------------------------

fn print_line_cases(report: &mut Report) {
    let libs = impls();
    let c: libloading::Symbol<FnPrintLine> = sym(libs.c, "printLine");
    let r: libloading::Symbol<FnPrintLine> = sym(libs.rust, "printLine");

    let one = |label: String, arg: *const c_char, report: &mut Report| {
        let c_out = capture_stdout(|| unsafe { c(arg) });
        let rust_out = capture_stdout(|| unsafe { r(arg) });
        report.check(&label, &c_out, &rust_out);
    };

    // NULL must produce no output at all.
    one("printLine(NULL)".to_string(), std::ptr::null(), report);

    // Plain strings, whitespace and embedded newlines.
    let plain: &[&str] = &[
        "",
        " ",
        "a",
        "hello",
        "helperBad string",
        "helperGood1 string",
        "line with trailing space ",
        "\ttab-indented",
        "multi\nline\nembedded",
        "trailing newline\n",
        "\n",
        "\n\n\n",
        "carriage\rreturn",
        "\x01\x02\x03\x04\x05\x06\x07\x08\x0b\x0c\x0e\x0f",
        "\x1b[31mansi\x1b[0m",
        "\x7f",
    ];
    for s in plain {
        let owned = CString::new(*s).expect("no interior NUL");
        one(format!("printLine({s:?})"), owned.as_ptr(), report);
    }

    // The C side passes the string as an *argument* to "%s\n" (gcc lowers it to
    // puts), so specifiers must be emitted literally, never interpreted.
    let specifiers: &[&str] = &[
        "%s",
        "%d",
        "%n",
        "%p",
        "%%",
        "%s%s%s%s%s",
        "%99999999d",
        "%.*f",
        "%1$s",
        "100% done",
        "{}",
        "{0}",
        "{:?}",
    ];
    for s in specifiers {
        let owned = CString::new(*s).unwrap();
        one(format!("printLine({s:?})"), owned.as_ptr(), report);
    }

    // Non-ASCII, invalid UTF-8 and every possible non-NUL byte value.
    let mut byte_cases: Vec<Vec<u8>> = vec![
        "héllo wörld".as_bytes().to_vec(),
        "日本語テキスト".as_bytes().to_vec(),
        "emoji \u{1F600}\u{1F4A9}".as_bytes().to_vec(),
        "“smart quotes”".as_bytes().to_vec(),
        vec![0x80],
        vec![0xff],
        vec![0xff, 0xfe, 0xfd],
        vec![0xc3],
        vec![0xed, 0xa0, 0x80],
        vec![0xf5, 0x80, 0x80, 0x80],
        (1u8..=255).collect(),
    ];
    byte_cases.push((1u8..=255).rev().collect());
    // Each individual byte value on its own, to rule out any sign-extension or
    // char-signedness discrepancy.
    for b in 1u8..=255 {
        byte_cases.push(vec![b]);
    }
    for bytes in &byte_cases {
        let owned = CString::new(bytes.clone()).expect("no interior NUL");
        one(
            format!("printLine(b\"{}\")", show(bytes)),
            owned.as_ptr(),
            report,
        );
    }

    // Lengths spanning the stdio buffer boundaries in both directions.
    for len in [
        1usize, 2, 15, 16, 17, 63, 64, 65, 127, 128, 129, 511, 1023, 1024, 1025, 4095, 4096, 4097,
        8191, 8192, 65535, 65536, 100_000,
    ] {
        let body: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let owned = CString::new(body).unwrap();
        one(format!("printLine(<{len} bytes>)"), owned.as_ptr(), report);
    }

    // Interior pointers, including the one aimed at the terminating NUL.
    let buf = CString::new("prefix|suffix").unwrap();
    let base = buf.as_ptr();
    for offset in 0..=13isize {
        one(
            format!("printLine(base + {offset})"),
            unsafe { base.offset(offset) },
            report,
        );
    }

    // Many calls inside one capture window: exercises stdio buffering and
    // interleaved NULL arguments, which must contribute nothing.
    let strings: Vec<CString> = (0..200)
        .map(|i| CString::new(format!("repeated line {i} %s %d")).unwrap())
        .collect();
    let run = |f: &dyn Fn(*const c_char)| {
        for (i, s) in strings.iter().enumerate() {
            f(s.as_ptr());
            if i % 7 == 0 {
                f(std::ptr::null());
            }
        }
    };
    let c_out = capture_stdout(|| run(&|p| unsafe { c(p) }));
    let rust_out = capture_stdout(|| run(&|p| unsafe { r(p) }));
    report.check("printLine x200 with interleaved NULLs", &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// Level 1: bad() / good()
// ---------------------------------------------------------------------------

fn bad_and_good_cases(report: &mut Report) {
    let libs = impls();

    for name in ["bad", "good"] {
        let c: libloading::Symbol<FnVoid> = sym(libs.c, name);
        let r: libloading::Symbol<FnVoid> = sym(libs.rust, name);

        // Single call.
        let c_out = capture_stdout(|| unsafe { c() });
        let rust_out = capture_stdout(|| unsafe { r() });
        report.check(&format!("{name}()"), &c_out, &rust_out);

        // Repeated calls: `good` returns a pointer into static storage that must
        // stay stable, and `bad` must not accumulate anything either.
        let c_out = capture_stdout(|| {
            for _ in 0..50 {
                unsafe { c() }
            }
        });
        let rust_out = capture_stdout(|| {
            for _ in 0..50 {
                unsafe { r() }
            }
        });
        report.check(&format!("{name}() x50"), &c_out, &rust_out);
    }

    // Interleaved, so a stale/overwritten buffer in either library would show.
    let cg: libloading::Symbol<FnVoid> = sym(libs.c, "good");
    let cb: libloading::Symbol<FnVoid> = sym(libs.c, "bad");
    let rg: libloading::Symbol<FnVoid> = sym(libs.rust, "good");
    let rb: libloading::Symbol<FnVoid> = sym(libs.rust, "bad");

    let c_out = capture_stdout(|| {
        for i in 0..40 {
            if i % 2 == 0 {
                unsafe { cg() }
            } else {
                unsafe { cb() }
            }
        }
    });
    let rust_out = capture_stdout(|| {
        for i in 0..40 {
            if i % 2 == 0 {
                unsafe { rg() }
            } else {
                unsafe { rb() }
            }
        }
    });
    report.check("good()/bad() interleaved x40", &c_out, &rust_out);

    // bad() immediately after good(): if `bad` ever returned a live pointer, the
    // recycled stack frame contents would surface here.
    let c_out = capture_stdout(|| unsafe {
        cg();
        cb();
        cb();
    });
    let rust_out = capture_stdout(|| unsafe {
        rg();
        rb();
        rb();
    });
    report.check("good(); bad(); bad()", &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// Level 2: driver(int)
// ---------------------------------------------------------------------------

fn driver_cases(report: &mut Report) {
    let libs = impls();
    let c: libloading::Symbol<FnDriver> = sym(libs.c, "driver");
    let r: libloading::Symbol<FnDriver> = sym(libs.rust, "driver");

    let one = |arg: c_int, report: &mut Report| {
        let c_out = capture_stdout(|| unsafe { c(arg) });
        let rust_out = capture_stdout(|| unsafe { r(arg) });
        report.check(&format!("driver({arg})"), &c_out, &rust_out);
    };

    let edges: [i32; 16] = [
        0,
        1,
        -1,
        2,
        -2,
        7,
        0x100,
        0xffff,
        0x1_0000,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        // Truthy values whose low byte / low 16 bits are zero.
        0x7f00_0000,
        -0x8000,
        0x0001_0000,
    ];
    for arg in edges {
        one(arg as c_int, report);
    }
    for arg in -300..=300 {
        one(arg as c_int, report);
    }

    // A long mixed sequence inside a single capture window.
    let args: Vec<c_int> = (0..300)
        .map(|i| match i % 5 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MIN,
            _ => i as i32,
        })
        .collect();
    let c_out = capture_stdout(|| {
        for &a in &args {
            unsafe { c(a) }
        }
    });
    let rust_out = capture_stdout(|| {
        for &a in &args {
            unsafe { r(a) }
        }
    });
    report.check("driver() mixed sequence x300", &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// Cross-level consistency: driver(x) must equal good()/bad() directly
// ---------------------------------------------------------------------------

fn cross_level_consistency(report: &mut Report) {
    let libs = impls();
    for (tag, lib) in [("C", libs.c), ("Rust", libs.rust)] {
        let driver: libloading::Symbol<FnDriver> = sym(lib, "driver");
        let good: libloading::Symbol<FnVoid> = sym(lib, "good");
        let bad: libloading::Symbol<FnVoid> = sym(lib, "bad");

        let via_driver_true = capture_stdout(|| unsafe { driver(1) });
        let via_good = capture_stdout(|| unsafe { good() });
        report.check(
            &format!("{tag}: driver(1) vs good()"),
            &via_driver_true,
            &via_good,
        );

        let via_driver_false = capture_stdout(|| unsafe { driver(0) });
        let via_bad = capture_stdout(|| unsafe { bad() });
        report.check(
            &format!("{tag}: driver(0) vs bad()"),
            &via_driver_false,
            &via_bad,
        );
    }
}
