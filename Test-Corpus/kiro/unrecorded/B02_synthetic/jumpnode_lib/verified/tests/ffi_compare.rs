use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type JumpnodeFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libjumpnode_lib.so")
}

fn load_jumpnode(lib: &Library) -> Symbol<JumpnodeFn> {
    unsafe { lib.get(b"jumpnode").expect("failed to load jumpnode symbol") }
}

/// Call jumpnode on both libraries and assert identical results.
fn compare(c_lib: &Library, rs_lib: &Library, mode: c_int, node_id: c_int, depth: c_int, flags: c_int) {
    let c_fn = load_jumpnode(c_lib);
    let rs_fn = load_jumpnode(rs_lib);
    let c_result = unsafe { c_fn(mode, node_id, depth, flags) };
    let rs_result = unsafe { rs_fn(mode, node_id, depth, flags) };
    assert_eq!(
        c_result, rs_result,
        "MISMATCH: jumpnode({mode}, {node_id}, {depth}, {flags}) => C={c_result}, Rust={rs_result}"
    );
}

#[test]
fn test_mode1_no_nodes() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // Mode 1 with no nodes: should return STATUS_ERROR | 0o20 = 18
    for node_id in [0, 1, 5, -1, 100] {
        compare(&c_lib, &rs_lib, 1, node_id, 0, 0);
        compare(&c_lib, &rs_lib, 1, node_id, 3, 1);
    }
}

#[test]
fn test_mode2_no_nodes() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // Mode 2 with no nodes: should return STATUS_ERROR | 0o40 = 34
    for node_id in [0, 1, -1, 99] {
        compare(&c_lib, &rs_lib, 2, node_id, 0, 0);
        compare(&c_lib, &rs_lib, 2, node_id, 5, 10);
    }
}

#[test]
fn test_mode3_various_inputs() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // Mode 3 doesn't depend on node state — tests sprintf + compute_size_metric + flags mask
    for node_id in [0, 1, 5, 42, -1, 100, 999, -999] {
        for depth in [0, 1, 5, 10, -1, 100] {
            for flags in [0, 1, 0x7F, 0xFF, 0, 63, 127, 128, 255, -1] {
                compare(&c_lib, &rs_lib, 3, node_id, depth, flags);
            }
        }
    }
}

#[test]
fn test_mode4_no_nodes() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // Mode 4 with no nodes: should return STATUS_ERROR | 0o100 = 66
    for node_id in [0, 1, -1, 50] {
        compare(&c_lib, &rs_lib, 4, node_id, 0, 0);
        compare(&c_lib, &rs_lib, 4, node_id, 3, 7);
    }
}

#[test]
fn test_default_mode() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // Any mode not 1-4 should return STATUS_ERROR | 0o200 = 130
    for mode in [0, 5, -1, 100, 255, -100] {
        compare(&c_lib, &rs_lib, mode, 0, 0, 0);
        compare(&c_lib, &rs_lib, mode, 1, 5, 10);
    }
}
