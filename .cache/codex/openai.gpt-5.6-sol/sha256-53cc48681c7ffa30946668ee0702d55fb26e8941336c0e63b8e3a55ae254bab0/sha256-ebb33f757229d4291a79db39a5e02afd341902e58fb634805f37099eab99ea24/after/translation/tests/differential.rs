use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type VoidFn = unsafe extern "C" fn();
type DriverFn = unsafe extern "C" fn(c_int);
type PrintLineFn = unsafe extern "C" fn(*const c_char);
type PrintHexCharLineFn = unsafe extern "C" fn(c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest_dir
            .parent()
            .expect("translation has a parent directory")
            .join("c_src/build/libdriver.so");
        let rust_path = std::env::var_os("DRIVER_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest_dir.join("target/release/libdriver.so"));

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

        // SAFETY: Both paths identify shared libraries built by this workspace.
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }
}

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        assert_ne!(seed, 0);
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
}

fn capture_stdout(invoke: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let capture_id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{capture_id}.stdout",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create stdout capture file");

    // SAFETY: These calls only redirect process stdout while the process-wide
    // mutex is held. fflush(NULL) is defined to flush all open output streams.
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(dup2(file.as_raw_fd(), 1), 1, "redirect stdout failed");

        invoke();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout failed");
        assert_eq!(close(saved_stdout), 0, "close saved stdout failed");
    }

    file.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read capture");
    drop(file);
    remove_file(path).expect("remove stdout capture file");
    bytes
}

unsafe fn output_of_void(library: &Library, symbol: &[u8]) -> Vec<u8> {
    // SAFETY: The symbol table establishes this signature for these functions.
    let function: Symbol<VoidFn> = unsafe { library.get(symbol).expect("load void symbol") };
    capture_stdout(|| unsafe { function() })
}

unsafe fn output_of_driver(library: &Library, use_good: c_int) -> Vec<u8> {
    // SAFETY: driver is declared as void driver(int).
    let function: Symbol<DriverFn> = unsafe { library.get(b"driver").expect("load driver") };
    capture_stdout(|| unsafe { function(use_good) })
}

unsafe fn output_of_print_line(library: &Library, line: *const c_char) -> Vec<u8> {
    // SAFETY: printLine is defined as void printLine(const char *).
    let function: Symbol<PrintLineFn> =
        unsafe { library.get(b"printLine").expect("load printLine") };
    capture_stdout(|| unsafe { function(line) })
}

unsafe fn output_of_print_hex(library: &Library, value: c_char) -> Vec<u8> {
    // SAFETY: printHexCharLine is defined as void printHexCharLine(char).
    let function: Symbol<PrintHexCharLineFn> = unsafe {
        library
            .get(b"printHexCharLine")
            .expect("load printHexCharLine")
    };
    capture_stdout(|| unsafe { function(value) })
}

#[test]
fn all_c_exports_are_loadable_from_both_libraries() {
    let libraries = Libraries::load();
    for symbol in [
        b"bad\0".as_slice(),
        b"driver\0",
        b"good\0",
        b"printHexCharLine\0",
        b"printLine\0",
    ] {
        // SAFETY: This test only checks symbol presence and does not invoke it.
        unsafe {
            libraries
                .c
                .get::<*const c_void>(symbol)
                .expect("C symbol missing");
            libraries
                .rust
                .get::<*const c_void>(symbol)
                .expect("Rust symbol missing");
        }
    }
}

#[test]
fn config_1_print_line_non_null_randomized() {
    let libraries = Libraries::load();
    let mut rng = XorShift64::new(0x81c3_79a4_5b6d_e201);
    let mut payloads = vec![Vec::new(), vec![b'X'], vec![b'Z'; 65_536]];

    for _ in 0..256 {
        let length = (rng.next() % 257) as usize;
        let payload = (0..length)
            .map(|_| ((rng.next() % 255) + 1) as u8)
            .collect();
        payloads.push(payload);
    }

    for payload in payloads {
        let input = CString::new(payload).expect("generated bytes contain no NUL");
        // SAFETY: input is a live NUL-terminated C string for both calls.
        let (c_output, rust_output) = unsafe {
            (
                output_of_print_line(&libraries.c, input.as_ptr()),
                output_of_print_line(&libraries.rust, input.as_ptr()),
            )
        };
        assert_eq!(rust_output, c_output);
    }
}

