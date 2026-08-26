use libloading::Library;
use std::ffi::{c_char, c_int, c_long, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

const MAX_CHILDREN: usize = 32;
const MAX_DATA_LENGTH: usize = 256;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
struct HashmapEntry {
    key: u64,
    value: *mut c_void,
    occupied: c_int,
    deleted: c_int,
}

#[repr(C)]
struct Hashmap {
    entries: *mut HashmapEntry,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

#[repr(C)]
struct TreeNode {
    id: u64,
    parent_id: u64,
    child_ids: [u64; MAX_CHILDREN],
    child_count: c_int,
    data: [c_char; MAX_DATA_LENGTH],
}

#[repr(C)]
struct Tree {
    node_map: *mut Hashmap,
    root_id: u64,
    has_root: c_int,
    node_count: usize,
}

type HashmapCreate = unsafe extern "C" fn() -> *mut Hashmap;
type HashmapDestroy = unsafe extern "C" fn(*mut Hashmap);
type HashmapPut = unsafe extern "C" fn(*mut Hashmap, u64, *mut c_void) -> c_int;
type HashmapGet = unsafe extern "C" fn(*mut Hashmap, u64) -> *mut c_void;
type HashmapRemove = unsafe extern "C" fn(*mut Hashmap, u64) -> *mut c_void;
type HashmapContains = unsafe extern "C" fn(*mut Hashmap, u64) -> c_int;
type HashmapSize = unsafe extern "C" fn(*mut Hashmap) -> usize;
type HashmapClear = unsafe extern "C" fn(*mut Hashmap);

type TreeCreate = unsafe extern "C" fn() -> *mut Tree;
type TreeDelete = unsafe extern "C" fn(*mut Tree);
type TreeAddNode = unsafe extern "C" fn(*mut Tree, u64, u64, *const c_char) -> c_int;
type TreeRemoveNode = unsafe extern "C" fn(*mut Tree, u64) -> c_int;
type TreeGetNode = unsafe extern "C" fn(*mut Tree, u64) -> *mut TreeNode;
type TreeContains = unsafe extern "C" fn(*mut Tree, u64) -> c_int;
type TreeSize = unsafe extern "C" fn(*mut Tree) -> usize;
type TreePrint = unsafe extern "C" fn(*mut Tree);
type TreeGetDepth = unsafe extern "C" fn(*mut Tree, u64) -> c_int;
type TreeGetHeight = unsafe extern "C" fn(*mut Tree, u64) -> c_int;
type TreeCountDescendants = unsafe extern "C" fn(*mut Tree, u64) -> c_int;
type TreeFindPath = unsafe extern "C" fn(*mut Tree, u64, *mut u64, c_int) -> c_int;

struct Api {
    hashmap_create: HashmapCreate,
    hashmap_destroy: HashmapDestroy,
    hashmap_put: HashmapPut,
    hashmap_get: HashmapGet,
    hashmap_remove: HashmapRemove,
    hashmap_contains: HashmapContains,
    hashmap_size: HashmapSize,
    hashmap_clear: HashmapClear,
    tree_create: TreeCreate,
    tree_delete: TreeDelete,
    tree_add_node: TreeAddNode,
    tree_remove_node: TreeRemoveNode,
    tree_get_node: TreeGetNode,
    tree_contains: TreeContains,
    tree_size: TreeSize,
    tree_print: TreePrint,
    tree_get_depth: TreeGetDepth,
    tree_get_height: TreeGetHeight,
    tree_count_descendants: TreeCountDescendants,
    tree_find_path: TreeFindPath,
    _library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = Library::new(path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $type:ty) => {
                *library
                    .get::<$type>(concat!($name, "\0").as_bytes())
                    .unwrap()
            };
        }
        Self {
            hashmap_create: symbol!("hashmap_create", HashmapCreate),
            hashmap_destroy: symbol!("hashmap_destroy", HashmapDestroy),
            hashmap_put: symbol!("hashmap_put", HashmapPut),
            hashmap_get: symbol!("hashmap_get", HashmapGet),
            hashmap_remove: symbol!("hashmap_remove", HashmapRemove),
            hashmap_contains: symbol!("hashmap_contains", HashmapContains),
            hashmap_size: symbol!("hashmap_size", HashmapSize),
            hashmap_clear: symbol!("hashmap_clear", HashmapClear),
            tree_create: symbol!("tree_create", TreeCreate),
            tree_delete: symbol!("tree_delete", TreeDelete),
            tree_add_node: symbol!("tree_add_node", TreeAddNode),
            tree_remove_node: symbol!("tree_remove_node", TreeRemoveNode),
            tree_get_node: symbol!("tree_get_node", TreeGetNode),
            tree_contains: symbol!("tree_contains", TreeContains),
            tree_size: symbol!("tree_size", TreeSize),
            tree_print: symbol!("tree_print", TreePrint),
            tree_get_depth: symbol!("tree_get_depth", TreeGetDepth),
            tree_get_height: symbol!("tree_get_height", TreeGetHeight),
            tree_count_descendants: symbol!("tree_count_descendants", TreeCountDescendants),
            tree_find_path: symbol!("tree_find_path", TreeFindPath),
            _library: library,
        }
    }
}

struct ApiPair {
    c: Api,
    rust: Api,
}

impl ApiPair {
    unsafe fn load() -> Self {
        Self {
            c: Api::load(&c_library_path()),
            rust: Api::load(&rust_library_path()),
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("c_src/build/libdriver_c.so"))
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target/debug/libdriver.so"))
}

fn c_string(bytes: &[u8]) -> Vec<c_char> {
    bytes
        .iter()
        .copied()
        .chain(std::iter::once(0))
        .map(|byte| byte as c_char)
        .collect()
}

fn hash(key: u64) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in key.to_ne_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn below(&mut self, limit: usize) -> usize {
        (self.next() as usize) % limit
    }
}

