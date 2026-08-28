//! End-to-end differential test of the `driver()` entry point from
//! `c_src/test.c`.
//!
//! `driver()` writes to stdout through `printf`, so the comparison needs to
//! redirect fd 1 around each call.  fd 1 is process-wide and libtest itself
//! writes progress lines to it, so this file deliberately contains exactly ONE
//! `#[test]` function: while it runs, nothing else in the binary can print.
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Write;

/// Runs the full `driver()` entry point from `test.c` on both libraries and
/// requires byte-identical stdout.
fn run_driver_case(
    strings: &[&str; 7],
    numbers: [[c_int; 3]; 3],
    ids: [c_int; 4],
    fields_raw: [(&str, f64, f64, &str, &str, &str, &str, &str); 2],
) {
    let (c, r) = both();

    let owned: Vec<std::ffi::CString> = strings.iter().map(|s| cs(s)).collect();
    let string_ptrs: Vec<*const c_char> = owned.iter().map(|s| s.as_ptr()).collect();

    let mut keep: Vec<std::ffi::CString> = Vec::new();
    let mut records: Vec<Record> = Vec::new();
    for f in fields_raw.iter() {
        let p = cs(f.0);
        let a = cs(f.3);
        let ci = cs(f.4);
        let st = cs(f.5);
        let z = cs(f.6);
        let co = cs(f.7);
        records.push(Record {
            precision: p.as_ptr(),
            lat: f.1,
            lon: f.2,
            address: a.as_ptr(),
            city: ci.as_ptr(),
            state: st.as_ptr(),
            zip: z.as_ptr(),
            country: co.as_ptr(),
        });
        keep.extend([p, a, ci, st, z, co]);
    }

    unsafe {
        let out_c = capture_stdout(|| {
            (c.driver)(
                string_ptrs.as_ptr(),
                numbers.as_ptr(),
                ids.as_ptr(),
                records.as_ptr(),
            );
        });
        let out_r = capture_stdout(|| {
            (r.driver)(
                string_ptrs.as_ptr(),
                numbers.as_ptr(),
                ids.as_ptr(),
                records.as_ptr(),
            );
        });
        assert!(!out_c.is_empty(), "C driver produced no output");
        if out_c != out_r {
            panic!(
                "driver() stdout differs\n--- C ---\n{}\n--- Rust ---\n{}",
                String::from_utf8_lossy(&out_c),
                String::from_utf8_lossy(&out_r)
            );
        }
    }
    drop(keep);
    drop(owned);
}

fn driver_upstream_example() {
    run_driver_case(
        &[
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ],
        [[0, -1, 0], [1, 0, 0], [0, 0, 1]],
        [116, 943, 234, 38793],
        [
            (
                "zip",
                37.7668,
                -122.3959,
                "",
                "SAN FRANCISCO",
                "CA",
                "94107",
                "US",
            ),
            (
                "zip",
                37.371991,
                -122.026020,
                "",
                "SUNNYVALE",
                "CA",
                "94085",
                "US",
            ),
        ],
    );
}

fn driver_randomized_inputs() {
    let mut rng = Rng::new(0xD1CE_F00D);
    for iter in 0..40 {
        // Strings with escapes, control characters, UTF-8 and empties.
        let pool = [
            "",
            "a",
            "tab\there",
            "nl\nhere",
            "quote\"here",
            "back\\slash",
            "\u{1}\u{2}\u{1f}",
            "ünïcøde \u{1F600}",
            "0123456789012345678901234567890123456789",
            "/slash/",
            "\u{8}\u{c}\r",
        ];
        let pick = |rng: &mut Rng| pool[rng.below(pool.len())];
        let s: [&str; 7] = [
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
        ];
        let mut numbers = [[0i32; 3]; 3];
        for row in numbers.iter_mut() {
            for v in row.iter_mut() {
                *v = rng.range_i32(i32::MIN, i32::MAX);
            }
        }
        let ids = [
            rng.range_i32(i32::MIN, i32::MAX),
            rng.range_i32(-10, 10),
            0,
            i32::MIN,
        ];
        let f0 = (
            pick(&mut rng),
            rng.json_f64(),
            rng.json_f64(),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
        );
        let f1 = (
            pick(&mut rng),
            rng.json_f64(),
            rng.json_f64(),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
        );
        eprintln!("driver iteration {iter}");
        run_driver_case(&s, numbers, ids, [f0, f1]);
    }
}

/// The single entry point for this binary (see the module comment).
#[test]
fn driver_differential() {
    // Make sure libtest's pending "test driver_differential ... " prefix and
    // anything else buffered in Rust's stdout is out of the way before fd 1
    // gets redirected.
    let _ = std::io::stdout().flush();
    driver_upstream_example();
    driver_randomized_inputs();
}