#[test]
fn config_2_print_hex_char_line_full_domain_randomized() {
    let libraries = Libraries::load();
    let mut rng = XorShift64::new(0xc4f7_20de_a951_638b);
    let mut values = vec![c_char::MIN, -1, 0, 1, c_char::MAX];
    values.extend((0..1024).map(|_| rng.next() as u8 as c_char));

    for value in values {
        // SAFETY: Every bit pattern is valid for C char.
        let (c_output, rust_output) = unsafe {
            (
                output_of_print_hex(&libraries.c, value),
                output_of_print_hex(&libraries.rust, value),
            )
        };
        assert_eq!(rust_output, c_output, "value={value}");
    }
}

#[test]
fn config_3_bad_matches() {
    let libraries = Libraries::load();
    // SAFETY: bad has the void(void) ABI in both libraries.
    let (c_output, rust_output) = unsafe {
        (
            output_of_void(&libraries.c, b"bad"),
            output_of_void(&libraries.rust, b"bad"),
        )
    };
    assert_eq!(rust_output, c_output);
}

#[test]
fn config_4_good_matches() {
    let libraries = Libraries::load();
    // SAFETY: good has the void(void) ABI in both libraries.
    let (c_output, rust_output) = unsafe {
        (
            output_of_void(&libraries.c, b"good"),
            output_of_void(&libraries.rust, b"good"),
        )
    };
    assert_eq!(rust_output, c_output);
}

#[test]
fn config_5_driver_zero_selects_bad() {
    let libraries = Libraries::load();
    // SAFETY: zero is a valid C int argument.
    let (c_output, rust_output) = unsafe {
        (
            output_of_driver(&libraries.c, 0),
            output_of_driver(&libraries.rust, 0),
        )
    };
    assert_eq!(rust_output, c_output);
}

#[test]
fn config_6_driver_nonzero_randomized() {
    let libraries = Libraries::load();
    let mut rng = XorShift64::new(0x39a1_5ed7_02bc_846f);
    let mut values = vec![c_int::MIN, -1, 1, c_int::MAX];
    for _ in 0..256 {
        let value = rng.next() as c_int;
        values.push(if value == 0 { 1 } else { value });
    }

    for value in values {
        // SAFETY: Every nonzero C int is accepted and selects good.
        let (c_output, rust_output) = unsafe {
            (
                output_of_driver(&libraries.c, value),
                output_of_driver(&libraries.rust, value),
            )
        };
        assert_eq!(rust_output, c_output, "useGood={value}");
    }
}

#[test]
fn error_1_print_line_null_is_silent() {
    let libraries = Libraries::load();
    // SAFETY: The C implementation explicitly accepts and checks NULL.
    let (c_output, rust_output) = unsafe {
        (
            output_of_print_line(&libraries.c, ptr::null()),
            output_of_print_line(&libraries.rust, ptr::null()),
        )
    };
    assert_eq!(c_output, b"");
    assert_eq!(rust_output, c_output);
}

#[test]
fn error_2_good_b2g_rejects_unsafe_multiplication() {
    let libraries = Libraries::load();
    let expected = b"04\ndata value is too large to perform arithmetic safely.\n".as_slice();

    // SAFETY: good has the void(void) ABI and driver accepts every C int.
    unsafe {
        let c_good = output_of_void(&libraries.c, b"good");
        let rust_good = output_of_void(&libraries.rust, b"good");
        assert_eq!(c_good, expected);
        assert_eq!(rust_good, c_good);

        let c_driver = output_of_driver(&libraries.c, 1);
        let rust_driver = output_of_driver(&libraries.rust, 1);
        assert_eq!(c_driver, expected);
        assert_eq!(rust_driver, c_driver);
    }
}
