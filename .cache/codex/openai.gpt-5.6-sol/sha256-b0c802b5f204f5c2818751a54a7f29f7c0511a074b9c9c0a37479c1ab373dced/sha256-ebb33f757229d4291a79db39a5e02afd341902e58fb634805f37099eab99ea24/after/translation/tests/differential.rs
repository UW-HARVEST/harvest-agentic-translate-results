use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_long, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

#[repr(C)]
struct Matrix {
    matrix: *mut *mut c_int,
    width: c_int,
    height: c_int,
}

type AllocateMatrix = unsafe extern "C" fn(c_int, c_int) -> *mut Matrix;
type FreeMatrix = unsafe extern "C" fn(*mut Matrix);
type InitializeMatrix = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut Matrix;
type MultiplyMatrices = unsafe extern "C" fn(*mut Matrix, *mut Matrix) -> *mut Matrix;
type MatrixToString = unsafe extern "C" fn(*mut Matrix) -> *mut c_char;
type WriteToFile = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type Driver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

struct Api {
    _library: Library,
    allocate_matrix: AllocateMatrix,
    free_matrix: FreeMatrix,
    initialize_matrix_from_string: InitializeMatrix,
    multiply_matrices: MultiplyMatrices,
    matrix_to_string: MatrixToString,
    write_to_file: WriteToFile,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        unsafe {
            let library = Library::new(path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
            let allocate_matrix = *library.get(b"allocate_matrix\0").unwrap();
            let free_matrix = *library.get(b"free_matrix\0").unwrap();
            let initialize_matrix_from_string =
                *library.get(b"initialize_matrix_from_string\0").unwrap();
            let multiply_matrices = *library.get(b"multiply_matrices\0").unwrap();
            let matrix_to_string = *library.get(b"matrix_to_string\0").unwrap();
            let write_to_file = *library.get(b"write_to_file\0").unwrap();
            let driver = *library.get(b"driver\0").unwrap();
            Self {
                _library: library,
                allocate_matrix,
                free_matrix,
                initialize_matrix_from_string,
                multiply_matrices,
                matrix_to_string,
                write_to_file,
                driver,
            }
        }
    }
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

static PROCESS_STATE: Mutex<()> = Mutex::new(());

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    root().join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    // A cdylib is not emitted by `cargo test`; Phase D builds this exact
    // external-call artifact before running the integration suite.
    root().join("target/release/libdriver.so")
}

unsafe fn apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn range(&mut self, low: i32, high_inclusive: i32) -> i32 {
        low + (self.next_u32() % ((high_inclusive - low + 1) as u32)) as i32
    }
}

unsafe fn matrix_values(matrix: *mut Matrix) -> Vec<Vec<i32>> {
    unsafe {
        assert!(!matrix.is_null());
        let mut rows = Vec::new();
        for row in 0..(*matrix).height {
            let mut values = Vec::new();
            for column in 0..(*matrix).width {
                values.push(*(*(*matrix).matrix.add(row as usize)).add(column as usize));
            }
            rows.push(values);
        }
        rows
    }
}

unsafe fn set_matrix_values(matrix: *mut Matrix, values: &[Vec<i32>]) {
    unsafe {
        for (row, values) in values.iter().enumerate() {
            for (column, value) in values.iter().enumerate() {
                *(*(*matrix).matrix.add(row)).add(column) = *value;
            }
        }
    }
}

unsafe fn allocate_with_values(api: &Api, values: &[Vec<i32>], width: i32) -> *mut Matrix {
    unsafe {
        let matrix = (api.allocate_matrix)(width, values.len() as i32);
        assert!(!matrix.is_null());
        set_matrix_values(matrix, values);
        matrix
    }
}

unsafe fn owned_c_string(pointer: *mut c_char) -> Option<Vec<u8>> {
    unsafe {
        if pointer.is_null() {
            return None;
        }
        let bytes = CStr::from_ptr(pointer).to_bytes().to_vec();
        free(pointer.cast());
        Some(bytes)
    }
}

