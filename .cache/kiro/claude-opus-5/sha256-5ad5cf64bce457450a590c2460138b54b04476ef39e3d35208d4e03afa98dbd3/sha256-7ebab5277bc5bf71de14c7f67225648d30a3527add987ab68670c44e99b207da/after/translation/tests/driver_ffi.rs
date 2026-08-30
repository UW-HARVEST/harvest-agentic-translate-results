//! Differential tests for `driver.c` vs. its Rust translation.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! exercised only via their exported C symbols, so the `#[no_mangle]` wrappers
//! are part of what is under test. Cases run from the lowest-level function
//! (`printLine`) upward to the top-level entry point (`driver`).
//!
//! This target sets `harness = false`: capturing output means redirecting the
//! process's stdout, and libtest's own progress writes would otherwise be
//! captured too. The runner at the bottom of this file executes every case
//! sequentially and reports results on stderr.

mod common;

use std::ffi::CString;

use common::assert_same;
use common::capture_stdout;
use common::str_fn;
use common::void_fn;
use common::Pair;

/// Byte strings fed to `printLine`. Covers empty input, format-specifier-like
/// content, embedded newlines and tabs, non-UTF-8 bytes, and long buffers that
/// cross typical stdio buffer boundaries.
fn print_line_cases() -> Vec<Vec<u8>> {
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"a".to_vec(),
        b"hello".to_vec(),
        b"printLine()".to_vec(),
        // Every literal the C source itself passes to printLine.
        b"good()".to_vec(),
        b"bad()".to_vec(),
        b"helperGood()".to_vec(),
        b"helperBad()".to_vec(),
        b"Calling good()...".to_vec(),
        b"Finished good()".to_vec(),
        b"Calling bad()...".to_vec(),
        b"Finished bad()".to_vec(),
        // Format specifiers must be emitted literally: the C code passes the
        // argument to `printf` as a "%s" operand, never as the format itself.
        b"%s".to_vec(),
        b"%d %i %u %x".to_vec(),
        b"%n".to_vec(),
        b"100%".to_vec(),
        b"%%".to_vec(),
        // Whitespace and control characters.
        b"line\nwith\nnewlines".to_vec(),
        b"tab\tseparated".to_vec(),
        b"carriage\rreturn".to_vec(),
        b"trailing newline\n".to_vec(),
        b"  leading and trailing  ".to_vec(),
        // Punctuation and quoting.
        b"\"quoted\"".to_vec(),
        b"back\\slash".to_vec(),
        b"~!@#$^&*()_+-=[]{}|;:',.<>/?".to_vec(),
        // Non-ASCII and non-UTF-8 payloads.
        "unicode: \u{00e9}\u{4e2d}\u{6587}\u{1f600}"
            .as_bytes()
            .to_vec(),
        vec![0x80, 0xfe, 0xff, 0x01, 0x7f],
        (1u8..=0x7f).collect::<Vec<u8>>(),
        (0x80u8..=0xff).collect::<Vec<u8>>(),
    ];
    // Lengths around common buffer sizes.
    for len in [1usize, 63, 64, 127, 128, 255, 1023, 1024, 4095, 4096, 8193] {
        cases.push(vec![b'x'; len]);
    }
    cases
}

fn print_line_matches_for_many_inputs() {
    let pair = Pair::load();
    let c_print_line = str_fn(&pair.c, "printLine");
    let rust_print_line = str_fn(&pair.rust, "printLine");

    for case in print_line_cases() {
        let arg = CString::new(case.clone()).expect("no interior NUL in test case");
        let c_out = capture_stdout(|| unsafe { c_print_line(arg.as_ptr()) });
        let rust_out = capture_stdout(|| unsafe { rust_print_line(arg.as_ptr()) });

        assert_same(
            &format!("printLine({:?})", String::from_utf8_lossy(&case)),
            &c_out,
            &rust_out,
        );

        // Also pin the absolute contract: the argument followed by one newline.
        let mut expected = case.clone();
        expected.push(b'\n');
        assert_eq!(
            c_out,
            expected,
            "C printLine did not emit `<arg>\\n` for {:?}",
            String::from_utf8_lossy(&case)
        );
    }
}

fn print_line_with_null_prints_nothing() {
    let pair = Pair::load();
    let c_print_line = str_fn(&pair.c, "printLine");
    let rust_print_line = str_fn(&pair.rust, "printLine");

    let c_out = capture_stdout(|| unsafe { c_print_line(std::ptr::null()) });
    let rust_out = capture_stdout(|| unsafe { rust_print_line(std::ptr::null()) });

    assert_same("printLine(NULL)", &c_out, &rust_out);
    assert!(
        c_out.is_empty(),
        "C printLine(NULL) wrote {}",
        common::render(&c_out)
    );
}

fn print_line_repeated_calls_accumulate_identically() {
    let pair = Pair::load();
    let c_print_line = str_fn(&pair.c, "printLine");
    let rust_print_line = str_fn(&pair.rust, "printLine");

    let words: Vec<CString> = ["first", "", "third", "%s", "last"]
        .iter()
        .map(|w| CString::new(*w).unwrap())
        .collect();

    // Interleave NULL arguments to confirm they are skipped in both versions.
    let c_out = capture_stdout(|| unsafe {
        for w in &words {
            c_print_line(w.as_ptr());
            c_print_line(std::ptr::null());
        }
    });
    let rust_out = capture_stdout(|| unsafe {
        for w in &words {
            rust_print_line(w.as_ptr());
            rust_print_line(std::ptr::null());
        }
    });

    assert_same("repeated printLine calls", &c_out, &rust_out);
}

