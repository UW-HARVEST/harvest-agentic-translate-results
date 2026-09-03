//! CONFIGS.md row 90 — the `driver` entry point from `c_src/test.c`.
//!
//! `driver` is the only symbol of the C `cJSON_test` shared object; its entire
//! observable behaviour is what it writes to stdout, so the test captures fd 1
//! around each call and compares the bytes.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

struct Inputs {
    strings: Vec<Vec<c_char>>,
    numbers: [[c_int; 3]; 3],
    ids: [c_int; 4],
    field_strings: Vec<Vec<c_char>>,
    lats: [f64; 2],
    lons: [f64; 2],
}

fn make_inputs(rng: &mut Rng, variant: usize) -> Inputs {
    let words: &[&str] = &[
        "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
        "", "with \"quotes\"", "tab\there", "newline\nhere", "\u{1}ctrl", "caf\u{e9}",
        "a very long day name that forces the print buffer to grow past 256 bytes",
    ];
    let strings: Vec<Vec<c_char>> = (0..7)
        .map(|i| {
            let w = if variant == 0 {
                words[i]
            } else {
                words[rng.below(words.len())]
            };
            cbytes(w.as_bytes())
        })
        .collect();

    let mut numbers = [[0i32; 3]; 3];
    for row in numbers.iter_mut() {
        for v in row.iter_mut() {
            *v = match rng.below(6) {
                0 => 0,
                1 => i32::MAX,
                2 => i32::MIN,
                3 => -1,
                4 => 1_000_000,
                _ => rng.i32(),
            };
        }
    }
    if variant == 0 {
        numbers = [[0, -1, 2], [3, -4, 5], [6, -7, 8]];
    }

    let mut ids = [0i32; 4];
    for v in ids.iter_mut() {
        *v = rng.i32();
    }
    if variant == 0 {
        ids = [116, 943, 234, 38793];
    }

    // 7 strings per record x 2 records
    let field_strings: Vec<Vec<c_char>> = (0..14)
        .map(|i| {
            let w = if variant == 0 {
                ["zip", "SanFrancisco", "CA", "94107", "US", "100 Main Street", "zip"][i % 7]
            } else {
                words[rng.below(words.len())]
            };
            cbytes(w.as_bytes())
        })
        .collect();

    let lats = if variant == 0 {
        [37.7668, 37.371991]
    } else {
        [
            match rng.below(5) {
                0 => 0.0,
                1 => -0.0,
                2 => f64::INFINITY,
                3 => f64::NAN,
                _ => rng.f64(),
            },
            rng.f64(),
        ]
    };
    let lons = if variant == 0 {
        [-122.3959, -122.026020]
    } else {
        [rng.f64(), (rng.i32() as f64) / 1024.0]
    };

    Inputs { strings, numbers, ids, field_strings, lats, lons }
}

unsafe fn call_driver(f: DriverFn, inp: &Inputs) -> Vec<u8> {
    let sptrs: Vec<*const c_char> = inp.strings.iter().map(|v| v.as_ptr()).collect();
    let records: Vec<Record> = (0..2)
        .map(|i| Record {
            precision: inp.field_strings[i * 7].as_ptr(),
            lat: inp.lats[i],
            lon: inp.lons[i],
            address: inp.field_strings[i * 7 + 5].as_ptr(),
            city: inp.field_strings[i * 7 + 1].as_ptr(),
            state: inp.field_strings[i * 7 + 2].as_ptr(),
            zip: inp.field_strings[i * 7 + 3].as_ptr(),
            country: inp.field_strings[i * 7 + 4].as_ptr(),
        })
        .collect();

    let mut rc: c_int = -12345;
    let out = capture_stdout(|| {
        rc = f(
            sptrs.as_ptr(),
            inp.numbers.as_ptr() as *const [c_int; 3],
            inp.ids.as_ptr(),
            records.as_ptr(),
        );
    });
    let mut full = out;
    full.extend_from_slice(format!("\n<<rc={rc}>>").as_bytes());
    full
}

#[test]
fn row_90_driver_stdout_matches() {
    let _g = lock();
    // Make sure neither library is left with non-default hooks.
    let p = pair();
    unsafe {
        (p.c.cJSON_InitHooks)(std::ptr::null_mut());
        (p.r.cJSON_InitHooks)(std::ptr::null_mut());
    }

    let cd = c_driver();
    let rd = rust_driver();
    let mut rng = Rng::new(0x5EED_0090);

    unsafe {
        for variant in 0..40 {
            let inp = make_inputs(&mut rng, variant);
            let cout = call_driver(cd, &inp);
            let rout = call_driver(rd, &inp);
            assert!(
                cout == rout,
                "driver stdout differs for variant {variant}\n--- C ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}",
                cout.len(),
                String::from_utf8_lossy(&cout),
                rout.len(),
                String::from_utf8_lossy(&rout)
            );
            assert!(!cout.is_empty(), "driver produced no output for variant {variant}");
        }
    }
}

#[test]
fn row_90_driver_output_is_the_expected_shape() {
    let _g = lock();
    let cd = c_driver();
    let rd = rust_driver();
    let mut rng = Rng::new(1);
    let inp = make_inputs(&mut rng, 0);
    unsafe {
        let cout = call_driver(cd, &inp);
        let rout = call_driver(rd, &inp);
        assert!(cout == rout);
        let s = String::from_utf8_lossy(&cout);
        assert!(s.starts_with("Version: 1.7.19\n"), "unexpected driver output:\n{s}");
        // six printed documents plus the version line
        assert_eq!(s.matches("\"name\"").count(), 1, "output:\n{s}");
        assert!(s.contains("\"number\":\tnull"), "1.0/0.0 must print as null:\n{s}");
        assert!(s.contains("<<rc=0>>"), "driver must return 0");
    }
}
