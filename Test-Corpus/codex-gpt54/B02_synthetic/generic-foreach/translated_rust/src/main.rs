mod inventory;

use std::collections::LinkedList;
use std::io::{self, Read};

use inventory::{
    calculate_inventory_stats, calculate_order_stats, create_item, create_order,
    find_items_by_category, print_item, print_order, Item, Order,
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
}

fn demo_integer_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 1: Integer Containers");
    println!("========================================");

    let mut int_array = Vec::with_capacity(10);
    println!();
    println!("--- Integer Array ---");
    println!("Adding integers: 10, 20, 30, 40, 50");
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);

    print!("Array contents: ");
    for val in &int_array {
        print!("{} ", val);
    }
    println!();

    let mut sum = 0i32;
    for val in &int_array {
        sum += *val;
    }
    println!("Sum: {}", sum);
    println!("Average: {:.2}", f64::from(sum) / int_array.len() as f64);

    let mut int_list = LinkedList::new();
    println!();
    println!("--- Integer List ---");
    println!("Adding integers: 100, 200, 300, 400, 500");
    int_list.push_back(100);
    int_list.push_back(200);
    int_list.push_back(300);
    int_list.push_back(400);
    int_list.push_back(500);

    print!("List contents: ");
    for val in &int_list {
        print!("{} ", val);
    }
    println!();

    let mut product = 1i64;
    for val in &int_list {
        product *= i64::from(*val);
    }
    println!("Product: {}", product);
}

fn demo_double_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 2: Double Containers");
    println!("========================================");

    let mut double_array = Vec::with_capacity(5);
    println!();
    println!("--- Double Array (Temperatures in Celsius) ---");

    let temps = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    let num_temps = temps.len();

    print!("Adding temperatures: ");
    for temp in temps {
        double_array.push(temp);
        print!("{:.1} ", temp);
    }
    println!();

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

    println!("Minimum: {:.1}°C", min_temp);
    println!("Maximum: {:.1}°C", max_temp);
    println!("Average: {:.1}°C", sum_temp / num_temps as f64);

    let mut price_list = LinkedList::new();
    println!();
    println!("--- Double List (Product Prices) ---");

    let prices = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    print!("Adding prices: ");
    for price in prices {
        price_list.push_back(price);
        print!("${:.2} ", price);
    }
    println!();

    let mut total = 0.0;
    let mut count_under_10 = 0i32;

    for temp in &price_list {
        total += *temp;
        if *temp < 10.0 {
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

    let mut inventory = Vec::with_capacity(20);

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
    for item in &inventory {
        print_item(*item);
        println!();
    }

    calculate_inventory_stats(&inventory);

    find_items_by_category(&inventory, "Electronics");
    find_items_by_category(&inventory, "Furniture");

    println!();
    println!("--- Low Stock Items (< 20) ---");
    let mut low_stock_count = 0i32;
    for item in &inventory {
        if item.quantity < 20 {
            print_item(*item);
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

    let mut orders = LinkedList::new();

    println!();
    println!("--- Adding Orders ---");
    orders.push_back(create_order(1001, "Alice Johnson", 1249.95));
    orders.push_back(create_order(1002, "Bob Smith", 89.99));
    orders.push_back(create_order(1003, "Carol White", 549.98));
    orders.push_back(create_order(1004, "David Brown", 24.99));
    orders.push_back(create_order(1005, "Eve Davis", 899.99));
    orders.push_back(create_order(1006, "Frank Miller", 374.97));
    orders.push_back(create_order(1007, "Grace Lee", 159.98));
    orders.push_back(create_order(1008, "Henry Wilson", 1099.99));

    println!("Added {} orders", orders.len());

    println!();
    println!("--- All Orders ---");
    for order in &orders {
        print_order(*order);
    }

    calculate_order_stats(&orders);

    println!();
    println!("--- Large Orders (> $500) ---");
    let mut large_order_count = 0i32;
    let mut large_order_total = 0.0;

    for order in &orders {
        if order.total_amount > 500.0 {
            print_order(*order);
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
    let mut list_inventory: LinkedList<Item> = LinkedList::new();

    println!();
    println!("--- Populating both Array and List ---");

    let items = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items = items.len();

    for item in items {
        array_inventory.push(item);
        list_inventory.push_back(item);
    }

    println!("Added {} items to both containers", num_items);

    println!();
    println!("--- Iterating through Array ---");
    let mut array_count = 0i32;
    for _item in &array_inventory {
        array_count += 1;
    }
    println!("Array iteration count: {}", array_count);

    println!();
    println!("--- Iterating through List ---");
    let mut list_count = 0i32;
    for _item in &list_inventory {
        list_count += 1;
    }
    println!("List iteration count: {}", list_count);

    let price_threshold = 200.0;

    println!();
    println!("--- Items above ${:.2} (Array) ---", price_threshold);
    for item in &array_inventory {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", item.name(), item.price);
        }
    }

    println!();
    println!("--- Items above ${:.2} (List) ---", price_threshold);
    for item in &list_inventory {
        if item.price >= price_threshold {
            println!("  {}: ${:.2}", item.name(), item.price);
        }
    }
}

fn read_fgets_line<R: Read>(reader: &mut R, buffer_size: usize) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];

    while line.len() < buffer_size.saturating_sub(1) {
        match reader.read(&mut byte)? {
            0 => {
                if line.is_empty() {
                    return Ok(None);
                }
                break;
            }
            _ => {
                line.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
        }
    }

    Ok(Some(line))
}

fn parse_scanf_int(bytes: &[u8]) -> Option<i32> {
    let mut index = 0usize;

    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let mut sign = 1i64;
    if index < bytes.len() {
        if bytes[index] == b'-' {
            sign = -1;
            index += 1;
        } else if bytes[index] == b'+' {
            index += 1;
        }
    }

    if index >= bytes.len() || !bytes[index].is_ascii_digit() {
        return None;
    }

    let mut value = 0i64;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        value = value.checked_mul(10)?.checked_add(i64::from(bytes[index] - b'0'))?;
        index += 1;
    }

    let signed = value.checked_mul(sign)?;
    i32::try_from(signed).ok()
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║   GENERIC FOR_EACH MACRO DEMO         ║");
    println!("║   Demonstrating Generic Containers    ║");
    println!("╚════════════════════════════════════════╝");

    let stdin = io::stdin();
    let mut handle = stdin.lock();

    loop {
        print_menu();

        let Some(input) = read_fgets_line(&mut handle, 256).expect("stdin read failed") else {
            break;
        };

        let Some(choice) = parse_scanf_int(&input) else {
            println!("Invalid input");
            continue;
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
            _ => println!("Invalid choice"),
        }
    }
}