fn bad_matches() {
    let pair = Pair::load();
    let c_bad = void_fn(&pair.c, "bad");
    let rust_bad = void_fn(&pair.rust, "bad");

    let c_out = capture_stdout(|| unsafe { c_bad() });
    let rust_out = capture_stdout(|| unsafe { rust_bad() });

    assert_same("bad()", &c_out, &rust_out);
    // `bad()` in the C source deliberately does not call `helperBad()`.
    assert_eq!(c_out, b"bad()\n");
}

fn good_matches() {
    let pair = Pair::load();
    let c_good = void_fn(&pair.c, "good");
    let rust_good = void_fn(&pair.rust, "good");

    let c_out = capture_stdout(|| unsafe { c_good() });
    let rust_out = capture_stdout(|| unsafe { rust_good() });

    assert_same("good()", &c_out, &rust_out);
    assert_eq!(c_out, b"good()\nhelperGood()\n");
}

fn driver_matches() {
    let pair = Pair::load();
    let c_driver = void_fn(&pair.c, "driver");
    let rust_driver = void_fn(&pair.rust, "driver");

    let c_out = capture_stdout(|| unsafe { c_driver() });
    let rust_out = capture_stdout(|| unsafe { rust_driver() });

    assert_same("driver()", &c_out, &rust_out);
    assert_eq!(
        c_out,
        b"Calling good()...\ngood()\nhelperGood()\nFinished good()\nCalling bad()...\nbad()\nFinished bad()\n"
    );
}

fn repeated_driver_calls_match() {
    let pair = Pair::load();
    let c_driver = void_fn(&pair.c, "driver");
    let rust_driver = void_fn(&pair.rust, "driver");

    let c_out = capture_stdout(|| unsafe {
        for _ in 0..5 {
            c_driver();
        }
    });
    let rust_out = capture_stdout(|| unsafe {
        for _ in 0..5 {
            rust_driver();
        }
    });

    assert_same("driver() x5", &c_out, &rust_out);
}

/// Calls the whole public surface in a single capture so any difference in
/// ordering or stdio buffering between the two libraries would show up.
fn full_api_sequence_matches() {
    let pair = Pair::load();

    let run = |lib: &libloading::Library| {
        let print_line = str_fn(lib, "printLine");
        let bad = void_fn(lib, "bad");
        let good = void_fn(lib, "good");
        let driver = void_fn(lib, "driver");
        let marker = CString::new("-- marker --").unwrap();

        capture_stdout(|| unsafe {
            print_line(marker.as_ptr());
            good();
            print_line(std::ptr::null());
            bad();
            driver();
            print_line(marker.as_ptr());
            bad();
            good();
            driver();
        })
    };

    let c_out = run(&pair.c);
    let rust_out = run(&pair.rust);
    assert_same("full API sequence", &c_out, &rust_out);
}

/// Loading and dropping each library repeatedly must not change behaviour,
/// which guards against state left behind by either shared object.
fn reload_cycles_match() {
    for _ in 0..3 {
        let pair = Pair::load();
        let c_driver = void_fn(&pair.c, "driver");
        let rust_driver = void_fn(&pair.rust, "driver");
        let c_out = capture_stdout(|| unsafe { c_driver() });
        let rust_out = capture_stdout(|| unsafe { rust_driver() });
        assert_same("driver() after reload", &c_out, &rust_out);
    }
}

// --- Sequential runner -------------------------------------------------------

type Case = (&'static str, fn());

const CASES: &[Case] = &[
    // Lowest level first, then callers, then the top-level entry point.
    ("print_line_with_null_prints_nothing", print_line_with_null_prints_nothing),
    ("print_line_matches_for_many_inputs", print_line_matches_for_many_inputs),
    (
        "print_line_repeated_calls_accumulate_identically",
        print_line_repeated_calls_accumulate_identically,
    ),
    ("bad_matches", bad_matches),
    ("good_matches", good_matches),
    ("driver_matches", driver_matches),
    ("repeated_driver_calls_match", repeated_driver_calls_match),
    ("full_api_sequence_matches", full_api_sequence_matches),
    ("reload_cycles_match", reload_cycles_match),
];

fn main() {
    // Accept and honour a name filter so `cargo test <substring>` still works.
    let filter: Option<String> = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .filter(|a| a != "--exact");

    // Progress goes to stderr; stdout is reserved for the captures.
    eprintln!("running {} differential cases", CASES.len());
    let mut failures: Vec<&str> = Vec::new();
    let mut ran = 0usize;

    for (name, case) in CASES {
        if let Some(f) = &filter {
            if !name.contains(f.as_str()) {
                continue;
            }
        }
        ran += 1;
        eprint!("case {name} ... ");
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(case)) {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failures.push(name);
            }
        }
    }

    eprintln!(
        "\nresult: {}. {} passed; {} failed",
        if failures.is_empty() { "ok" } else { "FAILED" },
        ran - failures.len(),
        failures.len()
    );
    if !failures.is_empty() {
        eprintln!("failing cases: {failures:?}");
        std::process::exit(1);
    }
}
