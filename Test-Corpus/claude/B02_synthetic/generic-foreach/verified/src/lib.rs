// FFI shared-library entry points for the inventory / generic-containers code.
//
// All public C symbols are re-exported here with `#[no_mangle] extern "C"` so
// that the produced `cdylib` is a drop-in replacement for the C shared library
// built from `c_src/`.
//
// Memory layout for the container structs is chosen to match exactly what the
// macros in `c_src/include/generic_containers.h` and the structs in
// `c_src/include/inventory.h` produce. We use `libc::malloc` / `libc::free` /
// `libc::realloc` so that pointers handed back from the Rust side can be freed
// by callers that may have been built against the C side and vice-versa.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use libc::{c_char, c_double, c_int, c_void, size_t};

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

// ---------------------------------------------------------------------------
// Inventory structs (binary-identical to the C ones).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct item_t {
    pub id: c_int,
    pub name: [c_char; MAX_NAME_LENGTH],
    pub category: [c_char; MAX_CATEGORY_LENGTH],
    pub price: c_double,
    pub quantity: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct order_t {
    pub customer_id: c_int,
    pub customer_name: [c_char; MAX_NAME_LENGTH],
    pub total_amount: c_double,
}

// ---------------------------------------------------------------------------
// Generic dynamic-array structs and node structs for linked lists.
//
// The C macros generate `typedef struct { TYPE *data; size_t size; size_t
// capacity; } array_##TYPE##_t;` (and similar for lists). We declare one Rust
// type per element type so that pointer arithmetic and FFI is straight-forward.
// ---------------------------------------------------------------------------

macro_rules! define_array_struct {
    ($name:ident, $elem:ty) => {
        #[repr(C)]
        pub struct $name {
            pub data: *mut $elem,
            pub size: size_t,
            pub capacity: size_t,
        }
    };
}

define_array_struct!(array_int_t, c_int);
define_array_struct!(array_double_t, c_double);
define_array_struct!(array_item_t_t, item_t);
define_array_struct!(array_order_t_t, order_t);

macro_rules! define_list_node {
    ($node:ident, $elem:ty) => {
        #[repr(C)]
        pub struct $node {
            pub data: $elem,
            pub next: *mut $node,
        }
    };
}

define_list_node!(list_node_int_t, c_int);
define_list_node!(list_node_double_t, c_double);
define_list_node!(list_node_item_t_t, item_t);
define_list_node!(list_node_order_t_t, order_t);

macro_rules! define_list_struct {
    ($name:ident, $node:ident) => {
        #[repr(C)]
        pub struct $name {
            pub head: *mut $node,
            pub tail: *mut $node,
            pub size: size_t,
        }
    };
}

define_list_struct!(list_int_t, list_node_int_t);
define_list_struct!(list_double_t, list_node_double_t);
define_list_struct!(list_item_t_t, list_node_item_t_t);
define_list_struct!(list_order_t_t, list_node_order_t_t);

// ---------------------------------------------------------------------------
// Generic dynamic-array implementation for one element type.
//
// This mirrors `DEFINE_ARRAY(TYPE)` exactly:
//   - capacity defaults to 16 when the requested initial capacity is 0
//   - push() doubles capacity using realloc()
//   - get() does not bounds-check (matches the C macro)
//   - size() tolerates a null pointer
//   - clear() resets size without freeing memory
// ---------------------------------------------------------------------------

