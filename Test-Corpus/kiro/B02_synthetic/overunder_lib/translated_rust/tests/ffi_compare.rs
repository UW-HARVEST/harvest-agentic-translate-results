use libloading::{Library, Symbol};
use std::ffi::c_int;

fn c_lib() -> Library {
    unsafe { Library::new("/tmp/harvest-work-AkLIsF/translated_rust/c_src/build/libtranslated_rust.so").unwrap() }
}

fn rust_lib() -> Library {
    unsafe { Library::new("/tmp/harvest-work-AkLIsF/translated_rust/target/debug/liboverunder_lib.so").unwrap() }
}

#[repr(C)]
#[derive(Clone)]
struct DataBlock {
    id: c_int,
    value: f64,
    label: [u8; 20],
}

#[test]
fn test_safe_double_to_int() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> = unsafe { c.get(b"safe_double_to_int").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> = unsafe { r.get(b"safe_double_to_int").unwrap() };

    let cases: &[f64] = &[
        0.0, 1.0, -1.0, 1.5, -1.5, 100.7, -100.7,
        i32::MAX as f64, i32::MIN as f64,
        i32::MAX as f64 + 1.0, i32::MIN as f64 - 1.0,
        1e15, -1e15, f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
        0.49, 0.51, -0.49, -0.51,
    ];
    for &d in cases {
        let c_res = unsafe { c_fn(d) };
        let r_res = unsafe { r_fn(d) };
        assert_eq!(c_res, r_res, "safe_double_to_int({d}) mismatch: C={c_res}, Rust={r_res}");
    }
}

#[test]
fn test_process_with_fallthrough() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> = unsafe { c.get(b"process_with_fallthrough").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> = unsafe { r.get(b"process_with_fallthrough").unwrap() };

    for code in -1..=7 {
        for base in &[0, 10, -5, 100] {
            let c_res = unsafe { c_fn(code, *base) };
            let r_res = unsafe { r_fn(code, *base) };
            assert_eq!(c_res, r_res, "process_with_fallthrough({code}, {base}) mismatch: C={c_res}, Rust={r_res}");
        }
    }
}

#[test]
fn test_handle_pointer_operations() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> = unsafe { c.get(b"handle_pointer_operations").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> = unsafe { r.get(b"handle_pointer_operations").unwrap() };

    for v in &[0, 1, -1, 50, -50, 1000, i32::MAX / 2, i32::MIN / 2] {
        let c_res = unsafe { c_fn(*v) };
        let r_res = unsafe { r_fn(*v) };
        assert_eq!(c_res, r_res, "handle_pointer_operations({v}) mismatch: C={c_res}, Rust={r_res}");
    }
}

#[test]
fn test_copy_data_block() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*mut DataBlock, *const DataBlock)> = unsafe { c.get(b"copy_data_block").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*mut DataBlock, *const DataBlock)> = unsafe { r.get(b"copy_data_block").unwrap() };

    let mut label = [0u8; 20];
    label[..5].copy_from_slice(b"Hello");
    let src = DataBlock { id: 42, value: 3.14, label };

    let mut c_dest = DataBlock { id: 0, value: 0.0, label: [0u8; 20] };
    let mut r_dest = DataBlock { id: 0, value: 0.0, label: [0u8; 20] };
    unsafe {
        c_fn(&mut c_dest, &src);
        r_fn(&mut r_dest, &src);
    }
    let c_bytes: &[u8] = unsafe { std::slice::from_raw_parts(&c_dest as *const _ as *const u8, std::mem::size_of::<DataBlock>()) };
    let r_bytes: &[u8] = unsafe { std::slice::from_raw_parts(&r_dest as *const _ as *const u8, std::mem::size_of::<DataBlock>()) };
    assert_eq!(c_bytes, r_bytes, "copy_data_block byte mismatch");
}

#[test]
fn test_overunder() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = unsafe { c.get(b"overunder").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = unsafe { r.get(b"overunder").unwrap() };

    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (5, 10, 15, 20),
        (-1, -2, -3, -4),
        (6, 7, 8, 9),
        (100, 200, 300, 400),
        (1, 0, 0, 0),
        (0, 1, 0, 0),
        (11, 12, 0, 1),
    ];
    for &(a, b, c_val, d) in cases {
        let c_res = unsafe { c_fn(a, b, c_val, d) };
        let r_res = unsafe { r_fn(a, b, c_val, d) };
        assert_eq!(c_res, r_res, "overunder({a}, {b}, {c_val}, {d}) mismatch: C={c_res}, Rust={r_res}");
    }
}