unsafe fn assert_map_layout_equal(c: *mut Hashmap, rust: *mut Hashmap) {
    assert_eq!((*c).capacity, (*rust).capacity);
    assert_eq!((*c).size, (*rust).size);
    assert_eq!((*c).deleted_count, (*rust).deleted_count);
    for index in 0..(*c).capacity {
        let left = &*(*c).entries.add(index);
        let right = &*(*rust).entries.add(index);
        assert_eq!(left.key, right.key, "key mismatch in slot {index}");
        assert_eq!(
            left.occupied, right.occupied,
            "occupied mismatch in slot {index}"
        );
        assert_eq!(
            left.deleted, right.deleted,
            "deleted mismatch in slot {index}"
        );
    }
}

unsafe fn pointed_i64(pointer: *mut c_void) -> Option<i64> {
    (!pointer.is_null()).then(|| *pointer.cast::<i64>())
}

unsafe fn assert_hashmap_queries_equal(
    pair: &ApiPair,
    c_map: *mut Hashmap,
    rust_map: *mut Hashmap,
    keys: impl IntoIterator<Item = u64>,
) {
    assert_map_layout_equal(c_map, rust_map);
    assert_eq!(
        (pair.c.hashmap_size)(c_map),
        (pair.rust.hashmap_size)(rust_map)
    );
    for key in keys {
        let c_value = (pair.c.hashmap_get)(c_map, key);
        let rust_value = (pair.rust.hashmap_get)(rust_map, key);
        assert_eq!(pointed_i64(c_value), pointed_i64(rust_value), "key {key}");
        assert_eq!(
            (pair.c.hashmap_contains)(c_map, key),
            (pair.rust.hashmap_contains)(rust_map, key),
            "contains key {key}"
        );
    }
}

unsafe fn assert_node_equal(c: *mut TreeNode, rust: *mut TreeNode) {
    assert_eq!(c.is_null(), rust.is_null());
    if c.is_null() {
        return;
    }
    assert_eq!((*c).id, (*rust).id);
    assert_eq!((*c).parent_id, (*rust).parent_id);
    assert_eq!((*c).child_count, (*rust).child_count);
    for index in 0..(*c).child_count as usize {
        assert_eq!((*c).child_ids[index], (*rust).child_ids[index]);
    }
    let c_data = std::slice::from_raw_parts((*c).data.as_ptr().cast::<u8>(), 256);
    let rust_data = std::slice::from_raw_parts((*rust).data.as_ptr().cast::<u8>(), 256);
    let observable_length = c_data
        .iter()
        .position(|byte| *byte == 0)
        .map_or(256, |position| position + 1);
    assert_eq!(
        &c_data[..observable_length],
        &rust_data[..observable_length]
    );
}

unsafe fn assert_tree_equal(pair: &ApiPair, c_tree: *mut Tree, rust_tree: *mut Tree) {
    assert_eq!((*c_tree).root_id, (*rust_tree).root_id);
    assert_eq!((*c_tree).has_root, (*rust_tree).has_root);
    assert_eq!((*c_tree).node_count, (*rust_tree).node_count);
    assert_map_layout_equal((*c_tree).node_map, (*rust_tree).node_map);
    for index in 0..(*(*c_tree).node_map).capacity {
        let c_entry = &*(*(*c_tree).node_map).entries.add(index);
        if c_entry.occupied != 0 && c_entry.deleted == 0 {
            let id = c_entry.key;
            assert_node_equal(
                (pair.c.tree_get_node)(c_tree, id),
                (pair.rust.tree_get_node)(rust_tree, id),
            );
        }
    }
}

unsafe fn add_node_pair(
    pair: &ApiPair,
    c_tree: *mut Tree,
    rust_tree: *mut Tree,
    id: u64,
    parent: u64,
    data: Option<&[u8]>,
) -> (c_int, c_int) {
    let storage = data.map(c_string);
    let pointer = storage.as_ref().map_or(ptr::null(), |value| value.as_ptr());
    (
        (pair.c.tree_add_node)(c_tree, id, parent, pointer),
        (pair.rust.tree_add_node)(rust_tree, id, parent, pointer),
    )
}

extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

unsafe fn capture_stdout(action: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut fds = [0; 2];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0);
    fflush(ptr::null_mut());
    assert_eq!(dup2(fds[1], 1), 1);
    close(fds[1]);

    action();

    fflush(ptr::null_mut());
    assert_eq!(dup2(saved_stdout, 1), 1);
    close(saved_stdout);
    let mut output = Vec::new();
    File::from_raw_fd(fds[0]).read_to_end(&mut output).unwrap();
    output
}

