use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

type TreeIdT = u64;

const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;

#[repr(C)]
struct HashmapEntry {
    key: TreeIdT,
    value: *mut c_void,
    occupied: c_int,
    deleted: c_int,
}

#[repr(C)]
struct HashmapT {
    entries: *mut HashmapEntry,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

#[repr(C)]
struct TreeNodeT {
    id: TreeIdT,
    parent_id: TreeIdT,
    child_ids: [TreeIdT; MAX_CHILDREN],
    child_count: c_int,
    data: [u8; MAX_DATA_LENGTH],
}

#[repr(C)]
struct TreeT {
    node_map: *mut HashmapT,
    root_id: TreeIdT,
    has_root: c_int,
    node_count: usize,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

macro_rules! load_fn {
    ($lib:expr, $name:expr, $ty:ty) => {
        unsafe { $lib.get::<$ty>($name).unwrap() }
    };
}

// ---- Hashmap tests ----

#[test]
fn test_hashmap_create_destroy() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        let c_create: Symbol<unsafe extern "C" fn() -> *mut HashmapT> = load_fn!(c, b"hashmap_create", unsafe extern "C" fn() -> *mut HashmapT);
        let c_destroy: Symbol<unsafe extern "C" fn(*mut HashmapT)> = load_fn!(c, b"hashmap_destroy", unsafe extern "C" fn(*mut HashmapT));
        let c_size: Symbol<unsafe extern "C" fn(*mut HashmapT) -> usize> = load_fn!(c, b"hashmap_size", unsafe extern "C" fn(*mut HashmapT) -> usize);

        let r_create: Symbol<unsafe extern "C" fn() -> *mut HashmapT> = load_fn!(r, b"hashmap_create", unsafe extern "C" fn() -> *mut HashmapT);
        let r_destroy: Symbol<unsafe extern "C" fn(*mut HashmapT)> = load_fn!(r, b"hashmap_destroy", unsafe extern "C" fn(*mut HashmapT));
        let r_size: Symbol<unsafe extern "C" fn(*mut HashmapT) -> usize> = load_fn!(r, b"hashmap_size", unsafe extern "C" fn(*mut HashmapT) -> usize);

        let cm = c_create();
        let rm = r_create();
        assert!(!cm.is_null());
        assert!(!rm.is_null());
        assert_eq!(c_size(cm), r_size(rm));
        assert_eq!(c_size(cm), 0);
        c_destroy(cm);
        r_destroy(rm);
    }
}

#[test]
fn test_hashmap_put_get_contains_size() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type CreateFn = unsafe extern "C" fn() -> *mut HashmapT;
        type DestroyFn = unsafe extern "C" fn(*mut HashmapT);
        type PutFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT, *mut c_void) -> c_int;
        type GetFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT) -> *mut c_void;
        type ContainsFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT) -> c_int;
        type SizeFn = unsafe extern "C" fn(*mut HashmapT) -> usize;

        let c_create: Symbol<CreateFn> = load_fn!(c, b"hashmap_create", CreateFn);
        let c_destroy: Symbol<DestroyFn> = load_fn!(c, b"hashmap_destroy", DestroyFn);
        let c_put: Symbol<PutFn> = load_fn!(c, b"hashmap_put", PutFn);
        let c_get: Symbol<GetFn> = load_fn!(c, b"hashmap_get", GetFn);
        let c_contains: Symbol<ContainsFn> = load_fn!(c, b"hashmap_contains", ContainsFn);
        let c_size: Symbol<SizeFn> = load_fn!(c, b"hashmap_size", SizeFn);

        let r_create: Symbol<CreateFn> = load_fn!(r, b"hashmap_create", CreateFn);
        let r_destroy: Symbol<DestroyFn> = load_fn!(r, b"hashmap_destroy", DestroyFn);
        let r_put: Symbol<PutFn> = load_fn!(r, b"hashmap_put", PutFn);
        let r_get: Symbol<GetFn> = load_fn!(r, b"hashmap_get", GetFn);
        let r_contains: Symbol<ContainsFn> = load_fn!(r, b"hashmap_contains", ContainsFn);
        let r_size: Symbol<SizeFn> = load_fn!(r, b"hashmap_size", SizeFn);

        let cm = c_create();
        let rm = r_create();

        // Use stack-allocated values as void* pointers (just testing pointer storage)
        let mut vals: [i32; 5] = [10, 20, 30, 40, 50];

        for i in 0..5u64 {
            let c_ret = c_put(cm, i + 1, &mut vals[i as usize] as *mut i32 as *mut c_void);
            // For Rust, use a separate set of values at different addresses
            let r_ret = r_put(rm, i + 1, &mut vals[i as usize] as *mut i32 as *mut c_void);
            assert_eq!(c_ret, r_ret, "put return mismatch for key {}", i + 1);
        }

        assert_eq!(c_size(cm), r_size(rm));
        assert_eq!(c_size(cm), 5);

        // Test contains
        for i in 0..7u64 {
            assert_eq!(c_contains(cm, i), r_contains(rm, i), "contains mismatch for key {}", i);
        }

        // Test get returns non-null for existing keys, null for missing
        for i in 1..=5u64 {
            assert!(!c_get(cm, i).is_null());
            assert!(!r_get(rm, i).is_null());
        }
        assert!(c_get(cm, 99).is_null());
        assert!(r_get(rm, 99).is_null());

        c_destroy(cm);
        r_destroy(rm);
    }
}