macro_rules! impl_array {
    ($create:ident, $destroy:ident, $push:ident, $get:ident, $size:ident,
     $clear:ident, $arr:ident, $elem:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $create(initial_capacity: size_t) -> *mut $arr {
            let arr = libc::malloc(std::mem::size_of::<$arr>()) as *mut $arr;
            if arr.is_null() {
                return std::ptr::null_mut();
            }
            let capacity = if initial_capacity > 0 { initial_capacity } else { 16 };
            (*arr).capacity = capacity;
            (*arr).size = 0;
            let bytes = std::mem::size_of::<$elem>().wrapping_mul(capacity);
            (*arr).data = libc::malloc(bytes) as *mut $elem;
            if (*arr).data.is_null() {
                libc::free(arr as *mut c_void);
                return std::ptr::null_mut();
            }
            arr
        }

        #[no_mangle]
        pub unsafe extern "C" fn $destroy(arr: *mut $arr) {
            if !arr.is_null() {
                libc::free((*arr).data as *mut c_void);
                libc::free(arr as *mut c_void);
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $push(arr: *mut $arr, value: $elem) -> c_int {
            if arr.is_null() {
                return -1;
            }
            if (*arr).size >= (*arr).capacity {
                let new_capacity = (*arr).capacity.wrapping_mul(2);
                let bytes = std::mem::size_of::<$elem>().wrapping_mul(new_capacity);
                let new_data = libc::realloc((*arr).data as *mut c_void, bytes)
                    as *mut $elem;
                if new_data.is_null() {
                    return -1;
                }
                (*arr).data = new_data;
                (*arr).capacity = new_capacity;
            }
            *(*arr).data.add((*arr).size) = value;
            (*arr).size = (*arr).size.wrapping_add(1);
            0
        }

        #[no_mangle]
        pub unsafe extern "C" fn $get(arr: *mut $arr, index: size_t) -> $elem {
            *(*arr).data.add(index)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $size(arr: *mut $arr) -> size_t {
            if arr.is_null() {
                0
            } else {
                (*arr).size
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $clear(arr: *mut $arr) {
            if !arr.is_null() {
                (*arr).size = 0;
            }
        }
    };
}

impl_array!(
    array_int_create,
    array_int_destroy,
    array_int_push,
    array_int_get,
    array_int_size,
    array_int_clear,
    array_int_t,
    c_int
);
impl_array!(
    array_double_create,
    array_double_destroy,
    array_double_push,
    array_double_get,
    array_double_size,
    array_double_clear,
    array_double_t,
    c_double
);
impl_array!(
    array_item_t_create,
    array_item_t_destroy,
    array_item_t_push,
    array_item_t_get,
    array_item_t_size,
    array_item_t_clear,
    array_item_t_t,
    item_t
);
impl_array!(
    array_order_t_create,
    array_order_t_destroy,
    array_order_t_push,
    array_order_t_get,
    array_order_t_size,
    array_order_t_clear,
    array_order_t_t,
    order_t
);

// ---------------------------------------------------------------------------
// Generic linked-list implementation for one element type.
// Mirrors `DEFINE_LIST(TYPE)` exactly.
// ---------------------------------------------------------------------------

macro_rules! impl_list {
    ($create:ident, $destroy:ident, $append:ident, $prepend:ident, $size:ident,
     $clear:ident, $list:ident, $node:ident, $elem:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $create() -> *mut $list {
            let list = libc::malloc(std::mem::size_of::<$list>()) as *mut $list;
            if list.is_null() {
                return std::ptr::null_mut();
            }
            (*list).head = std::ptr::null_mut();
            (*list).tail = std::ptr::null_mut();
            (*list).size = 0;
            list
        }

        #[no_mangle]
        pub unsafe extern "C" fn $destroy(list: *mut $list) {
            if list.is_null() {
                return;
            }
            let mut current = (*list).head;
            while !current.is_null() {
                let next = (*current).next;
                libc::free(current as *mut c_void);
                current = next;
            }
            libc::free(list as *mut c_void);
        }

        #[no_mangle]
        pub unsafe extern "C" fn $append(list: *mut $list, value: $elem) -> c_int {
            if list.is_null() {
                return -1;
            }
            let node = libc::malloc(std::mem::size_of::<$node>()) as *mut $node;
            if node.is_null() {
                return -1;
            }
            (*node).data = value;
            (*node).next = std::ptr::null_mut();
            if (*list).head.is_null() {
                (*list).head = node;
                (*list).tail = node;
            } else {
                (*(*list).tail).next = node;
                (*list).tail = node;
            }
            (*list).size = (*list).size.wrapping_add(1);
            0
        }

        #[no_mangle]
        pub unsafe extern "C" fn $prepend(list: *mut $list, value: $elem) -> c_int {
            if list.is_null() {
                return -1;
            }
            let node = libc::malloc(std::mem::size_of::<$node>()) as *mut $node;
            if node.is_null() {
                return -1;
            }
            (*node).data = value;
            (*node).next = (*list).head;
            (*list).head = node;
            if (*list).tail.is_null() {
                (*list).tail = node;
            }
            (*list).size = (*list).size.wrapping_add(1);
            0
        }

        #[no_mangle]
        pub unsafe extern "C" fn $size(list: *mut $list) -> size_t {
            if list.is_null() {
                0
            } else {
                (*list).size
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $clear(list: *mut $list) {
            if list.is_null() {
                return;
            }
            let mut current = (*list).head;
            while !current.is_null() {
                let next = (*current).next;
                libc::free(current as *mut c_void);
                current = next;
            }
            (*list).head = std::ptr::null_mut();
            (*list).tail = std::ptr::null_mut();
            (*list).size = 0;
        }
    };
}

impl_list!(
    list_int_create,
    list_int_destroy,
    list_int_append,
    list_int_prepend,
    list_int_size,
    list_int_clear,
    list_int_t,
    list_node_int_t,
    c_int
);
impl_list!(
    list_double_create,
    list_double_destroy,
    list_double_append,
    list_double_prepend,
    list_double_size,
    list_double_clear,
    list_double_t,
    list_node_double_t,
    c_double
);
impl_list!(
    list_item_t_create,
    list_item_t_destroy,
    list_item_t_append,
    list_item_t_prepend,
    list_item_t_size,
    list_item_t_clear,
    list_item_t_t,
    list_node_item_t_t,
    item_t
);
impl_list!(
    list_order_t_create,
    list_order_t_destroy,
    list_order_t_append,
    list_order_t_prepend,
    list_order_t_size,
    list_order_t_clear,
    list_order_t_t,
    list_node_order_t_t,
    order_t
);

// ---------------------------------------------------------------------------
// Inventory functions
// ---------------------------------------------------------------------------

/// Mirrors `strncpy(dst, src, n - 1); dst[n-1] = '\0';` for a fixed-length C
/// buffer that we treat as a C string.
unsafe fn copy_c_string(dst: *mut c_char, dst_len: usize, src: *const c_char) {
    if dst_len == 0 {
        return;
    }
    // strncpy semantics: copy up to (n-1) bytes, NOT including the terminating
    // NUL of the source if it is encountered earlier (it stops copying and
    // pads the rest of the destination with NUL bytes).
    libc::strncpy(dst, src, dst_len - 1);
    *dst.add(dst_len - 1) = 0;
}

#[no_mangle]
pub unsafe extern "C" fn print_item(item: item_t) {
    // Match the format strings from c_src/src/inventory.c byte-for-byte.
    libc::printf(b"  [%d] %s\n\0".as_ptr() as *const c_char, item.id, item.name.as_ptr());
    libc::printf(
        b"      Category: %s\n\0".as_ptr() as *const c_char,
        item.category.as_ptr(),
    );
    libc::printf(b"      Price: $%.2f\n\0".as_ptr() as *const c_char, item.price);
    libc::printf(b"      Quantity: %d\n\0".as_ptr() as *const c_char, item.quantity);
}

#[no_mangle]
pub unsafe extern "C" fn print_order(order: order_t) {
    libc::printf(
        b"  Order - Customer ID: %d, Name: %s\n\0".as_ptr() as *const c_char,
        order.customer_id,
        order.customer_name.as_ptr(),
    );
    libc::printf(
        b"          Total: $%.2f\n\0".as_ptr() as *const c_char,
        order.total_amount,
    );
}

#[no_mangle]
pub unsafe extern "C" fn create_item(
    id: c_int,
    name: *const c_char,
    category: *const c_char,
    price: c_double,
    quantity: c_int,
) -> item_t {
    let mut item = item_t {
        id,
        name: [0; MAX_NAME_LENGTH],
        category: [0; MAX_CATEGORY_LENGTH],
        price,
        quantity,
    };
    copy_c_string(item.name.as_mut_ptr(), MAX_NAME_LENGTH, name);
    copy_c_string(item.category.as_mut_ptr(), MAX_CATEGORY_LENGTH, category);
    item
}

#[no_mangle]
pub unsafe extern "C" fn create_order(
    customer_id: c_int,
    customer_name: *const c_char,
    total_amount: c_double,
) -> order_t {
    let mut order = order_t {
        customer_id,
        customer_name: [0; MAX_NAME_LENGTH],
        total_amount,
    };
    copy_c_string(
        order.customer_name.as_mut_ptr(),
        MAX_NAME_LENGTH,
        customer_name,
    );
    order
}

#[no_mangle]
pub unsafe extern "C" fn calculate_inventory_stats(items: *mut array_item_t_t) {
    if items.is_null() || (*items).size == 0 {
        libc::printf(b"No items in inventory\n\0".as_ptr() as *const c_char);
        return;
    }

    libc::printf(b"\n=== Inventory Statistics (Array) ===\n\0".as_ptr() as *const c_char);

    let mut total_value: c_double = 0.0;
    let mut total_items: c_int = 0;
    let mut max_price: c_double = 0.0;
    let mut min_price: c_double = (*(*items).data).price;

    let mut i: size_t = 0;
    while i < (*items).size {
        let item = *(*items).data.add(i);
        total_value += item.price * item.quantity as c_double;
        total_items = total_items.wrapping_add(item.quantity);
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
        i = i.wrapping_add(1);
    }

    libc::printf(
        b"Total unique items: %zu\n\0".as_ptr() as *const c_char,
        (*items).size,
    );
    libc::printf(
        b"Total item count: %d\n\0".as_ptr() as *const c_char,
        total_items,
    );
    libc::printf(
        b"Total inventory value: $%.2f\n\0".as_ptr() as *const c_char,
        total_value,
    );
    // C: total_value / total_items where total_items is int; the int gets
    // implicitly promoted to double for the division.
    libc::printf(
        b"Average item price: $%.2f\n\0".as_ptr() as *const c_char,
        total_value / total_items as c_double,
    );
    libc::printf(
        b"Most expensive item: $%.2f\n\0".as_ptr() as *const c_char,
        max_price,
    );
    libc::printf(
        b"Least expensive item: $%.2f\n\0".as_ptr() as *const c_char,
        min_price,
    );
}

#[no_mangle]
pub unsafe extern "C" fn calculate_order_stats(orders: *mut list_order_t_t) {
    if orders.is_null() || (*orders).size == 0 {
        libc::printf(b"No orders to analyze\n\0".as_ptr() as *const c_char);
        return;
    }

    libc::printf(b"\n=== Order Statistics (List) ===\n\0".as_ptr() as *const c_char);

    let mut total_revenue: c_double = 0.0;
    let mut max_order: c_double = 0.0;
    let mut min_order: c_double = -1.0;

    let mut node = (*orders).head;
    while !node.is_null() {
        let order = (*node).data;
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
        node = (*node).next;
    }

    libc::printf(
        b"Total orders: %zu\n\0".as_ptr() as *const c_char,
        (*orders).size,
    );
    libc::printf(
        b"Total revenue: $%.2f\n\0".as_ptr() as *const c_char,
        total_revenue,
    );
    libc::printf(
        b"Average order value: $%.2f\n\0".as_ptr() as *const c_char,
        total_revenue / (*orders).size as c_double,
    );
    libc::printf(
        b"Largest order: $%.2f\n\0".as_ptr() as *const c_char,
        max_order,
    );
    libc::printf(
        b"Smallest order: $%.2f\n\0".as_ptr() as *const c_char,
        min_order,
    );
}

#[no_mangle]
pub unsafe extern "C" fn find_items_by_category(
    items: *mut array_item_t_t,
    category: *const c_char,
) {
    if items.is_null() || category.is_null() {
        return;
    }

    libc::printf(
        b"\n=== Items in category '%s' ===\n\0".as_ptr() as *const c_char,
        category,
    );

    let mut found: c_int = 0;
    let mut i: size_t = 0;
    while i < (*items).size {
        let item = *(*items).data.add(i);
        if libc::strcmp(item.category.as_ptr(), category) == 0 {
            print_item(item);
            found += 1;
        }
        i = i.wrapping_add(1);
    }

    if found == 0 {
        libc::printf(b"No items found in this category\n\0".as_ptr() as *const c_char);
    } else {
        libc::printf(
            b"Found %d items\n\0".as_ptr() as *const c_char,
            found,
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn find_expensive_items(items: *mut list_item_t_t, min_price: c_double) {
    if items.is_null() {
        return;
    }

    libc::printf(
        b"\n=== Items priced above $%.2f ===\n\0".as_ptr() as *const c_char,
        min_price,
    );

    let mut found: c_int = 0;
    let mut node = (*items).head;
    while !node.is_null() {
        let item = (*node).data;
        if item.price >= min_price {
            print_item(item);
            found += 1;
        }
        node = (*node).next;
    }

    if found == 0 {
        libc::printf(b"No items found above this price\n\0".as_ptr() as *const c_char);
    } else {
        libc::printf(b"Found %d items\n\0".as_ptr() as *const c_char, found);
    }
}

