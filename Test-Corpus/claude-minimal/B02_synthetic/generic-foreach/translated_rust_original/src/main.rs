// main.rs - Translation of main.c
// Demonstration of generic containers (Vec and LinkedList) holding
// integers, doubles, and inventory/order records.

mod inventory;

use std::io::{self, BufRead, Write};

use inventory::{
    buf_to_str, calculate_inventory_stats, calculate_order_stats, create_item, create_order,
    find_items_by_category, print_item, print_order, Item, LinkedList, Order,
};

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

    // Create integer array (Vec<i32>)
    let mut int_array: Vec<i32> = Vec::with_capacity(10);
    println!("\n--- Integer Array ---");
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

    // Calculate sum
    let mut sum: i32 = 0;
    for &val in int_array.iter() {
        sum += val;
    }
    println!("Sum: {}", sum);
    println!("Average: {:.2}", sum as f64 / int_array.len() as f64);

    // Create integer list
    let mut int_list: LinkedList<i32> = LinkedList::new();
    println!("\n--- Integer List ---");
    println!("Adding integers: 100, 200, 300, 400, 500");
    int_list.append(100);
    int_list.append(200);
    int_list.append(300);
    int_list.append(400);
    int_list.append(500);

    print!("List contents: ");
    for &val in int_list.iter() {
        print!("{} ", val);
    }
    println!();

    // Calculate product
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

    // Create double array
    let mut double_array: Vec<f64> = Vec::with_capacity(5);
    println!("\n--- Double Array (Temperatures in Celsius) ---");

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    print!("Adding temperatures: ");
    for &t in temps.iter() {
        double_array.push(t);
        print!("{:.1} ", t);
    }
    println!();

    // Find min, max, average
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

    // Create double list
    let mut price_list: LinkedList<f64> = LinkedList::new();
    println!("\n--- Double List (Product Prices) ---");

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    print!("Adding prices: ");
    for &p in prices.iter() {
        price_list.append(p);
        print!("${:.2} ", p);
    }
    println!();

    // Calculate total and find items under $10
    let mut total = 0.0_f64;
    let mut count_under_10 = 0;

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

    // Create inventory array
    let mut inventory: Vec<Item> = Vec::with_capacity(20);

    // Add items
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

    // Display all items
    println!("\n--- All Inventory Items ---");
    for item in inventory.iter() {
        print_item(item);
        println!();
    }

    // Calculate statistics
    calculate_inventory_stats(&inventory);

    // Find items by category
    find_items_by_category(&inventory, "Electronics");
    find_items_by_category(&inventory, "Furniture");

    // Find low stock items
    println!("\n--- Low Stock Items (< 20) ---");
    let mut low_stock_count = 0;
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

    // Create order list
    let mut orders: LinkedList<Order> = LinkedList::new();

    // Add orders
    println!("\n--- Adding Orders ---");
    orders.append(create_order(1001, "Alice Johnson", 1249.95));
    orders.append(create_order(1002, "Bob Smith", 89.99));
    orders.append(create_order(1003, "Carol White", 549.98));
    orders.append(create_order(1004, "David Brown", 24.99));
    orders.append(create_order(1005, "Eve Davis", 899.99));
    orders.append(create_order(1006, "Frank Miller", 374.97));
    orders.append(create_order(1007, "Grace Lee", 159.98));
    orders.append(create_order(1008, "Henry Wilson", 1099.99));

    println!("Added {} orders", orders.len());

    // Display all orders
    println!("\n--- All Orders ---");
    for order in orders.iter() {
        print_order(order);
    }

    // Calculate statistics
    calculate_order_stats(&orders);

    // Find large orders
    println!("\n--- Large Orders (> $500) ---");
    let mut large_order_count = 0;
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

    // Create both array and list with items
    let mut array_inventory: Vec<Item> = Vec::with_capacity(10);
    let mut list_inventory: LinkedList<Item> = LinkedList::new();

    println!("\n--- Populating both Array and List ---");

    // Add same items to both
    let items: [Item; 5] = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items = items.len();

    for item in items.iter() {
        array_inventory.push(*item);
        list_inventory.append(*item);
    }

    println!("Added {} items to both containers", num_items);

    // Iterate array
    println!("\n--- Iterating through Array ---");
    let mut array_count = 0;
    for _ in array_inventory.iter() {
        array_count += 1;
    }
    println!("Array iteration count: {}", array_count);

    println!("\n--- Iterating through List ---");
    let mut list_count = 0;
    for _ in list_inventory.iter() {
        list_count += 1;
    }
    println!("List iteration count: {}", list_count);

    // Find items above certain price
    let price_threshold = 200.0_f64;

    println!("\n--- Items above ${:.2} (Array) ---", price_threshold);
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", buf_to_str(&item.name), item.price);
        }
    }

    println!("\n--- Items above ${:.2} (List) ---", price_threshold);
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", buf_to_str(&item.name), item.price);
        }
    }
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║   GENERIC FOR_EACH MACRO DEMO         ║");
    println!("║   Demonstrating Generic Containers    ║");
    println!("╚════════════════════════════════════════╝");

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        print_menu();

        let line = match lines.next() {
            Some(Ok(l)) => l,
            _ => break,
        };

        let choice: i32 = match line.trim().parse() {
            Ok(n) => n,
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
                return;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
}
