use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

type Driver = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;

fn shared_library_paths() -> (PathBuf, PathBuf) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = manifest_dir.join("../c_src/build/libdriver.so");
    let rust_library = manifest_dir.join("target/release/libdriver.so");

    assert!(
        c_library.is_file(),
        "C shared library does not exist: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "Rust shared library does not exist: {}",
        rust_library.display()
    );

    (c_library, rust_library)
}

fn capture_calls(library_path: &Path, inputs: &[(c_int, c_int)]) -> Vec<u8> {
    let library = unsafe {
        Library::new(library_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", library_path.display()))
    };
    let driver: Symbol<Driver> = unsafe {
        library
            .get(b"driver\0")
            .unwrap_or_else(|error| panic!("failed to resolve driver: {error}"))
    };

    let capture_path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}.stdout",
        std::process::id(),
        if library_path.to_string_lossy().contains("c_src") {
            "c"
        } else {
            "rust"
        }
    ));
    let mut capture = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&capture_path)
        .unwrap_or_else(|error| {
            panic!(
                "failed to create stdout capture {}: {error}",
                capture_path.display()
            )
        });

    unsafe {
        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "pre-redirect fflush failed"
        );
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(
            dup2(std::os::fd::AsRawFd::as_raw_fd(&capture), STDOUT_FILENO),
            STDOUT_FILENO,
            "redirecting stdout failed"
        );

        for &(x, y) in inputs {
            driver(x, y);
        }

        assert_eq!(fflush(std::ptr::null_mut()), 0, "captured fflush failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restoring stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "closing saved stdout failed");
    }

    capture
        .seek(SeekFrom::Start(0))
        .expect("seeking captured stdout succeeds");
    let mut output = Vec::new();
    capture
        .read_to_end(&mut output)
        .expect("reading captured stdout succeeds");
    drop(capture);
    std::fs::remove_file(&capture_path).expect("removing stdout capture succeeds");
    output
}

fn test_inputs() -> Vec<(c_int, c_int)> {
    let boundaries = [
        c_int::MIN,
        c_int::MAX,
        -1,
        0,
        1,
        0x5555_5555,
        0xaaaa_aaaau32 as c_int,
    ];
    let mut inputs = Vec::with_capacity(boundaries.len() * boundaries.len() + 16_384);

    for &x in &boundaries {
        for &y in &boundaries {
            inputs.push((x, y));
        }
    }

    let mut state = 0xd1ff_e2e1_5eed_1234_u64;
    for _ in 0..16_384 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let x = state as u32 as c_int;

        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let y = (state >> 32) as u32 as c_int;

        inputs.push((x, y));
    }

    inputs
}

#[test]
fn driver_matches_c_for_full_int_configuration_surface() {
    let (c_library, rust_library) = shared_library_paths();
    let inputs = test_inputs();

    let c_output = capture_calls(&c_library, &inputs);
    let rust_output = capture_calls(&rust_library, &inputs);

    assert_eq!(
        rust_output, c_output,
        "stdout differs for the deterministic boundary and randomized corpus"
    );
}
