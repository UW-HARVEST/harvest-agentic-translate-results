// main.rs
//
// Rust translation of main.c

#[macro_use]
mod cstdio;
mod containers;
mod inventory;

use std::io::{BufWriter, Write};

use containers::{Array, List};
use cstdio::{fgets, fmt_f, sscanf_int};
use inventory::{
    calculate_inventory_stats, calculate_order_stats, create_item, create_order,
    find_items_by_category, print_item, print_order, Item, Order,
};

fn print_menu(out: &mut dyn Write) {
    p!(out, "\n");
    p!(out, "========================================\n");
    p!(out, "  GENERIC FOR_EACH MACRO DEMO\n");
    p!(out, "========================================\n");
    p!(out, "1. Demo: Integer Containers\n");
    p!(out, "2. Demo: Double Containers\n");
    p!(out, "3. Demo: Inventory Array\n");
    p!(out, "4. Demo: Order List\n");
    p!(out, "5. Demo: Mixed Operations\n");
    p!(out, "6. Run All Demos\n");
    p!(out, "7. Exit\n");
    p!(out, "========================================\n");
    p!(out, "Choice: ");
}

fn demo_integer_containers(out: &mut dyn Write) {
    p!(out, "\n");
    p!(out, "========================================\n");
    p!(out, "  DEMO 1: Integer Containers\n");
    p!(out, "========================================\n");

    // Create integer array
    let mut int_array: Array<i32> = Array::create(10);
    p!(out, "\n--- Integer Array ---\n");
    p!(out, "Adding integers: 10, 20, 30, 40, 50\n");
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);

    p!(out, "Array contents: ");
    for val in int_array.iter() {
        p!(out, "{} ", val);
    }
    p!(out, "\n");

    // Calculate sum
    let mut sum: i32 = 0;
    for val in int_array.iter() {
        sum = sum.wrapping_add(*val);
    }
    p!(out, "Sum: {}\n", sum);
    p!(
        out,
        "Average: {}\n",
        fmt_f(f64::from(sum) / int_array.size() as f64, 2)
    );

    // Create integer list
    let mut int_list: List<i32> = List::create();
    p!(out, "\n--- Integer List ---\n");
    p!(out, "Adding integers: 100, 200, 300, 400, 500\n");
    int_list.append(100);
    int_list.append(200);
    int_list.append(300);
    int_list.append(400);
    int_list.append(500);

    p!(out, "List contents: ");
    for val in int_list.iter() {
        p!(out, "{} ", val);
    }
    p!(out, "\n");

    // Calculate product
    let mut product: i64 = 1;
    for val in int_list.iter() {
        product = product.wrapping_mul(i64::from(*val));
    }
    p!(out, "Product: {}\n", product);
}

fn demo_double_containers(out: &mut dyn Write) {
    p!(out, "\n");
    p!(out, "========================================\n");
    p!(out, "  DEMO 2: Double Containers\n");
    p!(out, "========================================\n");

    // Create double array
    let mut double_array: Array<f64> = Array::create(5);
    p!(out, "\n--- Double Array (Temperatures in Celsius) ---\n");

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    p!(out, "Adding temperatures: ");
    for &t in temps.iter() {
        double_array.push(t);
        p!(out, "{} ", fmt_f(t, 1));
    }
    p!(out, "\n");

    // Find min, max, average
    let mut min_temp: f64 = temps[0];
    let mut max_temp: f64 = temps[0];
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

    p!(out, "Minimum: {}\u{00b0}C\n", fmt_f(min_temp, 1));
    p!(out, "Maximum: {}\u{00b0}C\n", fmt_f(max_temp, 1));
    p!(
        out,
        "Average: {}\u{00b0}C\n",
        fmt_f(sum_temp / double_array.size() as f64, 1)
    );

    // Create double list
    let mut price_list: List<f64> = List::create();
    p!(out, "\n--- Double List (Product Prices) ---\n");

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    p!(out, "Adding prices: ");
    for &pr in prices.iter() {
        price_list.append(pr);
        p!(out, "${} ", fmt_f(pr, 2));
    }
    p!(out, "\n");

    // Calculate total and count items under $10
    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;

    for temp in price_list.iter() {
        total += *temp;
        if *temp < 10.0 {
            count_under_10 += 1;
        }
    }

    p!(out, "Total cost: ${}\n", fmt_f(total, 2));
    p!(out, "Items under $10: {}\n", count_under_10);
}

