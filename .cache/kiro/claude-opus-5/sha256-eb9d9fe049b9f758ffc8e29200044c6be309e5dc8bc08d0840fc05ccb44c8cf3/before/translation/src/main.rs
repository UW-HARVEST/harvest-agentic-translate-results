// main.rs
//
// Translation of c_src/src/main.c.

#[macro_use]
mod stdio;

mod containers;
mod inventory;

use containers::{Array, List};
use inventory::{
    calculate_inventory_stats, calculate_order_stats, create_item, create_order,
    find_items_by_category, print_item, print_order, Item,
};
use stdio::{fgets, out_flush, sscanf_int};

fn print_menu() {
    printf!("\n");
    printf!("========================================\n");
    printf!("  GENERIC FOR_EACH MACRO DEMO\n");
    printf!("========================================\n");
    printf!("1. Demo: Integer Containers\n");
    printf!("2. Demo: Double Containers\n");
    printf!("3. Demo: Inventory Array\n");
    printf!("4. Demo: Order List\n");
    printf!("5. Demo: Mixed Operations\n");
    printf!("6. Run All Demos\n");
    printf!("7. Exit\n");
    printf!("========================================\n");
    printf!("Choice: ");
}

fn demo_integer_containers() {
    printf!("\n");
    printf!("========================================\n");
    printf!("  DEMO 1: Integer Containers\n");
    printf!("========================================\n");

    // Create integer array
    let mut int_array: Array<i32> = Array::create(10);
    printf!("\n--- Integer Array ---\n");
    printf!("Adding integers: 10, 20, 30, 40, 50\n");
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);

    printf!("Array contents: ");
    for val in int_array.iter() {
        printf!("{} ", val);
    }
    printf!("\n");

    // Calculate sum using ARRAY_FOREACH
    let mut sum: i32 = 0;
    for val in int_array.iter() {
        sum = sum.wrapping_add(*val);
    }
    printf!("Sum: {}\n", sum);
    printf!("Average: {:.2}\n", sum as f64 / int_array.size() as f64);

    // Create integer list
    let mut int_list: List<i32> = List::create();
    printf!("\n--- Integer List ---\n");
    printf!("Adding integers: 100, 200, 300, 400, 500\n");
    int_list.append(100);
    int_list.append(200);
    int_list.append(300);
    int_list.append(400);
    int_list.append(500);

    printf!("List contents: ");
    for val in int_list.iter() {
        printf!("{} ", val);
    }
    printf!("\n");

    // Calculate product using LIST_FOREACH
    let mut product: i64 = 1;
    for val in int_list.iter() {
        product = product.wrapping_mul(*val as i64);
    }
    printf!("Product: {}\n", product);
}

fn demo_double_containers() {
    printf!("\n");
    printf!("========================================\n");
    printf!("  DEMO 2: Double Containers\n");
    printf!("========================================\n");

    // Create double array
    let mut double_array: Array<f64> = Array::create(5);
    printf!("\n--- Double Array (Temperatures in Celsius) ---\n");

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    let num_temps = temps.len();

    printf!("Adding temperatures: ");
    for i in 0..num_temps {
        double_array.push(temps[i]);
        printf!("{:.1} ", temps[i]);
    }
    printf!("\n");

    // Find min, max, average using ARRAY_FOREACH
    let mut min_temp = temps[0];
    let mut max_temp = temps[0];
    let mut sum_temp: f64 = 0.0;

    for temp in double_array.iter() {
        if *temp < min_temp {
            min_temp = *temp;
        }
        if *temp > max_temp {
            max_temp = *temp;
        }
        sum_temp += *temp;
    }

    printf!("Minimum: {:.1}\u{b0}C\n", min_temp);
    printf!("Maximum: {:.1}\u{b0}C\n", max_temp);
    printf!(
        "Average: {:.1}\u{b0}C\n",
        sum_temp / double_array.size() as f64
    );

    // Create double list
    let mut price_list: List<f64> = List::create();
    printf!("\n--- Double List (Product Prices) ---\n");

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    let num_prices = prices.len();

    printf!("Adding prices: ");
    for i in 0..num_prices {
        price_list.append(prices[i]);
        printf!("${:.2} ", prices[i]);
    }
    printf!("\n");

    // Calculate total and find items under $10 using LIST_FOREACH
    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;

    for temp in price_list.iter() {
        total += *temp;
        if *temp < 10.0 {
            count_under_10 += 1;
        }
    }

    printf!("Total cost: ${:.2}\n", total);
    printf!("Items under $10: {}\n", count_under_10);
}

