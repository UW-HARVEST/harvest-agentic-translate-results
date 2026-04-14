use crate::generic_containers::{ArrayItemTT, ListItemTT, ListOrderTT};

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

#[derive(Clone)]
pub struct ItemT {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub price: f64,
    pub quantity: i32,
}

#[derive(Clone)]
pub struct OrderT {
    pub customer_id: i32,
    pub customer_name: String,
    pub total_amount: f64,
}

fn truncate_string(input: &str, max_len: usize) -> String {
    input.chars().take(max_len.saturating_sub(1)).collect()
}

pub fn print_item(item: &ItemT) {
    println!("  [{}] {}", item.id, item.name);
    println!("      Category: {}", item.category);
    println!("      Price: ${:.2}", item.price);
    println!("      Quantity: {}", item.quantity);
}

pub fn print_order(order: &OrderT) {
    println!(
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id, order.customer_name
    );
    println!("          Total: ${:.2}", order.total_amount);
}

pub fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> ItemT {
    ItemT {
        id,
        name: truncate_string(name, MAX_NAME_LENGTH),
        category: truncate_string(category, MAX_CATEGORY_LENGTH),
        price,
        quantity,
    }
}

pub fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> OrderT {
    OrderT {
        customer_id,
        customer_name: truncate_string(customer_name, MAX_NAME_LENGTH),
        total_amount,
    }
}

pub fn calculate_inventory_stats(items: &ArrayItemTT) {
    if items.size() == 0 {
        println!("No items in inventory");
        return;
    }

    println!("\n=== Inventory Statistics (Array) ===");

    let first_price = items.as_slice()[0].price;
    let mut total_value = 0.0;
    let mut total_items = 0i32;
    let mut max_price = 0.0;
    let mut min_price = first_price;

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

    println!("Total unique items: {}", items.size());
    println!("Total item count: {}", total_items);
    println!("Total inventory value: ${:.2}", total_value);
    println!("Average item price: ${:.2}", total_value / total_items as f64);
    println!("Most expensive item: ${:.2}", max_price);
    println!("Least expensive item: ${:.2}", min_price);
}

pub fn calculate_order_stats(orders: &ListOrderTT) {
    if orders.size() == 0 {
        println!("No orders to analyze");
        return;
    }

    println!("\n=== Order Statistics (List) ===");

    let mut total_revenue = 0.0;
    let mut max_order = 0.0;
    let mut min_order = -1.0;

    for order in orders.iter() {
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
    }

    println!("Total orders: {}", orders.size());
    println!("Total revenue: ${:.2}", total_revenue);
    println!("Average order value: ${:.2}", total_revenue / orders.size() as f64);
    println!("Largest order: ${:.2}", max_order);
    println!("Smallest order: ${:.2}", min_order);
}

pub fn find_items_by_category(items: &ArrayItemTT, category: &str) {
    println!("\n=== Items in category '{}' ===", category);

    let mut found = 0;
    for item in items.iter() {
        if item.category == category {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        println!("No items found in this category");
    } else {
        println!("Found {} items", found);
    }
}

pub fn find_expensive_items(items: &ListItemTT, min_price: f64) {
    println!("\n=== Items priced above ${:.2} ===", min_price);

    let mut found = 0;
    for item in items.iter() {
        if item.price >= min_price {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        println!("No items found above this price");
    } else {
        println!("Found {} items", found);
    }
}
