use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::slice;

type ExtractFilename =
    unsafe extern "C" fn(path: *const c_char, separator: c_char) -> *const c_char;
type CreateFilename = unsafe extern "C" fn(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char;

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

struct Api {
    _library: Library,
    extract_filename: ExtractFilename,
    create_filename: CreateFilename,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let extract_filename = unsafe {
            *library
                .get::<ExtractFilename>(b"extractFilename\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load extractFilename from {}: {error}",
                        path.display()
                    )
                })
        };
        let create_filename = unsafe {
            *library
                .get::<CreateFilename>(b"FIO_createFilename_fromOutDir\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load FIO_createFilename_fromOutDir from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            extract_filename,
            create_filename,
        }
    }
}

#[derive(Clone, Copy)]
enum PathShape {
    NoSeparator,
    OneSeparator,
    MultipleSeparators,
    TrailingSeparator,
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

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() as usize) % upper_exclusive
    }

    fn byte_except(&mut self, excluded: &[u8]) -> u8 {
        loop {
            let value = self.next_u64() as u8;
            if value != 0 && !excluded.contains(&value) {
                return value;
            }
        }
    }

    fn bytes(&mut self, len: usize, excluded: &[u8]) -> Vec<u8> {
        (0..len).map(|_| self.byte_except(excluded)).collect()
    }

    fn variable_bytes(
        &mut self,
        minimum_len: usize,
        maximum_len_exclusive: usize,
        excluded: &[u8],
    ) -> Vec<u8> {
        let len = minimum_len + self.usize(maximum_len_exclusive - minimum_len);
        self.bytes(len, excluded)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(path);
    }

    let profile_dir = manifest_dir().join("target/debug");
    let direct = profile_dir.join("libdriver.so");
    if direct.is_file() {
        return direct;
    }

    let release = manifest_dir().join("target/release/libdriver.so");
    if release.is_file() {
        return release;
    }

    let deps = profile_dir.join("deps");
    let mut candidates: Vec<_> = std::fs::read_dir(&deps)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", deps.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libdriver") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no Rust driver shared library found in {}", deps.display()))
}

fn load_apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

fn random_component(rng: &mut Rng, allow_empty: bool) -> Vec<u8> {
    let minimum = usize::from(!allow_empty);
    let len = minimum + rng.usize(65 - minimum);
    rng.bytes(len, &[b'/'])
}

fn make_path(rng: &mut Rng, shape: PathShape, iteration: usize) -> Vec<u8> {
    match shape {
        PathShape::NoSeparator => {
            if iteration == 0 {
                Vec::new()
            } else {
                random_component(rng, false)
            }
        }
        PathShape::OneSeparator => {
            let mut path = random_component(rng, false);
            path.push(b'/');
            path.extend(random_component(rng, false));
            path
        }
        PathShape::MultipleSeparators => {
            let mut path = random_component(rng, false);
            for _ in 0..(2 + rng.usize(5)) {
                path.push(b'/');
                path.extend(random_component(rng, false));
            }
            path
        }
        PathShape::TrailingSeparator => {
            let mut path = random_component(rng, true);
            path.push(b'/');
            path
        }
    }
}

fn make_out_dir(rng: &mut Rng, trailing_separator: bool) -> Vec<u8> {
    let mut out_dir = random_component(rng, false);
    if trailing_separator {
        out_dir.push(b'/');
    }
    out_dir
}

fn filename_len(path: &[u8]) -> usize {
    path.iter()
        .rposition(|byte| *byte == b'/')
        .map_or(path.len(), |position| path.len() - position - 1)
}

unsafe fn compare_create_raw(
    c_api: &Api,
    rust_api: &Api,
    c_path: *const c_char,
    rust_path: *const c_char,
    c_out_dir: *const c_char,
    rust_out_dir: *const c_char,
    allocation_size: usize,
    expected: &[u8],
    suffix_len: usize,
    label: &str,
) {
    let c_result = unsafe { (c_api.create_filename)(c_path, c_out_dir, suffix_len) };
    let rust_result = unsafe { (rust_api.create_filename)(rust_path, rust_out_dir, suffix_len) };
    assert!(!c_result.is_null(), "{label}: C returned NULL");
    assert!(!rust_result.is_null(), "{label}: Rust returned NULL");

    let c_bytes = unsafe { slice::from_raw_parts(c_result.cast::<u8>(), allocation_size) }.to_vec();
    let rust_bytes =
        unsafe { slice::from_raw_parts(rust_result.cast::<u8>(), allocation_size) }.to_vec();
    unsafe {
        free(c_result.cast::<c_void>());
        free(rust_result.cast::<c_void>());
    }

    let mut expected_allocation = expected.to_vec();
    expected_allocation.resize(allocation_size, 0);
    assert_eq!(c_bytes, expected_allocation, "{label}: unexpected C bytes");
    assert_eq!(rust_bytes, c_bytes, "{label}: Rust/C byte mismatch");
}

