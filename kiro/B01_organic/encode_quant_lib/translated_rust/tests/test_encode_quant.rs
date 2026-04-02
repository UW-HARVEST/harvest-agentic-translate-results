use libloading::{Library, Symbol};
use std::os::raw::c_int;

type EncodeQuantFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

fn load_c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libencode_quant_lib.so");
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn load_rust_lib() -> Library {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // cdylib is built in target/debug/
    let path = format!("{manifest}/target/debug/libencode_quant_lib.so");
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

fn call(lib: &Library, uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32, lsbit: i32) -> i32 {
    unsafe {
        let f: Symbol<EncodeQuantFn> = lib.get(b"encode_quant").unwrap();
        f(uni, step, pred, tgt, tgt2, lsbit)
    }
}

const TEST_CASES: &[(i32, i32, i32, i32, i32, i32)] = &[
    // lsbit == 0
    (5, 100, 500, 520, 530, 0),
    (0, 50, 0, 10, 20, 0),
    (15, 200, 1000, 1050, 1100, 0),
    (7, 80, 300, 310, 320, 0),
    (8, 120, 400, 380, 390, 0),
    // lsbit == 4
    (5, 100, 500, 520, 530, 4),
    (0, 50, 0, 10, 20, 4),
    (15, 200, 1000, 1050, 1100, 4),
    // lsbit odd
    (5, 100, 500, 520, 530, 1),
    (5, 100, 500, 520, 530, 3),
    (0, 50, 0, 10, 20, 1),
    // lsbit even nonzero (not 4)
    (5, 100, 500, 520, 530, 2),
    (0, 50, 0, 10, 20, 6),
    // boundary: uni at octet boundary
    (7, 100, 500, 520, 530, 0),
    (8, 100, 500, 520, 530, 0),
    (0, 100, 500, 520, 530, 0),
    (8, 100, 500, 480, 490, 0),
    // negative pred/tgt
    (5, 100, -500, -480, -470, 0),
    (10, 200, -1000, -950, -900, 0),
    // large step
    (5, 10000, 500, 5000, 6000, 0),
    // zero step
    (5, 0, 500, 520, 530, 0),
    // all zeros
    (0, 0, 0, 0, 0, 0),
];

#[test]
fn test_encode_quant_matches_c() {
    // First build the Rust cdylib
    let c_lib = load_c_lib();
    let rust_lib = load_rust_lib();
    for &(uni, step, pred, tgt, tgt2, lsbit) in TEST_CASES {
        let c_result = call(&c_lib, uni, step, pred, tgt, tgt2, lsbit);
        let rust_result = call(&rust_lib, uni, step, pred, tgt, tgt2, lsbit);
        assert_eq!(
            c_result, rust_result,
            "MISMATCH for encode_quant({uni}, {step}, {pred}, {tgt}, {tgt2}, {lsbit}): C={c_result}, Rust={rust_result}"
        );
    }
}
