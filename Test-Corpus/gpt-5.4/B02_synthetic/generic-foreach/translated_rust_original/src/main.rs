mod generic_containers;
mod inventory;

use std::io;

use generic_containers::*;
use inventory::*;

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
    use std::io::Write;
    let _ = io::stdout().flush();
}

fn demo_integer_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 1: Integer Containers");
    println!("========================================");

    let mut int_array = array_int_create(10);
    println!("\n--- Integer Array ---");
    println!("Adding integers: 10, 20, 30, 40, 50");
    array_int_push(&mut int_array, 10);
    array_int_push(&mut int_array, 20);
    array_int_push(&mut int_array, 30);
    array_int_push(&mut int_array, 40);
    array_int_push(&mut int_array, 50);

    print!("Array contents: ");
    use std::io::Write;
    let _ = io::stdout().flush();
    for val in int_array.iter() {
        print!("{} ", val);
    }
    println!();

    let mut sum = 0;
    for val in int_array.iter() {
        sum += *val;
    }
    println!("Sum: {}", sum);
    println!("Average: {:.2}", sum as f64 / int_array.size() as f64);

    let mut int_list = list_int_create();
    println!("\n--- Integer List ---");
    println!("Adding integers: 100, 200, 300, 400, 500");
    list_int_append(&mut int_list, 100);
    list_int_append(&mut int_list, 200);
    list_int_append(&mut int_list, 300);
    list_int_append(&mut int_list, 400);
    list_int_append(&mut int_list, 500);

    print!("List contents: ");
    let _ = io::stdout().flush();
    for val in int_list.iter() {
        print!("{} ", val);
    }
    println!();

    let mut product: i64 = 1;
    for val in int_list.iter() {
        product *= *val as i64;
    }
    println!("Product: {}", product);

    array_int_destroy(int_array);
    list_int_destroy(int_list);
}

fn demo_double_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 2: Double Containers");
    println!("========================================");

    let mut double_array = array_double_create(5);
    println!("\n--- Double Array (Temperatures in Celsius) ---");

    let temps = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    print!("Adding temperatures: ");
    use std::io::Write;
    let _ = io::stdout().flush();
    for temp in temps {
        array_double_push(&mut double_array, temp);
        print!("{:.1} ", temp);
    }
    println!();

    let mut min_temp = temps[0];
    let mut max_temp = temps[0];
    let mut sum_temp = 0.0;

    for temp in double_array.iter() {
        if *temp < min_temp {
            min_temp = *temp;
        }
        if *temp > max_temp {
            max_temp = *temp;
        }
        sum_temp += *temp;
    }

    println!("Minimum: {:.1}°C", min_temp);
    println!("Maximum: {:.1}°C", max_temp);
    println!("Average: {:.1}°C", sum_temp / double_array.size() as f64);

    let mut price_list = list_double_create();
    println!("\n--- Double List (Product Prices) ---");

    let prices = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    print!("Adding prices: ");
    let _ = io::stdout().flush();
    for price in prices {
        list_double_append(&mut price_list, price);
        print!("${:.2} ", price);
    }
    println!();

    let mut total = 0.0;
    let mut count_under_10 = 0;

    for temp in price_list.iter() {
        total += *temp;
        if *temp < 10.0 {
            count_under_10 += 1;
        }
    }

    println!("Total cost: ${:.2}", total);
    println!("Items under $10: {}", count_under_10);

    array_double_destroy(double_array);
    list_double_destroy(price_list);
}

