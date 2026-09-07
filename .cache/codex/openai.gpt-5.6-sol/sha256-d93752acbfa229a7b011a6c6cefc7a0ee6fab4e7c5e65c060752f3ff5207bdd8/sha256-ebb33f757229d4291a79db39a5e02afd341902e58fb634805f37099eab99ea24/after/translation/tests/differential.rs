use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
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

struct ConfigRow {
    number: usize,
    description: &'static str,
    values: Vec<u8>,
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        manifest.join("../c_src/build/libdriver.so"),
        manifest.join("target/release/libdriver.so"),
    )
}

unsafe fn capture_stdout(function: Driver, value: u8) -> Vec<u8> {
    let mut pipe_fds = [-1; 2];

    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    unsafe { function(value as c_char) };
    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);

    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut output).unwrap();
    output
}

fn config_rows() -> Vec<ConfigRow> {
    vec![
        ConfigRow {
            number: 1,
            description: "non-whitespace controls",
            values: (0x00..=0x08)
                .chain(0x0e..=0x1f)
                .chain(std::iter::once(0x7f))
                .collect(),
        },
        ConfigRow {
            number: 2,
            description: "horizontal tab",
            values: vec![0x09],
        },
        ConfigRow {
            number: 3,
            description: "other whitespace controls",
            values: (0x0a..=0x0d).collect(),
        },
        ConfigRow {
            number: 4,
            description: "ASCII space",
            values: vec![0x20],
        },
        ConfigRow {
            number: 5,
            description: "punctuation",
            values: (0x21..=0x2f)
                .chain(0x3a..=0x40)
                .chain(0x5b..=0x60)
                .chain(0x7b..=0x7e)
                .collect(),
        },
        ConfigRow {
            number: 6,
            description: "decimal and hexadecimal digits",
            values: (b'0'..=b'9').collect(),
        },
        ConfigRow {
            number: 7,
            description: "uppercase hexadecimal letters",
            values: (b'A'..=b'F').collect(),
        },
        ConfigRow {
            number: 8,
            description: "other uppercase letters",
            values: (b'G'..=b'Z').collect(),
        },
        ConfigRow {
            number: 9,
            description: "lowercase hexadecimal letters",
            values: (b'a'..=b'f').collect(),
        },
        ConfigRow {
            number: 10,
            description: "other lowercase letters",
            values: (b'g'..=b'z').collect(),
        },
        ConfigRow {
            number: 11,
            description: "negative promoted char values",
            values: (0x80..=0xfe).collect(),
        },
        ConfigRow {
            number: 12,
            description: "EOF-equivalent signed char",
            values: vec![0xff],
        },
    ]
}

#[test]
fn every_configuration_row_matches_through_shared_library_exports() {
    let _stdout_guard = STDOUT_LOCK.lock().unwrap();
    let (c_path, rust_path) = library_paths();

    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }.unwrap();
    let rust_library = unsafe { Library::new(&rust_path) }.unwrap();
    let c_driver: Driver = unsafe { *c_library.get::<Driver>(b"driver\0").unwrap() };
    let rust_driver: Driver = unsafe { *rust_library.get::<Driver>(b"driver\0").unwrap() };

    let rows = config_rows();
    let mut domain = rows
        .iter()
        .flat_map(|row| row.values.iter().copied())
        .collect::<Vec<_>>();
    domain.sort_unstable();
    assert_eq!(domain, (0u8..=u8::MAX).collect::<Vec<_>>());

    for row in rows {
        // Exhaust the row once, then make many deterministic randomized calls.
        let mut inputs = row.values.clone();
        let mut state = 0x9e37_79b9_7f4a_7c15_u64 ^ row.number as u64;
        for _ in 0..128 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            inputs.push(row.values[(state as usize) % row.values.len()]);
        }

        for value in inputs {
            let c_output = unsafe { capture_stdout(c_driver, value) };
            let rust_output = unsafe { capture_stdout(rust_driver, value) };
            assert_eq!(
                rust_output, c_output,
                "CONFIGS.md row {} ({}) diverged for byte 0x{value:02x}",
                row.number, row.description
            );
        }
    }
}
