// Integration test: load both the C-built shared library and the Rust-built
// shared library via libloading and compare outputs through the FFI boundary.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_double, c_int};
use std::path::PathBuf;

const MAX_NAME_LEN: usize = 50;

#[repr(C)]
#[derive(Copy, Clone)]
struct Node {
    id: c_int,
    parent_id: c_int,
    name: [c_char; MAX_NAME_LEN],
    value: c_double,
    active: c_int,
}

type AddNodeFn =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_double) -> c_int;
type FindNodeByIdFn = unsafe extern "C" fn(c_int) -> *mut Node;
type GetChildrenCountFn = unsafe extern "C" fn(c_int) -> c_int;
type CalcSubtreeSumFn = unsafe extern "C" fn(c_int) -> c_double;
type ProcessStringFn = unsafe extern "C" fn(*mut c_char) -> c_int;
type SafeD2IFn = unsafe extern "C" fn(c_double) -> c_int;
type MaxnminFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/release/libmaxnmin_lib.so");
    p
}

fn load_libs() -> (Library, Library) {
    let c = unsafe { Library::new(c_so_path()).expect("failed to load C .so") };
    let r = unsafe { Library::new(rust_so_path()).expect("failed to load Rust .so") };
    (c, r)
}

fn cstr(buf: &mut Vec<u8>) -> *mut c_char {
    buf.push(0); // ensure NUL-terminated
    buf.as_mut_ptr() as *mut c_char
}

#[test]
fn test_safe_double_to_int() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<SafeD2IFn> = c.get(b"safe_double_to_int").unwrap();
        let rf: Symbol<SafeD2IFn> = r.get(b"safe_double_to_int").unwrap();

        let cases: &[f64] = &[
            0.0,
            1.0,
            -1.0,
            1.5,
            -1.5,
            42.7,
            -42.7,
            i32::MAX as f64,
            i32::MIN as f64,
            (i32::MAX as f64) + 1.0,
            (i32::MIN as f64) - 1.0,
            1e100,
            -1e100,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];
        for &d in cases {
            let cv = cf(d);
            let rv = rf(d);
            assert_eq!(cv, rv, "safe_double_to_int({}) mismatch C={} R={}", d, cv, rv);
        }
    }
}

#[test]
fn test_process_string() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<ProcessStringFn> = c.get(b"process_string").unwrap();
        let rf: Symbol<ProcessStringFn> = r.get(b"process_string").unwrap();

        let cases: &[&[u8]] = &[
            b"",
            b"a",
            b"hello",
            b"abcXYZ123",
            b"\xff\x80\x7f",
        ];
        for &s in cases {
            let mut buf_c = s.to_vec();
            let mut buf_r = s.to_vec();
            let ptr_c = cstr(&mut buf_c);
            let ptr_r = cstr(&mut buf_r);
            let cv = cf(ptr_c);
            let rv = rf(ptr_r);
            assert_eq!(cv, rv, "process_string({:?}) mismatch C={} R={}", s, cv, rv);
        }
    }
}

#[test]
fn test_add_node_and_find() {
    let (c, r) = load_libs();
    unsafe {
        let c_add: Symbol<AddNodeFn> = c.get(b"add_node").unwrap();
        let r_add: Symbol<AddNodeFn> = r.get(b"add_node").unwrap();
        let c_find: Symbol<FindNodeByIdFn> = c.get(b"find_node_by_id").unwrap();
        let r_find: Symbol<FindNodeByIdFn> = r.get(b"find_node_by_id").unwrap();

        let names: &[&[u8]] = &[
            b"alpha\0",
            b"bravo\0",
            b"charlie\0",
            b"delta\0",
        ];

        for (i, name) in names.iter().enumerate() {
            let id = (i as c_int) + 1;
            let parent = if i == 0 { -1 } else { 1 };
            let val = (i as f64) * 1.25 + 0.5;
            let nptr = name.as_ptr() as *const c_char;
            let cv = c_add(id, parent, nptr, val);
            let rv = r_add(id, parent, nptr, val);
            assert_eq!(cv, rv, "add_node return mismatch idx={}", i);
        }

        for id in 0..6 {
            let cn = c_find(id);
            let rn = r_find(id);
            assert_eq!(cn.is_null(), rn.is_null(), "find_node_by_id null mismatch id={}", id);
            if !cn.is_null() {
                let cn = &*cn;
                let rn = &*rn;
                assert_eq!(cn.id, rn.id);
                assert_eq!(cn.parent_id, rn.parent_id);
                assert_eq!(cn.value.to_bits(), rn.value.to_bits(), "value mismatch id={}", id);
                assert_eq!(cn.active, rn.active);
                let cn_name: &[u8] = std::slice::from_raw_parts(
                    cn.name.as_ptr() as *const u8, MAX_NAME_LEN);
                let rn_name: &[u8] = std::slice::from_raw_parts(
                    rn.name.as_ptr() as *const u8, MAX_NAME_LEN);
                assert_eq!(cn_name, rn_name, "name bytes mismatch id={}", id);
            }
        }
    }
}