fn demo_inventory_array() {
    printf!("\n");
    printf!("========================================\n");
    printf!("  DEMO 3: Inventory Array (Items)\n");
    printf!("========================================\n");

    // Create inventory array
    let mut inventory: Array<Item> = Array::create(20);

    // Add items
    printf!("\n--- Adding Items to Inventory ---\n");
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

    printf!("Added {} items to inventory\n", inventory.size());

    // Display all items using ARRAY_FOREACH
    printf!("\n--- All Inventory Items ---\n");
    for item in inventory.iter() {
        print_item(item);
        printf!("\n");
    }

    // Calculate statistics
    calculate_inventory_stats(&inventory);

    // Find items by category
    find_items_by_category(&inventory, "Electronics");
    find_items_by_category(&inventory, "Furniture");

    // Find low stock items using ARRAY_FOREACH
    printf!("\n--- Low Stock Items (< 20) ---\n");
    let mut low_stock_count: i32 = 0;
    for item in inventory.iter() {
        if item.quantity < 20 {
            print_item(item);
            low_stock_count += 1;
        }
    }
    printf!("Total low stock items: {}\n", low_stock_count);
}

fn demo_order_list() {
    printf!("\n");
    printf!("========================================\n");
    printf!("  DEMO 4: Order List (Orders)\n");
    printf!("========================================\n");

    // Create order list
    let mut orders = List::create();

    // Add orders
    printf!("\n--- Adding Orders ---\n");
    orders.append(create_order(1001, "Alice Johnson", 1249.95));
    orders.append(create_order(1002, "Bob Smith", 89.99));
    orders.append(create_order(1003, "Carol White", 549.98));
    orders.append(create_order(1004, "David Brown", 24.99));
    orders.append(create_order(1005, "Eve Davis", 899.99));
    orders.append(create_order(1006, "Frank Miller", 374.97));
    orders.append(create_order(1007, "Grace Lee", 159.98));
    orders.append(create_order(1008, "Henry Wilson", 1099.99));

    printf!("Added {} orders\n", orders.size());

    // Display all orders using LIST_FOREACH
    printf!("\n--- All Orders ---\n");
    for order in orders.iter() {
        print_order(order);
    }

    // Calculate statistics
    calculate_order_stats(&orders);

    // Find large orders using LIST_FOREACH
    printf!("\n--- Large Orders (> $500) ---\n");
    let mut large_order_count: i32 = 0;
    let mut large_order_total: f64 = 0.0;

    for order in orders.iter() {
        if order.total_amount > 500.0 {
            print_order(order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    printf!("Total large orders: {}\n", large_order_count);
    printf!("Revenue from large orders: ${:.2}\n", large_order_total);
}

fn demo_mixed_operations() {
    printf!("\n");
    printf!("========================================\n");
    printf!("  DEMO 5: Mixed Operations\n");
    printf!("========================================\n");

    // Create both array and list with items
    let mut array_inventory: Array<Item> = Array::create(10);
    let mut list_inventory: List<Item> = List::create();

    printf!("\n--- Populating both Array and List ---\n");

    // Add same items to both
    let items: [Item; 5] = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items: i32 = items.len() as i32;

    for i in 0..items.len() {
        array_inventory.push(items[i]);
        list_inventory.append(items[i]);
    }

    printf!("Added {} items to both containers\n", num_items);

    // Compare iteration performance (conceptually)
    printf!("\n--- Iterating through Array ---\n");
    let mut array_count: i32 = 0;
    for _item in array_inventory.iter() {
        array_count += 1;
    }
    printf!("Array iteration count: {}\n", array_count);

    printf!("\n--- Iterating through List ---\n");
    let mut list_count: i32 = 0;
    for _item in list_inventory.iter() {
        list_count += 1;
    }
    printf!("List iteration count: {}\n", list_count);

    // Find items above certain price in both
    let price_threshold: f64 = 200.0;

    printf!("\n--- Items above ${:.2} (Array) ---\n", price_threshold);
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            printf!("  ");
            stdio::out_raw(inventory::c_str(&item.name));
            printf!(": ${:.2}\n", item.price);
        }
    }

    printf!("\n--- Items above ${:.2} (List) ---\n", price_threshold);
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            printf!("  ");
            stdio::out_raw(inventory::c_str(&item.name));
            printf!(": ${:.2}\n", item.price);
        }
    }
}

fn main() {
    printf!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}\n");
    printf!("\u{2551}   GENERIC FOR_EACH MACRO DEMO         \u{2551}\n");
    printf!("\u{2551}   Demonstrating Generic Containers    \u{2551}\n");
    printf!("\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}\n");

    // char input[256];
    const INPUT_SIZE: usize = 256;

    loop {
        print_menu();

        let input = match fgets(INPUT_SIZE) {
            Some(line) => line,
            None => break,
        };

        let (matched, choice) = sscanf_int(&input);
        if matched != 1 {
            printf!("Invalid input\n");
            continue;
        }

        match choice {
            1 => demo_integer_containers(),
            2 => demo_double_containers(),
            3 => demo_inventory_array(),
            4 => demo_order_list(),
            5 => demo_mixed_operations(),
            6 => {
                printf!("\n=== Running All Demos ===\n");
                demo_integer_containers();
                demo_double_containers();
                demo_inventory_array();
                demo_order_list();
                demo_mixed_operations();
                printf!("\n========================================\n");
                printf!("  All demos completed successfully!\n");
                printf!("========================================\n");
            }
            7 => {
                printf!("\nGoodbye!\n");
                out_flush();
                std::process::exit(0);
            }
            _ => {
                printf!("Invalid choice\n");
            }
        }
    }

    out_flush();
}
