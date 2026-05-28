// Compare C and Rust libraries through FFI boundary.
// Both .so files are loaded with libloading and the exported symbols are
// invoked from Rust. The Rust .so is treated as a black box exactly like
// the C .so.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::sync::{Mutex, MutexGuard, OnceLock};

const C_SO: &str = "c_src/build/libtranslated_rust.so";
const RUST_SO: &str = "target/debug/libinreftree_lib.so";

// Tests that mutate the global state in either library MUST serialize on this
// mutex. Both libraries are dlopen'd as a single instance per process, so any
// test that reads/writes node_table or node_count would race with others.
fn state_lock() -> MutexGuard<'static, ()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|p| p.into_inner())
}

#[repr(C)]
#[derive(Copy, Clone)]
struct TreeNode {
    id: c_int,
    value: c_int,
    parent_id: c_int,
    left_child_id: c_int,
    right_child_id: c_int,
    label: [c_char; 32],
}

const MAX_NODES: usize = 50;

unsafe fn load(path: &str) -> Library {
    Library::new(path).unwrap_or_else(|e| panic!("Failed to load {}: {}", path, e))
}

fn open_pair() -> (Library, Library) {
    unsafe { (load(C_SO), load(RUST_SO)) }
}

unsafe fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    lib.get(name).unwrap_or_else(|e| {
        panic!(
            "Failed to find symbol {} in lib: {}",
            String::from_utf8_lossy(name),
            e
        )
    })
}

fn cstr(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

unsafe fn reset_state(lib: &Library) {
    let table: Symbol<*mut TreeNode> = sym(lib, b"node_table");
    let count: Symbol<*mut c_int> = sym(lib, b"node_count");
    // table and count are *Symbol* wrappers; we need raw pointers.
    let table_ptr = *table;
    let count_ptr = *count;
    *count_ptr = 0;
    for i in 0..MAX_NODES {
        let n = table_ptr.add(i);
        (*n).id = 0;
        (*n).value = 0;
        (*n).parent_id = 0;
        (*n).left_child_id = 0;
        (*n).right_child_id = 0;
        for j in 0..32 {
            (*n).label[j] = 0;
        }
    }
}

unsafe fn get_count(lib: &Library) -> c_int {
    let count: Symbol<*mut c_int> = sym(lib, b"node_count");
    *(*count)
}

unsafe fn get_node(lib: &Library, idx: usize) -> TreeNode {
    let table: Symbol<*mut TreeNode> = sym(lib, b"node_table");
    *(*table).add(idx)
}

fn nodes_eq(a: &TreeNode, b: &TreeNode) -> bool {
    a.id == b.id
        && a.value == b.value
        && a.parent_id == b.parent_id
        && a.left_child_id == b.left_child_id
        && a.right_child_id == b.right_child_id
        && a.label.iter().zip(b.label.iter()).all(|(x, y)| x == y)
}

#[test]
fn test_add_op() {
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let c_fn: Symbol<Fn> = sym(&c_lib, b"add_op");
        let r_fn: Symbol<Fn> = sym(&r_lib, b"add_op");
        for &a in &[0, 1, -1, 100, -100, i32::MAX, i32::MIN, 12345] {
            for &b in &[0, 1, -1, 50, -50, i32::MAX, i32::MIN, 99] {
                let cv = c_fn(a, b, 0, 0);
                let rv = r_fn(a, b, 0, 0);
                assert_eq!(cv, rv, "add_op({}, {}) mismatch", a, b);
            }
        }
    }
}

#[test]
fn test_multiply_op() {
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let c_fn: Symbol<Fn> = sym(&c_lib, b"multiply_op");
        let r_fn: Symbol<Fn> = sym(&r_lib, b"multiply_op");
        for &a in &[0, 1, -1, 100, -100, 65536, 12345, -12345] {
            for &b in &[0, 1, -1, 50, -50, 65536, 7, -7] {
                let cv = c_fn(a, b, 0, 0);
                let rv = r_fn(a, b, 0, 0);
                assert_eq!(cv, rv, "multiply_op({}, {}) mismatch", a, b);
            }
        }
    }
}

#[test]
fn test_subtract_op() {
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let c_fn: Symbol<Fn> = sym(&c_lib, b"subtract_op");
        let r_fn: Symbol<Fn> = sym(&r_lib, b"subtract_op");
        for &a in &[0, 1, -1, 100, -100, i32::MAX, i32::MIN] {
            for &b in &[0, 1, -1, 50, -50, i32::MAX, i32::MIN] {
                let cv = c_fn(a, b, 0, 0);
                let rv = r_fn(a, b, 0, 0);
                assert_eq!(cv, rv, "subtract_op({}, {}) mismatch", a, b);
            }
        }
    }
}

