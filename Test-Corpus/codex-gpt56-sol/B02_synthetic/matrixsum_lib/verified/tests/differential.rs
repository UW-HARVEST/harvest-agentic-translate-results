use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::{Mutex, OnceLock};

static TEST_LOCK: Mutex<()> = Mutex::new(());
static RUST_LIBRARY: OnceLock<PathBuf> = OnceLock::new();

const DEFAULT_MATRIX: Matrix = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

type Matrix = [[c_int; 4]; 3];

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

type InitArray = unsafe extern "C" fn(usize) -> *mut DynamicArray;
type ExpandArray = unsafe extern "C" fn(*mut DynamicArray) -> c_int;
type AddElement = unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int;
type FreeArray = unsafe extern "C" fn(*mut DynamicArray);
type ProcessFlags = unsafe extern "C" fn(c_int) -> c_int;
type CalculateMatrixChecksum = unsafe extern "C" fn() -> c_int;
type Matrixsum = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    init_array: InitArray,
    expand_array: ExpandArray,
    add_element: AddElement,
    free_array: FreeArray,
    process_flags: ProcessFlags,
    calculate_matrix_checksum: CalculateMatrixChecksum,
    matrixsum: Matrixsum,
    matrix: *mut Matrix,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        let init_array = unsafe { *library.get(b"init_array\0").unwrap() };
        let expand_array = unsafe { *library.get(b"expand_array\0").unwrap() };
        let add_element = unsafe { *library.get(b"add_element\0").unwrap() };
        let free_array = unsafe { *library.get(b"free_array\0").unwrap() };
        let process_flags = unsafe { *library.get(b"process_flags\0").unwrap() };
        let calculate_matrix_checksum =
            unsafe { *library.get(b"calculate_matrix_checksum\0").unwrap() };
        let matrixsum = unsafe { *library.get(b"matrixsum\0").unwrap() };
        let matrix = unsafe { *library.get::<*mut Matrix>(b"matrix\0").unwrap() };

        Self {
            _library: library,
            init_array,
            expand_array,
            add_element,
            free_array,
            process_flags,
            calculate_matrix_checksum,
            matrixsum,
            matrix,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + (self.next_u64() as usize % (end - start))
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    RUST_LIBRARY
        .get_or_init(|| {
            let executable = std::env::current_exe().expect("test executable path");
            let profile_dir = executable
                .parent()
                .and_then(Path::parent)
                .expect("Cargo profile directory");
            let library = profile_dir.join("libmatrixsum_lib.so");

            if !library.is_file() {
                let profile = profile_dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("Cargo profile name");
                let mut command = Command::new(env!("CARGO"));
                command
                    .current_dir(manifest_dir())
                    .args(["build", "--no-default-features"]);
                if profile == "release" {
                    command.arg("--release");
                }
                let status = command.status().expect("build Rust cdylib");
                assert!(status.success(), "Rust cdylib build failed");
            }

            assert!(
                library.is_file(),
                "missing Rust cdylib {}",
                library.display()
            );
            library
        })
        .clone()
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C library {}; build it with CMake first",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust cdylib {}",
        rust_path.display()
    );

    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

unsafe fn array_header(array: *mut DynamicArray) -> (usize, usize) {
    unsafe { ((*array).size, (*array).capacity) }
}

unsafe fn array_values(array: *mut DynamicArray) -> Vec<c_int> {
    let header = unsafe { &*array };
    unsafe { std::slice::from_raw_parts(header.data, header.size) }.to_vec()
}

unsafe fn assert_arrays_match(c_array: *mut DynamicArray, rust_array: *mut DynamicArray) {
    assert_eq!(c_array.is_null(), rust_array.is_null());
    if !c_array.is_null() {
        assert_eq!(unsafe { array_header(c_array) }, unsafe {
            array_header(rust_array)
        });
        assert_eq!(unsafe { array_values(c_array) }, unsafe {
            array_values(rust_array)
        });
    }
}

unsafe fn set_matrix(api: &Api, value: Matrix) {
    unsafe { ptr::write(api.matrix, value) };
}

unsafe fn get_matrix(api: &Api) -> Matrix {
    unsafe { ptr::read(api.matrix) }
}

