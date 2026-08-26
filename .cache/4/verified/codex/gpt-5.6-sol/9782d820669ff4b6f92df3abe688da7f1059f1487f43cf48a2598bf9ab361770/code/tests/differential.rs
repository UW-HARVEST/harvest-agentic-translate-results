use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::mem::size_of;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;

#[repr(C)]
struct Matrix {
    matrix: *mut *mut c_int,
    width: c_int,
    height: c_int,
}

type Allocate = unsafe extern "C" fn(c_int, c_int) -> *mut Matrix;
type Free = unsafe extern "C" fn(*mut Matrix);
type Initialize = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut Matrix;
type Multiply = unsafe extern "C" fn(*mut Matrix, *mut Matrix) -> *mut Matrix;
type ToString = unsafe extern "C" fn(*mut Matrix) -> *mut c_char;
type Write = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type Driver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

struct Api {
    _library: Library,
    allocate: Allocate,
    free: Free,
    initialize: Initialize,
    multiply: Multiply,
    to_string: ToString,
    write: Write,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        let allocate = unsafe { *library.get(b"allocate_matrix\0").unwrap() };
        let free = unsafe { *library.get(b"free_matrix\0").unwrap() };
        let initialize = unsafe { *library.get(b"initialize_matrix_from_string\0").unwrap() };
        let multiply = unsafe { *library.get(b"multiply_matrices\0").unwrap() };
        let to_string = unsafe { *library.get(b"matrix_to_string\0").unwrap() };
        let write = unsafe { *library.get(b"write_to_file\0").unwrap() };
        let driver = unsafe { *library.get(b"driver\0").unwrap() };
        Self {
            _library: library,
            allocate,
            free,
            initialize,
            multiply,
            to_string,
            write,
            driver,
        }
    }
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    width: i32,
    height: i32,
    values: Vec<i32>,
}

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

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

unsafe fn snapshot(matrix: *mut Matrix) -> Snapshot {
    assert!(!matrix.is_null());
    let width = unsafe { (*matrix).width };
    let height = unsafe { (*matrix).height };
    let mut values = Vec::new();
    for row in 0..height.max(0) {
        for column in 0..width.max(0) {
            values.push(unsafe { *(*(*matrix).matrix.add(row as usize)).add(column as usize) });
        }
    }
    Snapshot {
        width,
        height,
        values,
    }
}

unsafe fn set_values(matrix: *mut Matrix, values: &[i32]) {
    let width = unsafe { (*matrix).width };
    let height = unsafe { (*matrix).height };
    assert_eq!(values.len(), (width * height) as usize);
    for row in 0..height {
        for column in 0..width {
            unsafe {
                *(*(*matrix).matrix.add(row as usize)).add(column as usize) =
                    values[(row * width + column) as usize];
            }
        }
    }
}

unsafe fn allocated(api: &Api, width: i32, height: i32, values: &[i32]) -> *mut Matrix {
    let matrix = unsafe { (api.allocate)(width, height) };
    assert!(!matrix.is_null());
    unsafe { set_values(matrix, values) };
    matrix
}

fn matrix_input(width: i32, height: i32, values: &[i32]) -> CString {
    assert_eq!(values.len(), (width * height) as usize);
    let mut text = String::new();
    for row in 0..height {
        for column in 0..width {
            if column != 0 {
                text.push(' ');
            }
            text.push_str(&values[(row * width + column) as usize].to_string());
        }
        text.push('\n');
    }
    CString::new(text).unwrap()
}

unsafe fn initialize_snapshot(api: &Api, text: &CStr, width: i32, height: i32) -> Snapshot {
    let matrix = unsafe { (api.initialize)(text.as_ptr(), width, height) };
    assert!(!matrix.is_null());
    let result = unsafe { snapshot(matrix) };
    unsafe { (api.free)(matrix) };
    result
}

unsafe fn string_snapshot(api: &Api, matrix: *mut Matrix) -> Vec<u8> {
    let text = unsafe { (api.to_string)(matrix) };
    assert!(!text.is_null());
    let bytes = unsafe { CStr::from_ptr(text) }.to_bytes().to_vec();
    unsafe { free(text.cast()) };
    bytes
}

fn assert_initializers_match(c: &Api, rust: &Api, text: &CStr, width: i32, height: i32) {
    let c_value = unsafe { initialize_snapshot(c, text, width, height) };
    let rust_value = unsafe { initialize_snapshot(rust, text, width, height) };
    assert_eq!(c_value, rust_value);
}

