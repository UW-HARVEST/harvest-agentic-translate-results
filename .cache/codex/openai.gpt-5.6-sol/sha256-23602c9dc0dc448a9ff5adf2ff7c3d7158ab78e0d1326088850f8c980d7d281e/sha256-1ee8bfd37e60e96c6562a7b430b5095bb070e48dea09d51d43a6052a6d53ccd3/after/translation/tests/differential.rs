use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_long, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, ptr, slice};

type ExtractFilename = unsafe extern "C" fn(*const c_char, c_char) -> *const c_char;
type CreateFilename = unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char;

const C_LIBRARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../c_src/build/libdriver.so");
const RUST_LIBRARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/release/libdriver.so");
const RANDOM_CASES: usize = 128;

unsafe extern "C" {
    fn free(pointer: *mut c_void);
    fn mmap(
        address: *mut c_void,
        length: usize,
        protection: c_int,
        flags: c_int,
        descriptor: c_int,
        offset: isize,
    ) -> *mut c_void;
    fn mprotect(address: *mut c_void, length: usize, protection: c_int) -> c_int;
    fn sysconf(name: c_int) -> c_long;
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
                .expect("missing extractFilename")
        };
        let create_filename = unsafe {
            *library
                .get::<CreateFilename>(b"FIO_createFilename_fromOutDir\0")
                .expect("missing FIO_createFilename_fromOutDir")
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
    NoSlash,
    OneSlash,
    ManySlashes,
    TrailingSlash,
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

    fn usize_in(&mut self, start: usize, end: usize) -> usize {
        start + self.next_u64() as usize % (end - start)
    }

    fn segment(&mut self, min_len: usize, max_len: usize) -> Vec<u8> {
        const ALPHABET: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-";
        let length = self.usize_in(min_len, max_len + 1);
        (0..length)
            .map(|_| ALPHABET[self.usize_in(0, ALPHABET.len())])
            .collect()
    }
}

fn generated_path(rng: &mut Rng, shape: PathShape) -> Vec<u8> {
    match shape {
        PathShape::Empty => Vec::new(),
        PathShape::NoSlash => rng.segment(1, 64),
        PathShape::OneSlash => {
            let mut path = rng.segment(1, 32);
            path.push(b'/');
            path.extend(rng.segment(1, 32));
            path
        }
        PathShape::ManySlashes => {
            let mut path = rng.segment(1, 16);
            for _ in 0..rng.usize_in(2, 6) {
                path.push(b'/');
                path.extend(rng.segment(1, 16));
            }
            path
        }
        PathShape::TrailingSlash => {
            let mut path = rng.segment(1, 64);
            path.push(b'/');
            path
        }
    }
}

fn generated_out_dir(rng: &mut Rng, trailing_slash: bool) -> Vec<u8> {
    let mut out_dir = Vec::from(b"/tmp/".as_slice());
    out_dir.extend(rng.segment(1, 48));
    if trailing_slash {
        out_dir.push(b'/');
    }
    out_dir
}

unsafe fn compare_extract(path: &[u8], separator: c_char) {
    let c_api = unsafe { Api::load(Path::new(C_LIBRARY)) };
    let rust_api = unsafe { Api::load(Path::new(RUST_LIBRARY)) };
    let c_path = CString::new(path).expect("generated path contains NUL");
    let rust_path = CString::new(path).expect("generated path contains NUL");

    let c_result = unsafe { (c_api.extract_filename)(c_path.as_ptr(), separator) };
    let rust_result = unsafe { (rust_api.extract_filename)(rust_path.as_ptr(), separator) };
    assert!(!c_result.is_null());
    assert!(!rust_result.is_null());

    let c_offset = c_result as usize - c_path.as_ptr() as usize;
    let rust_offset = rust_result as usize - rust_path.as_ptr() as usize;
    assert_eq!(
        rust_offset, c_offset,
        "path={path:?}, separator={separator}"
    );
    assert!(
        c_offset <= path.len() + 1,
        "C returned an out-of-range relative pointer"
    );

    if c_offset <= path.len() {
        assert_eq!(
            &rust_path.as_bytes_with_nul()[rust_offset..],
            &c_path.as_bytes_with_nul()[c_offset..],
            "path={path:?}, separator={separator}"
        );
    }
}

