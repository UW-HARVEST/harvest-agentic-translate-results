// Translation of MIT Lincoln Laboratory generic containers demo (C -> Rust).
// Reproduces byte-identical output for matching inputs.

use std::io::{self, Read, Write};

// ============================================================================
// Domain types
// ============================================================================

const MAX_NAME_LENGTH: usize = 64;
const MAX_CATEGORY_LENGTH: usize = 32;

#[derive(Clone)]
struct Item {
    id: i32,
    name: String,
    category: String,
    price: f64,
    quantity: i32,
}

#[derive(Clone)]
struct Order {
    customer_id: i32,
    customer_name: String,
    total_amount: f64,
}

fn truncate_to(s: &str, max_with_nul: usize) -> String {
    // Mimic strncpy + manual null-terminator: keep first (max_with_nul - 1) bytes.
    let limit = max_with_nul.saturating_sub(1);
    if s.len() <= limit {
        s.to_string()
    } else {
        // Truncate by bytes. For our inputs (ASCII) this is fine.
        s[..limit].to_string()
    }
}

fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: truncate_to(name, MAX_NAME_LENGTH),
        category: truncate_to(category, MAX_CATEGORY_LENGTH),
        price,
        quantity,
    }
}

fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: truncate_to(customer_name, MAX_NAME_LENGTH),
        total_amount,
    }
}

fn print_item(item: &Item) {
    println!("  [{}] {}", item.id, item.name);
    println!("      Category: {}", item.category);
    println!("      Price: ${:.2}", item.price);
    println!("      Quantity: {}", item.quantity);
}

fn print_order(order: &Order) {
    println!(
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id, order.customer_name
    );
    println!("          Total: ${:.2}", order.total_amount);
}

// ============================================================================
// Statistics
// ============================================================================

fn calculate_inventory_stats(items: &[Item]) {
    if items.is_empty() {
        println!("No items in inventory");
        return;
    }

    println!();
    println!("=== Inventory Statistics (Array) ===");

    let mut total_value: f64 = 0.0;
    let mut total_items: i32 = 0;
    let mut max_price: f64 = 0.0;
    let mut min_price: f64 = items[0].price;

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

    println!("Total unique items: {}", items.len());
    println!("Total item count: {}", total_items);
    println!("Total inventory value: ${:.2}", total_value);
    println!(
        "Average item price: ${:.2}",
        total_value / total_items as f64
    );
    println!("Most expensive item: ${:.2}", max_price);
    println!("Least expensive item: ${:.2}", min_price);
}

fn calculate_order_stats(orders: &[Order]) {
    if orders.is_empty() {
        println!("No orders to analyze");
        return;
    }

    println!();
    println!("=== Order Statistics (List) ===");

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

    println!("Total orders: {}", orders.len());
    println!("Total revenue: ${:.2}", total_revenue);
    println!(
        "Average order value: ${:.2}",
        total_revenue / orders.len() as f64
    );
    println!("Largest order: ${:.2}", max_order);
    println!("Smallest order: ${:.2}", min_order);
}

fn find_items_by_category(items: &[Item], category: &str) {
    println!();
    println!("=== Items in category '{}' ===", category);

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

#[allow(dead_code)]
fn find_expensive_items(items: &[Item], min_price: f64) {
    println!();
    println!("=== Items priced above ${:.2} ===", min_price);

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

// ============================================================================
// Demos
// ============================================================================

fn demo_integer_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 1: Integer Containers");
    println!("========================================");

    // Integer "array"
    let mut int_array: Vec<i32> = Vec::with_capacity(10);
    println!();
    println!("--- Integer Array ---");
    println!("Adding integers: 10, 20, 30, 40, 50");
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);

    print!("Array contents: ");
    for &val in int_array.iter() {
        print!("{} ", val);
    }
    println!();

    let mut sum: i32 = 0;
    for &val in int_array.iter() {
        sum += val;
    }
    println!("Sum: {}", sum);
    println!("Average: {:.2}", sum as f64 / int_array.len() as f64);

    // Integer "list"
    let mut int_list: Vec<i32> = Vec::new();
    println!();
    println!("--- Integer List ---");
    println!("Adding integers: 100, 200, 300, 400, 500");
    int_list.push(100);
    int_list.push(200);
    int_list.push(300);
    int_list.push(400);
    int_list.push(500);

    print!("List contents: ");
    for &val in int_list.iter() {
        print!("{} ", val);
    }
    println!();

    let mut product: i64 = 1;
    for &val in int_list.iter() {
        product = product.wrapping_mul(val as i64);
    }
    println!("Product: {}", product);
}