fn matrix_text(values: &[Vec<i32>]) -> CString {
    let mut text = String::new();
    for row in values {
        for (index, value) in row.iter().enumerate() {
            if index != 0 {
                text.push(' ');
            }
            text.push_str(&value.to_string());
        }
        text.push('\n');
    }
    CString::new(text).unwrap()
}

fn decorated_matrix_text(values: &[Vec<i32>]) -> Vec<u8> {
    let mut text = String::new();
    for (row_index, row) in values.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            if column_index != 0 {
                text.push(' ');
            }
            if row_index == 0 && column_index == 0 {
                text.push_str("nonnumeric");
            } else if row_index == 0 && column_index == 1 {
                text.push_str(&format!("{value}suffix"));
            } else {
                text.push_str(&value.to_string());
            }
        }
        text.push_str(" 999\n");
    }
    text.push_str("888 777\n");
    CString::new(text).unwrap().into_bytes_with_nul()
}

fn temp_dir(label: &str) -> PathBuf {
    let path = env::temp_dir().join(format!(
        "driver-differential-{}-{}-{}",
        label,
        std::process::id(),
        std::thread::current().name().unwrap_or("thread")
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn configs_01_to_05_allocate_and_free() {
    unsafe {
        let (c, rust) = apis();
        for &(width, height) in &[(0, 0), (0, 4), (4, 0), (1, 1), (7, 3)] {
            for _ in 0..32 {
                let c_matrix = (c.allocate_matrix)(width, height);
                let rust_matrix = (rust.allocate_matrix)(width, height);
                assert_eq!(c_matrix.is_null(), rust_matrix.is_null());
                assert!(!c_matrix.is_null());
                assert_eq!((*c_matrix).width, (*rust_matrix).width);
                assert_eq!((*c_matrix).height, (*rust_matrix).height);
                (c.free_matrix)(c_matrix);
                (rust.free_matrix)(rust_matrix);
            }
        }
    }
}

#[test]
fn configs_06_to_13_initialize_from_string() {
    unsafe {
        let (c, rust) = apis();
        let fixed = [
            (0, 0, "ignored"),
            (3, 0, "ignored"),
            (0, 3, "a\nb\nc"),
            (1, 1, "42"),
            (2, 2, "1 2\n3 4"),
            (2, 2, "1 2 99\n3 4 98\n7 8"),
            (2, 2, "  1   2  \n\n  3  4  "),
            (4, 1, "+12 -7 19suffix nope"),
        ];
        for &(width, height, input) in &fixed {
            let input = CString::new(input).unwrap();
            let c_matrix = (c.initialize_matrix_from_string)(input.as_ptr(), width, height);
            let rust_matrix = (rust.initialize_matrix_from_string)(input.as_ptr(), width, height);
            assert_eq!(c_matrix.is_null(), rust_matrix.is_null());
            assert!(!c_matrix.is_null(), "{width}x{height}: {input:?}");
            assert_eq!(matrix_values(c_matrix), matrix_values(rust_matrix));
            (c.free_matrix)(c_matrix);
            (rust.free_matrix)(rust_matrix);
        }

        let mut rng = Rng::new(0x88d0_0bad_f00d_1234);
        for _ in 0..256 {
            let height = rng.range(1, 6);
            let width = rng.range(1, 6);
            let values: Vec<Vec<i32>> = (0..height)
                .map(|_| (0..width).map(|_| rng.range(-100_000, 100_000)).collect())
                .collect();
            let input = matrix_text(&values);
            let c_matrix = (c.initialize_matrix_from_string)(input.as_ptr(), width, height);
            let rust_matrix = (rust.initialize_matrix_from_string)(input.as_ptr(), width, height);
            assert_eq!(matrix_values(c_matrix), values);
            assert_eq!(matrix_values(c_matrix), matrix_values(rust_matrix));
            (c.free_matrix)(c_matrix);
            (rust.free_matrix)(rust_matrix);
        }
    }
}

#[test]
fn configs_14_to_19_multiply() {
    unsafe {
        let (c, rust) = apis();
        let mut rng = Rng::new(0x1ced_cafe_5eed_9876);
        let shapes = [
            (1, 1, 1),
            (4, 1, 5),
            (3, 4, 2),
            (3, 0, 4),
            (0, 3, 4),
            (3, 4, 0),
        ];
        for &(height_a, inner, width_b) in &shapes {
            for _ in 0..64 {
                let a: Vec<Vec<i32>> = (0..height_a)
                    .map(|_| (0..inner).map(|_| rng.range(-100, 100)).collect())
                    .collect();
                let b: Vec<Vec<i32>> = (0..inner)
                    .map(|_| (0..width_b).map(|_| rng.range(-100, 100)).collect())
                    .collect();
                let c_a = allocate_with_values(&c, &a, inner);
                let c_b = allocate_with_values(&c, &b, width_b);
                let r_a = allocate_with_values(&rust, &a, inner);
                let r_b = allocate_with_values(&rust, &b, width_b);
                let c_result = (c.multiply_matrices)(c_a, c_b);
                let r_result = (rust.multiply_matrices)(r_a, r_b);
                assert_eq!(c_result.is_null(), r_result.is_null());
                assert!(!c_result.is_null());
                assert_eq!(matrix_values(c_result), matrix_values(r_result));
                for (api, first, second, result) in
                    [(&c, c_a, c_b, c_result), (&rust, r_a, r_b, r_result)]
                {
                    (api.free_matrix)(first);
                    (api.free_matrix)(second);
                    (api.free_matrix)(result);
                }
            }
        }
    }
}

#[test]
fn configs_20_to_25_matrix_to_string() {
    unsafe {
        let (c, rust) = apis();
        let fixed = [
            (vec![], 0),
            (vec![], 4),
            (vec![vec![], vec![], vec![]], 0),
            (vec![vec![0]], 1),
            (vec![vec![-7]], 1),
            (vec![vec![i32::MIN, i32::MAX]], 2),
            (vec![vec![1, 2, 3], vec![-4, 0, 6]], 3),
        ];
        for (values, width) in fixed {
            let c_matrix = allocate_with_values(&c, &values, width);
            let rust_matrix = allocate_with_values(&rust, &values, width);
            let c_text = owned_c_string((c.matrix_to_string)(c_matrix));
            let rust_text = owned_c_string((rust.matrix_to_string)(rust_matrix));
            assert_eq!(c_text, rust_text);
            (c.free_matrix)(c_matrix);
            (rust.free_matrix)(rust_matrix);
        }

        let mut rng = Rng::new(0xd1ff_3a11_5eed_0042);
        for _ in 0..256 {
            let height = rng.range(1, 7);
            let width = rng.range(1, 7);
            let values: Vec<Vec<i32>> = (0..height)
                .map(|_| {
                    (0..width)
                        .map(|_| rng.range(-1_000_000, 1_000_000))
                        .collect()
                })
                .collect();
            let c_matrix = allocate_with_values(&c, &values, width);
            let rust_matrix = allocate_with_values(&rust, &values, width);
            assert_eq!(
                owned_c_string((c.matrix_to_string)(c_matrix)),
                owned_c_string((rust.matrix_to_string)(rust_matrix))
            );
            (c.free_matrix)(c_matrix);
            (rust.free_matrix)(rust_matrix);
        }
    }
}

#[test]
fn configs_26_to_28_write_to_file() {
    let _guard = PROCESS_STATE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = apis();
        let directory = temp_dir("write");
        for (index, initial, replacement) in [
            (0, None, ""),
            (1, None, "line one\nline two\n"),
            (2, Some("old bytes that must disappear"), "new"),
        ] {
            let c_path = directory.join(format!("c-{index}.txt"));
            let rust_path = directory.join(format!("rust-{index}.txt"));
            if let Some(initial) = initial {
                fs::write(&c_path, initial).unwrap();
                fs::write(&rust_path, initial).unwrap();
            }
            let c_path_string = CString::new(c_path.as_os_str().as_encoded_bytes()).unwrap();
            let rust_path_string = CString::new(rust_path.as_os_str().as_encoded_bytes()).unwrap();
            let replacement = CString::new(replacement).unwrap();
            assert_eq!(
                (c.write_to_file)(c_path_string.as_ptr(), replacement.as_ptr()),
                (rust.write_to_file)(rust_path_string.as_ptr(), replacement.as_ptr())
            );
            assert_eq!(fs::read(c_path).unwrap(), fs::read(rust_path).unwrap());
        }
        fs::remove_dir_all(directory).unwrap();
    }
}

unsafe fn run_driver_case(
    api: &Api,
    directory: &Path,
    a: &[Vec<i32>],
    width_a: i32,
    b: &[Vec<i32>],
    width_b: i32,
    decorate: bool,
) -> (i32, Vec<u8>) {
    unsafe {
        let original = env::current_dir().unwrap();
        env::set_current_dir(directory).unwrap();
        let a_text = if decorate {
            decorated_matrix_text(a)
        } else {
            matrix_text(a).into_bytes_with_nul()
        };
        let b_text = if decorate {
            decorated_matrix_text(b)
        } else {
            matrix_text(b).into_bytes_with_nul()
        };
        let result = (api.driver)(
            width_a,
            a.len() as i32,
            a_text.as_ptr().cast(),
            width_b,
            b.len() as i32,
            b_text.as_ptr().cast(),
        );
        let output = fs::read("matrix.txt").unwrap_or_default();
        env::set_current_dir(original).unwrap();
        (result, output)
    }
}

#[test]
fn configs_29_to_33_driver_end_to_end() {
    let _guard = PROCESS_STATE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = apis();
        let c_directory = temp_dir("driver-c");
        let rust_directory = temp_dir("driver-rust");
        fs::write(c_directory.join("matrix.txt"), "old").unwrap();
        fs::write(rust_directory.join("matrix.txt"), "old").unwrap();

        let mut rng = Rng::new(0x600d_f00d_1234_5678);
        for iteration in 0..256 {
            let height_a = if iteration == 0 { 1 } else { rng.range(1, 5) };
            let inner = if iteration < 2 { 1 } else { rng.range(1, 5) };
            let width_b = if iteration == 0 { 1 } else { rng.range(1, 5) };
            let a: Vec<Vec<i32>> = (0..height_a)
                .map(|_| (0..inner).map(|_| rng.range(-100, 100)).collect())
                .collect();
            let b: Vec<Vec<i32>> = (0..inner)
                .map(|_| (0..width_b).map(|_| rng.range(-100, 100)).collect())
                .collect();
            let c_result = run_driver_case(
                &c,
                &c_directory,
                &a,
                inner,
                &b,
                width_b,
                iteration % 11 == 0,
            );
            let rust_result = run_driver_case(
                &rust,
                &rust_directory,
                &a,
                inner,
                &b,
                width_b,
                iteration % 11 == 0,
            );
            assert_eq!(c_result, rust_result);
        }

        fs::remove_dir_all(c_directory).unwrap();
        fs::remove_dir_all(rust_directory).unwrap();
    }
}

#[test]
fn errors_04_06_to_17_and_19_direct() {
    let _guard = PROCESS_STATE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = apis();

        (c.free_matrix)(ptr::null_mut());
        (rust.free_matrix)(ptr::null_mut());

        for (input, width, height) in [("1 2", 2, 2), ("1\n2", 2, 2)] {
            let input = CString::new(input).unwrap();
            let c_result = (c.initialize_matrix_from_string)(input.as_ptr(), width, height);
            let rust_result = (rust.initialize_matrix_from_string)(input.as_ptr(), width, height);
            assert_eq!(c_result.is_null(), rust_result.is_null());
            assert!(c_result.is_null());
        }

        let c_a = (c.allocate_matrix)(2, 1);
        let c_b = (c.allocate_matrix)(1, 1);
        let r_a = (rust.allocate_matrix)(2, 1);
        let r_b = (rust.allocate_matrix)(1, 1);
        assert_eq!(
            (c.multiply_matrices)(c_a, c_b).is_null(),
            (rust.multiply_matrices)(r_a, r_b).is_null()
        );
        (c.free_matrix)(c_a);
        (c.free_matrix)(c_b);
        (rust.free_matrix)(r_a);
        (rust.free_matrix)(r_b);

        assert_eq!(
            (c.matrix_to_string)(ptr::null_mut()).is_null(),
            (rust.matrix_to_string)(ptr::null_mut()).is_null()
        );

        let filename = CString::new("/tmp/unused").unwrap();
        assert_eq!(
            (c.write_to_file)(filename.as_ptr(), ptr::null()),
            (rust.write_to_file)(filename.as_ptr(), ptr::null())
        );

        let missing = CString::new("/definitely/missing/driver-differential/file").unwrap();
        let content = CString::new("content").unwrap();
        assert_eq!(
            (c.write_to_file)(missing.as_ptr(), content.as_ptr()),
            (rust.write_to_file)(missing.as_ptr(), content.as_ptr())
        );

        if Path::new("/dev/full").exists() {
            let full = CString::new("/dev/full").unwrap();
            let short = CString::new("x").unwrap();
            assert_eq!(
                (c.write_to_file)(full.as_ptr(), short.as_ptr()),
                (rust.write_to_file)(full.as_ptr(), short.as_ptr())
            );
            let long = CString::new(vec![b'x'; 32 * 1024]).unwrap();
            assert_eq!(
                (c.write_to_file)(full.as_ptr(), long.as_ptr()),
                (rust.write_to_file)(full.as_ptr(), long.as_ptr())
            );
        }

        let bad_a = CString::new("").unwrap();
        let good = CString::new("1").unwrap();
        assert_eq!(
            (c.driver)(1, 1, bad_a.as_ptr(), 1, 1, good.as_ptr()),
            (rust.driver)(1, 1, bad_a.as_ptr(), 1, 1, good.as_ptr())
        );
        assert_eq!(
            (c.driver)(1, 1, good.as_ptr(), 1, 1, bad_a.as_ptr()),
            (rust.driver)(1, 1, good.as_ptr(), 1, 1, bad_a.as_ptr())
        );
        assert_eq!(
            (c.driver)(
                2,
                1,
                CString::new("1 2").unwrap().as_ptr(),
                1,
                1,
                good.as_ptr()
            ),
            (rust.driver)(
                2,
                1,
                CString::new("1 2").unwrap().as_ptr(),
                1,
                1,
                good.as_ptr()
            )
        );

        let original = env::current_dir().unwrap();
        let c_fail = temp_dir("driver-write-c");
        let r_fail = temp_dir("driver-write-r");
        fs::create_dir(c_fail.join("matrix.txt")).unwrap();
        fs::create_dir(r_fail.join("matrix.txt")).unwrap();
        env::set_current_dir(&c_fail).unwrap();
        let c_result = (c.driver)(1, 1, good.as_ptr(), 1, 1, good.as_ptr());
        env::set_current_dir(&r_fail).unwrap();
        let rust_result = (rust.driver)(1, 1, good.as_ptr(), 1, 1, good.as_ptr());
        env::set_current_dir(original).unwrap();
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, 1);
        fs::remove_dir_all(c_fail).unwrap();
        fs::remove_dir_all(r_fail).unwrap();
    }
}

