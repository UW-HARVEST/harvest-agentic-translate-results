use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

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
    initialize_matrix: InitializeMatrix,
    multiply_matrices: MultiplyMatrices,
    matrix_to_string: MatrixToString,
    write_to_file: WriteToFile,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        unsafe {
            let library = Library::new(path).unwrap();
            let allocate_matrix = *library.get(b"allocate_matrix\0").unwrap();
            let free_matrix = *library.get(b"free_matrix\0").unwrap();
            let initialize_matrix = *library.get(b"initialize_matrix_from_string\0").unwrap();
            let multiply_matrices = *library.get(b"multiply_matrices\0").unwrap();
            let matrix_to_string = *library.get(b"matrix_to_string\0").unwrap();
            let write_to_file = *library.get(b"write_to_file\0").unwrap();
            let driver = *library.get(b"driver\0").unwrap();
            Self {
                _library: library,
                allocate_matrix,
                free_matrix,
                initialize_matrix,
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

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    crate_root().join("../c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    crate_root().join("target/release/libdriver.so")
}

fn apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library().canonicalize().unwrap()),
            Api::load(&rust_library().canonicalize().unwrap()),
        )
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        low + (self.next_u32() % (high - low) as u32) as i32
    }
}

unsafe fn snapshot(matrix: *mut Matrix) -> (i32, i32, Vec<i32>) {
    unsafe {
        assert!(!matrix.is_null());
        let width = (*matrix).width;
        let height = (*matrix).height;
        let mut values = Vec::new();
        for row in 0..height {
            for column in 0..width {
                values.push(*(*(*matrix).matrix.add(row as usize)).add(column as usize));
            }
        }
        (width, height, values)
    }
}

unsafe fn initialize_snapshot(
    api: &Api,
    input: &str,
    width: i32,
    height: i32,
) -> Option<(i32, i32, Vec<i32>)> {
    unsafe {
        let input = CString::new(input).unwrap();
        let matrix = (api.initialize_matrix)(input.as_ptr(), width, height);
        if matrix.is_null() {
            return None;
        }
        let result = snapshot(matrix);
        (api.free_matrix)(matrix);
        Some(result)
    }
}