#[test]
fn test_hashmap_remove() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type CreateFn = unsafe extern "C" fn() -> *mut HashmapT;
        type DestroyFn = unsafe extern "C" fn(*mut HashmapT);
        type PutFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT, *mut c_void) -> c_int;
        type RemoveFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT) -> *mut c_void;
        type SizeFn = unsafe extern "C" fn(*mut HashmapT) -> usize;
        type ContainsFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT) -> c_int;

        let c_create: Symbol<CreateFn> = load_fn!(c, b"hashmap_create", CreateFn);
        let c_destroy: Symbol<DestroyFn> = load_fn!(c, b"hashmap_destroy", DestroyFn);
        let c_put: Symbol<PutFn> = load_fn!(c, b"hashmap_put", PutFn);
        let c_remove: Symbol<RemoveFn> = load_fn!(c, b"hashmap_remove", RemoveFn);
        let c_size: Symbol<SizeFn> = load_fn!(c, b"hashmap_size", SizeFn);
        let c_contains: Symbol<ContainsFn> = load_fn!(c, b"hashmap_contains", ContainsFn);

        let r_create: Symbol<CreateFn> = load_fn!(r, b"hashmap_create", CreateFn);
        let r_destroy: Symbol<DestroyFn> = load_fn!(r, b"hashmap_destroy", DestroyFn);
        let r_put: Symbol<PutFn> = load_fn!(r, b"hashmap_put", PutFn);
        let r_remove: Symbol<RemoveFn> = load_fn!(r, b"hashmap_remove", RemoveFn);
        let r_size: Symbol<SizeFn> = load_fn!(r, b"hashmap_size", SizeFn);
        let r_contains: Symbol<ContainsFn> = load_fn!(r, b"hashmap_contains", ContainsFn);

        let cm = c_create();
        let rm = r_create();

        let mut v1 = 100i32;
        let mut v2 = 200i32;
        c_put(cm, 1, &mut v1 as *mut _ as *mut c_void);
        c_put(cm, 2, &mut v2 as *mut _ as *mut c_void);
        r_put(rm, 1, &mut v1 as *mut _ as *mut c_void);
        r_put(rm, 2, &mut v2 as *mut _ as *mut c_void);

        // Remove key 1
        let c_removed = c_remove(cm, 1);
        let r_removed = r_remove(rm, 1);
        assert_eq!(c_removed.is_null(), r_removed.is_null());
        assert_eq!(c_size(cm), r_size(rm));
        assert_eq!(c_contains(cm, 1), r_contains(rm, 1));
        assert_eq!(c_contains(cm, 2), r_contains(rm, 2));

        // Remove non-existent key
        let c_removed2 = c_remove(cm, 99);
        let r_removed2 = r_remove(rm, 99);
        assert_eq!(c_removed2.is_null(), r_removed2.is_null());

        c_destroy(cm);
        r_destroy(rm);
    }
}

