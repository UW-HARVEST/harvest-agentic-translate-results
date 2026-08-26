use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libStaticLoop.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts the cdylib next to deps
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    dir.join("libStaticLoop.so")
}

/// Helper: capture stdout produced by a closure (works for printf via libc).
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush before dup
    }

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1); }

    f();

    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);
        libc::close(pipe_fds[1]);
    }

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

#[test]
fn test_static_sum() {
    // Load each library fresh — each has its own static state starting at 0.
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"static_sum").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r_lib.get(b"static_sum").unwrap();

        // Test a sequence of calls — the static accumulator should match.
        let inputs = [1, 2, 3, -5, 0, 100, -100, 42];
        for &val in &inputs {
            let c_res = c_fn(val);
            let r_res = r_fn(val);
            assert_eq!(c_res, r_res, "static_sum({val}): C={c_res}, Rust={r_res}");
        }
    }
}

#[test]
fn test_driver() {
    unsafe {
        // Fresh libraries so static state starts at 0.
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int)> =
            c_lib.get(b"driver").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int)> =
            r_lib.get(b"driver").unwrap();

        for &stride in &[1, 0, -3, 7] {
            // Each call accumulates onto the previous state — that's fine as
            // long as both libraries see the same sequence.
            let c_out = capture_stdout(|| c_fn(stride));
            let r_out = capture_stdout(|| r_fn(stride));
            assert_eq!(
                c_out, r_out,
                "driver({stride}) output differs:\nC:\n{c_out}\nRust:\n{r_out}"
            );
        }
    }
}
