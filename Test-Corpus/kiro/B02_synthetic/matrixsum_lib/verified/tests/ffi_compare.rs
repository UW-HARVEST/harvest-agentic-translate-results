use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

#[repr(C)]
struct DynamicArray {
    data: *mut c_int,
    size: u64,
    capacity: u64,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.join("target/debug/libmatrixsum_lib.so")
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C .so");
        let r = Library::new(rust_lib_path()).expect("load Rust .so");
        (c, r)
    }
}

#[test]
fn test_matrix_data() {
    let (c, r) = load_libs();
    unsafe {
        let c_matrix: Symbol<*const [[c_int; 4]; 3]> = c.get(b"matrix").unwrap();
        let r_matrix: Symbol<*const [[c_int; 4]; 3]> = r.get(b"matrix").unwrap();
        let c_bytes = std::slice::from_raw_parts(*c_matrix as *const u8, std::mem::size_of::<[[c_int; 4]; 3]>());
        let r_bytes = std::slice::from_raw_parts(*r_matrix as *const u8, std::mem::size_of::<[[c_int; 4]; 3]>());
        assert_eq!(c_bytes, r_bytes, "matrix data mismatch");
    }
}

#[test]
fn test_process_flags() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> = c.get(b"process_flags").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> = r.get(b"process_flags").unwrap();
        for flags in 0..=0xFF {
            let c_val = c_fn(flags);
            let r_val = r_fn(flags);
            assert_eq!(c_val, r_val, "process_flags({flags}) mismatch: C={c_val} Rust={r_val}");
        }
    }
}

#[test]
fn test_calculate_matrix_checksum() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn() -> c_int> = c.get(b"calculate_matrix_checksum").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn() -> c_int> = r.get(b"calculate_matrix_checksum").unwrap();
        assert_eq!(c_fn(), r_fn(), "calculate_matrix_checksum mismatch");
    }
}

#[test]
fn test_init_add_free_array() {
    let (c, r) = load_libs();
    unsafe {
        let c_init: Symbol<unsafe extern "C" fn(u64) -> *mut DynamicArray> = c.get(b"init_array").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(u64) -> *mut DynamicArray> = r.get(b"init_array").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int> = c.get(b"add_element").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int> = r.get(b"add_element").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> = c.get(b"free_array").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> = r.get(b"free_array").unwrap();

        // Test init
        let ca = c_init(4);
        let ra = r_init(4);
        assert!(!ca.is_null());
        assert!(!ra.is_null());
        assert_eq!((*ca).size, (*ra).size, "initial size mismatch");
        assert_eq!((*ca).capacity, (*ra).capacity, "initial capacity mismatch");

        // Add elements (triggers expand at capacity boundary)
        let vals = [10, 20, 30, 40, 50];
        for &v in &vals {
            let cr = c_add(ca, v);
            let rr = r_add(ra, v);
            assert_eq!(cr, rr, "add_element return mismatch for {v}");
        }
        assert_eq!((*ca).size, (*ra).size, "size after adds mismatch");

        // Compare stored data
        for i in 0..(*ca).size as usize {
            assert_eq!(*(*ca).data.add(i), *(*ra).data.add(i), "data[{i}] mismatch");
        }

        c_free(ca);
        r_free(ra);
    }
}

#[test]
fn test_expand_array() {
    let (c, r) = load_libs();
    unsafe {
        let c_init: Symbol<unsafe extern "C" fn(u64) -> *mut DynamicArray> = c.get(b"init_array").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(u64) -> *mut DynamicArray> = r.get(b"init_array").unwrap();
        let c_expand: Symbol<unsafe extern "C" fn(*mut DynamicArray) -> c_int> = c.get(b"expand_array").unwrap();
        let r_expand: Symbol<unsafe extern "C" fn(*mut DynamicArray) -> c_int> = r.get(b"expand_array").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> = c.get(b"free_array").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut DynamicArray)> = r.get(b"free_array").unwrap();

        let ca = c_init(2);
        let ra = r_init(2);
        assert_eq!(c_expand(ca), r_expand(ra), "expand return mismatch");
        assert_eq!((*ca).capacity, (*ra).capacity, "capacity after expand mismatch");

        // Null test
        assert_eq!(c_expand(std::ptr::null_mut()), r_expand(std::ptr::null_mut()), "expand(null) mismatch");

        c_free(ca);
        r_free(ra);
    }
}

#[test]
fn test_matrixsum() {
    let (c, r) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c.get(b"matrixsum").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r.get(b"matrixsum").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 0, 0, 0),
            (0, 1, 0, 0),
            (0, 0, 1, 0),
            (0, 0, 0, 1),
            (1, 1, 1, 1),
            (1, 2, 3, 4),
            (100, 200, 300, 400),
            (-1, -2, -3, -4),
            (0x7FFFFFFF, 0, 0, 0),
            (0, 0, 0, -1),
            (10, 20, 30, 40),
            (255, 255, 255, 255),
        ];

        for &(a, b, c_val, d) in cases {
            let cv = c_fn(a, b, c_val, d);
            let rv = r_fn(a, b, c_val, d);
            assert_eq!(cv, rv, "matrixsum({a},{b},{c_val},{d}) mismatch: C={cv} Rust={rv}");
        }
    }
}
