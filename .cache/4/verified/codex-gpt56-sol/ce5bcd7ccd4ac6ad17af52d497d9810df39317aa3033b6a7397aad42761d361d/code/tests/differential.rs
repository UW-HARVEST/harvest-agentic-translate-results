use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

type VoidFn = unsafe extern "C" fn();
type MainFn = unsafe extern "C" fn() -> c_int;
type PrintLineFn = unsafe extern "C" fn(*const c_char);
type PrintHexCharLineFn = unsafe extern "C" fn(i8);

unsafe extern "C" {
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;

    fn __fpurge(stream: *mut c_void);
    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static TEMP_ID: AtomicU64 = AtomicU64::new(0);
static STDIO_LOCK: Mutex<()> = Mutex::new(());

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn range(&mut self, upper_exclusive: u64) -> u64 {
        self.next_u64() % upper_exclusive
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_LIB_PATH") {
        return path.into();
    }

    let direct = crate_root().join("target/debug/libdriver.so");
    if direct.is_file() {
        return direct;
    }

    let deps = crate_root().join("target/debug/deps");
    fs::read_dir(&deps)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", deps.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libdriver") && name.ends_with(".so"))
        })
        .unwrap_or_else(|| panic!("Rust shared library not found under target/debug"))
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_LIB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root().join("c_src/build/libdriver_c.so"))
}

fn temp_file(label: &str) -> (PathBuf, File) {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{id}-{label}",
        std::process::id()
    ));
    let file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    (path, file)
}

fn capture_stdio<T>(input: &[u8], call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _stdio_guard = STDIO_LOCK.lock().unwrap();
    let (input_path, mut input_file) = temp_file("stdin");
    let (output_path, mut output_file) = temp_file("stdout");
    input_file.write_all(input).unwrap();
    input_file.seek(SeekFrom::Start(0)).unwrap();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        let saved_stdin = dup(0);
        let saved_stdout = dup(1);
        assert!(saved_stdin >= 0);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(input_file.as_raw_fd(), 0), 0);
        assert_eq!(dup2(output_file.as_raw_fd(), 1), 1);
        __fpurge(stdin);
        __fpurge(stdout);
        clearerr(stdin);
        clearerr(stdout);

        let result = call();

        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdin, 0), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdin), 0);
        assert_eq!(close(saved_stdout), 0);
        __fpurge(stdin);
        clearerr(stdin);
        clearerr(stdout);

        output_file.seek(SeekFrom::Start(0)).unwrap();
        let mut output = Vec::new();
        output_file.read_to_end(&mut output).unwrap();
        drop(input_file);
        drop(output_file);
        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();
        (result, output)
    }
}

unsafe fn call_void(library: &Library, symbol: &[u8]) -> Vec<u8> {
    let function: Symbol<VoidFn> = library.get(symbol).unwrap();
    capture_stdio(&[], || function()).1
}

unsafe fn call_main(library: &Library, input: &[u8]) -> (c_int, Vec<u8>) {
    let function: Symbol<MainFn> = library.get(b"main").unwrap();
    capture_stdio(input, || function())
}

unsafe fn call_print_line(library: &Library, value: *const c_char) -> Vec<u8> {
    let function: Symbol<PrintLineFn> = library.get(b"printLine").unwrap();
    capture_stdio(&[], || function(value)).1
}

unsafe fn call_print_hex(library: &Library, value: i8) -> Vec<u8> {
    let function: Symbol<PrintHexCharLineFn> = library.get(b"printHexCharLine").unwrap();
    capture_stdio(&[], || function(value)).1
}

fn load_libraries() -> (Library, Library) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
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
        (
            Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display())),
            Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display())),
        )
    }
}

fn assert_main_equal(c: &Library, rust: &Library, input: &[u8]) {
    let c_result = unsafe { call_main(c, input) };
    let rust_result = unsafe { call_main(rust, input) };
    assert_eq!(rust_result, c_result, "main input: {input:?}");
}

