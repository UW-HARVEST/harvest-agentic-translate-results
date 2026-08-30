use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::ptr;

type Driver = unsafe extern "C" fn(c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0, "pipe failed");

    assert_eq!(
        unsafe { fflush(ptr::null_mut()) },
        0,
        "pre-call flush failed"
    );
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup failed");
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1, "stdout redirect failed");
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0, "pipe write close failed");

    call();

    assert_eq!(
        unsafe { fflush(ptr::null_mut()) },
        0,
        "post-call flush failed"
    );
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "stdout restore failed");
    assert_eq!(
        unsafe { close(saved_stdout) },
        0,
        "saved stdout close failed"
    );

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

unsafe fn run_batch(driver: Driver, inputs: &[u8]) -> Vec<u8> {
    unsafe {
        capture_stdout(|| {
            for &input in inputs {
                driver(input as c_char);
            }
        })
    }
}

fn randomized_samples(values: &[u8], state: &mut u64) -> Vec<u8> {
    const SAMPLES_PER_ROW: usize = 64;
    let mut samples = Vec::with_capacity(values.len() + SAMPLES_PER_ROW);
    samples.extend_from_slice(values);

    for _ in 0..SAMPLES_PER_ROW {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        samples.push(values[*state as usize % values.len()]);
    }
    samples
}

fn inclusive(start: u8, end: u8) -> Vec<u8> {
    (start..=end).collect()
}

#[test]
fn all_configuration_rows_match_through_dynamic_ffi() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path).expect("load C shared object") };
    let rust_library = unsafe { Library::new(&rust_path).expect("load Rust shared object") };
    let c_driver: Symbol<Driver> = unsafe { c_library.get(b"driver\0").expect("C driver export") };
    let rust_driver: Symbol<Driver> =
        unsafe { rust_library.get(b"driver\0").expect("Rust driver export") };

    let mut other_controls = inclusive(0x01, 0x08);
    other_controls.extend(inclusive(0x0e, 0x1f));
    other_controls.push(0x7f);

    let mut punctuation: Vec<u8> = inclusive(0x21, 0x7e)
        .into_iter()
        .filter(|c| !c.is_ascii_alphanumeric())
        .collect();
    punctuation.retain(|&c| c != 0x20);

    let rows = [
        ("01 signed negative", inclusive(0x80, 0xff)),
        ("02 NUL", vec![0x00]),
        ("03 horizontal tab", vec![0x09]),
        ("04 newline whitespace", inclusive(0x0a, 0x0d)),
        ("05 other controls", other_controls),
        ("06 space", vec![0x20]),
        ("07 decimal digit", inclusive(b'0', b'9')),
        ("08 uppercase hexadecimal", inclusive(b'A', b'F')),
        ("09 uppercase non-hexadecimal", inclusive(b'G', b'Z')),
        ("10 lowercase hexadecimal", inclusive(b'a', b'f')),
        ("11 lowercase non-hexadecimal", inclusive(b'g', b'z')),
        ("12 printable punctuation", punctuation),
    ];

    let mut seed = 0x4d59_5df4_d0f3_3173_u64;
    let mut covered = [false; 256];
    for (row, values) in rows {
        for &value in &values {
            covered[value as usize] = true;
        }
        let inputs = randomized_samples(&values, &mut seed);
        let c_output = unsafe { run_batch(*c_driver, &inputs) };
        let rust_output = unsafe { run_batch(*rust_driver, &inputs) };
        assert_eq!(rust_output, c_output, "CONFIGS.md row {row}");
    }

    assert!(
        covered.into_iter().all(|value| value),
        "configuration rows must cover every char bit pattern"
    );
}
