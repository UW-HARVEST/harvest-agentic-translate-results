use libloading::{Library, Symbol};
use std::ffi::c_char;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libbin2hex_lib.so");

type Bin2HexFn = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;

fn call_bin2hex(f: Bin2HexFn, input: &[u8]) -> Vec<u8> {
    let hex_maxlen = input.len() * 2 + 1;
    let mut buf = vec![0u8; hex_maxlen];
    unsafe { f(buf.as_mut_ptr() as *mut c_char, hex_maxlen, input.as_ptr(), input.len()) };
    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(nul);
    buf
}

#[test]
fn test_bin2hex_matches() {
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let c_fn: Symbol<Bin2HexFn> = unsafe { c_lib.get(b"bin2hex").expect("find bin2hex") };

    let inputs: &[&[u8]] = &[
        &[],
        &[0x00],
        &[0xff],
        &[0xde, 0xad, 0xbe, 0xef],
        &[0x00, 0x01, 0x09, 0x0a, 0x0f, 0x10, 0x7f, 0x80, 0xfe, 0xff],
        b"Hello, World!",
        &(0..=255).collect::<Vec<u8>>(),
    ];

    for input in inputs {
        let c_out = call_bin2hex(*c_fn, input);
        let r_out = call_bin2hex(bin2hex_lib::bin2hex, input);
        assert_eq!(
            c_out, r_out,
            "mismatch for input {:02x?}\n  C:    {:?}\n  Rust: {:?}",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}
