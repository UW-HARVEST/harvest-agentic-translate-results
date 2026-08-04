#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(label_break_value, raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
    fn __assert_fail(
        __assertion: *const libc::c_char,
        __file: *const libc::c_char,
        __line: libc::c_uint,
        __function: *const libc::c_char,
    ) -> !;
    fn hashmap_create() -> *mut hashmap_t;
    fn hashmap_destroy(map: *mut hashmap_t);
    fn hashmap_put(
        map: *mut hashmap_t,
        key: tree_id_t,
        value: *mut libc::c_void,
    ) -> libc::c_int;
    fn hashmap_get(map: *mut hashmap_t, key: tree_id_t) -> *mut libc::c_void;
    fn hashmap_remove(map: *mut hashmap_t, key: tree_id_t) -> *mut libc::c_void;
    fn hashmap_contains(map: *mut hashmap_t, key: tree_id_t) -> libc::c_int;
    fn hashmap_size(map: *mut hashmap_t) -> size_t;
    fn tree_create() -> *mut tree_t;
    fn tree_delete(tree: *mut tree_t);
    fn tree_add_node(
        tree: *mut tree_t,
        id: tree_id_t,
        parent_id: tree_id_t,
        data: *const libc::c_char,
    ) -> libc::c_int;
    fn tree_remove_node(tree: *mut tree_t, id: tree_id_t) -> libc::c_int;
    fn tree_get_node(tree: *mut tree_t, id: tree_id_t) -> *mut tree_node_t;
    fn tree_contains(tree: *mut tree_t, id: tree_id_t) -> libc::c_int;
    fn tree_size(tree: *mut tree_t) -> size_t;
    fn tree_print(tree: *mut tree_t);
    fn tree_get_depth(tree: *mut tree_t, id: tree_id_t) -> libc::c_int;
    fn tree_get_height(tree: *mut tree_t, id: tree_id_t) -> libc::c_int;
    fn tree_count_descendants(tree: *mut tree_t, id: tree_id_t) -> libc::c_int;
    fn tree_find_path(
        tree: *mut tree_t,
        id: tree_id_t,
        path: *mut tree_id_t,
        max_length: libc::c_int,
    ) -> libc::c_int;
}
pub type size_t = usize;
pub type __uint64_t = u64;
pub type uint64_t = __uint64_t;
pub type tree_id_t = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct hashmap_entry {
    pub key: tree_id_t,
    pub value: *mut libc::c_void,
    pub occupied: libc::c_int,
    pub deleted: libc::c_int,
}
pub type hashmap_entry_t = hashmap_entry;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct hashmap_t {
    pub entries: *mut hashmap_entry_t,
    pub capacity: size_t,
    pub size: size_t,
    pub deleted_count: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tree_node {
    pub id: tree_id_t,
    pub parent_id: tree_id_t,
    pub child_ids: [tree_id_t; 32],
    pub child_count: libc::c_int,
    pub data: [libc::c_char; 256],
}
pub type tree_node_t = tree_node;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tree_t {
    pub node_map: *mut hashmap_t,
    pub root_id: tree_id_t,
    pub has_root: libc::c_int,
    pub node_count: size_t,
}
pub const MAX_CHILDREN: libc::c_int = 32 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn test_hashmap_basic() {
    printf(
        b"\n=== Testing Hashmap Basic Operations ===\n\0" as *const u8
            as *const libc::c_char,
    );
    let mut map: *mut hashmap_t = hashmap_create();
    '_c2rust_label: {
        if !map.is_null() {
        } else {
            __assert_fail(
                b"map != NULL\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                39 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if hashmap_size(map) == 0 as size_t {
        } else {
            __assert_fail(
                b"hashmap_size(map) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                40 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut val1: libc::c_int = 42 as libc::c_int;
    let mut val2: libc::c_int = 100 as libc::c_int;
    let mut val3: libc::c_int = 200 as libc::c_int;
    '_c2rust_label_1: {
        if hashmap_put(
            map,
            1 as tree_id_t,
            &raw mut val1 as *mut libc::c_void,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"hashmap_put(map, 1, &val1) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                44 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if hashmap_put(
            map,
            2 as tree_id_t,
            &raw mut val2 as *mut libc::c_void,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"hashmap_put(map, 2, &val2) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                45 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if hashmap_put(
            map,
            3 as tree_id_t,
            &raw mut val3 as *mut libc::c_void,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"hashmap_put(map, 3, &val3) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                46 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if hashmap_size(map) == 3 as size_t {
        } else {
            __assert_fail(
                b"hashmap_size(map) == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                47 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if *(hashmap_get(map, 1 as tree_id_t) as *mut libc::c_int)
            == 42 as libc::c_int
        {
        } else {
            __assert_fail(
                b"*(int *)hashmap_get(map, 1) == 42\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                49 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_6: {
        if *(hashmap_get(map, 2 as tree_id_t) as *mut libc::c_int)
            == 100 as libc::c_int
        {
        } else {
            __assert_fail(
                b"*(int *)hashmap_get(map, 2) == 100\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                50 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_7: {
        if *(hashmap_get(map, 3 as tree_id_t) as *mut libc::c_int)
            == 200 as libc::c_int
        {
        } else {
            __assert_fail(
                b"*(int *)hashmap_get(map, 3) == 200\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                51 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut val4: libc::c_int = 500 as libc::c_int;
    '_c2rust_label_8: {
        if hashmap_put(
            map,
            1 as tree_id_t,
            &raw mut val4 as *mut libc::c_void,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"hashmap_put(map, 1, &val4) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                55 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_9: {
        if hashmap_size(map) == 3 as size_t {
        } else {
            __assert_fail(
                b"hashmap_size(map) == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                56 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_10: {
        if *(hashmap_get(map, 1 as tree_id_t) as *mut libc::c_int)
            == 500 as libc::c_int
        {
        } else {
            __assert_fail(
                b"*(int *)hashmap_get(map, 1) == 500\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                57 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut removed: *mut libc::c_void = hashmap_remove(map, 2 as tree_id_t);
    '_c2rust_label_11: {
        if removed == &raw mut val2 as *mut libc::c_void {
        } else {
            __assert_fail(
                b"removed == &val2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                61 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_12: {
        if hashmap_size(map) == 2 as size_t {
        } else {
            __assert_fail(
                b"hashmap_size(map) == 2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                62 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_13: {
        if hashmap_get(map, 2 as tree_id_t).is_null() {
        } else {
            __assert_fail(
                b"hashmap_get(map, 2) == NULL\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                63 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_14: {
        if hashmap_contains(map, 1 as tree_id_t) == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"hashmap_contains(map, 1) == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                66 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_15: {
        if hashmap_contains(map, 2 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"hashmap_contains(map, 2) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                67 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_16: {
        if hashmap_contains(map, 3 as tree_id_t) == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"hashmap_contains(map, 3) == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                68 as libc::c_uint,
                b"void test_hashmap_basic(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    hashmap_destroy(map);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_hashmap_basic\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_hashmap_collisions() {
    printf(b"\n=== Testing Hashmap Collisions ===\n\0" as *const u8 as *const libc::c_char);
    let mut map: *mut hashmap_t = hashmap_create();
    let mut values: [libc::c_int; 100] = [0; 100];
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 100 as libc::c_int {
        values[i as usize] = i * 10 as libc::c_int;
        '_c2rust_label: {
            if hashmap_put(
                map,
                i as tree_id_t,
                (&raw mut values as *mut libc::c_int).offset(i as isize)
                    as *mut libc::c_int as *mut libc::c_void,
            ) == 0 as libc::c_int
            {
            } else {
                __assert_fail(
                    b"hashmap_put(map, i, &values[i]) == 0\0" as *const u8
                        as *const libc::c_char,
                    b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                        as *const libc::c_char,
                    83 as libc::c_uint,
                    b"void test_hashmap_collisions(void)\0" as *const u8
                        as *const libc::c_char,
                );
            }
        };
        i += 1;
    }
    '_c2rust_label_0: {
        if hashmap_size(map) == 100 as size_t {
        } else {
            __assert_fail(
                b"hashmap_size(map) == 100\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                86 as libc::c_uint,
                b"void test_hashmap_collisions(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < 100 as libc::c_int {
        let mut val: *mut libc::c_int =
            hashmap_get(map, i_0 as tree_id_t) as *mut libc::c_int;
        '_c2rust_label_1: {
            if !val.is_null() {
            } else {
                __assert_fail(
                    b"val != NULL\0" as *const u8 as *const libc::c_char,
                    b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                        as *const libc::c_char,
                    91 as libc::c_uint,
                    b"void test_hashmap_collisions(void)\0" as *const u8
                        as *const libc::c_char,
                );
            }
        };
        '_c2rust_label_2: {
            if *val == i_0 * 10 as libc::c_int {
            } else {
                __assert_fail(
                    b"*val == i * 10\0" as *const u8 as *const libc::c_char,
                    b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                        as *const libc::c_char,
                    92 as libc::c_uint,
                    b"void test_hashmap_collisions(void)\0" as *const u8
                        as *const libc::c_char,
                );
            }
        };
        i_0 += 1;
    }
    hashmap_destroy(map);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_hashmap_collisions\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_creation() {
    printf(b"\n=== Testing Tree Creation ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if !tree.is_null() {
        } else {
            __assert_fail(
                b"tree != NULL\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                103 as libc::c_uint,
                b"void test_tree_creation(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_size(tree) == 0 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                104 as libc::c_uint,
                b"void test_tree_creation(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if (*tree).has_root == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree->has_root == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                105 as libc::c_uint,
                b"void test_tree_creation(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_creation\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_add_root() {
    printf(b"\n=== Testing Tree Add Root ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                117 as libc::c_uint,
                b"void test_tree_add_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_size(tree) == 1 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                118 as libc::c_uint,
                b"void test_tree_add_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if (*tree).has_root == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"tree->has_root == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                119 as libc::c_uint,
                b"void test_tree_add_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if (*tree).root_id == 1 as tree_id_t {
        } else {
            __assert_fail(
                b"tree->root_id == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                120 as libc::c_uint,
                b"void test_tree_add_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut root: *mut tree_node_t = tree_get_node(tree, 1 as tree_id_t);
    '_c2rust_label_3: {
        if !root.is_null() {
        } else {
            __assert_fail(
                b"root != NULL\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                123 as libc::c_uint,
                b"void test_tree_add_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if (*root).id == 1 as tree_id_t {
        } else {
            __assert_fail(
                b"root->id == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                124 as libc::c_uint,
                b"void test_tree_add_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if strcmp(
            &raw mut (*root).data as *mut libc::c_char,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"strcmp(root->data, \"root\") == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                125 as libc::c_uint,
                b"void test_tree_add_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_6: {
        if (*root).child_count == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"root->child_count == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                126 as libc::c_uint,
                b"void test_tree_add_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_add_root\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_add_children() {
    printf(b"\n=== Testing Tree Add Children ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                138 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"child1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"child1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                139 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            3 as tree_id_t,
            1 as tree_id_t,
            b"child2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 3, 1, \"child2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                140 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_add_node(
            tree,
            4 as tree_id_t,
            1 as tree_id_t,
            b"child3\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 4, 1, \"child3\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                141 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if tree_size(tree) == 4 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 4\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                143 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut root: *mut tree_node_t = tree_get_node(tree, 1 as tree_id_t);
    '_c2rust_label_4: {
        if (*root).child_count == 3 as libc::c_int {
        } else {
            __assert_fail(
                b"root->child_count == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                146 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if (*root).child_ids[0 as libc::c_int as usize] == 2 as tree_id_t {
        } else {
            __assert_fail(
                b"root->child_ids[0] == 2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                147 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_6: {
        if (*root).child_ids[1 as libc::c_int as usize] == 3 as tree_id_t {
        } else {
            __assert_fail(
                b"root->child_ids[1] == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                148 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_7: {
        if (*root).child_ids[2 as libc::c_int as usize] == 4 as tree_id_t {
        } else {
            __assert_fail(
                b"root->child_ids[2] == 4\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                149 as libc::c_uint,
                b"void test_tree_add_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_add_children\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_deep_hierarchy() {
    printf(b"\n=== Testing Tree Deep Hierarchy ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"level0\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"level0\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                161 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"level1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"level1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                162 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            3 as tree_id_t,
            2 as tree_id_t,
            b"level2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 3, 2, \"level2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                163 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_add_node(
            tree,
            4 as tree_id_t,
            3 as tree_id_t,
            b"level3\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 4, 3, \"level3\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                164 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if tree_add_node(
            tree,
            5 as tree_id_t,
            4 as tree_id_t,
            b"level4\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 5, 4, \"level4\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                165 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if tree_size(tree) == 5 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 5\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                167 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if tree_get_depth(tree, 1 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_depth(tree, 1) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                169 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_6: {
        if tree_get_depth(tree, 2 as tree_id_t) == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_depth(tree, 2) == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                170 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_7: {
        if tree_get_depth(tree, 3 as tree_id_t) == 2 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_depth(tree, 3) == 2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                171 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_8: {
        if tree_get_depth(tree, 4 as tree_id_t) == 3 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_depth(tree, 4) == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                172 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_9: {
        if tree_get_depth(tree, 5 as tree_id_t) == 4 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_depth(tree, 5) == 4\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                173 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_10: {
        if tree_get_height(tree, 1 as tree_id_t) == 4 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_height(tree, 1) == 4\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                175 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_11: {
        if tree_get_height(tree, 2 as tree_id_t) == 3 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_height(tree, 2) == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                176 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_12: {
        if tree_get_height(tree, 5 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_height(tree, 5) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                177 as libc::c_uint,
                b"void test_tree_deep_hierarchy(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_deep_hierarchy\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_remove_leaf() {
    printf(b"\n=== Testing Tree Remove Leaf ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                188 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"child1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"child1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                189 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            3 as tree_id_t,
            1 as tree_id_t,
            b"child2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 3, 1, \"child2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                190 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_size(tree) == 3 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                192 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if tree_remove_node(tree, 3 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_remove_node(tree, 3) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                195 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if tree_size(tree) == 2 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                196 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if tree_contains(tree, 3 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_contains(tree, 3) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                197 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut root: *mut tree_node_t = tree_get_node(tree, 1 as tree_id_t);
    '_c2rust_label_6: {
        if (*root).child_count == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"root->child_count == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                200 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_7: {
        if (*root).child_ids[0 as libc::c_int as usize] == 2 as tree_id_t {
        } else {
            __assert_fail(
                b"root->child_ids[0] == 2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                201 as libc::c_uint,
                b"void test_tree_remove_leaf(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_remove_leaf\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_remove_subtree() {
    printf(b"\n=== Testing Tree Remove Subtree ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                213 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"child1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"child1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                214 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            3 as tree_id_t,
            2 as tree_id_t,
            b"grandchild1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 3, 2, \"grandchild1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                215 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_add_node(
            tree,
            4 as tree_id_t,
            2 as tree_id_t,
            b"grandchild2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 4, 2, \"grandchild2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                216 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if tree_add_node(
            tree,
            5 as tree_id_t,
            1 as tree_id_t,
            b"child2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 5, 1, \"child2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                217 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if tree_size(tree) == 5 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 5\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                219 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if tree_remove_node(tree, 2 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_remove_node(tree, 2) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                222 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_6: {
        if tree_size(tree) == 2 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                223 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_7: {
        if tree_contains(tree, 2 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_contains(tree, 2) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                224 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_8: {
        if tree_contains(tree, 3 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_contains(tree, 3) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                225 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_9: {
        if tree_contains(tree, 4 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_contains(tree, 4) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                226 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_10: {
        if tree_contains(tree, 1 as tree_id_t) == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_contains(tree, 1) == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                227 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_11: {
        if tree_contains(tree, 5 as tree_id_t) == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_contains(tree, 5) == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                228 as libc::c_uint,
                b"void test_tree_remove_subtree(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_remove_subtree\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_remove_root() {
    printf(b"\n=== Testing Tree Remove Root ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                239 as libc::c_uint,
                b"void test_tree_remove_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"child1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"child1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                240 as libc::c_uint,
                b"void test_tree_remove_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            3 as tree_id_t,
            1 as tree_id_t,
            b"child2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 3, 1, \"child2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                241 as libc::c_uint,
                b"void test_tree_remove_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_size(tree) == 3 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                243 as libc::c_uint,
                b"void test_tree_remove_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if tree_remove_node(tree, 1 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_remove_node(tree, 1) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                246 as libc::c_uint,
                b"void test_tree_remove_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if tree_size(tree) == 0 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                247 as libc::c_uint,
                b"void test_tree_remove_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if (*tree).has_root == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree->has_root == 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                248 as libc::c_uint,
                b"void test_tree_remove_root(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_remove_root\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_count_descendants() {
    printf(
        b"\n=== Testing Tree Count Descendants ===\n\0" as *const u8 as *const libc::c_char,
    );
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                267 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"child1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"child1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                268 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            3 as tree_id_t,
            2 as tree_id_t,
            b"grandchild1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 3, 2, \"grandchild1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                269 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_add_node(
            tree,
            4 as tree_id_t,
            2 as tree_id_t,
            b"grandchild2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 4, 2, \"grandchild2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                270 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if tree_add_node(
            tree,
            5 as tree_id_t,
            1 as tree_id_t,
            b"child2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 5, 1, \"child2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                271 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if tree_count_descendants(tree, 1 as tree_id_t) == 4 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_count_descendants(tree, 1) == 4\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                273 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if tree_count_descendants(tree, 2 as tree_id_t) == 2 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_count_descendants(tree, 2) == 2\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                274 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_6: {
        if tree_count_descendants(tree, 3 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_count_descendants(tree, 3) == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                275 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_7: {
        if tree_count_descendants(tree, 5 as tree_id_t) == 0 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_count_descendants(tree, 5) == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                276 as libc::c_uint,
                b"void test_tree_count_descendants(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_count_descendants\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_find_path() {
    printf(b"\n=== Testing Tree Find Path ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                287 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"child\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"child\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                288 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            3 as tree_id_t,
            2 as tree_id_t,
            b"grandchild\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 3, 2, \"grandchild\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                289 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut path: [tree_id_t; 10] = [0; 10];
    let mut length: libc::c_int = 0;
    length = tree_find_path(
        tree,
        3 as tree_id_t,
        &raw mut path as *mut tree_id_t,
        10 as libc::c_int,
    );
    '_c2rust_label_2: {
        if length == 3 as libc::c_int {
        } else {
            __assert_fail(
                b"length == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                295 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if path[0 as libc::c_int as usize] == 1 as tree_id_t {
        } else {
            __assert_fail(
                b"path[0] == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                296 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if path[1 as libc::c_int as usize] == 2 as tree_id_t {
        } else {
            __assert_fail(
                b"path[1] == 2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                297 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if path[2 as libc::c_int as usize] == 3 as tree_id_t {
        } else {
            __assert_fail(
                b"path[2] == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                298 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    length = tree_find_path(
        tree,
        1 as tree_id_t,
        &raw mut path as *mut tree_id_t,
        10 as libc::c_int,
    );
    '_c2rust_label_6: {
        if length == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"length == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                301 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_7: {
        if path[0 as libc::c_int as usize] == 1 as tree_id_t {
        } else {
            __assert_fail(
                b"path[0] == 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                302 as libc::c_uint,
                b"void test_tree_find_path(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_find_path\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_duplicate_id() {
    printf(b"\n=== Testing Tree Duplicate ID ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                313 as libc::c_uint,
                b"void test_tree_duplicate_id(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"child\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"child\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                314 as libc::c_uint,
                b"void test_tree_duplicate_id(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"duplicate\0" as *const u8 as *const libc::c_char,
        ) != 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"duplicate\") != 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                317 as libc::c_uint,
                b"void test_tree_duplicate_id(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_size(tree) == 2 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 2\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                318 as libc::c_uint,
                b"void test_tree_duplicate_id(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_duplicate_id\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_max_children() {
    printf(b"\n=== Testing Tree Max Children ===\n\0" as *const u8 as *const libc::c_char);
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                329 as libc::c_uint,
                b"void test_tree_max_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < MAX_CHILDREN {
        '_c2rust_label_0: {
            if tree_add_node(
                tree,
                (i + 2 as libc::c_int) as tree_id_t,
                1 as tree_id_t,
                b"child\0" as *const u8 as *const libc::c_char,
            ) == 0 as libc::c_int
            {
            } else {
                __assert_fail(
                    b"tree_add_node(tree, i + 2, 1, \"child\") == 0\0" as *const u8
                        as *const libc::c_char,
                    b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                        as *const libc::c_char,
                    333 as libc::c_uint,
                    b"void test_tree_max_children(void)\0" as *const u8
                        as *const libc::c_char,
                );
            }
        };
        i += 1;
    }
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            (32 as libc::c_int + 2 as libc::c_int) as tree_id_t,
            1 as tree_id_t,
            b"overflow\0" as *const u8 as *const libc::c_char,
        ) != 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, MAX_CHILDREN + 2, 1, \"overflow\") != 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                337 as libc::c_uint,
                b"void test_tree_max_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_size(tree) == (32 as libc::c_int + 1 as libc::c_int) as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == MAX_CHILDREN + 1\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                338 as libc::c_uint,
                b"void test_tree_max_children(void)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_max_children\0" as *const u8 as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn test_tree_complex_structure() {
    printf(
        b"\n=== Testing Tree Complex Structure ===\n\0" as *const u8 as *const libc::c_char,
    );
    let mut tree: *mut tree_t = tree_create();
    '_c2rust_label: {
        if tree_add_node(
            tree,
            1 as tree_id_t,
            0 as tree_id_t,
            b"root\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 1, 0, \"root\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                359 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if tree_add_node(
            tree,
            2 as tree_id_t,
            1 as tree_id_t,
            b"child1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 2, 1, \"child1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                360 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if tree_add_node(
            tree,
            3 as tree_id_t,
            1 as tree_id_t,
            b"child2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 3, 1, \"child2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                361 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if tree_add_node(
            tree,
            4 as tree_id_t,
            1 as tree_id_t,
            b"child3\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 4, 1, \"child3\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                362 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if tree_add_node(
            tree,
            5 as tree_id_t,
            2 as tree_id_t,
            b"gc1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 5, 2, \"gc1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                363 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_4: {
        if tree_add_node(
            tree,
            6 as tree_id_t,
            2 as tree_id_t,
            b"gc2\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 6, 2, \"gc2\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                364 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_5: {
        if tree_add_node(
            tree,
            7 as tree_id_t,
            3 as tree_id_t,
            b"gc3\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 7, 3, \"gc3\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                365 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_6: {
        if tree_add_node(
            tree,
            8 as tree_id_t,
            4 as tree_id_t,
            b"gc4\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 8, 4, \"gc4\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                366 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_7: {
        if tree_add_node(
            tree,
            9 as tree_id_t,
            4 as tree_id_t,
            b"gc5\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 9, 4, \"gc5\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                367 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_8: {
        if tree_add_node(
            tree,
            10 as tree_id_t,
            7 as tree_id_t,
            b"ggc1\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
        {
        } else {
            __assert_fail(
                b"tree_add_node(tree, 10, 7, \"ggc1\") == 0\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                368 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_9: {
        if tree_size(tree) == 10 as size_t {
        } else {
            __assert_fail(
                b"tree_size(tree) == 10\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                370 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_10: {
        if tree_get_height(tree, 1 as tree_id_t) == 3 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_get_height(tree, 1) == 3\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                371 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_11: {
        if tree_count_descendants(tree, 1 as tree_id_t) == 9 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_count_descendants(tree, 1) == 9\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                372 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_12: {
        if tree_count_descendants(tree, 2 as tree_id_t) == 2 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_count_descendants(tree, 2) == 2\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                373 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_13: {
        if tree_count_descendants(tree, 7 as tree_id_t) == 1 as libc::c_int {
        } else {
            __assert_fail(
                b"tree_count_descendants(tree, 7) == 1\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-DaxS1H/driver/c_src/src/main.c\0" as *const u8
                    as *const libc::c_char,
                374 as libc::c_uint,
                b"void test_tree_complex_structure(void)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    tree_print(tree);
    tree_delete(tree);
    printf(
        b"\xE2\x9C\x93 PASS: %s\n\0" as *const u8 as *const libc::c_char,
        b"test_tree_complex_structure\0" as *const u8 as *const libc::c_char,
    );
}
unsafe fn main_0() -> libc::c_int {
    printf(
        b"\xE2\x95\x94\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x97\n\0"
            as *const u8 as *const libc::c_char,
    );
    printf(
        b"\xE2\x95\x91  TREE WITH HASHMAP ID MAPPING TESTS   \xE2\x95\x91\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"\xE2\x95\x9A\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x90\xE2\x95\x9D\n\0"
            as *const u8 as *const libc::c_char,
    );
    test_hashmap_basic();
    test_hashmap_collisions();
    test_tree_creation();
    test_tree_add_root();
    test_tree_add_children();
    test_tree_deep_hierarchy();
    test_tree_complex_structure();
    test_tree_remove_leaf();
    test_tree_remove_subtree();
    test_tree_remove_root();
    test_tree_count_descendants();
    test_tree_find_path();
    test_tree_duplicate_id();
    test_tree_max_children();
    printf(b"\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"========================================\n\0" as *const u8 as *const libc::c_char,
    );
    printf(b"  All tests passed successfully!\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"========================================\n\0" as *const u8 as *const libc::c_char,
    );
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
