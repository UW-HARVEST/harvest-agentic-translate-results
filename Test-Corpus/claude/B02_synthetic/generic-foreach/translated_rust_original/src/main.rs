use std::io::{self, BufRead, Write};

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

fn truncate_to_c_string(s: &str, max_len_with_nul: usize) -> String {
    // Mimics C's strncpy(dest, src, max_len_with_nul - 1) followed by setting last byte to '\0'.
    // The maximum number of usable chars (not counting NUL terminator) is max_len_with_nul - 1.
    let max_chars = max_len_with_nul - 1;
    let bytes = s.as_bytes();
    if bytes.len() <= max_chars {
        s.to_string()
    } else {
        // Truncate at byte boundary, keeping valid UTF-8 if possible.
        // For ASCII inputs (which all test inputs are) this is straightforward.
        let mut end = max_chars;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        s[..end].to_string()
    }
}

fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: truncate_to_c_string(name, MAX_NAME_LENGTH),
        category: truncate_to_c_string(category, MAX_CATEGORY_LENGTH),
        price,
        quantity,
    }
}

fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: truncate_to_c_string(customer_name, MAX_NAME_LENGTH),
        total_amount,
    }
}

fn print_item<W: Write>(out: &mut W, item: &Item) {
    writeln!(out, "  [{}] {}", item.id, item.name).unwrap();
    writeln!(out, "      Category: {}", item.category).unwrap();
    writeln!(out, "      Price: ${:.2}", item.price).unwrap();
    writeln!(out, "      Quantity: {}", item.quantity).unwrap();
}

fn print_order<W: Write>(out: &mut W, order: &Order) {
    writeln!(
        out,
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id, order.customer_name
    )
    .unwrap();
    writeln!(out, "          Total: ${:.2}", order.total_amount).unwrap();
}

fn calculate_inventory_stats<W: Write>(out: &mut W, items: &[Item]) {
    if items.is_empty() {
        writeln!(out, "No items in inventory").unwrap();
        return;
    }

    writeln!(out, "\n=== Inventory Statistics (Array) ===").unwrap();

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

    writeln!(out, "Total unique items: {}", items.len()).unwrap();
    writeln!(out, "Total item count: {}", total_items).unwrap();
    writeln!(out, "Total inventory value: ${:.2}", total_value).unwrap();
    writeln!(
        out,
        "Average item price: ${:.2}",
        total_value / (total_items as f64)
    )
    .unwrap();
    writeln!(out, "Most expensive item: ${:.2}", max_price).unwrap();
    writeln!(out, "Least expensive item: ${:.2}", min_price).unwrap();
}

fn calculate_order_stats<W: Write>(out: &mut W, orders: &[Order]) {
    if orders.is_empty() {
        writeln!(out, "No orders to analyze").unwrap();
        return;
    }

    writeln!(out, "\n=== Order Statistics (List) ===").unwrap();

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

    writeln!(out, "Total orders: {}", orders.len()).unwrap();
    writeln!(out, "Total revenue: ${:.2}", total_revenue).unwrap();
    writeln!(
        out,
        "Average order value: ${:.2}",
        total_revenue / (orders.len() as f64)
    )
    .unwrap();
    writeln!(out, "Largest order: ${:.2}", max_order).unwrap();
    writeln!(out, "Smallest order: ${:.2}", min_order).unwrap();
}