fn demo_inventory_array(out: &mut dyn Write) {
    p!(out, "\n");
    p!(out, "========================================\n");
    p!(out, "  DEMO 3: Inventory Array (Items)\n");
    p!(out, "========================================\n");

    // Create inventory array
    let mut inventory: Array<Item> = Array::create(20);

    // Add items
    p!(out, "\n--- Adding Items to Inventory ---\n");
    inventory.push(create_item(1, b"Laptop", b"Electronics", 899.99, 15));
    inventory.push(create_item(2, b"Mouse", b"Electronics", 24.99, 50));
    inventory.push(create_item(3, b"Keyboard", b"Electronics", 79.99, 30));
    inventory.push(create_item(4, b"Monitor", b"Electronics", 299.99, 20));
    inventory.push(create_item(5, b"Desk Chair", b"Furniture", 199.99, 10));
    inventory.push(create_item(6, b"Desk", b"Furniture", 349.99, 8));
    inventory.push(create_item(7, b"Notebook", b"Office", 4.99, 100));
    inventory.push(create_item(8, b"Pen Set", b"Office", 12.99, 75));
    inventory.push(create_item(9, b"USB Cable", b"Electronics", 9.99, 60));
    inventory.push(create_item(10, b"Bookshelf", b"Furniture", 149.99, 12));

    p!(out, "Added {} items to inventory\n", inventory.size());

    // Display all items
    p!(out, "\n--- All Inventory Items ---\n");
    for item in inventory.iter() {
        print_item(out, item);
        p!(out, "\n");
    }

    // Calculate statistics
    calculate_inventory_stats(out, &inventory);

    // Find items by category
    find_items_by_category(out, &inventory, b"Electronics");
    find_items_by_category(out, &inventory, b"Furniture");

    // Find low stock items
    p!(out, "\n--- Low Stock Items (< 20) ---\n");
    let mut low_stock_count: i32 = 0;
    for item in inventory.iter() {
        if item.quantity < 20 {
            print_item(out, item);
            low_stock_count += 1;
        }
    }
    p!(out, "Total low stock items: {}\n", low_stock_count);
}

