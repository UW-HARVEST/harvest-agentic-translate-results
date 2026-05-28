use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::path::PathBuf;

type Hex2BinFn = unsafe extern "C" fn(
    bin: *mut u8,
    bin_maxlen: usize,
    hex: *const c_char,
    hex_len: usize,
    ignore: *const c_char,
    hex_end_p: *mut *const c_char,
) -> c_int;

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try release first, fallback to debug
    p.push("target/release/libhex2bin_lib.so");
    if p.exists() {
        return p;
    }
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libhex2bin_lib.so");
    p
}

unsafe fn load_hex2bin(lib: &Library) -> Symbol<Hex2BinFn> {
    lib.get(b"hex2bin\0").expect("symbol hex2bin not found")
}

#[derive(Debug, Clone)]
struct CallResult {
    ret: c_int,
    bin: Vec<u8>,
    hex_end_offset: Option<isize>,
}

unsafe fn call_one(
    f: &Symbol<Hex2BinFn>,
    bin_maxlen: usize,
    hex: &[u8],
    hex_len: usize,
    ignore: Option<&[u8]>,
    use_hex_end: bool,
) -> CallResult {
    // Allocate output bin (with sentinel padding to detect overruns).
    let mut bin = vec![0u8; bin_maxlen.max(1) + 16];
    let bin_ptr: *mut u8 = if bin_maxlen == 0 {
        // Still pass a valid pointer; the C code does not access it when
        // bin_maxlen == 0 (it returns -1 once bin_pos >= bin_maxlen).
        bin.as_mut_ptr()
    } else {
        bin.as_mut_ptr()
    };

    let hex_ptr = hex.as_ptr() as *const c_char;
    let ignore_ptr: *const c_char = match ignore {
        Some(s) => s.as_ptr() as *const c_char,
        None => std::ptr::null(),
    };

    let mut hex_end: *const c_char = std::ptr::null();
    let hex_end_p: *mut *const c_char = if use_hex_end {
        &mut hex_end as *mut *const c_char
    } else {
        std::ptr::null_mut()
    };

    let ret = f(bin_ptr, bin_maxlen, hex_ptr, hex_len, ignore_ptr, hex_end_p);

    let hex_end_offset = if use_hex_end {
        Some(unsafe { hex_end.offset_from(hex_ptr) })
    } else {
        None
    };

    // Truncate bin to bin_maxlen for comparison.
    bin.truncate(bin_maxlen);
    CallResult {
        ret,
        bin,
        hex_end_offset,
    }
}

fn assert_results_match(c: &CallResult, r: &CallResult, ctx: &str) {
    assert_eq!(c.ret, r.ret, "ret mismatch for {ctx}");
    assert_eq!(
        c.hex_end_offset, r.hex_end_offset,
        "hex_end mismatch for {ctx}"
    );
    // Even if ret < 0, we still compare bin contents (C resets bin_pos on
    // error but does not zero the buffer; nonetheless, we just compare what
    // both produced, which should match).
    assert_eq!(c.bin, r.bin, "bin mismatch for {ctx}");
}

#[test]
fn test_hex2bin_compare_basic() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let cf = load_hex2bin(&c_lib);
        let rf = load_hex2bin(&r_lib);

        let cases: Vec<(&str, &[u8], usize, Option<&[u8]>, bool)> = vec![
            ("empty", b"", 0, None, true),
            ("empty no end", b"", 0, None, false),
            ("simple", b"deadbeef\0", 8, None, true),
            ("simple no end", b"deadbeef\0", 8, None, false),
            ("upper", b"DEADBEEF\0", 8, None, true),
            ("mixed", b"DeAdBeEf\0", 8, None, true),
            ("with ignore spaces", b"de ad be ef\0", 11, Some(b" \0"), true),
            ("ignore not used (no end)", b"de ad be ef\0", 11, Some(b" \0"), false),
            ("odd length", b"abc\0", 3, None, true),
            ("odd length no end", b"abc\0", 3, None, false),
            ("invalid char", b"abxx\0", 4, None, true),
            ("invalid char no end", b"abxx\0", 4, None, false),
            ("ignore but in mid-byte", b"a bc\0", 4, Some(b" \0"), true),
            ("zero len input", b"deadbeef\0", 0, None, true),
            ("partial parse", b"abcd!!", 6, None, true),
            ("only ignored", b"   \0", 3, Some(b" \0"), true),
            ("0x80 as char", b"a\x80", 2, None, true),
            ("non-hex first", b"!!", 2, None, true),
            ("non-hex first no end", b"!!", 2, None, false),
            ("ignore matches but state=1", b"a b\0", 3, Some(b" \0"), true),
            ("hex case A-F", b"0123456789abcdefABCDEF\0", 22, None, true),
        ];

        for (name, hex, hex_len, ignore, use_end) in cases {
            // Test with a few different bin_maxlen values.
            for &bin_max in &[0usize, 1, 2, 4, 8, 16, 32, 256] {
                let c_res = call_one(&cf, bin_max, hex, hex_len, ignore, use_end);
                let r_res = call_one(&rf, bin_max, hex, hex_len, ignore, use_end);
                let ctx = format!("{} (bin_max={})", name, bin_max);
                assert_results_match(&c_res, &r_res, &ctx);
            }
        }
    }
}