fn find_items_by_category<W: Write>(out: &mut W, items: &[Item], category: &str) {
    writeln!(out, "\n=== Items in category '{}' ===", category).unwrap();

    let mut found = 0;
    for item in items.iter() {
        if item.category == category {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        writeln!(out, "No items found in this category").unwrap();
    } else {
        writeln!(out, "Found {} items", found).unwrap();
    }
}

#[allow(dead_code)]
fn find_expensive_items<W: Write>(out: &mut W, items: &[Item], min_price: f64) {
    writeln!(out, "\n=== Items priced above ${:.2} ===", min_price).unwrap();

    let mut found = 0;
    for item in items.iter() {
        if item.price >= min_price {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        writeln!(out, "No items found above this price").unwrap();
    } else {
        writeln!(out, "Found {} items", found).unwrap();
    }
}

fn print_menu<W: Write>(out: &mut W) {
    writeln!(out).unwrap();
    writeln!(out, "========================================").unwrap();
    writeln!(out, "  GENERIC FOR_EACH MACRO DEMO").unwrap();
    writeln!(out, "========================================").unwrap();
    writeln!(out, "1. Demo: Integer Containers").unwrap();
    writeln!(out, "2. Demo: Double Containers").unwrap();
    writeln!(out, "3. Demo: Inventory Array").unwrap();
    writeln!(out, "4. Demo: Order List").unwrap();
    writeln!(out, "5. Demo: Mixed Operations").unwrap();
    writeln!(out, "6. Run All Demos").unwrap();
    writeln!(out, "7. Exit").unwrap();
    writeln!(out, "========================================").unwrap();
    write!(out, "Choice: ").unwrap();
    out.flush().unwrap();
}

fn demo_integer_containers<W: Write>(out: &mut W) {
    writeln!(out).unwrap();
    writeln!(out, "========================================").unwrap();
    writeln!(out, "  DEMO 1: Integer Containers").unwrap();
    writeln!(out, "========================================").unwrap();

    // Create integer array
    let mut int_array: Vec<i32> = Vec::with_capacity(10);
    writeln!(out, "\n--- Integer Array ---").unwrap();
    writeln!(out, "Adding integers: 10, 20, 30, 40, 50").unwrap();
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);

    write!(out, "Array contents: ").unwrap();
    for &val in int_array.iter() {
        write!(out, "{} ", val).unwrap();
    }
    writeln!(out).unwrap();

    // Calculate sum using ARRAY_FOREACH
    let mut sum: i32 = 0;
    for &val in int_array.iter() {
        sum += val;
    }
    writeln!(out, "Sum: {}", sum).unwrap();
    writeln!(
        out,
        "Average: {:.2}",
        (sum as f64) / (int_array.len() as f64)
    )
    .unwrap();

    // Create integer list
    let mut int_list: Vec<i32> = Vec::new();
    writeln!(out, "\n--- Integer List ---").unwrap();
    writeln!(out, "Adding integers: 100, 200, 300, 400, 500").unwrap();
    int_list.push(100);
    int_list.push(200);
    int_list.push(300);
    int_list.push(400);
    int_list.push(500);

    write!(out, "List contents: ").unwrap();
    for &val in int_list.iter() {
        write!(out, "{} ", val).unwrap();
    }
    writeln!(out).unwrap();

    // Calculate product using LIST_FOREACH
    let mut product: i64 = 1;
    for &val in int_list.iter() {
        product = product.wrapping_mul(val as i64);
    }
    writeln!(out, "Product: {}", product).unwrap();
}

fn demo_double_containers<W: Write>(out: &mut W) {
    writeln!(out).unwrap();
    writeln!(out, "========================================").unwrap();
    writeln!(out, "  DEMO 2: Double Containers").unwrap();
    writeln!(out, "========================================").unwrap();

    // Create double array
    let mut double_array: Vec<f64> = Vec::with_capacity(5);
    writeln!(out, "\n--- Double Array (Temperatures in Celsius) ---").unwrap();

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    write!(out, "Adding temperatures: ").unwrap();
    for &t in temps.iter() {
        double_array.push(t);
        write!(out, "{:.1} ", t).unwrap();
    }
    writeln!(out).unwrap();

    // Find min, max, average using ARRAY_FOREACH
    let mut min_temp: f64 = temps[0];
    let mut max_temp: f64 = temps[0];
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

    writeln!(out, "Minimum: {:.1}\u{00B0}C", min_temp).unwrap();
    writeln!(out, "Maximum: {:.1}\u{00B0}C", max_temp).unwrap();
    writeln!(
        out,
        "Average: {:.1}\u{00B0}C",
        sum_temp / (double_array.len() as f64)
    )
    .unwrap();

    // Create double list
    let mut price_list: Vec<f64> = Vec::new();
    writeln!(out, "\n--- Double List (Product Prices) ---").unwrap();

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    write!(out, "Adding prices: ").unwrap();
    for &p in prices.iter() {
        price_list.push(p);
        write!(out, "${:.2} ", p).unwrap();
    }
    writeln!(out).unwrap();

    // Calculate total and find items under $10 using LIST_FOREACH
    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;

    for &temp in price_list.iter() {
        total += temp;
        if temp < 10.0 {
            count_under_10 += 1;
        }
    }

    writeln!(out, "Total cost: ${:.2}", total).unwrap();
    writeln!(out, "Items under $10: {}", count_under_10).unwrap();
}

fn demo_inventory_array<W: Write>(out: &mut W) {
    writeln!(out).unwrap();
    writeln!(out, "========================================").unwrap();
    writeln!(out, "  DEMO 3: Inventory Array (Items)").unwrap();
    writeln!(out, "========================================").unwrap();

    let mut inventory: Vec<Item> = Vec::with_capacity(20);

    // Add items
    writeln!(out, "\n--- Adding Items to Inventory ---").unwrap();
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

    writeln!(out, "Added {} items to inventory", inventory.len()).unwrap();

    // Display all items using ARRAY_FOREACH
    writeln!(out, "\n--- All Inventory Items ---").unwrap();
    for item in inventory.iter() {
        print_item(out, item);
        writeln!(out).unwrap();
    }

    calculate_inventory_stats(out, &inventory);

    find_items_by_category(out, &inventory, "Electronics");
    find_items_by_category(out, &inventory, "Furniture");

    // Find low stock items using ARRAY_FOREACH
    writeln!(out, "\n--- Low Stock Items (< 20) ---").unwrap();
    let mut low_stock_count = 0;
    for item in inventory.iter() {
        if item.quantity < 20 {
            print_item(out, item);
            low_stock_count += 1;
        }
    }
    writeln!(out, "Total low stock items: {}", low_stock_count).unwrap();
}

fn demo_order_list<W: Write>(out: &mut W) {
    writeln!(out).unwrap();
    writeln!(out, "========================================").unwrap();
    writeln!(out, "  DEMO 4: Order List (Orders)").unwrap();
    writeln!(out, "========================================").unwrap();

    let mut orders: Vec<Order> = Vec::new();

    // Add orders
    writeln!(out, "\n--- Adding Orders ---").unwrap();
    orders.push(create_order(1001, "Alice Johnson", 1249.95));
    orders.push(create_order(1002, "Bob Smith", 89.99));
    orders.push(create_order(1003, "Carol White", 549.98));
    orders.push(create_order(1004, "David Brown", 24.99));
    orders.push(create_order(1005, "Eve Davis", 899.99));
    orders.push(create_order(1006, "Frank Miller", 374.97));
    orders.push(create_order(1007, "Grace Lee", 159.98));
    orders.push(create_order(1008, "Henry Wilson", 1099.99));

    writeln!(out, "Added {} orders", orders.len()).unwrap();

    // Display all orders using LIST_FOREACH
    writeln!(out, "\n--- All Orders ---").unwrap();
    for order in orders.iter() {
        print_order(out, order);
    }

    calculate_order_stats(out, &orders);

    // Find large orders using LIST_FOREACH
    writeln!(out, "\n--- Large Orders (> $500) ---").unwrap();
    let mut large_order_count: i32 = 0;
    let mut large_order_total: f64 = 0.0;

    for order in orders.iter() {
        if order.total_amount > 500.0 {
            print_order(out, order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    writeln!(out, "Total large orders: {}", large_order_count).unwrap();
    writeln!(
        out,
        "Revenue from large orders: ${:.2}",
        large_order_total
    )
    .unwrap();
}

fn demo_mixed_operations<W: Write>(out: &mut W) {
    writeln!(out).unwrap();
    writeln!(out, "========================================").unwrap();
    writeln!(out, "  DEMO 5: Mixed Operations").unwrap();
    writeln!(out, "========================================").unwrap();

    let mut array_inventory: Vec<Item> = Vec::with_capacity(10);
    let mut list_inventory: Vec<Item> = Vec::new();

    writeln!(out, "\n--- Populating both Array and List ---").unwrap();

    let items: [Item; 5] = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items = items.len() as i32;

    for item in items.iter() {
        array_inventory.push(item.clone());
        list_inventory.push(item.clone());
    }

    writeln!(out, "Added {} items to both containers", num_items).unwrap();

    // Compare iteration performance (conceptually)
    writeln!(out, "\n--- Iterating through Array ---").unwrap();
    let mut array_count: i32 = 0;
    for _item in array_inventory.iter() {
        array_count += 1;
    }
    writeln!(out, "Array iteration count: {}", array_count).unwrap();

    writeln!(out, "\n--- Iterating through List ---").unwrap();
    let mut list_count: i32 = 0;
    for _item in list_inventory.iter() {
        list_count += 1;
    }
    writeln!(out, "List iteration count: {}", list_count).unwrap();

    let price_threshold: f64 = 200.0;

    writeln!(out, "\n--- Items above ${:.2} (Array) ---", price_threshold).unwrap();
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            writeln!(out, "  {}: ${:.2}", item.name, item.price).unwrap();
        }
    }

    writeln!(out, "\n--- Items above ${:.2} (List) ---", price_threshold).unwrap();
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            writeln!(out, "  {}: ${:.2}", item.name, item.price).unwrap();
        }
    }
}

