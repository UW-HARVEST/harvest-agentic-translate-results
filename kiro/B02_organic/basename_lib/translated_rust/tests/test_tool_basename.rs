use libloading::{Library, Symbol};

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("Failed to load C libdriver.so") }
}

fn rust_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");
    unsafe { Library::new(path).expect("Failed to load Rust libdriver.so") }
}

fn call_lib(lib: &Library, input: &[u8]) -> Vec<u8> {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(*mut i8) -> *mut i8> =
            lib.get(b"tool_basename").unwrap();
        let mut buf = input.to_vec();
        let ptr = buf.as_mut_ptr() as *mut i8;
        let result = func(ptr);
        let len = libc::strlen(result as *const _);
        std::slice::from_raw_parts(result as *const u8, len).to_vec()
    }
}

#[test]
fn test_tool_basename_matches_c() {
    let c = c_lib();
    let r = rust_lib();
    let cases: &[&[u8]] = &[
        b"hello\0",
        b"/usr/bin/test\0",
        b"C:\\Windows\\file.txt\0",
        b"/usr/local\\mixed/path\\end\0",
        b"nopath\0",
        b"/\0",
        b"\\\0",
        b"/a\0",
        b"\\a\0",
        b"a/b\\c\0",
        b"a\\b/c\0",
        b"/a/b/c\0",
        b"\\a\\b\\c\0",
        b"/a\\b\0",
        b"\\a/b\0",
        b"just_a_file\0",
        b"/trailing/\0",
        b"\\trailing\\\0",
        b"a\0",
        b"\0",
    ];

    for input in cases {
        let c_out = call_lib(&c, input);
        let r_out = call_lib(&r, input);
        assert_eq!(
            c_out, r_out,
            "Mismatch for input {:?}: C={:?} Rust={:?}",
            String::from_utf8_lossy(&input[..input.len() - 1]),
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}
