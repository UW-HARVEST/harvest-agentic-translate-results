use libloading::Library;
use std::ffi::c_int;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

type InitArray = unsafe extern "C" fn(usize) -> *mut DynamicArray;
type Matrixsum = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type SetFailure = unsafe extern "C" fn(usize);

struct Api {
    _library: Library,
    init_array: InitArray,
    matrixsum: Matrixsum,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let init_array = unsafe { *library.get(b"init_array\0").unwrap() };
        let matrixsum = unsafe { *library.get(b"matrixsum\0").unwrap() };
        Self {
            _library: library,
            init_array,
            matrixsum,
        }
    }
}

fn c_library_path() -> PathBuf {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();
    assert_eq!(candidates.len(), 1);
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libmatrixsum_lib.so")
}

fn interposer_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/fail-alloc/libfail_alloc.so")
}

#[test]
fn allocator_failure_rows() {
    if std::env::var_os("MATRIXSUM_ALLOC_FAILURE_CHILD").is_some() {
        return;
    }

    let output = interposer_path();
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let compile = Command::new("cc")
        .args(["-shared", "-fPIC", "tests/fail_alloc.c", "-o"])
        .arg(&output)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("failed to run cc for allocator interposer");
    assert!(compile.success(), "allocator interposer compilation failed");
    assert!(output.is_file(), "allocator interposer was not produced");

    let status = Command::new(std::env::current_exe().unwrap())
        .args(["allocation_failure_child", "--exact", "--nocapture"])
        .env("LD_PRELOAD", &output)
        .env("MATRIXSUM_ALLOC_FAILURE_CHILD", "1")
        .env("MATRIXSUM_ALLOC_INTERPOSER", &output)
        .status()
        .expect("failed to start allocator-failure child");
    assert!(status.success(), "allocator-failure child failed");
}

#[test]
fn allocation_failure_child() {
    if std::env::var_os("MATRIXSUM_ALLOC_FAILURE_CHILD").is_none() {
        return;
    }

    unsafe {
        let c = Api::load(&c_library_path());
        let rust = Api::load(&rust_library_path());
        let interposer_path =
            PathBuf::from(std::env::var_os("MATRIXSUM_ALLOC_INTERPOSER").unwrap());
        let interposer = Library::new(&interposer_path).unwrap();
        let set_failure: SetFailure = *interposer
            .get(b"matrixsum_fail_allocation_of_size\0")
            .unwrap();

        // E1: fail the 24-byte DynamicArray object allocation.
        set_failure(size_of::<DynamicArray>());
        let c_result = (c.init_array)(2);
        assert!(c_result.is_null());
        set_failure(size_of::<DynamicArray>());
        let rust_result = (rust.init_array)(2);
        assert!(rust_result.is_null());

        // E2: object allocation succeeds, then the 2-int data allocation fails.
        set_failure(2 * size_of::<c_int>());
        let c_result = (c.init_array)(2);
        assert!(c_result.is_null());
        set_failure(2 * size_of::<c_int>());
        let rust_result = (rust.init_array)(2);
        assert!(rust_result.is_null());

        // E8: matrixsum propagates the same first-allocation failure as -1.
        set_failure(size_of::<DynamicArray>());
        let c_result = (c.matrixsum)(1, 2, 3, 4);
        assert_eq!(c_result, -1);
        set_failure(size_of::<DynamicArray>());
        let rust_result = (rust.matrixsum)(1, 2, 3, 4);
        assert_eq!(rust_result, -1);
        assert_eq!(c_result, rust_result);
    }
}
