// inventory.rs
//
// Rust translation of inventory.h / inventory.c

use std::io::Write;

use crate::containers::{Array, List};
use crate::cstdio::{cstr, fmt_f, strncpy_truncate};

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

#[derive(Clone, Copy)]
pub struct Item {
    pub id: i32,
    pub name: [u8; MAX_NAME_LENGTH],
    pub category: [u8; MAX_CATEGORY_LENGTH],
    pub price: f64,
    pub quantity: i32,
}

#[derive(Clone, Copy)]
pub struct Order {
    pub customer_id: i32,
    pub customer_name: [u8; MAX_NAME_LENGTH],
    pub total_amount: f64,
}

pub fn print_item(out: &mut dyn Write, item: &Item) {
    p!(out, "  [{}] ", item.id);
    pb!(out, cstr(&item.name));
    p!(out, "\n");
    p!(out, "      Category: ");
    pb!(out, cstr(&item.category));
    p!(out, "\n");
    p!(out, "      Price: ${}\n", fmt_f(item.price, 2));
    p!(out, "      Quantity: {}\n", item.quantity);
}

pub fn print_order(out: &mut dyn Write, order: &Order) {
    p!(out, "  Order - Customer ID: {}, Name: ", order.customer_id);
    pb!(out, cstr(&order.customer_name));
    p!(out, "\n");
    p!(out, "          Total: ${}\n", fmt_f(order.total_amount, 2));
}

pub fn create_item(id: i32, name: &[u8], category: &[u8], price: f64, quantity: i32) -> Item {
    let mut item = Item {
        id,
        name: [0u8; MAX_NAME_LENGTH],
        category: [0u8; MAX_CATEGORY_LENGTH],
        price,
        quantity,
    };
    strncpy_truncate(&mut item.name, name);
    strncpy_truncate(&mut item.category, category);
    item
}

pub fn create_order(customer_id: i32, customer_name: &[u8], total_amount: f64) -> Order {
    let mut order = Order {
        customer_id,
        customer_name: [0u8; MAX_NAME_LENGTH],
        total_amount,
    };
    strncpy_truncate(&mut order.customer_name, customer_name);
    order
}

pub fn calculate_inventory_stats(out: &mut dyn Write, items: &Array<Item>) {
    if items.size() == 0 {
        p!(out, "No items in inventory\n");
        return;
    }

    p!(out, "\n=== Inventory Statistics (Array) ===\n");

    let mut total_value: f64 = 0.0;
    let mut total_items: i32 = 0;
    let mut max_price: f64 = 0.0;
    let mut min_price: f64 = items.get(0).price;

    for item in items.iter() {
        total_value += item.price * f64::from(item.quantity);
        total_items = total_items.wrapping_add(item.quantity);
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    p!(out, "Total unique items: {}\n", items.size());
    p!(out, "Total item count: {}\n", total_items);
    p!(out, "Total inventory value: ${}\n", fmt_f(total_value, 2));
    p!(
        out,
        "Average item price: ${}\n",
        fmt_f(total_value / f64::from(total_items), 2)
    );
    p!(out, "Most expensive item: ${}\n", fmt_f(max_price, 2));
    p!(out, "Least expensive item: ${}\n", fmt_f(min_price, 2));
}

pub fn calculate_order_stats(out: &mut dyn Write, orders: &List<Order>) {
    if orders.size() == 0 {
        p!(out, "No orders to analyze\n");
        return;
    }

    p!(out, "\n=== Order Statistics (List) ===\n");

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

    p!(out, "Total orders: {}\n", orders.size());
    p!(out, "Total revenue: ${}\n", fmt_f(total_revenue, 2));
    p!(
        out,
        "Average order value: ${}\n",
        fmt_f(total_revenue / orders.size() as f64, 2)
    );
    p!(out, "Largest order: ${}\n", fmt_f(max_order, 2));
    p!(out, "Smallest order: ${}\n", fmt_f(min_order, 2));
}

pub fn find_items_by_category(out: &mut dyn Write, items: &Array<Item>, category: &[u8]) {
    p!(out, "\n=== Items in category '");
    pb!(out, cstr(category));
    p!(out, "' ===\n");

    let mut found: i32 = 0;

    for item in items.iter() {
        if cstr(&item.category) == cstr(category) {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        p!(out, "No items found in this category\n");
    } else {
        p!(out, "Found {} items\n", found);
    }
}

#[allow(dead_code)]
pub fn find_expensive_items(out: &mut dyn Write, items: &List<Item>, min_price: f64) {
    p!(
        out,
        "\n=== Items priced above ${} ===\n",
        fmt_f(min_price, 2)
    );

    let mut found: i32 = 0;

    for item in items.iter() {
        if item.price >= min_price {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        p!(out, "No items found above this price\n");
    } else {
        p!(out, "Found {} items\n", found);
    }
}
