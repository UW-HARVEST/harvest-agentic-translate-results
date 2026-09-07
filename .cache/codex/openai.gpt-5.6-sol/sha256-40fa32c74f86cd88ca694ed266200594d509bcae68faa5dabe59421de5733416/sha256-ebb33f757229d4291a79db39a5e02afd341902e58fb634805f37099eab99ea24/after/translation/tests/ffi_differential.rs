use libloading::Library;
use std::ffi::{c_int, c_long};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::slice;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Debug)]
struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

type Matrix = [[c_int; 4]; 3];
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

        let init_array = unsafe { *library.get::<InitArray>(b"init_array\0").unwrap() };
        let expand_array = unsafe { *library.get::<ExpandArray>(b"expand_array\0").unwrap() };
        let add_element = unsafe { *library.get::<AddElement>(b"add_element\0").unwrap() };
        let free_array = unsafe { *library.get::<FreeArray>(b"free_array\0").unwrap() };
        let process_flags = unsafe { *library.get::<ProcessFlags>(b"process_flags\0").unwrap() };
        let calculate_matrix_checksum = unsafe {
            *library
                .get::<CalculateMatrixChecksum>(b"calculate_matrix_checksum\0")
                .unwrap()
        };
        let matrixsum = unsafe { *library.get::<Matrixsum>(b"matrixsum\0").unwrap() };
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

struct ApiPair {
    c: Api,
    rust: Api,
}

impl ApiPair {
    unsafe fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest
            .join("../c_src/build")
            .join("libharvest-work-c3ErL0.so");
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let rust_path = manifest
            .join("target")
            .join(profile)
            .join("libmatrixsum_lib.so");

        assert!(c_path.is_file(), "missing C DSO: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust DSO: {}",
            rust_path.display()
        );

        Self {
            c: unsafe { Api::load(&c_path) },
            rust: unsafe { Api::load(&rust_path) },
        }
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn bounded_i32(&mut self, magnitude: i32) -> i32 {
        let width = (magnitude as u32) * 2 + 1;
        (self.next_u32() % width) as i32 - magnitude
    }

    fn nonzero_bounded_i32(&mut self, magnitude: i32) -> i32 {
        loop {
            let value = self.bounded_i32(magnitude);
            if value != 0 {
                return value;
            }
        }
    }
}

unsafe fn snapshot(array: *mut DynamicArray) -> (usize, usize, Vec<c_int>) {
    assert!(!array.is_null());
    let size = unsafe { (*array).size };
    let capacity = unsafe { (*array).capacity };
    let values = if size == 0 {
        Vec::new()
    } else {
        unsafe { slice::from_raw_parts((*array).data, size) }.to_vec()
    };
    (size, capacity, values)
}

unsafe fn assert_arrays_equal(c_array: *mut DynamicArray, rust_array: *mut DynamicArray) {
    assert_eq!(unsafe { snapshot(c_array) }, unsafe {
        snapshot(rust_array)
    });
}

unsafe fn free_pair(pair: &ApiPair, c_array: *mut DynamicArray, rust_array: *mut DynamicArray) {
    unsafe {
        (pair.c.free_array)(c_array);
        (pair.rust.free_array)(rust_array);
    }
}

