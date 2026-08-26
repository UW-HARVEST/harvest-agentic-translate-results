use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn find_rust_library() -> PathBuf {
    let executable = std::env::current_exe().expect("get integration test path");
    let deps_dir = executable.parent().expect("integration test parent");
    let profile_dir = deps_dir.parent().expect("Cargo profile directory");
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        profile_dir.join("libdriver.so"),
        deps_dir.join("libdriver.so"),
        manifest_dir.join("target/debug/libdriver.so"),
        manifest_dir.join("target/release/libdriver.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| {
            panic!(
                "Rust shared library not found under {}",
                profile_dir.display()
            )
        })
}

fn capture_stdout(call: impl FnOnce()) -> io::Result<Vec<u8>> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock poisoned");

    unsafe {
        if fflush(std::ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut pipe_fds = [-1; 2];
        if pipe(pipe_fds.as_mut_ptr()) != 0 {
            return Err(io::Error::last_os_error());
        }

        let saved_stdout = dup(STDOUT_FILENO);
        if saved_stdout < 0 {
            return Err(io::Error::last_os_error());
        }
        if dup2(pipe_fds[1], STDOUT_FILENO) < 0 {
            return Err(io::Error::last_os_error());
        }
        if close(pipe_fds[1]) != 0 {
            return Err(io::Error::last_os_error());
        }

        call();

        if fflush(std::ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }
        if dup2(saved_stdout, STDOUT_FILENO) < 0 {
            return Err(io::Error::last_os_error());
        }
        if close(saved_stdout) != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0]).read_to_end(&mut output)?;
        Ok(output)
    }
}

fn full_domain_samples() -> Vec<c_int> {
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -65_536,
        -256,
        -1,
        0,
        1,
        255,
        256,
        65_535,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    // Fixed-seed xorshift64* samples make failures reproducible.
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..2_048 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let random = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
        values.push((random >> 32) as u32 as c_int);
    }
    values
}

#[test]
fn driver_matches_for_full_int_domain_samples() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest_dir.join("c_src/build/libdriver.so");
    let rust_path = find_rust_library();

    assert!(
        c_path.is_file(),
        "C shared library missing: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "Rust shared library missing: {}",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_driver: Symbol<'_, Driver> = c_library.get(b"driver\0").expect("load C driver");
        let rust_driver: Symbol<'_, Driver> =
            rust_library.get(b"driver\0").expect("load Rust driver");
        let values = full_domain_samples();

        // Keep each capture below typical pipe capacity while reducing fd churn.
        for (batch_index, batch) in values.chunks(128).enumerate() {
            let c_output = capture_stdout(|| batch.iter().copied().for_each(|x| c_driver(x)))
                .expect("capture C stdout");
            let rust_output = capture_stdout(|| batch.iter().copied().for_each(|x| rust_driver(x)))
                .expect("capture Rust stdout");

            assert_eq!(
                rust_output, c_output,
                "driver output differs in batch {batch_index}, inputs {batch:?}"
            );
        }
    }
}
