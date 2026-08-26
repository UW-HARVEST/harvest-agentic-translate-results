use std::collections::LinkedList;

#[derive(Clone, Debug)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub price: f64,
    pub quantity: i32,
}

#[derive(Clone, Debug)]
pub struct Order {
    pub customer_id: i32,
    pub customer_name: String,
    pub total_amount: f64,
}

pub fn print_item(item: &Item) {
    println!("  [{}] {}", item.id, item.name);
    println!("      Category: {}", item.category);
    println!("      Price: ${:.2}", item.price);
    println!("      Quantity: {}", item.quantity);
}

pub fn print_order(order: &Order) {
    println!(
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id, order.customer_name
    );
    println!("          Total: ${:.2}", order.total_amount);
}

pub fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: name.to_string(),
        category: category.to_string(),
        price,
        quantity,
    }
}

pub fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: customer_name.to_string(),
        total_amount,
    }
}

pub fn calculate_inventory_stats(items: &[Item]) {
    if items.is_empty() {
        println!("No items in inventory");
        return;
    }

    println!("\n=== Inventory Statistics (Array) ===");

    let mut total_value = 0.0;
    let mut total_items = 0;
    let mut max_price = 0.0f64;
    let mut min_price = items[0].price;

    for item in items {
        total_value += item.price * (item.quantity as f64);
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
    println!("Average item price: ${:.2}", total_value / (total_items as f64));
    println!("Most expensive item: ${:.2}", max_price);
    println!("Least expensive item: ${:.2}", min_price);
}

pub fn calculate_order_stats(orders: &LinkedList<Order>) {
    if orders.is_empty() {
        println!("No orders to analyze");
        return;
    }

    println!("\n=== Order Statistics (List) ===");

    let mut total_revenue = 0.0;
    let mut max_order = 0.0f64;
    let mut min_order = -1.0f64;

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
    println!("Average order value: ${:.2}", total_revenue / (orders.len() as f64));
    println!("Largest order: ${:.2}", max_order);
    println!("Smallest order: ${:.2}", min_order);
}

pub fn find_items_by_category(items: &[Item], category: &str) {
    println!("\n=== Items in category '{}' ===", category);

    let mut found = 0;

    for item in items {
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

pub fn find_expensive_items(items: &LinkedList<Item>, min_price: f64) {
    println!("\n=== Items priced above ${:.2} ===", min_price);

    let mut found = 0;

    for item in items {
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