fn demo_order_list(out: &mut dyn Write) {
    p!(out, "\n");
    p!(out, "========================================\n");
    p!(out, "  DEMO 4: Order List (Orders)\n");
    p!(out, "========================================\n");

    // Create order list
    let mut orders: List<Order> = List::create();

    // Add orders
    p!(out, "\n--- Adding Orders ---\n");
    orders.append(create_order(1001, b"Alice Johnson", 1249.95));
    orders.append(create_order(1002, b"Bob Smith", 89.99));
    orders.append(create_order(1003, b"Carol White", 549.98));
    orders.append(create_order(1004, b"David Brown", 24.99));
    orders.append(create_order(1005, b"Eve Davis", 899.99));
    orders.append(create_order(1006, b"Frank Miller", 374.97));
    orders.append(create_order(1007, b"Grace Lee", 159.98));
    orders.append(create_order(1008, b"Henry Wilson", 1099.99));

    p!(out, "Added {} orders\n", orders.size());

    // Display all orders
    p!(out, "\n--- All Orders ---\n");
    for order in orders.iter() {
        print_order(out, order);
    }

    // Calculate statistics
    calculate_order_stats(out, &orders);

    // Find large orders
    p!(out, "\n--- Large Orders (> $500) ---\n");
    let mut large_order_count: i32 = 0;
    let mut large_order_total: f64 = 0.0;

    for order in orders.iter() {
        if order.total_amount > 500.0 {
            print_order(out, order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    p!(out, "Total large orders: {}\n", large_order_count);
    p!(
        out,
        "Revenue from large orders: ${}\n",
        fmt_f(large_order_total, 2)
    );
}

fn demo_mixed_operations(out: &mut dyn Write) {
    p!(out, "\n");
    p!(out, "========================================\n");
    p!(out, "  DEMO 5: Mixed Operations\n");
    p!(out, "========================================\n");

    // Create both array and list with items
    let mut array_inventory: Array<Item> = Array::create(10);
    let mut list_inventory: List<Item> = List::create();

    p!(out, "\n--- Populating both Array and List ---\n");

    // Add same items to both
    let items: [Item; 5] = [
        create_item(1, b"Smartphone", b"Electronics", 699.99, 25),
        create_item(2, b"Tablet", b"Electronics", 449.99, 18),
        create_item(3, b"Headphones", b"Electronics", 149.99, 40),
        create_item(4, b"Smart Watch", b"Electronics", 299.99, 22),
        create_item(5, b"Power Bank", b"Electronics", 39.99, 55),
    ];

    let num_items: i32 = items.len() as i32;

    for item in items.iter() {
        array_inventory.push(*item);
        list_inventory.append(*item);
    }

    p!(out, "Added {} items to both containers\n", num_items);

    // Compare iteration performance (conceptually)
    p!(out, "\n--- Iterating through Array ---\n");
    let mut array_count: i32 = 0;
    for _item in array_inventory.iter() {
        array_count += 1;
    }
    p!(out, "Array iteration count: {}\n", array_count);

    p!(out, "\n--- Iterating through List ---\n");
    let mut list_count: i32 = 0;
    for _item in list_inventory.iter() {
        list_count += 1;
    }
    p!(out, "List iteration count: {}\n", list_count);

    // Find items above certain price in both
    let price_threshold: f64 = 200.0;

    p!(
        out,
        "\n--- Items above ${} (Array) ---\n",
        fmt_f(price_threshold, 2)
    );
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            p!(out, "  ");
            let _ = out.write_all(cstdio::cstr(&item.name));
            p!(out, ": ${}\n", fmt_f(item.price, 2));
        }
    }

    p!(
        out,
        "\n--- Items above ${} (List) ---\n",
        fmt_f(price_threshold, 2)
    );
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            p!(out, "  ");
            let _ = out.write_all(cstdio::cstr(&item.name));
            p!(out, ": ${}\n", fmt_f(item.price, 2));
        }
    }
}

fn run(out: &mut dyn Write, stdin: &mut dyn std::io::BufRead) -> i32 {
    p!(out, "\u{2554}{}\u{2557}\n", "\u{2550}".repeat(40));
    p!(out, "\u{2551}   GENERIC FOR_EACH MACRO DEMO         \u{2551}\n");
    p!(out, "\u{2551}   Demonstrating Generic Containers    \u{2551}\n");
    p!(out, "\u{255a}{}\u{255d}\n", "\u{2550}".repeat(40));

    loop {
        print_menu(out);

        // Keep the prompt visible before blocking on input, as glibc does when
        // stdout is line buffered.
        let _ = out.flush();

        let input = match fgets(stdin, 256) {
            Some(buf) => buf,
            None => break,
        };

        let choice = match sscanf_int(&input) {
            Some(c) => c,
            None => {
                p!(out, "Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => demo_integer_containers(out),

            2 => demo_double_containers(out),

            3 => demo_inventory_array(out),

            4 => demo_order_list(out),

            5 => demo_mixed_operations(out),

            6 => {
                p!(out, "\n=== Running All Demos ===\n");
                demo_integer_containers(out);
                demo_double_containers(out);
                demo_inventory_array(out);
                demo_order_list(out);
                demo_mixed_operations(out);
                p!(out, "\n========================================\n");
                p!(out, "  All demos completed successfully!\n");
                p!(out, "========================================\n");
            }

            7 => {
                p!(out, "\nGoodbye!\n");
                return 0;
            }

            _ => {
                p!(out, "Invalid choice\n");
            }
        }
    }

    0
}

/// The Rust runtime installs SIG_IGN for SIGPIPE before `main` runs, but a C
/// program starts with the default disposition.  Restore it so that a vanished
/// stdout reader terminates this process with signal 13, exactly as it
/// terminates the C program, instead of silently swallowing the write error.
#[cfg(unix)]
fn reset_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn main() {
    reset_sigpipe();

    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let stdout = std::io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    let code = run(&mut writer, &mut reader);

    let _ = writer.flush();
    std::process::exit(code);
}
