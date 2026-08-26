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
//! inventory.rs -- translation of inventory.c / inventory.h

#![allow(dead_code)]

use std::io::Write;

use libc::{c_char, c_int};

use crate::cio::{c_str_ptr, fadd_c, fmt_f, strcmp, strncpy_field, w};
use crate::generic_containers::{array_foreach, list_foreach, ArrayT, ListT};

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

/// `item_t` (`sizeof` 120, `id` @0, `name` @4, `category` @68, `price` @104,
/// `quantity` @112)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ItemT {
    pub id: c_int,
    pub name: [u8; MAX_NAME_LENGTH],
    pub category: [u8; MAX_CATEGORY_LENGTH],
    pub price: f64,
    pub quantity: c_int,
}

/// `order_t` (`sizeof` 80, `customer_id` @0, `customer_name` @4,
/// `total_amount` @72)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrderT {
    pub customer_id: c_int,
    pub customer_name: [u8; MAX_NAME_LENGTH],
    pub total_amount: f64,
}

/// `void print_item(item_t item)`
///
/// `%s` is emitted with an unbounded read from the field's address, exactly like
/// `printf`: if a caller hands over a `name`/`category` with no NUL in it, C runs
/// on into the following bytes of the struct, and so does this.
pub fn print_item(out: &mut dyn Write, item: &ItemT) {
    // printf("  [%d] %s\n", item.id, item.name);
    cprintf!(out, "  [{}] ", item.id);
    unsafe { w(out, c_str_ptr(item.name.as_ptr() as *const c_char)) };
    w(out, b"\n");
    // printf("      Category: %s\n", item.category);
    w(out, b"      Category: ");
    unsafe { w(out, c_str_ptr(item.category.as_ptr() as *const c_char)) };
    w(out, b"\n");
    cprintf!(out, "      Price: ${}\n", fmt_f(item.price, 2));
    cprintf!(out, "      Quantity: {}\n", item.quantity);
}

/// `void print_order(order_t order)`
pub fn print_order(out: &mut dyn Write, order: &OrderT) {
    // printf("  Order - Customer ID: %d, Name: %s\n", ...);
    cprintf!(out, "  Order - Customer ID: {}, Name: ", order.customer_id);
    unsafe { w(out, c_str_ptr(order.customer_name.as_ptr() as *const c_char)) };
    w(out, b"\n");
    cprintf!(
        out,
        "          Total: ${}\n",
        fmt_f(order.total_amount, 2)
    );
}

/// `item_t create_item(int, const char *, const char *, double, int)`
///
/// # Safety
/// `name` and `category` must be NUL-terminated C strings, as `strncpy`
/// requires.
pub unsafe fn create_item(
    id: c_int,
    name: *const c_char,
    category: *const c_char,
    price: f64,
    quantity: c_int,
) -> ItemT {
    ItemT {
        id,
        name: strncpy_field::<MAX_NAME_LENGTH>(name),
        category: strncpy_field::<MAX_CATEGORY_LENGTH>(category),
        price,
        quantity,
    }
}

/// `order_t create_order(int, const char *, double)`
///
/// # Safety
/// `customer_name` must be a NUL-terminated C string.
pub unsafe fn create_order(
    customer_id: c_int,
    customer_name: *const c_char,
    total_amount: f64,
) -> OrderT {
    OrderT {
        customer_id,
        customer_name: strncpy_field::<MAX_NAME_LENGTH>(customer_name),
        total_amount,
    }
}

