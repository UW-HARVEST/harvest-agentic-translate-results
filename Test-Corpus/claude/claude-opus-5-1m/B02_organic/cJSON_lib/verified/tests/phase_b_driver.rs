//! Phase B — row C90: the `driver` entry point translated from `c_src/test.c`.
//!
//! `driver` writes to stdout, so the differential comparison captures file
//! descriptor 1 around each call and compares the produced bytes.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void, CString};
use std::fmt::Write as _;
use std::os::fd::AsRawFd;
use std::ptr::null_mut;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// `struct record` from `c_src/test.c`.
#[repr(C)]
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

/// Run `f` with fd 1 redirected into a temporary file and return what was
/// written.
unsafe fn capture<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!("cjson_driver_{tag}.out"));
    fflush(null_mut());
    let saved = dup(1);
    assert!(saved >= 0, "dup failed");
    {
        let file = std::fs::File::create(&path).expect("create capture file");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
    }
    f();
    fflush(null_mut());
    assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
    close(saved);
    std::fs::read(&path).expect("read capture file")
}

struct Args {
    strings: Vec<CString>,
    numbers: Vec<[c_int; 3]>,
    ids: Vec<c_int>,
    texts: Vec<CString>,
    lat: [f64; 2],
    lon: [f64; 2],
}

impl Args {
    fn canonical() -> Args {
        Args {
            strings: [
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
            ]
            .iter()
            .map(|s| cs(s))
            .collect(),
            numbers: vec![[0, -1, 0], [1, 0, 0], [0, 0, 1]],
            ids: vec![116, 943, 234, 38793],
            texts: [
                "zip",
                "SD",
                "SAN FRANCISCO",
                "CA",
                "94107",
                "US",
                "zip",
                "SD",
                "SUNNYVALE",
                "CA",
                "94085",
                "US",
            ]
            .iter()
            .map(|s| cs(s))
            .collect(),
            lat: [37.7668, 37.371991],
            lon: [-122.3959, -122.026020],
        }
    }

    fn random(rng: &mut Rng) -> Args {
        let mut strings = Vec::new();
        for _ in 0..7 {
            strings.push(CString::new(rng.ascii(12)).unwrap());
        }
        let mut numbers = Vec::new();
        for _ in 0..3 {
            numbers.push([
                rng.range_i32(i32::MIN, i32::MAX),
                rng.range_i32(-100, 100),
                rng.range_i32(i32::MIN, i32::MAX),
            ]);
        }
        let ids = (0..4)
            .map(|_| rng.range_i32(i32::MIN, i32::MAX))
            .collect();
        let mut texts = Vec::new();
        for _ in 0..12 {
            texts.push(CString::new(rng.ascii(10)).unwrap());
        }
        Args {
            strings,
            numbers,
            ids,
            texts,
            lat: [rng.nice_f64(), rng.nice_f64()],
            lon: [rng.nice_f64(), rng.nice_f64()],
        }
    }

    unsafe fn call(&self, driver: DriverFn) -> c_int {
        let sptrs: Vec<*const c_char> = self.strings.iter().map(|s| s.as_ptr()).collect();
        let records = [
            Record {
                precision: self.texts[0].as_ptr(),
                lat: self.lat[0],
                lon: self.lon[0],
                address: self.texts[1].as_ptr(),
                city: self.texts[2].as_ptr(),
                state: self.texts[3].as_ptr(),
                zip: self.texts[4].as_ptr(),
                country: self.texts[5].as_ptr(),
            },
            Record {
                precision: self.texts[6].as_ptr(),
                lat: self.lat[1],
                lon: self.lon[1],
                address: self.texts[7].as_ptr(),
                city: self.texts[8].as_ptr(),
                state: self.texts[9].as_ptr(),
                zip: self.texts[10].as_ptr(),
                country: self.texts[11].as_ptr(),
            },
        ];
        driver(
            sptrs.as_ptr(),
            self.numbers.as_ptr(),
            self.ids.as_ptr(),
            records.as_ptr(),
        )
    }
}