fn compile_fault_shim() -> PathBuf {
    let output = root().join("target/fail_alloc.so");
    let status = Command::new("cc")
        .current_dir(root())
        .args(["-shared", "-fPIC", "tests/fail_alloc.c", "-ldl", "-o"])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    output
}

#[test]
fn errors_01_02_03_05_10_18_allocation_failures() {
    if env::var_os("DRIVER_FAULT_CHILD").is_some() {
        return;
    }
    let _guard = PROCESS_STATE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let shim = compile_fault_shim();
    let status = Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("allocation_failure_child")
        .arg("--ignored")
        .arg("--nocapture")
        .env("LD_PRELOAD", shim)
        .env("DRIVER_FAULT_CHILD", "1")
        .status()
        .unwrap();
    assert!(status.success(), "fault-injection child failed: {status}");
}

#[test]
#[ignore]
fn allocation_failure_child() {
    if env::var_os("DRIVER_FAULT_CHILD").is_none() {
        return;
    }
    unsafe {
        let shim = Library::new(root().join("target/fail_alloc.so")).unwrap();
        let arm: unsafe extern "C" fn(c_long) = *shim.get(b"fail_alloc_arm\0").unwrap();
        let disarm: unsafe extern "C" fn() = *shim.get(b"fail_alloc_disarm\0").unwrap();
        let (c, rust) = apis();
        let input = CString::new("1").unwrap();
        let one = CString::new("1").unwrap();

        for api in [&c, &rust] {
            for failure_index in 0..=2 {
                arm(failure_index);
                let result = (api.allocate_matrix)(1, 1);
                disarm();
                assert!(result.is_null(), "allocation index {failure_index}");
            }

            arm(3);
            let result = (api.initialize_matrix_from_string)(input.as_ptr(), 1, 1);
            disarm();
            assert!(result.is_null());

            let matrix = (api.allocate_matrix)(1, 1);
            assert!(!matrix.is_null());
            arm(0);
            let result = (api.matrix_to_string)(matrix);
            disarm();
            assert!(result.is_null());
            (api.free_matrix)(matrix);

            // Each strdup is observed once at the interposed strdup wrapper
            // and once at its underlying malloc: 5 + 5 input allocations,
            // then 3 result-matrix allocations, then the string allocation.
            arm(13);
            let result = (api.driver)(1, 1, one.as_ptr(), 1, 1, one.as_ptr());
            disarm();
            assert_eq!(result, 1);
        }
    }
}

