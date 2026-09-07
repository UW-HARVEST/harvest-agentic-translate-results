//! Phase B — CONFIGS.md row 67: the `driver` entry point (translation of
//! `c_src/test.c`).  Both `.so`s write to the process' stdout through the C
//! library's `printf`, so stdout is redirected to a temporary file and the
//! captured bytes are compared exactly.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;

#[repr(C)]
#[derive(Clone, Copy)]
struct Record {
    precision: *const c_char,
    lat: f64,
    lon: f64,
    address: *const c_char,
    city: *const c_char,
    state: *const c_char,
    zip: *const c_char,
    country: *const c_char,
}

type DriverFn = unsafe extern "C" fn(
    *const *const c_char,
    *const [c_int; 3],
    *const c_int,
    *const Record,
) -> c_int;

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Call `driver` with stdout redirected into a temporary file; return
/// `(return value, captured bytes)`.
unsafe fn run_driver(
    f: DriverFn,
    strings: &[*const c_char],
    numbers: &[[c_int; 3]],
    ids: &[c_int],
    fields: &[Record],
) -> (c_int, Vec<u8>) {
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "cjson_driver_{}_{:p}.out",
        std::process::id(),
        f as *const ()
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .unwrap();

    fflush(std::ptr::null_mut());
    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

    let rc = f(
        strings.as_ptr(),
        numbers.as_ptr(),
        ids.as_ptr(),
        fields.as_ptr(),
    );

    fflush(std::ptr::null_mut());
    assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
    close(saved);

    let mut file = file;
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    let _ = std::fs::remove_file(&tmp);
    (rc, buf)
}

struct Owned {
    // keeps every NUL-terminated buffer alive for the duration of the call
    _keep: Vec<Vec<u8>>,
    strings: Vec<*const c_char>,
    numbers: Vec<[c_int; 3]>,
    ids: Vec<c_int>,
    fields: Vec<Record>,
}

fn make_inputs(
    week: &[&str; 7],
    numbers: [[c_int; 3]; 3],
    ids: [c_int; 4],
    recs: &[(&str, f64, f64, &str, &str, &str, &str, &str); 2],
) -> Owned {
    let mut keep: Vec<Vec<u8>> = Vec::new();
    let mut push = |s: &str, keep: &mut Vec<Vec<u8>>| -> *const c_char {
        keep.push(cbytes(s.as_bytes()));
        keep.last().unwrap().as_ptr() as *const c_char
    };
    let mut strings = Vec::new();
    for d in week {
        let p = push(d, &mut keep);
        strings.push(p);
    }
    let mut fields = Vec::new();
    for (precision, lat, lon, address, city, state, zip, country) in recs {
        let precision = push(precision, &mut keep);
        let address = push(address, &mut keep);
        let city = push(city, &mut keep);
        let state = push(state, &mut keep);
        let zip = push(zip, &mut keep);
        let country = push(country, &mut keep);
        fields.push(Record {
            precision,
            lat: *lat,
            lon: *lon,
            address,
            city,
            state,
            zip,
            country,
        });
    }
    Owned {
        _keep: keep,
        strings,
        numbers: numbers.to_vec(),
        ids: ids.to_vec(),
        fields,
    }
}

unsafe fn load_driver(path: &std::path::Path) -> (libloading::Library, DriverFn) {
    let lib = libloading::Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e));
    let f = *lib
        .get::<DriverFn>(b"driver\0")
        .unwrap_or_else(|e| panic!("{:?}: missing symbol driver: {}", path, e));
    (lib, f)
}

#[test]
fn row67_driver_stdout_matches() {
    let _g = global_lock();
    unsafe {
        let (_clib, cdrv) = load_driver(&c_driver_path());
        let (_rlib, rdrv) = load_driver(&rust_lib_path());

        // 1. the canonical inputs used by upstream cJSON's test driver
        let canonical = make_inputs(
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
            &[
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

        let mut cases: Vec<Owned> = vec![canonical];

        // 2. randomized inputs
        let mut rng = Rng::new(67);
        for _ in 0..40 {
            let week: Vec<String> = (0..7).map(|_| rng.string(12)).collect();
            let w: [&str; 7] = [
                &week[0], &week[1], &week[2], &week[3], &week[4], &week[5], &week[6],
            ];
            let numbers = [
                [rng.i32(), rng.i32(), rng.i32()],
                [rng.i32(), rng.i32(), rng.i32()],
                [rng.i32(), rng.i32(), rng.i32()],
            ];
            let ids = [rng.i32(), rng.i32(), rng.i32(), rng.i32()];
            let rs: Vec<(String, f64, f64, String, String, String, String, String)> = (0..2)
                .map(|_| {
                    (
                        rng.string(8),
                        rng.f64(),
                        rng.f64(),
                        rng.string(20),
                        rng.string(10),
                        rng.string(4),
                        rng.string(6),
                        rng.string(3),
                    )
                })
                .collect();
            let recs: [(&str, f64, f64, &str, &str, &str, &str, &str); 2] = [
                (
                    &rs[0].0, rs[0].1, rs[0].2, &rs[0].3, &rs[0].4, &rs[0].5, &rs[0].6, &rs[0].7,
                ),
                (
                    &rs[1].0, rs[1].1, rs[1].2, &rs[1].3, &rs[1].4, &rs[1].5, &rs[1].6, &rs[1].7,
                ),
            ];
            cases.push(make_inputs(&w, numbers, ids, &recs));
        }

        // 3. boundary numeric inputs
        cases.push(make_inputs(
            &["", "a", "\"", "\\", "\n", "é", "😀"],
            [
                [c_int::MAX, c_int::MIN, 0],
                [-1, 1, 2],
                [c_int::MIN, c_int::MAX, -0],
            ],
            [c_int::MAX, c_int::MIN, 0, -1],
            &[
                (
                    "p",
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    "addr",
                    "city",
                    "st",
                    "zip",
                    "co",
                ),
                (
                    "q",
                    f64::NAN,
                    0.0,
                    "a\"b",
                    "c\\d",
                    "e\nf",
                    "\u{1}",
                    "😀",
                ),
            ],
        ));

        for (i, case) in cases.iter().enumerate() {
            let (rc_c, out_c) = run_driver(
                cdrv,
                &case.strings,
                &case.numbers,
                &case.ids,
                &case.fields,
            );
            let (rc_r, out_r) = run_driver(
                rdrv,
                &case.strings,
                &case.numbers,
                &case.ids,
                &case.fields,
            );
            assert_eq!(rc_c, rc_r, "driver case #{} return value", i);
            assert_eq!(
                out_c,
                out_r,
                "driver case #{} stdout differs\n--- C ---\n{}\n--- RUST ---\n{}",
                i,
                String::from_utf8_lossy(&out_c),
                String::from_utf8_lossy(&out_r)
            );
            assert!(!out_c.is_empty(), "driver case #{} produced no output", i);
        }
    }
}
