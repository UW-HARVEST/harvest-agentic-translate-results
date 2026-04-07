use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::FromRawFd;

type MainFn = unsafe extern "C" fn(c_int, *const *const c_char) -> c_int;

fn project_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn call_main(lib: &Library, args: &[&str]) -> c_int {
    unsafe {
        let func: Symbol<MainFn> = lib.get(b"main").expect("no main symbol");
        let c_args: Vec<CString> = args.iter().map(|s| CString::new(*s).unwrap()).collect();
        let ptrs: Vec<*const c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
        func(ptrs.len() as c_int, ptrs.as_ptr())
    }
}

/// Capture stdout from calling main via a pipe+fork so we get byte-identical output comparison.
fn call_main_capture(lib_path: &str, args: &[&str]) -> (c_int, Vec<u8>) {
    use std::io::Read;

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }

    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // child: redirect stdout to pipe write end
        unsafe {
            libc::close(pipe_fds[0]);
            libc::dup2(pipe_fds[1], 1);
            libc::close(pipe_fds[1]);
        }
        let lib = unsafe { Library::new(lib_path).unwrap() };
        let code = call_main(&lib, args);
        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::_exit(code);
        }
    }
    // parent: read from pipe read end
    unsafe { libc::close(pipe_fds[1]); }
    let mut output = Vec::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    file.read_to_end(&mut output).unwrap();

    let mut status = 0i32;
    unsafe { libc::waitpid(pid, &mut status, 0); }
    let exit_code = if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) } else { -1 };
    (exit_code, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c_lib_path() -> String {
        project_root().join("c_src/build_so/libnineality.so").to_str().unwrap().to_string()
    }

    fn rust_lib_path() -> String {
        project_root().join("target/debug/libdriver.so").to_str().unwrap().to_string()
    }

    fn compare(args: &[&str]) {
        let (c_code, c_out) = call_main_capture(&c_lib_path(), args);
        let (r_code, r_out) = call_main_capture(&rust_lib_path(), args);
        assert_eq!(c_code, r_code, "exit codes differ for args {:?}: C={}, Rust={}", args, c_code, r_code);
        assert_eq!(c_out, r_out, "stdout differs for args {:?}:\nC:    {:?}\nRust: {:?}",
            args, String::from_utf8_lossy(&c_out), String::from_utf8_lossy(&r_out));
    }

    #[test]
    fn test_no_args() {
        compare(&["driver"]);
    }

    #[test]
    fn test_too_many_args() {
        compare(&["driver", "1", "2"]);
    }

    #[test]
    fn test_non_integer() {
        compare(&["driver", "abc"]);
    }

    #[test]
    fn test_start_at_0() {
        compare(&["driver", "0"]);
    }

    #[test]
    fn test_start_at_9() {
        compare(&["driver", "9"]);
    }

    #[test]
    fn test_start_at_5() {
        compare(&["driver", "5"]);
    }

    #[test]
    fn test_start_at_19() {
        compare(&["driver", "19"]);
    }

    #[test]
    fn test_start_at_20() {
        compare(&["driver", "20"]);
    }

    #[test]
    fn test_negative() {
        compare(&["driver", "-1"]);
    }

    #[test]
    fn test_negative_ending_9() {
        compare(&["driver", "-11"]);
    }

    #[test]
    fn test_large_number() {
        compare(&["driver", "100"]);
    }

    #[test]
    fn test_trailing_chars() {
        // C strtol parses "42abc" as 42 — Rust must match
        compare(&["driver", "42abc"]);
    }

    #[test]
    fn test_whitespace_prefix() {
        compare(&["driver", "  7"]);
    }

    #[test]
    fn test_plus_sign() {
        compare(&["driver", "+3"]);
    }
}