fn matrix_input(width: i32, height: i32, values: &[i32]) -> String {
    if height == 0 {
        return String::new();
    }
    if width == 0 {
        return (0..height).map(|_| "row").collect::<Vec<_>>().join("\n");
    }
    values
        .chunks(width as usize)
        .map(|row| {
            row.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

unsafe fn multiply_snapshot(
    api: &Api,
    a_shape: (i32, i32),
    a_values: &[i32],
    b_shape: (i32, i32),
    b_values: &[i32],
) -> Option<(i32, i32, Vec<i32>, Vec<u8>)> {
    unsafe {
        let a_input = CString::new(matrix_input(a_shape.0, a_shape.1, a_values)).unwrap();
        let b_input = CString::new(matrix_input(b_shape.0, b_shape.1, b_values)).unwrap();
        let a = (api.initialize_matrix)(a_input.as_ptr(), a_shape.0, a_shape.1);
        let b = (api.initialize_matrix)(b_input.as_ptr(), b_shape.0, b_shape.1);
        assert!(!a.is_null() && !b.is_null());
        let result = (api.multiply_matrices)(a, b);
        (api.free_matrix)(a);
        (api.free_matrix)(b);
        if result.is_null() {
            return None;
        }
        let matrix = snapshot(result);
        let text_pointer = (api.matrix_to_string)(result);
        assert!(!text_pointer.is_null());
        let text = CStr::from_ptr(text_pointer).to_bytes().to_vec();
        free(text_pointer.cast());
        (api.free_matrix)(result);
        Some((matrix.0, matrix.1, matrix.2, text))
    }
}

unsafe fn stringify(api: &Api, rows: &mut [Vec<i32>], width: i32, height: i32) -> Vec<u8> {
    unsafe {
        let mut pointers: Vec<*mut i32> = rows.iter_mut().map(|row| row.as_mut_ptr()).collect();
        let mut matrix = Matrix {
            matrix: pointers.as_mut_ptr(),
            width,
            height,
        };
        let output = (api.matrix_to_string)(&mut matrix);
        assert!(!output.is_null());
        let bytes = CStr::from_ptr(output).to_bytes().to_vec();
        free(output.cast());
        bytes
    }
}

fn temp_path(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "driver-diff-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        label
    ))
}

fn compare_write(c: &Api, rust: &Api, content: &[u8], preexisting: &[u8]) {
    let directory = temp_path("write");
    fs::create_dir(&directory).unwrap();
    let c_path = directory.join("c.txt");
    let rust_path = directory.join("rust.txt");
    fs::write(&c_path, preexisting).unwrap();
    fs::write(&rust_path, preexisting).unwrap();
    let c_path_string = CString::new(c_path.as_os_str().as_encoded_bytes()).unwrap();
    let rust_path_string = CString::new(rust_path.as_os_str().as_encoded_bytes()).unwrap();
    let content = CString::new(content).unwrap();
    let (c_result, rust_result) = unsafe {
        (
            (c.write_to_file)(c_path_string.as_ptr(), content.as_ptr()),
            (rust.write_to_file)(rust_path_string.as_ptr(), content.as_ptr()),
        )
    };
    assert_eq!(c_result, rust_result);
    assert_eq!(fs::read(c_path).unwrap(), fs::read(rust_path).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn compare_driver(
    c: &Api,
    rust: &Api,
    a_shape: (i32, i32),
    a_input: &str,
    b_shape: (i32, i32),
    b_input: &str,
) -> (i32, i32, Option<Vec<u8>>, Option<Vec<u8>>) {
    let _lock = CWD_LOCK.lock().unwrap();
    let old_directory = std::env::current_dir().unwrap();
    let directory = temp_path("driver");
    fs::create_dir(&directory).unwrap();
    std::env::set_current_dir(&directory).unwrap();
    let a = CString::new(a_input).unwrap();
    let b = CString::new(b_input).unwrap();
    let c_result = unsafe {
        (c.driver)(
            a_shape.0,
            a_shape.1,
            a.as_ptr(),
            b_shape.0,
            b_shape.1,
            b.as_ptr(),
        )
    };
    let c_output = fs::read("matrix.txt").ok();
    let _ = fs::remove_file("matrix.txt");
    let rust_result = unsafe {
        (rust.driver)(
            a_shape.0,
            a_shape.1,
            a.as_ptr(),
            b_shape.0,
            b_shape.1,
            b.as_ptr(),
        )
    };
    let rust_output = fs::read("matrix.txt").ok();
    std::env::set_current_dir(old_directory).unwrap();
    fs::remove_dir_all(directory).unwrap();
    (c_result, rust_result, c_output, rust_output)
}

#[test]
fn valid_configuration_surface() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x4d41_5452_4958);

    // CONFIGS 1-5: allocation/free dimension branches.
    for width in [0, 1, 7] {
        unsafe {
            let c_matrix = (c.allocate_matrix)(width, 0);
            let rust_matrix = (rust.allocate_matrix)(width, 0);
            assert_eq!(snapshot(c_matrix), snapshot(rust_matrix));
            (c.free_matrix)(c_matrix);
            (rust.free_matrix)(rust_matrix);
        }
    }
    for height in 1..=8 {
        unsafe {
            let c_matrix = (c.allocate_matrix)(0, height);
            let rust_matrix = (rust.allocate_matrix)(0, height);
            assert_eq!(snapshot(c_matrix), snapshot(rust_matrix));
            (c.free_matrix)(c_matrix);
            (rust.free_matrix)(rust_matrix);
        }
    }
    for _ in 0..32 {
        let width = rng.range(1, 8);
        let height = rng.range(1, 8);
        unsafe {
            let c_matrix = (c.allocate_matrix)(width, height);
            let rust_matrix = (rust.allocate_matrix)(width, height);
            assert_eq!(((*c_matrix).width, (*c_matrix).height), (width, height));
            assert_eq!(
                ((*c_matrix).width, (*c_matrix).height),
                ((*rust_matrix).width, (*rust_matrix).height)
            );
            (c.free_matrix)(c_matrix);
            (rust.free_matrix)(rust_matrix);
        }
    }
    unsafe {
        (c.free_matrix)(null_mut());
        (rust.free_matrix)(null_mut());
    }

    // CONFIGS 6-12: tokenization and atoi behavior.
    for input in ["", "ignored", "1 2\n3 4"] {
        for width in [0, 1, 4] {
            unsafe {
                assert_eq!(
                    initialize_snapshot(&c, input, width, 0),
                    initialize_snapshot(&rust, input, width, 0)
                );
            }
        }
    }
    for height in 1..=8 {
        let input = (0..height)
            .map(|index| format!("row{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        unsafe {
            assert_eq!(
                initialize_snapshot(&c, &input, 0, height),
                initialize_snapshot(&rust, &input, 0, height)
            );
        }
    }
    for _ in 0..32 {
        let value = rng.range(-100_000, 100_001);
        let input = value.to_string();
        unsafe {
            assert_eq!(
                initialize_snapshot(&c, &input, 1, 1),
                initialize_snapshot(&rust, &input, 1, 1)
            );
        }
    }
    for _ in 0..32 {
        let width = rng.range(1, 7);
        let height = rng.range(1, 7);
        let values = (0..width * height)
            .map(|_| rng.range(-10_000, 10_001))
            .collect::<Vec<_>>();
        let exact = matrix_input(width, height, &values);
        let extra = format!("{exact} 999\n777 888");
        unsafe {
            assert_eq!(
                initialize_snapshot(&c, &exact, width, height),
                initialize_snapshot(&rust, &exact, width, height)
            );
            assert_eq!(
                initialize_snapshot(&c, &extra, width, height),
                initialize_snapshot(&rust, &extra, width, height)
            );
        }
    }
    let collapsed = "  1   -2  3\n\n\n4  5     6  ";
    unsafe {
        assert_eq!(
            initialize_snapshot(&c, collapsed, 3, 2),
            initialize_snapshot(&rust, collapsed, 3, 2)
        );
    }
    for token in [
        "+17",
        "-0042",
        " 91",
        "123suffix",
        "nonnumeric",
        "--7",
        "0x20",
    ] {
        unsafe {
            assert_eq!(
                initialize_snapshot(&c, token, 1, 1),
                initialize_snapshot(&rust, token, 1, 1)
            );
        }
    }

    // CONFIGS 13-17: compatible multiplication shapes.
    for _ in 0..32 {
        let a = [rng.range(-1000, 1001)];
        let b = [rng.range(-1000, 1001)];
        unsafe {
            assert_eq!(
                multiply_snapshot(&c, (1, 1), &a, (1, 1), &b),
                multiply_snapshot(&rust, (1, 1), &a, (1, 1), &b)
            );
        }
    }
    for inner in [1, 2, 5] {
        for _ in 0..24 {
            let a_height = rng.range(1, 6);
            let b_width = rng.range(1, 6);
            let a = (0..a_height * inner)
                .map(|_| rng.range(-100, 101))
                .collect::<Vec<_>>();
            let b = (0..inner * b_width)
                .map(|_| rng.range(-100, 101))
                .collect::<Vec<_>>();
            unsafe {
                assert_eq!(
                    multiply_snapshot(&c, (inner, a_height), &a, (b_width, inner), &b),
                    multiply_snapshot(&rust, (inner, a_height), &a, (b_width, inner), &b)
                );
            }
        }
    }
    unsafe {
        assert_eq!(
            multiply_snapshot(&c, (0, 3), &[], (4, 0), &[]),
            multiply_snapshot(&rust, (0, 3), &[], (4, 0), &[])
        );
        assert_eq!(
            multiply_snapshot(&c, (1, 0), &[], (3, 1), &[1, 2, 3]),
            multiply_snapshot(&rust, (1, 0), &[], (3, 1), &[1, 2, 3])
        );
        assert_eq!(
            multiply_snapshot(&c, (1, 3), &[1, 2, 3], (0, 1), &[]),
            multiply_snapshot(&rust, (1, 3), &[1, 2, 3], (0, 1), &[])
        );
    }

    // CONFIGS 18-21: exact string bytes for shape/value branches.
    for height in 0..8 {
        let mut empty_rows = vec![Vec::new(); height as usize];
        unsafe {
            assert_eq!(
                stringify(&c, &mut empty_rows.clone(), 0, height),
                stringify(&rust, &mut empty_rows, 0, height)
            );
        }
    }
    for _ in 0..32 {
        let width = rng.range(1, 8);
        let height = rng.range(1, 8);
        let mut rows = (0..height)
            .map(|_| {
                (0..width)
                    .map(|_| rng.range(-1_000_000, 1_000_001))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        unsafe {
            assert_eq!(
                stringify(&c, &mut rows.clone(), width, height),
                stringify(&rust, &mut rows, width, height)
            );
        }
    }
    for value in [i32::MIN, i32::MAX] {
        let mut c_rows = vec![vec![value]];
        let mut rust_rows = c_rows.clone();
        unsafe {
            assert_eq!(
                stringify(&c, &mut c_rows, 1, 1),
                stringify(&rust, &mut rust_rows, 1, 1)
            );
        }
    }

    // CONFIGS 22-24: file creation, truncation, and byte content.
    compare_write(&c, &rust, b"", b"old trailing bytes");
    for _ in 0..24 {
        let content = format!("value={}\n", rng.range(-1_000_000, 1_000_001));
        compare_write(&c, &rust, content.as_bytes(), b"existing data is longer");
    }
    for rows in 1..=8 {
        let content = (0..rows)
            .map(|_| format!("{} {}", rng.range(-99, 100), rng.range(-99, 100)))
            .collect::<Vec<_>>()
            .join("\n");
        compare_write(&c, &rust, format!("{content}\n").as_bytes(), b"old");
    }

    // CONFIGS 25-28: composed driver paths.
    for _ in 0..24 {
        let a = rng.range(-1000, 1001).to_string();
        let b = rng.range(-1000, 1001).to_string();
        let result = compare_driver(&c, &rust, (1, 1), &a, (1, 1), &b);
        assert_eq!(result.0, result.1);
        assert_eq!(result.2, result.3);
    }
    for inner in [1, 2, 5] {
        for _ in 0..16 {
            let a_height = rng.range(1, 5);
            let b_width = rng.range(1, 5);
            let a_values = (0..a_height * inner)
                .map(|_| rng.range(-100, 101))
                .collect::<Vec<_>>();
            let b_values = (0..inner * b_width)
                .map(|_| rng.range(-100, 101))
                .collect::<Vec<_>>();
            let a = matrix_input(inner, a_height, &a_values);
            let b = matrix_input(b_width, inner, &b_values);
            let result = compare_driver(&c, &rust, (inner, a_height), &a, (b_width, inner), &b);
            assert_eq!(result.0, result.1);
            assert_eq!(result.2, result.3);
        }
    }
    let result = compare_driver(
        &c,
        &rust,
        (2, 2),
        "+2suffix  nonnumeric 999\n\n -003 4tail 888\n777 666",
        (1, 2),
        "5junk\n+6\n123",
    );
    assert_eq!(result.0, result.1);
    assert_eq!(result.2, result.3);
}

fn compile_interposer() -> PathBuf {
    static INTERPOSER: OnceLock<PathBuf> = OnceLock::new();
    INTERPOSER
        .get_or_init(|| {
            let output = temp_path("libfail_alloc.so");
            let status = Command::new("cc")
                .args(["-shared", "-fPIC", "-O2"])
                .arg(crate_root().join("tests/fail_alloc.c"))
                .args(["-ldl", "-o"])
                .arg(&output)
                .status()
                .unwrap();
            assert!(status.success());
            output
        })
        .clone()
}

fn run_fault_child(library: &Path, mode: &str, fail_malloc_at: Option<u32>, fail_strdup: bool) {
    let executable = std::env::current_exe().unwrap();
    let library = library.canonicalize().unwrap();
    let mut command = Command::new(executable);
    command
        .args(["--exact", "fault_injected_child", "--nocapture"])
        .env("LD_PRELOAD", compile_interposer())
        .env("DIFF_CHILD_MODE", mode)
        .env("DIFF_TARGET_DSO", &library)
        .env("DIFF_LIBRARY", &library);
    if let Some(index) = fail_malloc_at {
        command.env("DIFF_FAIL_MALLOC_AT", index.to_string());
    }
    if fail_strdup {
        command.env("DIFF_FAIL_STRDUP", "1");
    }
    let status = command.status().unwrap();
    assert!(status.success(), "fault child failed for {mode}: {status}");
}

#[test]
fn fault_injected_child() {
    let Ok(mode) = std::env::var("DIFF_CHILD_MODE") else {
        return;
    };
    let library = PathBuf::from(std::env::var_os("DIFF_LIBRARY").unwrap());
    let api = unsafe { Api::load(&library) };
    unsafe {
        match mode.as_str() {
            "allocate" => assert!((api.allocate_matrix)(1, 1).is_null()),
            "initialize" => {
                assert!((api.initialize_matrix)(c"1".as_ptr(), 1, 1).is_null())
            }
            "stringify" => {
                let mut value = 1;
                let mut row = &mut value as *mut i32;
                let mut matrix = Matrix {
                    matrix: &mut row,
                    width: 1,
                    height: 1,
                };
                assert!((api.matrix_to_string)(&mut matrix).is_null());
            }
            "driver_stringify" => {
                assert_eq!((api.driver)(1, 1, c"2".as_ptr(), 1, 1, c"3".as_ptr()), 1);
            }
            _ => panic!("unknown child mode {mode}"),
        }
    }
}

#[test]
fn error_surface() {
    let (c, rust) = apis();

    // ERRORS 1-4 and 9: deterministic allocator/strdup fault injection.
    for library in [c_library(), rust_library()] {
        run_fault_child(&library, "allocate", Some(1), false);
        run_fault_child(&library, "allocate", Some(2), false);
        run_fault_child(&library, "allocate", Some(3), false);
        run_fault_child(&library, "initialize", None, true);
        run_fault_child(&library, "stringify", Some(1), false);
    }

    // ERRORS 5-6: insufficient rows and columns.
    for (input, width, height) in [("1 2", 2, 2), ("1\n2", 2, 2)] {
        unsafe {
            assert_eq!(
                initialize_snapshot(&c, input, width, height).is_none(),
                initialize_snapshot(&rust, input, width, height).is_none()
            );
            assert!(initialize_snapshot(&c, input, width, height).is_none());
        }
    }

    // ERROR 7: incompatible dimensions.
    unsafe {
        let c_a = (c.initialize_matrix)(c"1 2".as_ptr(), 2, 1);
        let c_b = (c.initialize_matrix)(c"3".as_ptr(), 1, 1);
        let rust_a = (rust.initialize_matrix)(c"1 2".as_ptr(), 2, 1);
        let rust_b = (rust.initialize_matrix)(c"3".as_ptr(), 1, 1);
        assert!((c.multiply_matrices)(c_a, c_b).is_null());
        assert!((rust.multiply_matrices)(rust_a, rust_b).is_null());
        (c.free_matrix)(c_a);
        (c.free_matrix)(c_b);
        (rust.free_matrix)(rust_a);
        (rust.free_matrix)(rust_b);
    }

    // ERRORS 8 and 10: explicit null sentinels.
    unsafe {
        assert!((c.matrix_to_string)(null_mut()).is_null());
        assert!((rust.matrix_to_string)(null_mut()).is_null());
        let filename = c"/tmp/unused-driver-diff";
        assert_eq!(
            (c.write_to_file)(filename.as_ptr(), null()),
            (rust.write_to_file)(filename.as_ptr(), null())
        );
        assert_eq!((c.write_to_file)(filename.as_ptr(), null()), 22);
    }

    // ERROR 11: fopen failure and exact errno.
    let missing = temp_path("missing").join("file");
    let missing = CString::new(missing.as_os_str().as_encoded_bytes()).unwrap();
    unsafe {
        let c_result = (c.write_to_file)(missing.as_ptr(), c"x".as_ptr());
        let rust_result = (rust.write_to_file)(missing.as_ptr(), c"x".as_ptr());
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, 2);
    }

    // ERRORS 12-13: /dev/full fails during a large write and during close.
    let full = c"/dev/full";
    let large = CString::new(vec![b'x'; 64 * 1024]).unwrap();
    unsafe {
        let c_result = (c.write_to_file)(full.as_ptr(), large.as_ptr());
        let rust_result = (rust.write_to_file)(full.as_ptr(), large.as_ptr());
        assert_eq!(c_result, rust_result);
        assert_ne!(c_result, 0);
        let c_result = (c.write_to_file)(full.as_ptr(), c"x".as_ptr());
        let rust_result = (rust.write_to_file)(full.as_ptr(), c"x".as_ptr());
        assert_eq!(c_result, rust_result);
        assert_ne!(c_result, 0);
    }

    // ERRORS 14-16: each composed input/multiplication rejection.
    for (a_shape, a, b_shape, b) in [
        ((2, 2), "1 2", (1, 2), "3\n4"),
        ((1, 1), "1", (2, 2), "2 3"),
        ((2, 1), "1 2", (1, 1), "3"),
    ] {
        let result = compare_driver(&c, &rust, a_shape, a, b_shape, b);
        assert_eq!((result.0, result.1), (1, 1));
        assert_eq!(result.2, result.3);
    }

    // ERROR 17: matrix_to_string allocation failure in driver (10th malloc).
    for library in [c_library(), rust_library()] {
        run_fault_child(&library, "driver_stringify", Some(10), false);
    }

    // ERROR 18: matrix.txt is a directory, so write_to_file fails.
    {
        let _lock = CWD_LOCK.lock().unwrap();
        let old_directory = std::env::current_dir().unwrap();
        let directory = temp_path("driver-write-error");
        fs::create_dir(&directory).unwrap();
        fs::create_dir(directory.join("matrix.txt")).unwrap();
        std::env::set_current_dir(&directory).unwrap();
        let c_result = unsafe { (c.driver)(1, 1, c"2".as_ptr(), 1, 1, c"3".as_ptr()) };
        let rust_result = unsafe { (rust.driver)(1, 1, c"2".as_ptr(), 1, 1, c"3".as_ptr()) };
        std::env::set_current_dir(old_directory).unwrap();
        fs::remove_dir_all(directory).unwrap();
        assert_eq!((c_result, rust_result), (1, 1));
    }
}

fn run_null_child(library: &Path, mode: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "null_boundary_child", "--nocapture"])
        .env("DIFF_NULL_MODE", mode)
        .env("DIFF_LIBRARY", library.canonicalize().unwrap())
        .status()
        .unwrap()
}

#[test]
fn null_boundary_child() {
    let Ok(mode) = std::env::var("DIFF_NULL_MODE") else {
        return;
    };
    let api = unsafe { Api::load(Path::new(&std::env::var_os("DIFF_LIBRARY").unwrap())) };
    unsafe {
        match mode.as_str() {
            "initialize" => {
                (api.initialize_matrix)(null(), 1, 1);
            }
            "multiply_a" => {
                let matrix = (api.initialize_matrix)(c"1".as_ptr(), 1, 1);
                (api.multiply_matrices)(null_mut(), matrix);
            }
            "multiply_b" => {
                let matrix = (api.initialize_matrix)(c"1".as_ptr(), 1, 1);
                (api.multiply_matrices)(matrix, null_mut());
            }
            "driver_a" => {
                (api.driver)(1, 1, null(), 1, 1, c"1".as_ptr());
            }
            "driver_b" => {
                (api.driver)(1, 1, c"1".as_ptr(), 1, 1, null());
            }
            _ => panic!("unknown null mode {mode}"),
        }
    }
}

#[test]
fn generic_null_and_length_boundaries() {
    let (c, rust) = apis();

    // Explicitly supported null and zero-length cases.
    unsafe {
        (c.free_matrix)(null_mut());
        (rust.free_matrix)(null_mut());
        assert!((c.matrix_to_string)(null_mut()).is_null());
        assert!((rust.matrix_to_string)(null_mut()).is_null());
        assert_eq!(
            (c.write_to_file)(c"/tmp/unused-driver-diff".as_ptr(), null()),
            (rust.write_to_file)(c"/tmp/unused-driver-diff".as_ptr(), null())
        );
        assert_eq!(
            initialize_snapshot(&c, "", 0, 0),
            initialize_snapshot(&rust, "", 0, 0)
        );
    }

    // APIs that dereference null in C have undefined behavior; verify the
    // translation has the same process-level outcome at the external boundary.
    for mode in [
        "initialize",
        "multiply_a",
        "multiply_b",
        "driver_a",
        "driver_b",
    ] {
        let c_status = run_null_child(&c_library(), mode);
        let rust_status = run_null_child(&rust_library(), mode);
        assert_eq!(c_status.code(), rust_status.code(), "{mode}");
        assert_eq!(c_status.signal(), rust_status.signal(), "{mode}");
        assert!(c_status.signal().is_some(), "{mode} unexpectedly survived");
    }

    // Oversized dimensions are rejected by the same allocation sentinels.
    unsafe {
        let c_matrix = (c.allocate_matrix)(i32::MAX, 1);
        let rust_matrix = (rust.allocate_matrix)(i32::MAX, 1);
        assert_eq!(c_matrix.is_null(), rust_matrix.is_null());
        if !c_matrix.is_null() {
            (c.free_matrix)(c_matrix);
        }
        if !rust_matrix.is_null() {
            (rust.free_matrix)(rust_matrix);
        }
    }
}
