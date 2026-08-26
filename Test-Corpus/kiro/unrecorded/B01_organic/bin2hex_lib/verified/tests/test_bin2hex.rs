use libloading::{Library, Symbol};
use std::ffi::c_char;

type Bin2HexFn = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;

fn load_c_lib() -> Library {
    unsafe { Library::new(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so")).unwrap() }
}

fn load_rust_lib() -> Library {
    let dir = env!("CARGO_MANIFEST_DIR");
    let path = format!("{}/target/debug/libbin2hex_lib.so", dir);
    unsafe { Library::new(&path).unwrap() }
}

fn call_bin2hex(lib: &Library, input: &[u8]) -> Vec<u8> {
    unsafe {
        let func: Symbol<Bin2HexFn> = lib.get(b"bin2hex").unwrap();
        let hex_maxlen = input.len() * 2 + 1;
        let mut buf = vec![0u8; hex_maxlen];
        func(buf.as_mut_ptr() as *mut c_char, hex_maxlen, input.as_ptr(), input.len());
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        buf[..len].to_vec()
    }
}

#[test]
fn test_empty_input() {
    let c = load_c_lib();
    let r = load_rust_lib();
    assert_eq!(call_bin2hex(&c, &[]), call_bin2hex(&r, &[]));
}

#[test]
fn test_single_byte_zero() {
    let c = load_c_lib();
    let r = load_rust_lib();
    assert_eq!(call_bin2hex(&c, &[0x00]), call_bin2hex(&r, &[0x00]));
}

#[test]
fn test_single_byte_ff() {
    let c = load_c_lib();
    let r = load_rust_lib();
    assert_eq!(call_bin2hex(&c, &[0xff]), call_bin2hex(&r, &[0xff]));
}

#[test]
fn test_all_byte_values() {
    let c = load_c_lib();
    let r = load_rust_lib();
    let input: Vec<u8> = (0..=255).collect();
    let c_out = call_bin2hex(&c, &input);
    let r_out = call_bin2hex(&r, &input);
    assert_eq!(c_out, r_out, "Mismatch on all-bytes test.\nC:    {}\nRust: {}", String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
}

#[test]
fn test_known_values() {
    let c = load_c_lib();
    let r = load_rust_lib();
    for input in &[
        vec![0xde, 0xad, 0xbe, 0xef],
        vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
        vec![0x00, 0x00, 0x00],
        vec![0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f],
        vec![0xf0, 0xe0, 0xd0, 0xc0, 0xb0, 0xa0],
    ] {
        let c_out = call_bin2hex(&c, input);
        let r_out = call_bin2hex(&r, input);
        assert_eq!(c_out, r_out, "Mismatch for input {:02x?}\nC:    {}\nRust: {}", input, String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }
}

#[test]
fn test_return_value() {
    let c = load_c_lib();
    let r = load_rust_lib();
    unsafe {
        let func_c: Symbol<Bin2HexFn> = c.get(b"bin2hex").unwrap();
        let func_r: Symbol<Bin2HexFn> = r.get(b"bin2hex").unwrap();
        let input = [0xab, 0xcd];
        let mut buf_c = vec![0u8; 5];
        let mut buf_r = vec![0u8; 5];
        let ret_c = func_c(buf_c.as_mut_ptr() as *mut c_char, 5, input.as_ptr(), 2);
        let ret_r = func_r(buf_r.as_mut_ptr() as *mut c_char, 5, input.as_ptr(), 2);
        // Return value should be the same pointer passed in
        assert_eq!(ret_c, buf_c.as_mut_ptr() as *mut c_char);
        assert_eq!(ret_r, buf_r.as_mut_ptr() as *mut c_char);
        assert_eq!(buf_c, buf_r);
    }
}

#[test]
fn test_null_termination() {
    let c = load_c_lib();
    let r = load_rust_lib();
    unsafe {
        let func_c: Symbol<Bin2HexFn> = c.get(b"bin2hex").unwrap();
        let func_r: Symbol<Bin2HexFn> = r.get(b"bin2hex").unwrap();
        let input = [0x41];
        // Fill buffer with 0xFF to detect null termination
        let mut buf_c = vec![0xFFu8; 4];
        let mut buf_r = vec![0xFFu8; 4];
        func_c(buf_c.as_mut_ptr() as *mut c_char, 4, input.as_ptr(), 1);
        func_r(buf_r.as_mut_ptr() as *mut c_char, 4, input.as_ptr(), 1);
        assert_eq!(buf_c, buf_r);
        // Position 2 should be null terminator
        assert_eq!(buf_c[2], 0);
    }
}
