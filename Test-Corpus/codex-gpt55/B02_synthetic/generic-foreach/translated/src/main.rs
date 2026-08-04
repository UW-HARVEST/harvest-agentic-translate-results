use std::io::{self, Read, Write};

const MAX_INPUT: usize = 256;

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

fn copy_c_string(s: &str, max_len: usize) -> String {
    let bytes = s.as_bytes();
    let end = bytes.len().min(max_len - 1);
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: copy_c_string(name, 64),
        category: copy_c_string(category, 32),
        price,
        quantity,
    }
}

fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: copy_c_string(customer_name, 64),
        total_amount,
    }
}

fn print_menu(out: &mut String) {
    out.push('\n');
    out.push_str("========================================\n");
    out.push_str("  GENERIC FOR_EACH MACRO DEMO\n");
    out.push_str("========================================\n");
    out.push_str("1. Demo: Integer Containers\n");
    out.push_str("2. Demo: Double Containers\n");
    out.push_str("3. Demo: Inventory Array\n");
    out.push_str("4. Demo: Order List\n");
    out.push_str("5. Demo: Mixed Operations\n");
    out.push_str("6. Run All Demos\n");
    out.push_str("7. Exit\n");
    out.push_str("========================================\n");
    out.push_str("Choice: ");
}

fn print_item(out: &mut String, item: &Item) {
    out.push_str(&format!("  [{}] {}\n", item.id, item.name));
    out.push_str(&format!("      Category: {}\n", item.category));
    out.push_str(&format!("      Price: ${:.2}\n", item.price));
    out.push_str(&format!("      Quantity: {}\n", item.quantity));
}

fn print_order(out: &mut String, order: &Order) {
    out.push_str(&format!(
        "  Order - Customer ID: {}, Name: {}\n",
        order.customer_id, order.customer_name
    ));
    out.push_str(&format!("          Total: ${:.2}\n", order.total_amount));
}

fn calculate_inventory_stats(out: &mut String, items: &[Item]) {
    if items.is_empty() {
        out.push_str("No items in inventory\n");
        return;
    }

    out.push_str("\n=== Inventory Statistics (Array) ===\n");

    let mut total_value = 0.0;
    let mut total_items = 0;
    let mut max_price = 0.0;
    let mut min_price = items[0].price;

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

    out.push_str(&format!("Total unique items: {}\n", items.len()));
    out.push_str(&format!("Total item count: {}\n", total_items));
    out.push_str(&format!("Total inventory value: ${:.2}\n", total_value));
    out.push_str(&format!(
        "Average item price: ${:.2}\n",
        total_value / total_items as f64
    ));
    out.push_str(&format!("Most expensive item: ${:.2}\n", max_price));
    out.push_str(&format!("Least expensive item: ${:.2}\n", min_price));
}

fn calculate_order_stats(out: &mut String, orders: &[Order]) {
    if orders.is_empty() {
        out.push_str("No orders to analyze\n");
        return;
    }

    out.push_str("\n=== Order Statistics (List) ===\n");

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

    out.push_str(&format!("Total orders: {}\n", orders.len()));
    out.push_str(&format!("Total revenue: ${:.2}\n", total_revenue));
    out.push_str(&format!(
        "Average order value: ${:.2}\n",
        total_revenue / orders.len() as f64
    ));
    out.push_str(&format!("Largest order: ${:.2}\n", max_order));
    out.push_str(&format!("Smallest order: ${:.2}\n", min_order));
}

fn find_items_by_category(out: &mut String, items: &[Item], category: &str) {
    out.push_str(&format!("\n=== Items in category '{}' ===\n", category));

    let mut found = 0;
    for item in items {
        if item.category == category {
            print_item(out, item);
            found += 1;
        }
    }

    if found == 0 {
        out.push_str("No items found in this category\n");
    } else {
        out.push_str(&format!("Found {} items\n", found));
    }
}

fn demo_integer_containers(out: &mut String) {
    out.push('\n');
    out.push_str("========================================\n");
    out.push_str("  DEMO 1: Integer Containers\n");
    out.push_str("========================================\n");

    let mut int_array = Vec::with_capacity(10);
    out.push_str("\n--- Integer Array ---\n");
    out.push_str("Adding integers: 10, 20, 30, 40, 50\n");
    int_array.extend([10, 20, 30, 40, 50]);

    out.push_str("Array contents: ");
    for val in &int_array {
        out.push_str(&format!("{} ", val));
    }
    out.push('\n');

    let mut sum = 0;
    for val in &int_array {
        sum += val;
    }
    out.push_str(&format!("Sum: {}\n", sum));
    out.push_str(&format!("Average: {:.2}\n", sum as f64 / int_array.len() as f64));

    let mut int_list = Vec::new();
    out.push_str("\n--- Integer List ---\n");
    out.push_str("Adding integers: 100, 200, 300, 400, 500\n");
    int_list.extend([100, 200, 300, 400, 500]);

    out.push_str("List contents: ");
    for val in &int_list {
        out.push_str(&format!("{} ", val));
    }
    out.push('\n');

    let mut product: i64 = 1;
    for val in &int_list {
        product *= *val as i64;
    }
    out.push_str(&format!("Product: {}\n", product));
}