fn demo_double_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 2: Double Containers");
    println!("========================================");

    let mut double_array: Vec<f64> = Vec::with_capacity(5);
    println!();
    println!("--- Double Array (Temperatures in Celsius) ---");

    let temps = [23.5_f64, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    print!("Adding temperatures: ");
    for &t in temps.iter() {
        double_array.push(t);
        print!("{:.1} ", t);
    }
    println!();

    let mut min_temp = temps[0];
    let mut max_temp = temps[0];
    let mut sum_temp: f64 = 0.0;

    for &temp in double_array.iter() {
        if temp < min_temp {
            min_temp = temp;
        }
        if temp > max_temp {
            max_temp = temp;
        }
        sum_temp += temp;
    }

    println!("Minimum: {:.1}\u{00B0}C", min_temp);
    println!("Maximum: {:.1}\u{00B0}C", max_temp);
    println!(
        "Average: {:.1}\u{00B0}C",
        sum_temp / double_array.len() as f64
    );

    let mut price_list: Vec<f64> = Vec::new();
    println!();
    println!("--- Double List (Product Prices) ---");

    let prices = [9.99_f64, 14.50, 7.25, 22.00, 5.99, 18.75];

    print!("Adding prices: ");
    for &p in prices.iter() {
        price_list.push(p);
        print!("${:.2} ", p);
    }
    println!();

    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;

    for &temp in price_list.iter() {
        total += temp;
        if temp < 10.0 {
            count_under_10 += 1;
        }
    }

    println!("Total cost: ${:.2}", total);
    println!("Items under $10: {}", count_under_10);
}

fn demo_inventory_array() {
    println!();
    println!("========================================");
    println!("  DEMO 3: Inventory Array (Items)");
    println!("========================================");

    let mut inventory: Vec<Item> = Vec::with_capacity(20);

    println!();
    println!("--- Adding Items to Inventory ---");
    inventory.push(create_item(1, "Laptop", "Electronics", 899.99, 15));
    inventory.push(create_item(2, "Mouse", "Electronics", 24.99, 50));
    inventory.push(create_item(3, "Keyboard", "Electronics", 79.99, 30));
    inventory.push(create_item(4, "Monitor", "Electronics", 299.99, 20));
    inventory.push(create_item(5, "Desk Chair", "Furniture", 199.99, 10));
    inventory.push(create_item(6, "Desk", "Furniture", 349.99, 8));
    inventory.push(create_item(7, "Notebook", "Office", 4.99, 100));
    inventory.push(create_item(8, "Pen Set", "Office", 12.99, 75));
    inventory.push(create_item(9, "USB Cable", "Electronics", 9.99, 60));
    inventory.push(create_item(10, "Bookshelf", "Furniture", 149.99, 12));

    println!("Added {} items to inventory", inventory.len());

    println!();
    println!("--- All Inventory Items ---");
    for item in inventory.iter() {
        print_item(item);
        println!();
    }

    calculate_inventory_stats(&inventory);

    find_items_by_category(&inventory, "Electronics");
    find_items_by_category(&inventory, "Furniture");

    println!();
    println!("--- Low Stock Items (< 20) ---");
    let mut low_stock_count: i32 = 0;
    for item in inventory.iter() {
        if item.quantity < 20 {
            print_item(item);
            low_stock_count += 1;
        }
    }
    println!("Total low stock items: {}", low_stock_count);
}

