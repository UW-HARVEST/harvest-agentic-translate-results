use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type NoArgs = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libdriver.so");
        let rust_path = manifest.join("target/release/libdriver.so");

        assert_library_exists(&c_path);
        assert_library_exists(&rust_path);

        // SAFETY: Both paths point to shared libraries built for this process.
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "missing {}; build both shared libraries before running tests",
        path.display()
    );
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout capture lock poisoned");
    let mut pipe_fds = [-1; 2];

    // SAFETY: All file descriptors are checked before use, and stdout is
    // restored before this function returns.
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush stdout before call");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");

        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(dup2(pipe_fds[1], 1), 1, "redirect stdout");
        assert_eq!(close(pipe_fds[1]), 0, "close copied pipe writer");

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(
                pipe_fds[0],
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
            );
            assert!(count >= 0, "read captured stdout");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close pipe reader");
        output
    }
}

fn assert_same_output(c_call: impl FnOnce(), rust_call: impl FnOnce()) {
    let c_output = capture_stdout(c_call);
    let rust_output = capture_stdout(rust_call);
    assert_eq!(rust_output, c_output);
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

#[test]
fn config_1_print_line_non_null_strings() {
    let libraries = Libraries::load();
    // SAFETY: The symbol signatures match the C definitions.
    let (c_print, rust_print) = unsafe {
        (
            libraries.c.get::<PrintLine>(b"printLine\0").unwrap(),
            libraries.rust.get::<PrintLine>(b"printLine\0").unwrap(),
        )
    };

    let mut inputs = vec![
        CString::new(Vec::<u8>::new()).unwrap(),
        CString::new(b"a".as_slice()).unwrap(),
        CString::new(b"line one\nline two".as_slice()).unwrap(),
    ];
    let mut state = 0x8d26_4f31_70ab_9ce5;
    for _ in 0..256 {
        let length = (next_random(&mut state) % 513) as usize;
        let bytes = (0..length)
            .map(|_| ((next_random(&mut state) % 255) + 1) as u8)
            .collect::<Vec<_>>();
        inputs.push(CString::new(bytes).unwrap());
    }

    for input in inputs {
        let pointer = input.as_ptr();
        assert_same_output(
            || unsafe { c_print(pointer) },
            || unsafe { rust_print(pointer) },
        );
    }
}

#[test]
fn config_2_print_int_line_full_int_domain() {
    let libraries = Libraries::load();
    // SAFETY: The symbol signatures match the C definitions.
    let (c_print, rust_print) = unsafe {
        (
            libraries.c.get::<PrintIntLine>(b"printIntLine\0").unwrap(),
            libraries
                .rust
                .get::<PrintIntLine>(b"printIntLine\0")
                .unwrap(),
        )
    };

    let mut inputs = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    let mut state = 0xd1b5_4a32_d192_ed03;
    inputs.extend((0..512).map(|_| next_random(&mut state) as c_int));

    for input in inputs {
        assert_same_output(
            || unsafe { c_print(input) },
            || unsafe { rust_print(input) },
        );
    }
}

#[test]
fn config_3_bad_direct_call() {
    let libraries = Libraries::load();
    // SAFETY: The symbol signatures match the C definitions.
    let (c_bad, rust_bad) = unsafe {
        (
            libraries.c.get::<NoArgs>(b"bad\0").unwrap(),
            libraries.rust.get::<NoArgs>(b"bad\0").unwrap(),
        )
    };

    assert_same_output(|| unsafe { c_bad() }, || unsafe { rust_bad() });
}

#[test]
fn config_4_good_direct_call() {
    let libraries = Libraries::load();
    // SAFETY: The symbol signatures match the C definitions.
    let (c_good, rust_good) = unsafe {
        (
            libraries.c.get::<NoArgs>(b"good\0").unwrap(),
            libraries.rust.get::<NoArgs>(b"good\0").unwrap(),
        )
    };

    assert_same_output(|| unsafe { c_good() }, || unsafe { rust_good() });
}

#[test]
fn config_5_driver_zero_selects_bad() {
    let libraries = Libraries::load();
    // SAFETY: The symbol signatures match the C definitions.
    let (c_driver, rust_driver) = unsafe {
        (
            libraries.c.get::<Driver>(b"driver\0").unwrap(),
            libraries.rust.get::<Driver>(b"driver\0").unwrap(),
        )
    };

    assert_same_output(|| unsafe { c_driver(0) }, || unsafe { rust_driver(0) });
}

#[test]
fn config_6_driver_nonzero_selects_good() {
    let libraries = Libraries::load();
    // SAFETY: The symbol signatures match the C definitions.
    let (c_driver, rust_driver) = unsafe {
        (
            libraries.c.get::<Driver>(b"driver\0").unwrap(),
            libraries.rust.get::<Driver>(b"driver\0").unwrap(),
        )
    };

    let mut inputs = vec![c_int::MIN, -1, 1, c_int::MAX];
    let mut state = 0x94d0_49bb_1331_11eb;
    while inputs.len() < 516 {
        let input = next_random(&mut state) as c_int;
        if input != 0 {
            inputs.push(input);
        }
    }

    for input in inputs {
        assert_same_output(
            || unsafe { c_driver(input) },
            || unsafe { rust_driver(input) },
        );
    }
}

#[test]
fn error_1_print_line_rejects_null_without_output() {
    let libraries = Libraries::load();
    // SAFETY: The symbol signatures match the C definitions.
    let (c_print, rust_print) = unsafe {
        (
            libraries.c.get::<PrintLine>(b"printLine\0").unwrap(),
            libraries.rust.get::<PrintLine>(b"printLine\0").unwrap(),
        )
    };

    assert_same_output(
        || unsafe { c_print(std::ptr::null()) },
        || unsafe { rust_print(std::ptr::null()) },
    );
}
