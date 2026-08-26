use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_char);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("resolve test executable");
    test_executable
        .parent()
        .and_then(|deps| deps.parent())
        .expect("resolve Cargo profile directory")
        .join("libdriver.so")
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut pipe_fds = [-1; 2];

    // No panicking operation may run while stdout is redirected.
    let setup = unsafe {
        let flushed = fflush(std::ptr::null_mut());
        let piped = pipe(pipe_fds.as_mut_ptr());
        let saved_stdout = if piped == 0 { dup(1) } else { -1 };
        let redirected = if saved_stdout >= 0 {
            dup2(pipe_fds[1], 1)
        } else {
            -1
        };
        (flushed, piped, saved_stdout, redirected)
    };
    assert_eq!(setup.0, 0, "fflush before redirect failed");
    assert_eq!(setup.1, 0, "pipe failed");
    assert!(setup.2 >= 0, "dup stdout failed");
    assert_eq!(setup.3, 1, "redirect stdout failed");

    unsafe {
        close(pipe_fds[1]);
    }
    call();
    let teardown = unsafe {
        let flushed = fflush(std::ptr::null_mut());
        let restored = dup2(setup.2, 1);
        let closed = close(setup.2);
        (flushed, restored, closed)
    };

    let mut output = Vec::new();
    unsafe {
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
    }
    assert_eq!(teardown.0, 0, "fflush after call failed");
    assert_eq!(teardown.1, 1, "restore stdout failed");
    assert_eq!(teardown.2, 0, "close saved stdout failed");
    output
}

fn compare_category(row: &str, values: Vec<u8>) {
    assert!(!values.is_empty(), "{row} has no inputs");

    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C shared library");
    let rust_library =
        unsafe { Library::new(rust_library_path()) }.expect("load Rust shared library");
    let c_driver: Symbol<'_, Driver> =
        unsafe { c_library.get(b"driver\0") }.expect("load C driver symbol");
    let rust_driver: Symbol<'_, Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver symbol");

    let compare = |byte: u8| {
        let c_output = capture_stdout(|| unsafe { c_driver(byte as c_char) });
        let rust_output = capture_stdout(|| unsafe { rust_driver(byte as c_char) });
        assert_eq!(
            rust_output, c_output,
            "{row} diverged for byte 0x{byte:02x} (signed {})",
            byte as i8
        );
    };

    // Cover every member, then add reproducible property-style samples.
    for &byte in &values {
        compare(byte);
    }
    let mut state = 0x6a09_e667_f3bc_c909_u64;
    for _ in 0..128 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        compare(values[(state as usize) % values.len()]);
    }
}

fn inclusive(start: u8, end: u8) -> Vec<u8> {
    (start..=end).collect()
}

fn concatenate(ranges: &[(u8, u8)]) -> Vec<u8> {
    ranges
        .iter()
        .flat_map(|&(start, end)| start..=end)
        .collect()
}

macro_rules! differential_row {
    ($name:ident, $row:literal, $values:expr) => {
        #[test]
        fn $name() {
            compare_category($row, $values);
        }
    };
}

differential_row!(row_01_eof, "CONFIGS.md row 1", vec![0xff]);
differential_row!(
    row_02_negative_signed_chars,
    "CONFIGS.md row 2",
    inclusive(0x80, 0xfe)
);
differential_row!(row_03_nul, "CONFIGS.md row 3", vec![0x00]);
differential_row!(
    row_04_non_whitespace_control,
    "CONFIGS.md row 4",
    concatenate(&[(0x01, 0x08), (0x0e, 0x1f), (0x7f, 0x7f)])
);
differential_row!(row_05_horizontal_tab, "CONFIGS.md row 5", vec![0x09]);
differential_row!(
    row_06_other_whitespace_control,
    "CONFIGS.md row 6",
    inclusive(0x0a, 0x0d)
);
differential_row!(row_07_space, "CONFIGS.md row 7", vec![0x20]);
differential_row!(
    row_08_ascii_punctuation,
    "CONFIGS.md row 8",
    concatenate(&[(0x21, 0x2f), (0x3a, 0x40), (0x5b, 0x60), (0x7b, 0x7e)])
);
differential_row!(
    row_09_decimal_digit,
    "CONFIGS.md row 9",
    inclusive(b'0', b'9')
);
differential_row!(
    row_10_uppercase_hex,
    "CONFIGS.md row 10",
    inclusive(b'A', b'F')
);
differential_row!(
    row_11_uppercase_non_hex,
    "CONFIGS.md row 11",
    inclusive(b'G', b'Z')
);
differential_row!(
    row_12_lowercase_hex,
    "CONFIGS.md row 12",
    inclusive(b'a', b'f')
);
differential_row!(
    row_13_lowercase_non_hex,
    "CONFIGS.md row 13",
    inclusive(b'g', b'z')
);

#[test]
fn configuration_rows_partition_the_char_domain() {
    let rows = [
        vec![0xff],
        inclusive(0x80, 0xfe),
        vec![0x00],
        concatenate(&[(0x01, 0x08), (0x0e, 0x1f), (0x7f, 0x7f)]),
        vec![0x09],
        inclusive(0x0a, 0x0d),
        vec![0x20],
        concatenate(&[(0x21, 0x2f), (0x3a, 0x40), (0x5b, 0x60), (0x7b, 0x7e)]),
        inclusive(b'0', b'9'),
        inclusive(b'A', b'F'),
        inclusive(b'G', b'Z'),
        inclusive(b'a', b'f'),
        inclusive(b'g', b'z'),
    ];
    let mut counts = [0_u8; 256];
    for byte in rows.into_iter().flatten() {
        counts[usize::from(byte)] += 1;
    }
    assert!(counts.into_iter().all(|count| count == 1));
}
