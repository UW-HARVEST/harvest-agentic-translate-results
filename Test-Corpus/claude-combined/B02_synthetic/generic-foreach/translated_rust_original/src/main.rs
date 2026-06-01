// Rust translation of the C driver program.
// Reproduces the exact stdout output of the original C program byte-for-byte.

use std::io::{self, Read, Write};

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

fn truncate_to_max(s: &str, max_len: usize) -> String {
    // Mimic strncpy behavior: copy up to max_len-1 bytes and null-terminate.
    // Since the source strings are ASCII and short, this is straightforward.
    let bytes = s.as_bytes();
    let take = std::cmp::min(bytes.len(), max_len - 1);
    String::from_utf8_lossy(&bytes[..take]).into_owned()
}

fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: truncate_to_max(name, MAX_NAME_LENGTH),
        category: truncate_to_max(category, MAX_CATEGORY_LENGTH),
        price,
        quantity,
    }
}

fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: truncate_to_max(customer_name, MAX_NAME_LENGTH),
        total_amount,
    }
}

fn print_item(out: &mut impl Write, item: &Item) {
    let _ = writeln!(out, "  [{}] {}", item.id, item.name);
    let _ = writeln!(out, "      Category: {}", item.category);
    let _ = writeln!(out, "      Price: ${:.2}", item.price);
    let _ = writeln!(out, "      Quantity: {}", item.quantity);
}

fn print_order(out: &mut impl Write, order: &Order) {
    let _ = writeln!(
        out,
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id, order.customer_name
    );
    let _ = writeln!(out, "          Total: ${:.2}", order.total_amount);
}