#[test]
fn test_hashmap_clear() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type CreateFn = unsafe extern "C" fn() -> *mut HashmapT;
        type DestroyFn = unsafe extern "C" fn(*mut HashmapT);
        type PutFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT, *mut c_void) -> c_int;
        type ClearFn = unsafe extern "C" fn(*mut HashmapT);
        type SizeFn = unsafe extern "C" fn(*mut HashmapT) -> usize;

        let c_create: Symbol<CreateFn> = load_fn!(c, b"hashmap_create", CreateFn);
        let c_destroy: Symbol<DestroyFn> = load_fn!(c, b"hashmap_destroy", DestroyFn);
        let c_put: Symbol<PutFn> = load_fn!(c, b"hashmap_put", PutFn);
        let c_clear: Symbol<ClearFn> = load_fn!(c, b"hashmap_clear", ClearFn);
        let c_size: Symbol<SizeFn> = load_fn!(c, b"hashmap_size", SizeFn);

        let r_create: Symbol<CreateFn> = load_fn!(r, b"hashmap_create", CreateFn);
        let r_destroy: Symbol<DestroyFn> = load_fn!(r, b"hashmap_destroy", DestroyFn);
        let r_put: Symbol<PutFn> = load_fn!(r, b"hashmap_put", PutFn);
        let r_clear: Symbol<ClearFn> = load_fn!(r, b"hashmap_clear", ClearFn);
        let r_size: Symbol<SizeFn> = load_fn!(r, b"hashmap_size", SizeFn);

        let cm = c_create();
        let rm = r_create();

        let mut v = 42i32;
        for i in 0..10u64 {
            c_put(cm, i, &mut v as *mut _ as *mut c_void);
            r_put(rm, i, &mut v as *mut _ as *mut c_void);
        }
        assert_eq!(c_size(cm), r_size(rm));

        c_clear(cm);
        r_clear(rm);
        assert_eq!(c_size(cm), r_size(rm));
        assert_eq!(c_size(cm), 0);

        c_destroy(cm);
        r_destroy(rm);
    }
}

#[test]
fn test_hashmap_many_entries() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type CreateFn = unsafe extern "C" fn() -> *mut HashmapT;
        type DestroyFn = unsafe extern "C" fn(*mut HashmapT);
        type PutFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT, *mut c_void) -> c_int;
        type GetFn = unsafe extern "C" fn(*mut HashmapT, TreeIdT) -> *mut c_void;
        type SizeFn = unsafe extern "C" fn(*mut HashmapT) -> usize;

        let c_create: Symbol<CreateFn> = load_fn!(c, b"hashmap_create", CreateFn);
        let c_destroy: Symbol<DestroyFn> = load_fn!(c, b"hashmap_destroy", DestroyFn);
        let c_put: Symbol<PutFn> = load_fn!(c, b"hashmap_put", PutFn);
        let c_get: Symbol<GetFn> = load_fn!(c, b"hashmap_get", GetFn);
        let c_size: Symbol<SizeFn> = load_fn!(c, b"hashmap_size", SizeFn);

        let r_create: Symbol<CreateFn> = load_fn!(r, b"hashmap_create", CreateFn);
        let r_destroy: Symbol<DestroyFn> = load_fn!(r, b"hashmap_destroy", DestroyFn);
        let r_put: Symbol<PutFn> = load_fn!(r, b"hashmap_put", PutFn);
        let r_get: Symbol<GetFn> = load_fn!(r, b"hashmap_get", GetFn);
        let r_size: Symbol<SizeFn> = load_fn!(r, b"hashmap_size", SizeFn);

        let cm = c_create();
        let rm = r_create();

        let mut vals = vec![0i32; 100];
        for i in 0..100 {
            vals[i] = (i * 10) as i32;
            let c_ret = c_put(cm, i as u64, &mut vals[i] as *mut _ as *mut c_void);
            let r_ret = r_put(rm, i as u64, &mut vals[i] as *mut _ as *mut c_void);
            assert_eq!(c_ret, r_ret);
        }

        assert_eq!(c_size(cm), r_size(rm));
        assert_eq!(c_size(cm), 100);

        for i in 0..100u64 {
            assert_eq!(c_get(cm, i).is_null(), r_get(rm, i).is_null());
        }

        c_destroy(cm);
        r_destroy(rm);
    }
}

// ---- Tree tests ----

// Helper struct to hold loaded tree function symbols
struct TreeLib {
    _lib: Library,
}

macro_rules! define_tree_fns {
    ($lib:expr) => {{
        let _ = &$lib; // just to verify it's a Library
        &$lib
    }};
}

// We'll use raw function pointers loaded per-test to keep things simple.