fn demo_inventory_array() {
    println!();
    println!("========================================");
    println!("  DEMO 3: Inventory Array (Items)");
    println!("========================================");

    let mut inventory = array_item_t_create(20);

    println!("\n--- Adding Items to Inventory ---");
    array_item_t_push(&mut inventory, create_item(1, "Laptop", "Electronics", 899.99, 15));
    array_item_t_push(&mut inventory, create_item(2, "Mouse", "Electronics", 24.99, 50));
    array_item_t_push(&mut inventory, create_item(3, "Keyboard", "Electronics", 79.99, 30));
    array_item_t_push(&mut inventory, create_item(4, "Monitor", "Electronics", 299.99, 20));
    array_item_t_push(&mut inventory, create_item(5, "Desk Chair", "Furniture", 199.99, 10));
    array_item_t_push(&mut inventory, create_item(6, "Desk", "Furniture", 349.99, 8));
    array_item_t_push(&mut inventory, create_item(7, "Notebook", "Office", 4.99, 100));
    array_item_t_push(&mut inventory, create_item(8, "Pen Set", "Office", 12.99, 75));
    array_item_t_push(&mut inventory, create_item(9, "USB Cable", "Electronics", 9.99, 60));
    array_item_t_push(&mut inventory, create_item(10, "Bookshelf", "Furniture", 149.99, 12));

    println!("Added {} items to inventory", inventory.size());

    println!("\n--- All Inventory Items ---");
    for item in inventory.iter() {
        print_item(item);
        println!();
    }

    calculate_inventory_stats(&inventory);

    find_items_by_category(&inventory, "Electronics");
    find_items_by_category(&inventory, "Furniture");

    println!("\n--- Low Stock Items (< 20) ---");
    let mut low_stock_count = 0;
    for item in inventory.iter() {
        if item.quantity < 20 {
            print_item(item);
            low_stock_count += 1;
        }
    }
    println!("Total low stock items: {}", low_stock_count);

    array_item_t_destroy(inventory);
}

fn demo_order_list() {
    println!();
    println!("========================================");
    println!("  DEMO 4: Order List (Orders)");
    println!("========================================");

    let mut orders = list_order_t_create();

    println!("\n--- Adding Orders ---");
    list_order_t_append(&mut orders, create_order(1001, "Alice Johnson", 1249.95));
    list_order_t_append(&mut orders, create_order(1002, "Bob Smith", 89.99));
    list_order_t_append(&mut orders, create_order(1003, "Carol White", 549.98));
    list_order_t_append(&mut orders, create_order(1004, "David Brown", 24.99));
    list_order_t_append(&mut orders, create_order(1005, "Eve Davis", 899.99));
    list_order_t_append(&mut orders, create_order(1006, "Frank Miller", 374.97));
    list_order_t_append(&mut orders, create_order(1007, "Grace Lee", 159.98));
    list_order_t_append(&mut orders, create_order(1008, "Henry Wilson", 1099.99));

    println!("Added {} orders", orders.size());

    println!("\n--- All Orders ---");
    for order in orders.iter() {
        print_order(order);
    }

    calculate_order_stats(&orders);

    println!("\n--- Large Orders (> $500) ---");
    let mut large_order_count = 0;
    let mut large_order_total = 0.0;

    for order in orders.iter() {
        if order.total_amount > 500.0 {
            print_order(order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    println!("Total large orders: {}", large_order_count);
    println!("Revenue from large orders: ${:.2}", large_order_total);

    list_order_t_destroy(orders);
}

fn demo_mixed_operations() {
    println!();
    println!("========================================");
    println!("  DEMO 5: Mixed Operations");
    println!("========================================");

    let mut array_inventory = array_item_t_create(10);
    let mut list_inventory = list_item_t_create();

    println!("\n--- Populating both Array and List ---");

    let items = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    for item in items.iter().cloned() {
        array_item_t_push(&mut array_inventory, item.clone());
        list_item_t_append(&mut list_inventory, item);
    }

    println!("Added {} items to both containers", items.len());

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

    let price_threshold = 200.0;

    println!("\n--- Items above ${:.2} (Array) ---", price_threshold);
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", item.name, item.price);
        }
    }

    println!("\n--- Items above ${:.2} (List) ---", price_threshold);
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", item.name, item.price);
        }
    }

    array_item_t_destroy(array_inventory);
    list_item_t_destroy(list_inventory);
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║   GENERIC FOR_EACH MACRO DEMO         ║");
    println!("║   Demonstrating Generic Containers    ║");
    println!("╚════════════════════════════════════════╝");

    let mut input = String::new();

    loop {
        print_menu();
        input.clear();

        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let choice = match input.trim().parse::<i32>() {
            Ok(choice) => choice,
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
            _ => println!("Invalid choice"),
        }
    }
}
