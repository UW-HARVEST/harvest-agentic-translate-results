use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;

type ExtractFilename =
    unsafe extern "C" fn(path: *const c_char, separator: c_char) -> *const c_char;
type CreateFilename = unsafe extern "C" fn(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char;

unsafe extern "C" {
    fn free(pointer: *mut c_void);
    fn setrlimit(resource: c_int, limits: *const RLimit) -> c_int;
}

#[repr(C)]
struct RLimit {
    current: u64,
    maximum: u64,
}

const RLIMIT_AS: c_int = 9;
const RANDOM_CASES_PER_ROW: usize = 64;

struct Api {
    _library: Library,
    extract_filename: ExtractFilename,
    create_filename: CreateFilename,
}

impl Api {
    fn load(path: &Path) -> Self {
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
    Empty,
    Basename,
    SingleSlash,
    MultipleSlashes,
    TrailingSlash,
}

#[derive(Clone, Copy)]
enum OutDirShape {
    NonTrailing,
    Trailing,
    EmptyPrecededByNonSlash,
    EmptyPrecededBySlash,
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

    fn range(&mut self, start: usize, end: usize) -> usize {
        assert!(start < end);
        start + self.next_u64() as usize % (end - start)
    }

    fn ascii_byte_excluding(&mut self, excluded: &[u8]) -> u8 {
        loop {
            let byte = self.range(1, 127) as u8;
            if !excluded.contains(&byte) {
                return byte;
            }
        }
    }

    fn bytes(&mut self, min_len: usize, max_len_exclusive: usize, excluded: &[u8]) -> Vec<u8> {
        let len = self.range(min_len, max_len_exclusive);
        (0..len)
            .map(|_| self.ascii_byte_excluding(excluded))
            .collect()
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

fn nul_terminate(mut bytes: Vec<u8>) -> Vec<u8> {
    assert!(!bytes.contains(&0));
    bytes.push(0);
    bytes
}

fn random_path(rng: &mut Rng, shape: PathShape) -> Vec<u8> {
    match shape {
        PathShape::Empty => Vec::new(),
        PathShape::Basename => rng.bytes(1, 48, &[b'/']),
        PathShape::SingleSlash => {
            let mut path = rng.bytes(1, 24, &[b'/']);
            path.push(b'/');
            path.extend(rng.bytes(1, 24, &[b'/']));
            path
        }
        PathShape::MultipleSlashes => {
            let mut path = rng.bytes(1, 16, &[b'/']);
            path.push(b'/');
            path.extend(rng.bytes(1, 16, &[b'/']));
            path.push(b'/');
            path.extend(rng.bytes(1, 16, &[b'/']));
            if rng.next_u64() & 1 == 0 {
                let insertion = rng.range(0, path.len());
                path.insert(insertion, b'/');
            }
            path
        }
        PathShape::TrailingSlash => {
            let mut path = rng.bytes(0, 32, &[b'/']);
            if rng.next_u64() & 1 == 0 {
                path.push(b'/');
                path.extend(rng.bytes(0, 12, &[b'/']));
            }
            path.push(b'/');
            path
        }
    }
}

fn random_out_dir(rng: &mut Rng, shape: OutDirShape) -> (Vec<u8>, usize) {
    match shape {
        OutDirShape::NonTrailing => {
            let mut bytes = rng.bytes(1, 40, &[]);
            if bytes.last() == Some(&b'/') {
                *bytes.last_mut().expect("non-empty output directory") = b'x';
            }
            let bytes = nul_terminate(bytes);
            (bytes, 0)
        }
        OutDirShape::Trailing => {
            let mut bytes = rng.bytes(0, 40, &[]);
            bytes.push(b'/');
            (nul_terminate(bytes), 0)
        }
        OutDirShape::EmptyPrecededByNonSlash => {
            let prefix_len = rng.range(1, 9);
            let mut storage = rng.bytes(prefix_len, prefix_len + 1, &[]);
            if storage.last() == Some(&b'/') {
                *storage.last_mut().expect("non-empty prefix") = b'x';
            }
            storage.push(0);
            (storage, prefix_len)
        }
        OutDirShape::EmptyPrecededBySlash => {
            let prefix_len = rng.range(1, 9);
            let mut storage = rng.bytes(prefix_len, prefix_len + 1, &[]);
            *storage.last_mut().expect("non-empty prefix") = b'/';
            storage.push(0);
            (storage, prefix_len)
        }
    }
}

unsafe fn output_bytes(
    function: CreateFilename,
    path: &[u8],
    out_dir_storage: &[u8],
    out_dir_offset: usize,
    suffix_len: usize,
) -> Vec<u8> {
    let path_pointer = path.as_ptr().cast::<c_char>();
    let out_dir_pointer = unsafe {
        out_dir_storage
            .as_ptr()
            .add(out_dir_offset)
            .cast::<c_char>()
    };
    let filename_len = path[..path.len() - 1]
        .iter()
        .rposition(|byte| *byte == b'/')
        .map_or(path.len() - 1, |position| path.len() - position - 2);
    let out_dir_len = out_dir_storage[out_dir_offset..]
        .iter()
        .position(|byte| *byte == 0)
        .expect("out directory must be NUL terminated");
    let allocation_len = out_dir_len + 1 + filename_len + suffix_len + 1;

    let result = unsafe { function(path_pointer, out_dir_pointer, suffix_len) };
    assert!(!result.is_null());
    let bytes = unsafe { std::slice::from_raw_parts(result.cast::<u8>(), allocation_len) }.to_vec();
    unsafe {
        free(result.cast());
    }
    bytes
}

fn compare_extract_case(
    c_api: &Api,
    rust_api: &Api,
    path_bytes: Vec<u8>,
    separator: u8,
    row: usize,
) {
    let path = nul_terminate(path_bytes);
    let pointer = path.as_ptr().cast::<c_char>();
    let c_result = unsafe { (c_api.extract_filename)(pointer, separator as c_char) };
    let rust_result = unsafe { (rust_api.extract_filename)(pointer, separator as c_char) };
    let c_offset = unsafe { c_result.offset_from(pointer) };
    let rust_offset = unsafe { rust_result.offset_from(pointer) };
    assert_eq!(
        c_offset, rust_offset,
        "CONFIGS.md row {row}: extractFilename pointer offset differs"
    );
}

#[test]
fn valid_extract_filename_rows_1_through_8() {
    let c_api = Api::load(&c_library_path());
    let rust_api = Api::load(&rust_library_path());
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    for _ in 0..RANDOM_CASES_PER_ROW {
        let separator = rng.ascii_byte_excluding(&[]);
        compare_extract_case(
            &c_api,
            &rust_api,
            rng.bytes(1, 64, &[separator]),
            separator,
            1,
        );

        let separator = rng.ascii_byte_excluding(&[]);
        let mut first = vec![separator];
        first.extend(rng.bytes(1, 48, &[separator]));
        compare_extract_case(&c_api, &rust_api, first, separator, 2);

        let separator = rng.ascii_byte_excluding(&[]);
        let mut middle = rng.bytes(1, 32, &[separator]);
        middle.push(separator);
        middle.extend(rng.bytes(1, 32, &[separator]));
        compare_extract_case(&c_api, &rust_api, middle, separator, 3);

        let separator = rng.ascii_byte_excluding(&[]);
        let mut multiple = rng.bytes(0, 16, &[separator]);
        multiple.push(separator);
        multiple.extend(rng.bytes(0, 16, &[separator]));
        multiple.push(separator);
        multiple.extend(rng.bytes(0, 16, &[separator]));
        compare_extract_case(&c_api, &rust_api, multiple, separator, 4);

        let separator = rng.ascii_byte_excluding(&[]);
        let mut final_separator = rng.bytes(0, 48, &[separator]);
        final_separator.push(separator);
        compare_extract_case(&c_api, &rust_api, final_separator, separator, 5);

        compare_extract_case(
            &c_api,
            &rust_api,
            Vec::new(),
            rng.ascii_byte_excluding(&[]),
            6,
        );

        compare_extract_case(&c_api, &rust_api, rng.bytes(0, 64, &[]), 0, 7);

        let high_separator = rng.range(128, 256) as u8;
        let mut high_bit = rng.bytes(0, 24, &[]);
        high_bit.push(high_separator);
        high_bit.extend(rng.bytes(0, 24, &[]));
        compare_extract_case(&c_api, &rust_api, high_bit, high_separator, 8);
    }
}

#[test]
fn valid_create_filename_rows_9_through_68() {
    let c_api = Api::load(&c_library_path());
    let rust_api = Api::load(&rust_library_path());
    let path_shapes = [
        PathShape::Empty,
        PathShape::Basename,
        PathShape::SingleSlash,
        PathShape::MultipleSlashes,
        PathShape::TrailingSlash,
    ];
    let out_dir_shapes = [
        OutDirShape::NonTrailing,
        OutDirShape::Trailing,
        OutDirShape::EmptyPrecededByNonSlash,
        OutDirShape::EmptyPrecededBySlash,
    ];
    let mut rng = Rng::new(0x94d0_49bb_1331_11eb);

    for (path_index, path_shape) in path_shapes.into_iter().enumerate() {
        for (out_dir_index, out_dir_shape) in out_dir_shapes.into_iter().enumerate() {
            for suffix_index in 0..3 {
                let row = 9 + path_index * 12 + out_dir_index * 3 + suffix_index;
                for _ in 0..RANDOM_CASES_PER_ROW {
                    let suffix_len = match suffix_index {
                        0 => 0,
                        1 => 1,
                        _ => rng.range(2, 65),
                    };
                    let path = nul_terminate(random_path(&mut rng, path_shape));
                    let path_prefix_len = rng.range(0, 8);
                    let mut path_storage = rng.bytes(path_prefix_len, path_prefix_len + 1, &[]);
                    path_storage.extend_from_slice(&path);
                    let path = &path_storage[path_prefix_len..];
                    let (out_dir_storage, out_dir_offset) = random_out_dir(&mut rng, out_dir_shape);
                    let c_output = unsafe {
                        output_bytes(
                            c_api.create_filename,
                            &path,
                            &out_dir_storage,
                            out_dir_offset,
                            suffix_len,
                        )
                    };
                    let rust_output = unsafe {
                        output_bytes(
                            rust_api.create_filename,
                            &path,
                            &out_dir_storage,
                            out_dir_offset,
                            suffix_len,
                        )
                    };
                    assert_eq!(
                        c_output, rust_output,
                        "CONFIGS.md row {row}: output allocation differs byte-for-byte"
                    );
                }
            }
        }
    }
}

fn run_boundary_child(library: &Path, case: &str) -> Output {
    Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("boundary_child")
        .arg("--ignored")
        .arg("--nocapture")
        .env("DRIVER_BOUNDARY_LIBRARY", library)
        .env("DRIVER_BOUNDARY_CASE", case)
        .env("RUST_TEST_THREADS", "1")
        .output()
        .unwrap_or_else(|error| panic!("failed to run boundary child for {case}: {error}"))
}

#[test]
fn error_and_generic_boundary_paths_match() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();

    for case in ["null_extract_path", "null_create_path", "null_out_dir"] {
        let c_output = run_boundary_child(&c_path, case);
        let rust_output = run_boundary_child(&rust_path, case);
        assert!(!c_output.status.success(), "C unexpectedly accepted {case}");
        assert_eq!(
            c_output.status, rust_output.status,
            "termination status differs for {case}; C stderr={:?}, Rust stderr={:?}",
            c_output.stderr, rust_output.stderr
        );
        assert_eq!(
            c_output.stderr, rust_output.stderr,
            "stderr differs for {case}"
        );
    }

    let c_output = run_boundary_child(&c_path, "allocation_failure");
    let rust_output = run_boundary_child(&rust_path, "allocation_failure");
    assert_eq!(c_output.status.code(), Some(30));
    assert_eq!(rust_output.status.code(), Some(30));
    assert_eq!(
        c_output.stderr, rust_output.stderr,
        "ERRORS.md row 1: allocation-failure stderr differs"
    );
    assert!(
        c_output
            .stderr
            .starts_with(b"zstd: FIO_createFilename_fromOutDir: "),
        "ERRORS.md row 1: unexpected C stderr: {:?}",
        c_output.stderr
    );
}

#[test]
#[ignore = "invoked in an isolated subprocess by error_and_generic_boundary_paths_match"]
fn boundary_child() {
    let library_path = PathBuf::from(env::var_os("DRIVER_BOUNDARY_LIBRARY").expect("library path"));
    let case = env::var("DRIVER_BOUNDARY_CASE").expect("boundary case");
    let api = Api::load(&library_path);

    match case.as_str() {
        "null_extract_path" => unsafe {
            let _ = (api.extract_filename)(ptr::null(), b'/' as c_char);
        },
        "null_create_path" => unsafe {
            let out_dir = b"out\0";
            let result = (api.create_filename)(ptr::null(), out_dir.as_ptr().cast(), 0);
            if !result.is_null() {
                free(result.cast());
            }
        },
        "null_out_dir" => unsafe {
            let path = b"name\0";
            let result = (api.create_filename)(path.as_ptr().cast(), ptr::null(), 0);
            if !result.is_null() {
                free(result.cast());
            }
        },
        "allocation_failure" => unsafe {
            let limits = RLimit {
                current: 256 * 1024 * 1024,
                maximum: 256 * 1024 * 1024,
            };
            assert_eq!(setrlimit(RLIMIT_AS, &limits), 0);
            let path = b"name\0";
            let out_dir = b"out\0";
            let result = (api.create_filename)(
                path.as_ptr().cast(),
                out_dir.as_ptr().cast(),
                512 * 1024 * 1024,
            );
            if !result.is_null() {
                free(result.cast());
            }
        },
        other => panic!("unknown boundary case {other}"),
    }
}
