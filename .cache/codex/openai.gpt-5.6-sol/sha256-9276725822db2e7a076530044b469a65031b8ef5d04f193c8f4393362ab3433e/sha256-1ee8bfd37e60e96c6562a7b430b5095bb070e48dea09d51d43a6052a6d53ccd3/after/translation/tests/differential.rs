use libloading::Library;
use std::env;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

type Matrix = [[c_int; 4]; 3];

#[repr(C)]
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
type MatrixChecksum = unsafe extern "C" fn() -> c_int;
type MatrixSum = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    init_array: InitArray,
    expand_array: ExpandArray,
    add_element: AddElement,
    free_array: FreeArray,
    process_flags: ProcessFlags,
    matrix_checksum: MatrixChecksum,
    matrixsum: MatrixSum,
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
        let matrix_checksum = unsafe { *library.get(b"calculate_matrix_checksum\0").unwrap() };
        let matrixsum = unsafe { *library.get(b"matrixsum\0").unwrap() };
        let matrix = unsafe { *library.get::<*mut Matrix>(b"matrix\0").unwrap() };
        Self {
            _library: library,
            init_array,
            expand_array,
            add_element,
            free_array,
            process_flags,
            matrix_checksum,
            matrixsum,
            matrix,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ArraySnapshot {
    size: usize,
    capacity: usize,
    values: Vec<c_int>,
}

unsafe fn snapshot(array: *mut DynamicArray) -> ArraySnapshot {
    assert!(!array.is_null());
    let size = unsafe { (*array).size };
    let capacity = unsafe { (*array).capacity };
    let values = unsafe { std::slice::from_raw_parts((*array).data, size) }.to_vec();
    ArraySnapshot {
        size,
        capacity,
        values,
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-IPImlt.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libmatrixsum_lib.so")
}

unsafe fn load_apis() -> (Api, Api) {
    (unsafe { Api::load(&c_library_path()) }, unsafe {
        Api::load(&rust_library_path())
    })
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

    fn i32(&mut self) -> c_int {
        self.next_u64() as c_int
    }

    fn nonzero_i32(&mut self) -> c_int {
        loop {
            let value = self.i32();
            if value != 0 {
                return value;
            }
        }
    }

    fn usize_in(&mut self, start: usize, end: usize) -> usize {
        start + self.next_u64() as usize % (end - start)
    }

    fn matrix(&mut self) -> Matrix {
        std::array::from_fn(|_| std::array::from_fn(|_| self.i32()))
    }
}

unsafe fn initialize_pair(
    c: &Api,
    rust: &Api,
    capacity: usize,
) -> (*mut DynamicArray, *mut DynamicArray) {
    let c_array = unsafe { (c.init_array)(capacity) };
    let rust_array = unsafe { (rust.init_array)(capacity) };
    assert_eq!(
        c_array.is_null(),
        rust_array.is_null(),
        "init_array({capacity}) nullness differs"
    );
    assert!(!c_array.is_null(), "test allocation unexpectedly failed");
    (c_array, rust_array)
}

unsafe fn add_pair(
    c: &Api,
    rust: &Api,
    c_array: *mut DynamicArray,
    rust_array: *mut DynamicArray,
    value: c_int,
) {
    let c_result = unsafe { (c.add_element)(c_array, value) };
    let rust_result = unsafe { (rust.add_element)(rust_array, value) };
    assert_eq!(c_result, rust_result);
    assert_eq!(unsafe { snapshot(c_array) }, unsafe {
        snapshot(rust_array)
    });
}

unsafe fn free_pair(
    c: &Api,
    rust: &Api,
    c_array: *mut DynamicArray,
    rust_array: *mut DynamicArray,
) {
    unsafe {
        (c.free_array)(c_array);
        (rust.free_array)(rust_array);
    }
}

#[test]
fn valid_configuration_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_apis() };
    let mut rng = Rng::new(0x4d41_5452_4958_5355);

    // CONFIGS rows 1-2: initialized empty arrays for both capacity classes.
    for row in 1..=2 {
        for _ in 0..128 {
            let capacity = if row == 1 { 1 } else { rng.usize_in(2, 65) };
            let (c_array, rust_array) = unsafe { initialize_pair(&c, &rust, capacity) };
            assert_eq!(unsafe { snapshot(c_array) }, unsafe {
                snapshot(rust_array)
            });
            unsafe { free_pair(&c, &rust, c_array, rust_array) };
        }
    }

    // CONFIGS rows 3-4: appending below capacity and exactly filling capacity.
    for row in 3..=4 {
        for _ in 0..128 {
            let capacity = rng.usize_in(2, 33);
            let count = if row == 3 {
                rng.usize_in(1, capacity)
            } else {
                capacity
            };
            let (c_array, rust_array) = unsafe { initialize_pair(&c, &rust, capacity) };
            for _ in 0..count {
                unsafe { add_pair(&c, &rust, c_array, rust_array, rng.i32()) };
            }
            assert_eq!(unsafe { (*c_array).capacity }, capacity);
            assert_eq!(unsafe { (*rust_array).capacity }, capacity);
            unsafe { free_pair(&c, &rust, c_array, rust_array) };
        }
    }

    // CONFIGS row 5: an append at capacity performs one doubling.
    for _ in 0..128 {
        let capacity = rng.usize_in(1, 33);
        let (c_array, rust_array) = unsafe { initialize_pair(&c, &rust, capacity) };
        for _ in 0..=capacity {
            unsafe { add_pair(&c, &rust, c_array, rust_array, rng.i32()) };
        }
        assert_eq!(unsafe { (*c_array).capacity }, capacity * 2);
        assert_eq!(unsafe { (*rust_array).capacity }, capacity * 2);
        unsafe { free_pair(&c, &rust, c_array, rust_array) };
    }

    // CONFIGS row 6: repeated expansion through the consumer-facing sequence.
    for _ in 0..128 {
        let count = rng.usize_in(33, 129);
        let (c_array, rust_array) = unsafe { initialize_pair(&c, &rust, 1) };
        for _ in 0..count {
            unsafe { add_pair(&c, &rust, c_array, rust_array, rng.i32()) };
        }
        unsafe { free_pair(&c, &rust, c_array, rust_array) };
    }

    // CONFIGS row 7: direct low-level expansion preserves size and values.
    for _ in 0..128 {
        let capacity = rng.usize_in(2, 65);
        let count = rng.usize_in(1, capacity);
        let (c_array, rust_array) = unsafe { initialize_pair(&c, &rust, capacity) };
        for _ in 0..count {
            unsafe { add_pair(&c, &rust, c_array, rust_array, rng.i32()) };
        }
        assert_eq!(unsafe { (c.expand_array)(c_array) }, unsafe {
            (rust.expand_array)(rust_array)
        });
        assert_eq!(unsafe { snapshot(c_array) }, unsafe {
            snapshot(rust_array)
        });
        unsafe { free_pair(&c, &rust, c_array, rust_array) };
    }

    // CONFIGS row 8: null free is an idempotent no-op.
    for _ in 0..128 {
        unsafe {
            (c.free_array)(ptr::null_mut());
            (rust.free_array)(ptr::null_mut());
        }
    }

    // CONFIGS row 9: nonnull arrays can be freed after randomized use.
    for _ in 0..128 {
        let capacity = rng.usize_in(1, 17);
        let count = rng.usize_in(0, 65);
        let (c_array, rust_array) = unsafe { initialize_pair(&c, &rust, capacity) };
        for _ in 0..count {
            unsafe { add_pair(&c, &rust, c_array, rust_array, rng.i32()) };
        }
        unsafe { free_pair(&c, &rust, c_array, rust_array) };
    }

    // CONFIGS rows 10-12: exported object bytes and checksums, default then random.
    let expected_default: Matrix = [
        [0x01, 0x02, 0x03, 0x04],
        [0x10, 0x20, 0x30, 0x40],
        [0xA1, 0xB2, 0xC3, 0xD4],
    ];
    assert_eq!(unsafe { *c.matrix }, expected_default);
    assert_eq!(unsafe { *rust.matrix }, expected_default);
    assert_eq!(unsafe { (c.matrix_checksum)() }, unsafe {
        (rust.matrix_checksum)()
    });
    for _ in 0..256 {
        let matrix = rng.matrix();
        unsafe {
            *c.matrix = matrix;
            *rust.matrix = matrix;
        }
        assert_eq!(unsafe { *c.matrix }, unsafe { *rust.matrix });
        assert_eq!(unsafe { (c.matrix_checksum)() }, unsafe {
            (rust.matrix_checksum)()
        });
    }

    // CONFIGS rows 13-28: all known-bit masks with randomized ignored bits.
    for known_mask in 0..=0xF {
        for _ in 0..256 {
            let flags = (rng.i32() & !0xF) | known_mask;
            assert_eq!(
                unsafe { (c.process_flags)(flags) },
                unsafe { (rust.process_flags)(flags) },
                "process_flags differs for known mask {known_mask:#x}, flags {flags:#x}"
            );
        }
    }

    // CONFIGS rows 29-44: every parameter truthiness mask and randomized matrix.
    for nonzero_mask in 0..=0xF {
        for _ in 0..256 {
            let params: [c_int; 4] = std::array::from_fn(|index| {
                if nonzero_mask & (1 << index) == 0 {
                    0
                } else {
                    rng.nonzero_i32()
                }
            });
            let matrix = rng.matrix();
            unsafe {
                *c.matrix = matrix;
                *rust.matrix = matrix;
            }
            assert_eq!(
                unsafe { (c.matrixsum)(params[0], params[1], params[2], params[3]) },
                unsafe { (rust.matrixsum)(params[0], params[1], params[2], params[3]) },
                "matrixsum differs for nonzero mask {nonzero_mask:#x}, params {params:?}"
            );
        }
    }
}