#[test]
fn test_tree_create_delete() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type TCreateFn = unsafe extern "C" fn() -> *mut TreeT;
        type TDeleteFn = unsafe extern "C" fn(*mut TreeT);
        type TSizeFn = unsafe extern "C" fn(*mut TreeT) -> usize;

        let c_create: Symbol<TCreateFn> = load_fn!(c, b"tree_create", TCreateFn);
        let c_delete: Symbol<TDeleteFn> = load_fn!(c, b"tree_delete", TDeleteFn);
        let c_size: Symbol<TSizeFn> = load_fn!(c, b"tree_size", TSizeFn);

        let r_create: Symbol<TCreateFn> = load_fn!(r, b"tree_create", TCreateFn);
        let r_delete: Symbol<TDeleteFn> = load_fn!(r, b"tree_delete", TDeleteFn);
        let r_size: Symbol<TSizeFn> = load_fn!(r, b"tree_size", TSizeFn);

        let ct = c_create();
        let rt = r_create();
        assert!(!ct.is_null());
        assert!(!rt.is_null());
        assert_eq!(c_size(ct), r_size(rt));
        assert_eq!(c_size(ct), 0);
        assert_eq!((*ct).has_root, (*rt).has_root);
        c_delete(ct);
        r_delete(rt);
    }
}

#[test]
fn test_tree_add_and_query() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type TCreateFn = unsafe extern "C" fn() -> *mut TreeT;
        type TDeleteFn = unsafe extern "C" fn(*mut TreeT);
        type TAddFn = unsafe extern "C" fn(*mut TreeT, TreeIdT, TreeIdT, *const c_char) -> c_int;
        type TSizeFn = unsafe extern "C" fn(*mut TreeT) -> usize;
        type TContainsFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;
        type TGetNodeFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> *mut TreeNodeT;
        type TDepthFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;
        type THeightFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;
        type TDescFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;

        let c_create: Symbol<TCreateFn> = load_fn!(c, b"tree_create", TCreateFn);
        let c_delete: Symbol<TDeleteFn> = load_fn!(c, b"tree_delete", TDeleteFn);
        let c_add: Symbol<TAddFn> = load_fn!(c, b"tree_add_node", TAddFn);
        let c_size: Symbol<TSizeFn> = load_fn!(c, b"tree_size", TSizeFn);
        let c_contains: Symbol<TContainsFn> = load_fn!(c, b"tree_contains", TContainsFn);
        let c_get_node: Symbol<TGetNodeFn> = load_fn!(c, b"tree_get_node", TGetNodeFn);
        let c_depth: Symbol<TDepthFn> = load_fn!(c, b"tree_get_depth", TDepthFn);
        let c_height: Symbol<THeightFn> = load_fn!(c, b"tree_get_height", THeightFn);
        let c_desc: Symbol<TDescFn> = load_fn!(c, b"tree_count_descendants", TDescFn);

        let r_create: Symbol<TCreateFn> = load_fn!(r, b"tree_create", TCreateFn);
        let r_delete: Symbol<TDeleteFn> = load_fn!(r, b"tree_delete", TDeleteFn);
        let r_add: Symbol<TAddFn> = load_fn!(r, b"tree_add_node", TAddFn);
        let r_size: Symbol<TSizeFn> = load_fn!(r, b"tree_size", TSizeFn);
        let r_contains: Symbol<TContainsFn> = load_fn!(r, b"tree_contains", TContainsFn);
        let r_get_node: Symbol<TGetNodeFn> = load_fn!(r, b"tree_get_node", TGetNodeFn);
        let r_depth: Symbol<TDepthFn> = load_fn!(r, b"tree_get_depth", TDepthFn);
        let r_height: Symbol<THeightFn> = load_fn!(r, b"tree_get_height", THeightFn);
        let r_desc: Symbol<TDescFn> = load_fn!(r, b"tree_count_descendants", TDescFn);

        let ct = c_create();
        let rt = r_create();

        // Build tree:  1 -> {2, 3, 4}, 2 -> {5, 6}, 3 -> {7}, 7 -> {10}, 4 -> {8, 9}
        let nodes: &[(u64, u64, &str)] = &[
            (1, 0, "root"), (2, 1, "child1"), (3, 1, "child2"), (4, 1, "child3"),
            (5, 2, "gc1"), (6, 2, "gc2"), (7, 3, "gc3"), (8, 4, "gc4"),
            (9, 4, "gc5"), (10, 7, "ggc1"),
        ];

        for &(id, pid, data) in nodes {
            let cdata = CString::new(data).unwrap();
            let c_ret = c_add(ct, id, pid, cdata.as_ptr());
            let r_ret = r_add(rt, id, pid, cdata.as_ptr());
            assert_eq!(c_ret, r_ret, "add_node mismatch for id={}", id);
        }

        assert_eq!(c_size(ct), r_size(rt));
        assert_eq!((*ct).has_root, (*rt).has_root);
        assert_eq!((*ct).root_id, (*rt).root_id);

        // Test contains
        for id in 0..12u64 {
            assert_eq!(c_contains(ct, id), r_contains(rt, id), "contains mismatch id={}", id);
        }

        // Test get_node fields
        for &(id, _, _) in nodes {
            let cn = c_get_node(ct, id);
            let rn = r_get_node(rt, id);
            assert!(!cn.is_null());
            assert!(!rn.is_null());
            assert_eq!((*cn).id, (*rn).id);
            assert_eq!((*cn).parent_id, (*rn).parent_id);
            assert_eq!((*cn).child_count, (*rn).child_count);
            for i in 0..(*cn).child_count as usize {
                assert_eq!((*cn).child_ids[i], (*rn).child_ids[i]);
            }
            assert_eq!(&(&(*cn).data)[..], &(&(*rn).data)[..]);
        }

        // Test depth
        for &(id, _, _) in nodes {
            assert_eq!(c_depth(ct, id), r_depth(rt, id), "depth mismatch id={}", id);
        }
        assert_eq!(c_depth(ct, 99), r_depth(rt, 99)); // non-existent

        // Test height
        for &(id, _, _) in nodes {
            assert_eq!(c_height(ct, id), r_height(rt, id), "height mismatch id={}", id);
        }

        // Test count_descendants
        for &(id, _, _) in nodes {
            assert_eq!(c_desc(ct, id), r_desc(rt, id), "descendants mismatch id={}", id);
        }

        c_delete(ct);
        r_delete(rt);
    }
}

