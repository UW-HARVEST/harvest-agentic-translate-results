#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

pub type ShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type ProcessString = unsafe extern "C" fn(*const c_char) -> c_int;
pub type ApplyBitmask = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type InitMatrix = unsafe extern "C" fn(*mut [c_int; 4]);
pub type CompareAllocations = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Arity4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type Arity2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Arity3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type Arity = unsafe extern "C" fn(c_int, *mut c_int) -> c_int;

pub struct Api {
    pub shift_array: ShiftArray,
    pub process_string: ProcessString,
    pub apply_bitmask: ApplyBitmask,
    pub init_matrix: InitMatrix,
    pub compare_allocations: CompareAllocations,
    pub arity4: Arity4,
    pub arity2: Arity2,
    pub arity3: Arity3,
    pub arity: Arity,
    _library: Library,
}

impl Api {
    pub unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        unsafe {
            Self {
                shift_array: *library.get(b"shift_array\0").unwrap(),
                process_string: *library.get(b"process_string\0").unwrap(),
                apply_bitmask: *library.get(b"apply_bitmask\0").unwrap(),
                init_matrix: *library.get(b"init_matrix\0").unwrap(),
                compare_allocations: *library.get(b"compare_allocations\0").unwrap(),
                arity4: *library.get(b"arity4\0").unwrap(),
                arity2: *library.get(b"arity2\0").unwrap(),
                arity3: *library.get(b"arity3\0").unwrap(),
                arity: *library.get(b"arity\0").unwrap(),
                _library: library,
            }
        }
    }
}

pub fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-nJQRmM.so")
}

pub fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_DYLIB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libarity_lib.so")
        })
}

pub unsafe fn load_both() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

pub fn assert_i32_bytes(row: usize, sample: usize, c_value: c_int, rust_value: c_int) {
    assert_eq!(
        c_value.to_ne_bytes(),
        rust_value.to_ne_bytes(),
        "row {row}, sample {sample}: C={c_value}, Rust={rust_value}"
    );
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    pub fn i32_between(&mut self, minimum: i32, maximum: i32) -> i32 {
        let width = (i64::from(maximum) - i64::from(minimum) + 1) as u64;
        (i64::from(minimum) + i64::try_from(u64::from(self.next_u32()) % width).unwrap()) as i32
    }

    pub fn nonzero_i32(&mut self, minimum: i32, maximum: i32) -> i32 {
        loop {
            let value = self.i32_between(minimum, maximum);
            if value != 0 {
                return value;
            }
        }
    }

    pub fn nonzero_u8(&mut self) -> u8 {
        (self.next_u32() % 255 + 1) as u8
    }
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
}

pub struct AllocationHarness {
    held: Vec<*mut c_void>,
}

impl AllocationHarness {
    pub fn new() -> Self {
        let mut harness = Self {
            held: Vec::with_capacity(16_384),
        };
        // Empty this thread's small-allocation freelists before the first oracle call.
        for _ in 0..256 {
            harness.hold_one();
        }
        harness
    }

    fn hold_one(&mut self) {
        let pointer = unsafe { malloc(size_of::<c_int>()) };
        assert!(
            !pointer.is_null(),
            "test allocator unexpectedly returned NULL"
        );
        self.held.push(pointer);
    }

    fn retain_target_allocations(&mut self) {
        self.hold_one();
        self.hold_one();
    }

    pub fn compare(
        &mut self,
        row: usize,
        sample: usize,
        call_c: impl FnOnce() -> c_int,
        call_rust: impl FnOnce() -> c_int,
    ) {
        let c_value = call_c();
        self.retain_target_allocations();
        let rust_value = call_rust();
        self.retain_target_allocations();
        assert_i32_bytes(row, sample, c_value, rust_value);
    }
}

impl Drop for AllocationHarness {
    fn drop(&mut self) {
        for pointer in self.held.drain(..) {
            unsafe { free(pointer) };
        }
    }
}

pub fn mark(covered: &mut [bool], row: usize) {
    assert!(row < covered.len());
    covered[row] = true;
}

pub fn assert_covered(covered: &[bool], first: usize, last: usize) {
    let missing: Vec<_> = (first..=last).filter(|row| !covered[*row]).collect();
    assert!(
        missing.is_empty(),
        "configuration rows not exercised: {missing:?}"
    );
}
