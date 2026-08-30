use libloading::{Library, Symbol};
use std::ffi::{c_int, c_uint, c_void};
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(c_uint, c_uint, bool, c_int);
type PrintFoo = unsafe extern "C" fn(*const FooAbi);

#[repr(C)]
#[derive(Clone, Copy)]
struct FooAbi {
    bit_fields: c_uint,
    z: c_int,
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES_PER_ROW: usize = 128;
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum ZShape {
    Negative,
    Zero,
    Positive,
}

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("../c_src/build/libdriver.so"))
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(path);
    }

    let direct = manifest_dir().join("target/release/libdriver.so");
    if direct.is_file() {
        return direct;
    }

    let deps = manifest_dir().join("target/release/deps");
    fs::read_dir(&deps)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", deps.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libdriver") && name.ends_with(".so"))
        })
        .unwrap_or_else(|| panic!("Rust shared library not found under {}", deps.display()))
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{id}.out",
        std::process::id()
    ));
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", path.display()));

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "fflush before capture failed");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup stdout failed");
        assert_eq!(
            dup2(output.as_raw_fd(), STDOUT_FILENO),
            STDOUT_FILENO,
            "redirecting stdout failed"
        );

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "fflush after call failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restoring stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "closing saved stdout failed");
    }

    drop(output);
    let bytes = fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    fs::remove_file(&path)
        .unwrap_or_else(|error| panic!("failed to remove {}: {error}", path.display()));
    bytes
}

fn shaped_unsigned(rng: &mut XorShift64, bits: u32, truncated: bool, index: usize) -> u32 {
    let limit = 1_u32 << bits;
    if truncated {
        match index {
            0 => limit,
            1 => u32::MAX,
            _ => rng.next_u32() | limit,
        }
    } else {
        match index {
            0 => 0,
            1 => limit - 1,
            _ => rng.next_u32() & (limit - 1),
        }
    }
}

fn shaped_z(rng: &mut XorShift64, shape: ZShape, index: usize) -> i32 {
    match shape {
        ZShape::Negative => match index {
            0 => i32::MIN,
            1 => -1,
            _ => (rng.next_u32() | 0x8000_0000) as i32,
        },
        ZShape::Zero => 0,
        ZShape::Positive => match index {
            0 => 1,
            1 => i32::MAX,
            _ => (rng.next_u32() & 0x7fff_ffff).max(1) as i32,
        },
    }
}

fn run_configuration(row: u32, x_truncated: bool, y_truncated: bool, b: bool, z_shape: ZShape) {
    let _capture_guard = CAPTURE_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut rng = XorShift64::new(0xd1ff_e2e0_5eed_0000 ^ u64::from(row));
    let mut cases = Vec::with_capacity(RANDOM_CASES_PER_ROW);

    for index in 0..RANDOM_CASES_PER_ROW {
        let x = shaped_unsigned(&mut rng, 2, x_truncated, index);
        let y = shaped_unsigned(&mut rng, 3, y_truncated, index);
        let z = shaped_z(&mut rng, z_shape, index);
        let padding = rng.next_u32() & !0x3f;
        let foo = FooAbi {
            bit_fields: padding | (x & 0x3) | ((y & 0x7) << 2) | (u32::from(b) << 5),
            z,
        };
        cases.push((x, y, b, z, foo));
    }

    unsafe {
        let c_library = Library::new(c_library_path()).expect("load C shared library");
        let rust_library = Library::new(rust_library_path()).expect("load Rust shared library");
        let c_driver: Symbol<Driver> = c_library.get(b"driver").expect("load C driver");
        let rust_driver: Symbol<Driver> = rust_library.get(b"driver").expect("load Rust driver");
        let c_print_foo: Symbol<PrintFoo> = c_library.get(b"print_foo").expect("load C print_foo");
        let rust_print_foo: Symbol<PrintFoo> =
            rust_library.get(b"print_foo").expect("load Rust print_foo");

        let c_driver_output = capture_stdout(|| {
            for &(x, y, b, z, _) in &cases {
                c_driver(x, y, b, z);
            }
        });
        let rust_driver_output = capture_stdout(|| {
            for &(x, y, b, z, _) in &cases {
                rust_driver(x, y, b, z);
            }
        });
        let c_print_output = capture_stdout(|| {
            for &(_, _, _, _, foo) in &cases {
                c_print_foo(&foo);
            }
        });
        let rust_print_output = capture_stdout(|| {
            for &(_, _, _, _, foo) in &cases {
                rust_print_foo(&foo);
            }
        });

        assert_eq!(
            c_driver_output, rust_driver_output,
            "CONFIGS.md row {row}: driver output differs"
        );
        assert_eq!(
            c_print_output, rust_print_output,
            "CONFIGS.md row {row}: print_foo output differs"
        );
        assert_eq!(
            c_driver_output, c_print_output,
            "CONFIGS.md row {row}: test setup does not reproduce driver state"
        );
    }
}