unsafe fn compare_create(path: &[u8], out_dir: &[u8], suffix_len: usize) {
    let c_api = unsafe { Api::load(Path::new(C_LIBRARY)) };
    let rust_api = unsafe { Api::load(Path::new(RUST_LIBRARY)) };
    let path = CString::new(path).expect("generated path contains NUL");
    let out_dir = CString::new(out_dir).expect("generated output directory contains NUL");

    let c_result = unsafe { (c_api.create_filename)(path.as_ptr(), out_dir.as_ptr(), suffix_len) };
    let rust_result =
        unsafe { (rust_api.create_filename)(path.as_ptr(), out_dir.as_ptr(), suffix_len) };
    assert!(!c_result.is_null());
    assert!(!rust_result.is_null());

    let path_bytes = path.as_bytes();
    let filename_len = path_bytes
        .iter()
        .rposition(|byte| *byte == b'/')
        .map_or(path_bytes.len(), |position| path_bytes.len() - position - 1);
    let allocation_size = out_dir.as_bytes().len() + 1 + filename_len + suffix_len + 1;
    let c_bytes = unsafe { slice::from_raw_parts(c_result.cast::<u8>(), allocation_size) };
    let rust_bytes = unsafe { slice::from_raw_parts(rust_result.cast::<u8>(), allocation_size) };
    assert_eq!(
        rust_bytes, c_bytes,
        "path={path:?}, out_dir={out_dir:?}, suffix_len={suffix_len}"
    );

    unsafe {
        free(c_result.cast());
        free(rust_result.cast());
    }
}

fn exercise_fio_row(seed: u64, shape: PathShape, trailing_slash: bool, positive_suffix: bool) {
    let mut rng = Rng::new(seed);
    for _ in 0..RANDOM_CASES {
        let path = generated_path(&mut rng, shape);
        let out_dir = generated_out_dir(&mut rng, trailing_slash);
        let suffix_len = if positive_suffix {
            rng.usize_in(1, 513)
        } else {
            0
        };
        unsafe { compare_create(&path, &out_dir, suffix_len) };
    }
}

#[test]
fn config_01_extract_empty_path_separator_absent() {
    let mut rng = Rng::new(0x0101);
    for _ in 0..RANDOM_CASES {
        let separator = rng.usize_in(1, 128) as c_char;
        unsafe { compare_extract(b"", separator) };
    }
}

#[test]
fn config_02_extract_nonempty_separator_absent() {
    let mut rng = Rng::new(0x0202);
    for _ in 0..RANDOM_CASES {
        let path = rng.segment(1, 128);
        unsafe { compare_extract(&path, b'/' as c_char) };
    }
}

#[test]
fn config_03_extract_one_separator() {
    let mut rng = Rng::new(0x0303);
    for _ in 0..RANDOM_CASES {
        let path = generated_path(&mut rng, PathShape::OneSlash);
        unsafe { compare_extract(&path, b'/' as c_char) };
    }
}

#[test]
fn config_04_extract_multiple_separators() {
    let mut rng = Rng::new(0x0404);
    for _ in 0..RANDOM_CASES {
        let path = generated_path(&mut rng, PathShape::ManySlashes);
        unsafe { compare_extract(&path, b'/' as c_char) };
    }
}

#[test]
fn config_05_extract_trailing_separator() {
    let mut rng = Rng::new(0x0505);
    for _ in 0..RANDOM_CASES {
        let path = generated_path(&mut rng, PathShape::TrailingSlash);
        unsafe { compare_extract(&path, b'/' as c_char) };
    }
}

#[test]
fn config_06_extract_nul_separator() {
    let mut rng = Rng::new(0x0606);
    for _ in 0..RANDOM_CASES {
        let path = rng.segment(0, 128);
        unsafe { compare_extract(&path, 0) };
    }
}

