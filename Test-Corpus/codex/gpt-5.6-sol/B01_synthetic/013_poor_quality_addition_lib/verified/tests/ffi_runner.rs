use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::Path;

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
}

fn decode_hex(encoded: &str) -> Vec<u8> {
    assert!(encoded.len().is_multiple_of(2), "invalid hex input");
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("hex input is not UTF-8");
            u8::from_str_radix(text, 16).expect("invalid hex byte")
        })
        .collect()
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(library_path) = arguments.next() else {
        return;
    };
    let mode = arguments.next().expect("missing invocation mode");
    let library =
        unsafe { Library::new(Path::new(&library_path)).expect("failed to load shared library") };

    match mode.as_str() {
        "printLine" => {
            let values = arguments
                .map(|value| CString::new(decode_hex(&value)).unwrap())
                .collect::<Vec<_>>();
            let function: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { library.get(b"printLine\0").expect("missing printLine") };
            for value in &values {
                unsafe {
                    function(value.as_ptr());
                }
            }
        }
        "printLineNull" => {
            let function: Symbol<unsafe extern "C" fn(*const c_char)> =
                unsafe { library.get(b"printLine\0").expect("missing printLine") };
            unsafe {
                function(std::ptr::null());
            }
        }
        "printIntLine" => {
            let function: Symbol<unsafe extern "C" fn(c_int)> = unsafe {
                library
                    .get(b"printIntLine\0")
                    .expect("missing printIntLine")
            };
            for value in arguments {
                unsafe {
                    function(value.parse::<c_int>().expect("invalid C int"));
                }
            }
        }
        "bad" | "good" | "driver" => {
            let function: Symbol<unsafe extern "C" fn()> = unsafe {
                library
                    .get(format!("{mode}\0").as_bytes())
                    .expect("missing no-argument symbol")
            };
            unsafe {
                function();
            }
        }
        _ => panic!("unknown invocation mode: {mode}"),
    }

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush failed");
    }
}
