use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::os::raw::c_int;
use std::path::PathBuf;

type Hex2BinFn = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *const *const c_char,
) -> c_int;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libhex2bin_lib.so")
}

fn call_hex2bin(
    f: Hex2BinFn,
    hex: &[u8],
    bin_maxlen: usize,
    ignore: Option<&[u8]>,
    want_end_p: bool,
) -> (c_int, Vec<u8>, Option<usize>) {
    let mut bin = vec![0u8; bin_maxlen];
    let mut hex_end: *const c_char = std::ptr::null();
    let ignore_ptr = match ignore {
        Some(s) => s.as_ptr() as *const c_char,
        None => std::ptr::null(),
    };
    let hex_end_pp = if want_end_p {
        &mut hex_end as *mut *const c_char as *const *const c_char
    } else {
        std::ptr::null()
    };
    let ret = unsafe {
        f(
            bin.as_mut_ptr(),
            bin_maxlen,
            hex.as_ptr() as *const c_char,
            hex.len(),
            ignore_ptr,
            hex_end_pp,
        )
    };
    let end_offset = if want_end_p && !hex_end.is_null() {
        Some(unsafe { hex_end.offset_from(hex.as_ptr() as *const c_char) } as usize)
    } else {
        None
    };
    (ret, bin, end_offset)
}

struct TestCase {
    name: &'static str,
    hex: &'static [u8],
    bin_maxlen: usize,
    ignore: Option<&'static [u8]>,
    want_end_p: bool,
}

fn test_cases() -> Vec<TestCase> {
    vec![
        TestCase { name: "basic_deadbeef", hex: b"DeadBeef", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "lowercase", hex: b"0123456789abcdef", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "uppercase", hex: b"0123456789ABCDEF", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "empty", hex: b"", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "odd_length_end_p", hex: b"abc", bin_maxlen: 64, ignore: None, want_end_p: true },
        TestCase { name: "odd_length_no_end_p", hex: b"abc", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "invalid_char_end_p", hex: b"abXX", bin_maxlen: 64, ignore: None, want_end_p: true },
        TestCase { name: "invalid_char_no_end_p", hex: b"abXX", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "ignore_colon", hex: b"de:ad:be:ef", bin_maxlen: 64, ignore: Some(b":\0"), want_end_p: false },
        TestCase { name: "ignore_space", hex: b"de ad be ef", bin_maxlen: 64, ignore: Some(b" \0"), want_end_p: false },
        TestCase { name: "buffer_too_small", hex: b"deadbeef", bin_maxlen: 1, ignore: None, want_end_p: true },
        TestCase { name: "exact_buffer", hex: b"deadbeef", bin_maxlen: 4, ignore: None, want_end_p: false },
        TestCase { name: "all_zeros", hex: b"0000000000", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "all_ff", hex: b"FFFFFFFFFF", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "end_p_full_parse", hex: b"aabb", bin_maxlen: 64, ignore: None, want_end_p: true },
        TestCase { name: "ignore_not_matched", hex: b"aabb", bin_maxlen: 64, ignore: Some(b":\0"), want_end_p: false },
        TestCase { name: "single_byte", hex: b"ff", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "mixed_case", hex: b"aAbBcCdDeEfF", bin_maxlen: 64, ignore: None, want_end_p: false },
        TestCase { name: "ignore_mid_pair_invalid", hex: b"a:b", bin_maxlen: 64, ignore: Some(b":\0"), want_end_p: true },
        TestCase { name: "zero_maxlen", hex: b"aa", bin_maxlen: 0, ignore: None, want_end_p: true },
    ]
}

#[test]
fn test_hex2bin_c_vs_rust() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    let c_fn: Symbol<Hex2BinFn> = unsafe { c_lib.get(b"hex2bin").expect("hex2bin not found in C lib") };

    let rust_fn: Hex2BinFn = hex2bin_lib::hex2bin;

    for tc in test_cases() {
        let (c_ret, c_bin, c_end) = call_hex2bin(*c_fn, tc.hex, tc.bin_maxlen, tc.ignore, tc.want_end_p);
        let (r_ret, r_bin, r_end) = call_hex2bin(rust_fn, tc.hex, tc.bin_maxlen, tc.ignore, tc.want_end_p);

        assert_eq!(c_ret, r_ret, "[{}] return value mismatch: C={} Rust={}", tc.name, c_ret, r_ret);
        // Compare bin contents up to the returned length (if positive)
        let len = if c_ret > 0 { c_ret as usize } else { tc.bin_maxlen };
        assert_eq!(&c_bin[..len], &r_bin[..len], "[{}] bin output mismatch", tc.name);
        assert_eq!(c_end, r_end, "[{}] hex_end_p offset mismatch: C={:?} Rust={:?}", tc.name, c_end, r_end);
    }
}