#[test]
fn hashmap_valid_surface_randomized() {
    unsafe {
        let pair = ApiPair::load();
        let c_map = (pair.c.hashmap_create)();
        let rust_map = (pair.rust.hashmap_create)();
        assert!(!c_map.is_null() && !rust_map.is_null());
        assert_hashmap_queries_equal(&pair, c_map, rust_map, 0..20);
        assert_eq!((*c_map).capacity, 16);

        let mut collision_buckets = vec![Vec::new(); 16];
        for key in 0..10_000_u64 {
            collision_buckets[(hash(key) % 16) as usize].push(key);
        }
        let collisions = collision_buckets
            .iter()
            .find(|bucket| bucket.len() >= 8)
            .unwrap();

        let mut c_values = Vec::<Box<i64>>::new();
        let mut rust_values = Vec::<Box<i64>>::new();
        for (index, key) in collisions.iter().copied().take(8).enumerate() {
            c_values.push(Box::new(index as i64 * 17 - 9));
            rust_values.push(Box::new(index as i64 * 17 - 9));
            assert_eq!(
                (pair.c.hashmap_put)(
                    c_map,
                    key,
                    (&mut *c_values[index]) as *mut i64 as *mut c_void,
                ),
                (pair.rust.hashmap_put)(
                    rust_map,
                    key,
                    (&mut *rust_values[index]) as *mut i64 as *mut c_void,
                )
            );
            assert_hashmap_queries_equal(
                &pair,
                c_map,
                rust_map,
                collisions.iter().copied().take(index + 1),
            );
        }

        let update_key = collisions[3];
        c_values.push(Box::new(77_777));
        rust_values.push(Box::new(77_777));
        let update_index = c_values.len() - 1;
        let old_size = (*c_map).size;
        assert_eq!(
            (pair.c.hashmap_put)(
                c_map,
                update_key,
                (&mut *c_values[update_index]) as *mut i64 as *mut c_void,
            ),
            (pair.rust.hashmap_put)(
                rust_map,
                update_key,
                (&mut *rust_values[update_index]) as *mut i64 as *mut c_void,
            )
        );
        assert_eq!((*c_map).size, old_size);
        assert_hashmap_queries_equal(&pair, c_map, rust_map, [update_key]);

        let removed_key = collisions[1];
        assert_eq!(
            pointed_i64((pair.c.hashmap_remove)(c_map, removed_key)),
            pointed_i64((pair.rust.hashmap_remove)(rust_map, removed_key))
        );
        assert_hashmap_queries_equal(&pair, c_map, rust_map, collisions.iter().copied().take(8));

        let replacement_key = collision_buckets[(hash(removed_key) % 16) as usize][20];
        c_values.push(Box::new(-444));
        rust_values.push(Box::new(-444));
        let replacement_index = c_values.len() - 1;
        assert_eq!(
            (pair.c.hashmap_put)(
                c_map,
                replacement_key,
                (&mut *c_values[replacement_index]) as *mut i64 as *mut c_void,
            ),
            (pair.rust.hashmap_put)(
                rust_map,
                replacement_key,
                (&mut *rust_values[replacement_index]) as *mut i64 as *mut c_void,
            )
        );
        assert_hashmap_queries_equal(
            &pair,
            c_map,
            rust_map,
            collisions.iter().copied().take(8).chain([replacement_key]),
        );

        let null_key = u64::MAX;
        assert_eq!(
            (pair.c.hashmap_put)(c_map, null_key, ptr::null_mut()),
            (pair.rust.hashmap_put)(rust_map, null_key, ptr::null_mut())
        );
        assert_eq!((pair.c.hashmap_contains)(c_map, null_key), 0);
        assert_eq!((pair.rust.hashmap_contains)(rust_map, null_key), 0);
        assert!((pair.c.hashmap_get)(c_map, null_key).is_null());
        assert!((pair.rust.hashmap_get)(rust_map, null_key).is_null());
        assert_eq!(
            (pair.c.hashmap_remove)(c_map, null_key).is_null(),
            (pair.rust.hashmap_remove)(rust_map, null_key).is_null()
        );

        (pair.c.hashmap_clear)(c_map);
        (pair.rust.hashmap_clear)(rust_map);
        assert_hashmap_queries_equal(&pair, c_map, rust_map, 0..100);
        assert_eq!((*c_map).size, 0);
        assert_eq!((*c_map).deleted_count, 0);
        (pair.c.hashmap_destroy)(c_map);
        (pair.rust.hashmap_destroy)(rust_map);

        for deleted_resize in [false, true] {
            let c_map = (pair.c.hashmap_create)();
            let rust_map = (pair.rust.hashmap_create)();
            let mut left = Vec::<Box<i64>>::new();
            let mut right = Vec::<Box<i64>>::new();
            for key in 0..13_u64 {
                left.push(Box::new(key as i64));
                right.push(Box::new(key as i64));
                let index = left.len() - 1;
                assert_eq!(
                    (pair.c.hashmap_put)(
                        c_map,
                        key,
                        (&mut *left[index]) as *mut i64 as *mut c_void,
                    ),
                    (pair.rust.hashmap_put)(
                        rust_map,
                        key,
                        (&mut *right[index]) as *mut i64 as *mut c_void,
                    )
                );
            }
            if deleted_resize {
                assert_eq!(
                    pointed_i64((pair.c.hashmap_remove)(c_map, 4)),
                    pointed_i64((pair.rust.hashmap_remove)(rust_map, 4))
                );
            }
            left.push(Box::new(13));
            right.push(Box::new(13));
            assert_eq!(
                (pair.c.hashmap_put)(
                    c_map,
                    13,
                    (&mut **left.last_mut().unwrap()) as *mut i64 as *mut c_void,
                ),
                (pair.rust.hashmap_put)(
                    rust_map,
                    13,
                    (&mut **right.last_mut().unwrap()) as *mut i64 as *mut c_void,
                )
            );
            assert_eq!((*c_map).capacity, 32);
            assert_hashmap_queries_equal(&pair, c_map, rust_map, 0..14);
            (pair.c.hashmap_destroy)(c_map);
            (pair.rust.hashmap_destroy)(rust_map);
        }

        let c_map = (pair.c.hashmap_create)();
        let rust_map = (pair.rust.hashmap_create)();
        let mut left_values = Vec::<Box<i64>>::new();
        let mut right_values = Vec::<Box<i64>>::new();
        let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);
        for iteration in 0..1_500 {
            let operation = rng.below(100);
            let key = if iteration % 97 == 0 {
                rng.next()
            } else {
                rng.below(96) as u64
            };
            match operation {
                0..=49 => {
                    let value = rng.next() as i64;
                    left_values.push(Box::new(value));
                    right_values.push(Box::new(value));
                    let index = left_values.len() - 1;
                    assert_eq!(
                        (pair.c.hashmap_put)(
                            c_map,
                            key,
                            (&mut *left_values[index]) as *mut i64 as *mut c_void,
                        ),
                        (pair.rust.hashmap_put)(
                            rust_map,
                            key,
                            (&mut *right_values[index]) as *mut i64 as *mut c_void,
                        )
                    );
                }
                50..=74 => {
                    assert_eq!(
                        pointed_i64((pair.c.hashmap_remove)(c_map, key)),
                        pointed_i64((pair.rust.hashmap_remove)(rust_map, key))
                    );
                }
                75..=94 => {
                    assert_eq!(
                        pointed_i64((pair.c.hashmap_get)(c_map, key)),
                        pointed_i64((pair.rust.hashmap_get)(rust_map, key))
                    );
                }
                _ => {
                    (pair.c.hashmap_clear)(c_map);
                    (pair.rust.hashmap_clear)(rust_map);
                }
            }
            assert_hashmap_queries_equal(&pair, c_map, rust_map, 0..96);
        }
        (pair.c.hashmap_destroy)(c_map);
        (pair.rust.hashmap_destroy)(rust_map);
    }
}

