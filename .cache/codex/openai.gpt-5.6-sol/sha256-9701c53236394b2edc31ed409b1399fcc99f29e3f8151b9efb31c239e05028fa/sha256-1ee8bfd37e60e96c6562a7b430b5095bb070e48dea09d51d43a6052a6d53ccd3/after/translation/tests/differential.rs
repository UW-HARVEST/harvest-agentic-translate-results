use libloading::Library;
use std::env;
use std::ffi::{c_int, c_void};
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};

type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type Driver = unsafe extern "C" fn(*const c_int, c_int);

unsafe extern "C" {
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct Api {
    _library: Library,
    fma_array: FmaArray,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let fma_array = unsafe { *library.get::<FmaArray>(b"fma_array\0").unwrap() };
        let driver = unsafe { *library.get::<Driver>(b"driver\0").unwrap() };
        Self {
            _library: library,
            fma_array,
            driver,
        }
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn length(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + self.next_u32() as usize % (maximum - minimum + 1)
    }

    fn safe_i32(&mut self) -> i32 {
        (self.next_u32() % 2_001) as i32 - 1_000
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library was not built")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libdriver.so")
        .canonicalize()
        .expect("Rust release shared library was not built")
}

fn bytes(values: &[i32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_ne_bytes())
        .collect()
}

fn assert_buffers_equal(row: usize, c: &[Vec<i32>], rust: &[Vec<i32>]) {
    assert_eq!(c.len(), rust.len());
    for (index, (c_buffer, rust_buffer)) in c.iter().zip(rust).enumerate() {
        assert_eq!(
            bytes(c_buffer),
            bytes(rust_buffer),
            "CONFIGS.md row {row}, physical buffer {index}"
        );
    }
}

fn execute_exact_alias(api: &Api, initial: &[Vec<i32>], map: [usize; 4]) -> Vec<Vec<i32>> {
    let mut buffers = initial.to_vec();
    let pointers: Vec<*mut i32> = buffers.iter_mut().map(Vec::as_mut_ptr).collect();
    unsafe {
        (api.fma_array)(
            pointers[map[0]],
            pointers[map[1]],
            pointers[map[2]],
            pointers[map[3]],
            buffers[0].len() as c_int,
        );
    }
    buffers
}

fn full_value(rng: &mut Rng, index: usize) -> i32 {
    match index % 23 {
        0 => i32::MIN,
        1 => i32::MAX,
        2 => -1,
        3 => 0,
        4 => 1,
        _ => rng.next_i32(),
    }
}

fn compare_exact_alias_row(row: usize, map: [usize; 4], safe: bool, seed: u64) {
    let c_api = unsafe { Api::load(&c_library_path()) };
    let rust_api = unsafe { Api::load(&rust_library_path()) };
    let mut rng = Rng(seed);

    for iteration in 0..128 {
        let len = rng.length(1, 64);
        let initial: Vec<Vec<i32>> = (0..4)
            .map(|buffer| {
                (0..len)
                    .map(|index| {
                        if safe {
                            rng.safe_i32()
                        } else {
                            full_value(&mut rng, iteration + buffer * 7 + index)
                        }
                    })
                    .collect()
            })
            .collect();
        let c = execute_exact_alias(&c_api, &initial, map);
        let rust = execute_exact_alias(&rust_api, &initial, map);
        assert_buffers_equal(row, &c, &rust);
    }
}

fn execute_shifted(api: &Api, initial: &[i32], len: usize, forward: bool) -> Vec<i32> {
    let mut buffer = initial.to_vec();
    let base = buffer.as_mut_ptr();
    let (out, input) = if forward {
        (unsafe { base.add(1) }, base)
    } else {
        (base, unsafe { base.add(1) })
    };
    unsafe {
        (api.fma_array)(out, input, input, input, len as c_int);
    }
    buffer
}

fn compare_shifted_row(row: usize, forward: bool, seed: u64) {
    let c_api = unsafe { Api::load(&c_library_path()) };
    let rust_api = unsafe { Api::load(&rust_library_path()) };
    let mut rng = Rng(seed);

    for iteration in 0..128 {
        let len = rng.length(2, 64);
        let initial: Vec<i32> = (0..=len)
            .map(|index| full_value(&mut rng, iteration + index))
            .collect();
        let c = execute_shifted(&c_api, &initial, len, forward);
        let rust = execute_shifted(&rust_api, &initial, len, forward);
        assert_eq!(bytes(&c), bytes(&rust), "CONFIGS.md row {row}");
    }
}

#[test]
fn valid_configuration_surface_matches() {
    let c_api = unsafe { Api::load(&c_library_path()) };
    let rust_api = unsafe { Api::load(&rust_library_path()) };

    // CONFIGS.md row 1.
    unsafe {
        (c_api.fma_array)(
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
        );
        (rust_api.fma_array)(
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0,
        );
    }

    compare_exact_alias_row(2, [0, 1, 2, 3], true, 0x0202_0202);
    compare_exact_alias_row(3, [0, 1, 2, 3], false, 0x0303_0303);
    compare_exact_alias_row(4, [0, 0, 1, 2], false, 0x0404_0404);
    compare_exact_alias_row(5, [0, 1, 0, 2], false, 0x0505_0505);
    compare_exact_alias_row(6, [0, 1, 2, 0], false, 0x0606_0606);
    compare_exact_alias_row(7, [0, 0, 0, 1], false, 0x0707_0707);
    compare_exact_alias_row(8, [0, 0, 1, 0], false, 0x0808_0808);
    compare_exact_alias_row(9, [0, 1, 0, 0], false, 0x0909_0909);
    compare_exact_alias_row(10, [0, 0, 0, 0], false, 0x1010_1010);
    compare_exact_alias_row(11, [0, 1, 1, 2], false, 0x1111_1111);
    compare_exact_alias_row(12, [0, 1, 2, 1], false, 0x1212_1212);
    compare_exact_alias_row(13, [0, 1, 2, 2], false, 0x1313_1313);
    compare_exact_alias_row(14, [0, 1, 1, 1], false, 0x1414_1414);
    compare_shifted_row(15, true, 0x1515_1515);
    compare_shifted_row(16, false, 0x1616_1616);

    // CONFIGS.md rows 17-19.
    let empty_cases = vec![Vec::new()];
    assert_eq!(
        run_driver_cases(&c_library_path(), &empty_cases),
        run_driver_cases(&rust_library_path(), &empty_cases),
        "CONFIGS.md row 17"
    );

    let mut scalar_rng = Rng(0x1818_1818);
    let scalar_cases: Vec<Vec<i32>> = (0..128)
        .map(|index| vec![full_value(&mut scalar_rng, index)])
        .collect();
    assert_eq!(
        run_driver_cases(&c_library_path(), &scalar_cases),
        run_driver_cases(&rust_library_path(), &scalar_cases),
        "CONFIGS.md row 18"
    );

    let mut array_rng = Rng(0x1919_1919);
    let array_cases: Vec<Vec<i32>> = (0..128)
        .map(|iteration| {
            let len = array_rng.length(2, 64);
            (0..len)
                .map(|index| full_value(&mut array_rng, iteration + index))
                .collect()
        })
        .collect();
    assert_eq!(
        run_driver_cases(&c_library_path(), &array_cases),
        run_driver_cases(&rust_library_path(), &array_cases),
        "CONFIGS.md row 19"
    );
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn run_driver_cases(library: &Path, cases: &[Vec<i32>]) -> Vec<u8> {
    let encoded = cases
        .iter()
        .map(|case| {
            case.iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("|");
    let output_path = env::temp_dir().join(format!(
        "driver-differential-{}-{}",
        process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let status = Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_driver_child")
        .arg("--nocapture")
        .env("DRIVER_OUTPUT_CHILD", "1")
        .env("DRIVER_LIBRARY", library)
        .env("DRIVER_CASES", encoded)
        .env("DRIVER_OUTPUT_PATH", &output_path)
        .status()
        .expect("failed to launch driver child");
    assert_eq!(status.code(), Some(42), "driver child failed: {status}");
    let output = fs::read(&output_path).expect("driver child did not produce output");
    fs::remove_file(output_path).unwrap();
    output
}

#[test]
fn ffi_driver_child() {
    if env::var_os("DRIVER_OUTPUT_CHILD").is_none() {
        return;
    }

    let output_path = PathBuf::from(env::var_os("DRIVER_OUTPUT_PATH").unwrap());
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)
        .unwrap();
    assert_eq!(unsafe { dup2(output.as_raw_fd(), 1) }, 1);

    let library = PathBuf::from(env::var_os("DRIVER_LIBRARY").unwrap());
    let api = unsafe { Api::load(&library) };
    let encoded = env::var("DRIVER_CASES").unwrap();
    for case in encoded.split('|') {
        let values: Vec<i32> = if case.is_empty() {
            Vec::new()
        } else {
            case.split(',')
                .map(|value| value.parse().unwrap())
                .collect()
        };
        let data = if values.is_empty() {
            std::ptr::null()
        } else {
            values.as_ptr()
        };
        unsafe {
            (api.driver)(data, values.len() as c_int);
        }
    }
    unsafe {
        fflush(std::ptr::null_mut());
    }
    process::exit(42);
}

#[derive(Debug, Eq, PartialEq)]
enum Outcome {
    Normal,
    Signal(i32),
    Exit(Option<i32>),
}

fn run_boundary_case(library: &Path, case: &str) -> Outcome {
    let status = Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_boundary_child")
        .arg("--nocapture")
        .env("DRIVER_BOUNDARY_CHILD", case)
        .env("DRIVER_LIBRARY", library)
        .status()
        .expect("failed to launch boundary child");
    if status.code() == Some(42) {
        Outcome::Normal
    } else if let Some(signal) = status.signal() {
        Outcome::Signal(signal)
    } else {
        Outcome::Exit(status.code())
    }
}

#[test]
fn generic_error_surface_matches() {
    let c_library = c_library_path();
    let rust_library = rust_library_path();
    let cases = [
        ("driver_null_0", Outcome::Normal),
        ("driver_null_1", Outcome::Signal(11)),
        ("driver_negative", Outcome::Signal(11)),
        ("driver_int_max", Outcome::Signal(11)),
        ("fma_null_0", Outcome::Normal),
        ("fma_null_negative", Outcome::Normal),
        ("fma_out_null", Outcome::Signal(11)),
        ("fma_mul1_null", Outcome::Signal(11)),
        ("fma_mul2_null", Outcome::Signal(11)),
        ("fma_add_null", Outcome::Signal(11)),
        ("fma_int_max", Outcome::Signal(11)),
    ];

    for (row, (case, expected)) in cases.into_iter().enumerate() {
        let c = run_boundary_case(&c_library, case);
        assert_eq!(
            c,
            expected,
            "unexpected C result for ERRORS.md row {}",
            row + 1
        );
        let rust = run_boundary_case(&rust_library, case);
        assert_eq!(
            rust,
            c,
            "C/Rust mismatch for ERRORS.md row {} ({case})",
            row + 1
        );
    }
}

#[test]
fn ffi_boundary_child() {
    let Some(case) = env::var_os("DRIVER_BOUNDARY_CHILD") else {
        return;
    };
    let library = PathBuf::from(env::var_os("DRIVER_LIBRARY").unwrap());
    let api = unsafe { Api::load(&library) };
    let mut value = 7_i32;
    let mut out = 0_i32;

    unsafe {
        match case.to_str().unwrap() {
            "driver_null_0" => (api.driver)(std::ptr::null(), 0),
            "driver_null_1" => (api.driver)(std::ptr::null(), 1),
            "driver_negative" => (api.driver)(&value, -1),
            "driver_int_max" => (api.driver)(&value, c_int::MAX),
            "fma_null_0" => (api.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
            ),
            "fma_null_negative" => (api.fma_array)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                -1,
            ),
            "fma_out_null" => (api.fma_array)(std::ptr::null_mut(), &value, &value, &value, 1),
            "fma_mul1_null" => (api.fma_array)(&mut out, std::ptr::null(), &value, &value, 1),
            "fma_mul2_null" => (api.fma_array)(&mut out, &value, std::ptr::null(), &value, 1),
            "fma_add_null" => (api.fma_array)(&mut out, &value, &value, std::ptr::null(), 1),
            "fma_int_max" => (api.fma_array)(&mut value, &value, &value, &value, c_int::MAX),
            unknown => panic!("unknown boundary case {unknown}"),
        }
    }
    process::exit(42);
}
