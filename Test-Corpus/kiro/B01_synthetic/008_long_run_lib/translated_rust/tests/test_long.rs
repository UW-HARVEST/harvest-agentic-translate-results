use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout produced by calling `f()` via a pipe + dup2 trick.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let mut pipes = [0i32; 2];
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush any pending stdout
        assert_eq!(libc::pipe(pipes.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        assert!(saved >= 0);
        libc::dup2(pipes[1], 1);
        libc::close(pipes[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let mut file = std::fs::File::from_raw_fd(pipes[0]);
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf
    }
}

#[test]
fn test_long_exec_seed_42() {
    let c_lib = unsafe {
        Library::new("/tmp/harvest-work-XWsuiA/translated_rust/c_src/build/liblong.so")
    }
    .expect("Failed to load C .so");

    // Find the Rust .so - cargo puts it in target/release or target/debug
    let rust_so = if std::path::Path::new("target/release/liblong.so").exists() {
        "target/release/liblong.so"
    } else {
        "target/debug/liblong.so"
    };
    let rust_lib =
        unsafe { Library::new(rust_so) }.expect("Failed to load Rust .so");

    let seed: u32 = 42;

    let c_out = {
        let f: Symbol<unsafe extern "C" fn(u32)> =
            unsafe { c_lib.get(b"long_exec") }.expect("C long_exec not found");
        capture_stdout(|| unsafe { f(seed) })
    };

    let rust_out = {
        let f: Symbol<unsafe extern "C" fn(u32)> =
            unsafe { rust_lib.get(b"long_exec") }.expect("Rust long_exec not found");
        capture_stdout(|| unsafe { f(seed) })
    };

    // Strip any test-harness noise (e.g. "has been running for over 60 seconds")
    // and compare only lines that look like program output.
    let extract = |s: &str| -> String {
        s.lines()
            .filter(|l| !l.contains("has been running"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let c_clean = extract(&c_out);
    let rust_clean = extract(&rust_out);
    assert_eq!(
        c_clean, rust_clean,
        "Output mismatch for seed {seed}:\n  C:    {c_clean:?}\n  Rust: {rust_clean:?}"
    );
}
