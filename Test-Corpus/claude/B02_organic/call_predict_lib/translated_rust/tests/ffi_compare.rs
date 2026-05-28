use libloading::{Library, Symbol};
use std::path::PathBuf;

type CallPredictFn = unsafe extern "C" fn(pfcn: i32) -> i32;

fn workspace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Tests run with the dev profile by default. Try debug then release.
    let dir = workspace_dir().join("target");
    let candidates = [
        dir.join("release/libcall_predict_lib.so"),
        dir.join("debug/libcall_predict_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    // fall back to release path even if it doesn't exist; load will fail loudly
    candidates[0].clone()
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("failed to load C .so");
        let r = Library::new(rust_lib_path()).expect("failed to load Rust .so");
        (c, r)
    }
}

#[test]
fn call_predict_matches_for_all_pfcn() {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<CallPredictFn> = c_lib.get(b"call_predict").expect("C symbol");
        let r_fn: Symbol<CallPredictFn> = r_lib.get(b"call_predict").expect("Rust symbol");

        // Cover documented pfcn values 0..=11 plus default-branch values
        // (negatives, 12..=15, and a few large ones).
        let mut inputs: Vec<i32> = (-3..=20).collect();
        inputs.push(100);
        inputs.push(-100);
        inputs.push(i32::MIN);
        inputs.push(i32::MAX);

        for pfcn in inputs {
            let c_val = c_fn(pfcn);
            let r_val = r_fn(pfcn);
            assert_eq!(
                c_val, r_val,
                "mismatch for pfcn={}: C={}, Rust={}",
                pfcn, c_val, r_val
            );
        }
    }
}