#[test]
fn dynamic_array_valid_configurations() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x58a9_f431_74cd_1201);

    for capacity in [0, 1] {
        let c_array = unsafe { (c.init_array)(capacity) };
        let rust_array = unsafe { (rust.init_array)(capacity) };
        unsafe { assert_arrays_match(c_array, rust_array) };
        assert!(!c_array.is_null());
        assert_eq!(unsafe { array_header(c_array) }, (0, capacity));
        unsafe {
            (c.free_array)(c_array);
            (rust.free_array)(rust_array);
        }
    }

    for _ in 0..128 {
        let capacity = rng.range(2, 65);
        let c_array = unsafe { (c.init_array)(capacity) };
        let rust_array = unsafe { (rust.init_array)(capacity) };
        unsafe { assert_arrays_match(c_array, rust_array) };
        assert_eq!(unsafe { array_header(c_array) }, (0, capacity));
        unsafe {
            (c.free_array)(c_array);
            (rust.free_array)(rust_array);
        }
    }

    let c_array = unsafe { (c.init_array)(1) };
    let rust_array = unsafe { (rust.init_array)(1) };
    assert_eq!(unsafe { (c.expand_array)(c_array) }, unsafe {
        (rust.expand_array)(rust_array)
    });
    unsafe { assert_arrays_match(c_array, rust_array) };
    assert_eq!(unsafe { array_header(c_array) }, (0, 2));
    unsafe {
        (c.free_array)(c_array);
        (rust.free_array)(rust_array);
    }

    let c_array = unsafe { (c.init_array)(2) };
    let rust_array = unsafe { (rust.init_array)(2) };
    for value in [rng.next_i32(), rng.next_i32()] {
        assert_eq!(unsafe { (c.add_element)(c_array, value) }, unsafe {
            (rust.add_element)(rust_array, value)
        });
    }
    assert_eq!(unsafe { (c.expand_array)(c_array) }, unsafe {
        (rust.expand_array)(rust_array)
    });
    unsafe { assert_arrays_match(c_array, rust_array) };
    assert_eq!(unsafe { array_header(c_array) }, (2, 4));
    unsafe {
        (c.free_array)(c_array);
        (rust.free_array)(rust_array);
    }

    for initial_capacity in 1..=8 {
        let c_array = unsafe { (c.init_array)(initial_capacity) };
        let rust_array = unsafe { (rust.init_array)(initial_capacity) };
        let element_count = 96 + rng.range(0, 161);

        for index in 0..element_count {
            let value = match index {
                0 => 0,
                1 => c_int::MIN,
                2 => c_int::MAX,
                _ => rng.next_i32(),
            };
            let c_result = unsafe { (c.add_element)(c_array, value) };
            let rust_result = unsafe { (rust.add_element)(rust_array, value) };
            assert_eq!(c_result, rust_result);
            unsafe { assert_arrays_match(c_array, rust_array) };
        }

        unsafe {
            (c.free_array)(c_array);
            (rust.free_array)(rust_array);
        }
    }

    for populated in [false, true] {
        let c_array = unsafe { (c.init_array)(2) };
        let rust_array = unsafe { (rust.init_array)(2) };
        if populated {
            unsafe {
                (c.add_element)(c_array, 17);
                (rust.add_element)(rust_array, 17);
            }
        }
        unsafe {
            (c.free_array)(c_array);
            (rust.free_array)(rust_array);
        }
    }
}

#[test]
fn process_flags_all_configurations() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x913c_7a55_d004_8e27);

    for low_nibble in 0..=0x0f {
        for _ in 0..512 {
            let flags = (rng.next_i32() & !0x0f) | low_nibble;
            assert_eq!(unsafe { (c.process_flags)(flags) }, unsafe {
                (rust.process_flags)(flags)
            });
        }
    }
}

#[test]
fn matrix_object_and_checksum_configurations() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x2d90_671a_c44f_8bb3);

    unsafe {
        set_matrix(&c, DEFAULT_MATRIX);
        set_matrix(&rust, DEFAULT_MATRIX);
    }
    assert_eq!(unsafe { get_matrix(&c) }, unsafe { get_matrix(&rust) });
    assert_eq!(unsafe { (c.calculate_matrix_checksum)() }, unsafe {
        (rust.calculate_matrix_checksum)()
    });

    for _ in 0..512 {
        let mut matrix = [[0; 4]; 3];
        for row in &mut matrix {
            for element in row {
                *element = rng.next_i32();
            }
        }
        unsafe {
            set_matrix(&c, matrix);
            set_matrix(&rust, matrix);
        }
        assert_eq!(unsafe { get_matrix(&c) }, unsafe { get_matrix(&rust) });
        assert_eq!(unsafe { (c.calculate_matrix_checksum)() }, unsafe {
            (rust.calculate_matrix_checksum)()
        });
    }

    unsafe {
        set_matrix(&c, DEFAULT_MATRIX);
        set_matrix(&rust, DEFAULT_MATRIX);
    }
}

