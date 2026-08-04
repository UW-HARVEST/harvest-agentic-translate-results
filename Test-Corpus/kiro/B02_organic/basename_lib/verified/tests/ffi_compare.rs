use libloading::{Library, Symbol};
use std::os::raw::c_char;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/libdriver.so");

fn rust_lib_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    // Find the built Rust .so in target/debug/
    let p = format!("{dir}/target/debug/libdriver.so");
    if std::path::Path::new(&p).exists() {
        return p;
    }
    panic!("Rust .so not found at {p}");
}

type ToolBasenameFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

fn compare_tool_basename(input: &[u8]) {
    unsafe {
        let c_lib = Library::new(C_LIB).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<ToolBasenameFn> = c_lib.get(b"tool_basename").unwrap();
        let r_fn: Symbol<ToolBasenameFn> = r_lib.get(b"tool_basename").unwrap();

        // Create two mutable copies so each lib gets its own buffer
        let mut c_buf = input.to_vec();
        let mut r_buf = input.to_vec();

        let c_ptr = c_buf.as_mut_ptr() as *mut c_char;
        let r_ptr = r_buf.as_mut_ptr() as *mut c_char;

        let c_res = c_fn(c_ptr);
        let r_res = r_fn(r_ptr);

        // Compare as offset from start of buffer
        let c_off = c_res.offset_from(c_ptr);
        let r_off = r_res.offset_from(r_ptr);

        assert_eq!(
            c_off, r_off,
            "tool_basename mismatch for input {:?}: C offset={c_off}, Rust offset={r_off}",
            std::str::from_utf8(input).unwrap_or("<non-utf8>")
        );
    }
}

#[test]
fn test_tool_basename() {
    let cases: &[&[u8]] = &[
        b"hello\0",
        b"/usr/bin/tool\0",
        b"C:\\Windows\\file.txt\0",
        b"/mixed\\path/to\\file\0",
        b"\\back\\slash\0",
        b"/forward/slash\0",
        b"nodelim\0",
        b"\0",
        b"/\0",
        b"\\\0",
        b"a/b\\c\0",
        b"a\\b/c\0",
        b"//double//slash\0",
        b"\\\\double\\\\back\0",
        b"trailing/\0",
        b"trailing\\\0",
    ];
    for case in cases {
        compare_tool_basename(case);
    }
}
