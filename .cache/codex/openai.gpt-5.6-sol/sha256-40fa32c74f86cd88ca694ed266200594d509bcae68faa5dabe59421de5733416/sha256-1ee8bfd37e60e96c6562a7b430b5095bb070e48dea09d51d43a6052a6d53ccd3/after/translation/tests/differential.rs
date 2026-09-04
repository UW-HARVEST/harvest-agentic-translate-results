use libloading::Library;
use std::ffi::c_int;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;

type Matrix = [[c_int; 4]; 3];

#[repr(C)]
#[derive(Debug)]
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
        let init_array = unsafe { *library.get(b"init_array\0").unwrap() };
        let expand_array = unsafe { *library.get(b"expand_array\0").unwrap() };
        let add_element = unsafe { *library.get(b"add_element\0").unwrap() };
        let free_array = unsafe { *library.get(b"free_array\0").unwrap() };
        let process_flags = unsafe { *library.get(b"process_flags\0").unwrap() };
        let calculate_matrix_checksum =
            unsafe { *library.get(b"calculate_matrix_checksum\0").unwrap() };
        let matrixsum = unsafe { *library.get(b"matrixsum\0").unwrap() };
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

    fn usize_in(&mut self, start: usize, end_inclusive: usize) -> usize {
        start + self.next_u32() as usize % (end_inclusive - start + 1)
    }

    fn i32_in(&mut self, start: i32, end_inclusive: i32) -> i32 {
        start + (self.next_u32() % (end_inclusive - start + 1) as u32) as i32
    }

    fn nonzero_i32(&mut self, magnitude: i32) -> i32 {
        loop {
            let value = self.i32_in(-magnitude, magnitude);
            if value != 0 {
                return value;
            }
        }
    }
}

fn c_library_path() -> PathBuf {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<_> = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();
    assert_eq!(candidates.len(), 1, "expected one C shared library");
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libmatrixsum_lib.so")
}

unsafe fn load_apis() -> (Api, Api) {
    (unsafe { Api::load(&c_library_path()) }, unsafe {
        Api::load(&rust_library_path())
    })
}

unsafe fn assert_array_equal(c_array: *mut DynamicArray, rust_array: *mut DynamicArray) {
    assert_eq!(c_array.is_null(), rust_array.is_null());
    if c_array.is_null() {
        return;
    }
    let c_array = unsafe { &*c_array };
    let rust_array = unsafe { &*rust_array };
    assert_eq!(c_array.size, rust_array.size);
    assert_eq!(c_array.capacity, rust_array.capacity);
    assert_eq!(c_array.data.is_null(), rust_array.data.is_null());
    if !c_array.data.is_null() {
        for index in 0..c_array.size {
            assert_eq!(
                unsafe { *c_array.data.add(index) },
                unsafe { *rust_array.data.add(index) },
                "array data differs at index {index}"
            );
        }
    }
}

