use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{FromRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::process::Command;

type Main = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

const STDOUT_FILENO: RawFd = 1;

struct StdoutRestore(RawFd);

impl Drop for StdoutRestore {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.0, STDOUT_FILENO);
            close(self.0);
        }
    }
}

fn run(command: &mut Command) {
    let output = command.output().expect("failed to start build command");
    assert!(
        output.status.success(),
        "command failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn build_c_oracle(crate_root: &Path) -> PathBuf {
    let source_root = crate_root.join("c_src");
    let build_dir = source_root.join("build");
    std::fs::create_dir_all(&build_dir).expect("failed to create C build directory");

    run(Command::new("cmake")
        .current_dir(&build_dir)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"]));
    run(Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", "."]));

    let library = build_dir.join("libdriver_c.so");
    run(Command::new("cc")
        .current_dir(&source_root)
        .args(["-fPIC", "-shared", "-o"])
        .arg(&library)
        .arg("src/main.c"));
    library
}

fn rust_library() -> PathBuf {
    let test_binary = env::current_exe().expect("failed to locate test binary");
    let deps_dir = test_binary
        .parent()
        .expect("unexpected Cargo target layout");
    let test_profile_library = deps_dir.join("libdriver.so");
    if test_profile_library.is_file() {
        return test_profile_library;
    }

    deps_dir
        .parent()
        .expect("unexpected Cargo target layout")
        .join("libdriver.so")
}

unsafe fn capture_stdout(function: Main) -> io::Result<(c_int, Vec<u8>)> {
    let mut pipe_fds = [0; 2];
    if pipe(pipe_fds.as_mut_ptr()) == -1 {
        return Err(io::Error::last_os_error());
    }

    let saved_stdout = dup(STDOUT_FILENO);
    if saved_stdout == -1 {
        close(pipe_fds[0]);
        close(pipe_fds[1]);
        return Err(io::Error::last_os_error());
    }
    let restore = StdoutRestore(saved_stdout);

    fflush(std::ptr::null_mut());
    if dup2(pipe_fds[1], STDOUT_FILENO) == -1 {
        close(pipe_fds[0]);
        close(pipe_fds[1]);
        return Err(io::Error::last_os_error());
    }
    close(pipe_fds[1]);

    let result = function();
    fflush(std::ptr::null_mut());
    dup2(restore.0, STDOUT_FILENO);

    let mut output = Vec::new();
    File::from_raw_fd(pipe_fds[0]).read_to_end(&mut output)?;
    Ok((result, output))
}

#[test]
fn main_matches_c_through_dynamic_exports() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = build_c_oracle(&crate_root);
    let rust_path = rust_library();
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    unsafe {
        let c_library = Library::new(&c_path).expect("failed to load C shared object");
        let rust_library = Library::new(&rust_path).expect("failed to load Rust shared object");
        let c_main: Symbol<Main> = c_library.get(b"main").expect("C main is not exported");
        let rust_main: Symbol<Main> = rust_library
            .get(b"main")
            .expect("Rust main is not exported");

        for trial in 0..64 {
            let c_result = capture_stdout(*c_main).expect("failed to capture C stdout");
            let rust_result = capture_stdout(*rust_main).expect("failed to capture Rust stdout");
            assert_eq!(
                rust_result, c_result,
                "differential mismatch on trial {trial}"
            );
        }
    }
}