#[test]
fn test_tree_find_path() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type TCreateFn = unsafe extern "C" fn() -> *mut TreeT;
        type TDeleteFn = unsafe extern "C" fn(*mut TreeT);
        type TAddFn = unsafe extern "C" fn(*mut TreeT, TreeIdT, TreeIdT, *const c_char) -> c_int;
        type TFindPathFn = unsafe extern "C" fn(*mut TreeT, TreeIdT, *mut TreeIdT, c_int) -> c_int;

        let c_create: Symbol<TCreateFn> = load_fn!(c, b"tree_create", TCreateFn);
        let c_delete: Symbol<TDeleteFn> = load_fn!(c, b"tree_delete", TDeleteFn);
        let c_add: Symbol<TAddFn> = load_fn!(c, b"tree_add_node", TAddFn);
        let c_find: Symbol<TFindPathFn> = load_fn!(c, b"tree_find_path", TFindPathFn);

        let r_create: Symbol<TCreateFn> = load_fn!(r, b"tree_create", TCreateFn);
        let r_delete: Symbol<TDeleteFn> = load_fn!(r, b"tree_delete", TDeleteFn);
        let r_add: Symbol<TAddFn> = load_fn!(r, b"tree_add_node", TAddFn);
        let r_find: Symbol<TFindPathFn> = load_fn!(r, b"tree_find_path", TFindPathFn);

        let ct = c_create();
        let rt = r_create();

        let nodes: &[(u64, u64, &str)] = &[
            (1, 0, "root"), (2, 1, "child"), (3, 2, "grandchild"),
            (4, 3, "great-grandchild"),
        ];
        for &(id, pid, data) in nodes {
            let cdata = CString::new(data).unwrap();
            c_add(ct, id, pid, cdata.as_ptr());
            r_add(rt, id, pid, cdata.as_ptr());
        }

        // Test find_path for each node
        for &(id, _, _) in nodes {
            let mut c_path = [0u64; 10];
            let mut r_path = [0u64; 10];
            let c_len = c_find(ct, id, c_path.as_mut_ptr(), 10);
            let r_len = r_find(rt, id, r_path.as_mut_ptr(), 10);
            assert_eq!(c_len, r_len, "find_path length mismatch for id={}", id);
            for i in 0..c_len as usize {
                assert_eq!(c_path[i], r_path[i], "find_path[{}] mismatch for id={}", i, id);
            }
        }

        // Non-existent node
        let mut c_path = [0u64; 10];
        let mut r_path = [0u64; 10];
        assert_eq!(c_find(ct, 99, c_path.as_mut_ptr(), 10), r_find(rt, 99, r_path.as_mut_ptr(), 10));

        c_delete(ct);
        r_delete(rt);
    }
}

