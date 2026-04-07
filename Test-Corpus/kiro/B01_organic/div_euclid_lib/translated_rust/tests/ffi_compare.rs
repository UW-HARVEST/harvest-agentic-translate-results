use libloading::{Library, Symbol};
use std::ffi::c_int;

const INT_MIN: c_int = -0x7fffffff - 1;
const INT_MAX: c_int = 0x7fffffff;

fn libs() -> (Library, Library) {
    let c_lib = unsafe { Library::new("c_src/build/libtranslated_rust.so") }
        .expect("failed to load C .so");
    let rust_lib = unsafe {
        Library::new(format!(
            "target/debug/lib{}.so",
            "div_euclid_lib"
        ))
    }
    .expect("failed to load Rust .so");
    (c_lib, rust_lib)
}

type DivEuclid = unsafe extern "C" fn(c_int, c_int) -> c_int;

#[test]
fn test_div_euclid_exhaustive() {
    let (c_lib, rust_lib) = libs();
    let c_fn: Symbol<DivEuclid> = unsafe { c_lib.get(b"div_euclid") }.unwrap();
    let r_fn: Symbol<DivEuclid> = unsafe { rust_lib.get(b"div_euclid") }.unwrap();

    // Edge values to test
    let vals: &[c_int] = &[
        INT_MIN, INT_MIN + 1, INT_MIN + 2,
        -1000, -7, -3, -2, -1,
        0,
        1, 2, 3, 7, 1000,
        INT_MAX - 2, INT_MAX - 1, INT_MAX,
    ];

    let mut failures = Vec::new();
    for &v1 in vals {
        for &v2 in vals {
            let c_res = unsafe { c_fn(v1, v2) };
            let r_res = unsafe { r_fn(v1, v2) };
            if c_res != r_res {
                failures.push((v1, v2, c_res, r_res));
            }
        }
    }
    if !failures.is_empty() {
        for (v1, v2, c, r) in &failures {
            eprintln!("MISMATCH: div_euclid({v1}, {v2}) C={c} Rust={r}");
        }
        panic!("{} mismatches found", failures.len());
    }
}