#[test]
fn matrixsum_all_zero_nonzero_configurations() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xb355_21a0_ee7c_491d);

    unsafe {
        set_matrix(&c, DEFAULT_MATRIX);
        set_matrix(&rust, DEFAULT_MATRIX);
    }

    for nonzero_mask in 0..=0x0f {
        for iteration in 0..512 {
            let mut params = [0; 4];
            for (index, param) in params.iter_mut().enumerate() {
                if nonzero_mask & (1 << index) != 0 {
                    *param = match iteration {
                        0 => 1,
                        1 => -1,
                        2 => c_int::MAX,
                        3 => c_int::MIN,
                        _ => {
                            let random = rng.next_i32();
                            if random == 0 { 1 } else { random }
                        }
                    };
                }
            }

            let c_result = unsafe { (c.matrixsum)(params[0], params[1], params[2], params[3]) };
            let rust_result =
                unsafe { (rust.matrixsum)(params[0], params[1], params[2], params[3]) };
            assert_eq!(
                c_result, rust_result,
                "mask={nonzero_mask:#x}, params={params:?}"
            );
        }
    }
}

#[test]
fn null_and_length_boundaries() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let (c, rust) = load_apis();

    let c_result = unsafe { (c.expand_array)(ptr::null_mut()) };
    let rust_result = unsafe { (rust.expand_array)(ptr::null_mut()) };
    assert_eq!((c_result, rust_result), (0, 0));

    let c_result = unsafe { (c.add_element)(ptr::null_mut(), 123) };
    let rust_result = unsafe { (rust.add_element)(ptr::null_mut(), 123) };
    assert_eq!((c_result, rust_result), (0, 0));
    unsafe {
        (c.free_array)(ptr::null_mut());
        (rust.free_array)(ptr::null_mut());
    }

    for capacity in [0, usize::MAX, usize::MAX / size_of::<c_int>() + 1] {
        let c_array = unsafe { (c.init_array)(capacity) };
        let rust_array = unsafe { (rust.init_array)(capacity) };
        unsafe { assert_arrays_match(c_array, rust_array) };
        if capacity == usize::MAX {
            assert!(c_array.is_null() && rust_array.is_null());
        }
        if !c_array.is_null() {
            unsafe {
                (c.free_array)(c_array);
                (rust.free_array)(rust_array);
            }
        }
    }

    let c_array = unsafe { (c.init_array)(0) };
    let rust_array = unsafe { (rust.init_array)(0) };
    assert!(!c_array.is_null() && !rust_array.is_null());
    let c_result = unsafe { (c.expand_array)(c_array) };
    let rust_result = unsafe { (rust.expand_array)(rust_array) };
    assert_eq!((c_result, rust_result), (0, 0));
    assert_eq!(unsafe { array_header(c_array) }, unsafe {
        array_header(rust_array)
    });
    if c_result != 0 {
        unsafe {
            (c.free_array)(c_array);
            (rust.free_array)(rust_array);
        }
    }
}

fn allocator_interposer_path() -> PathBuf {
    let output_dir = manifest_dir().join("target/differential-support");
    std::fs::create_dir_all(&output_dir).expect("create allocator support directory");
    let output = output_dir.join("libfail_alloc.so");
    let status = Command::new("cc")
        .args(["-std=c11", "-shared", "-fPIC", "-o"])
        .arg(&output)
        .arg(manifest_dir().join("tests/support/fail_alloc.c"))
        .status()
        .expect("compile allocator interposer");
    assert!(status.success(), "allocator interposer compilation failed");
    output
}

