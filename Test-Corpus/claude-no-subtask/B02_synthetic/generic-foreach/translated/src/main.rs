use std::io::{self, Read, Write};

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

fn cstr_to_str(buf: &[u8]) -> &str {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    // Safety: input is always ASCII string copied from a valid UTF-8 &str
    std::str::from_utf8(&buf[..len]).unwrap_or("")
}

fn fill_cstr(dst: &mut [u8], src: &str) {
    // Mimic strncpy semantics: copy up to dst.len()-1 bytes from src, null-terminate.
    for b in dst.iter_mut() {
        *b = 0;
    }
    let max = dst.len().saturating_sub(1);
    let bytes = src.as_bytes();
    let n = bytes.len().min(max);
    dst[..n].copy_from_slice(&bytes[..n]);
    dst[n] = 0;
}

fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    let mut item = Item {
        id,
        name: [0u8; MAX_NAME_LENGTH],
        category: [0u8; MAX_CATEGORY_LENGTH],
        price,
        quantity,
    };
    fill_cstr(&mut item.name, name);
    fill_cstr(&mut item.category, category);
    item
}

fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    let mut order = Order {
        customer_id,
        customer_name: [0u8; MAX_NAME_LENGTH],
        total_amount,
    };
    fill_cstr(&mut order.customer_name, customer_name);
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
    println!("Average: {:.2}", sum as f64 / int_array.len() as f64);

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
        product *= val as i64;
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

    let temps = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    print!("Adding temperatures: ");
    for &t in temps.iter() {
        double_array.push(t);
        print!("{:.1} ", t);
    }
    println!();

    let mut min_temp = temps[0];
    let mut max_temp = temps[0];
    let mut sum_temp = 0.0_f64;

    for &temp in double_array.iter() {
        if temp < min_temp {
            min_temp = temp;
        }
        if temp > max_temp {
            max_temp = temp;
        }
        sum_temp += temp;
    }

    println!("Minimum: {:.1}°C", min_temp);
    println!("Maximum: {:.1}°C", max_temp);
    println!("Average: {:.1}°C", sum_temp / double_array.len() as f64);

    let mut price_list: Vec<f64> = Vec::new();
    println!();
    println!("--- Double List (Product Prices) ---");

    let prices = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    print!("Adding prices: ");
    for &p in prices.iter() {
        price_list.push(p);
        print!("${:.2} ", p);
    }
    println!();

    let mut total = 0.0_f64;
    let mut count_under_10 = 0i32;

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
    let mut low_stock_count = 0i32;
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
    let mut large_order_count = 0i32;
    let mut large_order_total = 0.0_f64;

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

    let items = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items = items.len() as i32;

    for &item in items.iter() {
        array_inventory.push(item);
        list_inventory.push(item);
    }

    println!("Added {} items to both containers", num_items);

    println!();
    println!("--- Iterating through Array ---");
    let mut array_count = 0i32;
    for _item in array_inventory.iter() {
        array_count += 1;
    }
    println!("Array iteration count: {}", array_count);

    println!();
    println!("--- Iterating through List ---");
    let mut list_count = 0i32;
    for _item in list_inventory.iter() {
        list_count += 1;
    }
    println!("List iteration count: {}", list_count);

    let price_threshold = 200.0_f64;

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

/// Reads bytes from stdin until either a newline is included (\n kept), EOF, or
/// the buffer would overflow `capacity - 1` bytes (mimicking C's fgets).
fn fgets_line(stdin_bytes: &mut std::io::Bytes<io::StdinLock>, capacity: usize) -> Option<String> {
    let max = capacity.saturating_sub(1);
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < max {
        match stdin_bytes.next() {
            Some(Ok(b)) => {
                buf.push(b);
                if b == b'\n' {
                    break;
                }
            }
            Some(Err(_)) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
            None => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
        }
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Mimics `sscanf(input, "%d", &choice) == 1`: skip leading whitespace, then
/// parse an optional sign followed by decimal digits.
fn sscanf_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let start = i;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let digit_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digit_start {
        return None;
    }
    s[start..i].parse::<i32>().ok()
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║   GENERIC FOR_EACH MACRO DEMO         ║");
    println!("║   Demonstrating Generic Containers    ║");
    println!("╚════════════════════════════════════════╝");

    let stdin = io::stdin();
    let lock = stdin.lock();
    let mut bytes_iter = lock.bytes();

    loop {
        print_menu();

        let input = match fgets_line(&mut bytes_iter, 256) {
            Some(s) => s,
            None => break,
        };

        let choice = match sscanf_int(&input) {
            Some(c) => c,
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
                return;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
}