fn compare_create(
    c_api: &Api,
    rust_api: &Api,
    path: &[u8],
    out_dir: &[u8],
    suffix_len: usize,
    label: &str,
) {
    let c_path = CString::new(path).unwrap();
    let rust_path = CString::new(path).unwrap();
    let c_out_dir = CString::new(out_dir).unwrap();
    let rust_out_dir = CString::new(out_dir).unwrap();

    let filename_start = path
        .iter()
        .rposition(|byte| *byte == b'/')
        .map_or(0, |position| position + 1);
    let mut expected = out_dir.to_vec();
    if !out_dir.ends_with(b"/") {
        expected.push(b'/');
    }
    expected.extend_from_slice(&path[filename_start..]);

    let allocation_size = out_dir.len() + 1 + filename_len(path) + suffix_len + 1;
    unsafe {
        compare_create_raw(
            c_api,
            rust_api,
            c_path.as_ptr(),
            rust_path.as_ptr(),
            c_out_dir.as_ptr(),
            rust_out_dir.as_ptr(),
            allocation_size,
            &expected,
            suffix_len,
            label,
        );
    }
}

fn compare_extract(
    c_api: &Api,
    rust_api: &Api,
    path: &[u8],
    separator: u8,
    expected_offset: usize,
    label: &str,
) {
    let c_path = CString::new(path).unwrap();
    let rust_path = CString::new(path).unwrap();
    let c_result = unsafe { (c_api.extract_filename)(c_path.as_ptr(), separator as c_char) };
    let rust_result =
        unsafe { (rust_api.extract_filename)(rust_path.as_ptr(), separator as c_char) };
    let c_offset = unsafe { c_result.offset_from(c_path.as_ptr()) } as usize;
    let rust_offset = unsafe { rust_result.offset_from(rust_path.as_ptr()) } as usize;

    assert_eq!(c_offset, expected_offset, "{label}: unexpected C offset");
    assert_eq!(rust_offset, c_offset, "{label}: Rust/C offset mismatch");
    if expected_offset <= path.len() {
        let c_bytes = unsafe { CStr::from_ptr(c_result) }.to_bytes();
        let rust_bytes = unsafe { CStr::from_ptr(rust_result) }.to_bytes();
        assert_eq!(
            c_bytes,
            &path[expected_offset..],
            "{label}: unexpected C bytes"
        );
        assert_eq!(rust_bytes, c_bytes, "{label}: Rust/C byte mismatch");
    }
}

#[test]
fn extract_filename_configuration_rows_e1_through_e5() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x4f4f_3d2c_1b0a_9988);

    for iteration in 0..256 {
        let separator = rng.byte_except(&[]);
        let absent = if iteration == 0 {
            Vec::new()
        } else {
            rng.variable_bytes(1, 129, &[separator])
        };
        compare_extract(&c_api, &rust_api, &absent, separator, 0, "E1");

        let mut one = rng.variable_bytes(1, 65, &[separator]);
        one.push(separator);
        one.extend(rng.variable_bytes(1, 65, &[separator]));
        let one_offset = one.iter().position(|byte| *byte == separator).unwrap() + 1;
        compare_extract(&c_api, &rust_api, &one, separator, one_offset, "E2");

        let mut multiple = rng.variable_bytes(1, 33, &[separator]);
        for _ in 0..(2 + rng.usize(6)) {
            multiple.push(separator);
            multiple.extend(rng.variable_bytes(1, 33, &[separator]));
        }
        let multiple_offset = multiple
            .iter()
            .rposition(|byte| *byte == separator)
            .unwrap()
            + 1;
        compare_extract(
            &c_api,
            &rust_api,
            &multiple,
            separator,
            multiple_offset,
            "E3",
        );

        let mut trailing = rng.variable_bytes(0, 128, &[separator]);
        trailing.push(separator);
        let trailing_len = trailing.len();
        compare_extract(&c_api, &rust_api, &trailing, separator, trailing_len, "E4");

        let nul_path = rng.variable_bytes(0, 128, &[]);
        let nul_offset = nul_path.len() + 1;
        compare_extract(&c_api, &rust_api, &nul_path, 0, nul_offset, "E5");
    }
}

#[test]
fn create_filename_configuration_rows_f1_through_f16_and_boundary_b5() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0x1020_3040_5060_7080);
    let shapes = [
        PathShape::NoSeparator,
        PathShape::OneSeparator,
        PathShape::MultipleSeparators,
        PathShape::TrailingSeparator,
    ];

    for (shape_index, shape) in shapes.into_iter().enumerate() {
        for trailing_out_dir in [false, true] {
            for zero_suffix in [true, false] {
                let row = 1
                    + shape_index * 4
                    + usize::from(trailing_out_dir) * 2
                    + usize::from(!zero_suffix);
                let label = format!("F{row}");
                for iteration in 0..128 {
                    let path = make_path(&mut rng, shape, iteration);
                    let out_dir = make_out_dir(&mut rng, trailing_out_dir);
                    let suffix_len = if zero_suffix { 0 } else { 1 + rng.usize(256) };
                    compare_create(&c_api, &rust_api, &path, &out_dir, suffix_len, &label);
                }
            }
        }
    }
}

