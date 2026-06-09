// Rust translation of c_src/ as a cdylib library.
// All public C functions retain their C ABI symbol names so consumers
// linking against this library see identical entry points.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_char;
use std::os::raw::{c_double, c_int, c_longlong, c_void};

// libc bindings used to faithfully reproduce printf/scanf/fgets behavior
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut c_void) -> *mut c_char;
    fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    static stdin: *mut c_void;
}

// =============================================================================
// Constants matching inventory.h
// =============================================================================
pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

// =============================================================================
// Struct layouts matching inventory.h exactly (C-compatible)
// =============================================================================

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

// =============================================================================
// Generic ARRAY container (DECLARE_ARRAY / DEFINE_ARRAY expansions)
// Each instantiation produces struct array_<TYPE>_t plus six functions.
// =============================================================================

macro_rules! define_array {
    ($struct_name:ident, $ty:ty,
     $create:ident, $destroy:ident, $push:ident,
     $get:ident, $size:ident, $clear:ident) => {
        #[repr(C)]
        pub struct $struct_name {
            pub data: *mut $ty,
            pub size: usize,
            pub capacity: usize,
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $create(initial_capacity: usize) -> *mut $struct_name {
            let arr = malloc(std::mem::size_of::<$struct_name>()) as *mut $struct_name;
            if arr.is_null() {
                return std::ptr::null_mut();
            }
            let cap = if initial_capacity > 0 { initial_capacity } else { 16 };
            (*arr).capacity = cap;
            (*arr).size = 0;
            let data = malloc(std::mem::size_of::<$ty>() * cap) as *mut $ty;
            if data.is_null() {
                free(arr as *mut c_void);
                return std::ptr::null_mut();
            }
            (*arr).data = data;
            arr
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $destroy(arr: *mut $struct_name) {
            if !arr.is_null() {
                free((*arr).data as *mut c_void);
                free(arr as *mut c_void);
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $push(arr: *mut $struct_name, value: $ty) -> c_int {
            if arr.is_null() {
                return -1;
            }
            if (*arr).size >= (*arr).capacity {
                let new_capacity = (*arr).capacity * 2;
                let new_data = realloc(
                    (*arr).data as *mut c_void,
                    std::mem::size_of::<$ty>() * new_capacity,
                ) as *mut $ty;
                if new_data.is_null() {
                    return -1;
                }
                (*arr).data = new_data;
                (*arr).capacity = new_capacity;
            }
            let idx = (*arr).size;
            std::ptr::write((*arr).data.add(idx), value);
            (*arr).size += 1;
            0
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $get(arr: *mut $struct_name, index: usize) -> $ty {
            std::ptr::read((*arr).data.add(index))
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $size(arr: *mut $struct_name) -> usize {
            if arr.is_null() {
                0
            } else {
                (*arr).size
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $clear(arr: *mut $struct_name) {
            if !arr.is_null() {
                (*arr).size = 0;
            }
        }
    };
}

// =============================================================================
// Generic LIST container (DECLARE_LIST / DEFINE_LIST expansions)
// =============================================================================

macro_rules! define_list {
    ($node_struct:ident, $list_struct:ident, $ty:ty,
     $create:ident, $destroy:ident, $append:ident,
     $prepend:ident, $size:ident, $clear:ident) => {
        #[repr(C)]
        pub struct $node_struct {
            pub data: $ty,
            pub next: *mut $node_struct,
        }

        #[repr(C)]
        pub struct $list_struct {
            pub head: *mut $node_struct,
            pub tail: *mut $node_struct,
            pub size: usize,
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $create() -> *mut $list_struct {
            let list = malloc(std::mem::size_of::<$list_struct>()) as *mut $list_struct;
            if list.is_null() {
                return std::ptr::null_mut();
            }
            (*list).head = std::ptr::null_mut();
            (*list).tail = std::ptr::null_mut();
            (*list).size = 0;
            list
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $destroy(list: *mut $list_struct) {
            if list.is_null() {
                return;
            }
            let mut current = (*list).head;
            while !current.is_null() {
                let next = (*current).next;
                free(current as *mut c_void);
                current = next;
            }
            free(list as *mut c_void);
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $append(list: *mut $list_struct, value: $ty) -> c_int {
            if list.is_null() {
                return -1;
            }
            let node = malloc(std::mem::size_of::<$node_struct>()) as *mut $node_struct;
            if node.is_null() {
                return -1;
            }
            std::ptr::write(&mut (*node).data, value);
            (*node).next = std::ptr::null_mut();
            if (*list).head.is_null() {
                (*list).head = node;
                (*list).tail = node;
            } else {
                (*(*list).tail).next = node;
                (*list).tail = node;
            }
            (*list).size += 1;
            0
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $prepend(list: *mut $list_struct, value: $ty) -> c_int {
            if list.is_null() {
                return -1;
            }
            let node = malloc(std::mem::size_of::<$node_struct>()) as *mut $node_struct;
            if node.is_null() {
                return -1;
            }
            std::ptr::write(&mut (*node).data, value);
            (*node).next = (*list).head;
            (*list).head = node;
            if (*list).tail.is_null() {
                (*list).tail = node;
            }
            (*list).size += 1;
            0
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $size(list: *mut $list_struct) -> usize {
            if list.is_null() {
                0
            } else {
                (*list).size
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $clear(list: *mut $list_struct) {
            if list.is_null() {
                return;
            }
            let mut current = (*list).head;
            while !current.is_null() {
                let next = (*current).next;
                free(current as *mut c_void);
                current = next;
            }
            (*list).head = std::ptr::null_mut();
            (*list).tail = std::ptr::null_mut();
            (*list).size = 0;
        }
    };
}

// =============================================================================
// Instantiate containers required by inventory.h
// =============================================================================

// arrays
define_array!(array_int_t, c_int,
    array_int_create, array_int_destroy, array_int_push,
    array_int_get, array_int_size, array_int_clear);

define_array!(array_double_t, c_double,
    array_double_create, array_double_destroy, array_double_push,
    array_double_get, array_double_size, array_double_clear);

define_array!(array_item_t_t, item_t,
    array_item_t_create, array_item_t_destroy, array_item_t_push,
    array_item_t_get, array_item_t_size, array_item_t_clear);

define_array!(array_order_t_t, order_t,
    array_order_t_create, array_order_t_destroy, array_order_t_push,
    array_order_t_get, array_order_t_size, array_order_t_clear);

// lists
define_list!(list_node_int_t, list_int_t, c_int,
    list_int_create, list_int_destroy, list_int_append,
    list_int_prepend, list_int_size, list_int_clear);

define_list!(list_node_double_t, list_double_t, c_double,
    list_double_create, list_double_destroy, list_double_append,
    list_double_prepend, list_double_size, list_double_clear);

define_list!(list_node_item_t_t, list_item_t_t, item_t,
    list_item_t_create, list_item_t_destroy, list_item_t_append,
    list_item_t_prepend, list_item_t_size, list_item_t_clear);

define_list!(list_node_order_t_t, list_order_t_t, order_t,
    list_order_t_create, list_order_t_destroy, list_order_t_append,
    list_order_t_prepend, list_order_t_size, list_order_t_clear);

// =============================================================================
// Inventory functions (from inventory.c)
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_item(item: item_t) {
    printf(
        b"  [%d] %s\n\0".as_ptr() as *const c_char,
        item.id,
        item.name.as_ptr(),
    );
    printf(
        b"      Category: %s\n\0".as_ptr() as *const c_char,
        item.category.as_ptr(),
    );
    printf(
        b"      Price: $%.2f\n\0".as_ptr() as *const c_char,
        item.price,
    );
    printf(
        b"      Quantity: %d\n\0".as_ptr() as *const c_char,
        item.quantity,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_order(order: order_t) {
    printf(
        b"  Order - Customer ID: %d, Name: %s\n\0".as_ptr() as *const c_char,
        order.customer_id,
        order.customer_name.as_ptr(),
    );
    printf(
        b"          Total: $%.2f\n\0".as_ptr() as *const c_char,
        order.total_amount,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_item(
    id: c_int,
    name: *const c_char,
    category: *const c_char,
    price: c_double,
    quantity: c_int,
) -> item_t {
    let mut item: item_t = std::mem::zeroed();
    item.id = id;
    strncpy(item.name.as_mut_ptr(), name, MAX_NAME_LENGTH - 1);
    item.name[MAX_NAME_LENGTH - 1] = 0;
    strncpy(item.category.as_mut_ptr(), category, MAX_CATEGORY_LENGTH - 1);
    item.category[MAX_CATEGORY_LENGTH - 1] = 0;
    item.price = price;
    item.quantity = quantity;
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_order(
    customer_id: c_int,
    customer_name: *const c_char,
    total_amount: c_double,
) -> order_t {
    let mut order: order_t = std::mem::zeroed();
    order.customer_id = customer_id;
    strncpy(order.customer_name.as_mut_ptr(), customer_name, MAX_NAME_LENGTH - 1);
    order.customer_name[MAX_NAME_LENGTH - 1] = 0;
    order.total_amount = total_amount;
    order
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_inventory_stats(items: *mut array_item_t_t) {
    if items.is_null() || (*items).size == 0 {
        printf(b"No items in inventory\n\0".as_ptr() as *const c_char);
        return;
    }

    printf(b"\n=== Inventory Statistics (Array) ===\n\0".as_ptr() as *const c_char);

    let mut total_value: c_double = 0.0;
    let mut total_items: c_int = 0;
    let mut max_price: c_double = 0.0;
    let mut min_price: c_double = (*(*items).data.add(0)).price;

    // ARRAY_FOREACH equivalent
    let n = (*items).size;
    for i in 0..n {
        let item = std::ptr::read((*items).data.add(i));
        total_value += item.price * item.quantity as c_double;
        total_items += item.quantity;
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    printf(
        b"Total unique items: %zu\n\0".as_ptr() as *const c_char,
        (*items).size,
    );
    printf(b"Total item count: %d\n\0".as_ptr() as *const c_char, total_items);
    printf(
        b"Total inventory value: $%.2f\n\0".as_ptr() as *const c_char,
        total_value,
    );
    printf(
        b"Average item price: $%.2f\n\0".as_ptr() as *const c_char,
        total_value / total_items as c_double,
    );
    printf(
        b"Most expensive item: $%.2f\n\0".as_ptr() as *const c_char,
        max_price,
    );
    printf(
        b"Least expensive item: $%.2f\n\0".as_ptr() as *const c_char,
        min_price,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_order_stats(orders: *mut list_order_t_t) {
    if orders.is_null() || (*orders).size == 0 {
        printf(b"No orders to analyze\n\0".as_ptr() as *const c_char);
        return;
    }

    printf(b"\n=== Order Statistics (List) ===\n\0".as_ptr() as *const c_char);

    let mut total_revenue: c_double = 0.0;
    let mut max_order: c_double = 0.0;
    let mut min_order: c_double = -1.0;

    // LIST_FOREACH equivalent
    let mut node = (*orders).head;
    while !node.is_null() {
        let order = std::ptr::read(&(*node).data);
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
        node = (*node).next;
    }

    printf(
        b"Total orders: %zu\n\0".as_ptr() as *const c_char,
        (*orders).size,
    );
    printf(
        b"Total revenue: $%.2f\n\0".as_ptr() as *const c_char,
        total_revenue,
    );
    printf(
        b"Average order value: $%.2f\n\0".as_ptr() as *const c_char,
        total_revenue / (*orders).size as c_double,
    );
    printf(
        b"Largest order: $%.2f\n\0".as_ptr() as *const c_char,
        max_order,
    );
    printf(
        b"Smallest order: $%.2f\n\0".as_ptr() as *const c_char,
        min_order,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_items_by_category(
    items: *mut array_item_t_t,
    category: *const c_char,
) {
    if items.is_null() || category.is_null() {
        return;
    }

    printf(
        b"\n=== Items in category '%s' ===\n\0".as_ptr() as *const c_char,
        category,
    );

    let mut found: c_int = 0;
    let n = (*items).size;
    for i in 0..n {
        let item = std::ptr::read((*items).data.add(i));
        if strcmp(item.category.as_ptr(), category) == 0 {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        printf(b"No items found in this category\n\0".as_ptr() as *const c_char);
    } else {
        printf(b"Found %d items\n\0".as_ptr() as *const c_char, found);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_expensive_items(
    items: *mut list_item_t_t,
    min_price: c_double,
) {
    if items.is_null() {
        return;
    }

    printf(
        b"\n=== Items priced above $%.2f ===\n\0".as_ptr() as *const c_char,
        min_price,
    );

    let mut found: c_int = 0;
    let mut node = (*items).head;
    while !node.is_null() {
        let item = std::ptr::read(&(*node).data);
        if item.price >= min_price {
            print_item(item);
            found += 1;
        }
        node = (*node).next;
    }

    if found == 0 {
        printf(b"No items found above this price\n\0".as_ptr() as *const c_char);
    } else {
        printf(b"Found %d items\n\0".as_ptr() as *const c_char, found);
    }
}

// =============================================================================
// Top-level demo functions and main (translated from main.c)
// These are exported so a host can call them just like the C executable.
// =============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_menu() {
    printf(b"\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);
    printf(b"  GENERIC FOR_EACH MACRO DEMO\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);
    printf(b"1. Demo: Integer Containers\n\0".as_ptr() as *const c_char);
    printf(b"2. Demo: Double Containers\n\0".as_ptr() as *const c_char);
    printf(b"3. Demo: Inventory Array\n\0".as_ptr() as *const c_char);
    printf(b"4. Demo: Order List\n\0".as_ptr() as *const c_char);
    printf(b"5. Demo: Mixed Operations\n\0".as_ptr() as *const c_char);
    printf(b"6. Run All Demos\n\0".as_ptr() as *const c_char);
    printf(b"7. Exit\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);
    printf(b"Choice: \0".as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn demo_integer_containers() {
    printf(b"\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);
    printf(b"  DEMO 1: Integer Containers\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);

    // Integer array
    let int_array = array_int_create(10);
    printf(b"\n--- Integer Array ---\n\0".as_ptr() as *const c_char);
    printf(b"Adding integers: 10, 20, 30, 40, 50\n\0".as_ptr() as *const c_char);
    array_int_push(int_array, 10);
    array_int_push(int_array, 20);
    array_int_push(int_array, 30);
    array_int_push(int_array, 40);
    array_int_push(int_array, 50);

    printf(b"Array contents: \0".as_ptr() as *const c_char);
    let n = (*int_array).size;
    for i in 0..n {
        let val = *(*int_array).data.add(i);
        printf(b"%d \0".as_ptr() as *const c_char, val);
    }
    printf(b"\n\0".as_ptr() as *const c_char);

    let mut sum: c_int = 0;
    for i in 0..n {
        let val = *(*int_array).data.add(i);
        sum += val;
    }
    printf(b"Sum: %d\n\0".as_ptr() as *const c_char, sum);
    printf(
        b"Average: %.2f\n\0".as_ptr() as *const c_char,
        sum as c_double / (*int_array).size as c_double,
    );

    // Integer list
    let int_list = list_int_create();
    printf(b"\n--- Integer List ---\n\0".as_ptr() as *const c_char);
    printf(b"Adding integers: 100, 200, 300, 400, 500\n\0".as_ptr() as *const c_char);
    list_int_append(int_list, 100);
    list_int_append(int_list, 200);
    list_int_append(int_list, 300);
    list_int_append(int_list, 400);
    list_int_append(int_list, 500);

    printf(b"List contents: \0".as_ptr() as *const c_char);
    let mut node = (*int_list).head;
    while !node.is_null() {
        let val = (*node).data;
        printf(b"%d \0".as_ptr() as *const c_char, val);
        node = (*node).next;
    }
    printf(b"\n\0".as_ptr() as *const c_char);

    let mut product: c_longlong = 1;
    let mut node = (*int_list).head;
    while !node.is_null() {
        let val = (*node).data;
        product *= val as c_longlong;
        node = (*node).next;
    }
    printf(b"Product: %lld\n\0".as_ptr() as *const c_char, product);

    array_int_destroy(int_array);
    list_int_destroy(int_list);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn demo_double_containers() {
    printf(b"\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);
    printf(b"  DEMO 2: Double Containers\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);

    let double_array = array_double_create(5);
    printf(
        b"\n--- Double Array (Temperatures in Celsius) ---\n\0".as_ptr() as *const c_char,
    );

    let temps: [c_double; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    let num_temps: c_int = temps.len() as c_int;

    printf(b"Adding temperatures: \0".as_ptr() as *const c_char);
    for i in 0..num_temps as usize {
        array_double_push(double_array, temps[i]);
        printf(b"%.1f \0".as_ptr() as *const c_char, temps[i]);
    }
    printf(b"\n\0".as_ptr() as *const c_char);

    let mut min_temp: c_double = temps[0];
    let mut max_temp: c_double = temps[0];
    let mut sum_temp: c_double = 0.0;

    let n = (*double_array).size;
    for i in 0..n {
        let temp = *(*double_array).data.add(i);
        if temp < min_temp {
            min_temp = temp;
        }
        if temp > max_temp {
            max_temp = temp;
        }
        sum_temp += temp;
    }

    printf(
        b"Minimum: %.1f\xc2\xb0C\n\0".as_ptr() as *const c_char,
        min_temp,
    );
    printf(
        b"Maximum: %.1f\xc2\xb0C\n\0".as_ptr() as *const c_char,
        max_temp,
    );
    printf(
        b"Average: %.1f\xc2\xb0C\n\0".as_ptr() as *const c_char,
        sum_temp / (*double_array).size as c_double,
    );

    let price_list = list_double_create();
    printf(b"\n--- Double List (Product Prices) ---\n\0".as_ptr() as *const c_char);

    let prices: [c_double; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    let num_prices: c_int = prices.len() as c_int;

    printf(b"Adding prices: \0".as_ptr() as *const c_char);
    for i in 0..num_prices as usize {
        list_double_append(price_list, prices[i]);
        printf(b"$%.2f \0".as_ptr() as *const c_char, prices[i]);
    }
    printf(b"\n\0".as_ptr() as *const c_char);

    let mut total: c_double = 0.0;
    let mut count_under_10: c_int = 0;

    let mut node = (*price_list).head;
    while !node.is_null() {
        let temp = (*node).data;
        total += temp;
        if temp < 10.0 {
            count_under_10 += 1;
        }
        node = (*node).next;
    }

    printf(b"Total cost: $%.2f\n\0".as_ptr() as *const c_char, total);
    printf(
        b"Items under $10: %d\n\0".as_ptr() as *const c_char,
        count_under_10,
    );

    array_double_destroy(double_array);
    list_double_destroy(price_list);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn demo_inventory_array() {
    printf(b"\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);
    printf(b"  DEMO 3: Inventory Array (Items)\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);

    let inventory = array_item_t_create(20);

    printf(b"\n--- Adding Items to Inventory ---\n\0".as_ptr() as *const c_char);
    array_item_t_push(
        inventory,
        create_item(
            1,
            b"Laptop\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            899.99,
            15,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            2,
            b"Mouse\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            24.99,
            50,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            3,
            b"Keyboard\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            79.99,
            30,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            4,
            b"Monitor\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            299.99,
            20,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            5,
            b"Desk Chair\0".as_ptr() as *const c_char,
            b"Furniture\0".as_ptr() as *const c_char,
            199.99,
            10,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            6,
            b"Desk\0".as_ptr() as *const c_char,
            b"Furniture\0".as_ptr() as *const c_char,
            349.99,
            8,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            7,
            b"Notebook\0".as_ptr() as *const c_char,
            b"Office\0".as_ptr() as *const c_char,
            4.99,
            100,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            8,
            b"Pen Set\0".as_ptr() as *const c_char,
            b"Office\0".as_ptr() as *const c_char,
            12.99,
            75,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            9,
            b"USB Cable\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            9.99,
            60,
        ),
    );
    array_item_t_push(
        inventory,
        create_item(
            10,
            b"Bookshelf\0".as_ptr() as *const c_char,
            b"Furniture\0".as_ptr() as *const c_char,
            149.99,
            12,
        ),
    );

    printf(
        b"Added %zu items to inventory\n\0".as_ptr() as *const c_char,
        (*inventory).size,
    );

    printf(b"\n--- All Inventory Items ---\n\0".as_ptr() as *const c_char);
    let n = (*inventory).size;
    for i in 0..n {
        let item = std::ptr::read((*inventory).data.add(i));
        print_item(item);
        printf(b"\n\0".as_ptr() as *const c_char);
    }

    calculate_inventory_stats(inventory);

    find_items_by_category(inventory, b"Electronics\0".as_ptr() as *const c_char);
    find_items_by_category(inventory, b"Furniture\0".as_ptr() as *const c_char);

    printf(b"\n--- Low Stock Items (< 20) ---\n\0".as_ptr() as *const c_char);
    let mut low_stock_count: c_int = 0;
    let n = (*inventory).size;
    for i in 0..n {
        let item = std::ptr::read((*inventory).data.add(i));
        if item.quantity < 20 {
            print_item(item);
            low_stock_count += 1;
        }
    }
    printf(
        b"Total low stock items: %d\n\0".as_ptr() as *const c_char,
        low_stock_count,
    );

    array_item_t_destroy(inventory);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn demo_order_list() {
    printf(b"\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);
    printf(b"  DEMO 4: Order List (Orders)\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);

    let orders = list_order_t_create();

    printf(b"\n--- Adding Orders ---\n\0".as_ptr() as *const c_char);
    list_order_t_append(
        orders,
        create_order(1001, b"Alice Johnson\0".as_ptr() as *const c_char, 1249.95),
    );
    list_order_t_append(
        orders,
        create_order(1002, b"Bob Smith\0".as_ptr() as *const c_char, 89.99),
    );
    list_order_t_append(
        orders,
        create_order(1003, b"Carol White\0".as_ptr() as *const c_char, 549.98),
    );
    list_order_t_append(
        orders,
        create_order(1004, b"David Brown\0".as_ptr() as *const c_char, 24.99),
    );
    list_order_t_append(
        orders,
        create_order(1005, b"Eve Davis\0".as_ptr() as *const c_char, 899.99),
    );
    list_order_t_append(
        orders,
        create_order(1006, b"Frank Miller\0".as_ptr() as *const c_char, 374.97),
    );
    list_order_t_append(
        orders,
        create_order(1007, b"Grace Lee\0".as_ptr() as *const c_char, 159.98),
    );
    list_order_t_append(
        orders,
        create_order(1008, b"Henry Wilson\0".as_ptr() as *const c_char, 1099.99),
    );

    printf(
        b"Added %zu orders\n\0".as_ptr() as *const c_char,
        (*orders).size,
    );

    printf(b"\n--- All Orders ---\n\0".as_ptr() as *const c_char);
    let mut node = (*orders).head;
    while !node.is_null() {
        let order = std::ptr::read(&(*node).data);
        print_order(order);
        node = (*node).next;
    }

    calculate_order_stats(orders);

    printf(b"\n--- Large Orders (> $500) ---\n\0".as_ptr() as *const c_char);
    let mut large_order_count: c_int = 0;
    let mut large_order_total: c_double = 0.0;

    let mut node = (*orders).head;
    while !node.is_null() {
        let order = std::ptr::read(&(*node).data);
        if order.total_amount > 500.0 {
            print_order(order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
        node = (*node).next;
    }

    printf(
        b"Total large orders: %d\n\0".as_ptr() as *const c_char,
        large_order_count,
    );
    printf(
        b"Revenue from large orders: $%.2f\n\0".as_ptr() as *const c_char,
        large_order_total,
    );

    list_order_t_destroy(orders);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn demo_mixed_operations() {
    printf(b"\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);
    printf(b"  DEMO 5: Mixed Operations\n\0".as_ptr() as *const c_char);
    printf(b"========================================\n\0".as_ptr() as *const c_char);

    let array_inventory = array_item_t_create(10);
    let list_inventory = list_item_t_create();

    printf(b"\n--- Populating both Array and List ---\n\0".as_ptr() as *const c_char);

    let items: [item_t; 5] = [
        create_item(
            1,
            b"Smartphone\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            699.99,
            25,
        ),
        create_item(
            2,
            b"Tablet\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            449.99,
            18,
        ),
        create_item(
            3,
            b"Headphones\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            149.99,
            40,
        ),
        create_item(
            4,
            b"Smart Watch\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            299.99,
            22,
        ),
        create_item(
            5,
            b"Power Bank\0".as_ptr() as *const c_char,
            b"Electronics\0".as_ptr() as *const c_char,
            39.99,
            55,
        ),
    ];

    let num_items: c_int = items.len() as c_int;

    for i in 0..num_items as usize {
        array_item_t_push(array_inventory, items[i]);
        list_item_t_append(list_inventory, items[i]);
    }

    printf(
        b"Added %d items to both containers\n\0".as_ptr() as *const c_char,
        num_items,
    );

    printf(b"\n--- Iterating through Array ---\n\0".as_ptr() as *const c_char);
    let mut array_count: c_int = 0;
    let n = (*array_inventory).size;
    for _i in 0..n {
        array_count += 1;
    }
    printf(
        b"Array iteration count: %d\n\0".as_ptr() as *const c_char,
        array_count,
    );

    printf(b"\n--- Iterating through List ---\n\0".as_ptr() as *const c_char);
    let mut list_count: c_int = 0;
    let mut node = (*list_inventory).head;
    while !node.is_null() {
        list_count += 1;
        node = (*node).next;
    }
    printf(
        b"List iteration count: %d\n\0".as_ptr() as *const c_char,
        list_count,
    );

    let price_threshold: c_double = 200.0;

    printf(
        b"\n--- Items above $%.2f (Array) ---\n\0".as_ptr() as *const c_char,
        price_threshold,
    );
    let n = (*array_inventory).size;
    for i in 0..n {
        let item = std::ptr::read((*array_inventory).data.add(i));
        if item.price >= price_threshold {
            printf(
                b"  %s: $%.2f\n\0".as_ptr() as *const c_char,
                item.name.as_ptr(),
                item.price,
            );
        }
    }

    printf(
        b"\n--- Items above $%.2f (List) ---\n\0".as_ptr() as *const c_char,
        price_threshold,
    );
    let mut node = (*list_inventory).head;
    while !node.is_null() {
        let item = std::ptr::read(&(*node).data);
        if item.price >= price_threshold {
            printf(
                b"  %s: $%.2f\n\0".as_ptr() as *const c_char,
                item.name.as_ptr(),
                item.price,
            );
        }
        node = (*node).next;
    }

    array_item_t_destroy(array_inventory);
    list_item_t_destroy(list_inventory);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    printf(
        b"\xe2\x95\x94\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x97\n\0".as_ptr() as *const c_char,
    );
    printf(
        b"\xe2\x95\x91   GENERIC FOR_EACH MACRO DEMO         \xe2\x95\x91\n\0".as_ptr()
            as *const c_char,
    );
    printf(
        b"\xe2\x95\x91   Demonstrating Generic Containers    \xe2\x95\x91\n\0".as_ptr()
            as *const c_char,
    );
    printf(
        b"\xe2\x95\x9a\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x90\xe2\x95\x9d\n\0".as_ptr() as *const c_char,
    );

    let mut input = [0i8; 256];
    let mut choice: c_int = 0;

    loop {
        print_menu();

        if fgets(input.as_mut_ptr() as *mut c_char, 256, stdin).is_null() {
            break;
        }

        if sscanf(
            input.as_ptr() as *const c_char,
            b"%d\0".as_ptr() as *const c_char,
            &mut choice as *mut c_int,
        ) != 1
        {
            printf(b"Invalid input\n\0".as_ptr() as *const c_char);
            continue;
        }

        match choice {
            1 => demo_integer_containers(),
            2 => demo_double_containers(),
            3 => demo_inventory_array(),
            4 => demo_order_list(),
            5 => demo_mixed_operations(),
            6 => {
                printf(b"\n=== Running All Demos ===\n\0".as_ptr() as *const c_char);
                demo_integer_containers();
                demo_double_containers();
                demo_inventory_array();
                demo_order_list();
                demo_mixed_operations();
                printf(b"\n========================================\n\0".as_ptr() as *const c_char);
                printf(b"  All demos completed successfully!\n\0".as_ptr() as *const c_char);
                printf(b"========================================\n\0".as_ptr() as *const c_char);
            }
            7 => {
                printf(b"\nGoodbye!\n\0".as_ptr() as *const c_char);
                return 0;
            }
            _ => {
                printf(b"Invalid choice\n\0".as_ptr() as *const c_char);
            }
        }
    }

    0
}

