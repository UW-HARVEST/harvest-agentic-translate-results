//! Harness self-check: verifies stdout capture works and that both `.so`s
//! export all five symbols.

mod common;

use common::{api, capture, cstr, render, Impl, BOTH};

#[test]
fn both_libraries_export_all_five_symbols() {
    for imp in BOTH {
        // `api()` panics with a clear message if any symbol is missing.
        let _ = api(imp);
    }
}

#[test]
fn capture_actually_captures() {
    for imp in BOTH {
        let a = api(imp);
        let s = cstr(b"hello");
        let out = capture(|| unsafe { (a.print_line)(s.as_ptr().cast()) });
        assert_eq!(
            out,
            b"hello\n",
            "{} .so produced {}",
            imp.name(),
            render(&out)
        );
    }
}

#[test]
fn known_driver_output_matches_the_c_source_by_inspection() {
    // driver(2.0, 4.0) should print, per c_src/src/driver.c:
    //   Calling good()...   / 50 (goodG2B) / 50 (goodB2G 100/2) / Finished good()
    //   Calling bad()...    / 25 (100/4)   / Finished bad()
    let expected: &[u8] = b"Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n25\nFinished bad()\n";
    for imp in BOTH {
        let a = api(imp);
        let out = capture(|| unsafe { (a.driver)(2.0, 4.0) });
        assert_eq!(
            out,
            expected,
            "{} .so produced {}",
            imp.name(),
            render(&out)
        );
    }
}

#[test]
fn capture_is_empty_for_null_print_line() {
    for imp in [Impl::C, Impl::Rust] {
        let a = api(imp);
        let out = capture(|| unsafe { (a.print_line)(std::ptr::null()) });
        assert!(
            out.is_empty(),
            "{} .so printed {} for NULL",
            imp.name(),
            render(&out)
        );
    }
}