#[test]
fn test_divide_op() {
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let c_fn: Symbol<Fn> = sym(&c_lib, b"divide_op");
        let r_fn: Symbol<Fn> = sym(&r_lib, b"divide_op");
        // Avoid INT_MIN / -1 (UB in C, panic in Rust without wrapping_div)
        for &a in &[0, 1, -1, 100, -100, 12345, -12345] {
            for &b in &[0, 1, -1, 5, -5, 100, -100, 7] {
                let cv = c_fn(a, b, 0, 0);
                let rv = r_fn(a, b, 0, 0);
                assert_eq!(cv, rv, "divide_op({}, {}) mismatch", a, b);
            }
        }
    }
}

#[test]
fn test_modulo_op() {
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let c_fn: Symbol<Fn> = sym(&c_lib, b"modulo_op");
        let r_fn: Symbol<Fn> = sym(&r_lib, b"modulo_op");
        for &a in &[0, 1, -1, 100, -100, 12345, -12345] {
            for &b in &[0, 1, -1, 5, -5, 100, -100, 7] {
                let cv = c_fn(a, b, 0, 0);
                let rv = r_fn(a, b, 0, 0);
                assert_eq!(cv, rv, "modulo_op({}, {}) mismatch", a, b);
            }
        }
    }
}

#[test]
fn test_get_operation_func() {
    // get_operation_func returns a function pointer; comparing pointers across
    // libraries doesn't make sense, so exercise it by calling the result on
    // a fixed input and ensuring both libraries' returns produce the same answer.
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type Fn = unsafe extern "C" fn(c_int) -> Option<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int>;
        let c_fn: Symbol<Fn> = sym(&c_lib, b"get_operation_func");
        let r_fn: Symbol<Fn> = sym(&r_lib, b"get_operation_func");
        for op in 0..=6 {
            let cf = c_fn(op).expect("c get_operation_func returned NULL");
            let rf = r_fn(op).expect("rust get_operation_func returned NULL");
            // Apply both to a fixed pair to verify they pick the same op.
            for &(a, b) in &[(10, 3), (100, 7), (-5, 4), (0, 1), (123, 456)] {
                let cv = cf(a, b, 0, 0);
                let rv = rf(a, b, 0, 0);
                assert_eq!(cv, rv, "get_operation_func({})(a={},b={}) mismatch", op, a, b);
            }
        }
    }
}

#[test]
fn test_parse_operation() {
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type Fn = unsafe extern "C" fn(*const c_char) -> c_int;
        let c_fn: Symbol<Fn> = sym(&c_lib, b"parse_operation");
        let r_fn: Symbol<Fn> = sym(&r_lib, b"parse_operation");

        let cases: Vec<&[u8]> = vec![
            b"+",
            b"*",
            b"-",
            b"/",
            b"%",
            b"abc+def",
            b"xx*yy",
            b"abc",
            b"",
            b"-+", // strchr finds + first per fallthrough order in C? No: C checks + first.
            b"%/-*+",
        ];
        for c in &cases {
            let s = cstr(c);
            let cv = c_fn(s.as_ptr());
            let rv = r_fn(s.as_ptr());
            assert_eq!(cv, rv, "parse_operation({:?}) mismatch", c);
        }

        // NULL pointer
        let cv = c_fn(std::ptr::null());
        let rv = r_fn(std::ptr::null());
        assert_eq!(cv, rv, "parse_operation(NULL) mismatch");
    }
}