#[test]
fn create_filename_empty_out_dir_rows_f17_and_f18_and_boundary_b4() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0xa5a5_5a5a_dead_beef);

    for (guard, label) in [(b'/', "F17"), (b'x', "F18")] {
        for iteration in 0..128 {
            let path = make_path(
                &mut rng,
                match iteration % 4 {
                    0 => PathShape::NoSeparator,
                    1 => PathShape::OneSeparator,
                    2 => PathShape::MultipleSeparators,
                    _ => PathShape::TrailingSeparator,
                },
                iteration,
            );
            let c_path = CString::new(path.clone()).unwrap();
            let rust_path = CString::new(path.clone()).unwrap();
            let c_guarded_out_dir = [guard, 0];
            let rust_guarded_out_dir = [guard, 0];
            let c_out_dir = unsafe { c_guarded_out_dir.as_ptr().add(1).cast::<c_char>() };
            let rust_out_dir = unsafe { rust_guarded_out_dir.as_ptr().add(1).cast::<c_char>() };
            let suffix_len = if iteration % 2 == 0 {
                0
            } else {
                1 + rng.usize(256)
            };
            let filename_start = path
                .iter()
                .rposition(|byte| *byte == b'/')
                .map_or(0, |position| position + 1);
            let mut expected = Vec::new();
            if guard != b'/' {
                expected.push(b'/');
            }
            expected.extend_from_slice(&path[filename_start..]);
            let allocation_size = 1 + filename_len(&path) + suffix_len + 1;

            unsafe {
                compare_create_raw(
                    &c_api,
                    &rust_api,
                    c_path.as_ptr(),
                    rust_path.as_ptr(),
                    c_out_dir,
                    rust_out_dir,
                    allocation_size,
                    &expected,
                    suffix_len,
                    label,
                );
            }
        }
    }
}

fn run_boundary_child(library_path: &Path, scenario: &str) -> Output {
    Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_boundary_child", "--nocapture"])
        .env("DIFF_CHILD_LIBRARY", library_path)
        .env("DIFF_CHILD_SCENARIO", scenario)
        .output()
        .unwrap_or_else(|error| panic!("failed to run {scenario} child: {error}"))
}

#[cfg(unix)]
fn assert_same_signal(c_output: &Output, rust_output: &Output, label: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_signal = c_output.status.signal();
    let rust_signal = rust_output.status.signal();
    assert!(c_signal.is_some(), "{label}: C did not terminate by signal");
    assert_eq!(
        rust_signal, c_signal,
        "{label}: Rust/C termination signal mismatch"
    );
}

#[test]
fn generic_null_pointer_boundaries_b1_through_b3() {
    for (scenario, label) in [
        ("null_extract_path", "B1"),
        ("null_create_path", "B2"),
        ("null_create_out_dir", "B3"),
    ] {
        let c_output = run_boundary_child(&c_library_path(), scenario);
        let rust_output = run_boundary_child(&rust_library_path(), scenario);
        #[cfg(unix)]
        assert_same_signal(&c_output, &rust_output, label);
        #[cfg(not(unix))]
        assert_eq!(
            rust_output.status, c_output.status,
            "{label}: Rust/C process status mismatch"
        );
    }
}

#[test]
fn allocation_failure_error_row_1_and_boundary_b6() {
    let c_output = run_boundary_child(&c_library_path(), "oversized_suffix");
    let rust_output = run_boundary_child(&rust_library_path(), "oversized_suffix");

    assert_eq!(c_output.status.code(), Some(30), "C did not exit with 30");
    assert_eq!(
        rust_output.status.code(),
        c_output.status.code(),
        "Rust/C exit code mismatch"
    );
    assert_eq!(
        rust_output.stderr, c_output.stderr,
        "Rust/C allocation-failure stderr mismatch"
    );
}

#[test]
fn ffi_boundary_child() {
    let Some(library_path) = env::var_os("DIFF_CHILD_LIBRARY") else {
        return;
    };
    let scenario = env::var("DIFF_CHILD_SCENARIO").expect("child scenario");
    let api = unsafe { Api::load(Path::new(&library_path)) };
    let path = CString::new("directory/input").unwrap();
    let out_dir = CString::new("output").unwrap();

    unsafe {
        match scenario.as_str() {
            "null_extract_path" => {
                (api.extract_filename)(std::ptr::null(), b'/' as c_char);
            }
            "null_create_path" => {
                (api.create_filename)(std::ptr::null(), out_dir.as_ptr(), 0);
            }
            "null_create_out_dir" => {
                (api.create_filename)(path.as_ptr(), std::ptr::null(), 0);
            }
            "oversized_suffix" => {
                let suffix_len = usize::MAX / 2 - 4096;
                (api.create_filename)(path.as_ptr(), out_dir.as_ptr(), suffix_len);
            }
            other => panic!("unknown child scenario {other}"),
        }
    }

    std::process::exit(91);
}
