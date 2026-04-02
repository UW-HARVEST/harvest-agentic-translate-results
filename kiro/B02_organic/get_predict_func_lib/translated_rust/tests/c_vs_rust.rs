use libloading::{Library, Symbol};
use std::os::raw::c_int;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");

fn find_rust_lib() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // cargo builds cdylib into target/debug/
    for name in &["libget_predict_func_lib.so"] {
        let p = format!("{}/target/debug/{}", manifest, name);
        if std::path::Path::new(&p).exists() {
            return p;
        }
    }
    panic!("Rust .so not found - build with `cargo build` first");
}

type GetPredictFuncFn = unsafe extern "C" fn(c_int) -> c_int;

fn load_func(lib: &Library) -> GetPredictFuncFn {
    unsafe {
        let sym: Symbol<GetPredictFuncFn> = lib.get(b"get_predict_func").expect("Symbol not found");
        *sym
    }
}

#[test]
fn test_get_predict_func_all_cases() {
    let c_lib = unsafe { Library::new(C_LIB_PATH).expect("Failed to load C .so") };
    let rust_lib = unsafe { Library::new(find_rust_lib()).expect("Failed to load Rust .so") };
    let c_fn = load_func(&c_lib);
    let rust_fn = load_func(&rust_lib);

    let test_values: Vec<c_int> = (-2..=20).collect();
    for &pfcn in &test_values {
        let c_result = unsafe { c_fn(pfcn) };
        let rust_result = unsafe { rust_fn(pfcn) };
        assert_eq!(
            c_result, rust_result,
            "Mismatch for pfcn={}: C={}, Rust={}",
            pfcn, c_result, rust_result
        );
    }
}
