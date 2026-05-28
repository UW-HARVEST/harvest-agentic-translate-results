use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_uchar;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct ParseBuffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CJson {
    type_: c_int,
    valueint: c_int,
    valuedouble: f64,
}

type ParseNumberFn = unsafe extern "C" fn(*mut CJson, *mut ParseBuffer) -> c_int;

fn c_so_path() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("c_src/build/libdriver.so")
}

fn rust_so_path() -> std::path::PathBuf {
    // Cargo puts cdylib output in target/<profile>/libdriver.so
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Look for "debug" first; if not present, fall back to "release"
    let debug_path = manifest_dir.join("target/debug/libdriver.so");
    let release_path = manifest_dir.join("target/release/libdriver.so");
    if debug_path.exists() {
        debug_path
    } else {
        release_path
    }
}

struct LoadedLib {
    _lib: Library,
    parse_number: ParseNumberFn,
}

unsafe fn load(path: &std::path::Path) -> LoadedLib {
    let lib = Library::new(path).expect("failed to load library");
    let sym: Symbol<ParseNumberFn> = lib
        .get(b"parse_number\0")
        .expect("parse_number symbol not found");
    // Take the raw function pointer so we can keep the lib alive in the struct
    let f: ParseNumberFn = *sym.into_raw();
    LoadedLib {
        _lib: lib,
        parse_number: f,
    }
}

fn run_one(lib: &LoadedLib, input: &[u8], offset: usize) -> (c_int, CJson, usize) {
    let mut buf = ParseBuffer {
        content: input.as_ptr(),
        length: input.len(),
        offset,
        depth: 0,
    };
    let mut item = CJson {
        type_: -1,
        valueint: -1,
        valuedouble: 0.0,
    };
    let r = unsafe { (lib.parse_number)(&mut item, &mut buf) };
    (r, item, buf.offset)
}

fn assert_match(input: &[u8], offset: usize) {
    unsafe {
        let c = load(&c_so_path());
        let r = load(&rust_so_path());
        let (c_ret, c_item, c_off) = run_one(&c, input, offset);
        let (r_ret, r_item, r_off) = run_one(&r, input, offset);
        assert_eq!(
            c_ret, r_ret,
            "return code mismatch for input {:?} offset={}",
            input, offset
        );
        assert_eq!(c_off, r_off, "offset mismatch for input {:?}", input);
        // For successful parses, compare the entire item
        if c_ret != 0 {
            assert_eq!(
                c_item.type_, r_item.type_,
                "type mismatch for input {:?}",
                input
            );
            assert_eq!(
                c_item.valueint, r_item.valueint,
                "valueint mismatch for input {:?}",
                input
            );
            // Compare valuedouble as raw bits to catch NaN exactly
            assert_eq!(
                c_item.valuedouble.to_bits(),
                r_item.valuedouble.to_bits(),
                "valuedouble mismatch for input {:?}: c={} r={}",
                input,
                c_item.valuedouble,
                r_item.valuedouble
            );
        }
    }
}

#[test]
fn test_simple_integers() {
    for s in [
        "0", "1", "-1", "42", "-42", "100", "9999999", "12345678", "-12345678",
    ] {
        assert_match(s.as_bytes(), 0);
    }
}

#[test]
fn test_simple_doubles() {
    for s in [
        "0.0", "1.0", "-1.0", "3.14", "-3.14", "0.5", "1.5e10", "1.5E10", "1e-5", "1.23e+45",
        "-1.23e-45",
    ] {
        assert_match(s.as_bytes(), 0);
    }
}

#[test]
fn test_overflow_saturation() {
    for s in [
        "2147483648",         // INT_MAX + 1
        "-2147483649",        // INT_MIN - 1
        "1e100",
        "-1e100",
        "1e-100",
        "9999999999999999999",
    ] {
        assert_match(s.as_bytes(), 0);
    }
}

#[test]
fn test_with_trailing_content() {
    // The function should stop at a non-numeric char
    let inputs: &[&[u8]] = &[
        b"123,456",
        b"3.14}",
        b"42 trailing",
        b"-7]",
        b"1e10garbage",
    ];
    for input in inputs {
        assert_match(input, 0);
    }
}

#[test]
fn test_with_offset() {
    // Use offset so that the buffer points into the middle
    let cases: &[(&[u8], usize)] = &[
        (b"abc123", 3),
        (b"prefix-42suffix", 6),
        (b"...3.14...", 3),
        (b"xyz1e5xyz", 3),
    ];
    for (input, offset) in cases {
        assert_match(input, *offset);
    }
}

#[test]
fn test_invalid_inputs() {
    // Non-numeric leading chars => parse_number returns false
    let inputs: &[&[u8]] = &[b"abc", b"+", b"-", b".", b"e10", b"E5"];
    for input in inputs {
        assert_match(input, 0);
    }
}

#[test]
fn test_empty_at_end() {
    // offset == length: nothing to parse
    let s = b"123";
    assert_match(s, 3);
}

#[test]
fn test_unusual_inputs() {
    // The C code's loop accepts +, -, e, E, ., 0-9 in any order. strtod will
    // determine validity. Replicate weird inputs that the loop accepts but
    // strtod might reject.
    let inputs: &[&[u8]] = &[
        b"++",
        b"--",
        b"ee",
        b"EE",
        b"..",
        b"+-",
        b"-+",
        b"+1",
        b"+1.0",
        b"1.2.3",
        b"1e",
        b"1e+",
        b"1e-",
        b"1e2e3",
    ];
    for input in inputs {
        assert_match(input, 0);
    }
}
