//! The `driver` entry point exported by both `libcJSON_test.so` (C) and the
//! Rust `cdylib`.  It writes to `stdout`, so stdout is redirected to a
//! temporary file around each call and the two byte streams are compared.
mod common;

use common::*;
use std::ffi::CString;
use std::io::{Read, Seek};
use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// Run `f` with fd 1 pointed at a fresh temporary file and return everything
/// that was written.
unsafe fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    unsafe {
        let path = std::env::temp_dir().join(format!(
            "cjson_driver_{}_{:?}.out",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = std::fs::File::options()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        use std::os::fd::AsRawFd;
        let fd = file.as_raw_fd();

        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(fd, 1) >= 0);

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);

        let mut out = Vec::new();
        let mut file = file;
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        file.read_to_end(&mut out).unwrap();
        let _ = std::fs::remove_file(&path);
        out
    }
}

struct Inputs {
    _keep: Vec<CString>,
    strings: Vec<*const c_char>,
    numbers: Vec<[c_int; 3]>,
    ids: Vec<c_int>,
    fields: Vec<Record>,
}

impl Inputs {
    fn new(
        strings: &[&str; 7],
        numbers: [[c_int; 3]; 3],
        ids: [c_int; 4],
        recs: &[(&str, f64, f64, &str, &str, &str, &str, &str); 2],
    ) -> Inputs {
        let mut keep: Vec<CString> = Vec::new();
        let mut sptrs = Vec::new();
        for s in strings {
            let c = CString::new(*s).unwrap();
            sptrs.push(c.as_ptr());
            keep.push(c);
        }
        let mut fields = Vec::new();
        for r in recs {
            let mk = |s: &str, keep: &mut Vec<CString>| {
                let c = CString::new(s).unwrap();
                let p = c.as_ptr();
                keep.push(c);
                p
            };
            fields.push(Record {
                precision: mk(r.0, &mut keep),
                lat: r.1,
                lon: r.2,
                address: mk(r.3, &mut keep),
                city: mk(r.4, &mut keep),
                state: mk(r.5, &mut keep),
                zip: mk(r.6, &mut keep),
                country: mk(r.7, &mut keep),
            });
        }
        Inputs {
            _keep: keep,
            strings: sptrs,
            numbers: numbers.to_vec(),
            ids: ids.to_vec(),
            fields,
        }
    }

    unsafe fn run(&mut self, f: DriverFn) -> (c_int, Vec<u8>) {
        let mut rc = 0;
        let strings = self.strings.as_ptr();
        let numbers = self.numbers.as_mut_ptr();
        let ids = self.ids.as_mut_ptr();
        let fields = self.fields.as_mut_ptr();
        let out = unsafe { capture_stdout(|| rc = unsafe { f(strings, numbers, ids, fields) }) };
        (rc, out)
    }
}

fn datasets() -> Vec<Inputs> {
    vec![
        // the canonical inputs from cJSON's own test program
        Inputs::new(
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
        ),
        // strings that need escaping, extreme numbers
        Inputs::new(
            &[
                "quote\"inside",
                "back\\slash",
                "new\nline",
                "tab\there",
                "\u{0001}control",
                "unicode \u{00e9}\u{1f600}",
                "",
            ],
            [
                [i32::MAX, i32::MIN, 0],
                [-1, 1, 2147483647],
                [-2147483648, 42, -42],
            ],
            [i32::MAX, i32::MIN, 0, -1],
            &[
                (
                    "\"esc\"",
                    f64::MAX,
                    f64::MIN,
                    "addr\\1",
                    "city\nnl",
                    "st",
                    "0",
                    "",
                ),
                (
                    "",
                    1.0 / 3.0,
                    -0.0,
                    "\u{007f}",
                    "\u{00fc}mlaut",
                    "\t",
                    "94085",
                    "US",
                ),
            ],
        ),
        // tiny / denormal / infinite doubles
        Inputs::new(
            &["a", "b", "c", "d", "e", "f", "g"],
            [[0, 0, 0], [0, 0, 0], [0, 0, 0]],
            [0, 0, 0, 0],
            &[
                ("p", f64::MIN_POSITIVE, 5e-324, "a", "b", "c", "d", "e"),
                ("q", 1e308, -1e308, "a", "b", "c", "d", "e"),
            ],
        ),
    ]
}

#[test]
fn driver_output_matches() {
    let _guard = serial();
    // make sure both libraries are loaded and using default hooks
    let a = apis();
    unsafe {
        a.c.cJSON_InitHooks(std::ptr::null_mut());
        a.rust.cJSON_InitHooks(std::ptr::null_mut());
    }
    let c = c_driver();
    let r = rust_driver();

    for (i, mut data) in datasets().into_iter().enumerate() {
        unsafe {
            let (crc, cout) = data.run(c);
            let (rrc, rout) = data.run(r);
            assert_eq!(crc, rrc, "driver return value (dataset {i})");
            assert_eq!(
                cout,
                rout,
                "driver stdout (dataset {i})\nC:\n{}\nRust:\n{}",
                String::from_utf8_lossy(&cout),
                String::from_utf8_lossy(&rout)
            );
            assert!(!cout.is_empty(), "driver produced no output");
        }
    }
}

/// The driver must behave identically when a non-default locale is active.
#[test]
fn driver_output_matches_in_other_locale() {
    let _guard = serial();
    unsafe extern "C" {
        fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    }
    const LC_ALL: c_int = 6;
    let _ = apis();
    let c = c_driver();
    let r = rust_driver();
    unsafe {
        for name in ["de_DE.utf8", "fr_FR.utf8", "C"] {
            let ln = CString::new(name).unwrap();
            if setlocale(LC_ALL, ln.as_ptr()).is_null() {
                continue;
            }
            for (i, mut data) in datasets().into_iter().enumerate() {
                let (crc, cout) = data.run(c);
                let (rrc, rout) = data.run(r);
                assert_eq!(crc, rrc, "driver rc (locale {name}, dataset {i})");
                assert_eq!(
                    cout,
                    rout,
                    "driver stdout (locale {name}, dataset {i})\nC:\n{}\nRust:\n{}",
                    String::from_utf8_lossy(&cout),
                    String::from_utf8_lossy(&rout)
                );
            }
        }
        let c_locale = CString::new("C").unwrap();
        setlocale(LC_ALL, c_locale.as_ptr());
    }
}