#[test]
fn test_tree_remove_leaf() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type TCreateFn = unsafe extern "C" fn() -> *mut TreeT;
        type TDeleteFn = unsafe extern "C" fn(*mut TreeT);
        type TAddFn = unsafe extern "C" fn(*mut TreeT, TreeIdT, TreeIdT, *const c_char) -> c_int;
        type TRemoveFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;
        type TSizeFn = unsafe extern "C" fn(*mut TreeT) -> usize;
        type TContainsFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;
        type TGetNodeFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> *mut TreeNodeT;

        let c_create: Symbol<TCreateFn> = load_fn!(c, b"tree_create", TCreateFn);
        let c_delete: Symbol<TDeleteFn> = load_fn!(c, b"tree_delete", TDeleteFn);
        let c_add: Symbol<TAddFn> = load_fn!(c, b"tree_add_node", TAddFn);
        let c_remove: Symbol<TRemoveFn> = load_fn!(c, b"tree_remove_node", TRemoveFn);
        let c_size: Symbol<TSizeFn> = load_fn!(c, b"tree_size", TSizeFn);
        let c_contains: Symbol<TContainsFn> = load_fn!(c, b"tree_contains", TContainsFn);
        let c_get_node: Symbol<TGetNodeFn> = load_fn!(c, b"tree_get_node", TGetNodeFn);

        let r_create: Symbol<TCreateFn> = load_fn!(r, b"tree_create", TCreateFn);
        let r_delete: Symbol<TDeleteFn> = load_fn!(r, b"tree_delete", TDeleteFn);
        let r_add: Symbol<TAddFn> = load_fn!(r, b"tree_add_node", TAddFn);
        let r_remove: Symbol<TRemoveFn> = load_fn!(r, b"tree_remove_node", TRemoveFn);
        let r_size: Symbol<TSizeFn> = load_fn!(r, b"tree_size", TSizeFn);
        let r_contains: Symbol<TContainsFn> = load_fn!(r, b"tree_contains", TContainsFn);
        let r_get_node: Symbol<TGetNodeFn> = load_fn!(r, b"tree_get_node", TGetNodeFn);

        let ct = c_create();
        let rt = r_create();

        for &(id, pid, data) in &[(1u64, 0u64, "root"), (2, 1, "child1"), (3, 1, "child2")] {
            let cdata = CString::new(data).unwrap();
            c_add(ct, id, pid, cdata.as_ptr());
            r_add(rt, id, pid, cdata.as_ptr());
        }

        // Remove leaf node 3
        let c_ret = c_remove(ct, 3);
        let r_ret = r_remove(rt, 3);
        assert_eq!(c_ret, r_ret);
        assert_eq!(c_size(ct), r_size(rt));
        assert_eq!(c_contains(ct, 3), r_contains(rt, 3));

        // Check parent's child list
        let cn = c_get_node(ct, 1);
        let rn = r_get_node(rt, 1);
        assert_eq!((*cn).child_count, (*rn).child_count);
        for i in 0..(*cn).child_count as usize {
            assert_eq!((*cn).child_ids[i], (*rn).child_ids[i]);
        }

        c_delete(ct);
        r_delete(rt);
    }
}

