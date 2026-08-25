use libloading::Library;
use std::ffi::c_void;
use std::fs;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, MutexGuard};

type DriverFn = unsafe extern "C" fn(c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

const STDIN_FILENO: c_int = 0;
const STDOUT_FILENO: c_int = 1;

static PROCESS_IO: Mutex<()> = Mutex::new(());

extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
    fn clearerr(stream: *mut c_void);
    fn __fpurge(stream: *mut c_void);
    static mut stdin: *mut c_void;
}

struct Api {
    _library: Library,
    driver: DriverFn,
    main: MainFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = Library::new(path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = *library
            .get::<DriverFn>(b"driver\0")
            .unwrap_or_else(|error| panic!("missing driver in {}: {error}", path.display()));
        let main = *library
            .get::<MainFn>(b"main\0")
            .unwrap_or_else(|error| panic!("missing main in {}: {error}", path.display()));

        Self {
            _library: library,
            driver,
            main,
        }
    }
}

fn libraries() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = root.join("c_src/build/libdriver_c.so");
    let rust_path = root.join("target/debug/libdriver.so");

    assert!(
        c_path.is_file(),
        "C shared object does not exist: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "Rust shared object does not exist: {}",
        rust_path.display()
    );

    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn io_lock() -> MutexGuard<'static, ()> {
    PROCESS_IO
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn syscall_result(result: c_int, operation: &str) -> c_int {
    if result < 0 {
        panic!("{operation} failed: {}", std::io::Error::last_os_error());
    }
    result
}

unsafe fn write_all(fd: c_int, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let written = write(fd, bytes.as_ptr().cast(), bytes.len());
        if written < 0 {
            panic!("write failed: {}", std::io::Error::last_os_error());
        }
        assert_ne!(written, 0, "write unexpectedly returned zero");
        bytes = &bytes[written as usize..];
    }
}

unsafe fn read_to_end(fd: c_int) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 256];

    loop {
        let count = read(fd, buffer.as_mut_ptr().cast(), buffer.len());
        if count < 0 {
            panic!("read failed: {}", std::io::Error::last_os_error());
        }
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    output
}

unsafe fn capture_call<R>(input: Option<&[u8]>, call: impl FnOnce() -> R) -> (R, Vec<u8>) {
    assert_eq!(fflush(ptr::null_mut()), 0, "fflush before capture failed");

    let stdout_backup = syscall_result(dup(STDOUT_FILENO), "dup stdout");
    let mut output_pipe = [0; 2];
    syscall_result(pipe(output_pipe.as_mut_ptr()), "pipe stdout");
    syscall_result(dup2(output_pipe[1], STDOUT_FILENO), "redirect stdout");
    syscall_result(close(output_pipe[1]), "close output writer");

    let stdin_backup = input.map(|bytes| {
        let backup = syscall_result(dup(STDIN_FILENO), "dup stdin");
        let mut input_pipe = [0; 2];
        syscall_result(pipe(input_pipe.as_mut_ptr()), "pipe stdin");
        write_all(input_pipe[1], bytes);
        syscall_result(close(input_pipe[1]), "close input writer");
        __fpurge(stdin);
        clearerr(stdin);
        syscall_result(dup2(input_pipe[0], STDIN_FILENO), "redirect stdin");
        syscall_result(close(input_pipe[0]), "close input reader");
        backup
    });

    let result = call();

    assert_eq!(fflush(ptr::null_mut()), 0, "fflush after capture failed");
    syscall_result(dup2(stdout_backup, STDOUT_FILENO), "restore stdout");
    syscall_result(close(stdout_backup), "close stdout backup");

    if let Some(backup) = stdin_backup {
        __fpurge(stdin);
        clearerr(stdin);
        syscall_result(dup2(backup, STDIN_FILENO), "restore stdin");
        syscall_result(close(backup), "close stdin backup");
        clearerr(stdin);
    }

    let output = read_to_end(output_pipe[0]);
    syscall_result(close(output_pipe[0]), "close output reader");
    (result, output)
}