#[test]
fn test_add_tree_node_and_find() {
    let _guard = state_lock();
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type AddFn =
            unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int;
        type FindFn = unsafe extern "C" fn(c_int) -> *mut TreeNode;
        let c_add: Symbol<AddFn> = sym(&c_lib, b"add_tree_node");
        let r_add: Symbol<AddFn> = sym(&r_lib, b"add_tree_node");
        let c_find: Symbol<FindFn> = sym(&c_lib, b"find_node_by_id");
        let r_find: Symbol<FindFn> = sym(&r_lib, b"find_node_by_id");

        reset_state(&c_lib);
        reset_state(&r_lib);

        let cases: Vec<(c_int, c_int, c_int, Vec<u8>)> = vec![
            (1, 100, -1, b"root".to_vec()),
            (2, 200, 1, b"left".to_vec()),
            (3, 300, 1, b"right".to_vec()),
            (4, 400, 2, b"left-left".to_vec()),
            (5, 500, 2, b"left-right".to_vec()),
            (6, 600, 99, b"orphan".to_vec()), // should fail (parent missing)
            (7, 700, 3, b"r-l".to_vec()),
        ];

        for (id, value, parent, label) in &cases {
            let lbl = cstr(label);
            let cv = c_add(*id, *value, *parent, lbl.as_ptr());
            let rv = r_add(*id, *value, *parent, lbl.as_ptr());
            assert_eq!(cv, rv, "add_tree_node({}, {}, {}, {:?}) mismatch", id, value, parent, label);
        }

        assert_eq!(get_count(&c_lib), get_count(&r_lib), "node_count mismatch");

        // Compare each node in the table.
        let cnt = get_count(&c_lib) as usize;
        for i in 0..cnt {
            let c_node = get_node(&c_lib, i);
            let r_node = get_node(&r_lib, i);
            assert!(nodes_eq(&c_node, &r_node), "node {} mismatch", i);
        }

        // Test find_node_by_id for existing and missing IDs.
        for id in &[1, 2, 3, 4, 5, 7, 99, 0, -1, 100] {
            let cn = c_find(*id);
            let rn = r_find(*id);
            assert_eq!(cn.is_null(), rn.is_null(), "find_node_by_id({}) null mismatch", id);
            if !cn.is_null() {
                assert!(nodes_eq(&*cn, &*rn), "find_node_by_id({}) struct mismatch", id);
            }
        }
    }
}

#[test]
fn test_calculate_tree_sum() {
    let _guard = state_lock();
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type AddFn =
            unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int;
        type SumFn = unsafe extern "C" fn(c_int) -> c_int;
        let c_add: Symbol<AddFn> = sym(&c_lib, b"add_tree_node");
        let r_add: Symbol<AddFn> = sym(&r_lib, b"add_tree_node");
        let c_sum: Symbol<SumFn> = sym(&c_lib, b"calculate_tree_sum");
        let r_sum: Symbol<SumFn> = sym(&r_lib, b"calculate_tree_sum");

        reset_state(&c_lib);
        reset_state(&r_lib);

        let labels = [b"root\0".as_ref(), b"a\0".as_ref(), b"b\0".as_ref(), b"c\0".as_ref(), b"d\0".as_ref()];
        let nodes: Vec<(c_int, c_int, c_int)> = vec![
            (1, 10, -1),
            (2, 20, 1),
            (3, 30, 1),
            (4, 40, 2),
            (5, 50, 2),
        ];

        for (i, (id, value, parent)) in nodes.iter().enumerate() {
            let lbl: Vec<c_char> = labels[i].iter().map(|&b| b as c_char).collect();
            c_add(*id, *value, *parent, lbl.as_ptr());
            r_add(*id, *value, *parent, lbl.as_ptr());
        }

        for &id in &[1, 2, 3, 4, 5, 99, 0, -1] {
            let cv = c_sum(id);
            let rv = r_sum(id);
            assert_eq!(cv, rv, "calculate_tree_sum({}) mismatch", id);
        }
    }
}

#[test]
fn test_inreftree() {
    let _guard = state_lock();
    unsafe {
        let (c_lib, r_lib) = open_pair();
        type Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let c_fn: Symbol<Fn> = sym(&c_lib, b"inreftree");
        let r_fn: Symbol<Fn> = sym(&r_lib, b"inreftree");

        // NOTE: When tree_sum < 0, the C code accesses op_string[tree_sum % 4]
        // with a negative index — undefined behavior in C. Only test inputs
        // where tree_sum >= 0 so both implementations are exercising
        // well-defined code paths.
        let cases = vec![
            (1, 2, 3, 4),
            (0, 0, 0, 0),
            (10, 20, 30, 40),
            (-5, 7, 3, 9), // sum = 14
            (100, 0, 0, 0),
            (1, 1, 1, 1),
            (0, 5, 0, 0),
            (0, 0, 5, 0),
            (0, 0, 0, 5),
            (7, 11, 13, 17),
            (1000000, 2000000, 3000000, 4000000),
            (5, 2, 1, 3), // sum = 11 (mod 4 = 3 -> %)
            (2, 2, 2, 2), // sum = 8 (mod 4 = 0 -> +)
            (3, 1, 4, 1), // sum = 9 (mod 4 = 1 -> *)
            (1, 2, 1, 2), // sum = 6 (mod 4 = 2 -> -)
        ];
        for (a, b, c, d) in cases {
            let cv = c_fn(a, b, c, d);
            let rv = r_fn(a, b, c, d);
            assert_eq!(cv, rv, "inreftree({}, {}, {}, {}) mismatch", a, b, c, d);
        }
    }
}
