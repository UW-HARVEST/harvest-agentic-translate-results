use std::collections::LinkedList;

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

#[derive(Clone, Copy)]
pub struct Item {
    pub id: i32,
    name: [u8; MAX_NAME_LENGTH],
    category: [u8; MAX_CATEGORY_LENGTH],
    pub price: f64,
    pub quantity: i32,
}

#[derive(Clone, Copy)]
pub struct Order {
    pub customer_id: i32,
    customer_name: [u8; MAX_NAME_LENGTH],
    pub total_amount: f64,
}

fn copy_c_string<const N: usize>(src: &str) -> [u8; N] {
    let mut out = [0u8; N];
    let bytes = src.as_bytes();
    let len = bytes.len().min(N.saturating_sub(1));
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

impl Item {
    pub fn name(&self) -> String {
        c_string(&self.name)
    }

    pub fn category(&self) -> String {
        c_string(&self.category)
    }
}

impl Order {
    pub fn customer_name(&self) -> String {
        c_string(&self.customer_name)
    }
}

pub fn print_item(item: Item) {
    println!("  [{}] {}", item.id, item.name());
    println!("      Category: {}", item.category());
    println!("      Price: ${:.2}", item.price);
    println!("      Quantity: {}", item.quantity);
}

pub fn print_order(order: Order) {
    println!(
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id,
        order.customer_name()
    );
    println!("          Total: ${:.2}", order.total_amount);
}

pub fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: copy_c_string(name),
        category: copy_c_string(category),
        price,
        quantity,
    }
}

pub fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: copy_c_string(customer_name),
        total_amount,
    }
}

pub fn calculate_inventory_stats(items: &[Item]) {
    if items.is_empty() {
        println!("No items in inventory");
        return;
    }

    println!();
    println!("=== Inventory Statistics (Array) ===");

    let mut total_value = 0.0;
    let mut total_items = 0i32;
    let mut max_price = 0.0;
    let mut min_price = items[0].price;

    for item in items {
        total_value += item.price * f64::from(item.quantity);
        total_items += item.quantity;
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    println!("Total unique items: {}", items.len());
    println!("Total item count: {}", total_items);
    println!("Total inventory value: ${:.2}", total_value);
    println!("Average item price: ${:.2}", total_value / f64::from(total_items));
    println!("Most expensive item: ${:.2}", max_price);
    println!("Least expensive item: ${:.2}", min_price);
}

pub fn calculate_order_stats(orders: &LinkedList<Order>) {
    if orders.is_empty() {
        println!("No orders to analyze");
        return;
    }

    println!();
    println!("=== Order Statistics (List) ===");

    let mut total_revenue = 0.0;
    let mut max_order = 0.0;
    let mut min_order = -1.0;

    for order in orders {
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
    }

    println!("Total orders: {}", orders.len());
    println!("Total revenue: ${:.2}", total_revenue);
    println!("Average order value: ${:.2}", total_revenue / orders.len() as f64);
    println!("Largest order: ${:.2}", max_order);
    println!("Smallest order: ${:.2}", min_order);
}

pub fn find_items_by_category(items: &[Item], category: &str) {
    println!();
    println!("=== Items in category '{}' ===", category);

    let mut found = 0i32;

    for item in items {
        if item.category() == category {
            print_item(*item);
            found += 1;
        }
    }

    if found == 0 {
        println!("No items found in this category");
    } else {
        println!("Found {} items", found);
    }
}

pub fn find_expensive_items(items: &LinkedList<Item>, min_price: f64) {
    println!();
    println!("=== Items priced above ${:.2} ===", min_price);

    let mut found = 0i32;

    for item in items {
        if item.price >= min_price {
            print_item(*item);
            found += 1;
        }
    }

    if found == 0 {
        println!("No items found above this price");
    } else {
        println!("Found {} items", found);
    }
}
