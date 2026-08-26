//! Phase B (rows B01..B06) and Phase C (rows 14, 15, 17) — differential tests
//! that call the exported C symbols **in this process** through `dlopen` /
//! `dlsym` and capture whatever the library writes to file descriptor 1.
//!
//! This target runs with `harness = false` (see `Cargo.toml`) because capturing
//! fd 1 is a process-wide operation: libtest's own parallel progress output
//! would otherwise be interleaved into the captured bytes.  A custom harness
//! guarantees strictly sequential execution and keeps all progress reporting on
//! stderr.

mod common;

use common::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

// ---------------------------------------------------------------------------
// tiny sequential harness
// ---------------------------------------------------------------------------

struct Harness {
    passed: usize,
    failed: Vec<String>,
    filter: Option<String>,
}

impl Harness {
    fn new() -> Self {
        let filter = std::env::args()
            .skip(1)
            .find(|a| !a.starts_with("--"))
            .clone();
        Harness {
            passed: 0,
            failed: Vec::new(),
            filter,
        }
    }

    fn run(&mut self, name: &str, f: impl FnOnce()) {
        if let Some(filter) = &self.filter {
            if !name.contains(filter.as_str()) {
                return;
            }
        }
        eprint!("test {name} ... ");
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(()) => {
                self.passed += 1;
                eprintln!("ok");
            }
            Err(_) => {
                self.failed.push(name.to_string());
                eprintln!("FAILED");
            }
        }
    }

    fn finish(self) {
        eprintln!(
            "\nin-process differential result: {} passed; {} failed",
            self.passed,
            self.failed.len()
        );
        if !self.failed.is_empty() {
            eprintln!("failures: {:?}", self.failed);
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Calls `printHexCharLine(value)` on one library and returns the bytes it
/// wrote to fd 1.
fn print_once(lib: &libloading::Library, value: i32, via_pipe: bool) -> Vec<u8> {
    let f = print_hex_char_line(lib);
    let run = || unsafe { f(value) };
    if via_pipe {
        capture_fd1_pipe(run).0
    } else {
        capture_fd1_file(run).0
    }
}

/// Calls `printHexCharLine` once per value inside a single capture window.
fn print_many(lib: &libloading::Library, values: &[i32], via_pipe: bool) -> Vec<u8> {
    let f = print_hex_char_line(lib);
    let run = || unsafe {
        for &v in values {
            f(v);
        }
    };
    if via_pipe {
        capture_fd1_pipe(run).0
    } else {
        capture_fd1_file(run).0
    }
}

#[derive(Copy, Clone)]
enum CLib {
    O0,
    O2,
}

fn c_lib(which: CLib) -> &'static libloading::Library {
    match which {
        CLib::O0 => &libs().c,
        CLib::O2 => &libs().c_o2,
    }
}

/// One capture window per value, compared byte for byte.
fn compare_values(values: &[i32], which: CLib, via_pipe: bool, ctx: &str) {
    let c = c_lib(which);
    let r = &libs().rust;
    for &v in values {
        let out_c = print_once(c, v, via_pipe);
        let out_r = print_once(r, v, via_pipe);
        assert_eq!(
            out_c,
            out_r,
            "{ctx}: printHexCharLine({v} = {v:#010x}) diverged\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r),
        );
        assert!(
            out_c.ends_with(b"\n") && out_c.len() >= 3,
            "{ctx}: unexpected C output {:?} for {v}",
            String::from_utf8_lossy(&out_c)
        );
    }
}

fn all_chars_as_int() -> Vec<i32> {
    // A well-formed C caller sign-extends the `char` into the argument register,
    // i.e. passes `(char)b` promoted to `int`.
    (0u16..=255).map(|b| (b as u8) as i8 as i32).collect()
}

fn wide_int_cases() -> Vec<i32> {
    let mut values: Vec<i32> = vec![
        256,
        0x1ff,
        -1000,
        1000,
        i32::MIN,
        i32::MAX,
        0x7fff_ff80u32 as i32,
        0xffff_ff00u32 as i32,
        0x0000_0100,
        0x0000_01ff,
        -129,
        128,
        65535,
        -65536,
        0x1234_5600,
        0x1234_567f,
        0x1234_5680,
        0x1234_56ff,
    ];
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    values.extend((0..2048).map(|_| rng.next_i32()));
    values
}

// ---------------------------------------------------------------------------

fn main() {
    let mut h = Harness::new();

    // ---- B01: exhaustive over all 256 char bit patterns -----------------
    h.run("B01_print_hex_char_line_exhaustive_chars", || {
        compare_values(&all_chars_as_int(), CLib::O0, false, "B01");
    });

    // ---- B02: randomized char values ------------------------------------
    h.run("B02_print_hex_char_line_randomized_chars", || {
        let mut rng = Rng::new(Rng::DEFAULT_SEED);
        let values: Vec<i32> = (0..4096).map(|_| rng.next_u8() as i8 as i32).collect();
        for chunk in values.chunks(256) {
            let out_c = print_many(c_lib(CLib::O0), chunk, false);
            let out_r = print_many(&libs().rust, chunk, false);
            if out_c != out_r {
                // Narrow the divergence down to a single value.
                compare_values(chunk, CLib::O0, false, "B02");
                panic!("B02: batched output differs while per-value output matches");
            }
        }
    });

    // ---- B03: boundary char values + ground-truth spot check -------------
    h.run("B03_print_hex_char_line_boundaries", || {
        let values: Vec<i32> = [0x00u8, 0x01, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff]
            .iter()
            .map(|b| *b as i8 as i32)
            .collect();
        compare_values(&values, CLib::O0, false, "B03");

        let expected: [(&str, i32); 6] = [
            ("00\n", 0x00),
            ("01\n", 0x01),
            ("7e\n", 0x7e),
            ("7f\n", 0x7f),
            ("ffffff80\n", -128),
            ("ffffffff\n", -1),
        ];
        for (want, v) in expected {
            let got_c = print_once(c_lib(CLib::O0), v, false);
            let got_r = print_once(&libs().rust, v, false);
            assert_eq!(
                String::from_utf8_lossy(&got_c),
                want,
                "C ground truth changed for {v}"
            );
            assert_eq!(
                String::from_utf8_lossy(&got_r),
                want,
                "Rust output changed for {v}"
            );
        }
    });

    // ---- B04 / ERRORS row 14: out-of-char-range int arguments ------------
    h.run("B04_err14_print_hex_char_line_out_of_range_ints", || {
        let values = wide_int_cases();
        compare_values(&values, CLib::O0, false, "B04");
        // Ground truth: only the low byte matters (gcc: `movsbl %dil,%esi`).
        for (v, want) in [
            (0x1ff, "ffffffff\n"),
            (256, "00\n"),
            (0x1234_5680u32 as i32, "ffffff80\n"),
            (i32::MIN, "00\n"),
            (i32::MAX, "ffffffff\n"),
        ] {
            let got = print_once(c_lib(CLib::O0), v, false);
            assert_eq!(
                String::from_utf8_lossy(&got),
                want,
                "C ground truth for wide argument {v:#010x}"
            );
        }
    });

    // ---- B05: many consecutive calls in one process ----------------------
    h.run("B05_print_hex_char_line_1000_calls_one_process", || {
        let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
        let values: Vec<i32> = (0..1000).map(|_| rng.next_u8() as i8 as i32).collect();
        let out_c = print_many(c_lib(CLib::O0), &values, false);
        let out_r = print_many(&libs().rust, &values, false);
        assert_eq!(
            out_c.iter().filter(|b| **b == b'\n').count(),
            1000,
            "expected exactly one line per call"
        );
        assert_eq!(
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r),
            "B05: accumulated output differs"
        );
    });

    // ---- B06: fd 1 is a pipe --------------------------------------------
    h.run("B06_print_hex_char_line_stdout_is_pipe", || {
        compare_values(&all_chars_as_int(), CLib::O0, true, "B06");
    });

    // ---- C2: the -O2 build of the C library ------------------------------
    h.run("C2_optimised_c_library_print_hex_char_line", || {
        compare_values(&all_chars_as_int(), CLib::O2, false, "C2");
        compare_values(&wide_int_cases(), CLib::O2, false, "C2-wide");
    });

    // ---- ERRORS row 15: negative chars print 8 hex digits ----------------
    h.run("err15_negative_char_sign_extension", || {
        let negatives: Vec<i32> = (0x80u16..=0xff).map(|b| (b as u8) as i8 as i32).collect();
        compare_values(&negatives, CLib::O0, false, "err15");
        for v in -128..0i32 {
            let out = print_once(c_lib(CLib::O0), v, false);
            let s = String::from_utf8_lossy(&out).to_string();
            assert_eq!(s.len(), 9, "expected 8 hex digits + newline, got {s:?}");
            assert_eq!(
                s.trim_end(),
                format!("{:x}", v as u32),
                "C sign-extension changed"
            );
            let out_r = print_once(&libs().rust, v, false);
            assert_eq!(out, out_r, "err15: divergence for {v}");
        }
    });

    // ---- ERRORS row 17: no state accumulates across calls ----------------
    h.run("err17_repeated_calls_no_state", || {
        let values: Vec<i32> = std::iter::repeat(-1).take(1000).collect();
        let out_c = print_many(c_lib(CLib::O0), &values, false);
        let out_r = print_many(&libs().rust, &values, false);
        assert_eq!(out_c, out_r, "err17: divergence on repeated calls");
        assert_eq!(
            out_c,
            "ffffffff\n".repeat(1000).into_bytes(),
            "err17: unexpected C output"
        );
    });

    h.finish();
}
