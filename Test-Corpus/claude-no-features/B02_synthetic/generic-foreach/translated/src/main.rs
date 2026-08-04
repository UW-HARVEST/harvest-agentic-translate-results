// Rust translation of c_src/src/main.c + inventory.c
// Produces byte-identical output for the same inputs.

use std::io::{self, Read, Write};
use std::process::ExitCode;

const MAX_NAME_LENGTH: usize = 64;
const MAX_CATEGORY_LENGTH: usize = 32;

#[derive(Clone, Copy)]
struct Item {
    id: i32,
    name: [u8; MAX_NAME_LENGTH],
    category: [u8; MAX_CATEGORY_LENGTH],
    price: f64,
    quantity: i32,
}

#[derive(Clone, Copy)]
struct Order {
    customer_id: i32,
    customer_name: [u8; MAX_NAME_LENGTH],
    total_amount: f64,
}

fn copy_truncated(dst: &mut [u8], src: &str) {
    // Mirrors strncpy + manual NUL termination of last byte
    let max_copy = dst.len() - 1;
    let bytes = src.as_bytes();
    let n = bytes.len().min(max_copy);
    for i in 0..n {
        dst[i] = bytes[i];
    }
    // strncpy fills the rest with 0s up to max_copy
    for i in n..dst.len() {
        dst[i] = 0;
    }
    // explicit NUL at last byte (matches the C code's manual safety NUL)
    dst[dst.len() - 1] = 0;
}

fn cstr_to_str(buf: &[u8]) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    // Strings in this program are ASCII; this is safe.
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}

fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    let mut item = Item {
        id,
        name: [0u8; MAX_NAME_LENGTH],
        category: [0u8; MAX_CATEGORY_LENGTH],
        price,
        quantity,
    };
    copy_truncated(&mut item.name, name);
    copy_truncated(&mut item.category, category);
    item
}

fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    let mut order = Order {
        customer_id,
        customer_name: [0u8; MAX_NAME_LENGTH],
        total_amount,
    };
    copy_truncated(&mut order.customer_name, customer_name);
    order
}

fn print_item(item: &Item) {
    println!("  [{}] {}", item.id, cstr_to_str(&item.name));
    println!("      Category: {}", cstr_to_str(&item.category));
    println!("      Price: ${:.2}", item.price);
    println!("      Quantity: {}", item.quantity);
}

fn print_order(order: &Order) {
    println!(
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id,
        cstr_to_str(&order.customer_name)
    );
    println!("          Total: ${:.2}", order.total_amount);
}

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
    println!(
        "Average item price: ${:.2}",
        total_value / (total_items as f64)
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
        total_revenue / (orders.len() as f64)
    );
    println!("Largest order: ${:.2}", max_order);
    println!("Smallest order: ${:.2}", min_order);
}

fn find_items_by_category(items: &[Item], category: &str) {
    println!();
    println!("=== Items in category '{}' ===", category);

    let mut found = 0i32;
    for item in items.iter() {
        if cstr_to_str(&item.category) == category {
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

fn demo_integer_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 1: Integer Containers");
    println!("========================================");

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
    println!("Average: {:.2}", (sum as f64) / (int_array.len() as f64));

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

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    let mut double_array: Vec<f64> = Vec::with_capacity(5);
    println!();
    println!("--- Double Array (Temperatures in Celsius) ---");

    print!("Adding temperatures: ");
    for &t in temps.iter() {
        double_array.push(t);
        print!("{:.1} ", t);
    }
    println!();

    let mut min_temp: f64 = temps[0];
    let mut max_temp: f64 = temps[0];
    let mut sum_temp: f64 = 0.0;

    for &t in double_array.iter() {
        if t < min_temp {
            min_temp = t;
        }
        if t > max_temp {
            max_temp = t;
        }
        sum_temp += t;
    }

    println!("Minimum: {:.1}°C", min_temp);
    println!("Maximum: {:.1}°C", max_temp);
    println!("Average: {:.1}°C", sum_temp / (double_array.len() as f64));

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    let mut price_list: Vec<f64> = Vec::new();
    println!();
    println!("--- Double List (Product Prices) ---");

    print!("Adding prices: ");
    for &p in prices.iter() {
        price_list.push(p);
        print!("${:.2} ", p);
    }
    println!();

    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;
    for &t in price_list.iter() {
        total += t;
        if t < 10.0 {
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

    let items: [Item; 5] = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items = items.len() as i32;

    for it in items.iter() {
        array_inventory.push(*it);
        list_inventory.push(*it);
    }

    println!("Added {} items to both containers", num_items);

    println!();
    println!("--- Iterating through Array ---");
    let mut array_count: i32 = 0;
    for _it in array_inventory.iter() {
        array_count += 1;
    }
    println!("Array iteration count: {}", array_count);

    println!();
    println!("--- Iterating through List ---");
    let mut list_count: i32 = 0;
    for _it in list_inventory.iter() {
        list_count += 1;
    }
    println!("List iteration count: {}", list_count);

    let price_threshold: f64 = 200.0;

    println!();
    println!("--- Items above ${:.2} (Array) ---", price_threshold);
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", cstr_to_str(&item.name), item.price);
        }
    }

    println!();
    println!("--- Items above ${:.2} (List) ---", price_threshold);
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", cstr_to_str(&item.name), item.price);
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

// Mimic fgets: read up to (and including) a newline, or up to buffer_size-1 bytes,
// or until EOF. Returns None on EOF with no bytes read.
fn fgets(reader: &mut impl Read, buffer_size: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    let max_chars = buffer_size.saturating_sub(1);
    let mut byte = [0u8; 1];
    while out.len() < max_chars {
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

// Mimic sscanf(s, "%d", &x): skip leading whitespace, accept optional sign,
// parse base-10 digits. Returns count of items parsed (0 or 1) and value.
fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n' || s[i] == b'\r' || s[i] == 0x0b || s[i] == 0x0c) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    let mut had_digit = false;
    while i < s.len() && s[i].is_ascii_digit() {
        value = value.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        had_digit = true;
        i += 1;
    }
    if !had_digit {
        return None;
    }
    let _ = start;
    if neg {
        value = -value;
    }
    Some(value as i32)
}

fn run() -> i32 {
    println!("╔════════════════════════════════════════╗");
    println!("║   GENERIC FOR_EACH MACRO DEMO         ║");
    println!("║   Demonstrating Generic Containers    ║");
    println!("╚════════════════════════════════════════╝");

    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        print_menu();

        let line = match fgets(&mut handle, 256) {
            Some(l) => l,
            None => break,
        };

        let choice = match sscanf_int(&line) {
            Some(v) => v,
            None => {
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
                return 0;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }

    0
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