fn demo_order_list() {
    println!();
    println!("========================================");
    println!("  DEMO 4: Order List (Orders)");
    println!("========================================");

    let mut orders: Vec<Order> = Vec::new();

    println!();
    println!("--- Adding Orders ---");
    orders.push(create_order(1001, "Alice Johnson", 1249.95));
    orders.push(create_order(1002, "Bob Smith", 89.99));
    orders.push(create_order(1003, "Carol White", 549.98));
    orders.push(create_order(1004, "David Brown", 24.99));
    orders.push(create_order(1005, "Eve Davis", 899.99));
    orders.push(create_order(1006, "Frank Miller", 374.97));
    orders.push(create_order(1007, "Grace Lee", 159.98));
    orders.push(create_order(1008, "Henry Wilson", 1099.99));

    println!("Added {} orders", orders.len());

    println!();
    println!("--- All Orders ---");
    for order in orders.iter() {
        print_order(order);
    }

    calculate_order_stats(&orders);

    println!();
    println!("--- Large Orders (> $500) ---");
    let mut large_order_count: i32 = 0;
    let mut large_order_total: f64 = 0.0;

    for order in orders.iter() {
        if order.total_amount > 500.0 {
            print_order(order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    println!("Total large orders: {}", large_order_count);
    println!("Revenue from large orders: ${:.2}", large_order_total);
}

fn demo_mixed_operations() {
    println!();
    println!("========================================");
    println!("  DEMO 5: Mixed Operations");
    println!("========================================");

    let mut array_inventory: Vec<Item> = Vec::with_capacity(10);
    let mut list_inventory: Vec<Item> = Vec::new();

    println!();
    println!("--- Populating both Array and List ---");

    let items: Vec<Item> = vec![
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items = items.len() as i32;

    for it in items.iter() {
        array_inventory.push(it.clone());
        list_inventory.push(it.clone());
    }

    println!("Added {} items to both containers", num_items);

    println!();
    println!("--- Iterating through Array ---");
    let mut array_count: i32 = 0;
    for _item in array_inventory.iter() {
        array_count += 1;
    }
    println!("Array iteration count: {}", array_count);

    println!();
    println!("--- Iterating through List ---");
    let mut list_count: i32 = 0;
    for _item in list_inventory.iter() {
        list_count += 1;
    }
    println!("List iteration count: {}", list_count);

    let price_threshold: f64 = 200.0;

    println!();
    println!("--- Items above ${:.2} (Array) ---", price_threshold);
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", item.name, item.price);
        }
    }

    println!();
    println!("--- Items above ${:.2} (List) ---", price_threshold);
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", item.name, item.price);
        }
    }
}

fn print_menu() {
    println!();
    println!("========================================");
    println!("  GENERIC FOR_EACH MACRO DEMO");
    println!("========================================");
    println!("1. Demo: Integer Containers");
    println!("2. Demo: Double Containers");
    println!("3. Demo: Inventory Array");
    println!("4. Demo: Order List");
    println!("5. Demo: Mixed Operations");
    println!("6. Run All Demos");
    println!("7. Exit");
    println!("========================================");
    print!("Choice: ");
    io::stdout().flush().ok();
}

// ============================================================================
// Input handling: mimic fgets(input, 256, stdin) + sscanf(input, "%d", ...)
// ============================================================================

/// Read up to (capacity - 1) bytes from stdin, stopping at the first newline
/// (newline is included in the returned bytes), or at EOF.
/// Returns None on immediate EOF (no bytes read), to mimic fgets() returning NULL.
fn fgets_like(reader: &mut impl Read, capacity: usize) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    let max_chars = capacity.saturating_sub(1);
    let mut byte = [0u8; 1];
    while buf.len() < max_chars {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Approximate sscanf("%d", &choice). Skips leading whitespace, then
/// optionally a sign, then consumes ASCII digits. Returns Some(value) only
/// if at least one digit was matched (sscanf would return 1).
fn sscanf_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        negative = bytes[i] == b'-';
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return None;
    }
    let mut val: i64 = 0;
    for &b in &bytes[start..i] {
        val = val.wrapping_mul(10).wrapping_add((b - b'0') as i64);
    }
    if negative {
        val = val.wrapping_neg();
    }
    Some(val as i32)
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║   GENERIC FOR_EACH MACRO DEMO         ║");
    println!("║   Demonstrating Generic Containers    ║");
    println!("╚════════════════════════════════════════╝");

    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        print_menu();

        let line = match fgets_like(&mut handle, 256) {
            Some(b) => b,
            None => break,
        };
        let s = match std::str::from_utf8(&line) {
            Ok(v) => v,
            Err(_) => {
                // Best-effort: treat as latin-1 like; for ASCII inputs this never triggers.
                let lossy = String::from_utf8_lossy(&line).into_owned();
                let parsed = sscanf_int(&lossy);
                if let Some(choice) = parsed {
                    if !dispatch(choice) {
                        return;
                    }
                } else {
                    println!("Invalid input");
                }
                continue;
            }
        };

        let choice = match sscanf_int(s) {
            Some(c) => c,
            None => {
                println!("Invalid input");
                continue;
            }
        };

        if !dispatch(choice) {
            return;
        }
    }
}

/// Returns false if the program should exit.
fn dispatch(choice: i32) -> bool {
    match choice {
        1 => demo_integer_containers(),
        2 => demo_double_containers(),
        3 => demo_inventory_array(),
        4 => demo_order_list(),
        5 => demo_mixed_operations(),
        6 => {
            println!();
            println!("=== Running All Demos ===");
            demo_integer_containers();
            demo_double_containers();
            demo_inventory_array();
            demo_order_list();
            demo_mixed_operations();
            println!();
            println!("========================================");
            println!("  All demos completed successfully!");
            println!("========================================");
        }
        7 => {
            println!();
            println!("Goodbye!");
            return false;
        }
        _ => {
            println!("Invalid choice");
        }
    }
    true
}
