//! Phase B rows 97-98 — the `driver` entry point translated from
//! `c_src/test.c`.  Both the C `libcJSON_test.so` and the Rust `.so` are loaded
//! through `libloading`; `driver` is invoked with identical inputs and its
//! stdout is captured and compared byte-for-byte.
mod harness;
use harness::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int};
use std::io::Read;

type DriverFn = unsafe extern "C" fn(
    *const *const c_char,
    *mut [c_int; 3],
    *mut c_int,
    *mut record,
) -> c_int;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// Call `f` with fd 1 redirected into a temporary file and return what was
/// written.  `fflush(NULL)` is issued on both sides of the swap so nothing
/// leaks between captures.
unsafe fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    fflush(std::ptr::null_mut());
    let path = std::env::temp_dir().join(format!(
        "cjson_driver_{}_{}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).unwrap();
    let saved = dup(1);
    assert!(saved >= 0);
    {
        use std::os::unix::io::AsRawFd;
        assert!(dup2(file.as_raw_fd(), 1) >= 0);
    }
    f();
    fflush(std::ptr::null_mut());
    assert!(dup2(saved, 1) >= 0);
    close(saved);
    drop(file);
    let mut out = Vec::new();
    std::fs::File::open(&path)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    let _ = std::fs::remove_file(&path);
    out
}

struct Inputs {
    strings: Vec<Vec<u8>>,
    numbers: Vec<[c_int; 3]>,
    ids: Vec<c_int>,
    /* keep the string storage alive for the whole call */
    _store: Vec<Vec<u8>>,
    fields: Vec<record>,
}

fn default_inputs() -> Inputs {
    build_inputs(
        &[
            "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
        ],
        [[0, -1, 2], [3, 4, 5], [6, 7, 8]],
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
    )
}

#[allow(clippy::type_complexity)]
fn build_inputs(
    strings: &[&str; 7],
    numbers: [[c_int; 3]; 3],
    ids: [c_int; 4],
    fields: &[(&str, f64, f64, &str, &str, &str, &str, &str); 2],
) -> Inputs {
    let sbuf: Vec<Vec<u8>> = strings.iter().map(|s| cstr(s)).collect();
    let mut store: Vec<Vec<u8>> = Vec::new();
    for f in fields {
        store.push(cstr(f.0));
        store.push(cstr(f.3));
        store.push(cstr(f.4));
        store.push(cstr(f.5));
        store.push(cstr(f.6));
        store.push(cstr(f.7));
    }
    let mut recs = Vec::new();
    for (i, f) in fields.iter().enumerate() {
        let b = i * 6;
        recs.push(record {
            precision: store[b].as_ptr() as *const c_char,
            lat: f.1 as c_double,
            lon: f.2 as c_double,
            address: store[b + 1].as_ptr() as *const c_char,
            city: store[b + 2].as_ptr() as *const c_char,
            state: store[b + 3].as_ptr() as *const c_char,
            zip: store[b + 4].as_ptr() as *const c_char,
            country: store[b + 5].as_ptr() as *const c_char,
        });
    }
    Inputs {
        strings: sbuf,
        numbers: numbers.to_vec(),
        ids: ids.to_vec(),
        _store: store,
        fields: recs,
    }
}

unsafe fn run_driver(f: DriverFn, inp: &mut Inputs) -> (c_int, Vec<u8>) {
    let ptrs: Vec<*const c_char> = inp.strings.iter().map(|s| s.as_ptr() as *const c_char).collect();
    let mut rc = 0;
    let out = capture_stdout(|| {
        rc = f(
            ptrs.as_ptr(),
            inp.numbers.as_mut_ptr(),
            inp.ids.as_mut_ptr(),
            inp.fields.as_mut_ptr(),
        );
    });
    (rc, out)
}

fn load_driver(path: &std::path::Path) -> (Library, DriverFn) {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
    let f = unsafe {
        let s: Symbol<DriverFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("{}: no `driver` symbol: {e}", path.display()));
        *s
    };
    (lib, f)
}