#[test]
fn valid_dynamic_array_configurations() {
    let _guard = TEST_LOCK.lock().unwrap();
    let pair = unsafe { ApiPair::load() };
    let mut rng = Rng::new(0x7d91_45ab_0c3e_f821);

    for capacity in [1, 2] {
        for _ in 0..128 {
            let c_array = unsafe { (pair.c.init_array)(capacity) };
            let rust_array = unsafe { (pair.rust.init_array)(capacity) };
            assert_eq!(c_array.is_null(), rust_array.is_null());
            assert!(!c_array.is_null());
            unsafe { assert_arrays_equal(c_array, rust_array) };
            assert_eq!(unsafe { (*c_array).capacity }, capacity);
            unsafe { free_pair(&pair, c_array, rust_array) };
        }
    }

    for _ in 0..256 {
        let capacity = 3 + (rng.next_u32() as usize % 30);
        let c_array = unsafe { (pair.c.init_array)(capacity) };
        let rust_array = unsafe { (pair.rust.init_array)(capacity) };
        assert!(!c_array.is_null() && !rust_array.is_null());
        unsafe { assert_arrays_equal(c_array, rust_array) };
        unsafe { free_pair(&pair, c_array, rust_array) };
    }

    for full in [false, true] {
        for _ in 0..256 {
            let capacity = 1 + (rng.next_u32() as usize % 16);
            let count = if full {
                capacity
            } else {
                rng.next_u32() as usize % capacity
            };
            let c_array = unsafe { (pair.c.init_array)(capacity) };
            let rust_array = unsafe { (pair.rust.init_array)(capacity) };
            for _ in 0..count {
                let value = rng.next_i32();
                assert_eq!(unsafe { (pair.c.add_element)(c_array, value) }, unsafe {
                    (pair.rust.add_element)(rust_array, value)
                });
            }
            assert_eq!(unsafe { (pair.c.expand_array)(c_array) }, unsafe {
                (pair.rust.expand_array)(rust_array)
            });
            unsafe { assert_arrays_equal(c_array, rust_array) };
            assert_eq!(unsafe { (*c_array).capacity }, capacity * 2);
            unsafe { free_pair(&pair, c_array, rust_array) };
        }
    }

    for expands in [false, true] {
        for _ in 0..256 {
            let capacity = 1 + (rng.next_u32() as usize % 16);
            let initial_count = if expands {
                capacity
            } else {
                rng.next_u32() as usize % capacity
            };
            let c_array = unsafe { (pair.c.init_array)(capacity) };
            let rust_array = unsafe { (pair.rust.init_array)(capacity) };
            for _ in 0..initial_count {
                let value = rng.next_i32();
                unsafe {
                    (pair.c.add_element)(c_array, value);
                    (pair.rust.add_element)(rust_array, value);
                }
            }
            let value = rng.next_i32();
            assert_eq!(unsafe { (pair.c.add_element)(c_array, value) }, unsafe {
                (pair.rust.add_element)(rust_array, value)
            });
            unsafe { assert_arrays_equal(c_array, rust_array) };
            unsafe { free_pair(&pair, c_array, rust_array) };
        }
    }

    for initial_capacity in [1, 2] {
        for _ in 0..256 {
            let count = if initial_capacity == 1 {
                3 + (rng.next_u32() as usize % 30)
            } else {
                4
            };
            let c_array = unsafe { (pair.c.init_array)(initial_capacity) };
            let rust_array = unsafe { (pair.rust.init_array)(initial_capacity) };
            for _ in 0..count {
                let value = rng.next_i32();
                assert_eq!(unsafe { (pair.c.add_element)(c_array, value) }, unsafe {
                    (pair.rust.add_element)(rust_array, value)
                });
                unsafe { assert_arrays_equal(c_array, rust_array) };
            }
            unsafe { free_pair(&pair, c_array, rust_array) };
        }
    }

    for _ in 0..128 {
        let c_array = unsafe { (pair.c.init_array)(4) };
        let rust_array = unsafe { (pair.rust.init_array)(4) };
        for _ in 0..4 {
            let value = rng.next_i32();
            unsafe {
                (pair.c.add_element)(c_array, value);
                (pair.rust.add_element)(rust_array, value);
            }
        }
        unsafe { assert_arrays_equal(c_array, rust_array) };
        unsafe { free_pair(&pair, c_array, rust_array) };
    }
}

#[test]
fn valid_process_flag_configurations() {
    let _guard = TEST_LOCK.lock().unwrap();
    let pair = unsafe { ApiPair::load() };
    let mut rng = Rng::new(0x43f0_d72b_92a6_185c);

    for low_mask in 0..16 {
        for _ in 0..512 {
            let flags = (rng.next_i32() & !0xF) | low_mask;
            let c_result = unsafe { (pair.c.process_flags)(flags) };
            let rust_result = unsafe { (pair.rust.process_flags)(flags) };
            assert_eq!(c_result, rust_result, "flags={flags:#034b}");
        }
    }
}

