use libloading::{Library, Symbol};
use md5_digest_lib::{md5_digest, tflac_md5};

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libmd5_digest_lib.so")
}

fn call_c_md5_digest(lib: &Library, m: &tflac_md5) -> [u8; 16] {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(*const tflac_md5, *mut u8)> =
            lib.get(b"md5_digest").expect("md5_digest not found in C lib");
        let mut out = [0u8; 16];
        func(m as *const tflac_md5, out.as_mut_ptr());
        out
    }
}

fn call_rust_md5_digest(m: &tflac_md5) -> [u8; 16] {
    let mut out = [0u8; 16];
    unsafe { md5_digest(m as *const tflac_md5, out.as_mut_ptr()) };
    out
}

#[test]
fn test_md5_digest_zeros() {
    let lib = unsafe { Library::new(c_lib_path()) }.expect("Failed to load C lib");
    let m = tflac_md5 { a: 0, b: 0, c: 0, d: 0 };
    assert_eq!(call_c_md5_digest(&lib, &m), call_rust_md5_digest(&m));
}

#[test]
fn test_md5_digest_known_values() {
    let lib = unsafe { Library::new(c_lib_path()) }.expect("Failed to load C lib");
    let cases = [
        tflac_md5 { a: 0x01020304, b: 0x05060708, c: 0x090a0b0c, d: 0x0d0e0f10 },
        tflac_md5 { a: 0xFFFFFFFF, b: 0x00000001, c: 0x80000000, d: 0xDEADBEEF },
        tflac_md5 { a: 1, b: 2, c: 3, d: 4 },
        tflac_md5 { a: u32::MAX, b: u32::MAX, c: u32::MAX, d: u32::MAX },
    ];
    for m in &cases {
        let c_out = call_c_md5_digest(&lib, m);
        let r_out = call_rust_md5_digest(m);
        assert_eq!(c_out, r_out, "Mismatch for a={:#x} b={:#x} c={:#x} d={:#x}", m.a, m.b, m.c, m.d);
    }
}