#[test]
fn tree_valid_surface_randomized() {
    unsafe {
        let pair = ApiPair::load();

        let c_tree = (pair.c.tree_create)();
        let rust_tree = (pair.rust.tree_create)();
        assert!(!c_tree.is_null() && !rust_tree.is_null());
        assert_tree_equal(&pair, c_tree, rust_tree);
        assert_eq!((pair.c.tree_size)(c_tree), 0);
        assert_eq!((pair.rust.tree_size)(rust_tree), 0);

        assert_eq!(
            add_node_pair(&pair, c_tree, rust_tree, 0, u64::MAX, None),
            (0, 0)
        );
        assert_tree_equal(&pair, c_tree, rust_tree);
        assert_eq!((*c_tree).root_id, 0);
        assert_eq!((*(pair.c.tree_get_node)(c_tree, 0)).parent_id, 0);

        let data_shapes = [
            Vec::new(),
            b"short".to_vec(),
            vec![b'a'; 255],
            vec![b'b'; 256],
            vec![b'c'; 400],
        ];
        for (index, data) in data_shapes.iter().enumerate() {
            let id = if index == 4 {
                u64::MAX
            } else {
                index as u64 + 1
            };
            assert_eq!(
                add_node_pair(&pair, c_tree, rust_tree, id, 0, Some(data)),
                (0, 0)
            );
            assert_node_equal(
                (pair.c.tree_get_node)(c_tree, id),
                (pair.rust.tree_get_node)(rust_tree, id),
            );
        }
        assert_tree_equal(&pair, c_tree, rust_tree);
        assert_eq!((pair.c.tree_get_depth)(c_tree, 0), 0);
        assert_eq!((pair.rust.tree_get_depth)(rust_tree, 0), 0);
        assert_eq!((pair.c.tree_get_height)(c_tree, 1), 0);
        assert_eq!((pair.rust.tree_get_height)(rust_tree, 1), 0);
        assert_eq!((pair.c.tree_count_descendants)(c_tree, 1), 0);
        assert_eq!((pair.rust.tree_count_descendants)(rust_tree, 1), 0);
        (pair.c.tree_delete)(c_tree);
        (pair.rust.tree_delete)(rust_tree);

        let c_tree = (pair.c.tree_create)();
        let rust_tree = (pair.rust.tree_create)();
        assert_eq!(
            add_node_pair(&pair, c_tree, rust_tree, 1, 999, Some(b"root")),
            (0, 0)
        );
        for id in 2..=33_u64 {
            assert_eq!(
                add_node_pair(&pair, c_tree, rust_tree, id, 1, Some(b"child")),
                (0, 0)
            );
        }
        assert_eq!((*(pair.c.tree_get_node)(c_tree, 1)).child_count, 32);
        assert_tree_equal(&pair, c_tree, rust_tree);
        (pair.c.tree_delete)(c_tree);
        (pair.rust.tree_delete)(rust_tree);

        for remove_index in [0_usize, 2, 4] {
            let c_tree = (pair.c.tree_create)();
            let rust_tree = (pair.rust.tree_create)();
            assert_eq!(
                add_node_pair(&pair, c_tree, rust_tree, 100, 0, Some(b"root")),
                (0, 0)
            );
            let children = [101_u64, 102, 103, 104, 105];
            for id in children {
                assert_eq!(
                    add_node_pair(&pair, c_tree, rust_tree, id, 100, Some(b"leaf")),
                    (0, 0)
                );
            }
            assert_eq!(
                (pair.c.tree_remove_node)(c_tree, children[remove_index]),
                (pair.rust.tree_remove_node)(rust_tree, children[remove_index])
            );
            assert_tree_equal(&pair, c_tree, rust_tree);
            let root_c = (pair.c.tree_get_node)(c_tree, 100);
            let expected: Vec<_> = children
                .iter()
                .copied()
                .filter(|id| *id != children[remove_index])
                .collect();
            assert_eq!(
                &(&(*root_c).child_ids)[..(*root_c).child_count as usize],
                expected.as_slice()
            );
            (pair.c.tree_delete)(c_tree);
            (pair.rust.tree_delete)(rust_tree);
        }

        let c_tree = (pair.c.tree_create)();
        let rust_tree = (pair.rust.tree_create)();
        for (id, parent) in [
            (1, 0),
            (2, 1),
            (3, 1),
            (4, 2),
            (5, 2),
            (6, 4),
            (7, 3),
            (8, 7),
        ] {
            let data = format!("node-{id}");
            assert_eq!(
                add_node_pair(&pair, c_tree, rust_tree, id, parent, Some(data.as_bytes())),
                (0, 0)
            );
        }
        for id in 1..=8 {
            assert_eq!(
                (pair.c.tree_contains)(c_tree, id),
                (pair.rust.tree_contains)(rust_tree, id)
            );
            assert_eq!(
                (pair.c.tree_get_depth)(c_tree, id),
                (pair.rust.tree_get_depth)(rust_tree, id)
            );
            assert_eq!(
                (pair.c.tree_get_height)(c_tree, id),
                (pair.rust.tree_get_height)(rust_tree, id)
            );
            assert_eq!(
                (pair.c.tree_count_descendants)(c_tree, id),
                (pair.rust.tree_count_descendants)(rust_tree, id)
            );
            for max_length in [-3, 0, 1, 2, 3, 20] {
                let mut c_path = [u64::MAX; 20];
                let mut rust_path = [u64::MAX; 20];
                assert_eq!(
                    (pair.c.tree_find_path)(c_tree, id, c_path.as_mut_ptr(), max_length),
                    (pair.rust.tree_find_path)(rust_tree, id, rust_path.as_mut_ptr(), max_length)
                );
                assert_eq!(c_path, rust_path);
            }
        }
        assert_eq!(
            (pair.c.tree_remove_node)(c_tree, 2),
            (pair.rust.tree_remove_node)(rust_tree, 2)
        );
        assert_tree_equal(&pair, c_tree, rust_tree);
        for id in [2, 4, 5, 6] {
            assert_eq!((pair.c.tree_contains)(c_tree, id), 0);
            assert_eq!((pair.rust.tree_contains)(rust_tree, id), 0);
        }
        assert_eq!(
            (pair.c.tree_remove_node)(c_tree, 1),
            (pair.rust.tree_remove_node)(rust_tree, 1)
        );
        assert_tree_equal(&pair, c_tree, rust_tree);
        assert_eq!((*c_tree).has_root, 0);
        assert_eq!((*c_tree).root_id, 0);
        assert_eq!((*c_tree).node_count, 0);
        (pair.c.tree_delete)(c_tree);
        (pair.rust.tree_delete)(rust_tree);

        let mut rng = Rng::new(0xd1ff_e2e3_a4b5_c607);
        for case in 0..40_u64 {
            let c_tree = (pair.c.tree_create)();
            let rust_tree = (pair.rust.tree_create)();
            let node_count = 20 + rng.below(45);
            let mut ids = Vec::with_capacity(node_count);
            let mut child_counts = Vec::with_capacity(node_count);
            for index in 0..node_count {
                let mut id = rng.next() ^ (case << 48) ^ index as u64;
                while ids.contains(&id) {
                    id = rng.next();
                }
                let parent = if index == 0 {
                    rng.next()
                } else {
                    let candidates: Vec<_> = child_counts
                        .iter()
                        .enumerate()
                        .filter_map(|(position, count)| (*count < 32).then_some(position))
                        .collect();
                    let position = candidates[rng.below(candidates.len())];
                    child_counts[position] += 1;
                    ids[position]
                };
                let data_len = rng.below(330);
                let data: Vec<_> = (0..data_len)
                    .map(|_| (b'!' + rng.below(90) as u8) as u8)
                    .collect();
                assert_eq!(
                    add_node_pair(&pair, c_tree, rust_tree, id, parent, Some(&data)),
                    (0, 0)
                );
                ids.push(id);
                child_counts.push(0_usize);
                assert_tree_equal(&pair, c_tree, rust_tree);
            }

            for _ in 0..150 {
                let id = if rng.below(5) == 0 {
                    rng.next()
                } else {
                    ids[rng.below(ids.len())]
                };
                assert_node_equal(
                    (pair.c.tree_get_node)(c_tree, id),
                    (pair.rust.tree_get_node)(rust_tree, id),
                );
                assert_eq!(
                    (pair.c.tree_contains)(c_tree, id),
                    (pair.rust.tree_contains)(rust_tree, id)
                );
                assert_eq!(
                    (pair.c.tree_get_depth)(c_tree, id),
                    (pair.rust.tree_get_depth)(rust_tree, id)
                );
                assert_eq!(
                    (pair.c.tree_get_height)(c_tree, id),
                    (pair.rust.tree_get_height)(rust_tree, id)
                );
                assert_eq!(
                    (pair.c.tree_count_descendants)(c_tree, id),
                    (pair.rust.tree_count_descendants)(rust_tree, id)
                );
                let max_length = rng.below(node_count + 3) as c_int;
                let mut c_path = vec![u64::MAX; node_count + 3];
                let mut rust_path = vec![u64::MAX; node_count + 3];
                assert_eq!(
                    (pair.c.tree_find_path)(c_tree, id, c_path.as_mut_ptr(), max_length),
                    (pair.rust.tree_find_path)(rust_tree, id, rust_path.as_mut_ptr(), max_length)
                );
                assert_eq!(c_path, rust_path);
            }

            if node_count > 1 {
                let remove_id = ids[1 + rng.below(node_count - 1)];
                assert_eq!(
                    (pair.c.tree_remove_node)(c_tree, remove_id),
                    (pair.rust.tree_remove_node)(rust_tree, remove_id)
                );
                assert_tree_equal(&pair, c_tree, rust_tree);
            }
            (pair.c.tree_delete)(c_tree);
            (pair.rust.tree_delete)(rust_tree);
        }
    }
}

