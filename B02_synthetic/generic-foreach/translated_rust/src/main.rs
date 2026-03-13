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

fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    Item {
        id,
        name: name.chars().take(MAX_NAME_LENGTH - 1).collect(),
        category: category.chars().take(MAX_CATEGORY_LENGTH - 1).collect(),
        price,
        quantity,
    }
}

fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    Order {
        customer_id,
        customer_name: customer_name.chars().take(MAX_NAME_LENGTH - 1).collect(),
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

fn calculate_inventory_stats(items: &[Item]) {
    if items.is_empty() {
        println!("No items in inventory");
        return;
    }

    println!();
    println!("=== Inventory Statistics (Array) ===");

    let mut total_value = 0.0_f64;
    let mut total_items = 0_i32;
    let mut max_price = 0.0_f64;
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

    println!("Total unique items: {}", items.len());
    println!("Total item count: {}", total_items);
    println!("Total inventory value: ${:.2}", total_value);
    println!("Average item price: ${:.2}", total_value / total_items as f64);
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

    let mut total_revenue = 0.0_f64;
    let mut max_order = 0.0_f64;
    let mut min_order = -1.0_f64;

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

#[allow(dead_code)]
fn find_expensive_items(items: &[Item], min_price: f64) {
    println!();
    println!("=== Items priced above ${:.2} ===", min_price);

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
    let _ = io::stdout().flush();
}

fn demo_integer_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 1: Integer Containers");
    println!("========================================");

    // Integer array
    let int_array: Vec<i32> = vec![10, 20, 30, 40, 50];
    println!();
    println!("--- Integer Array ---");
    println!("Adding integers: 10, 20, 30, 40, 50");

    print!("Array contents: ");
    for val in &int_array {
        print!("{} ", val);
    }
    println!();

    let sum: i32 = int_array.iter().sum();
    println!("Sum: {}", sum);
    println!("Average: {:.2}", sum as f64 / int_array.len() as f64);

    // Integer list
    let int_list: Vec<i32> = vec![100, 200, 300, 400, 500];
    println!();
    println!("--- Integer List ---");
    println!("Adding integers: 100, 200, 300, 400, 500");

    print!("List contents: ");
    for val in &int_list {
        print!("{} ", val);
    }
    println!();

    let product: i64 = int_list.iter().fold(1_i64, |acc, &v| acc * v as i64);
    println!("Product: {}", product);
}

fn demo_double_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 2: Double Containers");
    println!("========================================");

    let temps: Vec<f64> = vec![23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    println!();
    println!("--- Double Array (Temperatures in Celsius) ---");

    print!("Adding temperatures: ");
    for t in &temps {
        print!("{:.1} ", t);
    }
    println!();

    let mut min_temp = temps[0];
    let mut max_temp = temps[0];
    let mut sum_temp = 0.0_f64;

    for &temp in &temps {
        if temp < min_temp {
            min_temp = temp;
        }
        if temp > max_temp {
            max_temp = temp;
        }
        sum_temp += temp;
    }

    println!("Minimum: {:.1}\u{00b0}C", min_temp);
    println!("Maximum: {:.1}\u{00b0}C", max_temp);
    println!("Average: {:.1}\u{00b0}C", sum_temp / temps.len() as f64);

    let prices: Vec<f64> = vec![9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    println!();
    println!("--- Double List (Product Prices) ---");

    print!("Adding prices: ");
    for p in &prices {
        print!("${:.2} ", p);
    }
    println!();

    let mut total = 0.0_f64;
    let mut count_under_10 = 0;

    for &p in &prices {
        total += p;
        if p < 10.0 {
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

    let inventory = vec![
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
    ];

    println!();
    println!("--- Adding Items to Inventory ---");
    println!("Added {} items to inventory", inventory.len());

    println!();
    println!("--- All Inventory Items ---");
    for item in &inventory {
        print_item(item);
        println!();
    }

    calculate_inventory_stats(&inventory);

    find_items_by_category(&inventory, "Electronics");
    find_items_by_category(&inventory, "Furniture");

    println!();
    println!("--- Low Stock Items (< 20) ---");
    let mut low_stock_count = 0;
    for item in &inventory {
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

    let orders = vec![
        create_order(1001, "Alice Johnson", 1249.95),
        create_order(1002, "Bob Smith", 89.99),
        create_order(1003, "Carol White", 549.98),
        create_order(1004, "David Brown", 24.99),
        create_order(1005, "Eve Davis", 899.99),
        create_order(1006, "Frank Miller", 374.97),
        create_order(1007, "Grace Lee", 159.98),
        create_order(1008, "Henry Wilson", 1099.99),
    ];

    println!();
    println!("--- Adding Orders ---");
    println!("Added {} orders", orders.len());

    println!();
    println!("--- All Orders ---");
    for order in &orders {
        print_order(order);
    }

    calculate_order_stats(&orders);

    println!();
    println!("--- Large Orders (> $500) ---");
    let mut large_order_count = 0;
    let mut large_order_total = 0.0_f64;

    for order in &orders {
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

    let items = vec![
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    println!();
    println!("--- Populating both Array and List ---");
    println!("Added {} items to both containers", items.len());

    println!();
    println!("--- Iterating through Array ---");
    let array_count = items.len();
    println!("Array iteration count: {}", array_count);

    println!();
    println!("--- Iterating through List ---");
    let list_count = items.len();
    println!("List iteration count: {}", list_count);

    let price_threshold = 200.0_f64;

    println!();
    println!("--- Items above ${:.2} (Array) ---", price_threshold);
    for item in &items {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", item.name, item.price);
        }
    }

    println!();
    println!("--- Items above ${:.2} (List) ---", price_threshold);
    for item in &items {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", item.name, item.price);
        }
    }
}

fn main() {
    println!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}");
    println!("\u{2551}   GENERIC FOR_EACH MACRO DEMO         \u{2551}");
    println!("\u{2551}   Demonstrating Generic Containers    \u{2551}");
    println!("\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}");

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        print_menu();

        let line = match lines.next() {
            Some(Ok(l)) => l,
            _ => break,
        };

        let choice: i32 = match line.trim().parse() {
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