/// Mimics sscanf("%d") on a buffer: skips leading whitespace, parses optional sign,
/// then digits. Returns Some(n) if at least one digit found, else None.
fn scanf_int(input: &str) -> Option<i32> {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return None;
    }
    let digits = &input[start..i];
    // Use i64 to allow overflow handling like C (sscanf would store overflow).
    // C's sscanf with %d on overflow has undefined behavior; for normal short ints we just use i32 parse with saturation.
    let val: i64 = digits.parse().ok()?;
    let val = if neg { -val } else { val };
    Some(val as i32)
}

fn main() {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    writeln!(
        out,
        "\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}"
    )
    .unwrap();
    writeln!(
        out,
        "\u{2551}   GENERIC FOR_EACH MACRO DEMO         \u{2551}"
    )
    .unwrap();
    writeln!(
        out,
        "\u{2551}   Demonstrating Generic Containers    \u{2551}"
    )
    .unwrap();
    writeln!(
        out,
        "\u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}"
    )
    .unwrap();

    // The C version uses fgets(input, 256, stdin). It reads up to 255 chars or until newline (inclusive).
    // We mimic with read_line. For inputs longer than 256 bytes the C version would split, but typical menu input is short.
    loop {
        print_menu(&mut out);

        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        let choice = match scanf_int(&input) {
            Some(n) => n,
            None => {
                writeln!(out, "Invalid input").unwrap();
                continue;
            }
        };

        match choice {
            1 => demo_integer_containers(&mut out),
            2 => demo_double_containers(&mut out),
            3 => demo_inventory_array(&mut out),
            4 => demo_order_list(&mut out),
            5 => demo_mixed_operations(&mut out),
            6 => {
                writeln!(out, "\n=== Running All Demos ===").unwrap();
                demo_integer_containers(&mut out);
                demo_double_containers(&mut out);
                demo_inventory_array(&mut out);
                demo_order_list(&mut out);
                demo_mixed_operations(&mut out);
                writeln!(out, "\n========================================").unwrap();
                writeln!(out, "  All demos completed successfully!").unwrap();
                writeln!(out, "========================================").unwrap();
            }
            7 => {
                writeln!(out, "\nGoodbye!").unwrap();
                return;
            }
            _ => {
                writeln!(out, "Invalid choice").unwrap();
            }
        }
    }
}