macro_rules! fio_config_test {
    ($name:ident, $seed:expr, $shape:expr, $trailing:expr, $positive:expr) => {
        #[test]
        fn $name() {
            exercise_fio_row($seed, $shape, $trailing, $positive);
        }
    };
}

fio_config_test!(
    config_07_fio_empty_trailing_zero,
    0x0707,
    PathShape::Empty,
    true,
    false
);
fio_config_test!(
    config_08_fio_empty_trailing_positive,
    0x0808,
    PathShape::Empty,
    true,
    true
);
fio_config_test!(
    config_09_fio_empty_plain_zero,
    0x0909,
    PathShape::Empty,
    false,
    false
);
fio_config_test!(
    config_10_fio_empty_plain_positive,
    0x1010,
    PathShape::Empty,
    false,
    true
);
fio_config_test!(
    config_11_fio_no_slash_trailing_zero,
    0x1111,
    PathShape::NoSlash,
    true,
    false
);
fio_config_test!(
    config_12_fio_no_slash_trailing_positive,
    0x1212,
    PathShape::NoSlash,
    true,
    true
);
fio_config_test!(
    config_13_fio_no_slash_plain_zero,
    0x1313,
    PathShape::NoSlash,
    false,
    false
);
fio_config_test!(
    config_14_fio_no_slash_plain_positive,
    0x1414,
    PathShape::NoSlash,
    false,
    true
);
fio_config_test!(
    config_15_fio_one_slash_trailing_zero,
    0x1515,
    PathShape::OneSlash,
    true,
    false
);
fio_config_test!(
    config_16_fio_one_slash_trailing_positive,
    0x1616,
    PathShape::OneSlash,
    true,
    true
);
fio_config_test!(
    config_17_fio_one_slash_plain_zero,
    0x1717,
    PathShape::OneSlash,
    false,
    false
);
fio_config_test!(
    config_18_fio_one_slash_plain_positive,
    0x1818,
    PathShape::OneSlash,
    false,
    true
);
fio_config_test!(
    config_19_fio_many_slash_trailing_zero,
    0x1919,
    PathShape::ManySlashes,
    true,
    false
);
fio_config_test!(
    config_20_fio_many_slash_trailing_positive,
    0x2020,
    PathShape::ManySlashes,
    true,
    true
);
fio_config_test!(
    config_21_fio_many_slash_plain_zero,
    0x2121,
    PathShape::ManySlashes,
    false,
    false
);
fio_config_test!(
    config_22_fio_many_slash_plain_positive,
    0x2222,
    PathShape::ManySlashes,
    false,
    true
);
fio_config_test!(
    config_23_fio_trailing_path_trailing_zero,
    0x2323,
    PathShape::TrailingSlash,
    true,
    false
);
fio_config_test!(
    config_24_fio_trailing_path_trailing_positive,
    0x2424,
    PathShape::TrailingSlash,
    true,
    true
);
fio_config_test!(
    config_25_fio_trailing_path_plain_zero,
    0x2525,
    PathShape::TrailingSlash,
    false,
    false
);
fio_config_test!(
    config_26_fio_trailing_path_plain_positive,
    0x2626,
    PathShape::TrailingSlash,
    false,
    true
);