#[test]
fn generic_ffi_boundaries_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_apis() };

    assert_eq!(unsafe { (c.expand_array)(ptr::null_mut()) }, unsafe {
        (rust.expand_array)(ptr::null_mut())
    });
    assert_eq!(
        unsafe { (c.add_element)(ptr::null_mut(), c_int::MIN) },
        unsafe { (rust.add_element)(ptr::null_mut(), c_int::MIN) }
    );
    unsafe {
        (c.free_array)(ptr::null_mut());
        (rust.free_array)(ptr::null_mut());
    }

    for capacity in [0, usize::MAX] {
        let c_array = unsafe { (c.init_array)(capacity) };
        let rust_array = unsafe { (rust.init_array)(capacity) };
        assert_eq!(
            c_array.is_null(),
            rust_array.is_null(),
            "capacity {capacity}"
        );
        if !c_array.is_null() {
            assert_eq!(unsafe { snapshot(c_array) }, unsafe {
                snapshot(rust_array)
            });
            unsafe { free_pair(&c, &rust, c_array, rust_array) };
        }
    }

    for flags in [c_int::MIN, -1, 0x10, 0x100, c_int::MAX] {
        assert_eq!(unsafe { (c.process_flags)(flags) }, unsafe {
            (rust.process_flags)(flags)
        });
    }

    let boundary_parameters = [
        [c_int::MIN, 0, 0, 0],
        [c_int::MAX, 0, 0, 0],
        [c_int::MIN, c_int::MAX, -1, 1],
        [c_int::MAX, c_int::MAX, c_int::MAX, c_int::MAX],
    ];
    let matrix = [[0; 4]; 3];
    unsafe {
        *c.matrix = matrix;
        *rust.matrix = matrix;
    }
    for params in boundary_parameters {
        assert_eq!(
            unsafe { (c.matrixsum)(params[0], params[1], params[2], params[3]) },
            unsafe { (rust.matrixsum)(params[0], params[1], params[2], params[3]) },
            "boundary params {params:?}"
        );
    }
}

