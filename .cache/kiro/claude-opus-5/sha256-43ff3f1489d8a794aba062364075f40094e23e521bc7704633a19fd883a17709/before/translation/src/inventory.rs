//! Translation of `inventory.h` / `inventory.c`.

use crate::containers::{Array, List};
use crate::cstr::{cstr_eq, cstr_from, cstr_str};

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

/// `item_t`
///
/// `name` and `category` stay as fixed-size NUL-terminated byte buffers so that
/// `create_item`'s `strncpy` truncation behaviour is reproduced exactly.
#[derive(Clone, Copy)]
pub struct Item {
    pub id: i32,
    pub name: [u8; MAX_NAME_LENGTH],
    pub category: [u8; MAX_CATEGORY_LENGTH],
    pub price: f64,
    pub quantity: i32,
}

/// `order_t`
#[derive(Clone, Copy)]
pub struct Order {
    pub customer_id: i32,
    pub customer_name: [u8; MAX_NAME_LENGTH],
    pub total_amount: f64,
}

/// ```c
/// printf("  [%d] %s\n", item.id, item.name);
/// printf("      Category: %s\n", item.category);
/// printf("      Price: $%.2f\n", item.price);
/// printf("      Quantity: %d\n", item.quantity);
/// ```
pub fn print_item(out: &mut impl std::io::Write, item: &Item) {
    let _ = write!(out, "  [{}] {}\n", item.id, cstr_str(&item.name));
    let _ = write!(out, "      Category: {}\n", cstr_str(&item.category));
    let _ = write!(out, "      Price: ${:.2}\n", item.price);
    let _ = write!(out, "      Quantity: {}\n", item.quantity);
}

/// ```c
/// printf("  Order - Customer ID: %d, Name: %s\n", order.customer_id, order.customer_name);
/// printf("          Total: $%.2f\n", order.total_amount);
/// ```
pub fn print_order(out: &mut impl std::io::Write, order: &Order) {
    let _ = write!(
        out,
        "  Order - Customer ID: {}, Name: {}\n",
        order.customer_id,
        cstr_str(&order.customer_name)
    );
    let _ = write!(out, "          Total: ${:.2}\n", order.total_amount);
}

/// `create_item` - `strncpy` into fixed buffers, then force the last byte NUL.
pub fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: cstr_from::<MAX_NAME_LENGTH>(name),
        category: cstr_from::<MAX_CATEGORY_LENGTH>(category),
        price,
        quantity,
    }
}

/// `create_order`
pub fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: cstr_from::<MAX_NAME_LENGTH>(customer_name),
        total_amount,
    }
}

/// `calculate_inventory_stats(array_item_t_t *items)`
pub fn calculate_inventory_stats(out: &mut impl std::io::Write, items: &Array<Item>) {
    // The C code checks `!items || items->size == 0` first; the null case cannot
    // arise here because the caller always passes a live container.
    if items.size() == 0 {
        let _ = write!(out, "No items in inventory\n");
        return;
    }

    let _ = write!(out, "\n=== Inventory Statistics (Array) ===\n");

    let mut total_value: f64 = 0.0;
    let mut total_items: i32 = 0;
    // NOTE: matches the C source exactly - max_price seeds from 0.0 while
    // min_price seeds from the first element's price.
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

    let _ = write!(out, "Total unique items: {}\n", items.size());
    let _ = write!(out, "Total item count: {}\n", total_items);
    let _ = write!(out, "Total inventory value: ${:.2}\n", total_value);
    let _ = write!(
        out,
        "Average item price: ${:.2}\n",
        total_value / f64::from(total_items)
    );
    let _ = write!(out, "Most expensive item: ${:.2}\n", max_price);
    let _ = write!(out, "Least expensive item: ${:.2}\n", min_price);
}

/// `calculate_order_stats(list_order_t_t *orders)`
pub fn calculate_order_stats(out: &mut impl std::io::Write, orders: &List<Order>) {
    if orders.size() == 0 {
        let _ = write!(out, "No orders to analyze\n");
        return;
    }

    let _ = write!(out, "\n=== Order Statistics (List) ===\n");

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

    let _ = write!(out, "Total orders: {}\n", orders.size());
    let _ = write!(out, "Total revenue: ${:.2}\n", total_revenue);
    let _ = write!(
        out,
        "Average order value: ${:.2}\n",
        total_revenue / orders.size() as f64
    );
    let _ = write!(out, "Largest order: ${:.2}\n", max_order);
    let _ = write!(out, "Smallest order: ${:.2}\n", min_order);
}

/// `find_items_by_category(array_item_t_t *items, const char *category)`
pub fn find_items_by_category(out: &mut impl std::io::Write, items: &Array<Item>, category: &str) {
    let _ = write!(out, "\n=== Items in category '{}' ===\n", category);

    let mut found: i32 = 0;
    for item in items.iter() {
        if cstr_eq(&item.category, category) {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        let _ = write!(out, "No items found in this category\n");
    } else {
        let _ = write!(out, "Found {} items\n", found);
    }
}

/// `find_expensive_items(list_item_t_t *items, double min_price)`
#[allow(dead_code)]
pub fn find_expensive_items(out: &mut impl std::io::Write, items: &List<Item>, min_price: f64) {
    let _ = write!(out, "\n=== Items priced above ${:.2} ===\n", min_price);

    let mut found: i32 = 0;
    for item in items.iter() {
        if item.price >= min_price {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        let _ = write!(out, "No items found above this price\n");
    } else {
        let _ = write!(out, "Found {} items\n", found);
    }
}