#[test]
fn test_tree_remove_subtree() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type TCreateFn = unsafe extern "C" fn() -> *mut TreeT;
        type TDeleteFn = unsafe extern "C" fn(*mut TreeT);
        type TAddFn = unsafe extern "C" fn(*mut TreeT, TreeIdT, TreeIdT, *const c_char) -> c_int;
        type TRemoveFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;
        type TSizeFn = unsafe extern "C" fn(*mut TreeT) -> usize;
        type TContainsFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;

        let c_create: Symbol<TCreateFn> = load_fn!(c, b"tree_create", TCreateFn);
        let c_delete: Symbol<TDeleteFn> = load_fn!(c, b"tree_delete", TDeleteFn);
        let c_add: Symbol<TAddFn> = load_fn!(c, b"tree_add_node", TAddFn);
        let c_remove: Symbol<TRemoveFn> = load_fn!(c, b"tree_remove_node", TRemoveFn);
        let c_size: Symbol<TSizeFn> = load_fn!(c, b"tree_size", TSizeFn);
        let c_contains: Symbol<TContainsFn> = load_fn!(c, b"tree_contains", TContainsFn);

        let r_create: Symbol<TCreateFn> = load_fn!(r, b"tree_create", TCreateFn);
        let r_delete: Symbol<TDeleteFn> = load_fn!(r, b"tree_delete", TDeleteFn);
        let r_add: Symbol<TAddFn> = load_fn!(r, b"tree_add_node", TAddFn);
        let r_remove: Symbol<TRemoveFn> = load_fn!(r, b"tree_remove_node", TRemoveFn);
        let r_size: Symbol<TSizeFn> = load_fn!(r, b"tree_size", TSizeFn);
        let r_contains: Symbol<TContainsFn> = load_fn!(r, b"tree_contains", TContainsFn);

        let ct = c_create();
        let rt = r_create();

        let nodes: &[(u64, u64, &str)] = &[
            (1, 0, "root"), (2, 1, "child1"), (3, 2, "gc1"),
            (4, 2, "gc2"), (5, 1, "child2"),
        ];
        for &(id, pid, data) in nodes {
            let cdata = CString::new(data).unwrap();
            c_add(ct, id, pid, cdata.as_ptr());
            r_add(rt, id, pid, cdata.as_ptr());
        }

        // Remove node 2 (has children 3, 4)
        assert_eq!(c_remove(ct, 2), r_remove(rt, 2));
        assert_eq!(c_size(ct), r_size(rt));

        for id in 1..=5u64 {
            assert_eq!(c_contains(ct, id), r_contains(rt, id), "contains after remove id={}", id);
        }

        c_delete(ct);
        r_delete(rt);
    }
}

#[test]
fn test_tree_remove_root() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type TCreateFn = unsafe extern "C" fn() -> *mut TreeT;
        type TDeleteFn = unsafe extern "C" fn(*mut TreeT);
        type TAddFn = unsafe extern "C" fn(*mut TreeT, TreeIdT, TreeIdT, *const c_char) -> c_int;
        type TRemoveFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> c_int;
        type TSizeFn = unsafe extern "C" fn(*mut TreeT) -> usize;

        let c_create: Symbol<TCreateFn> = load_fn!(c, b"tree_create", TCreateFn);
        let c_delete: Symbol<TDeleteFn> = load_fn!(c, b"tree_delete", TDeleteFn);
        let c_add: Symbol<TAddFn> = load_fn!(c, b"tree_add_node", TAddFn);
        let c_remove: Symbol<TRemoveFn> = load_fn!(c, b"tree_remove_node", TRemoveFn);
        let c_size: Symbol<TSizeFn> = load_fn!(c, b"tree_size", TSizeFn);

        let r_create: Symbol<TCreateFn> = load_fn!(r, b"tree_create", TCreateFn);
        let r_delete: Symbol<TDeleteFn> = load_fn!(r, b"tree_delete", TDeleteFn);
        let r_add: Symbol<TAddFn> = load_fn!(r, b"tree_add_node", TAddFn);
        let r_remove: Symbol<TRemoveFn> = load_fn!(r, b"tree_remove_node", TRemoveFn);
        let r_size: Symbol<TSizeFn> = load_fn!(r, b"tree_size", TSizeFn);

        let ct = c_create();
        let rt = r_create();

        for &(id, pid, data) in &[(1u64, 0u64, "root"), (2, 1, "c1"), (3, 1, "c2")] {
            let cdata = CString::new(data).unwrap();
            c_add(ct, id, pid, cdata.as_ptr());
            r_add(rt, id, pid, cdata.as_ptr());
        }

        assert_eq!(c_remove(ct, 1), r_remove(rt, 1));
        assert_eq!(c_size(ct), r_size(rt));
        assert_eq!(c_size(ct), 0);
        assert_eq!((*ct).has_root, (*rt).has_root);

        c_delete(ct);
        r_delete(rt);
    }
}

