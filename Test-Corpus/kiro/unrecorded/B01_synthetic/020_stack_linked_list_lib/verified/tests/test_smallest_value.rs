use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::ptr;

#[repr(C)]
struct ListNode {
    value: c_int,
    next: *mut ListNode,
}

fn make_list(values: &[c_int]) -> Vec<ListNode> {
    let mut nodes: Vec<ListNode> = values
        .iter()
        .map(|&v| ListNode { value: v, next: ptr::null_mut() })
        .collect();
    for i in 0..nodes.len().saturating_sub(1) {
        let next_ptr = &mut nodes[i + 1] as *mut ListNode;
        nodes[i].next = next_ptr;
    }
    nodes
}

type SmallestValueFn = unsafe extern "C" fn(*mut ListNode) -> c_int;

fn load_fn(lib: &Library) -> Symbol<SmallestValueFn> {
    unsafe { lib.get(b"smallestValue").expect("symbol not found") }
}

fn rust_lib_path() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target/debug/libSimpleList.so");
    path
}

fn c_lib_path() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("c_src/build/libSimpleList.so");
    path
}

#[test]
fn test_smallest_value() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };
    let c_fn = load_fn(&c_lib);
    let r_fn = load_fn(&r_lib);

    // null pointer
    let c_res = unsafe { c_fn(ptr::null_mut()) };
    let r_res = unsafe { r_fn(ptr::null_mut()) };
    assert_eq!(c_res, r_res, "null case: C={c_res} Rust={r_res}");

    // test cases: various linked lists
    let cases: &[&[c_int]] = &[
        &[42],
        &[1, 2, 3],
        &[3, 2, 1],
        &[5, 1, 5],
        &[-1, -5, -3],
        &[0, 0, 0],
        &[i32::MAX, i32::MIN, 0],
        &[100, 200, 50, 300, 10],
    ];

    for vals in cases {
        let mut c_nodes = make_list(vals);
        let mut r_nodes = make_list(vals);
        let c_res = unsafe { c_fn(&mut c_nodes[0] as *mut ListNode) };
        let r_res = unsafe { r_fn(&mut r_nodes[0] as *mut ListNode) };
        assert_eq!(c_res, r_res, "vals={vals:?}: C={c_res} Rust={r_res}");
    }
}