#[test]
fn valid_matrix_configurations() {
    let _guard = TEST_LOCK.lock().unwrap();
    let pair = unsafe { ApiPair::load() };
    let mut rng = Rng::new(0xa228_79dc_4f61_0b35);

    let c_original = unsafe { *pair.c.matrix };
    let rust_original = unsafe { *pair.rust.matrix };
    assert_eq!(c_original, rust_original);
    assert_eq!(unsafe { (pair.c.calculate_matrix_checksum)() }, unsafe {
        (pair.rust.calculate_matrix_checksum)()
    });

    for _ in 0..256 {
        let mut matrix = [[0; 4]; 3];
        for row in &mut matrix {
            for value in row {
                *value = rng.bounded_i32(50_000_000);
            }
        }
        unsafe {
            *pair.c.matrix = matrix;
            *pair.rust.matrix = matrix;
        }
        assert_eq!(unsafe { *pair.c.matrix }, unsafe { *pair.rust.matrix });
        assert_eq!(unsafe { (pair.c.calculate_matrix_checksum)() }, unsafe {
            (pair.rust.calculate_matrix_checksum)()
        });
    }

    unsafe {
        *pair.c.matrix = c_original;
        *pair.rust.matrix = rust_original;
    }
}

#[test]
fn valid_matrixsum_configurations() {
    let _guard = TEST_LOCK.lock().unwrap();
    let pair = unsafe { ApiPair::load() };
    let mut rng = Rng::new(0xe961_2ab4_7c05_d83f);

    for truth_mask in 0..16 {
        for _ in 0..256 {
            let mut parameters = [0; 4];
            for (index, parameter) in parameters.iter_mut().enumerate() {
                if truth_mask & (1 << index) != 0 {
                    *parameter = rng.nonzero_bounded_i32(1_000_000);
                }
            }
            let c_result = unsafe {
                (pair.c.matrixsum)(parameters[0], parameters[1], parameters[2], parameters[3])
            };
            let rust_result = unsafe {
                (pair.rust.matrixsum)(parameters[0], parameters[1], parameters[2], parameters[3])
            };
            assert_eq!(
                c_result, rust_result,
                "truth_mask={truth_mask:#06b}, parameters={parameters:?}"
            );
        }
    }

    let boundary_cases = [
        [c_int::MAX, 0, 0, 0],
        [c_int::MIN, 0, 0, 0],
        [c_int::MAX, c_int::MAX, c_int::MAX, c_int::MAX],
        [c_int::MIN, c_int::MIN, c_int::MIN, c_int::MIN],
        [c_int::MAX, c_int::MIN, 1, -1],
    ];
    for parameters in boundary_cases {
        assert_eq!(
            unsafe {
                (pair.c.matrixsum)(parameters[0], parameters[1], parameters[2], parameters[3])
            },
            unsafe {
                (pair.rust.matrixsum)(parameters[0], parameters[1], parameters[2], parameters[3])
            }
        );
    }
}

#[test]
fn non_allocator_error_and_boundary_configurations() {
    let _guard = TEST_LOCK.lock().unwrap();
    let pair = unsafe { ApiPair::load() };

    assert_eq!(unsafe { (pair.c.expand_array)(ptr::null_mut()) }, unsafe {
        (pair.rust.expand_array)(ptr::null_mut())
    });
    assert_eq!(
        unsafe { (pair.c.add_element)(ptr::null_mut(), 123) },
        unsafe { (pair.rust.add_element)(ptr::null_mut(), 123) }
    );
    unsafe {
        (pair.c.free_array)(ptr::null_mut());
        (pair.rust.free_array)(ptr::null_mut());
    }

    let c_zero = unsafe { (pair.c.init_array)(0) };
    let rust_zero = unsafe { (pair.rust.init_array)(0) };
    assert_eq!(c_zero.is_null(), rust_zero.is_null());
    assert!(!c_zero.is_null());
    if !c_zero.is_null() {
        unsafe { assert_arrays_equal(c_zero, rust_zero) };
        assert_eq!(unsafe { (*c_zero).capacity }, 0);
    }
    unsafe { free_pair(&pair, c_zero, rust_zero) };

    let c_oversized = unsafe { (pair.c.init_array)(usize::MAX) };
    let rust_oversized = unsafe { (pair.rust.init_array)(usize::MAX) };
    assert_eq!(c_oversized.is_null(), rust_oversized.is_null());
    assert!(c_oversized.is_null());
}