#[test]
fn test_hex2bin_compare_all_bytes_as_first_char() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let cf = load_hex2bin(&c_lib);
        let rf = load_hex2bin(&r_lib);

        // Try every possible byte as the first character (followed by '0').
        for b in 0u8..=255 {
            let buf = [b, b'0', 0u8];
            for use_end in [true, false] {
                let c_res = call_one(&cf, 16, &buf, 2, None, use_end);
                let r_res = call_one(&rf, 16, &buf, 2, None, use_end);
                let ctx = format!("byte=0x{:02x} use_end={}", b, use_end);
                assert_results_match(&c_res, &r_res, &ctx);
            }
        }
    }
}

#[test]
fn test_hex2bin_compare_random_inputs() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let cf = load_hex2bin(&c_lib);
        let rf = load_hex2bin(&r_lib);

        // Generate deterministic pseudo-random inputs.
        let mut state: u64 = 0xdeadbeefcafebabe;
        let next = |s: &mut u64| -> u8 {
            *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (*s >> 33) as u8
        };

        for trial in 0..200 {
            let len = (next(&mut state) as usize) % 64;
            let mut buf: Vec<u8> = (0..len).map(|_| next(&mut state)).collect();
            buf.push(0);
            let bin_max = (next(&mut state) as usize) % 40;
            let use_end = (next(&mut state) & 1) != 0;
            let use_ignore = (next(&mut state) & 1) != 0;
            let ignore: Option<&[u8]> = if use_ignore {
                Some(b" \t\n\0")
            } else {
                None
            };

            let c_res = call_one(&cf, bin_max, &buf, len, ignore, use_end);
            let r_res = call_one(&rf, bin_max, &buf, len, ignore, use_end);
            let ctx = format!("trial={} len={} bin_max={}", trial, len, bin_max);
            assert_results_match(&c_res, &r_res, &ctx);
        }
    }
}

#[test]
fn test_hex2bin_compare_ignore_edge_cases() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let cf = load_hex2bin(&c_lib);
        let rf = load_hex2bin(&r_lib);

        // Ignore string with various special chars
        let ignores: &[&[u8]] = &[
            b"\0",         // empty ignore
            b" \0",
            b" \t\n:\0",
            b"abc\0",      // ignore set includes hex chars
            b"-_\0",
        ];
        let inputs: &[&[u8]] = &[
            b"",
            b"a",
            b"ab",
            b"a:b:c:d",
            b" a b ",
            b"a b\0",
            b":::",
            b"ab-cd_ef",
        ];

        for ig in ignores {
            for inp in inputs {
                let len = inp.len();
                let mut padded = inp.to_vec();
                padded.push(0);
                for use_end in [true, false] {
                    for &bin_max in &[0usize, 4, 16, 64] {
                        let c_res = call_one(&cf, bin_max, &padded, len, Some(ig), use_end);
                        let r_res = call_one(&rf, bin_max, &padded, len, Some(ig), use_end);
                        let ctx = format!(
                            "ig={:?} inp={:?} bin_max={} use_end={}",
                            ig, inp, bin_max, use_end
                        );
                        assert_results_match(&c_res, &r_res, &ctx);
                    }
                }
            }
        }
    }
}
