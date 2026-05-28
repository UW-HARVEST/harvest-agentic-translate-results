// Integration test: load both the C-built and Rust-built shared libraries
// via libloading, call exported FFI symbols on both, and assert the results
// match byte-for-byte.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type FmaArrayFn = unsafe extern "C" fn(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
);
type CallFmaFn = unsafe extern "C" fn(data: *const c_int, len: c_int) -> c_int;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    // Cargo sets OUT_DIR for build scripts only; tests run with the
    // workspace target dir as a sibling. We compile the cdylib via the
    // same `cargo test` invocation, so it lives in target/<profile>/.
    // CARGO_TARGET_TMPDIR is available in integration tests.
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| project_root().join("target"));

    // Try debug then release; tests usually use debug.
    let candidates = [
        target_dir.join("debug").join("libdriver.so"),
        target_dir.join("release").join("libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

struct Libs {
    _c: Library,
    _r: Library,
    c_fma_array: FmaArrayFn,
    r_fma_array: FmaArrayFn,
    c_call_fma: CallFmaFn,
    r_call_fma: CallFmaFn,
}

impl Libs {
    fn load() -> Self {
        let c_path = c_lib_path();
        let r_path = rust_lib_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {:?}. Build it with: cc -shared -fPIC c_src/src/main.c -o libdriver_c.so",
            c_path
        );
        assert!(
            r_path.exists(),
            "Rust shared library not found at {:?}. Build it with: cargo build",
            r_path
        );
        unsafe {
            let c = Library::new(&c_path).expect("load C lib");
            let r = Library::new(&r_path).expect("load Rust lib");

            let c_fma_array_sym: Symbol<FmaArrayFn> =
                c.get(b"fma_array\0").expect("C fma_array");
            let r_fma_array_sym: Symbol<FmaArrayFn> =
                r.get(b"fma_array\0").expect("R fma_array");
            let c_call_fma_sym: Symbol<CallFmaFn> = c.get(b"call_fma\0").expect("C call_fma");
            let r_call_fma_sym: Symbol<CallFmaFn> = r.get(b"call_fma\0").expect("R call_fma");

            let c_fma_array = *c_fma_array_sym;
            let r_fma_array = *r_fma_array_sym;
            let c_call_fma = *c_call_fma_sym;
            let r_call_fma = *r_call_fma_sym;

            Libs {
                _c: c,
                _r: r,
                c_fma_array,
                r_fma_array,
                c_call_fma,
                r_call_fma,
            }
        }
    }
}

fn run_fma_array_case(libs: &Libs, mul1: &[i32], mul2: &[i32], add: &[i32]) {
    let len = mul1.len();
    assert_eq!(len, mul2.len());
    assert_eq!(len, add.len());

    let mut out_c = vec![0i32; len.max(1)];
    let mut out_r = vec![0i32; len.max(1)];

    unsafe {
        (libs.c_fma_array)(
            out_c.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len as c_int,
        );
        (libs.r_fma_array)(
            out_r.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len as c_int,
        );
    }
    assert_eq!(
        out_c[..len],
        out_r[..len],
        "fma_array mismatch for inputs mul1={:?} mul2={:?} add={:?}",
        mul1,
        mul2,
        add
    );
}

fn run_call_fma_case(libs: &Libs, data: &[i32]) {
    let len = data.len() as c_int;
    let c_res = unsafe { (libs.c_call_fma)(data.as_ptr(), len) };
    let r_res = unsafe { (libs.r_call_fma)(data.as_ptr(), len) };
    assert_eq!(
        c_res, r_res,
        "call_fma mismatch for data={:?} (C={}, R={})",
        data, c_res, r_res
    );
}

#[test]
fn test_fma_array_basic() {
    let libs = Libs::load();
    run_fma_array_case(&libs, &[1, 2, 3], &[4, 5, 6], &[7, 8, 9]);
    run_fma_array_case(&libs, &[0], &[0], &[0]);
    run_fma_array_case(&libs, &[-1, -2, -3], &[2, 3, 4], &[10, 20, 30]);
}

#[test]
fn test_fma_array_overflow_wrap() {
    let libs = Libs::load();
    // Overflow cases — C signed overflow is UB but in practice wraps; the
    // Rust translation uses wrapping_* explicitly.
    run_fma_array_case(
        &libs,
        &[i32::MAX, i32::MIN],
        &[2, 2],
        &[0, 0],
    );
    run_fma_array_case(
        &libs,
        &[i32::MAX],
        &[1],
        &[1], // i32::MAX + 1 wraps to i32::MIN
    );
    run_fma_array_case(
        &libs,
        &[i32::MIN],
        &[-1],
        &[0], // overflow on multiply
    );
}

#[test]
fn test_fma_array_zero_len() {
    let libs = Libs::load();
    let empty: [i32; 0] = [];
    let mut out_c: [i32; 1] = [42];
    let mut out_r: [i32; 1] = [42];
    unsafe {
        (libs.c_fma_array)(
            out_c.as_mut_ptr(),
            empty.as_ptr(),
            empty.as_ptr(),
            empty.as_ptr(),
            0,
        );
        (libs.r_fma_array)(
            out_r.as_mut_ptr(),
            empty.as_ptr(),
            empty.as_ptr(),
            empty.as_ptr(),
            0,
        );
    }
    assert_eq!(out_c, out_r);
    assert_eq!(out_c, [42]); // unchanged
}

#[test]
fn test_call_fma_basic() {
    let libs = Libs::load();
    run_call_fma_case(&libs, &[]);
    run_call_fma_case(&libs, &[5]);
    run_call_fma_case(&libs, &[1, 2, 3]);
    run_call_fma_case(&libs, &[10, 20, 30, 40, 50]);
    run_call_fma_case(&libs, &[-7, 0, 99]);
}

#[test]
fn test_call_fma_returns_last_element() {
    let libs = Libs::load();
    // call_fma returns out[len-1] = ones[len-1]*data[len-1]+zeros[len-1]
    //                              = data[len-1].
    for data in [
        vec![42],
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        vec![i32::MIN],
        vec![i32::MAX, 0, -1],
        (0..100).collect::<Vec<i32>>(),
    ] {
        run_call_fma_case(&libs, &data);
    }
}

#[test]
fn test_call_fma_extremes() {
    let libs = Libs::load();
    run_call_fma_case(&libs, &[i32::MIN, i32::MAX, 0, -1, 1]);
    run_call_fma_case(&libs, &[i32::MIN; 10]);
    run_call_fma_case(&libs, &[i32::MAX; 10]);
}

#[test]
fn test_fma_array_random_like() {
    let libs = Libs::load();
    // A deterministic pseudo-random pattern.
    let mut a = Vec::with_capacity(100);
    let mut b = Vec::with_capacity(100);
    let mut c = Vec::with_capacity(100);
    let mut s: u32 = 0xdeadbeef;
    for _ in 0..100 {
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        a.push(s as i32);
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        b.push(s as i32);
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        c.push(s as i32);
    }
    run_fma_array_case(&libs, &a, &b, &c);
}

#[test]
fn test_exports_match() {
    // Ensure both libraries export the public API symbols we depend on.
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    unsafe {
        let c = Library::new(&c_path).expect("c lib");
        let r = Library::new(&r_path).expect("rust lib");
        let _: Symbol<FmaArrayFn> = c.get(b"fma_array\0").expect("C: fma_array missing");
        let _: Symbol<FmaArrayFn> = r.get(b"fma_array\0").expect("R: fma_array missing");
        let _: Symbol<CallFmaFn> = c.get(b"call_fma\0").expect("C: call_fma missing");
        let _: Symbol<CallFmaFn> = r.get(b"call_fma\0").expect("R: call_fma missing");
        // `main` is exported by both:
        type MainFn = unsafe extern "C" fn() -> c_int;
        let _: Symbol<MainFn> = c.get(b"main\0").expect("C: main missing");
        let _: Symbol<MainFn> = r.get(b"main\0").expect("R: main missing");
    }
}