#[test]
fn row97_driver_default_inputs_stdout_identical() {
    let _g = lock();
    let (_cl, cf) = load_driver(&c_driver_so_path());
    let (_rl, rf) = load_driver(&rust_so_path());
    unsafe {
        let mut ci = default_inputs();
        let mut ri = default_inputs();
        let (crc, cout) = run_driver(cf, &mut ci);
        let (rrc, rout) = run_driver(rf, &mut ri);
        assert_eq!(crc, rrc, "driver return value");
        assert!(cout.len() > 500, "driver output too small: {} bytes", cout.len());
        eprintln!("driver stdout captured: {} bytes, {} lines", cout.len(), cout.iter().filter(|&&b| b==b'\n').count());
        if cout != rout {
            let c = String::from_utf8_lossy(&cout);
            let r = String::from_utf8_lossy(&rout);
            for (i, (a, b)) in c.lines().zip(r.lines()).enumerate() {
                if a != b {
                    panic!("driver stdout differs at line {i}:\n  C: {a:?}\n  R: {b:?}");
                }
            }
            panic!(
                "driver stdout differs in length: C={} R={}",
                cout.len(),
                rout.len()
            );
        }
    }
}

#[test]
fn row98_driver_randomized_inputs_stdout_identical() {
    let _g = lock();
    let (_cl, cf) = load_driver(&c_driver_so_path());
    let (_rl, rf) = load_driver(&rust_so_path());
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    unsafe {
        for iter in 0..60 {
            // Build randomized-but-printable inputs; the driver embeds them in
            // JSON strings, so escape-triggering bytes are deliberately used.
            let owned: Vec<String> = (0..7)
                .map(|i| {
                    let raw = rng.cstring(12);
                    let mut s: String = raw
                        .iter()
                        .map(|&b| if b < 0x20 || b >= 0x7f { '\u{1}' } else { b as char })
                        .collect();
                    s.push_str(&format!("#{i}"));
                    s
                })
                .collect();
            let strings: [&str; 7] = [
                &owned[0], &owned[1], &owned[2], &owned[3], &owned[4], &owned[5], &owned[6],
            ];
            let numbers = [
                [rng.i32(), rng.i32(), rng.i32()],
                [rng.i32(), rng.i32(), rng.i32()],
                [rng.i32(), rng.i32(), rng.i32()],
            ];
            let ids = [rng.i32(), rng.i32(), rng.i32(), rng.i32()];
            let f_owned: Vec<String> = (0..12)
                .map(|_| {
                    rng.cstring(10)
                        .iter()
                        .map(|&b| if b < 0x20 || b >= 0x7f { 'q' } else { b as char })
                        .collect()
                })
                .collect();
            let lats = [rng.nice_f64(), rng.nice_f64()];
            let lons = [rng.nice_f64(), rng.nice_f64()];
            let fields = [
                (
                    f_owned[0].as_str(),
                    lats[0],
                    lons[0],
                    f_owned[1].as_str(),
                    f_owned[2].as_str(),
                    f_owned[3].as_str(),
                    f_owned[4].as_str(),
                    f_owned[5].as_str(),
                ),
                (
                    f_owned[6].as_str(),
                    lats[1],
                    lons[1],
                    f_owned[7].as_str(),
                    f_owned[8].as_str(),
                    f_owned[9].as_str(),
                    f_owned[10].as_str(),
                    f_owned[11].as_str(),
                ),
            ];
            let mut ci = build_inputs(&strings, numbers, ids, &fields);
            let mut rio = build_inputs(&strings, numbers, ids, &fields);
            let (crc, cout) = run_driver(cf, &mut ci);
            let (rrc, rout) = run_driver(rf, &mut rio);
            assert_eq!(crc, rrc, "iter {iter}: driver rc");
            if cout != rout {
                let c = String::from_utf8_lossy(&cout);
                let r = String::from_utf8_lossy(&rout);
                for (i, (a, b)) in c.lines().zip(r.lines()).enumerate() {
                    if a != b {
                        panic!(
                            "iter {iter}: driver stdout differs at line {i}:\n  C: {a:?}\n  R: {b:?}"
                        );
                    }
                }
                panic!("iter {iter}: stdout length differs {} vs {}", cout.len(), rout.len());
            }
        }
    }
}