/// `void calculate_inventory_stats(array_item_t_t *items)`
///
/// # Safety
/// `items` must be NULL or a valid `array_item_t_t` pointer.
pub unsafe fn calculate_inventory_stats(out: &mut dyn Write, items: *mut ArrayT<ItemT>) {
    if items.is_null() || (*items).size == 0 {
        w(out, b"No items in inventory\n");
        return;
    }

    w(out, b"\n=== Inventory Statistics (Array) ===\n");

    let mut total_value: f64 = 0.0;
    let mut total_items: c_int = 0;
    let mut max_price: f64 = 0.0;
    let mut min_price: f64 = (*(*items).data).price;

    array_foreach(items, |item: ItemT| {
        total_value = fadd_c(total_value, item.price * (item.quantity as f64));
        // `total_items += item.quantity` on `int`: wraps like the -O0 C build.
        total_items = total_items.wrapping_add(item.quantity);
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    });

    cprintf!(out, "Total unique items: {}\n", (*items).size);
    cprintf!(out, "Total item count: {}\n", total_items);
    cprintf!(out, "Total inventory value: ${}\n", fmt_f(total_value, 2));
    cprintf!(
        out,
        "Average item price: ${}\n",
        fmt_f(total_value / (total_items as f64), 2)
    );
    cprintf!(out, "Most expensive item: ${}\n", fmt_f(max_price, 2));
    cprintf!(out, "Least expensive item: ${}\n", fmt_f(min_price, 2));
}

/// `void calculate_order_stats(list_order_t_t *orders)`
///
/// # Safety
/// `orders` must be NULL or a valid `list_order_t_t` pointer.
pub unsafe fn calculate_order_stats(out: &mut dyn Write, orders: *mut ListT<OrderT>) {
    if orders.is_null() || (*orders).size == 0 {
        w(out, b"No orders to analyze\n");
        return;
    }

    w(out, b"\n=== Order Statistics (List) ===\n");

    let mut total_revenue: f64 = 0.0;
    let mut max_order: f64 = 0.0;
    let mut min_order: f64 = -1.0;

    list_foreach(orders, |order: OrderT| {
        total_revenue = fadd_c(total_revenue, order.total_amount);
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
    });

    cprintf!(out, "Total orders: {}\n", (*orders).size);
    cprintf!(out, "Total revenue: ${}\n", fmt_f(total_revenue, 2));
    cprintf!(
        out,
        "Average order value: ${}\n",
        fmt_f(total_revenue / ((*orders).size as f64), 2)
    );
    cprintf!(out, "Largest order: ${}\n", fmt_f(max_order, 2));
    cprintf!(out, "Smallest order: ${}\n", fmt_f(min_order, 2));
}

/// `void find_items_by_category(array_item_t_t *items, const char *category)`
///
/// # Safety
/// `items` must be NULL or a valid array pointer; `category` must be NULL or a
/// NUL-terminated C string.
pub unsafe fn find_items_by_category(
    out: &mut dyn Write,
    items: *mut ArrayT<ItemT>,
    category: *const c_char,
) {
    if items.is_null() || category.is_null() {
        return;
    }

    w(out, b"\n=== Items in category '");
    w(out, c_str_ptr(category));
    w(out, b"' ===\n");

    let mut found: c_int = 0;

    array_foreach(items, |item: ItemT| {
        // strcmp(item.category, category) == 0, on the loop variable's copy.
        let item = item;
        if strcmp(item.category.as_ptr(), category as *const u8) == 0 {
            print_item(out, &item);
            found += 1;
        }
    });

    if found == 0 {
        w(out, b"No items found in this category\n");
    } else {
        cprintf!(out, "Found {} items\n", found);
    }
}

/// `void find_expensive_items(list_item_t_t *items, double min_price)`
///
/// # Safety
/// `items` must be NULL or a valid `list_item_t_t` pointer.
pub unsafe fn find_expensive_items(out: &mut dyn Write, items: *mut ListT<ItemT>, min_price: f64) {
    if items.is_null() {
        return;
    }

    cprintf!(
        out,
        "\n=== Items priced above ${} ===\n",
        fmt_f(min_price, 2)
    );

    let mut found: c_int = 0;

    list_foreach(items, |item: ItemT| {
        if item.price >= min_price {
            print_item(out, &item);
            found += 1;
        }
    });

    if found == 0 {
        w(out, b"No items found above this price\n");
    } else {
        cprintf!(out, "Found {} items\n", found);
    }
}