fn assert_multiplication_matches(
    c: &Api,
    rust: &Api,
    a_shape: (i32, i32),
    a_values: &[i32],
    b_shape: (i32, i32),
    b_values: &[i32],
) {
    unsafe {
        let c_a = allocated(c, a_shape.0, a_shape.1, a_values);
        let c_b = allocated(c, b_shape.0, b_shape.1, b_values);
        let rust_a = allocated(rust, a_shape.0, a_shape.1, a_values);
        let rust_b = allocated(rust, b_shape.0, b_shape.1, b_values);
        let c_result = (c.multiply)(c_a, c_b);
        let rust_result = (rust.multiply)(rust_a, rust_b);
        assert!(!c_result.is_null());
        assert!(!rust_result.is_null());
        assert_eq!(snapshot(c_result), snapshot(rust_result));
        (c.free)(c_a);
        (c.free)(c_b);
        (c.free)(c_result);
        (rust.free)(rust_a);
        (rust.free)(rust_b);
        (rust.free)(rust_result);
    }
}

fn compile_fault_shim() -> PathBuf {
    let output = manifest_dir().join("target/differential/libfault_alloc.so");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(manifest_dir().join("tests/fault_alloc.c"))
        .args(["-o"])
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success());
    output
}

fn helper_output(library: &Path, scenario: &str, shim: Option<&Path>) -> Output {
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .args(["--exact", "differential_surface", "--nocapture"])
        .env("DIFF_HELPER", "1")
        .env("DIFF_LIBRARY", library)
        .env("DIFF_SCENARIO", scenario);
    if let Some(shim) = shim {
        command.env("LD_PRELOAD", shim).env("DIFF_SHIM", shim);
    }
    command.output().unwrap()
}

