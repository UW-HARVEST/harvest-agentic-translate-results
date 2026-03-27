use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout from a closure by redirecting fd 1 to a pipe.
fn capture_stdout(f: impl FnOnce()) -> String {
    // flush before redirecting
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);

    let old_stdout = unsafe { libc::dup(1) };
    assert!(old_stdout >= 0);
    unsafe { libc::dup2(fds[1], 1) };
    unsafe { libc::close(fds[1]) };

    f();

    // flush C and Rust stdout
    unsafe { libc::fflush(std::ptr::null_mut()) };
    use std::io::Write;
    let _ = std::io::stdout().flush();

    // restore
    unsafe { libc::dup2(old_stdout, 1) };
    unsafe { libc::close(old_stdout) };

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("c_src/build/libSieve.so")
}

#[test]
fn test_sieve_matches() {
    let c_lib = unsafe { libloading::Library::new(c_lib_path()).expect("load C lib") };
    let c_sieve: libloading::Symbol<unsafe extern "C" fn(i32)> =
        unsafe { c_lib.get(b"sieve").expect("find sieve in C lib") };

    // Test several starting values
    for &start in &[0, 1, 5, 9, 10, 15, 19, 20, 99, 100, -1, -10] {
        let c_out = capture_stdout(|| unsafe { c_sieve(start) });
        let rust_out = capture_stdout(|| Sieve::sieve(start));
        assert_eq!(
            c_out, rust_out,
            "Output mismatch for sieve({})\nC:    {:?}\nRust: {:?}",
            start, c_out, rust_out
        );
    }
}
