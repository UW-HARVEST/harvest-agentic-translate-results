
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// (stub removed — real implementation is defined below as `main_main`)

// Idiomatic Rust reimplementation of the C inventory / demo code.
//
// The original C code defined POD structs (item_t, order_t) and instantiated
// generic array / linked-list containers via macros. Rather than mimicking
// the C layouts (fixed-size char arrays, opaque bindgen structs), we use
// idiomatic Rust types: owned `String`s for text, `Vec<T>` for arrays, and
// `std::collections::LinkedList<T>` for the linked lists. This eliminates
// any need for `unsafe`, raw pointers, manual char-array packing, or
// bindgen-generated struct construction.

use std::collections::LinkedList;

// ---------- Idiomatic Rust equivalents of item_t and order_t ----------

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

// ---------- Constructors (mirror create_item / create_order) ----------

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

// ---------- Printing helpers ----------

pub fn print_item(item: &Item) {
    println!("  [{}] {}", item.id, item.name);
    println!("      Category: {}", item.category);
    println!("      Price: ${:.2}", item.price);
    println!("      Quantity: {}", item.quantity);
}

pub fn print_order(order: &Order) {
    println!("  Customer [{}] {}", order.customer_id, order.customer_name);
    println!("      Total: ${:.2}", order.total_amount);
}

// ---------- calculate_inventory_stats ----------

pub fn calculate_inventory_stats(items: &[Item]) {
    if items.is_empty() {
        println!("No items in inventory");
        return;
    }

    println!("\n=== Inventory Statistics (Array) ===");

    let mut total_value: f64 = 0.0;
    let mut total_items: i32 = 0;
    let mut max_price: f64 = 0.0;
    let mut min_price: f64 = items[0].price;

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

    println!("Total unique items: {}", items.len());
    println!("Total item count: {}", total_items);
    println!("Total inventory value: ${:.2}", total_value);
    if total_items != 0 {
        println!("Average item price: ${:.2}", total_value / (total_items as f64));
    } else {
        println!("Average item price: $inf");
    }
    println!("Most expensive item: ${:.2}", max_price);
    println!("Least expensive item: ${:.2}", min_price);
}

// ---------- calculate_order_stats ----------

pub fn calculate_order_stats(orders: &LinkedList<Order>) {
    if orders.is_empty() {
        println!("No orders to analyze");
        return;
    }

    println!("\n=== Order Statistics (List) ===");

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
        total_revenue / (orders.len() as f64)
    );
    println!("Largest order: ${:.2}", max_order);
    println!("Smallest order: ${:.2}", min_order);
}

// ---------- find_items_by_category ----------

pub fn find_items_by_category(items: &[Item], category: &str) {
    println!("\n=== Items in category '{}' ===", category);

    let mut found: i32 = 0;
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

// ---------- find_expensive_items ----------

pub fn find_expensive_items(items: &LinkedList<Item>, min_price: f64) {
    println!("\n=== Items priced at ${:.2} or more ===", min_price);

    let mut found: i32 = 0;
    for item in items.iter() {
        if item.price >= min_price {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        println!("No expensive items found");
    } else {
        println!("Found {} expensive items", found);
    }
}

// ---------- demo_double_containers ----------

pub fn demo_double_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 2: Double Containers");
    println!("========================================");

    // Double array (Vec<f64>)
    let mut double_array: Vec<f64> = Vec::with_capacity(5);
    println!("\n--- Double Array (Temperatures in Celsius) ---");

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

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
        sum_temp / (double_array.len() as f64)
    );

    // Double linked list (LinkedList<f64>)
    let mut price_list: LinkedList<f64> = LinkedList::new();
    println!("\n--- Double List (Product Prices) ---");

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    print!("Adding prices: ");
    for &p in prices.iter() {
        price_list.push_back(p);
        print!("${:.2} ", p);
    }
    println!();

    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;

    for &v in price_list.iter() {
        total += v;
        if v < 10.0 {
            count_under_10 += 1;
        }
    }

    println!("Total cost: ${:.2}", total);
    println!("Items under $10: {}", count_under_10);

    // Cleanup is automatic in Rust (Drop).
}

// ---------- demo_integer_containers ----------

pub fn demo_integer_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 1: Integer Containers");
    println!("========================================");

    // Integer array
    let mut int_array: Vec<i32> = Vec::with_capacity(10);
    println!("\n--- Integer Array ---");
    println!("Adding integers: 10, 20, 30, 40, 50");
    int_array.extend_from_slice(&[10, 20, 30, 40, 50]);

    print!("Array contents: ");
    for &v in int_array.iter() {
        print!("{} ", v);
    }
    println!();

    let sum: i32 = int_array.iter().sum();
    println!("Sum: {}", sum);
    println!(
        "Average: {:.2}",
        (sum as f64) / (int_array.len() as f64)
    );

    // Integer linked list
    let mut int_list: LinkedList<i32> = LinkedList::new();
    println!("\n--- Integer List ---");
    println!("Adding integers: 100, 200, 300, 400, 500");
    for &v in &[100, 200, 300, 400, 500] {
        int_list.push_back(v);
    }

    print!("List contents: ");
    for &v in int_list.iter() {
        print!("{} ", v);
    }
    println!();

    let mut product: i64 = 1;
    for &v in int_list.iter() {
        product = product.wrapping_mul(v as i64);
    }
    println!("Product: {}", product);
}

