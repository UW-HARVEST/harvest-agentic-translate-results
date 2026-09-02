// inventory.rs
//
// Translation of c_src/include/inventory.h and c_src/src/inventory.c.

#![allow(dead_code)]

use crate::containers::{Array, List};
use crate::stdio::out_raw;

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

#[derive(Copy, Clone)]
pub struct Item {
    pub id: i32,
    pub name: [u8; MAX_NAME_LENGTH],
    pub category: [u8; MAX_CATEGORY_LENGTH],
    pub price: f64,
    pub quantity: i32,
}

#[derive(Copy, Clone)]
pub struct Order {
    pub customer_id: i32,
    pub customer_name: [u8; MAX_NAME_LENGTH],
    pub total_amount: f64,
}

/// `strncpy(dst, src, N - 1); dst[N - 1] = '\0';`
///
/// Copies at most `N - 1` bytes and zero-fills the remainder, so the result is
/// always NUL-terminated and any longer source is silently truncated.
fn strncpy_terminated<const N: usize>(src: &str) -> [u8; N] {
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

/// The `%s` view of a fixed-size char buffer: bytes up to the first NUL.
pub fn c_str(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(n) => &buf[..n],
        None => buf,
    }
}

/// `strcmp(buf, s) == 0` for a NUL-terminated char buffer.
fn c_str_eq(buf: &[u8], s: &str) -> bool {
    c_str(buf) == s.as_bytes()
}

pub fn print_item(item: &Item) {
    printf!("  [{}] ", item.id);
    out_raw(c_str(&item.name));
    printf!("\n");
    printf!("      Category: ");
    out_raw(c_str(&item.category));
    printf!("\n");
    printf!("      Price: ${:.2}\n", item.price);
    printf!("      Quantity: {}\n", item.quantity);
}

pub fn print_order(order: &Order) {
    printf!("  Order - Customer ID: {}, Name: ", order.customer_id);
    out_raw(c_str(&order.customer_name));
    printf!("\n");
    printf!("          Total: ${:.2}\n", order.total_amount);
}

pub fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: strncpy_terminated::<MAX_NAME_LENGTH>(name),
        category: strncpy_terminated::<MAX_CATEGORY_LENGTH>(category),
        price,
        quantity,
    }
}

pub fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: strncpy_terminated::<MAX_NAME_LENGTH>(customer_name),
        total_amount,
    }
}

pub fn calculate_inventory_stats(items: &Array<Item>) {
    if items.size() == 0 {
        printf!("No items in inventory\n");
        return;
    }

    printf!("\n=== Inventory Statistics (Array) ===\n");

    let mut total_value: f64 = 0.0;
    let mut total_items: i32 = 0;
    // Note: `max_price` starts at 0.0 rather than at the first element's price.
    let mut max_price: f64 = 0.0;
    let mut min_price: f64 = items.get(0).price;

    for item in items.iter() {
        total_value += item.price * item.quantity as f64;
        total_items += item.quantity;
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    printf!("Total unique items: {}\n", items.size());
    printf!("Total item count: {}\n", total_items);
    printf!("Total inventory value: ${:.2}\n", total_value);
    printf!(
        "Average item price: ${:.2}\n",
        total_value / total_items as f64
    );
    printf!("Most expensive item: ${:.2}\n", max_price);
    printf!("Least expensive item: ${:.2}\n", min_price);
}

pub fn calculate_order_stats(orders: &List<Order>) {
    if orders.size() == 0 {
        printf!("No orders to analyze\n");
        return;
    }

    printf!("\n=== Order Statistics (List) ===\n");

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

    printf!("Total orders: {}\n", orders.size());
    printf!("Total revenue: ${:.2}\n", total_revenue);
    printf!(
        "Average order value: ${:.2}\n",
        total_revenue / orders.size() as f64
    );
    printf!("Largest order: ${:.2}\n", max_order);
    printf!("Smallest order: ${:.2}\n", min_order);
}

pub fn find_items_by_category(items: &Array<Item>, category: &str) {
    printf!("\n=== Items in category '{}' ===\n", category);

    let mut found: i32 = 0;

    for item in items.iter() {
        if c_str_eq(&item.category, category) {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        printf!("No items found in this category\n");
    } else {
        printf!("Found {} items\n", found);
    }
}

pub fn find_expensive_items(items: &List<Item>, min_price: f64) {
    printf!("\n=== Items priced above ${:.2} ===\n", min_price);

    let mut found: i32 = 0;

    for item in items.iter() {
        if item.price >= min_price {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        printf!("No items found above this price\n");
    } else {
        printf!("Found {} items\n", found);
    }
}