unsafe fn run_allocation_failure_child() {
    type FailAfter = unsafe extern "C" fn(isize);

    let this = libloading::os::unix::Library::this();
    let fail_malloc: FailAfter = unsafe { *this.get(b"fail_malloc_after\0").unwrap() };
    let fail_realloc: FailAfter = unsafe { *this.get(b"fail_realloc_after\0").unwrap() };
    let (c, rust) = load_apis();

    unsafe { fail_malloc(0) };
    assert!(unsafe { (c.init_array)(2) }.is_null());
    unsafe { fail_malloc(0) };
    assert!(unsafe { (rust.init_array)(2) }.is_null());

    unsafe { fail_malloc(1) };
    assert!(unsafe { (c.init_array)(2) }.is_null());
    unsafe { fail_malloc(1) };
    assert!(unsafe { (rust.init_array)(2) }.is_null());

    let c_array = unsafe { (c.init_array)(2) };
    let rust_array = unsafe { (rust.init_array)(2) };
    unsafe {
        (c.add_element)(c_array, 81);
        (rust.add_element)(rust_array, 81);
    }
    let c_before = unsafe { (*c_array, array_values(c_array)) };
    let rust_before = unsafe { (*rust_array, array_values(rust_array)) };
    unsafe { fail_realloc(0) };
    let c_result = unsafe { (c.expand_array)(c_array) };
    unsafe { fail_realloc(0) };
    let rust_result = unsafe { (rust.expand_array)(rust_array) };
    assert_eq!((c_result, rust_result), (0, 0));
    assert_eq!(
        (c_before.0.size, c_before.0.capacity, &c_before.1),
        (rust_before.0.size, rust_before.0.capacity, &rust_before.1)
    );
    assert_eq!(unsafe { (*c_array).data }, c_before.0.data);
    assert_eq!(unsafe { (*rust_array).data }, rust_before.0.data);
    unsafe { assert_arrays_match(c_array, rust_array) };
    unsafe {
        (c.free_array)(c_array);
        (rust.free_array)(rust_array);
    }

    let c_array = unsafe { (c.init_array)(1) };
    let rust_array = unsafe { (rust.init_array)(1) };
    unsafe {
        (c.add_element)(c_array, 91);
        (rust.add_element)(rust_array, 91);
    }
    unsafe { fail_realloc(0) };
    let c_result = unsafe { (c.add_element)(c_array, 92) };
    unsafe { fail_realloc(0) };
    let rust_result = unsafe { (rust.add_element)(rust_array, 92) };
    assert_eq!((c_result, rust_result), (0, 0));
    unsafe { assert_arrays_match(c_array, rust_array) };
    assert_eq!(unsafe { array_header(c_array) }, (1, 1));
    assert_eq!(unsafe { array_values(c_array) }, [91]);
    unsafe {
        (c.free_array)(c_array);
        (rust.free_array)(rust_array);
    }

    for successful_allocations in [0, 1] {
        unsafe { fail_malloc(successful_allocations) };
        let c_result = unsafe { (c.matrixsum)(1, 2, 3, 4) };
        unsafe { fail_malloc(successful_allocations) };
        let rust_result = unsafe { (rust.matrixsum)(1, 2, 3, 4) };
        assert_eq!((c_result, rust_result), (-1, -1));
    }
}

#[test]
fn allocation_failure_differential() {
    if std::env::var_os("MATRIXSUM_ALLOC_FAILURE_CHILD").is_some() {
        unsafe { run_allocation_failure_child() };
        return;
    }

    let interposer = allocator_interposer_path();
    let executable = std::env::current_exe().expect("test executable path");
    let mut preload = interposer.into_os_string();
    if let Some(existing) = std::env::var_os("LD_PRELOAD") {
        preload.push(":");
        preload.push(existing);
    }

    let output = Command::new(executable)
        .args([
            "--exact",
            "allocation_failure_differential",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("MATRIXSUM_ALLOC_FAILURE_CHILD", "1")
        .env("LD_PRELOAD", preload)
        .output()
        .expect("run allocation-failure child");

    assert!(
        output.status.success(),
        "allocation-failure child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn defined_symbols(path: &Path) -> Vec<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap_or_else(|error| panic!("failed to inspect {}: {error}", path.display()));
    assert!(output.status.success());

    let mut symbols: Vec<_> = String::from_utf8(output.stdout)
        .expect("nm output is UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect();
    symbols.sort();
    symbols
}

#[test]
fn dynamic_symbol_parity() {
    let c_symbols = defined_symbols(&c_library_path());
    let rust_symbols = defined_symbols(&rust_library_path());
    assert_eq!(c_symbols, rust_symbols);
}