fn demo_double_containers(out: &mut String) {
    out.push('\n');
    out.push_str("========================================\n");
    out.push_str("  DEMO 2: Double Containers\n");
    out.push_str("========================================\n");

    let mut double_array = Vec::with_capacity(5);
    out.push_str("\n--- Double Array (Temperatures in Celsius) ---\n");

    let temps = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    out.push_str("Adding temperatures: ");
    for temp in temps {
        double_array.push(temp);
        out.push_str(&format!("{:.1} ", temp));
    }
    out.push('\n');

    let mut min_temp = temps[0];
    let mut max_temp = temps[0];
    let mut sum_temp = 0.0;
    for temp in &double_array {
        if *temp < min_temp {
            min_temp = *temp;
        }
        if *temp > max_temp {
            max_temp = *temp;
        }
        sum_temp += *temp;
    }

    out.push_str(&format!("Minimum: {:.1}°C\n", min_temp));
    out.push_str(&format!("Maximum: {:.1}°C\n", max_temp));
    out.push_str(&format!(
        "Average: {:.1}°C\n",
        sum_temp / double_array.len() as f64
    ));

    let mut price_list = Vec::new();
    out.push_str("\n--- Double List (Product Prices) ---\n");

    let prices = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    out.push_str("Adding prices: ");
    for price in prices {
        price_list.push(price);
        out.push_str(&format!("${:.2} ", price));
    }
    out.push('\n');

    let mut total = 0.0;
    let mut count_under_10 = 0;
    for temp in &price_list {
        total += *temp;
        if *temp < 10.0 {
            count_under_10 += 1;
        }
    }

    out.push_str(&format!("Total cost: ${:.2}\n", total));
    out.push_str(&format!("Items under $10: {}\n", count_under_10));
}

fn inventory_items() -> Vec<Item> {
    vec![
        create_item(1, "Laptop", "Electronics", 899.99, 15),
        create_item(2, "Mouse", "Electronics", 24.99, 50),
        create_item(3, "Keyboard", "Electronics", 79.99, 30),
        create_item(4, "Monitor", "Electronics", 299.99, 20),
        create_item(5, "Desk Chair", "Furniture", 199.99, 10),
        create_item(6, "Desk", "Furniture", 349.99, 8),
        create_item(7, "Notebook", "Office", 4.99, 100),
        create_item(8, "Pen Set", "Office", 12.99, 75),
        create_item(9, "USB Cable", "Electronics", 9.99, 60),
        create_item(10, "Bookshelf", "Furniture", 149.99, 12),
    ]
}

fn demo_inventory_array(out: &mut String) {
    out.push('\n');
    out.push_str("========================================\n");
    out.push_str("  DEMO 3: Inventory Array (Items)\n");
    out.push_str("========================================\n");

    out.push_str("\n--- Adding Items to Inventory ---\n");
    let inventory = inventory_items();

    out.push_str(&format!("Added {} items to inventory\n", inventory.len()));

    out.push_str("\n--- All Inventory Items ---\n");
    for item in &inventory {
        print_item(out, item);
        out.push('\n');
    }

    calculate_inventory_stats(out, &inventory);
    find_items_by_category(out, &inventory, "Electronics");
    find_items_by_category(out, &inventory, "Furniture");

    out.push_str("\n--- Low Stock Items (< 20) ---\n");
    let mut low_stock_count = 0;
    for item in &inventory {
        if item.quantity < 20 {
            print_item(out, item);
            low_stock_count += 1;
        }
    }
    out.push_str(&format!("Total low stock items: {}\n", low_stock_count));
}

