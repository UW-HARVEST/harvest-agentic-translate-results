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
    print!("  [{}] {}\n", item.id, item.name);
    print!("      Category: {}\n", item.category);
    print!("      Price: ${:.2}\n", item.price);
    print!("      Quantity: {}\n", item.quantity);
}

fn print_order(order: &Order) {
    print!(
        "  Order - Customer ID: {}, Name: {}\n",
        order.customer_id, order.customer_name
    );
    print!("          Total: ${:.2}\n", order.total_amount);
}

fn calculate_inventory_stats(items: &[Item]) {
    if items.is_empty() {
        print!("No items in inventory\n");
        return;
    }

    print!("\n=== Inventory Statistics (Array) ===\n");

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

    print!("Total unique items: {}\n", items.len());
    print!("Total item count: {}\n", total_items);
    print!("Total inventory value: ${:.2}\n", total_value);
    print!("Average item price: ${:.2}\n", total_value / total_items as f64);
    print!("Most expensive item: ${:.2}\n", max_price);
    print!("Least expensive item: ${:.2}\n", min_price);
}

fn calculate_order_stats(orders: &[Order]) {
    if orders.is_empty() {
        print!("No orders to analyze\n");
        return;
    }

    print!("\n=== Order Statistics (List) ===\n");

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

    print!("Total orders: {}\n", orders.len());
    print!("Total revenue: ${:.2}\n", total_revenue);
    print!(
        "Average order value: ${:.2}\n",
        total_revenue / orders.len() as f64
    );
    print!("Largest order: ${:.2}\n", max_order);
    print!("Smallest order: ${:.2}\n", min_order);
}