#[test]
fn tree_find_path_thousand_element_boundary() {
    unsafe {
        let pair = ApiPair::load();
        let c_tree = (pair.c.tree_create)();
        let rust_tree = (pair.rust.tree_create)();
        for id in 1..=1_001_u64 {
            let parent = id.saturating_sub(1);
            assert_eq!(
                add_node_pair(&pair, c_tree, rust_tree, id, parent, Some(b"x")),
                (0, 0)
            );
        }
        let mut c_path = [u64::MAX; 1_005];
        let mut rust_path = [u64::MAX; 1_005];
        assert_eq!(
            (pair.c.tree_find_path)(c_tree, 1_001, c_path.as_mut_ptr(), 1_005),
            1_000
        );
        assert_eq!(
            (pair.rust.tree_find_path)(rust_tree, 1_001, rust_path.as_mut_ptr(), 1_005),
            1_000
        );
        assert_eq!(c_path, rust_path);
        assert_eq!(c_path[0], 2);
        assert_eq!(c_path[999], 1_001);
        (pair.c.tree_delete)(c_tree);
        (pair.rust.tree_delete)(rust_tree);
    }
}

#[test]
fn tree_print_bytes_match() {
    unsafe {
        let pair = ApiPair::load();
        let c_empty = (pair.c.tree_create)();
        let rust_empty = (pair.rust.tree_create)();
        let c_output = capture_stdout(|| (pair.c.tree_print)(c_empty));
        let rust_output = capture_stdout(|| (pair.rust.tree_print)(rust_empty));
        assert_eq!(c_output, b"(empty tree)\n");
        assert_eq!(c_output, rust_output);
        (pair.c.tree_delete)(c_empty);
        (pair.rust.tree_delete)(rust_empty);

        let c_tree = (pair.c.tree_create)();
        let rust_tree = (pair.rust.tree_create)();
        for (id, parent, data) in [
            (10, 0, &b"root"[..]),
            (20, 10, &b"left"[..]),
            (30, 10, &b"right"[..]),
            (40, 20, &b"leaf"[..]),
        ] {
            assert_eq!(
                add_node_pair(&pair, c_tree, rust_tree, id, parent, Some(data)),
                (0, 0)
            );
        }
        let c_output = capture_stdout(|| (pair.c.tree_print)(c_tree));
        let rust_output = capture_stdout(|| (pair.rust.tree_print)(rust_tree));
        assert_eq!(
            c_output,
            b"[10] root\n  [20] left\n    [40] leaf\n  [30] right\n"
        );
        assert_eq!(c_output, rust_output);
        (pair.c.tree_delete)(c_tree);
        (pair.rust.tree_delete)(rust_tree);
    }
}