/// `ERRORS.md` row 183 — `driver` with a `NULL` entry in `strings[]`:
/// `cJSON_CreateStringArray` returns `NULL`, `cJSON_Print(NULL)` returns `NULL`
/// and `strlen(NULL)` dereferences it. Both implementations must die the same
/// way, so each is run in its own child process and the fatal signals compared.
#[test]
fn row183_null_string_argument_kills_both_identically() {
    use std::os::unix::process::ExitStatusExt;

    // child mode: crash on purpose
    if let Ok(which) = std::env::var("CJSON_CRASH_CHILD") {
        let path = if which == "c" {
            c_driver_so_path()
        } else {
            rust_driver_so_path()
        };
        let lib = unsafe { libloading::Library::new(path) }.expect("dlopen");
        let driver: libloading::Symbol<DriverFn> =
            unsafe { lib.get(b"driver\0") }.expect("driver");
        let args = Args::canonical();
        let mut sptrs: Vec<*const c_char> = args.strings.iter().map(|s| s.as_ptr()).collect();
        sptrs[3] = null_mut(); // the NULL that breaks cJSON_CreateStringArray
        let records = [
            Record {
                precision: args.texts[0].as_ptr(),
                lat: args.lat[0],
                lon: args.lon[0],
                address: args.texts[1].as_ptr(),
                city: args.texts[2].as_ptr(),
                state: args.texts[3].as_ptr(),
                zip: args.texts[4].as_ptr(),
                country: args.texts[5].as_ptr(),
            },
            Record {
                precision: args.texts[6].as_ptr(),
                lat: args.lat[1],
                lon: args.lon[1],
                address: args.texts[7].as_ptr(),
                city: args.texts[8].as_ptr(),
                state: args.texts[9].as_ptr(),
                zip: args.texts[10].as_ptr(),
                country: args.texts[11].as_ptr(),
            },
        ];
        unsafe {
            (*driver)(
                sptrs.as_ptr(),
                args.numbers.as_ptr(),
                args.ids.as_ptr(),
                records.as_ptr(),
            );
        }
        // must not get here
        std::process::exit(42);
    }

    let exe = std::env::current_exe().expect("current exe");
    let mut results = Vec::new();
    for which in ["c", "rust"] {
        let status = std::process::Command::new(&exe)
            .args([
                "row183_null_string_argument_kills_both_identically",
                "--exact",
                "--test-threads=1",
            ])
            .env("CJSON_CRASH_CHILD", which)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("spawn child");
        results.push(format!(
            "{which}: code={:?} signal={:?}",
            status.code(),
            status.signal()
        ));
    }
    assert_eq!(
        results[0].split_once(':').unwrap().1,
        results[1].split_once(':').unwrap().1,
        "row183: C and Rust died differently: {results:?}"
    );
    assert!(
        results[0].contains("signal=Some(11)"),
        "expected SIGSEGV, got {results:?}"
    );
}

#[test]
fn c90_driver_stdout_differential() {
    let c_lib = unsafe { libloading::Library::new(c_driver_so_path()) }
        .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_driver_so_path()));
    let r_lib = unsafe { libloading::Library::new(rust_driver_so_path()) }
        .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_driver_so_path()));
    let c_driver: libloading::Symbol<DriverFn> =
        unsafe { c_lib.get(b"driver\0") }.expect("C driver symbol");
    let r_driver: libloading::Symbol<DriverFn> =
        unsafe { r_lib.get(b"driver\0") }.expect("Rust driver symbol");

    let mut rng = Rng::new(0x600D_D11E_0000_0001);
    let mut cases = vec![Args::canonical()];
    for _ in 0..25 {
        cases.push(Args::random(&mut rng));
    }

    for (i, args) in cases.iter().enumerate() {
        let (rc_c, out_c) = unsafe {
            let mut rc = 0;
            let out = capture("c", || rc = args.call(*c_driver));
            (rc, out)
        };
        let (rc_r, out_r) = unsafe {
            let mut rc = 0;
            let out = capture("rust", || rc = args.call(*r_driver));
            (rc, out)
        };
        assert_eq!(rc_c, rc_r, "case {i}: driver return value differs");
        if out_c != out_r {
            let dir = std::env::temp_dir();
            let _ = std::fs::write(dir.join("driver_C.txt"), &out_c);
            let _ = std::fs::write(dir.join("driver_RUST.txt"), &out_r);
            let mut msg = format!(
                "case {i}: driver stdout differs (see {}/driver_{{C,RUST}}.txt)\n",
                dir.display()
            );
            for (n, (a, b)) in String::from_utf8_lossy(&out_c)
                .lines()
                .zip(String::from_utf8_lossy(&out_r).lines())
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .take(10)
            {
                let _ = write!(msg, "  line {n}:\n    C   : {a}\n    RUST: {b}\n");
            }
            panic!("{msg}");
        }
        if i == 0 {
            let text = String::from_utf8_lossy(&out_c);
            assert!(
                text.contains("Version: 1.7.19"),
                "captured output looks wrong: {text}"
            );
            assert!(text.contains("Jack (\\\"Bee\\\") Nimble"), "missing name: {text}");
            assert!(text.contains("SAN FRANCISCO"), "missing record: {text}");
            assert!(text.contains("\"number\":\tnull"), "missing 1/0 number: {text}");
        }
    }
}