// ---------- demo_inventory_array ----------

pub fn demo_inventory_array() {
    println!();
    println!("========================================");
    println!("  DEMO 3: Inventory Array (Items)");
    println!("========================================");

    let mut inventory: Vec<Item> = Vec::with_capacity(20);

    println!("\n--- Adding Items to Inventory ---");
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

    println!("\n--- All Inventory Items ---");
    for item in inventory.iter() {
        print_item(item);
        println!();
    }

    calculate_inventory_stats(&inventory);

    find_items_by_category(&inventory, "Electronics");
    find_items_by_category(&inventory, "Furniture");

    println!("\n--- Low Stock Items (< 20) ---");
    let mut low_stock_count: i32 = 0;
    for item in inventory.iter() {
        if item.quantity < 20 {
            print_item(item);
            low_stock_count += 1;
        }
    }
    println!("Total low stock items: {}", low_stock_count);

    // Cleanup automatic via Drop.
}

// ---------- demo_mixed_operations ----------

pub fn demo_mixed_operations() {
    println!();
    println!("========================================");
    println!("  DEMO 5: Mixed Operations");
    println!("========================================");

    println!("\n--- Populating both Array and List ---");

    let items: Vec<Item> = vec![
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items = items.len();

    let array_inventory: Vec<Item> = items.clone();
    let list_inventory: LinkedList<Item> = items.iter().cloned().collect();

    println!("Added {} items to both containers", num_items);

    println!("\n--- Iterating through Array ---");
    let array_count = array_inventory.iter().count();
    println!("Array iteration count: {}", array_count);

    println!("\n--- Iterating through List ---");
    let list_count = list_inventory.iter().count();
    println!("List iteration count: {}", list_count);

    let price_threshold: f64 = 200.0;

    println!("\n--- Items above ${:.2} (Array) ---", price_threshold);
    for item in array_inventory.iter().filter(|it| it.price >= price_threshold) {
        println!("  {}: ${:.2}", item.name, item.price);
    }

    println!("\n--- Items above ${:.2} (List) ---", price_threshold);
    for item in list_inventory.iter().filter(|it| it.price >= price_threshold) {
        println!("  {}: ${:.2}", item.name, item.price);
    }
}

// ---------- print_order (order_t variant used in demo_order_list) ----------
// The existing `print_order(&Order)` in lib.rs uses a different output
// format. The C variant used inside `demo_order_list` prints a different
// layout, so provide a distinct helper here.

fn print_order_detailed(order: &Order) {
    println!(
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id, order.customer_name
    );
    println!("          Total: ${:.2}", order.total_amount);
}

// ---------- demo_order_list ----------

pub fn demo_order_list() {
    println!();
    println!("========================================");
    println!("  DEMO 4: Order List (Orders)");
    println!("========================================");

    println!("\n--- Adding Orders ---");
    let orders: LinkedList<Order> = [
        create_order(1001, "Alice Johnson", 1249.95),
        create_order(1002, "Bob Smith", 89.99),
        create_order(1003, "Carol White", 549.98),
        create_order(1004, "David Brown", 24.99),
        create_order(1005, "Eve Davis", 899.99),
        create_order(1006, "Frank Miller", 374.97),
        create_order(1007, "Grace Lee", 159.98),
        create_order(1008, "Henry Wilson", 1099.99),
    ]
    .into_iter()
    .collect();

    println!("Added {} orders", orders.len());

    println!("\n--- All Orders ---");
    for order in orders.iter() {
        print_order_detailed(order);
    }

    calculate_order_stats(&orders);

    println!("\n--- Large Orders (> $500) ---");
    let mut large_order_count: i32 = 0;
    let mut large_order_total: f64 = 0.0;

    for order in orders.iter().filter(|o| o.total_amount > 500.0) {
        print_order_detailed(order);
        large_order_count += 1;
        large_order_total += order.total_amount;
    }

    println!("Total large orders: {}", large_order_count);
    println!("Revenue from large orders: ${:.2}", large_order_total);
}

// ---------- print_menu ----------

pub fn print_menu() {
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
}

// ---------- main entry point (FFI boundary, called from C `main`) ----------

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> core::ffi::c_int {
    use std::io::{self, BufRead, Write};

    println!("\u{2554}{}\u{2557}", "\u{2550}".repeat(40));
    println!("\u{2551}   GENERIC FOR_EACH MACRO DEMO         \u{2551}");
    println!("\u{2551}   Demonstrating Generic Containers    \u{2551}");
    println!("\u{255A}{}\u{255D}", "\u{2550}".repeat(40));

    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        print_menu();
        let _ = io::stdout().flush();

        let mut input = String::new();
        match handle.read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let choice: i32 = match input.trim().parse::<i32>() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => demo_integer_containers(),
            2 => demo_double_containers(),
            3 => demo_inventory_array(),
            4 => demo_order_list(),
            5 => demo_mixed_operations(),
            6 => {
                println!("\n=== Running All Demos ===");
                demo_integer_containers();
                demo_double_containers();
                demo_inventory_array();
                demo_order_list();
                demo_mixed_operations();
                println!("\n========================================");
                println!("  All demos completed successfully!");
                println!("========================================");
            }
            7 => {
                println!("\nGoodbye!");
                return 0;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }

    0
}