#[test]
fn test_get_children_count() {
    let (c, r) = load_libs();
    unsafe {
        let c_add: Symbol<AddNodeFn> = c.get(b"add_node").unwrap();
        let r_add: Symbol<AddNodeFn> = r.get(b"add_node").unwrap();
        let c_gc: Symbol<GetChildrenCountFn> = c.get(b"get_children_count").unwrap();
        let r_gc: Symbol<GetChildrenCountFn> = r.get(b"get_children_count").unwrap();

        // Note: each library has its own internal node_storage state — that's fine
        // because we mirror identical operations on both.
        let entries: &[(c_int, c_int, &[u8], f64)] = &[
            (1, -1, b"r\0", 1.0),
            (2,  1, b"a\0", 2.0),
            (3,  1, b"b\0", 3.0),
            (4,  2, b"c\0", 4.0),
            (5,  2, b"d\0", 5.0),
            (6,  3, b"e\0", 6.0),
        ];
        for &(id, p, name, v) in entries {
            let np = name.as_ptr() as *const c_char;
            c_add(id, p, np, v);
            r_add(id, p, np, v);
        }
        for pid in -1..=6 {
            let cv = c_gc(pid);
            let rv = r_gc(pid);
            assert_eq!(cv, rv, "get_children_count({}) mismatch", pid);
        }
    }
}

#[test]
fn test_calculate_subtree_sum() {
    let (c, r) = load_libs();
    unsafe {
        let c_add: Symbol<AddNodeFn> = c.get(b"add_node").unwrap();
        let r_add: Symbol<AddNodeFn> = r.get(b"add_node").unwrap();
        let c_css: Symbol<CalcSubtreeSumFn> = c.get(b"calculate_subtree_sum").unwrap();
        let r_css: Symbol<CalcSubtreeSumFn> = r.get(b"calculate_subtree_sum").unwrap();

        // Identical setup in both libraries.
        let entries: &[(c_int, c_int, &[u8], f64)] = &[
            (10, -1, b"x\0", 1.5),
            (11, 10, b"y\0", 2.25),
            (12, 10, b"z\0", -0.75),
            (13, 11, b"w\0", 4.0),
        ];
        for &(id, p, name, v) in entries {
            let np = name.as_ptr() as *const c_char;
            c_add(id, p, np, v);
            r_add(id, p, np, v);
        }
        for nid in &[10, 11, 12, 13, 99] {
            let cv = c_css(*nid);
            let rv = r_css(*nid);
            assert_eq!(cv.to_bits(), rv.to_bits(),
                "calculate_subtree_sum({}) mismatch C={} R={}", nid, cv, rv);
        }
    }
}

#[test]
fn test_maxnmin_many_inputs() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<MaxnminFn> = c.get(b"maxnmin").unwrap();
        let rf: Symbol<MaxnminFn> = r.get(b"maxnmin").unwrap();

        let params: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 2, 3, 4),
            (-1, -2, 3, 4),
            (5, 7, 11, 13),
            (100, 200, 300, 400),
            (-100, -200, -300, -400),
            (6, 12, 1, 2),
            (7, 8, 9, 10),
            (1000000, 1000000, 1000000, 1000000),
            (-1000000, -1000000, -1000000, -1000000),
            (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
            (i32::MIN, i32::MIN, 1, 1),
            (i32::MAX, 0, 1, 0),
            (0, 0, -1, 0), // param3+1 == 0 -> divide by zero produces NaN/inf
            (0, 0, -1, 1),
        ];
        for &(p1, p2, p3, p4) in params {
            let cv = cf(p1, p2, p3, p4);
            let rv = rf(p1, p2, p3, p4);
            assert_eq!(cv, rv,
                "maxnmin({}, {}, {}, {}) mismatch C={} R={}",
                p1, p2, p3, p4, cv, rv);
        }
    }
}

#[test]
fn test_export_symbols_present() {
    let (c, r) = load_libs();
    let symbols: &[&[u8]] = &[
        b"add_node",
        b"find_node_by_id",
        b"get_children_count",
        b"calculate_subtree_sum",
        b"process_string",
        b"safe_double_to_int",
        b"maxnmin",
    ];
    unsafe {
        for s in symbols {
            let _: Symbol<*const ()> = c.get(s).unwrap_or_else(|e| {
                panic!("C lib missing symbol {:?}: {}", std::str::from_utf8(s).unwrap(), e)
            });
            let _: Symbol<*const ()> = r.get(s).unwrap_or_else(|e| {
                panic!("Rust lib missing symbol {:?}: {}", std::str::from_utf8(s).unwrap(), e)
            });
        }
    }
}
