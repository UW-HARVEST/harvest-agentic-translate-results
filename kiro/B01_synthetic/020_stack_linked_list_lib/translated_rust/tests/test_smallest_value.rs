use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
struct ListNode {
    value: c_int,
    next: *mut ListNode,
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libSimpleList.so")
}

fn make_list(values: &[c_int]) -> Vec<ListNode> {
    let mut nodes: Vec<ListNode> = values
        .iter()
        .map(|&v| ListNode { value: v, next: std::ptr::null_mut() })
        .collect();
    for i in 0..nodes.len().saturating_sub(1) {
        let next_ptr = &mut nodes[i + 1] as *mut ListNode;
        nodes[i].next = next_ptr;
    }
    nodes
}

unsafe fn call_c_smallest(lib: &Library, head: *mut ListNode) -> c_int {
    let func: Symbol<unsafe extern "C" fn(*mut ListNode) -> c_int> =
        unsafe { lib.get(b"smallestValue") }.unwrap();
    unsafe { func(head) }
}

fn call_rust_smallest(head: *mut ListNode) -> c_int {
    unsafe { SimpleList::smallestValue(head as *mut SimpleList::ListNode) }
}

#[test]
fn test_null_input() {
    let lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let c_result = unsafe { call_c_smallest(&lib, std::ptr::null_mut()) };
    let rust_result = call_rust_smallest(std::ptr::null_mut());
    assert_eq!(c_result, rust_result, "null input: C={c_result}, Rust={rust_result}");
}

#[test]
fn test_single_element() {
    let lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let mut nodes = make_list(&[42]);
    let head = &mut nodes[0] as *mut ListNode;
    let c_result = unsafe { call_c_smallest(&lib, head) };
    let rust_result = call_rust_smallest(head);
    assert_eq!(c_result, rust_result, "single element: C={c_result}, Rust={rust_result}");
}

#[test]
fn test_ascending() {
    let lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let mut nodes = make_list(&[1, 2, 3, 4, 5]);
    let head = &mut nodes[0] as *mut ListNode;
    let c_result = unsafe { call_c_smallest(&lib, head) };
    let rust_result = call_rust_smallest(head);
    assert_eq!(c_result, rust_result, "ascending: C={c_result}, Rust={rust_result}");
}

#[test]
fn test_descending() {
    let lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let mut nodes = make_list(&[5, 4, 3, 2, 1]);
    let head = &mut nodes[0] as *mut ListNode;
    let c_result = unsafe { call_c_smallest(&lib, head) };
    let rust_result = call_rust_smallest(head);
    assert_eq!(c_result, rust_result, "descending: C={c_result}, Rust={rust_result}");
}

#[test]
fn test_negative_values() {
    let lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let mut nodes = make_list(&[3, -1, 7, -5, 2]);
    let head = &mut nodes[0] as *mut ListNode;
    let c_result = unsafe { call_c_smallest(&lib, head) };
    let rust_result = call_rust_smallest(head);
    assert_eq!(c_result, rust_result, "negatives: C={c_result}, Rust={rust_result}");
}

#[test]
fn test_all_same() {
    let lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let mut nodes = make_list(&[7, 7, 7]);
    let head = &mut nodes[0] as *mut ListNode;
    let c_result = unsafe { call_c_smallest(&lib, head) };
    let rust_result = call_rust_smallest(head);
    assert_eq!(c_result, rust_result, "all same: C={c_result}, Rust={rust_result}");
}

#[test]
fn test_min_at_end() {
    let lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let mut nodes = make_list(&[10, 20, 30, 0]);
    let head = &mut nodes[0] as *mut ListNode;
    let c_result = unsafe { call_c_smallest(&lib, head) };
    let rust_result = call_rust_smallest(head);
    assert_eq!(c_result, rust_result, "min at end: C={c_result}, Rust={rust_result}");
}
