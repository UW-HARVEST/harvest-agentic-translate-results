use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by calling `driver(c)` through the given library.
fn capture_driver(lib: &Library, c: c_char) -> String {
    unsafe {
        // Create a pipe: pipe_fds[0]=read, pipe_fds[1]=write
        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);

        // Save original stdout and redirect stdout to pipe write end
        let saved = libc::dup(1);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        // Call driver
        let func: Symbol<unsafe extern "C" fn(c_char)> = lib.get(b"driver").unwrap();
        func(c);

        // Flush stdout so all printf output goes through
        libc::fflush(std::ptr::null_mut());

        // Restore stdout
        libc::dup2(saved, 1);
        libc::close(saved);

        // Read captured output
        let mut f = std::fs::File::from_raw_fd(pipe_fds[0]);
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        buf
    }
}

#[test]
fn test_driver_matches() {
    let c_lib = unsafe {
        Library::new(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so")).unwrap()
    };
    let rust_lib = unsafe {
        // Find the Rust .so in target directory
        let manifest = env!("CARGO_MANIFEST_DIR");
        Library::new(format!("{}/target/debug/libdriver.so", manifest)).unwrap()
    };

    // Test a representative set of characters covering all ctype categories
    let test_chars: Vec<c_char> = vec![
        b'A' as c_char,  // uppercase alpha
        b'z' as c_char,  // lowercase alpha
        b'5' as c_char,  // digit
        b' ' as c_char,  // space
        b'!' as c_char,  // punctuation
        b'\t' as c_char, // blank/control
        b'\n' as c_char, // control/space
        0 as c_char,     // null / control
        b'f' as c_char,  // hex digit
        b'~' as c_char,  // printable
        127 as c_char,   // DEL / control
    ];

    for &c in &test_chars {
        let c_out = capture_driver(&c_lib, c);
        let rust_out = capture_driver(&rust_lib, c);
        assert_eq!(
            c_out, rust_out,
            "Mismatch for char value {}: C output:\n{}\nRust output:\n{}",
            c as i32, c_out, rust_out
        );
    }
}