fn find_items_by_category(items: &[Item], category: &str) {
    print!("\n=== Items in category '{}' ===\n", category);

    let mut found = 0;
    for item in items {
        if item.category == category {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        print!("No items found in this category\n");
    } else {
        print!("Found {} items\n", found);
    }
}

fn find_expensive_items(items: &[Item], min_price: f64) {
    print!("\n=== Items priced above ${:.2} ===\n", min_price);

    let mut found = 0;
    for item in items {
        if item.price >= min_price {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        print!("No items found above this price\n");
    } else {
        print!("Found {} items\n", found);
    }
}

fn print_menu() {
    print!("\n");
    print!("========================================\n");
    print!("  GENERIC FOR_EACH MACRO DEMO\n");
    print!("========================================\n");
    print!("1. Demo: Integer Containers\n");
    print!("2. Demo: Double Containers\n");
    print!("3. Demo: Inventory Array\n");
    print!("4. Demo: Order List\n");
    print!("5. Demo: Mixed Operations\n");
    print!("6. Run All Demos\n");
    print!("7. Exit\n");
    print!("========================================\n");
    print!("Choice: ");
    io::stdout().flush().unwrap();
}

fn demo_integer_containers() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 1: Integer Containers\n");
    print!("========================================\n");

    // Integer array
    let int_array: Vec<i32> = vec![10, 20, 30, 40, 50];
    print!("\n--- Integer Array ---\n");
    print!("Adding integers: 10, 20, 30, 40, 50\n");

    print!("Array contents: ");
    for val in &int_array {
        print!("{} ", val);
    }
    print!("\n");

    let sum: i32 = int_array.iter().sum();
    print!("Sum: {}\n", sum);
    print!("Average: {:.2}\n", sum as f64 / int_array.len() as f64);

    // Integer list
    let int_list: Vec<i32> = vec![100, 200, 300, 400, 500];
    print!("\n--- Integer List ---\n");
    print!("Adding integers: 100, 200, 300, 400, 500\n");

    print!("List contents: ");
    for val in &int_list {
        print!("{} ", val);
    }
    print!("\n");

    let product: i64 = int_list.iter().fold(1_i64, |acc, &v| acc * v as i64);
    print!("Product: {}\n", product);
}

fn demo_double_containers() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 2: Double Containers\n");
    print!("========================================\n");

    // Double array (temperatures)
    let temps: Vec<f64> = vec![23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    print!("\n--- Double Array (Temperatures in Celsius) ---\n");

    print!("Adding temperatures: ");
    for t in &temps {
        print!("{:.1} ", t);
    }
    print!("\n");

    let mut min_temp = temps[0];
    let mut max_temp = temps[0];
    let mut sum_temp = 0.0_f64;

    for &t in &temps {
        if t < min_temp {
            min_temp = t;
        }
        if t > max_temp {
            max_temp = t;
        }
        sum_temp += t;
    }

    print!("Minimum: {:.1}°C\n", min_temp);
    print!("Maximum: {:.1}°C\n", max_temp);
    print!("Average: {:.1}°C\n", sum_temp / temps.len() as f64);

    // Double list (prices)
    let prices: Vec<f64> = vec![9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    print!("\n--- Double List (Product Prices) ---\n");

    print!("Adding prices: ");
    for p in &prices {
        print!("${:.2} ", p);
    }
    print!("\n");

    let mut total = 0.0_f64;
    let mut count_under_10 = 0;

    for &p in &prices {
        total += p;
        if p < 10.0 {
            count_under_10 += 1;
        }
    }

    print!("Total cost: ${:.2}\n", total);
    print!("Items under $10: {}\n", count_under_10);
}

fn demo_inventory_array() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 3: Inventory Array (Items)\n");
    print!("========================================\n");

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

    print!("\n--- Adding Items to Inventory ---\n");
    print!("Added {} items to inventory\n", inventory.len());

    print!("\n--- All Inventory Items ---\n");
    for item in &inventory {
        print_item(item);
        print!("\n");
    }

    calculate_inventory_stats(&inventory);

    find_items_by_category(&inventory, "Electronics");
    find_items_by_category(&inventory, "Furniture");

    print!("\n--- Low Stock Items (< 20) ---\n");
    let mut low_stock_count = 0;
    for item in &inventory {
        if item.quantity < 20 {
            print_item(item);
            low_stock_count += 1;
        }
    }
    print!("Total low stock items: {}\n", low_stock_count);
}

fn demo_order_list() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 4: Order List (Orders)\n");
    print!("========================================\n");

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

    print!("\n--- Adding Orders ---\n");
    print!("Added {} orders\n", orders.len());

    print!("\n--- All Orders ---\n");
    for order in &orders {
        print_order(order);
    }

    calculate_order_stats(&orders);

    print!("\n--- Large Orders (> $500) ---\n");
    let mut large_order_count = 0;
    let mut large_order_total = 0.0_f64;

    for order in &orders {
        if order.total_amount > 500.0 {
            print_order(order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    print!("Total large orders: {}\n", large_order_count);
    print!("Revenue from large orders: ${:.2}\n", large_order_total);
}

fn demo_mixed_operations() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 5: Mixed Operations\n");
    print!("========================================\n");

    let items = vec![
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    // Both array and list get the same items
    let array_inventory = &items;
    let list_inventory = &items;

    print!("\n--- Populating both Array and List ---\n");
    print!("Added {} items to both containers\n", items.len());

    print!("\n--- Iterating through Array ---\n");
    let array_count = array_inventory.len();
    print!("Array iteration count: {}\n", array_count);

    print!("\n--- Iterating through List ---\n");
    let list_count = list_inventory.len();
    print!("List iteration count: {}\n", list_count);

    let price_threshold = 200.0_f64;

    print!("\n--- Items above ${:.2} (Array) ---\n", price_threshold);
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            print!("  {}: ${:.2}\n", item.name, item.price);
        }
    }

    print!("\n--- Items above ${:.2} (List) ---\n", price_threshold);
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            print!("  {}: ${:.2}\n", item.name, item.price);
        }
    }
}

fn main() {
    print!("╔════════════════════════════════════════╗\n");
    print!("║   GENERIC FOR_EACH MACRO DEMO         ║\n");
    print!("║   Demonstrating Generic Containers    ║\n");
    print!("╚════════════════════════════════════════╝\n");

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
                print!("Invalid input\n");
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
                print!("\n=== Running All Demos ===\n");
                demo_integer_containers();
                demo_double_containers();
                demo_inventory_array();
                demo_order_list();
                demo_mixed_operations();
                print!("\n========================================\n");
                print!("  All demos completed successfully!\n");
                print!("========================================\n");
            }
            7 => {
                print!("\nGoodbye!\n");
                return;
            }
            _ => {
                print!("Invalid choice\n");
            }
        }
    }
}
