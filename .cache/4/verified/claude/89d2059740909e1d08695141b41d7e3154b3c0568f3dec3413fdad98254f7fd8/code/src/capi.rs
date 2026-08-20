/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
//! capi.rs -- the C ABI surface of the library.
//!
//! One `#[no_mangle] extern "C"` entry point per symbol exported by the C
//! build, including the ones the preprocessor generates from `DEFINE_ARRAY` /
//! `DEFINE_LIST`, and `main` itself.

#![allow(clippy::missing_safety_doc)]

use std::io::Write;

use libc::{c_char, c_int};

use crate::cio::CStdout;
use crate::demos;
use crate::generic_containers as gc;
use crate::generic_containers::{ArrayT, ListT};
use crate::inventory as inv;
use crate::inventory::{ItemT, OrderT};

// ============================================================================
// DEFINE_ARRAY(TYPE) instantiations
// ============================================================================

macro_rules! define_array {
    ($ty:ty, $create:ident, $destroy:ident, $push:ident, $get:ident, $size:ident, $clear:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $create(initial_capacity: usize) -> *mut ArrayT<$ty> {
            gc::array_create::<$ty>(initial_capacity)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $destroy(arr: *mut ArrayT<$ty>) {
            gc::array_destroy::<$ty>(arr)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $push(arr: *mut ArrayT<$ty>, value: $ty) -> c_int {
            gc::array_push::<$ty>(arr, value)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $get(arr: *mut ArrayT<$ty>, index: usize) -> $ty {
            gc::array_get::<$ty>(arr, index)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $size(arr: *mut ArrayT<$ty>) -> usize {
            gc::array_size::<$ty>(arr)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $clear(arr: *mut ArrayT<$ty>) {
            gc::array_clear::<$ty>(arr)
        }
    };
}

define_array!(
    c_int,
    array_int_create,
    array_int_destroy,
    array_int_push,
    array_int_get,
    array_int_size,
    array_int_clear
);
define_array!(
    f64,
    array_double_create,
    array_double_destroy,
    array_double_push,
    array_double_get,
    array_double_size,
    array_double_clear
);
define_array!(
    ItemT,
    array_item_t_create,
    array_item_t_destroy,
    array_item_t_push,
    array_item_t_get,
    array_item_t_size,
    array_item_t_clear
);
define_array!(
    OrderT,
    array_order_t_create,
    array_order_t_destroy,
    array_order_t_push,
    array_order_t_get,
    array_order_t_size,
    array_order_t_clear
);

// ============================================================================
// DEFINE_LIST(TYPE) instantiations
// ============================================================================

macro_rules! define_list {
    ($ty:ty, $create:ident, $destroy:ident, $append:ident, $prepend:ident, $size:ident, $clear:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $create() -> *mut ListT<$ty> {
            gc::list_create::<$ty>()
        }

        #[no_mangle]
        pub unsafe extern "C" fn $destroy(list: *mut ListT<$ty>) {
            gc::list_destroy::<$ty>(list)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $append(list: *mut ListT<$ty>, value: $ty) -> c_int {
            gc::list_append::<$ty>(list, value)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $prepend(list: *mut ListT<$ty>, value: $ty) -> c_int {
            gc::list_prepend::<$ty>(list, value)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $size(list: *mut ListT<$ty>) -> usize {
            gc::list_size::<$ty>(list)
        }

        #[no_mangle]
        pub unsafe extern "C" fn $clear(list: *mut ListT<$ty>) {
            gc::list_clear::<$ty>(list)
        }
    };
}

define_list!(
    c_int,
    list_int_create,
    list_int_destroy,
    list_int_append,
    list_int_prepend,
    list_int_size,
    list_int_clear
);
define_list!(
    f64,
    list_double_create,
    list_double_destroy,
    list_double_append,
    list_double_prepend,
    list_double_size,
    list_double_clear
);
define_list!(
    ItemT,
    list_item_t_create,
    list_item_t_destroy,
    list_item_t_append,
    list_item_t_prepend,
    list_item_t_size,
    list_item_t_clear
);
define_list!(
    OrderT,
    list_order_t_create,
    list_order_t_destroy,
    list_order_t_append,
    list_order_t_prepend,
    list_order_t_size,
    list_order_t_clear
);

// ============================================================================
// inventory.c
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn print_item(item: ItemT) {
    let mut stdout = CStdout;
    let out: &mut dyn Write = &mut stdout;
    inv::print_item(out, &item)
}

#[no_mangle]
pub unsafe extern "C" fn print_order(order: OrderT) {
    let mut stdout = CStdout;
    let out: &mut dyn Write = &mut stdout;
    inv::print_order(out, &order)
}

#[no_mangle]
pub unsafe extern "C" fn create_item(
    id: c_int,
    name: *const c_char,
    category: *const c_char,
    price: f64,
    quantity: c_int,
) -> ItemT {
    inv::create_item(id, name, category, price, quantity)
}

#[no_mangle]
pub unsafe extern "C" fn create_order(
    customer_id: c_int,
    customer_name: *const c_char,
    total_amount: f64,
) -> OrderT {
    inv::create_order(customer_id, customer_name, total_amount)
}

#[no_mangle]
pub unsafe extern "C" fn calculate_inventory_stats(items: *mut ArrayT<ItemT>) {
    let mut stdout = CStdout;
    let out: &mut dyn Write = &mut stdout;
    inv::calculate_inventory_stats(out, items)
}

#[no_mangle]
pub unsafe extern "C" fn calculate_order_stats(orders: *mut ListT<OrderT>) {
    let mut stdout = CStdout;
    let out: &mut dyn Write = &mut stdout;
    inv::calculate_order_stats(out, orders)
}

#[no_mangle]
pub unsafe extern "C" fn find_items_by_category(
    items: *mut ArrayT<ItemT>,
    category: *const c_char,
) {
    let mut stdout = CStdout;
    let out: &mut dyn Write = &mut stdout;
    inv::find_items_by_category(out, items, category)
}

#[no_mangle]
pub unsafe extern "C" fn find_expensive_items(items: *mut ListT<ItemT>, min_price: f64) {
    let mut stdout = CStdout;
    let out: &mut dyn Write = &mut stdout;
    inv::find_expensive_items(out, items, min_price)
}

// ============================================================================
// main.c
// ============================================================================

#[no_mangle]
pub extern "C" fn print_menu() {
    let mut stdout = CStdout;
    let out: &mut dyn Write = &mut stdout;
    demos::print_menu(out)
}

macro_rules! define_demo {
    ($name:ident, $body:path) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name() {
            let mut stdout = CStdout;
            let out: &mut dyn Write = &mut stdout;
            $body(out)
        }
    };
}

define_demo!(demo_integer_containers, demos::demo_integer_containers);
define_demo!(demo_double_containers, demos::demo_double_containers);
define_demo!(demo_inventory_array, demos::demo_inventory_array);
define_demo!(demo_order_list, demos::demo_order_list);
define_demo!(demo_mixed_operations, demos::demo_mixed_operations);

/// `int main(void)`
///
/// Exported under its C name so the shared library presents exactly the symbol
/// set of the C build (`main.c` is part of that build, so its `main` lands in
/// the `.so` too). The `driver` executable has its own Rust entry point that
/// calls the same [`demos::c_main`].
///
/// Excluded from `cfg(test)` builds only because the unit-test harness supplies
/// its own `main`; the `cdylib` always exports it.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    demos::c_main()
}