fn run_boundary_child(library: &str, case: &str) -> Output {
    Command::new(env::current_exe().expect("test executable path"))
        .args([
            "--exact",
            "ffi_boundary_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DIFFERENTIAL_CHILD_LIBRARY", library)
        .env("DIFFERENTIAL_CHILD_CASE", case)
        .output()
        .unwrap_or_else(|error| panic!("failed to run boundary child {case}: {error}"))
}

#[cfg(unix)]
fn assert_same_signal(c_output: &Output, rust_output: &Output, case: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_signal = c_output.status.signal();
    let rust_signal = rust_output.status.signal();
    assert!(
        c_signal.is_some(),
        "C did not terminate by signal for {case}"
    );
    assert_eq!(
        rust_signal, c_signal,
        "different terminating signal for {case}"
    );
}

#[test]
fn error_01_and_g6_allocation_failure() {
    let c_output = run_boundary_child(C_LIBRARY, "allocation_failure");
    let rust_output = run_boundary_child(RUST_LIBRARY, "allocation_failure");
    assert_eq!(c_output.status.code(), Some(30), "C exit status");
    assert_eq!(rust_output.status.code(), Some(30), "Rust exit status");
    assert_eq!(rust_output.stderr, c_output.stderr, "allocation stderr");
}

#[test]
fn error_g1_extract_null_path() {
    let c_output = run_boundary_child(C_LIBRARY, "extract_null_path");
    let rust_output = run_boundary_child(RUST_LIBRARY, "extract_null_path");
    assert_same_signal(&c_output, &rust_output, "extract_null_path");
}

#[test]
fn error_g2_fio_null_path() {
    let c_output = run_boundary_child(C_LIBRARY, "fio_null_path");
    let rust_output = run_boundary_child(RUST_LIBRARY, "fio_null_path");
    assert_same_signal(&c_output, &rust_output, "fio_null_path");
}

#[test]
fn error_g3_fio_null_out_dir() {
    let c_output = run_boundary_child(C_LIBRARY, "fio_null_out_dir");
    let rust_output = run_boundary_child(RUST_LIBRARY, "fio_null_out_dir");
    assert_same_signal(&c_output, &rust_output, "fio_null_out_dir");
}

#[test]
fn error_g4_fio_empty_out_dir_guard_page() {
    let c_output = run_boundary_child(C_LIBRARY, "fio_empty_out_dir");
    let rust_output = run_boundary_child(RUST_LIBRARY, "fio_empty_out_dir");
    assert_same_signal(&c_output, &rust_output, "fio_empty_out_dir");
}

#[test]
fn error_g5_zero_suffix_is_valid() {
    unsafe { compare_create(b"dir/input.bin", b"/output", 0) };
}

#[test]
fn ffi_boundary_child() {
    let Ok(library_path) = env::var("DIFFERENTIAL_CHILD_LIBRARY") else {
        return;
    };
    let case = env::var("DIFFERENTIAL_CHILD_CASE").expect("boundary case");
    let api = unsafe { Api::load(PathBuf::from(library_path).as_path()) };
    let path = CString::new("dir/input.bin").unwrap();
    let out_dir = CString::new("/output").unwrap();

    match case.as_str() {
        "allocation_failure" => unsafe {
            (api.create_filename)(path.as_ptr(), out_dir.as_ptr(), isize::MAX as usize);
        },
        "extract_null_path" => unsafe {
            (api.extract_filename)(ptr::null(), b'/' as c_char);
        },
        "fio_null_path" => unsafe {
            (api.create_filename)(ptr::null(), out_dir.as_ptr(), 0);
        },
        "fio_null_out_dir" => unsafe {
            (api.create_filename)(path.as_ptr(), ptr::null(), 0);
        },
        "fio_empty_out_dir" => unsafe {
            const PROT_NONE: c_int = 0;
            const PROT_READ: c_int = 1;
            const PROT_WRITE: c_int = 2;
            const MAP_PRIVATE: c_int = 2;
            const MAP_ANONYMOUS: c_int = 0x20;
            const SC_PAGESIZE: c_int = 30;

            let page_size = sysconf(SC_PAGESIZE) as usize;
            assert!(page_size > 0);
            let mapping = mmap(
                ptr::null_mut(),
                page_size * 2,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert_ne!(mapping as isize, -1, "mmap failed");
            assert_eq!(mprotect(mapping, page_size, PROT_NONE), 0);
            let empty_out_dir = mapping.cast::<u8>().add(page_size);
            *empty_out_dir = 0;
            (api.create_filename)(path.as_ptr(), empty_out_dir.cast(), 0);
        },
        _ => panic!("unknown boundary case {case}"),
    }
}