fn formatted_integer(rng: &mut Rng, value: i64, allow_plus: bool) -> Vec<u8> {
    let whitespace: [&[u8]; 5] = [b"", b" ", b"\t", b"\n", b" \t\r\n"];
    let prefix = whitespace[rng.range(whitespace.len() as u64) as usize];
    let leading_zeroes = rng.range(5) as usize;
    let sign = if value < 0 {
        "-"
    } else if allow_plus && rng.range(2) == 0 {
        "+"
    } else {
        ""
    };
    let magnitude = value.unsigned_abs();
    format!(
        "{}{}{}{}",
        String::from_utf8_lossy(prefix),
        sign,
        "0".repeat(leading_zeroes),
        magnitude
    )
    .into_bytes()
}

#[test]
fn valid_configuration_surface_matches() {
    let (c, rust) = load_libraries();
    let mut rng = Rng::new(0x5eed_c0de_1234_5678);

    let empty = CString::new("").unwrap();
    assert_eq!(unsafe { call_print_line(&rust, empty.as_ptr()) }, unsafe {
        call_print_line(&c, empty.as_ptr())
    });

    for _ in 0..256 {
        let length = 1 + rng.range(128) as usize;
        let bytes: Vec<u8> = (0..length).map(|_| 1 + rng.range(255) as u8).collect();
        let value = CString::new(bytes).unwrap();
        assert_eq!(unsafe { call_print_line(&rust, value.as_ptr()) }, unsafe {
            call_print_line(&c, value.as_ptr())
        });
    }

    for raw in i8::MIN..=i8::MAX {
        assert_eq!(
            unsafe { call_print_hex(&rust, raw) },
            unsafe { call_print_hex(&c, raw) },
            "printHexCharLine({raw})"
        );
    }

    assert_eq!(unsafe { call_void(&rust, b"bad") }, unsafe {
        call_void(&c, b"bad")
    });
    assert_eq!(unsafe { call_void(&rust, b"good") }, unsafe {
        call_void(&c, b"good")
    });

    for _ in 0..128 {
        assert_main_equal(&c, &rust, &formatted_integer(&mut rng, 0, true));

        let positive = 1 + rng.range(i32::MAX as u64);
        assert_main_equal(
            &c,
            &rust,
            &formatted_integer(&mut rng, positive as i64, true),
        );

        let magnitude = 1 + rng.range((i32::MAX as u64) + 1);
        assert_main_equal(
            &c,
            &rust,
            &formatted_integer(&mut rng, -(magnitude as i64), false),
        );
    }
}

#[test]
fn error_surface_matches() {
    let (c, rust) = load_libraries();

    assert_eq!(
        unsafe { call_print_line(&rust, std::ptr::null()) },
        unsafe { call_print_line(&c, std::ptr::null()) }
    );

    for input in [
        b"".as_slice(),
        b" \t\r\n".as_slice(),
        b"x".as_slice(),
        b" x123".as_slice(),
        b"+".as_slice(),
        b"-".as_slice(),
        b"2147483648".as_slice(),
        b"-2147483649".as_slice(),
    ] {
        assert_main_equal(&c, &rust, input);
    }

    assert_eq!(unsafe { call_void(&rust, b"good") }, unsafe {
        call_void(&c, b"good")
    });
}

#[test]
fn every_c_api_symbol_is_loadable_from_both_libraries() {
    let (c, rust) = load_libraries();
    for symbol in [
        b"bad".as_slice(),
        b"good".as_slice(),
        b"main".as_slice(),
        b"printHexCharLine".as_slice(),
        b"printLine".as_slice(),
    ] {
        unsafe {
            let _: Symbol<*mut c_void> = c.get(symbol).unwrap();
            let _: Symbol<*mut c_void> = rust.get(symbol).unwrap();
        }
    }
}

#[test]
fn shared_library_paths_are_files() {
    for path in [c_library_path(), rust_library_path()] {
        assert!(Path::new(&path).is_file(), "missing {}", path.display());
    }
}
