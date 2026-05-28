// Integration test: load both the C .so and the Rust .so and call hm_geti
// on both, verifying both complete successfully (no abort/panic).
//
// The C implementation is the ground truth; the Rust translation must
// behave equivalently (which, for hm_geti, means "no assertion fires").
//
// We always call the Rust function via libloading from its cdylib,
// exactly as an external caller would, to exercise the #[no_mangle]
// export wrappers as well.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // CARGO_TARGET_DIR or default `target/<profile>` location.
    // We try debug first, then release.
    let base = manifest_dir().join("target");
    let debug = base.join("debug").join("libhm_geti_lib.so");
    let release = base.join("release").join("libhm_geti_lib.so");
    if debug.exists() {
        debug
    } else {
        release
    }
}

unsafe fn call_hm_geti(lib_path: &PathBuf, num: c_int) {
    let lib = unsafe { Library::new(lib_path).expect("failed to load library") };
    let sym: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { lib.get(b"hm_geti").expect("hm_geti symbol not found") };
    unsafe { sym(num) };
}

#[test]
fn hm_geti_matches_for_various_inputs() {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();

    assert!(c_path.exists(), "C library not built at {:?}", c_path);
    assert!(r_path.exists(), "Rust library not built at {:?}", r_path);

    // hm_geti's only observable behavior is "did any assertion fire?".
    // We sweep a range of inputs that exercise both the empty-table
    // special case (num = 0, 1) and the loop bodies.
    for &num in &[0, 1, 2, 3, 4, 8, 16, 32, 64, 100, 256, 1000] {
        // Calling C: if any assert fires, abort() kills the process.
        unsafe { call_hm_geti(&c_path, num) };
        // Calling Rust: if any assert! fires, panic unwinds and the
        // test fails. Either way, the test catches a mismatch.
        unsafe { call_hm_geti(&r_path, num) };
    }
}
