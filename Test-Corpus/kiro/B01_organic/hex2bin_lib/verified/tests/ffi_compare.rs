use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_char;
use std::ptr;

type Hex2BinFn = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut *const c_char,
) -> c_int;

struct Libs {
    _c_lib: Library,
    _rs_lib: Library,
    c_fn: Hex2BinFn,
    rs_fn: Hex2BinFn,
}

impl Libs {
    fn load() -> Self {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let c_path = format!("{}/c_src/build/libtranslated_rust.so", manifest);
        let rs_path = format!("{}/target/debug/libhex2bin_lib.so", manifest);
        unsafe {
            let c_lib = Library::new(&c_path).expect("load C .so");
            let rs_lib = Library::new(&rs_path).expect("load Rust .so");
            let c_fn: Symbol<Hex2BinFn> = c_lib.get(b"hex2bin").expect("C hex2bin");
            let rs_fn: Symbol<Hex2BinFn> = rs_lib.get(b"hex2bin").expect("Rust hex2bin");
            let c_fn = *c_fn;
            let rs_fn = *rs_fn;
            Libs { _c_lib: c_lib, _rs_lib: rs_lib, c_fn, rs_fn }
        }
    }
}

/// Call hex2bin on both libs and assert identical results.
/// When `hex_end_p` is used, we compare the offset from `hex` start.
fn compare(
    libs: &Libs,
    hex: &[u8],
    bin_maxlen: usize,
    ignore: Option<&[u8]>,  // null-terminated
    use_hex_end: bool,
) {
    let mut c_bin = vec![0xFFu8; bin_maxlen];
    let mut rs_bin = vec![0xFFu8; bin_maxlen];
    let mut c_end: *const c_char = ptr::null();
    let mut rs_end: *const c_char = ptr::null();

    let hex_ptr = hex.as_ptr() as *const c_char;
    let hex_len = hex.len();
    let ign_ptr = ignore.map_or(ptr::null(), |s| s.as_ptr() as *const c_char);
    let c_end_pp = if use_hex_end { &mut c_end as *mut _ } else { ptr::null_mut() };
    let rs_end_pp = if use_hex_end { &mut rs_end as *mut _ } else { ptr::null_mut() };

    let c_ret = unsafe { (libs.c_fn)(c_bin.as_mut_ptr(), bin_maxlen, hex_ptr, hex_len, ign_ptr, c_end_pp) };
    let rs_ret = unsafe { (libs.rs_fn)(rs_bin.as_mut_ptr(), bin_maxlen, hex_ptr, hex_len, ign_ptr, rs_end_pp) };

    assert_eq!(c_ret, rs_ret, "return mismatch for hex={:?}", std::str::from_utf8(hex));
    assert_eq!(c_bin, rs_bin, "bin mismatch for hex={:?}", std::str::from_utf8(hex));

    if use_hex_end {
        let c_off = unsafe { c_end.offset_from(hex_ptr) };
        let rs_off = unsafe { rs_end.offset_from(hex_ptr) };
        assert_eq!(c_off, rs_off, "hex_end offset mismatch for hex={:?}", std::str::from_utf8(hex));
    }
}

#[test]
fn test_basic_valid_hex() {
    let libs = Libs::load();
    compare(&libs, b"48656c6c6f", 16, None, false);
    compare(&libs, b"48656C6C6F", 16, None, false);  // uppercase
    compare(&libs, b"00ff00ff", 16, None, false);
    compare(&libs, b"", 16, None, false);  // empty
}

#[test]
fn test_with_hex_end() {
    let libs = Libs::load();
    compare(&libs, b"48656c6c6f", 16, None, true);
    compare(&libs, b"4865zz", 16, None, true);  // stops at 'z'
    compare(&libs, b"486", 16, None, true);  // odd length
}

#[test]
fn test_ignore_chars() {
    let libs = Libs::load();
    compare(&libs, b"48:65:6c:6c:6f", 16, Some(b":\0"), true);
    compare(&libs, b"48 65 6c 6c 6f", 16, Some(b" \0"), true);
    compare(&libs, b"48-65-6c", 16, Some(b"-\0"), true);
    // ignore only works between pairs (state==0)
    compare(&libs, b"4:8", 16, Some(b":\0"), true);
}

#[test]
fn test_buffer_overflow() {
    let libs = Libs::load();
    compare(&libs, b"48656c6c6f", 2, None, true);  // 5 bytes needed, only 2 available
    compare(&libs, b"48656c6c6f", 0, None, true);
}

#[test]
fn test_invalid_chars() {
    let libs = Libs::load();
    compare(&libs, b"zz", 16, None, true);
    compare(&libs, b"4g", 16, None, true);
    compare(&libs, b"48656c6c6fXX", 16, None, true);
}

#[test]
fn test_odd_length_no_hex_end() {
    let libs = Libs::load();
    // odd number of valid hex chars, no hex_end_p → should return -1
    compare(&libs, b"486", 16, None, false);
}

#[test]
fn test_all_byte_values() {
    let libs = Libs::load();
    // encode 00..ff
    let mut hex = Vec::with_capacity(512);
    for b in 0..=255u8 {
        hex.push(b"0123456789abcdef"[(b >> 4) as usize]);
        hex.push(b"0123456789abcdef"[(b & 0xf) as usize]);
    }
    compare(&libs, &hex, 256, None, true);
}

#[test]
fn test_mixed_case() {
    let libs = Libs::load();
    compare(&libs, b"aAbBcCdDeEfF", 16, None, true);
}

#[test]
fn test_single_byte() {
    let libs = Libs::load();
    compare(&libs, b"ff", 1, None, true);
    compare(&libs, b"00", 1, None, true);
}

#[test]
fn test_ignore_at_boundary() {
    let libs = Libs::load();
    // ignore char at start
    compare(&libs, b":4865", 16, Some(b":\0"), true);
    // ignore char at end (after complete pair)
    compare(&libs, b"4865:", 16, Some(b":\0"), true);
    // multiple ignore chars in a row
    compare(&libs, b"::48::65::", 16, Some(b":\0"), true);
}
