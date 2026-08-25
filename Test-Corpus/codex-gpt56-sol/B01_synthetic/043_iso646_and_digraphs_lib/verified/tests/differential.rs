use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

type Driver = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DRIVER_RUST_SO") {
        return path.into();
    }

    let executable = std::env::current_exe().expect("failed to locate test executable");
    let profile_dir = executable
        .parent()
        .and_then(Path::parent)
        .expect("test executable is not under target/<profile>/deps");
    let profile_library = profile_dir.join("libdriver.so");
    if profile_library.is_file() {
        return profile_library;
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("libdriver.so")
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn test_cases() -> Vec<(c_int, c_int)> {
    let boundaries = [
        c_int::MIN,
        c_int::MIN + 1,
        -65_536,
        -32_769,
        -32_768,
        -257,
        -256,
        -2,
        -1,
        0,
        1,
        2,
        255,
        256,
        32_767,
        32_768,
        65_535,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    let mut cases = Vec::with_capacity(boundaries.len() * boundaries.len() + 10_000);

    for &x in &boundaries {
        for &y in &boundaries {
            cases.push((x, y));
        }
    }

    // SplitMix64 gives a stable stream without adding another test dependency.
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..10_000 {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        cases.push((z as u32 as c_int, (z >> 32) as u32 as c_int));
    }

    cases
}

fn capture_stdout(driver: Driver, cases: &[(c_int, c_int)], label: &str) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}.out",
        std::process::id(),
        label
    ));
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("failed to create output capture");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "fflush failed");
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(dup2(output.as_raw_fd(), 1), 1, "redirecting stdout failed");

        for &(x, y) in cases {
            driver(x, y);
        }

        let flush_result = fflush(std::ptr::null_mut());
        let restore_result = dup2(saved_stdout, 1);
        let close_result = close(saved_stdout);
        assert_eq!(flush_result, 0, "flushing captured stdout failed");
        assert_eq!(restore_result, 1, "restoring stdout failed");
        assert_eq!(close_result, 0, "closing duplicated stdout failed");
    }

    output
        .seek(SeekFrom::Start(0))
        .expect("failed to rewind captured output");
    let mut bytes = Vec::new();
    output
        .read_to_end(&mut bytes)
        .expect("failed to read captured output");
    std::fs::remove_file(path).expect("failed to remove output capture");
    bytes
}

#[test]
fn driver_matches_c_for_full_scalar_configuration() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {rust_path:?}"
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("failed to load C shared library");
        let rust_library = Library::new(&rust_path).expect("failed to load Rust shared library");
        let c_driver = *c_library
            .get::<Driver>(b"driver\0")
            .expect("C library does not export driver");
        let rust_driver = *rust_library
            .get::<Driver>(b"driver\0")
            .expect("Rust library does not export driver");

        let cases = test_cases();
        let c_output = capture_stdout(c_driver, &cases, "c");
        let rust_output = capture_stdout(rust_driver, &cases, "rust");
        assert_eq!(
            rust_output,
            c_output,
            "driver output differed across {} inputs",
            cases.len()
        );
    }
}