#[test]
fn non_allocation_error_surface_matches() {
    unsafe {
        let pair = ApiPair::load();

        (pair.c.hashmap_destroy)(ptr::null_mut());
        (pair.rust.hashmap_destroy)(ptr::null_mut());
        assert_eq!(
            (pair.c.hashmap_put)(ptr::null_mut(), 1, ptr::null_mut()),
            -1
        );
        assert_eq!(
            (pair.rust.hashmap_put)(ptr::null_mut(), 1, ptr::null_mut()),
            -1
        );
        assert!((pair.c.hashmap_get)(ptr::null_mut(), 1).is_null());
        assert!((pair.rust.hashmap_get)(ptr::null_mut(), 1).is_null());
        assert!((pair.c.hashmap_remove)(ptr::null_mut(), 1).is_null());
        assert!((pair.rust.hashmap_remove)(ptr::null_mut(), 1).is_null());
        assert_eq!((pair.c.hashmap_contains)(ptr::null_mut(), 1), 0);
        assert_eq!((pair.rust.hashmap_contains)(ptr::null_mut(), 1), 0);
        assert_eq!((pair.c.hashmap_size)(ptr::null_mut()), 0);
        assert_eq!((pair.rust.hashmap_size)(ptr::null_mut()), 0);
        (pair.c.hashmap_clear)(ptr::null_mut());
        (pair.rust.hashmap_clear)(ptr::null_mut());

        let c_map = (pair.c.hashmap_create)();
        let rust_map = (pair.rust.hashmap_create)();
        assert!((pair.c.hashmap_get)(c_map, 9).is_null());
        assert!((pair.rust.hashmap_get)(rust_map, 9).is_null());
        assert!((pair.c.hashmap_remove)(c_map, 9).is_null());
        assert!((pair.rust.hashmap_remove)(rust_map, 9).is_null());
        (pair.c.hashmap_destroy)(c_map);
        (pair.rust.hashmap_destroy)(rust_map);

        let mut c_entries: [HashmapEntry; 4] = std::array::from_fn(|index| HashmapEntry {
            key: index as u64,
            value: ptr::null_mut(),
            occupied: 1,
            deleted: 0,
        });
        let mut rust_entries: [HashmapEntry; 4] = std::array::from_fn(|index| HashmapEntry {
            key: index as u64,
            value: ptr::null_mut(),
            occupied: 1,
            deleted: 0,
        });
        let mut c_full = Hashmap {
            entries: c_entries.as_mut_ptr(),
            capacity: 4,
            size: 0,
            deleted_count: 0,
        };
        let mut rust_full = Hashmap {
            entries: rust_entries.as_mut_ptr(),
            capacity: 4,
            size: 0,
            deleted_count: 0,
        };
        assert_eq!((pair.c.hashmap_put)(&mut c_full, 99, ptr::null_mut()), -1);
        assert_eq!(
            (pair.rust.hashmap_put)(&mut rust_full, 99, ptr::null_mut()),
            -1
        );
        assert!((pair.c.hashmap_get)(&mut c_full, 99).is_null());
        assert!((pair.rust.hashmap_get)(&mut rust_full, 99).is_null());
        assert!((pair.c.hashmap_remove)(&mut c_full, 99).is_null());
        assert!((pair.rust.hashmap_remove)(&mut rust_full, 99).is_null());

        (pair.c.tree_delete)(ptr::null_mut());
        (pair.rust.tree_delete)(ptr::null_mut());
        assert_eq!(
            (pair.c.tree_add_node)(ptr::null_mut(), 1, 0, ptr::null()),
            -1
        );
        assert_eq!(
            (pair.rust.tree_add_node)(ptr::null_mut(), 1, 0, ptr::null()),
            -1
        );
        assert_eq!((pair.c.tree_remove_node)(ptr::null_mut(), 1), -1);
        assert_eq!((pair.rust.tree_remove_node)(ptr::null_mut(), 1), -1);
        assert!((pair.c.tree_get_node)(ptr::null_mut(), 1).is_null());
        assert!((pair.rust.tree_get_node)(ptr::null_mut(), 1).is_null());
        assert_eq!((pair.c.tree_contains)(ptr::null_mut(), 1), 0);
        assert_eq!((pair.rust.tree_contains)(ptr::null_mut(), 1), 0);
        assert_eq!((pair.c.tree_size)(ptr::null_mut()), 0);
        assert_eq!((pair.rust.tree_size)(ptr::null_mut()), 0);
        assert_eq!((pair.c.tree_get_depth)(ptr::null_mut(), 1), -1);
        assert_eq!((pair.rust.tree_get_depth)(ptr::null_mut(), 1), -1);
        assert_eq!((pair.c.tree_get_height)(ptr::null_mut(), 1), -1);
        assert_eq!((pair.rust.tree_get_height)(ptr::null_mut(), 1), -1);
        assert_eq!((pair.c.tree_count_descendants)(ptr::null_mut(), 1), -1);
        assert_eq!((pair.rust.tree_count_descendants)(ptr::null_mut(), 1), -1);
        let mut path = [0_u64; 4];
        assert_eq!(
            (pair.c.tree_find_path)(ptr::null_mut(), 1, path.as_mut_ptr(), path.len() as c_int),
            -1
        );
        assert_eq!(
            (pair.rust.tree_find_path)(ptr::null_mut(), 1, path.as_mut_ptr(), path.len() as c_int),
            -1
        );
        let c_null_print = capture_stdout(|| (pair.c.tree_print)(ptr::null_mut()));
        let rust_null_print = capture_stdout(|| (pair.rust.tree_print)(ptr::null_mut()));
        assert_eq!(c_null_print, b"(empty tree)\n");
        assert_eq!(c_null_print, rust_null_print);

        let c_tree = (pair.c.tree_create)();
        let rust_tree = (pair.rust.tree_create)();
        assert_eq!(
            add_node_pair(&pair, c_tree, rust_tree, 1, 0, Some(b"root")),
            (0, 0)
        );
        assert_eq!(
            add_node_pair(&pair, c_tree, rust_tree, 1, 0, Some(b"duplicate")),
            (-1, -1)
        );
        assert_eq!(
            add_node_pair(&pair, c_tree, rust_tree, 2, 999, Some(b"orphan")),
            (-1, -1)
        );
        assert_eq!((pair.c.tree_remove_node)(c_tree, 999), -1);
        assert_eq!((pair.rust.tree_remove_node)(rust_tree, 999), -1);
        assert!((pair.c.tree_get_node)(c_tree, 999).is_null());
        assert!((pair.rust.tree_get_node)(rust_tree, 999).is_null());
        assert_eq!((pair.c.tree_get_depth)(c_tree, 999), -1);
        assert_eq!((pair.rust.tree_get_depth)(rust_tree, 999), -1);
        assert_eq!((pair.c.tree_get_height)(c_tree, 999), -1);
        assert_eq!((pair.rust.tree_get_height)(rust_tree, 999), -1);
        assert_eq!((pair.c.tree_count_descendants)(c_tree, 999), -1);
        assert_eq!((pair.rust.tree_count_descendants)(rust_tree, 999), -1);
        assert_eq!(
            (pair.c.tree_find_path)(c_tree, 999, path.as_mut_ptr(), 4),
            -1
        );
        assert_eq!(
            (pair.rust.tree_find_path)(rust_tree, 999, path.as_mut_ptr(), 4),
            -1
        );
        assert_eq!((pair.c.tree_find_path)(c_tree, 1, ptr::null_mut(), 0), -1);
        assert_eq!(
            (pair.rust.tree_find_path)(rust_tree, 1, ptr::null_mut(), 0),
            -1
        );
        for id in 2..=33_u64 {
            assert_eq!(
                add_node_pair(&pair, c_tree, rust_tree, id, 1, Some(b"child")),
                (0, 0)
            );
        }
        assert_eq!(
            add_node_pair(&pair, c_tree, rust_tree, 34, 1, Some(b"overflow")),
            (-1, -1)
        );
        assert_tree_equal(&pair, c_tree, rust_tree);

        let c_child = (pair.c.tree_get_node)(c_tree, 2);
        let rust_child = (pair.rust.tree_get_node)(rust_tree, 2);
        (*c_child).parent_id = 9_999;
        (*rust_child).parent_id = 9_999;
        assert_eq!((pair.c.tree_get_depth)(c_tree, 2), -1);
        assert_eq!((pair.rust.tree_get_depth)(rust_tree, 2), -1);
        assert_eq!((pair.c.tree_find_path)(c_tree, 2, path.as_mut_ptr(), 4), -1);
        assert_eq!(
            (pair.rust.tree_find_path)(rust_tree, 2, path.as_mut_ptr(), 4),
            -1
        );
        (*c_child).parent_id = 1;
        (*rust_child).parent_id = 1;
        (pair.c.tree_delete)(c_tree);
        (pair.rust.tree_delete)(rust_tree);

        let mut c_parent = TreeNode {
            id: 1,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0; MAX_DATA_LENGTH],
        };
        let mut rust_parent = TreeNode {
            id: 1,
            parent_id: 0,
            child_ids: [0; MAX_CHILDREN],
            child_count: 0,
            data: [0; MAX_DATA_LENGTH],
        };
        let mut c_entries: [HashmapEntry; 4] = std::array::from_fn(|index| HashmapEntry {
            key: index as u64 + 1,
            value: ptr::null_mut(),
            occupied: 1,
            deleted: 0,
        });
        let mut rust_entries: [HashmapEntry; 4] = std::array::from_fn(|index| HashmapEntry {
            key: index as u64 + 1,
            value: ptr::null_mut(),
            occupied: 1,
            deleted: 0,
        });
        c_entries[0].value = (&mut c_parent as *mut TreeNode).cast();
        rust_entries[0].value = (&mut rust_parent as *mut TreeNode).cast();
        let mut c_map = Hashmap {
            entries: c_entries.as_mut_ptr(),
            capacity: 4,
            size: 0,
            deleted_count: 0,
        };
        let mut rust_map = Hashmap {
            entries: rust_entries.as_mut_ptr(),
            capacity: 4,
            size: 0,
            deleted_count: 0,
        };
        let mut c_tree = Tree {
            node_map: &mut c_map,
            root_id: 1,
            has_root: 1,
            node_count: 1,
        };
        let mut rust_tree = Tree {
            node_map: &mut rust_map,
            root_id: 1,
            has_root: 1,
            node_count: 1,
        };
        let data = c_string(b"will-fail");
        assert_eq!(
            (pair.c.tree_add_node)(&mut c_tree, 99, 1, data.as_ptr()),
            -1
        );
        assert_eq!(
            (pair.rust.tree_add_node)(&mut rust_tree, 99, 1, data.as_ptr()),
            -1
        );
        assert_eq!(c_parent.child_count, 1);
        assert_eq!(c_parent.child_count, rust_parent.child_count);
        assert_eq!(c_parent.child_ids[0], 99);
        assert_eq!(c_parent.child_ids[0], rust_parent.child_ids[0]);
    }
}

