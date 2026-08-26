use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
struct CnRndT {
    state: [u64; 2],
}

type NextDoubleFn = unsafe extern "C" fn(*mut CnRndT) -> f64;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libnext_double_lib.so")
}

#[test]
fn test_next_double_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
    let rust_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");

    let c_fn: Symbol<NextDoubleFn> = unsafe { c_lib.get(b"next_double") }.unwrap();
    let r_fn: Symbol<NextDoubleFn> = unsafe { rust_lib.get(b"next_double") }.unwrap();

    // Test with several seed states
    let seeds: &[[u64; 2]] = &[
        [0, 0],
        [1, 0],
        [0, 1],
        [1, 1],
        [u64::MAX, u64::MAX],
        [0x123456789ABCDEF0, 0xFEDCBA9876543210],
        [42, 7],
    ];

    for seed in seeds {
        let mut c_rnd = CnRndT { state: *seed };
        let mut r_rnd = CnRndT { state: *seed };

        // Call 100 times per seed and compare
        for i in 0..100 {
            let c_val = unsafe { c_fn(&mut c_rnd) };
            let r_val = unsafe { r_fn(&mut r_rnd) };
            assert_eq!(
                c_val.to_bits(), r_val.to_bits(),
                "Mismatch at iteration {i} for seed {seed:?}: C={c_val} Rust={r_val}"
            );
            assert_eq!(
                c_rnd.state, r_rnd.state,
                "State mismatch at iteration {i} for seed {seed:?}"
            );
        }
    }
}