macro_rules! configuration_test {
    ($name:ident, $row:literal, $x_truncated:literal, $y_truncated:literal, $b:literal, $z:ident) => {
        fn $name() {
            run_configuration($row, $x_truncated, $y_truncated, $b, ZShape::$z);
        }
    };
}

configuration_test!(config_01, 1, false, false, false, Negative);
configuration_test!(config_02, 2, false, false, false, Zero);
configuration_test!(config_03, 3, false, false, false, Positive);
configuration_test!(config_04, 4, false, false, true, Negative);
configuration_test!(config_05, 5, false, false, true, Zero);
configuration_test!(config_06, 6, false, false, true, Positive);
configuration_test!(config_07, 7, false, true, false, Negative);
configuration_test!(config_08, 8, false, true, false, Zero);
configuration_test!(config_09, 9, false, true, false, Positive);
configuration_test!(config_10, 10, false, true, true, Negative);
configuration_test!(config_11, 11, false, true, true, Zero);
configuration_test!(config_12, 12, false, true, true, Positive);
configuration_test!(config_13, 13, true, false, false, Negative);
configuration_test!(config_14, 14, true, false, false, Zero);
configuration_test!(config_15, 15, true, false, false, Positive);
configuration_test!(config_16, 16, true, false, true, Negative);
configuration_test!(config_17, 17, true, false, true, Zero);
configuration_test!(config_18, 18, true, false, true, Positive);
configuration_test!(config_19, 19, true, true, false, Negative);
configuration_test!(config_20, 20, true, true, false, Zero);
configuration_test!(config_21, 21, true, true, false, Positive);
configuration_test!(config_22, 22, true, true, true, Negative);
configuration_test!(config_23, 23, true, true, true, Zero);
configuration_test!(config_24, 24, true, true, true, Positive);

fn assert_null_pointer_boundary_matches() {
    use std::os::unix::process::ExitStatusExt;

    fn probe(path: &Path) -> std::process::ExitStatus {
        Command::new(std::env::current_exe().expect("find current test executable"))
            .arg("--exact")
            .arg("differential_surface")
            .arg("--nocapture")
            .env("NULL_PROBE_LIBRARY", path)
            .status()
            .unwrap_or_else(|error| {
                panic!(
                    "failed to run null-pointer probe for {}: {error}",
                    path.display()
                )
            })
    }

    let c_status = probe(&c_library_path());
    let rust_status = probe(&rust_library_path());

    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "null pointer terminated C and Rust differently"
    );
    assert_eq!(
        c_status.signal(),
        Some(11),
        "C null-pointer baseline did not receive SIGSEGV"
    );
}

#[test]
fn differential_surface() {
    if let Some(path) = std::env::var_os("NULL_PROBE_LIBRARY") {
        unsafe {
            let library = Library::new(path).expect("load null-probe shared library");
            let print_foo: Symbol<PrintFoo> = library
                .get(b"print_foo")
                .expect("load null-probe print_foo");
            print_foo(ptr::null());
        }
        return;
    }

    config_01();
    config_02();
    config_03();
    config_04();
    config_05();
    config_06();
    config_07();
    config_08();
    config_09();
    config_10();
    config_11();
    config_12();
    config_13();
    config_14();
    config_15();
    config_16();
    config_17();
    config_18();
    config_19();
    config_20();
    config_21();
    config_22();
    config_23();
    config_24();
    assert_null_pointer_boundary_matches();
}
