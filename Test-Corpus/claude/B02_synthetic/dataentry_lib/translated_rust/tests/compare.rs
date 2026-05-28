use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type DataentryFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn c_lib_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The Rust cdylib is built next to the test binary; the harness tells
    // us the binary location via CARGO_MANIFEST_DIR, but the lib is in
    // target/<profile>/. Cargo runs tests with the lib already in
    // target/<profile>/deps for cdylib.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try both debug and release.
    for prof in &["debug", "release"] {
        let p = manifest_dir.join("target").join(prof).join("libdataentry_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("could not locate libdataentry_lib.so");
}

struct Libs {
    _c: Library,
    _r: Library,
    c_fn: DataentryFn,
    r_fn: DataentryFn,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            let c = Library::new(c_lib_path()).expect("load C lib");
            let r = Library::new(rust_lib_path()).expect("load Rust lib");
            let c_sym: Symbol<DataentryFn> = c.get(b"dataentry").expect("C dataentry");
            let r_sym: Symbol<DataentryFn> = r.get(b"dataentry").expect("Rust dataentry");
            let c_fn = *c_sym;
            let r_fn = *r_sym;
            Libs { _c: c, _r: r, c_fn, r_fn }
        }
    }

    fn call(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> (c_int, c_int) {
        unsafe {
            let cv = (self.c_fn)(a, b, c, d);
            let rv = (self.r_fn)(a, b, c, d);
            (cv, rv)
        }
    }

    fn assert_eq(&self, a: c_int, b: c_int, c: c_int, d: c_int) {
        let (cv, rv) = self.call(a, b, c, d);
        assert_eq!(cv, rv,
            "mismatch for dataentry({}, {}, {}, {}): C={} Rust={}",
            a, b, c, d, cv, rv);
    }
}

#[test]
fn mode_1_basic() {
    let libs = Libs::load();
    // Mode 1: create count entries with base_id=100, find target=100+param2
    libs.assert_eq(1, 5, 0, 0);   // find id=100
    libs.assert_eq(1, 5, 1, 0);   // find id=101
    libs.assert_eq(1, 5, 4, 0);   // find id=104
    libs.assert_eq(1, 5, 99, 0);  // not found -> -2
    libs.assert_eq(1, 0, 0, 0);   // count defaults to 5
    libs.assert_eq(1, -1, 0, 0);  // count defaults to 5
    libs.assert_eq(1, 1, 0, 0);   // count=1, find id=100
    libs.assert_eq(1, 10, 5, 0);  // count=10
    libs.assert_eq(1, 3, 2, 0);   // count=3, find id=102
}

#[test]
fn mode_2_basic() {
    let libs = Libs::load();
    // Mode 2: create count entries base 200, multiplier=param2, add param3
    libs.assert_eq(2, 3, 1, 0);
    libs.assert_eq(2, 3, 2, 5);
    libs.assert_eq(2, 0, 2, 5);   // defaults to 3
    libs.assert_eq(2, -1, 2, 5);  // defaults to 3
    libs.assert_eq(2, 5, 0, 100); // multiplier=0 -> entries.value*0=0 ; condition requires temp_value!=0; since base 200..204 nonzero, result=0; total stays 0; result=0 means no add
    libs.assert_eq(2, 1, 3, -10);
    libs.assert_eq(2, 4, -1, 5);
}

#[test]
fn mode_3_basic() {
    let libs = Libs::load();
    // Mode 3: lookup table[row][col]
    for row in 0..4 {
        for col in 0..3 {
            libs.assert_eq(3, row, col, 0);
            libs.assert_eq(3, row, col, 7);
        }
    }
    // Out of range
    libs.assert_eq(3, -1, 0, 0);
    libs.assert_eq(3, 4, 0, 0);
    libs.assert_eq(3, 0, -1, 0);
    libs.assert_eq(3, 0, 3, 0);
    libs.assert_eq(3, 5, 5, 5);
}

#[test]
fn mode_default() {
    let libs = Libs::load();
    // Default branch (mode != 1,2,3)
    libs.assert_eq(0, 0, 0, 0);
    libs.assert_eq(0, 1, 0, 0);
    libs.assert_eq(0, 5, 0, 0);
    libs.assert_eq(0, -3, 0, 0);
    libs.assert_eq(99, 7, 0, 0);
    libs.assert_eq(-1, 4, 0, 0);
}

#[test]
fn comprehensive_grid() {
    let libs = Libs::load();
    for mode in &[-2i32, -1, 0, 1, 2, 3, 4, 5, 100] {
        for p1 in &[-2i32, -1, 0, 1, 2, 3, 4, 5, 10] {
            for p2 in &[-2i32, -1, 0, 1, 2, 3, 4, 7] {
                for p3 in &[-5i32, 0, 1, 7, 100] {
                    libs.assert_eq(*mode, *p1, *p2, *p3);
                }
            }
        }
    }
}
