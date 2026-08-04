use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    // cdylib is built in target/debug/
    format!("{dir}/target/debug/libdriver.so")
}

type CustomStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;

fn load_fn(lib: &Library) -> Symbol<CustomStrdup> {
    unsafe { lib.get(b"custom_strdup") }.expect("symbol not found")
}

fn call_and_compare(input: Option<&CStr>) {
    let c_lib = unsafe { Library::new(C_LIB) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");
    let c_fn = load_fn(&c_lib);
    let r_fn = load_fn(&r_lib);

    let ptr = input.map_or(std::ptr::null(), |s| s.as_ptr());

    unsafe {
        let c_out = c_fn(ptr);
        let r_out = r_fn(ptr);

        if ptr.is_null() {
            assert!(c_out.is_null(), "C should return null for null input");
            assert!(r_out.is_null(), "Rust should return null for null input");
        } else {
            assert!(!c_out.is_null());
            assert!(!r_out.is_null());
            let c_bytes = CStr::from_ptr(c_out).to_bytes_with_nul();
            let r_bytes = CStr::from_ptr(r_out).to_bytes_with_nul();
            assert_eq!(c_bytes, r_bytes, "mismatch for input {:?}", input);
            libc::free(c_out as *mut _);
            libc::free(r_out as *mut _);
        }
    }
}

#[test]
fn null_input() {
    call_and_compare(None);
}

#[test]
fn empty_string() {
    call_and_compare(Some(&CString::new("").unwrap()));
}

#[test]
fn simple_string() {
    call_and_compare(Some(&CString::new("hello world").unwrap()));
}

#[test]
fn binary_content() {
    // String with high bytes (but no interior NUL since it's C string)
    call_and_compare(Some(&CString::new(vec![0x80, 0xFF, 0x01, 0x7F]).unwrap()));
}

#[test]
fn long_string() {
    let s = CString::new("A".repeat(10000)).unwrap();
    call_and_compare(Some(&s));
}