fn calculate_inventory_stats(out: &mut impl Write, items: &[Item]) {
    if items.is_empty() {
        let _ = writeln!(out, "No items in inventory");
        return;
    }

    let _ = writeln!(out, "\n=== Inventory Statistics (Array) ===");

    let mut total_value: f64 = 0.0;
    let mut total_items: i32 = 0;
    let mut max_price: f64 = 0.0;
    let mut min_price: f64 = items[0].price;

    for item in items {
        total_value += item.price * item.quantity as f64;
        total_items += item.quantity;
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    let _ = writeln!(out, "Total unique items: {}", items.len());
    let _ = writeln!(out, "Total item count: {}", total_items);
    let _ = writeln!(out, "Total inventory value: ${:.2}", total_value);
    let _ = writeln!(
        out,
        "Average item price: ${:.2}",
        total_value / total_items as f64
    );
    let _ = writeln!(out, "Most expensive item: ${:.2}", max_price);
    let _ = writeln!(out, "Least expensive item: ${:.2}", min_price);
}

fn calculate_order_stats(out: &mut impl Write, orders: &[Order]) {
    if orders.is_empty() {
        let _ = writeln!(out, "No orders to analyze");
        return;
    }

    let _ = writeln!(out, "\n=== Order Statistics (List) ===");

    let mut total_revenue: f64 = 0.0;
    let mut max_order: f64 = 0.0;
    let mut min_order: f64 = -1.0;

    for order in orders {
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
    }

    let _ = writeln!(out, "Total orders: {}", orders.len());
    let _ = writeln!(out, "Total revenue: ${:.2}", total_revenue);
    let _ = writeln!(
        out,
        "Average order value: ${:.2}",
        total_revenue / orders.len() as f64
    );
    let _ = writeln!(out, "Largest order: ${:.2}", max_order);
    let _ = writeln!(out, "Smallest order: ${:.2}", min_order);
}

fn find_items_by_category(out: &mut impl Write, items: &[Item], category: &str) {
    let _ = writeln!(out, "\n=== Items in category '{}' ===", category);

    let mut found = 0i32;
    for item in items {
        if item.category == category {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        let _ = writeln!(out, "No items found in this category");
    } else {
        let _ = writeln!(out, "Found {} items", found);
    }
}

#[allow(dead_code)]
fn find_expensive_items(out: &mut impl Write, items: &[Item], min_price: f64) {
    let _ = writeln!(out, "\n=== Items priced above ${:.2} ===", min_price);

    let mut found = 0i32;
    for item in items {
        if item.price >= min_price {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        let _ = writeln!(out, "No items found above this price");
    } else {
        let _ = writeln!(out, "Found {} items", found);
    }
}

fn print_menu(out: &mut impl Write) {
    let _ = writeln!(out);
    let _ = writeln!(out, "========================================");
    let _ = writeln!(out, "  GENERIC FOR_EACH MACRO DEMO");
    let _ = writeln!(out, "========================================");
    let _ = writeln!(out, "1. Demo: Integer Containers");
    let _ = writeln!(out, "2. Demo: Double Containers");
    let _ = writeln!(out, "3. Demo: Inventory Array");
    let _ = writeln!(out, "4. Demo: Order List");
    let _ = writeln!(out, "5. Demo: Mixed Operations");
    let _ = writeln!(out, "6. Run All Demos");
    let _ = writeln!(out, "7. Exit");
    let _ = writeln!(out, "========================================");
    let _ = write!(out, "Choice: ");
}

fn demo_integer_containers(out: &mut impl Write) {
    let _ = writeln!(out);
    let _ = writeln!(out, "========================================");
    let _ = writeln!(out, "  DEMO 1: Integer Containers");
    let _ = writeln!(out, "========================================");

    // Integer "array"
    let mut int_array: Vec<i32> = Vec::with_capacity(10);
    let _ = writeln!(out, "\n--- Integer Array ---");
    let _ = writeln!(out, "Adding integers: 10, 20, 30, 40, 50");
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);

    let _ = write!(out, "Array contents: ");
    for &val in &int_array {
        let _ = write!(out, "{} ", val);
    }
    let _ = writeln!(out);

    let mut sum: i32 = 0;
    for &val in &int_array {
        sum += val;
    }
    let _ = writeln!(out, "Sum: {}", sum);
    let _ = writeln!(out, "Average: {:.2}", sum as f64 / int_array.len() as f64);

    // Integer "list"
    let mut int_list: Vec<i32> = Vec::new();
    let _ = writeln!(out, "\n--- Integer List ---");
    let _ = writeln!(out, "Adding integers: 100, 200, 300, 400, 500");
    int_list.push(100);
    int_list.push(200);
    int_list.push(300);
    int_list.push(400);
    int_list.push(500);

    let _ = write!(out, "List contents: ");
    for &val in &int_list {
        let _ = write!(out, "{} ", val);
    }
    let _ = writeln!(out);

    let mut product: i64 = 1;
    for &val in &int_list {
        product = product.wrapping_mul(val as i64);
    }
    let _ = writeln!(out, "Product: {}", product);
}

fn demo_double_containers(out: &mut impl Write) {
    let _ = writeln!(out);
    let _ = writeln!(out, "========================================");
    let _ = writeln!(out, "  DEMO 2: Double Containers");
    let _ = writeln!(out, "========================================");

    let mut double_array: Vec<f64> = Vec::with_capacity(5);
    let _ = writeln!(out, "\n--- Double Array (Temperatures in Celsius) ---");

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    let _ = write!(out, "Adding temperatures: ");
    for &t in temps.iter() {
        double_array.push(t);
        let _ = write!(out, "{:.1} ", t);
    }
    let _ = writeln!(out);

    let mut min_temp = temps[0];
    let mut max_temp = temps[0];
    let mut sum_temp: f64 = 0.0;

    for &temp in &double_array {
        if temp < min_temp {
            min_temp = temp;
        }
        if temp > max_temp {
            max_temp = temp;
        }
        sum_temp += temp;
    }

    let _ = writeln!(out, "Minimum: {:.1}°C", min_temp);
    let _ = writeln!(out, "Maximum: {:.1}°C", max_temp);
    let _ = writeln!(
        out,
        "Average: {:.1}°C",
        sum_temp / double_array.len() as f64
    );

    let mut price_list: Vec<f64> = Vec::new();
    let _ = writeln!(out, "\n--- Double List (Product Prices) ---");

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    let _ = write!(out, "Adding prices: ");
    for &p in prices.iter() {
        price_list.push(p);
        let _ = write!(out, "${:.2} ", p);
    }
    let _ = writeln!(out);

    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;

    for &temp in &price_list {
        total += temp;
        if temp < 10.0 {
            count_under_10 += 1;
        }
    }

    let _ = writeln!(out, "Total cost: ${:.2}", total);
    let _ = writeln!(out, "Items under $10: {}", count_under_10);
}

fn demo_inventory_array(out: &mut impl Write) {
    let _ = writeln!(out);
    let _ = writeln!(out, "========================================");
    let _ = writeln!(out, "  DEMO 3: Inventory Array (Items)");
    let _ = writeln!(out, "========================================");

    let mut inventory: Vec<Item> = Vec::with_capacity(20);

    let _ = writeln!(out, "\n--- Adding Items to Inventory ---");
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

    let _ = writeln!(out, "Added {} items to inventory", inventory.len());

    let _ = writeln!(out, "\n--- All Inventory Items ---");
    for item in &inventory {
        print_item(out, item);
        let _ = writeln!(out);
    }

    calculate_inventory_stats(out, &inventory);

    find_items_by_category(out, &inventory, "Electronics");
    find_items_by_category(out, &inventory, "Furniture");

    let _ = writeln!(out, "\n--- Low Stock Items (< 20) ---");
    let mut low_stock_count = 0i32;
    for item in &inventory {
        if item.quantity < 20 {
            print_item(out, item);
            low_stock_count += 1;
        }
    }
    let _ = writeln!(out, "Total low stock items: {}", low_stock_count);
}

fn demo_order_list(out: &mut impl Write) {
    let _ = writeln!(out);
    let _ = writeln!(out, "========================================");
    let _ = writeln!(out, "  DEMO 4: Order List (Orders)");
    let _ = writeln!(out, "========================================");

    let mut orders: Vec<Order> = Vec::new();

    let _ = writeln!(out, "\n--- Adding Orders ---");
    orders.push(create_order(1001, "Alice Johnson", 1249.95));
    orders.push(create_order(1002, "Bob Smith", 89.99));
    orders.push(create_order(1003, "Carol White", 549.98));
    orders.push(create_order(1004, "David Brown", 24.99));
    orders.push(create_order(1005, "Eve Davis", 899.99));
    orders.push(create_order(1006, "Frank Miller", 374.97));
    orders.push(create_order(1007, "Grace Lee", 159.98));
    orders.push(create_order(1008, "Henry Wilson", 1099.99));

    let _ = writeln!(out, "Added {} orders", orders.len());

    let _ = writeln!(out, "\n--- All Orders ---");
    for order in &orders {
        print_order(out, order);
    }

    calculate_order_stats(out, &orders);

    let _ = writeln!(out, "\n--- Large Orders (> $500) ---");
    let mut large_order_count = 0i32;
    let mut large_order_total: f64 = 0.0;

    for order in &orders {
        if order.total_amount > 500.0 {
            print_order(out, order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    let _ = writeln!(out, "Total large orders: {}", large_order_count);
    let _ = writeln!(out, "Revenue from large orders: ${:.2}", large_order_total);
}

fn demo_mixed_operations(out: &mut impl Write) {
    let _ = writeln!(out);
    let _ = writeln!(out, "========================================");
    let _ = writeln!(out, "  DEMO 5: Mixed Operations");
    let _ = writeln!(out, "========================================");

    let mut array_inventory: Vec<Item> = Vec::with_capacity(10);
    let mut list_inventory: Vec<Item> = Vec::new();

    let _ = writeln!(out, "\n--- Populating both Array and List ---");

    let items: [Item; 5] = [
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

    let _ = writeln!(out, "Added {} items to both containers", num_items);

    let _ = writeln!(out, "\n--- Iterating through Array ---");
    let mut array_count = 0i32;
    for _item in &array_inventory {
        array_count += 1;
    }
    let _ = writeln!(out, "Array iteration count: {}", array_count);

    let _ = writeln!(out, "\n--- Iterating through List ---");
    let mut list_count = 0i32;
    for _item in &list_inventory {
        list_count += 1;
    }
    let _ = writeln!(out, "List iteration count: {}", list_count);

    let price_threshold: f64 = 200.0;

    let _ = writeln!(
        out,
        "\n--- Items above ${:.2} (Array) ---",
        price_threshold
    );
    for item in &array_inventory {
        if item.price >= price_threshold {
            let _ = writeln!(out, "  {}: ${:.2}", item.name, item.price);
        }
    }

    let _ = writeln!(
        out,
        "\n--- Items above ${:.2} (List) ---",
        price_threshold
    );
    for item in &list_inventory {
        if item.price >= price_threshold {
            let _ = writeln!(out, "  {}: ${:.2}", item.name, item.price);
        }
    }
}

/// Read entire stdin into a buffer (we'll process line-by-line below).
fn read_all_stdin() -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = io::stdin().read_to_end(&mut buf);
    buf
}

/// Mimics fgets: returns the next line (including any trailing '\n')
/// up to a buffer size of `size - 1` bytes (one less for the null terminator
/// like the C version). Returns None if there's no more input.
fn fgets<'a>(input: &'a [u8], pos: &mut usize, size: usize) -> Option<&'a [u8]> {
    if *pos >= input.len() {
        return None;
    }
    let start = *pos;
    let max_take = size - 1; // C: fgets reads at most size-1 chars
    let mut end = start;
    let limit = std::cmp::min(input.len(), start + max_take);
    while end < limit {
        let c = input[end];
        end += 1;
        if c == b'\n' {
            break;
        }
    }
    *pos = end;
    Some(&input[start..end])
}

/// Mimics sscanf("%d", ...): parses leading whitespace then a signed integer.
/// Returns Some(i32) if a valid integer was read, else None.
fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    // Skip whitespace (matching isspace in C locale: space, tab, newline, etc.)
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    if i >= s.len() {
        return None;
    }
    let mut neg = false;
    if s[i] == b'+' {
        i += 1;
    } else if s[i] == b'-' {
        neg = true;
        i += 1;
    }
    let digit_start = i;
    let mut value: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == digit_start {
        return None;
    }
    if neg {
        value = value.wrapping_neg();
    }
    // Truncate to i32 the way C does on overflow (implementation defined,
    // but practically equivalent for our use).
    Some(value as i32)
}

fn main() {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    {
        let out = &mut handle;
        let _ = writeln!(out, "╔════════════════════════════════════════╗");
        let _ = writeln!(out, "║   GENERIC FOR_EACH MACRO DEMO         ║");
        let _ = writeln!(out, "║   Demonstrating Generic Containers    ║");
        let _ = writeln!(out, "╚════════════════════════════════════════╝");
    }

    let input = read_all_stdin();
    let mut pos = 0usize;

    loop {
        print_menu(&mut handle);
        let _ = handle.flush();

        let line = match fgets(&input, &mut pos, 256) {
            Some(l) => l,
            None => break,
        };

        let choice = match sscanf_int(line) {
            Some(c) => c,
            None => {
                let _ = writeln!(handle, "Invalid input");
                continue;
            }
        };

        match choice {
            1 => demo_integer_containers(&mut handle),
            2 => demo_double_containers(&mut handle),
            3 => demo_inventory_array(&mut handle),
            4 => demo_order_list(&mut handle),
            5 => demo_mixed_operations(&mut handle),
            6 => {
                let _ = writeln!(handle, "\n=== Running All Demos ===");
                demo_integer_containers(&mut handle);
                demo_double_containers(&mut handle);
                demo_inventory_array(&mut handle);
                demo_order_list(&mut handle);
                demo_mixed_operations(&mut handle);
                let _ = writeln!(handle, "\n========================================");
                let _ = writeln!(handle, "  All demos completed successfully!");
                let _ = writeln!(handle, "========================================");
            }
            7 => {
                let _ = writeln!(handle, "\nGoodbye!");
                return;
            }
            _ => {
                let _ = writeln!(handle, "Invalid choice");
            }
        }
    }
}
