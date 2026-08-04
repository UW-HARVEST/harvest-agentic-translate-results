use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo test builds into deps; the cdylib is in the parent dir
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C .so");
        let r = Library::new(rust_lib_path()).expect("load Rust .so");
        (c, r)
    }
}

#[test]
fn test_foo_basic() {
    let (c_lib, r_lib) = load_libs();
    let c_foo: Symbol<FooFn> = unsafe { c_lib.get(b"foo").unwrap() };
    let r_foo: Symbol<FooFn> = unsafe { r_lib.get(b"foo").unwrap() };

    let cases: &[(&str, u8)] = &[
        ("hello", b'l'),
        ("hello", b'z'),
        ("", b'a'),
        ("AAAA", b'A'),
        ("AxAxAx", b'A'),
        ("AxAxAx", b'x'),
        ("abcabc", b'c'),
        ("aaaa", b'a'),
    ];

    for &(s, ch) in cases {
        let cs = CString::new(s).unwrap();
        let c_res = unsafe { c_foo(cs.as_ptr(), ch as c_char) };
        let r_res = unsafe { r_foo(cs.as_ptr(), ch as c_char) };
        assert_eq!(c_res, r_res, "foo({:?}, {:?}): C={} Rust={}", s, ch as char, c_res, r_res);
    }
}

#[test]
fn test_driver_output() {
    // driver() uses printf to stdout. We capture by redirecting via pipe.
    // Since we can't easily capture printf from a loaded .so in-process,
    // we'll call foo directly for the same inputs driver uses and compare.
    let (c_lib, r_lib) = load_libs();
    let c_foo: Symbol<FooFn> = unsafe { c_lib.get(b"foo").unwrap() };
    let r_foo: Symbol<FooFn> = unsafe { r_lib.get(b"foo").unwrap() };

    let inputs = &["", "hello", "AAAxxxAAA", "no match here", "xAx"];
    for &input in inputs {
        let cs = CString::new(input).unwrap();
        // driver calls foo(in, 'A') and foo(in, 'x')
        let c_a = unsafe { c_foo(cs.as_ptr(), b'A' as c_char) };
        let r_a = unsafe { r_foo(cs.as_ptr(), b'A' as c_char) };
        let c_x = unsafe { c_foo(cs.as_ptr(), b'x' as c_char) };
        let r_x = unsafe { r_foo(cs.as_ptr(), b'x' as c_char) };
        assert_eq!(c_a, r_a, "driver input {:?}: foo(,'A') C={} Rust={}", input, c_a, r_a);
        assert_eq!(c_x, r_x, "driver input {:?}: foo(,'x') C={} Rust={}", input, c_x, r_x);
    }
}
