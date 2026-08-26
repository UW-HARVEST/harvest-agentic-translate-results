use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

type EncodeQuantFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_libs() -> (Library, Library) {
    let c_path = project_root().join("c_src/build/libtranslated_rust.so");
    let rust_path = project_root().join("target/debug/libencode_quant_lib.so");
    unsafe {
        let c_lib = Library::new(&c_path).expect("Failed to load C .so");
        let r_lib = Library::new(&rust_path).expect("Failed to load Rust .so");
        (c_lib, r_lib)
    }
}

fn call_both(
    c_lib: &Library,
    r_lib: &Library,
    uni: c_int,
    step: c_int,
    pred: c_int,
    tgt: c_int,
    tgt2: c_int,
    lsbit: c_int,
) -> (c_int, c_int) {
    unsafe {
        let c_fn: Symbol<EncodeQuantFn> = c_lib.get(b"encode_quant").unwrap();
        let r_fn: Symbol<EncodeQuantFn> = r_lib.get(b"encode_quant").unwrap();
        (c_fn(uni, step, pred, tgt, tgt2, lsbit), r_fn(uni, step, pred, tgt, tgt2, lsbit))
    }
}

#[test]
fn test_encode_quant_exhaustive() {
    let (c_lib, r_lib) = load_libs();

    // Test a broad set of inputs covering all branches
    let uni_vals: &[c_int] = &[0, 1, 7, 8, 9, 15, 16, -1, -8, -16, 100, -100, i32::MAX, i32::MIN];
    let step_vals: &[c_int] = &[0, 1, 8, 16, 100, -1, -100];
    let pred_vals: &[c_int] = &[0, 50, -50, 1000, -1000];
    let tgt_vals: &[c_int] = &[0, 50, -50, 1000, -1000];
    let tgt2_vals: &[c_int] = &[0, 50, -50, 1000, -1000];
    let lsbit_vals: &[c_int] = &[0, 1, 2, 3, 4, 5, 6];

    let mut count = 0u64;
    for &uni in uni_vals {
        for &step in step_vals {
            for &pred in pred_vals {
                for &tgt in tgt_vals {
                    for &tgt2 in tgt2_vals {
                        for &lsbit in lsbit_vals {
                            let (c_res, r_res) = call_both(&c_lib, &r_lib, uni, step, pred, tgt, tgt2, lsbit);
                            assert_eq!(
                                c_res, r_res,
                                "MISMATCH: encode_quant({uni}, {step}, {pred}, {tgt}, {tgt2}, {lsbit}) => C={c_res}, Rust={r_res}"
                            );
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    eprintln!("Tested {count} input combinations — all matched.");
}
