//! Differential tests: every function is invoked through the exported symbols
//! of the C `.so` and of the Rust `.so` (both loaded with `libloading`), and
//! the bytes each one writes to stdout must be identical.
//!
//! Ordered lowest-level first: printLine / printIntLine, then the callers
//! bad() / good(), then the top-level driver().
//!
//! This test runs with `harness = false` (see Cargo.toml): capturing fd 1 is
//! process-global, so nothing else may write to stdout while a capture is live.

mod common;

use common::{assert_same, run_int, run_str, run_void};
use std::ffi::{CString, c_int};

struct Runner {
    passed: usize,
    failed: Vec<String>,
    logs: Vec<String>,
}

impl Runner {
    fn new() -> Self {
        Runner {
            passed: 0,
            failed: Vec::new(),
            logs: Vec::new(),
        }
    }

    /// Run one case. Any panic (including assertion failures) is caught so the
    /// remaining cases still run, and so stdout is never left redirected.
    fn case<F: FnOnce() + std::panic::UnwindSafe>(&mut self, name: &str, f: F) {
        let prev = std::panic::take_hook();
        let captured: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();
        let sink = captured.clone();
        std::panic::set_hook(Box::new(move |info| {
            sink.lock().unwrap().push(format!("{info}"));
        }));
        let result = std::panic::catch_unwind(f);
        std::panic::set_hook(prev);

        match result {
            Ok(()) => {
                self.passed += 1;
                self.logs.push(format!("test {name} ... ok"));
            }
            Err(_) => {
                self.failed.push(name.to_string());
                self.logs.push(format!("test {name} ... FAILED"));
                for msg in captured.lock().unwrap().iter() {
                    self.logs.push(format!("    {msg}"));
                }
            }
        }
    }

    fn finish(self) -> ! {
        for line in &self.logs {
            println!("{line}");
        }
        println!();
        if self.failed.is_empty() {
            println!("test result: ok. {} passed; 0 failed", self.passed);
            std::process::exit(0);
        }
        println!("failures:");
        for f in &self.failed {
            println!("    {f}");
        }
        println!(
            "test result: FAILED. {} passed; {} failed",
            self.passed,
            self.failed.len()
        );
        std::process::exit(1);
    }
}

fn main() {
    let mut r = Runner::new();

    println!("running differential tests (C .so vs Rust .so via libloading)");
    println!("  C    : {}", common::c_lib_path().display());
    println!("  Rust : {}", common::rust_lib_path().display());
    println!();

    // -----------------------------------------------------------------------
    // Level 0: printLine(const char *)
    // -----------------------------------------------------------------------
    r.case("print_line_null_prints_nothing", || {
        let (c, rr) = run_str("printLine", std::ptr::null());
        assert_same("printLine(NULL)", &c, &rr);
        assert!(c.is_empty(), "C printLine(NULL) unexpectedly printed {c:?}");
    });

    r.case("print_line_strings", || {
        let cases: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b" ".to_vec(),
            b"a".to_vec(),
            b"Calling good()...".to_vec(),
            b"Finished good()".to_vec(),
            b"Calling bad()...".to_vec(),
            b"Finished bad()".to_vec(),
            b"line with\ttab".to_vec(),
            b"embedded\nnewline".to_vec(),
            b"trailing newline\n".to_vec(),
            b"\r\n".to_vec(),
            // Format-specifier-looking text: the format string is fixed, so
            // these must be printed literally by both implementations.
            b"100%".to_vec(),
            b"%s %d %n %%".to_vec(),
            b"%p%p%p%p".to_vec(),
            // Non-ASCII / high bytes.
            "h\u{e9}llo w\u{f6}rld \u{2014} \u{fc}n\u{ef}code".as_bytes().to_vec(),
            vec![0x80, 0xfe, 0xff, 0x41],
            (1u8..=255u8).collect::<Vec<u8>>(),
            // Long strings, including lengths around common buffer sizes.
            vec![b'x'; 1023],
            vec![b'y'; 4096],
            vec![b'z'; 8191],
            vec![b'w'; 65_536],
        ];

        for case in cases {
            let cs = CString::new(case.clone()).expect("no interior NUL");
            let (c, rr) = run_str("printLine", cs.as_ptr());
            assert_same(&format!("printLine(len={})", case.len()), &c, &rr);
        }
    });

    // -----------------------------------------------------------------------
    // Level 0: printIntLine(int)
    // -----------------------------------------------------------------------
    r.case("print_int_line_values", || {
        let mut cases: Vec<c_int> = vec![
            0,
            1,
            -1,
            2,
            -2,
            7,
            9,
            10,
            99,
            100,
            -99,
            -100,
            12345,
            -12345,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            i16::MAX as c_int,
            i16::MIN as c_int,
            u16::MAX as c_int,
            1_000_000_000,
            -1_000_000_000,
        ];
        // Deterministic pseudo-random sweep across the whole int range.
        let mut state: u32 = 0x1234_5678;
        for _ in 0..2000 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            cases.push(state as c_int);
        }

        for v in cases {
            let (c, rr) = run_int("printIntLine", v);
            assert_same(&format!("printIntLine({v})"), &c, &rr);
        }
    });

    // -----------------------------------------------------------------------
    // Level 1: bad() / good()
    // -----------------------------------------------------------------------
    r.case("bad_matches", || {
        let (c, rr) = run_void("bad");
        assert_same("bad()", &c, &rr);
        // The C source discards `intOne + intTwo`, so intSum stays 0 both times.
        assert_eq!(c, b"0\n0\n", "unexpected C bad() output: {c:?}");
    });

    r.case("good_matches", || {
        let (c, rr) = run_void("good");
        assert_same("good()", &c, &rr);
        assert_eq!(c, b"0\n2\n", "unexpected C good() output: {c:?}");
    });

    r.case("bad_and_good_are_repeatable", || {
        for _ in 0..5 {
            let (c, rr) = run_void("bad");
            assert_same("bad() repeat", &c, &rr);
            let (c, rr) = run_void("good");
            assert_same("good() repeat", &c, &rr);
        }
    });

    // -----------------------------------------------------------------------
    // Level 2: driver()
    // -----------------------------------------------------------------------
    r.case("driver_matches", || {
        let (c, rr) = run_void("driver");
        assert_same("driver()", &c, &rr);
        assert_eq!(
            c,
            b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n",
            "unexpected C driver() output: {c:?}"
        );
    });

    r.case("driver_repeatable", || {
        let (first_c, first_r) = run_void("driver");
        assert_same("driver() #1", &first_c, &first_r);
        for i in 2..=5 {
            let (c, rr) = run_void("driver");
            assert_same(&format!("driver() #{i}"), &c, &rr);
            assert_eq!(c, first_c, "C driver() not deterministic");
            assert_eq!(rr, first_r, "Rust driver() not deterministic");
        }
    });

    // -----------------------------------------------------------------------
    // Interleaving: calling the low-level helpers between higher-level ones
    // must not change behaviour in either implementation.
    // -----------------------------------------------------------------------
    r.case("mixed_call_sequence", || {
        for name in ["good", "bad", "driver", "bad", "good", "driver", "driver", "good"] {
            let (c, rr) = run_void(name);
            assert_same(&format!("sequence {name}()"), &c, &rr);
        }
        let s = CString::new("interleaved").unwrap();
        let (c, rr) = run_str("printLine", s.as_ptr());
        assert_same("sequence printLine()", &c, &rr);
        let (c, rr) = run_int("printIntLine", -42);
        assert_same("sequence printIntLine()", &c, &rr);
    });

    r.finish();
}
