use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libcontainer_of.so")
}

fn rust_lib_path() -> PathBuf {
    // cdylib is built in the deps dir or directly in target/debug
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    dir.join("libcontainer_of.so")
}

type FindContainerFn = unsafe extern "C" fn(*const i32) -> *const Test;

fn load_fn<'a>(lib: &'a Library, name: &[u8]) -> Symbol<'a, FindContainerFn> {
    unsafe { lib.get(name).unwrap() }
}

#[test]
fn test_find_container_of_a() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn = load_fn(&c_lib, b"find_container_of_a");
    let r_fn = load_fn(&r_lib, b"find_container_of_a");

    for (a, b) in [(1, 2), (0, 0), (-1, 100), (i32::MAX, i32::MIN)] {
        let t = Test { a, b };
        let ptr = &t.a as *const i32;

        let c_result = unsafe { &*c_fn(ptr) };
        let r_result = unsafe { &*r_fn(ptr) };

        assert_eq!(c_result.a, r_result.a, "a mismatch for input ({a}, {b})");
        assert_eq!(c_result.b, r_result.b, "b mismatch for input ({a}, {b})");
        // Both should return pointer to the original struct
        assert_eq!(c_result.a, a);
        assert_eq!(c_result.b, b);
    }
}

#[test]
fn test_find_container_of_b() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let c_fn = load_fn(&c_lib, b"find_container_of_b");
    let r_fn = load_fn(&r_lib, b"find_container_of_b");

    for (a, b) in [(1, 2), (0, 0), (-1, 100), (i32::MAX, i32::MIN)] {
        let t = Test { a, b };
        let ptr = &t.b as *const i32;

        let c_result = unsafe { &*c_fn(ptr) };
        let r_result = unsafe { &*r_fn(ptr) };

        assert_eq!(c_result.a, r_result.a, "a mismatch for input ({a}, {b})");
        assert_eq!(c_result.b, r_result.b, "b mismatch for input ({a}, {b})");
        assert_eq!(c_result.a, a);
        assert_eq!(c_result.b, b);
    }
}