#[test]
fn test_tree_duplicate_and_max_children() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type TCreateFn = unsafe extern "C" fn() -> *mut TreeT;
        type TDeleteFn = unsafe extern "C" fn(*mut TreeT);
        type TAddFn = unsafe extern "C" fn(*mut TreeT, TreeIdT, TreeIdT, *const c_char) -> c_int;
        type TSizeFn = unsafe extern "C" fn(*mut TreeT) -> usize;

        let c_create: Symbol<TCreateFn> = load_fn!(c, b"tree_create", TCreateFn);
        let c_delete: Symbol<TDeleteFn> = load_fn!(c, b"tree_delete", TDeleteFn);
        let c_add: Symbol<TAddFn> = load_fn!(c, b"tree_add_node", TAddFn);
        let c_size: Symbol<TSizeFn> = load_fn!(c, b"tree_size", TSizeFn);

        let r_create: Symbol<TCreateFn> = load_fn!(r, b"tree_create", TCreateFn);
        let r_delete: Symbol<TDeleteFn> = load_fn!(r, b"tree_delete", TDeleteFn);
        let r_add: Symbol<TAddFn> = load_fn!(r, b"tree_add_node", TAddFn);
        let r_size: Symbol<TSizeFn> = load_fn!(r, b"tree_size", TSizeFn);

        // Test duplicate ID
        let ct = c_create();
        let rt = r_create();
        let root = CString::new("root").unwrap();
        let child = CString::new("child").unwrap();
        let dup = CString::new("dup").unwrap();

        c_add(ct, 1, 0, root.as_ptr());
        r_add(rt, 1, 0, root.as_ptr());
        c_add(ct, 2, 1, child.as_ptr());
        r_add(rt, 2, 1, child.as_ptr());

        let c_ret = c_add(ct, 2, 1, dup.as_ptr());
        let r_ret = r_add(rt, 2, 1, dup.as_ptr());
        assert_eq!(c_ret, r_ret); // both should fail
        assert_eq!(c_size(ct), r_size(rt));

        c_delete(ct);
        r_delete(rt);

        // Test max children
        let ct = c_create();
        let rt = r_create();
        c_add(ct, 1, 0, root.as_ptr());
        r_add(rt, 1, 0, root.as_ptr());

        for i in 0..MAX_CHILDREN as u64 {
            let c_ret = c_add(ct, i + 2, 1, child.as_ptr());
            let r_ret = r_add(rt, i + 2, 1, child.as_ptr());
            assert_eq!(c_ret, r_ret, "max_children add i={}", i);
        }

        // One more should fail
        let overflow = CString::new("overflow").unwrap();
        let c_ret = c_add(ct, MAX_CHILDREN as u64 + 2, 1, overflow.as_ptr());
        let r_ret = r_add(rt, MAX_CHILDREN as u64 + 2, 1, overflow.as_ptr());
        assert_eq!(c_ret, r_ret);
        assert_eq!(c_size(ct), r_size(rt));

        c_delete(ct);
        r_delete(rt);
    }
}

#[test]
fn test_tree_null_data() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type TCreateFn = unsafe extern "C" fn() -> *mut TreeT;
        type TDeleteFn = unsafe extern "C" fn(*mut TreeT);
        type TAddFn = unsafe extern "C" fn(*mut TreeT, TreeIdT, TreeIdT, *const c_char) -> c_int;
        type TGetNodeFn = unsafe extern "C" fn(*mut TreeT, TreeIdT) -> *mut TreeNodeT;

        let c_create: Symbol<TCreateFn> = load_fn!(c, b"tree_create", TCreateFn);
        let c_delete: Symbol<TDeleteFn> = load_fn!(c, b"tree_delete", TDeleteFn);
        let c_add: Symbol<TAddFn> = load_fn!(c, b"tree_add_node", TAddFn);
        let c_get_node: Symbol<TGetNodeFn> = load_fn!(c, b"tree_get_node", TGetNodeFn);

        let r_create: Symbol<TCreateFn> = load_fn!(r, b"tree_create", TCreateFn);
        let r_delete: Symbol<TDeleteFn> = load_fn!(r, b"tree_delete", TDeleteFn);
        let r_add: Symbol<TAddFn> = load_fn!(r, b"tree_add_node", TAddFn);
        let r_get_node: Symbol<TGetNodeFn> = load_fn!(r, b"tree_get_node", TGetNodeFn);

        let ct = c_create();
        let rt = r_create();

        // Add with null data
        let c_ret = c_add(ct, 1, 0, std::ptr::null());
        let r_ret = r_add(rt, 1, 0, std::ptr::null());
        assert_eq!(c_ret, r_ret);

        let cn = c_get_node(ct, 1);
        let rn = r_get_node(rt, 1);
        assert_eq!((*cn).data[0], (*rn).data[0]); // both should be 0

        c_delete(ct);
        r_delete(rt);
    }
}