type FailAllocSet = unsafe extern "C" fn(c_long);

unsafe fn exercise_allocation_failures(api: &Api, fail_alloc_set: FailAllocSet) {
    fail_alloc_set(0);
    assert!((api.hashmap_create)().is_null());

    fail_alloc_set(1);
    assert!((api.hashmap_create)().is_null());

    let map = (api.hashmap_create)();
    assert!(!map.is_null());
    let mut values: Vec<Box<i64>> = (0..14).map(|value| Box::new(value)).collect();
    for key in 0..13_u64 {
        assert_eq!(
            (api.hashmap_put)(
                map,
                key,
                (&mut *values[key as usize]) as *mut i64 as *mut c_void,
            ),
            0
        );
    }
    fail_alloc_set(0);
    assert_eq!(
        (api.hashmap_put)(map, 13, (&mut *values[13]) as *mut i64 as *mut c_void,),
        -1
    );
    assert_eq!((*map).capacity, 16);
    assert_eq!((*map).size, 13);
    assert_eq!(
        (api.hashmap_put)(map, 13, (&mut *values[13]) as *mut i64 as *mut c_void,),
        0
    );
    assert_eq!((*map).capacity, 32);
    (api.hashmap_destroy)(map);

    fail_alloc_set(0);
    assert!((api.tree_create)().is_null());
    fail_alloc_set(1);
    assert!((api.tree_create)().is_null());
    fail_alloc_set(2);
    assert!((api.tree_create)().is_null());

    let tree = (api.tree_create)();
    assert!(!tree.is_null());
    let root = c_string(b"root");
    fail_alloc_set(0);
    assert_eq!((api.tree_add_node)(tree, 1, 0, root.as_ptr()), -1);
    assert_eq!((*tree).node_count, 0);
    assert_eq!((*tree).has_root, 0);
    (api.tree_delete)(tree);

    let tree = (api.tree_create)();
    let child = c_string(b"child");
    assert_eq!((api.tree_add_node)(tree, 1, 0, root.as_ptr()), 0);
    for id in 2..=13_u64 {
        assert_eq!((api.tree_add_node)(tree, id, 1, child.as_ptr()), 0);
    }
    assert_eq!((*(*tree).node_map).size, 13);
    fail_alloc_set(1);
    assert_eq!((api.tree_add_node)(tree, 14, 1, child.as_ptr()), -1);
    assert_eq!((*tree).node_count, 13);
    assert_eq!((*(*tree).node_map).capacity, 16);
    assert_eq!((*(api.tree_get_node)(tree, 1)).child_count, 13);
    assert_eq!((*(api.tree_get_node)(tree, 1)).child_ids[12], 14);
    (api.tree_delete)(tree);
}

