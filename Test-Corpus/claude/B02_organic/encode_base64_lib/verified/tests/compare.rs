use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

type EncodeBase64Fn =
    unsafe extern "C" fn(size: c_int, src: *const c_char) -> *mut c_char;
type FreeFn = unsafe extern "C" fn(*mut std::ffi::c_void);

const C_LIB: &str = "c_src/build/libdriver.so";
const RUST_LIB: &str = "target/release/libdriver.so";

fn libc_free() -> Library {
    unsafe { Library::new("libc.so.6").expect("load libc") }
}

fn run_encode(lib: &Library, size: c_int, src: *const c_char) -> Option<Vec<u8>> {
    unsafe {
        let f: Symbol<EncodeBase64Fn> = lib.get(b"encode_base64").unwrap();
        let ptr = f(size, src);
        if ptr.is_null() {
            return None;
        }
        let s = CStr::from_ptr(ptr).to_bytes().to_vec();
        let libc = libc_free();
        let free: Symbol<FreeFn> = libc.get(b"free").unwrap();
        free(ptr as *mut _);
        Some(s)
    }
}

fn compare(input: &[u8], explicit_size: Option<c_int>) {
    let c_lib = unsafe { Library::new(C_LIB).expect("load c lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load rust lib") };

    let size = explicit_size.unwrap_or(input.len() as c_int);
    let src_ptr = input.as_ptr() as *const c_char;

    let c_out = run_encode(&c_lib, size, src_ptr);
    let r_out = run_encode(&rust_lib, size, src_ptr);
    assert_eq!(c_out, r_out, "encode_base64 mismatch for {:?}", input);
}

#[test]
fn test_null_input() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load c lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load rust lib") };
    unsafe {
        let f_c: Symbol<EncodeBase64Fn> = c_lib.get(b"encode_base64").unwrap();
        let f_r: Symbol<EncodeBase64Fn> = rust_lib.get(b"encode_base64").unwrap();
        let p_c = f_c(0, std::ptr::null());
        let p_r = f_r(0, std::ptr::null());
        assert!(p_c.is_null());
        assert!(p_r.is_null());
    }
}

#[test]
fn test_empty_string_size_zero() {
    // size==0 triggers strlen of src; src is empty => strlen == 0
    let s = b"\0";
    compare(&s[..0], Some(0));
    // Pass an actual NUL-terminated empty C string with size==0
    let cstr = b"\0";
    let c_lib = unsafe { Library::new(C_LIB).expect("load c lib") };
    let rust_lib = unsafe { Library::new(RUST_LIB).expect("load rust lib") };
    let c_out = run_encode(&c_lib, 0, cstr.as_ptr() as *const c_char);
    let r_out = run_encode(&rust_lib, 0, cstr.as_ptr() as *const c_char);
    assert_eq!(c_out, r_out);
}

#[test]
fn test_one_byte() {
    let inputs: &[&[u8]] = &[b"A", b"a", b"0", b"\xff", b"\x00"];
    for input in inputs {
        // explicit size to avoid strlen issues with embedded nul
        let mut buf = input.to_vec();
        buf.push(0);
        compare(&buf[..input.len()], Some(input.len() as c_int));
    }
}

#[test]
fn test_two_bytes() {
    let inputs: &[&[u8]] = &[b"AB", b"\xff\xff", b"\x00\x00", b"hi"];
    for input in inputs {
        let mut buf = input.to_vec();
        buf.push(0);
        compare(&buf[..input.len()], Some(input.len() as c_int));
    }
}

#[test]
fn test_three_bytes() {
    let inputs: &[&[u8]] = &[b"ABC", b"abc", b"\xff\xff\xff", b"\x00\x01\x02"];
    for input in inputs {
        let mut buf = input.to_vec();
        buf.push(0);
        compare(&buf[..input.len()], Some(input.len() as c_int));
    }
}

#[test]
fn test_known_vectors() {
    // RFC 4648 test vectors
    let cases: &[(&[u8], &str)] = &[
        (b"", ""),
        (b"f", "Zg=="),
        (b"fo", "Zm8="),
        (b"foo", "Zm9v"),
        (b"foob", "Zm9vYg=="),
        (b"fooba", "Zm9vYmE="),
        (b"foobar", "Zm9vYmFy"),
    ];
    for (input, expected) in cases {
        let mut buf = input.to_vec();
        buf.push(0);
        let size = input.len() as c_int;
        let c_lib = unsafe { Library::new(C_LIB).expect("load c lib") };
        let rust_lib = unsafe { Library::new(RUST_LIB).expect("load rust lib") };
        let c_out = run_encode(&c_lib, size, buf.as_ptr() as *const c_char);
        let r_out = run_encode(&rust_lib, size, buf.as_ptr() as *const c_char);
        assert_eq!(c_out, r_out, "mismatch on {:?}", input);
        if !expected.is_empty() {
            assert_eq!(
                c_out.as_deref().map(|b| std::str::from_utf8(b).unwrap()),
                Some(*expected),
                "C output wrong for {:?}",
                input
            );
        }
    }
}

#[test]
fn test_size_zero_uses_strlen() {
    // size=0 means "use strlen". Need NUL terminator.
    let inputs: &[&[u8]] = &[
        b"hello\0",
        b"foobar\0",
        b"the quick brown fox\0",
        b"a\0",
    ];
    for input in inputs {
        let c_lib = unsafe { Library::new(C_LIB).expect("load c lib") };
        let rust_lib = unsafe { Library::new(RUST_LIB).expect("load rust lib") };
        let c_out = run_encode(&c_lib, 0, input.as_ptr() as *const c_char);
        let r_out = run_encode(&rust_lib, 0, input.as_ptr() as *const c_char);
        assert_eq!(c_out, r_out, "mismatch for {:?}", input);
    }
}

#[test]
fn test_long_input() {
    let mut data: Vec<u8> = (0u8..=255).cycle().take(1000).collect();
    let size = data.len() as c_int;
    data.push(0);
    compare(&data[..size as usize], Some(size));
}

#[test]
fn test_all_byte_values_explicit_size() {
    let mut data: Vec<u8> = (0u8..=255).collect();
    let size = data.len() as c_int;
    data.push(0);
    compare(&data[..size as usize], Some(size));
}

#[test]
fn test_lengths_modulo_three() {
    // Cover all length residues mod 3
    for len in 1..=20usize {
        let mut data: Vec<u8> = (0u8..len as u8).map(|i| i.wrapping_mul(7)).collect();
        let size = data.len() as c_int;
        data.push(0);
        compare(&data[..size as usize], Some(size));
    }
}
