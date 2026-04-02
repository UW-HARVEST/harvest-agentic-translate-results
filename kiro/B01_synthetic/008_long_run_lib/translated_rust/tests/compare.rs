use libloading::{Library, Symbol};
use std::ffi::c_uint;
use std::io::Read;
use std::os::unix::io::FromRawFd;

/// Capture stdout from a function that prints via C printf.
/// Forks a child process to avoid test harness interference.
fn capture_stdout_via_fork<F: FnOnce()>(f: F) -> String {
    unsafe {
        let mut pipefd = [0i32; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            // Child: redirect stdout to pipe write end, run f, exit
            libc::close(pipefd[0]);
            libc::dup2(pipefd[1], 1);
            libc::close(pipefd[1]);
            f();
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }

        // Parent: read from pipe
        libc::close(pipefd[1]);
        let mut file = std::fs::File::from_raw_fd(pipefd[0]);
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap_or(0);

        let mut status = 0;
        libc::waitpid(pid, &mut status, 0);
        s
    }
}

#[test]
fn test_long_exec_matches() {
    let c_lib_path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/liblong.so");
    let c_lib = unsafe { Library::new(c_lib_path).expect("Failed to load C .so") };
    let c_long_exec: Symbol<unsafe extern "C" fn(c_uint)> =
        unsafe { c_lib.get(b"long_exec").expect("Failed to find long_exec in C .so") };

    let seed: c_uint = 42;

    let c_output = capture_stdout_via_fork(|| unsafe { c_long_exec(seed) });
    let rust_output = capture_stdout_via_fork(|| long::long_exec(seed));

    eprintln!("C output:    {:?}", c_output);
    eprintln!("Rust output: {:?}", rust_output);

    assert_eq!(
        c_output, rust_output,
        "Mismatch!\nC output:    {:?}\nRust output: {:?}",
        c_output, rust_output
    );
}