#[test]
fn allocation_failure_surface_matches() {
    let helper_path = manifest_dir().join("target/debug/libfail_alloc.so");
    if std::env::var_os("DRIVER_FAIL_ALLOC_CHILD").is_none() {
        let compile = Command::new("cc")
            .args(["-fPIC", "-shared"])
            .arg(manifest_dir().join("tests/fail_alloc.c"))
            .args(["-o"])
            .arg(&helper_path)
            .output()
            .expect("failed to execute cc for allocator interposer");
        assert!(
            compile.status.success(),
            "allocator interposer compile failed:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );

        let current_exe = std::env::current_exe().unwrap();
        let preload = match std::env::var_os("LD_PRELOAD") {
            Some(existing) if !existing.is_empty() => {
                let mut value = helper_path.as_os_str().to_os_string();
                value.push(":");
                value.push(existing);
                value
            }
            _ => helper_path.as_os_str().to_os_string(),
        };
        let output = Command::new(current_exe)
            .args([
                "allocation_failure_surface_matches",
                "--exact",
                "--test-threads=1",
            ])
            .env("DRIVER_FAIL_ALLOC_CHILD", "1")
            .env("FAIL_ALLOC_SO", &helper_path)
            .env("LD_PRELOAD", preload)
            .env("C_DRIVER_SO", c_library_path())
            .env("RUST_DRIVER_SO", rust_library_path())
            .output()
            .expect("failed to execute allocator-failure child");
        assert!(
            output.status.success(),
            "allocator-failure child failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    unsafe {
        let helper = Library::new(std::env::var_os("FAIL_ALLOC_SO").unwrap()).unwrap();
        let fail_alloc_set = *helper.get::<FailAllocSet>(b"fail_alloc_set\0").unwrap();
        let pair = ApiPair::load();
        exercise_allocation_failures(&pair.c, fail_alloc_set);
        exercise_allocation_failures(&pair.rust, fail_alloc_set);
    }
}

fn defined_dynamic_symbols(path: &Path) -> std::collections::BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to execute nm");
    assert!(
        output.status.success(),
        "nm failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .map(str::to_owned)
        .collect()
}

#[test]
fn dynamic_symbol_surface_matches() {
    let expected: std::collections::BTreeSet<_> = [
        "hashmap_clear",
        "hashmap_contains",
        "hashmap_create",
        "hashmap_destroy",
        "hashmap_get",
        "hashmap_put",
        "hashmap_remove",
        "hashmap_size",
        "tree_add_node",
        "tree_contains",
        "tree_count_descendants",
        "tree_create",
        "tree_delete",
        "tree_find_path",
        "tree_get_depth",
        "tree_get_height",
        "tree_get_node",
        "tree_print",
        "tree_remove_node",
        "tree_size",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let c_symbols = defined_dynamic_symbols(&c_library_path());
    let rust_symbols = defined_dynamic_symbols(&rust_library_path());
    assert_eq!(c_symbols, expected);
    assert!(
        expected.is_subset(&rust_symbols),
        "missing Rust symbols: {:?}",
        expected.difference(&rust_symbols).collect::<Vec<_>>()
    );
}