fn child_status(library: &str, case: &str) -> std::process::ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("unchecked_null_child")
        .arg("--ignored")
        .arg("--nocapture")
        .env("DRIVER_NULL_CHILD_LIBRARY", library)
        .env("DRIVER_NULL_CHILD_CASE", case)
        .status()
        .unwrap()
}

#[test]
fn generic_null_pointer_and_dimension_boundaries() {
    if env::var_os("DRIVER_NULL_CHILD_LIBRARY").is_some() {
        return;
    }
    let _guard = PROCESS_STATE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for case in [
        "initialize_input",
        "multiply_a",
        "multiply_b",
        "write_filename",
        "driver_a",
        "driver_b",
    ] {
        let c = child_status("c", case);
        let rust = child_status("rust", case);
        assert_eq!(c.code(), rust.code(), "{case}: exit codes differ");
        assert_eq!(c.signal(), rust.signal(), "{case}: signals differ");
    }

    unsafe {
        let (c, rust) = apis();
        for api in [&c, &rust] {
            let matrix = (api.allocate_matrix)(i32::MAX, 0);
            assert!(!matrix.is_null());
            assert_eq!((*matrix).width, i32::MAX);
            (api.free_matrix)(matrix);

            let input = CString::new("ignored").unwrap();
            let matrix = (api.initialize_matrix_from_string)(input.as_ptr(), i32::MAX, 0);
            assert!(!matrix.is_null());
            (api.free_matrix)(matrix);
        }
    }
}