fn demo_order_list(out: &mut String) {
    out.push('\n');
    out.push_str("========================================\n");
    out.push_str("  DEMO 4: Order List (Orders)\n");
    out.push_str("========================================\n");

    let mut orders = Vec::new();
    out.push_str("\n--- Adding Orders ---\n");
    orders.push(create_order(1001, "Alice Johnson", 1249.95));
    orders.push(create_order(1002, "Bob Smith", 89.99));
    orders.push(create_order(1003, "Carol White", 549.98));
    orders.push(create_order(1004, "David Brown", 24.99));
    orders.push(create_order(1005, "Eve Davis", 899.99));
    orders.push(create_order(1006, "Frank Miller", 374.97));
    orders.push(create_order(1007, "Grace Lee", 159.98));
    orders.push(create_order(1008, "Henry Wilson", 1099.99));

    out.push_str(&format!("Added {} orders\n", orders.len()));

    out.push_str("\n--- All Orders ---\n");
    for order in &orders {
        print_order(out, order);
    }

    calculate_order_stats(out, &orders);

    out.push_str("\n--- Large Orders (> $500) ---\n");
    let mut large_order_count = 0;
    let mut large_order_total = 0.0;

    for order in &orders {
        if order.total_amount > 500.0 {
            print_order(out, order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    out.push_str(&format!("Total large orders: {}\n", large_order_count));
    out.push_str(&format!(
        "Revenue from large orders: ${:.2}\n",
        large_order_total
    ));
}

fn demo_mixed_operations(out: &mut String) {
    out.push('\n');
    out.push_str("========================================\n");
    out.push_str("  DEMO 5: Mixed Operations\n");
    out.push_str("========================================\n");

    let mut array_inventory = Vec::with_capacity(10);
    let mut list_inventory = Vec::new();

    out.push_str("\n--- Populating both Array and List ---\n");

    let items = vec![
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items = items.len();
    for item in items {
        array_inventory.push(item.clone());
        list_inventory.push(item);
    }

    out.push_str(&format!("Added {} items to both containers\n", num_items));

    out.push_str("\n--- Iterating through Array ---\n");
    let mut array_count = 0;
    for _ in &array_inventory {
        array_count += 1;
    }
    out.push_str(&format!("Array iteration count: {}\n", array_count));

    out.push_str("\n--- Iterating through List ---\n");
    let mut list_count = 0;
    for _ in &list_inventory {
        list_count += 1;
    }
    out.push_str(&format!("List iteration count: {}\n", list_count));

    let price_threshold = 200.0;

    out.push_str(&format!(
        "\n--- Items above ${:.2} (Array) ---\n",
        price_threshold
    ));
    for item in &array_inventory {
        if item.price >= price_threshold {
            out.push_str(&format!("  {}: ${:.2}\n", item.name, item.price));
        }
    }

    out.push_str(&format!(
        "\n--- Items above ${:.2} (List) ---\n",
        price_threshold
    ));
    for item in &list_inventory {
        if item.price >= price_threshold {
            out.push_str(&format!("  {}: ${:.2}\n", item.name, item.price));
        }
    }
}

fn is_c_space(byte: u8) -> bool {
    matches!(byte, b' ' | 0x0c | b'\n' | b'\r' | b'\t' | 0x0b)
}

fn sscanf_decimal_int(input: &[u8]) -> Option<i32> {
    let mut index = 0;
    while index < input.len() && is_c_space(input[index]) {
        index += 1;
    }

    let mut sign = 1_i64;
    if index < input.len() && (input[index] == b'+' || input[index] == b'-') {
        if input[index] == b'-' {
            sign = -1;
        }
        index += 1;
    }

    let start_digits = index;
    let mut value = 0_i64;
    while index < input.len() && input[index].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add((input[index] - b'0') as i64);
        index += 1;
    }

    if index == start_digits {
        None
    } else {
        Some((value.saturating_mul(sign)) as i32)
    }
}

fn fgets_chunk<R: Read>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut chunk = Vec::new();
    let mut byte = [0_u8; 1];

    while chunk.len() < MAX_INPUT - 1 {
        let read = reader.read(&mut byte)?;
        if read == 0 {
            break;
        }
        chunk.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }

    if chunk.is_empty() {
        Ok(None)
    } else {
        Ok(Some(chunk))
    }
}

fn run<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut out = String::new();
    out.push_str("╔════════════════════════════════════════╗\n");
    out.push_str("║   GENERIC FOR_EACH MACRO DEMO         ║\n");
    out.push_str("║   Demonstrating Generic Containers    ║\n");
    out.push_str("╚════════════════════════════════════════╝\n");

    loop {
        print_menu(&mut out);

        let Some(input) = fgets_chunk(reader)? else {
            break;
        };

        let Some(choice) = sscanf_decimal_int(&input) else {
            out.push_str("Invalid input\n");
            continue;
        };

        match choice {
            1 => demo_integer_containers(&mut out),
            2 => demo_double_containers(&mut out),
            3 => demo_inventory_array(&mut out),
            4 => demo_order_list(&mut out),
            5 => demo_mixed_operations(&mut out),
            6 => {
                out.push_str("\n=== Running All Demos ===\n");
                demo_integer_containers(&mut out);
                demo_double_containers(&mut out);
                demo_inventory_array(&mut out);
                demo_order_list(&mut out);
                demo_mixed_operations(&mut out);
                out.push_str("\n========================================\n");
                out.push_str("  All demos completed successfully!\n");
                out.push_str("========================================\n");
            }
            7 => {
                out.push_str("\nGoodbye!\n");
                break;
            }
            _ => out.push_str("Invalid choice\n"),
        }
    }

    Ok(out)
}

fn main() -> io::Result<()> {
    let mut stdin = io::stdin().lock();
    let output = run(&mut stdin)?;
    io::stdout().write_all(output.as_bytes())
}