unsafe fn allocate_pair(
    c: &Api,
    rust: &Api,
    capacity: usize,
) -> (*mut DynamicArray, *mut DynamicArray) {
    let c_array = unsafe { (c.init_array)(capacity) };
    let rust_array = unsafe { (rust.init_array)(capacity) };
    assert!(!c_array.is_null());
    assert!(!rust_array.is_null());
    unsafe { assert_array_equal(c_array, rust_array) };
    (c_array, rust_array)
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

unsafe fn set_array_contents(
    c_array: *mut DynamicArray,
    rust_array: *mut DynamicArray,
    values: &[c_int],
) {
    assert!(values.len() <= unsafe { (*c_array).capacity });
    for (index, value) in values.iter().copied().enumerate() {
        unsafe {
            *(*c_array).data.add(index) = value;
            *(*rust_array).data.add(index) = value;
        }
    }
    unsafe {
        (*c_array).size = values.len();
        (*rust_array).size = values.len();
    }
}

unsafe fn set_matrices(c: &Api, rust: &Api, matrix: Matrix) {
    unsafe {
        ptr::write(c.matrix, matrix);
        ptr::write(rust.matrix, matrix);
    }
}

#[test]
fn all_valid_configuration_rows_match() {
    unsafe {
        let (c, rust) = load_apis();
        let mut rng = Rng::new(0x4D41_5452_4958_5355);
        let original = [
            [0x01, 0x02, 0x03, 0x04],
            [0x10, 0x20, 0x30, 0x40],
            [0xA1, 0xB2, 0xC3, 0xD4],
        ];

        // C1: exported data object.
        assert_eq!(*c.matrix, original);
        assert_eq!(*rust.matrix, original);
        assert_eq!(
            std::slice::from_raw_parts(c.matrix.cast::<u8>(), size_of::<Matrix>()),
            std::slice::from_raw_parts(rust.matrix.cast::<u8>(), size_of::<Matrix>())
        );

        // C2-C4: zero, one, and many capacities.
        for fixed_capacity in [0, 1] {
            for _ in 0..128 {
                let c_array = (c.init_array)(fixed_capacity);
                let rust_array = (rust.init_array)(fixed_capacity);
                assert_array_equal(c_array, rust_array);
                if !c_array.is_null() {
                    assert_eq!((*c_array).size, 0);
                    assert_eq!((*c_array).capacity, fixed_capacity);
                }
                free_pair(&c, &rust, c_array, rust_array);
            }
        }
        for _ in 0..128 {
            let capacity = rng.usize_in(2, 256);
            let c_array = (c.init_array)(capacity);
            let rust_array = (rust.init_array)(capacity);
            assert_array_equal(c_array, rust_array);
            assert_eq!((*c_array).size, 0);
            assert_eq!((*c_array).capacity, capacity);
            free_pair(&c, &rust, c_array, rust_array);
        }

        // C5-C6: explicit expansion preserves one/many existing elements.
        for _ in 0..128 {
            let (c_array, rust_array) = allocate_pair(&c, &rust, 1);
            let values = [rng.next_u32() as i32];
            set_array_contents(c_array, rust_array, &values);
            assert_eq!((c.expand_array)(c_array), (rust.expand_array)(rust_array));
            assert_array_equal(c_array, rust_array);
            assert_eq!((*c_array).capacity, 2);
            free_pair(&c, &rust, c_array, rust_array);
        }
        for _ in 0..128 {
            let capacity = rng.usize_in(2, 128);
            let (c_array, rust_array) = allocate_pair(&c, &rust, capacity);
            let values: Vec<_> = (0..capacity).map(|_| rng.next_u32() as i32).collect();
            set_array_contents(c_array, rust_array, &values);
            assert_eq!((c.expand_array)(c_array), (rust.expand_array)(rust_array));
            assert_array_equal(c_array, rust_array);
            assert_eq!((*c_array).capacity, capacity * 2);
            free_pair(&c, &rust, c_array, rust_array);
        }

        // C7: direct append to an empty array with spare capacity.
        for _ in 0..128 {
            let (c_array, rust_array) = allocate_pair(&c, &rust, 8);
            let value = rng.next_u32() as i32;
            assert_eq!(
                (c.add_element)(c_array, value),
                (rust.add_element)(rust_array, value)
            );
            assert_array_equal(c_array, rust_array);
            free_pair(&c, &rust, c_array, rust_array);
        }

        // C8: direct append to a nonempty array with spare capacity.
        for _ in 0..128 {
            let capacity = rng.usize_in(3, 32);
            let size = rng.usize_in(1, capacity - 1);
            let (c_array, rust_array) = allocate_pair(&c, &rust, capacity);
            let values: Vec<_> = (0..size).map(|_| rng.next_u32() as i32).collect();
            set_array_contents(c_array, rust_array, &values);
            let value = rng.next_u32() as i32;
            assert_eq!(
                (c.add_element)(c_array, value),
                (rust.add_element)(rust_array, value)
            );
            assert_array_equal(c_array, rust_array);
            free_pair(&c, &rust, c_array, rust_array);
        }

        // C9-C10: append at exact capacity, selecting nested expansion.
        for _ in 0..128 {
            let (c_array, rust_array) = allocate_pair(&c, &rust, 1);
            let values = [rng.next_u32() as i32];
            set_array_contents(c_array, rust_array, &values);
            let value = rng.next_u32() as i32;
            assert_eq!(
                (c.add_element)(c_array, value),
                (rust.add_element)(rust_array, value)
            );
            assert_array_equal(c_array, rust_array);
            assert_eq!((*c_array).capacity, 2);
            free_pair(&c, &rust, c_array, rust_array);
        }
        for _ in 0..128 {
            let capacity = rng.usize_in(2, 64);
            let (c_array, rust_array) = allocate_pair(&c, &rust, capacity);
            let values: Vec<_> = (0..capacity).map(|_| rng.next_u32() as i32).collect();
            set_array_contents(c_array, rust_array, &values);
            let value = rng.next_u32() as i32;
            assert_eq!(
                (c.add_element)(c_array, value),
                (rust.add_element)(rust_array, value)
            );
            assert_array_equal(c_array, rust_array);
            assert_eq!((*c_array).capacity, capacity * 2);
            free_pair(&c, &rust, c_array, rust_array);
        }

        // C11-C12: freeing empty and populated arrays.
        for populated in [false, true] {
            for _ in 0..128 {
                let (c_array, rust_array) = allocate_pair(&c, &rust, 4);
                if populated {
                    set_array_contents(c_array, rust_array, &[1, -2, 3, -4]);
                }
                free_pair(&c, &rust, c_array, rust_array);
            }
        }

        // C13-C28: all 16 recognized flag combinations plus arbitrary high bits.
        for recognized_mask in 0..=0xF {
            for _ in 0..256 {
                let unrelated = (rng.next_u32() as i32) & !0xF;
                let flags = unrelated | recognized_mask;
                assert_eq!(
                    (c.process_flags)(flags),
                    (rust.process_flags)(flags),
                    "flags {flags:#x}"
                );
                assert_eq!(
                    (c.process_flags)(flags),
                    recognized_mask.count_ones() as i32
                );
            }
        }

        // C29-C31: original, zero, and randomized mutable matrices.
        set_matrices(&c, &rust, original);
        assert_eq!(
            (c.calculate_matrix_checksum)(),
            (rust.calculate_matrix_checksum)()
        );
        set_matrices(&c, &rust, [[0; 4]; 3]);
        assert_eq!(
            (c.calculate_matrix_checksum)(),
            (rust.calculate_matrix_checksum)()
        );
        for _ in 0..512 {
            let mut matrix = [[0; 4]; 3];
            for row in &mut matrix {
                for value in row {
                    *value = rng.i32_in(-100_000, 100_000);
                }
            }
            set_matrices(&c, &rust, matrix);
            assert_eq!(
                (c.calculate_matrix_checksum)(),
                (rust.calculate_matrix_checksum)()
            );
        }

        // C32-C47: every zero/nonzero argument mask with randomized values/matrix.
        for nonzero_mask in 0..=0xF {
            for _ in 0..256 {
                let mut parameters = [0; 4];
                for (index, parameter) in parameters.iter_mut().enumerate() {
                    if nonzero_mask & (1 << index) != 0 {
                        *parameter = rng.nonzero_i32(100_000);
                    }
                }
                let mut matrix = [[0; 4]; 3];
                for row in &mut matrix {
                    for value in row {
                        *value = rng.i32_in(-10_000, 10_000);
                    }
                }
                set_matrices(&c, &rust, matrix);
                let c_result =
                    (c.matrixsum)(parameters[0], parameters[1], parameters[2], parameters[3]);
                let rust_result =
                    (rust.matrixsum)(parameters[0], parameters[1], parameters[2], parameters[3]);
                assert_eq!(c_result, rust_result, "mask {nonzero_mask:#x}");
            }
        }

        // C48-C50: complete low-level consumer sequences with and without growth.
        for initial_capacity in [1, 2, 32] {
            for _ in 0..128 {
                let count = if initial_capacity == 32 {
                    32
                } else {
                    rng.usize_in(5, 40)
                };
                let (c_array, rust_array) = allocate_pair(&c, &rust, initial_capacity);
                for _ in 0..count {
                    let value = rng.next_u32() as i32;
                    assert_eq!(
                        (c.add_element)(c_array, value),
                        (rust.add_element)(rust_array, value)
                    );
                    assert_array_equal(c_array, rust_array);
                }
                free_pair(&c, &rust, c_array, rust_array);
            }
        }

        // C51: repeated mixed operations with synchronized global and heap state.
        for _ in 0..256 {
            let capacity = rng.usize_in(1, 16);
            let (c_array, rust_array) = allocate_pair(&c, &rust, capacity);
            for _ in 0..rng.usize_in(1, 48) {
                let value = rng.next_u32() as i32;
                assert_eq!(
                    (c.add_element)(c_array, value),
                    (rust.add_element)(rust_array, value)
                );
            }
            assert_array_equal(c_array, rust_array);
            let flags = rng.next_u32() as i32;
            assert_eq!((c.process_flags)(flags), (rust.process_flags)(flags));
            let mut matrix = [[0; 4]; 3];
            for row in &mut matrix {
                for value in row {
                    *value = rng.i32_in(-1_000, 1_000);
                }
            }
            set_matrices(&c, &rust, matrix);
            assert_eq!(
                (c.calculate_matrix_checksum)(),
                (rust.calculate_matrix_checksum)()
            );
            let parameters = [
                rng.nonzero_i32(1_000),
                rng.i32_in(-1_000, 1_000),
                rng.i32_in(-1_000, 1_000),
                rng.i32_in(-1_000, 1_000),
            ];
            assert_eq!(
                (c.matrixsum)(parameters[0], parameters[1], parameters[2], parameters[3]),
                (rust.matrixsum)(parameters[0], parameters[1], parameters[2], parameters[3])
            );
            free_pair(&c, &rust, c_array, rust_array);
        }

        set_matrices(&c, &rust, original);
    }
}

#[test]
fn non_allocator_error_rows_match() {
    unsafe {
        let (c, rust) = load_apis();

        // E2: oversized length makes the data allocation fail.
        assert_eq!(
            (c.init_array)(usize::MAX).is_null(),
            (rust.init_array)(usize::MAX).is_null()
        );
        assert!((c.init_array)(usize::MAX).is_null());
        assert!((rust.init_array)(usize::MAX).is_null());

        // E3 and E5: null array arguments.
        assert_eq!((c.expand_array)(ptr::null_mut()), 0);
        assert_eq!((rust.expand_array)(ptr::null_mut()), 0);
        assert_eq!((c.add_element)(ptr::null_mut(), i32::MIN), 0);
        assert_eq!((rust.add_element)(ptr::null_mut(), i32::MIN), 0);

        // E4: oversized reallocation fails and leaves the object unchanged.
        let capacity = usize::MAX / 2;
        let mut c_array = DynamicArray {
            data: ptr::null_mut(),
            size: 0,
            capacity,
        };
        let mut rust_array = DynamicArray {
            data: ptr::null_mut(),
            size: 0,
            capacity,
        };
        assert_eq!((c.expand_array)(&mut c_array), 0);
        assert_eq!((rust.expand_array)(&mut rust_array), 0);
        assert_eq!(c_array.size, rust_array.size);
        assert_eq!(c_array.capacity, rust_array.capacity);
        assert_eq!(c_array.data, rust_array.data);

        // E6: add at capacity propagates the same expansion failure.
        let mut c_array = DynamicArray {
            data: ptr::null_mut(),
            size: capacity,
            capacity,
        };
        let mut rust_array = DynamicArray {
            data: ptr::null_mut(),
            size: capacity,
            capacity,
        };
        assert_eq!((c.add_element)(&mut c_array, i32::MAX), 0);
        assert_eq!((rust.add_element)(&mut rust_array, i32::MAX), 0);
        assert_eq!(c_array.size, rust_array.size);
        assert_eq!(c_array.capacity, rust_array.capacity);
        assert_eq!(c_array.data, rust_array.data);

        // E7: null is an explicit safe no-op.
        (c.free_array)(ptr::null_mut());
        (rust.free_array)(ptr::null_mut());
    }
}
