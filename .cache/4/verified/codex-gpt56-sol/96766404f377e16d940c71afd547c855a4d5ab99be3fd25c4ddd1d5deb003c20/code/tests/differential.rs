use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_long, c_uint, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

type DriverFn = unsafe extern "C" fn(c_uint, c_uint, u8, c_int);
type PrintFooFn = unsafe extern "C" fn(*const RawFoo);

#[repr(C)]
#[derive(Clone, Copy)]
struct RawFoo {
    bits_and_padding: c_uint,
    z: c_int,
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    #[link_name = "stdout"]
    static mut C_STDOUT: *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fread(buffer: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fseek(stream: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn tmpfile() -> *mut c_void;
}

struct Api {
    _library: Library,
    driver: DriverFn,
    print_foo: PrintFooFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver: Symbol<DriverFn> =
            unsafe { library.get(b"driver\0") }.expect("missing driver export");
        let print_foo: Symbol<PrintFooFn> =
            unsafe { library.get(b"print_foo\0") }.expect("missing print_foo export");
        let driver = *driver;
        let print_foo = *print_foo;
        Self {
            _library: library,
            driver,
            print_foo,
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    let executable = env::current_exe().expect("test executable path");
    executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory")
        .join("libdriver.so")
}

fn assert_libraries_exist() {
    for path in [c_library_path(), rust_library_path()] {
        assert!(
            path.is_file(),
            "shared library does not exist: {}",
            path.display()
        );
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        let stream = tmpfile();
        assert!(!stream.is_null());
        let original_stdout = C_STDOUT;
        C_STDOUT = stream;

        call();
        assert_eq!(fflush(stream), 0);
        C_STDOUT = original_stdout;

        assert_eq!(fseek(stream, 0, 2), 0);
        let length = ftell(stream);
        assert!(length >= 0);
        assert_eq!(fseek(stream, 0, 0), 0);
        let mut output = vec![0_u8; length as usize];
        let bytes_read = fread(output.as_mut_ptr().cast(), 1, output.len(), stream);
        assert_eq!(bytes_read, output.len());
        assert_eq!(fclose(stream), 0);
        output
    }
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

fn varied_z(index: usize, state: &mut u64) -> i32 {
    match index % 8 {
        0 => i32::MIN,
        1 => i32::MAX,
        2 => -1,
        3 => 0,
        4 => 1,
        _ => next_random(state) as i32,
    }
}

fn compare_driver_row(c: &Api, rust: &Api, seed: u64, x_overflows: bool, y_overflows: bool, b: u8) {
    let mut state = seed;
    let mut inputs = Vec::with_capacity(256);
    for index in 0..256 {
        let random_x = next_random(&mut state);
        let random_y = next_random(&mut state);
        let x = if x_overflows {
            if index % 7 == 0 {
                u32::MAX
            } else {
                4 + random_x % (u32::MAX - 3)
            }
        } else {
            random_x & 3
        };
        let y = if y_overflows {
            if index % 7 == 0 {
                u32::MAX
            } else {
                8 + random_y % (u32::MAX - 7)
            }
        } else {
            random_y & 7
        };
        inputs.push((x, y, b, varied_z(index, &mut state)));
    }

    let c_output = capture_stdout(|| unsafe {
        for &(x, y, b, z) in &inputs {
            (c.driver)(x, y, b, z);
        }
    });
    let rust_output = capture_stdout(|| unsafe {
        for &(x, y, b, z) in &inputs {
            (rust.driver)(x, y, b, z);
        }
    });
    assert_eq!(c_output, rust_output);
}

fn compare_print_foo_row(c: &Api, rust: &Api, seed: u64, arbitrary_padding: bool) {
    let mut state = seed;
    let mut inputs = Vec::with_capacity(256);
    for index in 0..256 {
        let fields = next_random(&mut state) & 0x3f;
        let padding = if arbitrary_padding {
            next_random(&mut state) & !0x3f
        } else {
            0
        };
        inputs.push(RawFoo {
            bits_and_padding: fields | padding,
            z: varied_z(index, &mut state),
        });
    }

    let c_output = capture_stdout(|| unsafe {
        for input in &inputs {
            (c.print_foo)(input);
        }
    });
    let rust_output = capture_stdout(|| unsafe {
        for input in &inputs {
            (rust.print_foo)(input);
        }
    });
    assert_eq!(c_output, rust_output);
}

fn load_apis() -> (Api, Api) {
    assert_libraries_exist();
    (unsafe { Api::load(&c_library_path()) }, unsafe {
        Api::load(&rust_library_path())
    })
}

#[test]
fn v1_direct_packed_struct_matches() {
    let (c, rust) = load_apis();
    compare_print_foo_row(&c, &rust, 0x4d59_5df4_d0f3_3173, false);
}

#[test]
fn v2_direct_struct_with_ignored_bits_matches() {
    let (c, rust) = load_apis();
    compare_print_foo_row(&c, &rust, 0x8f68_6d72_3679_6c29, true);
}

#[test]
fn v3_in_range_false_matches() {
    let (c, rust) = load_apis();
    compare_driver_row(&c, &rust, 0x243f_6a88_85a3_08d3, false, false, 0);
}

#[test]
fn v4_in_range_true_matches() {
    let (c, rust) = load_apis();
    compare_driver_row(&c, &rust, 0x1319_8a2e_0370_7344, false, false, 1);
}

#[test]
fn v5_x_truncates_false_matches() {
    let (c, rust) = load_apis();
    compare_driver_row(&c, &rust, 0xa409_3822_299f_31d0, true, false, 0);
}

#[test]
fn v6_x_truncates_true_matches() {
    let (c, rust) = load_apis();
    compare_driver_row(&c, &rust, 0x082e_fa98_ec4e_6c89, true, false, 1);
}

#[test]
fn v7_y_truncates_false_matches() {
    let (c, rust) = load_apis();
    compare_driver_row(&c, &rust, 0x4528_21e6_38d0_1377, false, true, 0);
}

#[test]
fn v8_y_truncates_true_matches() {
    let (c, rust) = load_apis();
    compare_driver_row(&c, &rust, 0xbe54_66cf_34e9_0c6c, false, true, 1);
}

#[test]
fn v9_both_truncate_false_matches() {
    let (c, rust) = load_apis();
    compare_driver_row(&c, &rust, 0xc0ac_29b7_c97c_50dd, true, true, 0);
}

#[test]
fn v10_both_truncate_true_matches() {
    let (c, rust) = load_apis();
    compare_driver_row(&c, &rust, 0x3f84_d5b5_b547_0917, true, true, 1);
}

fn compare_noncanonical_bool(b: u8) {
    let (c, rust) = load_apis();
    let c_output = capture_stdout(|| unsafe {
        (c.driver)(u32::MAX, u32::MAX, b, i32::MIN);
    });
    let rust_output = capture_stdout(|| unsafe {
        (rust.driver)(u32::MAX, u32::MAX, b, i32::MIN);
    });
    assert_eq!(c_output, rust_output, "raw bool byte {b}");
}

#[test]
fn g2_bool_one_past_range_matches() {
    compare_noncanonical_bool(2);
}

#[test]
fn g3_bool_max_raw_byte_matches() {
    compare_noncanonical_bool(255);
}

#[test]
fn null_print_foo_probe() {
    let Some(which) = env::var_os("DRIVER_NULL_PROBE") else {
        return;
    };
    let path = if which == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };
    unsafe {
        (api.print_foo)(std::ptr::null());
    }
}

#[test]
fn null_print_foo_boundary_matches() {
    assert_libraries_exist();
    let executable = env::current_exe().expect("test executable path");
    let run_probe = |which: &str| {
        Command::new(&executable)
            .args(["--exact", "null_print_foo_probe", "--nocapture"])
            .env("DRIVER_NULL_PROBE", which)
            .status()
            .unwrap_or_else(|error| panic!("failed to run {which} null probe: {error}"))
    };

    let c_status = run_probe("c");
    let rust_status = run_probe("rust");
    assert_eq!(c_status.signal(), Some(11), "C status: {c_status}");
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "Rust status: {rust_status}"
    );
}

#[test]
fn dynamic_symbol_sets_cover_c_api() {
    assert_libraries_exist();
    let c_symbols = dynamic_defined_symbols(&c_library_path());
    let rust_symbols = dynamic_defined_symbols(&rust_library_path());
    let missing: Vec<_> = c_symbols
        .iter()
        .filter(|symbol| !rust_symbols.contains(symbol))
        .collect();
    assert!(missing.is_empty(), "Rust is missing C symbols: {missing:?}");
}

fn dynamic_defined_symbols(path: &Path) -> Vec<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap_or_else(|error| panic!("failed to inspect {}: {error}", path.display()));
    assert!(
        output.status.success(),
        "nm failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("nm output is UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn source_artifacts_exist() {
    for artifact in ["SYMBOLS.md", "ERRORS.md", "CONFIGS.md"] {
        let contents = fs::read_to_string(crate_root().join(artifact))
            .unwrap_or_else(|error| panic!("failed to read {artifact}: {error}"));
        assert!(!contents.is_empty(), "{artifact} is empty");
    }
}