fn allocator_shim_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("ffi-test-support")
        .join("libfail_alloc.so")
}

fn build_allocator_shim() -> PathBuf {
    let output = allocator_shim_path();
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fail_alloc.c");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-o"])
        .arg(&output)
        .arg(&source)
        .status()
        .expect("failed to invoke cc for allocator shim");
    assert!(status.success(), "allocator shim compilation failed");
    output
}

#[test]
fn allocator_failure_paths() {
    if std::env::var_os("MATRIXSUM_ALLOC_FAILURE_CHILD").is_none() {
        let shim = build_allocator_shim();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "allocator_failure_paths", "--nocapture"])
            .env("MATRIXSUM_ALLOC_FAILURE_CHILD", "1")
            .env("LD_PRELOAD", shim)
            .status()
            .expect("failed to execute allocator-failure child test");
        assert!(status.success(), "allocator-failure child test failed");
        return;
    }

    let _guard = TEST_LOCK.lock().unwrap();
    let pair = unsafe { ApiPair::load() };
    type ArmFailure = unsafe extern "C" fn(c_long);
    type DisableFailures = unsafe extern "C" fn();
    let process = libloading::os::unix::Library::this();
    let fail_malloc_after = unsafe { *process.get::<ArmFailure>(b"fail_malloc_after\0").unwrap() };
    let fail_realloc_after =
        unsafe { *process.get::<ArmFailure>(b"fail_realloc_after\0").unwrap() };
    let disable_failures = unsafe {
        *process
            .get::<DisableFailures>(b"disable_alloc_failures\0")
            .unwrap()
    };

    for api in [&pair.c, &pair.rust] {
        unsafe { fail_malloc_after(0) };
        let object_failure = unsafe { (api.init_array)(2) };
        unsafe { disable_failures() };
        assert!(object_failure.is_null());

        unsafe { fail_malloc_after(1) };
        let data_failure = unsafe { (api.init_array)(2) };
        unsafe { disable_failures() };
        assert!(data_failure.is_null());

        for failed_malloc in [0, 1] {
            unsafe { fail_malloc_after(failed_malloc) };
            let matrixsum_failure = unsafe { (api.matrixsum)(1, 2, 3, 4) };
            unsafe { disable_failures() };
            assert_eq!(matrixsum_failure, -1);
        }

        let array = unsafe { (api.init_array)(1) };
        assert!(!array.is_null());
        assert_eq!(unsafe { (api.add_element)(array, 77) }, 1);
        let before = unsafe { snapshot(array) };

        unsafe { fail_realloc_after(0) };
        let expand_failure = unsafe { (api.expand_array)(array) };
        unsafe { disable_failures() };
        assert_eq!(expand_failure, 0);
        assert_eq!(unsafe { snapshot(array) }, before);

        unsafe { fail_realloc_after(0) };
        let add_failure = unsafe { (api.add_element)(array, 88) };
        unsafe { disable_failures() };
        assert_eq!(add_failure, 0);
        assert_eq!(unsafe { snapshot(array) }, before);

        unsafe { (api.free_array)(array) };
    }

    unsafe { fail_realloc_after(0) };
    let c_ignored_add_failure = unsafe { (pair.c.matrixsum)(10, 20, 30, 40) };
    unsafe { disable_failures() };
    unsafe { fail_realloc_after(0) };
    let rust_ignored_add_failure = unsafe { (pair.rust.matrixsum)(10, 20, 30, 40) };
    unsafe { disable_failures() };
    assert_eq!(c_ignored_add_failure, rust_ignored_add_failure);
}
