use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
struct ListNode {
    value: c_int,
    next: *mut ListNode,
}

type SmallestValueFn = unsafe extern "C" fn(*mut ListNode) -> c_int;

fn c_lib_path() -> &'static str {
    "c_src/build/libSimpleList.so"
}

fn rust_lib_path() -> &'static str {
    // The cdylib output for the Rust crate ends up here under cargo's target dir.
    // Tests are launched with the crate root as CWD.
    if std::path::Path::new("target/debug/libSimpleList.so").exists() {
        "target/debug/libSimpleList.so"
    } else {
        "target/release/libSimpleList.so"
    }
}

fn build_list(values: &[i32]) -> Vec<Box<ListNode>> {
    // Build list as a Vec of Boxed nodes; link them in reverse, then return
    // ownership keeper. Caller must keep the Vec alive while traversing.
    let mut nodes: Vec<Box<ListNode>> = values
        .iter()
        .map(|&v| {
            Box::new(ListNode {
                value: v,
                next: std::ptr::null_mut(),
            })
        })
        .collect();

    // Link nodes
    for i in 0..nodes.len() {
        if i + 1 < nodes.len() {
            let next_ptr: *mut ListNode = &mut *nodes[i + 1];
            nodes[i].next = next_ptr;
        }
    }
    nodes
}

fn head_ptr(nodes: &mut [Box<ListNode>]) -> *mut ListNode {
    if nodes.is_empty() {
        std::ptr::null_mut()
    } else {
        &mut *nodes[0]
    }
}

fn run_both(values: Option<&[i32]>) -> (c_int, c_int) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let c_fn: Symbol<SmallestValueFn> = c_lib.get(b"smallestValue").expect("c sym");
        let r_fn: Symbol<SmallestValueFn> = rust_lib.get(b"smallestValue").expect("r sym");

        match values {
            None => (c_fn(std::ptr::null_mut()), r_fn(std::ptr::null_mut())),
            Some(vs) => {
                let mut nodes = build_list(vs);
                let head = head_ptr(&mut nodes);
                let cv = c_fn(head);
                // re-init in case anything was modified (it shouldn't be)
                // C function doesn't mutate, but Rust shouldn't either; reuse head
                let rv = r_fn(head);
                (cv, rv)
            }
        }
    }
}

#[test]
fn test_null_head() {
    let (c, r) = run_both(None);
    assert_eq!(c, r, "null head: c={} r={}", c, r);
    assert_eq!(c, -1);
}

#[test]
fn test_single_node_positive() {
    let (c, r) = run_both(Some(&[42]));
    assert_eq!(c, r);
    assert_eq!(c, 42);
}

#[test]
fn test_single_node_zero() {
    let (c, r) = run_both(Some(&[0]));
    assert_eq!(c, r);
    assert_eq!(c, 0);
}

#[test]
fn test_single_node_negative() {
    let (c, r) = run_both(Some(&[-7]));
    assert_eq!(c, r);
    assert_eq!(c, -7);
}

#[test]
fn test_increasing_list() {
    let (c, r) = run_both(Some(&[1, 2, 3, 4, 5]));
    assert_eq!(c, r);
    assert_eq!(c, 1);
}

#[test]
fn test_decreasing_list() {
    let (c, r) = run_both(Some(&[5, 4, 3, 2, 1]));
    assert_eq!(c, r);
    assert_eq!(c, 1);
}

#[test]
fn test_mixed_with_negatives() {
    let (c, r) = run_both(Some(&[3, -10, 5, -2, 7, 0]));
    assert_eq!(c, r);
    assert_eq!(c, -10);
}

#[test]
fn test_all_equal() {
    let (c, r) = run_both(Some(&[8, 8, 8, 8]));
    assert_eq!(c, r);
    assert_eq!(c, 8);
}

#[test]
fn test_min_at_head() {
    let (c, r) = run_both(Some(&[-100, 5, 8, 3]));
    assert_eq!(c, r);
    assert_eq!(c, -100);
}

#[test]
fn test_min_at_tail() {
    let (c, r) = run_both(Some(&[5, 8, 3, -42]));
    assert_eq!(c, r);
    assert_eq!(c, -42);
}

#[test]
fn test_int_min_max() {
    let (c, r) = run_both(Some(&[i32::MAX, i32::MIN, 0]));
    assert_eq!(c, r);
    assert_eq!(c, i32::MIN);
}

#[test]
fn test_long_list() {
    let values: Vec<i32> = (0..1000).rev().collect();
    let (c, r) = run_both(Some(&values));
    assert_eq!(c, r);
    assert_eq!(c, 0);
}