fn interposer_path() -> PathBuf {
    manifest_dir().join("target/test-support/libfail_alloc.so")
}

fn build_interposer() -> PathBuf {
    let output = interposer_path();
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    let status = Command::new("cc")
        .args(["-std=c11", "-shared", "-fPIC", "-O2"])
        .arg(manifest_dir().join("tests/support/fail_alloc.c"))
        .arg("-o")
        .arg(&output)
        .status()
        .expect("failed to invoke cc for allocator interposer");
    assert!(status.success(), "allocator interposer compilation failed");
    assert!(output.is_file(), "allocator interposer was not produced");
    output
}

#[test]
fn allocator_fault_injection_errors_match() {
    if env::var_os("MATRIXSUM_ALLOC_PROBE").is_some() {
        return;
    }

    let _guard = TEST_LOCK.lock().unwrap();
    let interposer = build_interposer();
    let status = Command::new(env::current_exe().unwrap())
        .args(["allocator_fault_probe", "--exact", "--nocapture"])
        .env("MATRIXSUM_ALLOC_PROBE", "1")
        .env("LD_PRELOAD", &interposer)
        .status()
        .expect("failed to run allocator fault probe");
    assert!(status.success(), "allocator fault probe failed");
}

#[test]
fn allocator_fault_probe() {
    if env::var_os("MATRIXSUM_ALLOC_PROBE").is_none() {
        return;
    }

    type FailAfter = unsafe extern "C" fn(isize);
    type FailFinish = unsafe extern "C" fn() -> usize;

    let interposer = unsafe { Library::new(interposer_path()) }.unwrap();
    let fail_after: FailAfter = unsafe { *interposer.get(b"fail_alloc_after\0").unwrap() };
    let fail_finish: FailFinish = unsafe { *interposer.get(b"fail_alloc_finish\0").unwrap() };
    let (c, rust) = unsafe { load_apis() };

    // ERRORS row 1: first allocation in init_array fails.
    unsafe { fail_after(0) };
    let c_result = unsafe { (c.init_array)(8) };
    let c_frees = unsafe { fail_finish() };
    unsafe { fail_after(0) };
    let rust_result = unsafe { (rust.init_array)(8) };
    let rust_frees = unsafe { fail_finish() };
    assert!(c_result.is_null());
    assert!(rust_result.is_null());
    assert_eq!((c_frees, rust_frees), (0, 0));

    // ERRORS row 2: the data allocation fails and the object is freed.
    unsafe { fail_after(1) };
    let c_result = unsafe { (c.init_array)(8) };
    let c_frees = unsafe { fail_finish() };
    unsafe { fail_after(1) };
    let rust_result = unsafe { (rust.init_array)(8) };
    let rust_frees = unsafe { fail_finish() };
    assert!(c_result.is_null());
    assert!(rust_result.is_null());
    assert_eq!((c_frees, rust_frees), (1, 1));

    // ERRORS row 4: direct realloc failure leaves the array unchanged.
    let (c_array, rust_array) = unsafe { initialize_pair(&c, &rust, 2) };
    unsafe {
        add_pair(&c, &rust, c_array, rust_array, 11);
        add_pair(&c, &rust, c_array, rust_array, 22);
    }
    let before = unsafe { snapshot(c_array) };
    unsafe { fail_after(0) };
    let c_result = unsafe { (c.expand_array)(c_array) };
    let c_frees = unsafe { fail_finish() };
    unsafe { fail_after(0) };
    let rust_result = unsafe { (rust.expand_array)(rust_array) };
    let rust_frees = unsafe { fail_finish() };
    assert_eq!((c_result, rust_result), (0, 0));
    assert_eq!((c_frees, rust_frees), (0, 0));
    assert_eq!(unsafe { snapshot(c_array) }, before);
    assert_eq!(unsafe { snapshot(rust_array) }, before);
    unsafe { free_pair(&c, &rust, c_array, rust_array) };

    // ERRORS row 6: nested expansion failure prevents the append.
    let (c_array, rust_array) = unsafe { initialize_pair(&c, &rust, 1) };
    unsafe { add_pair(&c, &rust, c_array, rust_array, 33) };
    let before = unsafe { snapshot(c_array) };
    unsafe { fail_after(0) };
    let c_result = unsafe { (c.add_element)(c_array, 44) };
    let c_frees = unsafe { fail_finish() };
    unsafe { fail_after(0) };
    let rust_result = unsafe { (rust.add_element)(rust_array, 44) };
    let rust_frees = unsafe { fail_finish() };
    assert_eq!((c_result, rust_result), (0, 0));
    assert_eq!((c_frees, rust_frees), (0, 0));
    assert_eq!(unsafe { snapshot(c_array) }, before);
    assert_eq!(unsafe { snapshot(rust_array) }, before);
    unsafe { free_pair(&c, &rust, c_array, rust_array) };

    // ERRORS row 7: matrixsum propagates init_array object-allocation failure.
    unsafe { fail_after(0) };
    let c_result = unsafe { (c.matrixsum)(1, 2, 3, 4) };
    let c_frees = unsafe { fail_finish() };
    unsafe { fail_after(0) };
    let rust_result = unsafe { (rust.matrixsum)(1, 2, 3, 4) };
    let rust_frees = unsafe { fail_finish() };
    assert_eq!((c_result, rust_result), (-1, -1));
    assert_eq!((c_frees, rust_frees), (0, 0));
}

#[test]
fn null_rejection_errors_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { load_apis() };

    // ERRORS rows 3 and 5.
    assert_eq!(unsafe { (c.expand_array)(ptr::null_mut()) }, unsafe {
        (rust.expand_array)(ptr::null_mut())
    });
    assert_eq!(unsafe { (c.add_element)(ptr::null_mut(), 123) }, unsafe {
        (rust.add_element)(ptr::null_mut(), 123)
    });
}