fn next_random(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

fn random_int(state: &mut u64) -> c_int {
    next_random(state) as c_int
}

#[test]
fn config_01_driver_direct_scalar() {
    let _io = io_lock();
    let (c, rust) = libraries();
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -1,
        0,
        1,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    let mut state = 0x7f4a_7c15_93d2_6e81;
    values.extend((0..256).map(|_| random_int(&mut state)));

    for x in values {
        let (_, c_output) = unsafe { capture_call(None, || (c.driver)(x)) };
        let (_, rust_output) = unsafe { capture_call(None, || (rust.driver)(x)) };
        assert_eq!(rust_output, c_output, "driver output differed for x={x}");
    }
}

#[test]
fn config_02_main_valid_decimal() {
    let _io = io_lock();
    let (c, rust) = libraries();
    let mut state = 0x243f_6a88_85a3_08d3;

    for index in 0..256 {
        let x = random_int(&mut state);
        let input = match index % 4 {
            0 => format!("{x}"),
            1 => format!(" \t{x}"),
            2 if x >= 0 => format!("+{x}"),
            2 => format!("{x}"),
            _ if x >= 0 => format!("{x:010}"),
            _ => format!("-{:010}", -(x as i64)),
        };
        let (c_result, c_output) = unsafe { capture_call(Some(input.as_bytes()), || (c.main)()) };
        let (rust_result, rust_output) =
            unsafe { capture_call(Some(input.as_bytes()), || (rust.main)()) };

        assert_eq!(rust_result, c_result, "main return differed for {input:?}");
        assert_eq!(c_result, 0, "C main returned nonzero for {input:?}");
        assert_eq!(rust_output, c_output, "main output differed for {input:?}");
    }
}

#[test]
fn config_03_main_nonmatching_input() {
    let _io = io_lock();
    let (c, rust) = libraries();
    let mut state = 0x1319_8a2e_0370_7344;

    for _ in 0..128 {
        let first = b'a' + (next_random(&mut state) % 26) as u8;
        let suffix = next_random(&mut state);
        let input = format!("{}{suffix:08x}", first as char);
        let (c_result, c_output) = unsafe { capture_call(Some(input.as_bytes()), || (c.main)()) };
        let (rust_result, rust_output) =
            unsafe { capture_call(Some(input.as_bytes()), || (rust.main)()) };

        assert_eq!(rust_result, c_result, "main return differed for {input:?}");
        assert_eq!(c_result, 0, "C main returned nonzero for {input:?}");
        assert_eq!(rust_output, c_output, "main output differed for {input:?}");
        assert_eq!(c_output, b"300\n", "unexpected C output for {input:?}");
    }
}

#[test]
fn config_04_main_eof() {
    let _io = io_lock();
    let (c, rust) = libraries();
    let mut state = 0xa409_3822_299f_31d0;

    for index in 0..64 {
        let whitespace = [b' ', b'\t', b'\n'];
        let input = if index == 0 {
            Vec::new()
        } else {
            (0..(next_random(&mut state) % 12))
                .map(|_| whitespace[(next_random(&mut state) % 3) as usize])
                .collect()
        };
        let (c_result, c_output) = unsafe { capture_call(Some(&input), || (c.main)()) };
        let (rust_result, rust_output) = unsafe { capture_call(Some(&input), || (rust.main)()) };

        assert_eq!(rust_result, c_result, "main return differed at EOF");
        assert_eq!(c_result, 0, "C main returned nonzero at EOF");
        assert_eq!(rust_output, c_output, "main output differed at EOF");
        assert_eq!(c_output, b"300\n", "unexpected C output at EOF");
    }
}

#[test]
fn generic_main_out_of_range_decimal() {
    let _io = io_lock();
    let (c, rust) = libraries();
    let inputs = [
        "2147483648",
        "-2147483649",
        "999999999999999999999999999999999999",
        "-999999999999999999999999999999999999",
    ];

    for input in inputs {
        let (c_result, c_output) = unsafe { capture_call(Some(input.as_bytes()), || (c.main)()) };
        let (rust_result, rust_output) =
            unsafe { capture_call(Some(input.as_bytes()), || (rust.main)()) };

        assert_eq!(rust_result, c_result, "main return differed for {input:?}");
        assert_eq!(rust_output, c_output, "main output differed for {input:?}");
    }
}

#[test]
fn shared_objects_export_every_c_symbol() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_symbols = dynamic_defined_symbols(&root.join("c_src/build/libdriver_c.so"));
    let rust_symbols = dynamic_defined_symbols(&root.join("target/debug/libdriver.so"));
    let missing: Vec<_> = c_symbols
        .iter()
        .filter(|symbol| !rust_symbols.contains(symbol))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust shared object is missing {missing:?}"
    );
}

fn dynamic_defined_symbols(path: &Path) -> Vec<String> {
    let output = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap_or_else(|error| panic!("failed to run nm for {}: {error}", path.display()));
    assert!(
        output.status.success(),
        "nm failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("nm output was not UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .map(str::to_owned)
        .collect()
}

#[test]
fn phase_a_artifacts_exist() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for name in ["SYMBOLS.md", "ERRORS.md", "CONFIGS.md"] {
        let contents = fs::read_to_string(root.join(name))
            .unwrap_or_else(|error| panic!("failed to read {name}: {error}"));
        assert!(!contents.trim().is_empty(), "{name} is empty");
    }
}