fn assert_helper_match(scenario: &str, shim: Option<&Path>, expect_success: bool) {
    let c = helper_output(&c_library(), scenario, shim);
    let rust = helper_output(&rust_library(), scenario, shim);
    assert_eq!(
        c.status.success(),
        expect_success,
        "C helper {scenario}: {}",
        String::from_utf8_lossy(&c.stderr)
    );
    assert_eq!(
        rust.status.success(),
        expect_success,
        "Rust helper {scenario}: {}",
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(c.status.code(), rust.status.code(), "{scenario}");
    assert_eq!(c.status.signal(), rust.status.signal(), "{scenario}");
    assert_eq!(c.stdout, rust.stdout, "{scenario}");
}

fn run_helper() {
    let library_path = PathBuf::from(env::var_os("DIFF_LIBRARY").unwrap());
    let scenario = env::var("DIFF_SCENARIO").unwrap();
    let api = unsafe { Api::load(&library_path) };
    let one = CString::new("1").unwrap();

    match scenario.as_str() {
        "alloc_struct"
        | "alloc_oversized_width"
        | "alloc_oversized_height"
        | "init_strdup"
        | "string_alloc"
        | "driver_string_alloc" => {
            let shim_path = PathBuf::from(env::var_os("DIFF_SHIM").unwrap());
            let shim = unsafe { Library::new(shim_path) }.unwrap();
            if scenario == "init_strdup" {
                let fail: unsafe extern "C" fn() =
                    unsafe { *shim.get(b"fault_fail_next_strdup\0").unwrap() };
                unsafe { fail() };
                let result = unsafe { (api.initialize)(one.as_ptr(), 1, 1) };
                println!("{}", result.is_null());
                assert!(result.is_null());
            } else {
                let fail: unsafe extern "C" fn(usize) =
                    unsafe { *shim.get(b"fault_fail_malloc_size\0").unwrap() };
                if scenario == "alloc_struct" {
                    unsafe { fail(size_of::<Matrix>()) };
                    let result = unsafe { (api.allocate)(1, 1) };
                    println!("{}", result.is_null());
                    assert!(result.is_null());
                } else if scenario == "alloc_oversized_width" {
                    unsafe { fail((i32::MAX as usize) * size_of::<i32>()) };
                    let result = unsafe { (api.allocate)(i32::MAX, 1) };
                    println!("{}", result.is_null());
                    assert!(result.is_null());
                } else if scenario == "alloc_oversized_height" {
                    unsafe { fail((i32::MAX as usize) * size_of::<*mut i32>()) };
                    let result = unsafe { (api.allocate)(1, i32::MAX) };
                    println!("{}", result.is_null());
                    assert!(result.is_null());
                } else if scenario == "string_alloc" {
                    let matrix = unsafe { allocated(&api, 1, 1, &[1]) };
                    unsafe { fail(13) };
                    let result = unsafe { (api.to_string)(matrix) };
                    println!("{}", result.is_null());
                    assert!(result.is_null());
                    unsafe { (api.free)(matrix) };
                } else {
                    unsafe { fail(13) };
                    let result = unsafe { (api.driver)(1, 1, one.as_ptr(), 1, 1, one.as_ptr()) };
                    println!("{result}");
                    assert_eq!(result, 1);
                }
            }
        }
        "init_null" => unsafe {
            (api.initialize)(ptr::null(), 1, 1);
        },
        "multiply_a_null" => unsafe {
            let other = allocated(&api, 1, 1, &[1]);
            (api.multiply)(ptr::null_mut(), other);
        },
        "multiply_b_null" => unsafe {
            let other = allocated(&api, 1, 1, &[1]);
            (api.multiply)(other, ptr::null_mut());
        },
        "write_filename_null" => unsafe {
            let result = (api.write)(ptr::null(), one.as_ptr());
            println!("{result}");
            assert_ne!(result, 0);
        },
        "driver_a_null" => unsafe {
            (api.driver)(1, 1, ptr::null(), 1, 1, one.as_ptr());
        },
        "driver_b_null" => unsafe {
            (api.driver)(1, 1, one.as_ptr(), 1, 1, ptr::null());
        },
        _ => panic!("unknown helper scenario: {scenario}"),
    }
}

fn compare_driver(
    c: &Api,
    rust: &Api,
    width_a: i32,
    height_a: i32,
    a: &CStr,
    width_b: i32,
    height_b: i32,
    b: &CStr,
) {
    let output = Path::new("matrix.txt");
    let _ = fs::remove_file(output);
    let c_status =
        unsafe { (c.driver)(width_a, height_a, a.as_ptr(), width_b, height_b, b.as_ptr()) };
    let c_bytes = fs::read(output).unwrap();
    fs::remove_file(output).unwrap();
    let rust_status =
        unsafe { (rust.driver)(width_a, height_a, a.as_ptr(), width_b, height_b, b.as_ptr()) };
    let rust_bytes = fs::read(output).unwrap();
    fs::remove_file(output).unwrap();
    assert_eq!(c_status, rust_status);
    assert_eq!(c_bytes, rust_bytes);
}

#[test]
fn differential_surface() {
    if env::var_os("DIFF_HELPER").is_some() {
        run_helper();
        return;
    }

    assert!(c_library().is_file());
    assert!(rust_library().is_file());
    let c = unsafe { Api::load(&c_library()) };
    let rust = unsafe { Api::load(&rust_library()) };
    let mut rng = Rng::new(0x5eed_c0de_d15c_a11e);

    // CONFIGS 1-5: allocation/free loop shapes.
    for _ in 0..64 {
        let shapes = [
            (0, 0),
            (0, rng.range(1, 7)),
            (1, 1),
            (rng.range(2, 7), rng.range(2, 7)),
        ];
        for (width, height) in shapes {
            unsafe {
                let c_matrix = (c.allocate)(width, height);
                let rust_matrix = (rust.allocate)(width, height);
                let values = (0..width * height)
                    .map(|_| rng.range(-1_000_000, 1_000_001))
                    .collect::<Vec<_>>();
                set_values(c_matrix, &values);
                set_values(rust_matrix, &values);
                assert_eq!(snapshot(c_matrix), snapshot(rust_matrix));
                (c.free)(c_matrix);
                (rust.free)(rust_matrix);
            }
        }
    }

    // CONFIGS 6-12: tokenization and atoi shapes.
    for _ in 0..64 {
        let ignored = CString::new(format!("{} {} extra", rng.next_u32(), rng.next_u32())).unwrap();
        assert_initializers_match(&c, &rust, &ignored, 0, 0);

        let rows = rng.range(1, 7);
        let row_text = CString::new(
            (0..rows)
                .map(|_| rng.next_u32().to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        assert_initializers_match(&c, &rust, &row_text, 0, rows);
        assert_initializers_match(&c, &rust, &ignored, rng.range(1, 7), 0);

        let scalar = CString::new(rng.range(-100_000, 100_001).to_string()).unwrap();
        assert_initializers_match(&c, &rust, &scalar, 1, 1);

        let width = rng.range(2, 6);
        let height = rng.range(2, 6);
        let values = (0..width * height)
            .map(|_| rng.range(-10_000, 10_001))
            .collect::<Vec<_>>();
        let normal = matrix_input(width, height, &values);
        let repeated = CString::new(
            normal
                .to_string_lossy()
                .replace(' ', "   ")
                .replace('\n', "\n\n"),
        )
        .unwrap();
        assert_initializers_match(&c, &rust, &repeated, width, height);

        let extra = CString::new(format!("{} 777\n888 999\n", normal.to_string_lossy())).unwrap();
        assert_initializers_match(&c, &rust, &extra, width, height);

        let odd = CString::new(format!(
            "+{} -{} nope {}tail\n",
            rng.range(1, 10_000),
            rng.range(1, 10_000),
            rng.range(1, 10_000)
        ))
        .unwrap();
        assert_initializers_match(&c, &rust, &odd, 4, 1);
    }

    // CONFIGS 13-18: conforming multiplication shapes.
    for _ in 0..64 {
        let a = [rng.range(-100, 101)];
        let b = [rng.range(-100, 101)];
        assert_multiplication_matches(&c, &rust, (1, 1), &a, (1, 1), &b);

        let height = rng.range(1, 6);
        let width = rng.range(1, 6);
        let a = (0..height)
            .map(|_| rng.range(-100, 101))
            .collect::<Vec<_>>();
        let b = (0..width).map(|_| rng.range(-100, 101)).collect::<Vec<_>>();
        assert_multiplication_matches(&c, &rust, (1, height), &a, (width, 1), &b);

        let inner = rng.range(2, 6);
        let a_width = inner;
        let a_height = rng.range(1, 6);
        let b_width = rng.range(1, 6);
        let a = (0..a_width * a_height)
            .map(|_| rng.range(-100, 101))
            .collect::<Vec<_>>();
        let b = (0..b_width * inner)
            .map(|_| rng.range(-100, 101))
            .collect::<Vec<_>>();
        assert_multiplication_matches(&c, &rust, (a_width, a_height), &a, (b_width, inner), &b);

        let zero_height = rng.range(1, 6);
        let zero_width = rng.range(1, 6);
        assert_multiplication_matches(&c, &rust, (0, zero_height), &[], (zero_width, 0), &[]);

        let inner = rng.range(1, 6);
        let b_width = rng.range(1, 6);
        let b = (0..b_width * inner)
            .map(|_| rng.range(-100, 101))
            .collect::<Vec<_>>();
        assert_multiplication_matches(&c, &rust, (inner, 0), &[], (b_width, inner), &b);

        let a_height = rng.range(1, 6);
        let a = (0..inner * a_height)
            .map(|_| rng.range(-100, 101))
            .collect::<Vec<_>>();
        assert_multiplication_matches(&c, &rust, (inner, a_height), &a, (0, inner), &[]);
    }

    // CONFIGS 19-22: formatting shapes and integer boundaries.
    for iteration in 0..64 {
        let scalar = match iteration % 3 {
            0 => 0,
            1 => rng.range(1, 1_000_001),
            _ => rng.range(-1_000_000, 0),
        };
        let shapes = [
            (0, 0, Vec::new()),
            (0, rng.range(1, 7), Vec::new()),
            (1, 1, vec![scalar]),
            (rng.range(2, 6), rng.range(2, 6), Vec::new()),
        ];
        for (width, height, mut values) in shapes {
            if width > 1 {
                values = (0..width * height)
                    .map(|index| match index % 4 {
                        0 => i32::MIN,
                        1 => i32::MAX,
                        _ => rng.range(-1_000_000, 1_000_001),
                    })
                    .collect();
            }
            unsafe {
                let c_matrix = allocated(&c, width, height, &values);
                let rust_matrix = allocated(&rust, width, height, &values);
                assert_eq!(
                    string_snapshot(&c, c_matrix),
                    string_snapshot(&rust, rust_matrix)
                );
                (c.free)(c_matrix);
                (rust.free)(rust_matrix);
            }
        }
    }

    let root = manifest_dir().join("target/differential/run");
    fs::create_dir_all(&root).unwrap();

    // CONFIGS 23-25: new, empty, nonempty, and truncating file writes.
    for iteration in 0..64 {
        let random_content = format!("payload-{}-{}", rng.next_u32(), rng.next_u32());
        let replacement = format!("replacement-{}-{}", rng.next_u32(), rng.next_u32());
        for (variant, content) in ["", random_content.as_str(), replacement.as_str()]
            .into_iter()
            .enumerate()
        {
            let c_path = root.join(format!("c-write-{iteration}-{variant}"));
            let rust_path = root.join(format!("rust-write-{iteration}-{variant}"));
            if content == replacement {
                fs::write(&c_path, b"older and longer contents").unwrap();
                fs::write(&rust_path, b"older and longer contents").unwrap();
            }
            let c_name = CString::new(c_path.as_os_str().as_encoded_bytes()).unwrap();
            let rust_name = CString::new(rust_path.as_os_str().as_encoded_bytes()).unwrap();
            let content = CString::new(content).unwrap();
            let c_status = unsafe { (c.write)(c_name.as_ptr(), content.as_ptr()) };
            let rust_status = unsafe { (rust.write)(rust_name.as_ptr(), content.as_ptr()) };
            assert_eq!(c_status, rust_status);
            assert_eq!(fs::read(c_path).unwrap(), fs::read(rust_path).unwrap());
        }
    }

    // CONFIGS 26-29: composed end-to-end driver paths.
    let original_directory = env::current_dir().unwrap();
    env::set_current_dir(&root).unwrap();
    for _ in 0..64 {
        let scalar_a = matrix_input(1, 1, &[rng.range(-100, 101)]);
        let scalar_b = matrix_input(1, 1, &[rng.range(-100, 101)]);
        compare_driver(&c, &rust, 1, 1, &scalar_a, 1, 1, &scalar_b);

        let height = rng.range(1, 6);
        let width = rng.range(1, 6);
        let a_values = (0..height)
            .map(|_| rng.range(-100, 101))
            .collect::<Vec<_>>();
        let b_values = (0..width).map(|_| rng.range(-100, 101)).collect::<Vec<_>>();
        compare_driver(
            &c,
            &rust,
            1,
            height,
            &matrix_input(1, height, &a_values),
            width,
            1,
            &matrix_input(width, 1, &b_values),
        );

        let inner = rng.range(2, 6);
        let a_height = rng.range(2, 6);
        let b_width = rng.range(2, 6);
        let a_values = (0..inner * a_height)
            .map(|_| rng.range(-100, 101))
            .collect::<Vec<_>>();
        let b_values = (0..b_width * inner)
            .map(|_| rng.range(-100, 101))
            .collect::<Vec<_>>();
        compare_driver(
            &c,
            &rust,
            inner,
            a_height,
            &matrix_input(inner, a_height, &a_values),
            b_width,
            inner,
            &matrix_input(b_width, inner, &b_values),
        );

        let extra_a = CString::new(format!(
            "{} {}\n{}\n",
            rng.range(-100, 101),
            rng.next_u32(),
            rng.next_u32()
        ))
        .unwrap();
        let extra_b = CString::new(format!(
            "{} {}\n{}\n",
            rng.range(-100, 101),
            rng.next_u32(),
            rng.next_u32()
        ))
        .unwrap();
        compare_driver(&c, &rust, 1, 1, &extra_a, 1, 1, &extra_b);
    }
    env::set_current_dir(original_directory).unwrap();

    // ERRORS 2-3: oversized dimensions deterministically fail row storage.
    unsafe {
        let c_rows = (c.allocate)(1, -1);
        let rust_rows = (rust.allocate)(1, -1);
        assert_eq!(c_rows.is_null(), rust_rows.is_null());
        assert!(c_rows.is_null());

        let c_columns = (c.allocate)(-1, 1);
        let rust_columns = (rust.allocate)(-1, 1);
        assert_eq!(c_columns.is_null(), rust_columns.is_null());
        assert!(c_columns.is_null());
    }

    // ERROR 4 and generic null handling explicitly accepted by C.
    unsafe {
        (c.free)(ptr::null_mut());
        (rust.free)(ptr::null_mut());
    }

    // ERRORS 6-7: exact parser rejection shapes.
    for _ in 0..64 {
        let short_rows = CString::new(rng.range(-100, 101).to_string()).unwrap();
        let c_result = unsafe { (c.initialize)(short_rows.as_ptr(), 1, 2) };
        let rust_result = unsafe { (rust.initialize)(short_rows.as_ptr(), 1, 2) };
        assert_eq!(c_result.is_null(), rust_result.is_null());
        assert!(c_result.is_null());

        let short_columns = CString::new(rng.range(-100, 101).to_string()).unwrap();
        let c_result = unsafe { (c.initialize)(short_columns.as_ptr(), 2, 1) };
        let rust_result = unsafe { (rust.initialize)(short_columns.as_ptr(), 2, 1) };
        assert_eq!(c_result.is_null(), rust_result.is_null());
        assert!(c_result.is_null());
    }

    // ERROR 8: incompatible dimensions.
    for _ in 0..64 {
        unsafe {
            let c_a = allocated(&c, 1, 1, &[1]);
            let c_b = allocated(&c, 1, 2, &[1, 2]);
            let rust_a = allocated(&rust, 1, 1, &[1]);
            let rust_b = allocated(&rust, 1, 2, &[1, 2]);
            let c_result = (c.multiply)(c_a, c_b);
            let rust_result = (rust.multiply)(rust_a, rust_b);
            assert_eq!(c_result.is_null(), rust_result.is_null());
            assert!(c_result.is_null());
            (c.free)(c_a);
            (c.free)(c_b);
            (rust.free)(rust_a);
            (rust.free)(rust_b);
        }
    }

    // ERROR 9: null matrix conversion.
    unsafe {
        assert!((c.to_string)(ptr::null_mut()).is_null());
        assert!((rust.to_string)(ptr::null_mut()).is_null());
    }

    // ERRORS 11-14: null content and each file I/O rejection.
    let valid_content = CString::new("x").unwrap();
    let missing = CString::new("/definitely/missing/directory/output").unwrap();
    let full = CString::new("/dev/full").unwrap();
    unsafe {
        assert_eq!((c.write)(missing.as_ptr(), ptr::null()), 22);
        assert_eq!((rust.write)(missing.as_ptr(), ptr::null()), 22);
        assert_eq!(
            (c.write)(missing.as_ptr(), valid_content.as_ptr()),
            (rust.write)(missing.as_ptr(), valid_content.as_ptr())
        );

        let large = CString::new(vec![b'x'; 1024 * 1024]).unwrap();
        let c_write = (c.write)(full.as_ptr(), large.as_ptr());
        let rust_write = (rust.write)(full.as_ptr(), large.as_ptr());
        assert_eq!(c_write, rust_write);
        assert_ne!(c_write, 0);

        let c_close = (c.write)(full.as_ptr(), valid_content.as_ptr());
        let rust_close = (rust.write)(full.as_ptr(), valid_content.as_ptr());
        assert_eq!(c_close, rust_close);
        assert_ne!(c_close, 0);
    }

    // ERRORS 15-17 and 19: each reachable driver propagation branch.
    let one = CString::new("1").unwrap();
    let two_rows = CString::new("1\n2").unwrap();
    let short = CString::new("").unwrap();
    unsafe {
        assert_eq!(
            (c.driver)(1, 1, short.as_ptr(), 1, 1, one.as_ptr()),
            (rust.driver)(1, 1, short.as_ptr(), 1, 1, one.as_ptr())
        );
        assert_eq!(
            (c.driver)(1, 1, one.as_ptr(), 1, 1, short.as_ptr()),
            (rust.driver)(1, 1, one.as_ptr(), 1, 1, short.as_ptr())
        );
        assert_eq!(
            (c.driver)(1, 1, one.as_ptr(), 1, 2, two_rows.as_ptr()),
            (rust.driver)(1, 1, one.as_ptr(), 1, 2, two_rows.as_ptr())
        );
    }
    env::set_current_dir("/proc").unwrap();
    unsafe {
        assert_eq!(
            (c.driver)(1, 1, one.as_ptr(), 1, 1, one.as_ptr()),
            (rust.driver)(1, 1, one.as_ptr(), 1, 1, one.as_ptr())
        );
    }
    env::set_current_dir(&root).unwrap();

    // ERRORS 1, 5, 10, and 18: deterministic allocator fault injection.
    let shim = compile_fault_shim();
    for scenario in [
        "alloc_struct",
        "alloc_oversized_width",
        "alloc_oversized_height",
        "init_strdup",
        "string_alloc",
        "driver_string_alloc",
    ] {
        assert_helper_match(scenario, Some(&shim), true);
    }

    // Generic unchecked null pointers: compare process-level C/Rust behavior.
    for scenario in [
        "init_null",
        "multiply_a_null",
        "multiply_b_null",
        "driver_a_null",
        "driver_b_null",
    ] {
        assert_helper_match(scenario, None, false);
    }
    assert_helper_match("write_filename_null", None, true);
}
