use std::ffi::{c_char, c_double, c_int, c_void};
use std::mem::{size_of, MaybeUninit};
use std::ptr;

const DEFAULT_CAPACITY: usize = 16;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn printf(format: *const c_char, ...) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strncpy(destination: *mut c_char, source: *const c_char, count: usize) -> *mut c_char;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Item {
    pub id: c_int,
    pub name: [c_char; 64],
    pub category: [c_char; 32],
    pub price: c_double,
    pub quantity: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Order {
    pub customer_id: c_int,
    pub customer_name: [c_char; 64],
    pub total_amount: c_double,
}

macro_rules! define_array {
    (
        $array:ident, $value:ty,
        $create:ident, $destroy:ident, $push:ident,
        $get:ident, $size:ident, $clear:ident
    ) => {
        #[repr(C)]
        pub struct $array {
            pub data: *mut $value,
            pub size: usize,
            pub capacity: usize,
        }

        #[no_mangle]
        pub unsafe extern "C" fn $create(initial_capacity: usize) -> *mut $array {
            let array = unsafe { malloc(size_of::<$array>()) }.cast::<$array>();
            if array.is_null() {
                return ptr::null_mut();
            }

            let capacity = if initial_capacity > 0 {
                initial_capacity
            } else {
                DEFAULT_CAPACITY
            };
            let bytes = size_of::<$value>().wrapping_mul(capacity);
            let data = unsafe { malloc(bytes) }.cast::<$value>();
            if data.is_null() {
                unsafe { free(array.cast()) };
                return ptr::null_mut();
            }

            unsafe {
                array.write($array {
                    data,
                    size: 0,
                    capacity,
                });
            }
            array
        }

        #[no_mangle]
        pub unsafe extern "C" fn $destroy(array: *mut $array) {
            if !array.is_null() {
                unsafe {
                    free((*array).data.cast());
                    free(array.cast());
                }
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $push(array: *mut $array, value: $value) -> c_int {
            if array.is_null() {
                return -1;
            }

            unsafe {
                if (*array).size >= (*array).capacity {
                    let new_capacity = (*array).capacity.wrapping_mul(2);
                    let bytes = size_of::<$value>().wrapping_mul(new_capacity);
                    let new_data = realloc((*array).data.cast(), bytes).cast::<$value>();
                    if new_data.is_null() {
                        return -1;
                    }
                    (*array).data = new_data;
                    (*array).capacity = new_capacity;
                }

                (*array).data.add((*array).size).write(value);
                (*array).size = (*array).size.wrapping_add(1);
            }
            0
        }

        #[no_mangle]
        pub unsafe extern "C" fn $get(array: *mut $array, index: usize) -> $value {
            unsafe { (*array).data.add(index).read() }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $size(array: *mut $array) -> usize {
            if array.is_null() {
                0
            } else {
                unsafe { (*array).size }
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $clear(array: *mut $array) {
            if !array.is_null() {
                unsafe { (*array).size = 0 };
            }
        }
    };
}

define_array!(
    ArrayInt,
    c_int,
    array_int_create,
    array_int_destroy,
    array_int_push,
    array_int_get,
    array_int_size,
    array_int_clear
);
define_array!(
    ArrayDouble,
    c_double,
    array_double_create,
    array_double_destroy,
    array_double_push,
    array_double_get,
    array_double_size,
    array_double_clear
);
define_array!(
    ArrayItem,
    Item,
    array_item_t_create,
    array_item_t_destroy,
    array_item_t_push,
    array_item_t_get,
    array_item_t_size,
    array_item_t_clear
);
define_array!(
    ArrayOrder,
    Order,
    array_order_t_create,
    array_order_t_destroy,
    array_order_t_push,
    array_order_t_get,
    array_order_t_size,
    array_order_t_clear
);

macro_rules! define_list {
    (
        $node:ident, $list:ident, $value:ty,
        $create:ident, $destroy:ident, $append:ident,
        $prepend:ident, $size:ident, $clear:ident
    ) => {
        #[repr(C)]
        pub struct $node {
            pub data: $value,
            pub next: *mut $node,
        }

        #[repr(C)]
        pub struct $list {
            pub head: *mut $node,
            pub tail: *mut $node,
            pub size: usize,
        }

        #[no_mangle]
        pub unsafe extern "C" fn $create() -> *mut $list {
            let list = unsafe { malloc(size_of::<$list>()) }.cast::<$list>();
            if list.is_null() {
                return ptr::null_mut();
            }
            unsafe {
                list.write($list {
                    head: ptr::null_mut(),
                    tail: ptr::null_mut(),
                    size: 0,
                });
            }
            list
        }

        #[no_mangle]
        pub unsafe extern "C" fn $destroy(list: *mut $list) {
            if list.is_null() {
                return;
            }
            unsafe {
                let mut current = (*list).head;
                while !current.is_null() {
                    let next = (*current).next;
                    free(current.cast());
                    current = next;
                }
                free(list.cast());
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $append(list: *mut $list, value: $value) -> c_int {
            if list.is_null() {
                return -1;
            }
            let node = unsafe { malloc(size_of::<$node>()) }.cast::<$node>();
            if node.is_null() {
                return -1;
            }

            unsafe {
                node.write($node {
                    data: value,
                    next: ptr::null_mut(),
                });
                if (*list).head.is_null() {
                    (*list).head = node;
                    (*list).tail = node;
                } else {
                    (*(*list).tail).next = node;
                    (*list).tail = node;
                }
                (*list).size = (*list).size.wrapping_add(1);
            }
            0
        }

        #[no_mangle]
        pub unsafe extern "C" fn $prepend(list: *mut $list, value: $value) -> c_int {
            if list.is_null() {
                return -1;
            }
            let node = unsafe { malloc(size_of::<$node>()) }.cast::<$node>();
            if node.is_null() {
                return -1;
            }

            unsafe {
                node.write($node {
                    data: value,
                    next: (*list).head,
                });
                (*list).head = node;
                if (*list).tail.is_null() {
                    (*list).tail = node;
                }
                (*list).size = (*list).size.wrapping_add(1);
            }
            0
        }

        #[no_mangle]
        pub unsafe extern "C" fn $size(list: *mut $list) -> usize {
            if list.is_null() {
                0
            } else {
                unsafe { (*list).size }
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn $clear(list: *mut $list) {
            if list.is_null() {
                return;
            }
            unsafe {
                let mut current = (*list).head;
                while !current.is_null() {
                    let next = (*current).next;
                    free(current.cast());
                    current = next;
                }
                (*list).head = ptr::null_mut();
                (*list).tail = ptr::null_mut();
                (*list).size = 0;
            }
        }
    };
}

define_list!(
    ListNodeInt,
    ListInt,
    c_int,
    list_int_create,
    list_int_destroy,
    list_int_append,
    list_int_prepend,
    list_int_size,
    list_int_clear
);
define_list!(
    ListNodeDouble,
    ListDouble,
    c_double,
    list_double_create,
    list_double_destroy,
    list_double_append,
    list_double_prepend,
    list_double_size,
    list_double_clear
);
define_list!(
    ListNodeItem,
    ListItem,
    Item,
    list_item_t_create,
    list_item_t_destroy,
    list_item_t_append,
    list_item_t_prepend,
    list_item_t_size,
    list_item_t_clear
);
define_list!(
    ListNodeOrder,
    ListOrder,
    Order,
    list_order_t_create,
    list_order_t_destroy,
    list_order_t_append,
    list_order_t_prepend,
    list_order_t_size,
    list_order_t_clear
);

#[no_mangle]
pub unsafe extern "C" fn print_item(item: Item) {
    unsafe {
        printf(
            b"  [%d] %s\n\0".as_ptr().cast(),
            item.id,
            item.name.as_ptr(),
        );
        printf(
            b"      Category: %s\n\0".as_ptr().cast(),
            item.category.as_ptr(),
        );
        printf(b"      Price: $%.2f\n\0".as_ptr().cast(), item.price);
        printf(b"      Quantity: %d\n\0".as_ptr().cast(), item.quantity);
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_order(order: Order) {
    unsafe {
        printf(
            b"  Order - Customer ID: %d, Name: %s\n\0".as_ptr().cast(),
            order.customer_id,
            order.customer_name.as_ptr(),
        );
        printf(
            b"          Total: $%.2f\n\0".as_ptr().cast(),
            order.total_amount,
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn create_item(
    id: c_int,
    name: *const c_char,
    category: *const c_char,
    price: c_double,
    quantity: c_int,
) -> Item {
    let mut item = MaybeUninit::<Item>::uninit();
    let item_pointer = item.as_mut_ptr();
    unsafe {
        ptr::addr_of_mut!((*item_pointer).id).write(id);
        strncpy(ptr::addr_of_mut!((*item_pointer).name).cast(), name, 63);
        (*item_pointer).name[63] = 0;
        strncpy(
            ptr::addr_of_mut!((*item_pointer).category).cast(),
            category,
            31,
        );
        (*item_pointer).category[31] = 0;
        ptr::addr_of_mut!((*item_pointer).price).write(price);
        ptr::addr_of_mut!((*item_pointer).quantity).write(quantity);
        item.assume_init()
    }
}

#[no_mangle]
pub unsafe extern "C" fn create_order(
    customer_id: c_int,
    customer_name: *const c_char,
    total_amount: c_double,
) -> Order {
    let mut order = MaybeUninit::<Order>::uninit();
    let order_pointer = order.as_mut_ptr();
    unsafe {
        ptr::addr_of_mut!((*order_pointer).customer_id).write(customer_id);
        strncpy(
            ptr::addr_of_mut!((*order_pointer).customer_name).cast(),
            customer_name,
            63,
        );
        (*order_pointer).customer_name[63] = 0;
        ptr::addr_of_mut!((*order_pointer).total_amount).write(total_amount);
        order.assume_init()
    }
}

#[no_mangle]
pub unsafe extern "C" fn calculate_inventory_stats(items: *mut ArrayItem) {
    if items.is_null() || unsafe { (*items).size == 0 } {
        unsafe { printf(b"No items in inventory\n\0".as_ptr().cast()) };
        return;
    }

    unsafe {
        printf(
            b"\n=== Inventory Statistics (Array) ===\n\0"
                .as_ptr()
                .cast(),
        )
    };
    let mut total_value = 0.0;
    let mut total_items: c_int = 0;
    let mut max_price = 0.0;
    let mut min_price = unsafe { (*(*items).data).price };

    for index in 0..unsafe { (*items).size } {
        let item = unsafe { (*items).data.add(index).read() };
        total_value += item.price * f64::from(item.quantity);
        total_items = total_items.wrapping_add(item.quantity);
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    unsafe {
        printf(
            b"Total unique items: %zu\n\0".as_ptr().cast(),
            (*items).size,
        );
        printf(b"Total item count: %d\n\0".as_ptr().cast(), total_items);
        printf(
            b"Total inventory value: $%.2f\n\0".as_ptr().cast(),
            total_value,
        );
        printf(
            b"Average item price: $%.2f\n\0".as_ptr().cast(),
            total_value / f64::from(total_items),
        );
        printf(b"Most expensive item: $%.2f\n\0".as_ptr().cast(), max_price);
        printf(
            b"Least expensive item: $%.2f\n\0".as_ptr().cast(),
            min_price,
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn calculate_order_stats(orders: *mut ListOrder) {
    if orders.is_null() || unsafe { (*orders).size == 0 } {
        unsafe { printf(b"No orders to analyze\n\0".as_ptr().cast()) };
        return;
    }

    unsafe { printf(b"\n=== Order Statistics (List) ===\n\0".as_ptr().cast()) };
    let mut total_revenue = 0.0;
    let mut max_order = 0.0;
    let mut min_order = -1.0;
    let mut node = unsafe { (*orders).head };

    while !node.is_null() {
        let order = unsafe { (*node).data };
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
        node = unsafe { (*node).next };
    }

    unsafe {
        printf(b"Total orders: %zu\n\0".as_ptr().cast(), (*orders).size);
        printf(b"Total revenue: $%.2f\n\0".as_ptr().cast(), total_revenue);
        printf(
            b"Average order value: $%.2f\n\0".as_ptr().cast(),
            total_revenue / (*orders).size as f64,
        );
        printf(b"Largest order: $%.2f\n\0".as_ptr().cast(), max_order);
        printf(b"Smallest order: $%.2f\n\0".as_ptr().cast(), min_order);
    }
}

#[no_mangle]
pub unsafe extern "C" fn find_items_by_category(items: *mut ArrayItem, category: *const c_char) {
    if items.is_null() || category.is_null() {
        return;
    }

    unsafe {
        printf(
            b"\n=== Items in category '%s' ===\n\0".as_ptr().cast(),
            category,
        );
    }
    let mut found: c_int = 0;
    for index in 0..unsafe { (*items).size } {
        let item = unsafe { (*items).data.add(index).read() };
        if unsafe { strcmp(item.category.as_ptr(), category) == 0 } {
            unsafe { print_item(item) };
            found = found.wrapping_add(1);
        }
    }

    if found == 0 {
        unsafe { printf(b"No items found in this category\n\0".as_ptr().cast()) };
    } else {
        unsafe { printf(b"Found %d items\n\0".as_ptr().cast(), found) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn find_expensive_items(items: *mut ListItem, min_price: c_double) {
    if items.is_null() {
        return;
    }

    unsafe {
        printf(
            b"\n=== Items priced above $%.2f ===\n\0".as_ptr().cast(),
            min_price,
        );
    }
    let mut found: c_int = 0;
    let mut node = unsafe { (*items).head };
    while !node.is_null() {
        let item = unsafe { (*node).data };
        if item.price >= min_price {
            unsafe { print_item(item) };
            found = found.wrapping_add(1);
        }
        node = unsafe { (*node).next };
    }

    if found == 0 {
        unsafe { printf(b"No items found above this price\n\0".as_ptr().cast()) };
    } else {
        unsafe { printf(b"Found %d items\n\0".as_ptr().cast(), found) };
    }
}
