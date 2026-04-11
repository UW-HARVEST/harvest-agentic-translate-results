extern "C" {
    
    
    
    
    
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
pub use crate::src::hashmap::hashmap_create;
pub use crate::src::hashmap::hashmap_destroy;
pub use crate::src::hashmap::hashmap_get;
pub use crate::src::hashmap::hashmap_put;
pub use crate::src::hashmap::hashmap_remove;
pub use crate::src::hashmap::size_t;
pub use crate::src::hashmap::__uint64_t;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
pub use crate::src::hashmap::uint64_t;
pub use crate::src::hashmap::tree_id_t;
// #[derive(Copy, Clone)]

pub use crate::src::hashmap::hashmap_entry;
pub use crate::src::hashmap::hashmap_entry_t;
// #[derive(Copy, Clone)]

pub use crate::src::hashmap::hashmap_t;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const MAX_CHILDREN: libc::c_int = 32 as libc::c_int;
pub const MAX_DATA_LENGTH: libc::c_int = 256 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn tree_create() -> *mut tree_t {
    let mut tree: *mut tree_t = malloc(std::mem::size_of::<tree_t>() as size_t) as *mut tree_t;
    if tree.is_null() {
        return std::ptr::null_mut::<tree_t>();
    }
    (*tree).node_map = hashmap_create();
    if (*tree).node_map.is_null() {
        free(tree as *mut libc::c_void);
        return std::ptr::null_mut::<tree_t>();
    }
    (*tree).root_id = 0 as tree_id_t;
    (*tree).has_root = 0 as libc::c_int;
    (*tree).node_count = 0 as size_t;
    return tree;
}
unsafe extern "C" fn tree_free_node(mut node: *mut tree_node_t) {
    if !node.is_null() {
        free(node as *mut libc::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn tree_delete(mut tree: *mut tree_t) {
    if tree.is_null() {
        return;
    }
    let mut i: size_t = 0 as size_t;
    while i < (*(*tree).node_map).capacity {
        if (*(*(*tree).node_map).entries.offset(i as isize)).occupied != 0
            && (*(*(*tree).node_map).entries.offset(i as isize)).deleted == 0
        {
            tree_free_node(
                (*(*(*tree).node_map).entries.offset(i as isize)).value as *mut tree_node_t,
            );
        }
        i = i.wrapping_add(1);
    }
    hashmap_destroy((*tree).node_map);
    free(tree as *mut libc::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn tree_add_node(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
    mut parent_id: tree_id_t,
    mut data: *const libc::c_char,
) -> libc::c_int {
    if tree.is_null() {
        return -(1 as libc::c_int);
    }
    if tree_contains(tree, id) != 0 {
        fprintf(
            stderr as *mut FILE,
            b"Error: Node with ID %lu already exists\n\0" as *const u8
                as *const libc::c_char,
            id,
        );
        return -(1 as libc::c_int);
    }
    let mut node: *mut tree_node_t =
        malloc(std::mem::size_of::<tree_node_t>() as size_t) as *mut tree_node_t;
    if node.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to allocate node\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    (*node).id = id;
    (*node).parent_id = parent_id;
    (*node).child_count = 0 as libc::c_int;
    if !data.is_null() {
        strncpy(
            &raw mut (*node).data as *mut libc::c_char,
            data,
            (MAX_DATA_LENGTH - 1 as libc::c_int) as size_t,
        );
        (*node).data[(MAX_DATA_LENGTH - 1 as libc::c_int) as usize] =
            '\0' as i32 as libc::c_char;
    } else {
        (*node).data[0 as libc::c_int as usize] = '\0' as i32 as libc::c_char;
    }
    if (*tree).has_root == 0 {
        (*tree).root_id = id;
        (*tree).has_root = 1 as libc::c_int;
        (*node).parent_id = 0 as tree_id_t;
    } else {
        let mut parent: *mut tree_node_t = tree_get_node(tree, parent_id);
        if parent.is_null() {
            fprintf(
                stderr as *mut FILE,
                b"Error: Parent node %lu not found\n\0" as *const u8 as *const libc::c_char,
                parent_id,
            );
            free(node as *mut libc::c_void);
            return -(1 as libc::c_int);
        }
        if (*parent).child_count >= MAX_CHILDREN {
            fprintf(
                stderr as *mut FILE,
                b"Error: Parent has maximum children\n\0" as *const u8
                    as *const libc::c_char,
            );
            free(node as *mut libc::c_void);
            return -(1 as libc::c_int);
        }
        let fresh0 = (*parent).child_count;
        (*parent).child_count = (*parent).child_count + 1;
        (*parent).child_ids[fresh0 as usize] = id;
    }
    if hashmap_put((*tree).node_map, id, node as *mut libc::c_void)
        != 0 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to add node to hashmap\n\0" as *const u8 as *const libc::c_char,
        );
        free(node as *mut libc::c_void);
        return -(1 as libc::c_int);
    }
    (*tree).node_count = (*tree).node_count.wrapping_add(1);
    return 0 as libc::c_int;
}
unsafe extern "C" fn tree_remove_subtree(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
) -> libc::c_int {
    let mut node: *mut tree_node_t = tree_get_node(tree, id);
    if node.is_null() {
        return -(1 as libc::c_int);
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*node).child_count {
        tree_remove_subtree(tree, (*node).child_ids[i as usize]);
        i += 1;
    }
    let mut removed: *mut tree_node_t = hashmap_remove((*tree).node_map, id) as *mut tree_node_t;
    if !removed.is_null() {
        tree_free_node(removed);
        (*tree).node_count = (*tree).node_count.wrapping_sub(1);
    }
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tree_remove_node(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
) -> libc::c_int {
    if tree.is_null() {
        return -(1 as libc::c_int);
    }
    let mut node: *mut tree_node_t = tree_get_node(tree, id);
    if node.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Node %lu not found\n\0" as *const u8 as *const libc::c_char,
            id,
        );
        return -(1 as libc::c_int);
    }
    if id == (*tree).root_id {
        tree_remove_subtree(tree, id);
        (*tree).has_root = 0 as libc::c_int;
        (*tree).root_id = 0 as tree_id_t;
        return 0 as libc::c_int;
    }
    let mut parent: *mut tree_node_t = tree_get_node(tree, (*node).parent_id);
    if !parent.is_null() {
        let mut i: libc::c_int = 0 as libc::c_int;
        while i < (*parent).child_count {
            if (*parent).child_ids[i as usize] == id {
                let mut j: libc::c_int = i;
                while j < (*parent).child_count - 1 as libc::c_int {
                    (*parent).child_ids[j as usize] =
                        (*parent).child_ids[(j + 1 as libc::c_int) as usize];
                    j += 1;
                }
                (*parent).child_count -= 1;
                break;
            } else {
                i += 1;
            }
        }
    }
    tree_remove_subtree(tree, id);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tree_get_node(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
) -> *mut tree_node_t {
    if tree.is_null() {
        return std::ptr::null_mut::<tree_node_t>();
    }
    return hashmap_get((*tree).node_map, id) as *mut tree_node_t;
}
#[no_mangle]
pub unsafe extern "C" fn tree_contains(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
) -> libc::c_int {
    return (tree_get_node(tree, id) != NULL as *mut tree_node_t) as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tree_size(mut tree: *mut tree_t) -> size_t {
    return if !tree.is_null() {
        (*tree).node_count
    } else {
        0 as size_t
    };
}
unsafe extern "C" fn tree_print_helper(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
    mut depth: libc::c_int,
) {
    let mut node: *mut tree_node_t = tree_get_node(tree, id);
    if node.is_null() {
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < depth {
        printf(b"  \0" as *const u8 as *const libc::c_char);
        i += 1;
    }
    printf(
        b"[%lu] %s\n\0" as *const u8 as *const libc::c_char,
        (*node).id,
        &raw mut (*node).data as *mut libc::c_char,
    );
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < (*node).child_count {
        tree_print_helper(
            tree,
            (*node).child_ids[i_0 as usize],
            depth + 1 as libc::c_int,
        );
        i_0 += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn tree_print(mut tree: *mut tree_t) {
    if tree.is_null() || (*tree).has_root == 0 {
        printf(b"(empty tree)\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    tree_print_helper(tree, (*tree).root_id, 0 as libc::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn tree_get_depth(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
) -> libc::c_int {
    if tree.is_null() || tree_contains(tree, id) == 0 {
        return -(1 as libc::c_int);
    }
    let mut depth: libc::c_int = 0 as libc::c_int;
    let mut current_id: tree_id_t = id;
    while current_id != (*tree).root_id {
        let mut node: *mut tree_node_t = tree_get_node(tree, current_id);
        if node.is_null() {
            return -(1 as libc::c_int);
        }
        current_id = (*node).parent_id;
        depth += 1;
    }
    return depth;
}
#[no_mangle]
pub unsafe extern "C" fn tree_get_height(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
) -> libc::c_int {
    let mut node: *mut tree_node_t = tree_get_node(tree, id);
    if node.is_null() {
        return -(1 as libc::c_int);
    }
    if (*node).child_count == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    let mut max_height: libc::c_int = 0 as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*node).child_count {
        let mut child_height: libc::c_int =
            tree_get_height(tree, (*node).child_ids[i as usize]);
        if child_height > max_height {
            max_height = child_height;
        }
        i += 1;
    }
    return max_height + 1 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tree_count_descendants(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
) -> libc::c_int {
    let mut node: *mut tree_node_t = tree_get_node(tree, id);
    if node.is_null() {
        return -(1 as libc::c_int);
    }
    let mut count: libc::c_int = 0 as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*node).child_count {
        count += 1;
        count += tree_count_descendants(tree, (*node).child_ids[i as usize]);
        i += 1;
    }
    return count;
}
#[no_mangle]
pub unsafe extern "C" fn tree_find_path(
    mut tree: *mut tree_t,
    mut id: tree_id_t,
    mut path: *mut tree_id_t,
    mut max_length: libc::c_int,
) -> libc::c_int {
    if tree.is_null() || path.is_null() || tree_contains(tree, id) == 0 {
        return -(1 as libc::c_int);
    }
    let mut temp_path: [tree_id_t; 1000] = [0; 1000];
    let mut length: libc::c_int = 0 as libc::c_int;
    let mut current_id: tree_id_t = id;
    while length < 1000 as libc::c_int {
        let fresh1 = length;
        length = length + 1;
        temp_path[fresh1 as usize] = current_id;
        if current_id == (*tree).root_id {
            break;
        }
        let mut node: *mut tree_node_t = tree_get_node(tree, current_id);
        if node.is_null() {
            return -(1 as libc::c_int);
        }
        current_id = (*node).parent_id;
    }
    if length > max_length {
        length = max_length;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < length {
        *path.offset(i as isize) = temp_path[(length - 1 as libc::c_int - i) as usize];
        i += 1;
    }
    return length;
}
