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

use crate::generic_containers::{Array, List};

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

/// `item_t`
#[derive(Clone, Copy)]
pub struct ItemT {
    pub id: i32,
    pub name: [u8; MAX_NAME_LENGTH],
    pub category: [u8; MAX_CATEGORY_LENGTH],
    pub price: f64,
    pub quantity: i32,
}

/// `order_t`
#[derive(Clone, Copy)]
pub struct OrderT {
    pub customer_id: i32,
    pub customer_name: [u8; MAX_NAME_LENGTH],
    pub total_amount: f64,
}

/// Emulates `strncpy(dst, src, n - 1); dst[n - 1] = '\0';` over a fixed-size,
/// zero-filled buffer (`strncpy` NUL-pads the tail when `src` is shorter).
fn copy_c_string<const N: usize>(src: &str) -> [u8; N] {
    let mut dst = [0u8; N];
    let bytes = src.as_bytes();
    let n = if bytes.len() < N - 1 {
        bytes.len()
    } else {
        N - 1
    };
    dst[..n].copy_from_slice(&bytes[..n]);
    dst
}

/// Reads a NUL-terminated fixed buffer the way `printf("%s", buf)` would.
fn c_str(buf: &[u8]) -> &[u8] {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..end]
}

/// Same as [`c_str`] but usable as a `{}` format argument. All strings in this
/// program originate from ASCII literals, so no lossy substitution can occur.
fn c_str_disp(buf: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(c_str(buf))
}

/// `void print_item(item_t item)`
pub fn print_item(out: &mut dyn Write, item: &ItemT) {
    cprintf!(out, "  [{}] {}\n", item.id, c_str_disp(&item.name));
    cprintf!(out, "      Category: {}\n", c_str_disp(&item.category));
    cprintf!(out, "      Price: ${:.2}\n", item.price);
    cprintf!(out, "      Quantity: {}\n", item.quantity);
}

/// `void print_order(order_t order)`
pub fn print_order(out: &mut dyn Write, order: &OrderT) {
    cprintf!(
        out,
        "  Order - Customer ID: {}, Name: {}\n",
        order.customer_id,
        c_str_disp(&order.customer_name)
    );
    cprintf!(out, "          Total: ${:.2}\n", order.total_amount);
}

/// `item_t create_item(...)`
pub fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> ItemT {
    ItemT {
        id,
        name: copy_c_string::<MAX_NAME_LENGTH>(name),
        category: copy_c_string::<MAX_CATEGORY_LENGTH>(category),
        price,
        quantity,
    }
}

/// `order_t create_order(...)`
pub fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> OrderT {
    OrderT {
        customer_id,
        customer_name: copy_c_string::<MAX_NAME_LENGTH>(customer_name),
        total_amount,
    }
}

/// `void calculate_inventory_stats(array_item_t_t *items)`
pub fn calculate_inventory_stats(out: &mut dyn Write, items: &Array<ItemT>) {
    if items.size() == 0 {
        cprintf!(out, "No items in inventory\n");
        return;
    }

    cprintf!(out, "\n=== Inventory Statistics (Array) ===\n");

    let mut total_value: f64 = 0.0;
    let mut total_items: i32 = 0;
    let mut max_price: f64 = 0.0;
    let mut min_price: f64 = items.data()[0].price;

    for item in items.iter() {
        total_value += item.price * (item.quantity as f64);
        total_items += item.quantity;
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    cprintf!(out, "Total unique items: {}\n", items.size());
    cprintf!(out, "Total item count: {}\n", total_items);
    cprintf!(out, "Total inventory value: ${:.2}\n", total_value);
    cprintf!(
        out,
        "Average item price: ${:.2}\n",
        total_value / (total_items as f64)
    );
    cprintf!(out, "Most expensive item: ${:.2}\n", max_price);
    cprintf!(out, "Least expensive item: ${:.2}\n", min_price);
}

/// `void calculate_order_stats(list_order_t_t *orders)`
pub fn calculate_order_stats(out: &mut dyn Write, orders: &List<OrderT>) {
    if orders.size() == 0 {
        cprintf!(out, "No orders to analyze\n");
        return;
    }

    cprintf!(out, "\n=== Order Statistics (List) ===\n");

    let mut total_revenue: f64 = 0.0;
    let mut max_order: f64 = 0.0;
    let mut min_order: f64 = -1.0;

    for order in orders.iter() {
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
    }

    cprintf!(out, "Total orders: {}\n", orders.size());
    cprintf!(out, "Total revenue: ${:.2}\n", total_revenue);
    cprintf!(
        out,
        "Average order value: ${:.2}\n",
        total_revenue / (orders.size() as f64)
    );
    cprintf!(out, "Largest order: ${:.2}\n", max_order);
    cprintf!(out, "Smallest order: ${:.2}\n", min_order);
}

/// `void find_items_by_category(array_item_t_t *items, const char *category)`
pub fn find_items_by_category(out: &mut dyn Write, items: &Array<ItemT>, category: &str) {
    cprintf!(out, "\n=== Items in category '{}' ===\n", category);

    let mut found: i32 = 0;

    for item in items.iter() {
        if c_str(&item.category) == category.as_bytes() {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        cprintf!(out, "No items found in this category\n");
    } else {
        cprintf!(out, "Found {} items\n", found);
    }
}

/// `void find_expensive_items(list_item_t_t *items, double min_price)`
pub fn find_expensive_items(out: &mut dyn Write, items: &List<ItemT>, min_price: f64) {
    cprintf!(out, "\n=== Items priced above ${:.2} ===\n", min_price);

    let mut found: i32 = 0;

    for item in items.iter() {
        if item.price >= min_price {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        cprintf!(out, "No items found above this price\n");
    } else {
        cprintf!(out, "Found {} items\n", found);
    }
}