#[test]
#[ignore]
fn unchecked_null_child() {
    let Some(library) = env::var_os("DRIVER_NULL_CHILD_LIBRARY") else {
        return;
    };
    let case = env::var("DRIVER_NULL_CHILD_CASE").unwrap();
    unsafe {
        let api = if library == "c" {
            Api::load(&c_library_path())
        } else {
            Api::load(&rust_library_path())
        };
        let one = CString::new("1").unwrap();
        match case.as_str() {
            "initialize_input" => {
                let _ = (api.initialize_matrix_from_string)(ptr::null(), 1, 1);
            }
            "multiply_a" => {
                let matrix = (api.allocate_matrix)(1, 1);
                let _ = (api.multiply_matrices)(ptr::null_mut(), matrix);
            }
            "multiply_b" => {
                let matrix = (api.allocate_matrix)(1, 1);
                let _ = (api.multiply_matrices)(matrix, ptr::null_mut());
            }
            "write_filename" => {
                let _ = (api.write_to_file)(ptr::null(), one.as_ptr());
            }
            "driver_a" => {
                let _ = (api.driver)(1, 1, ptr::null(), 1, 1, one.as_ptr());
            }
            "driver_b" => {
                let _ = (api.driver)(1, 1, one.as_ptr(), 1, 1, ptr::null());
            }
            _ => panic!("unknown null case"),
        }
    }
}
