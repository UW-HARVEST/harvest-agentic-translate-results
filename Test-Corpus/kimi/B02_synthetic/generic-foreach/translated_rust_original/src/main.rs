use std::io::{self, Write};

mod inventory;
use inventory::{Item, Order, InventoryArray, InventoryList, OrderList, IntArray, IntList, DoubleArray, DoubleList};

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
    io::stdout().flush().unwrap();
}

fn demo_integer_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 1: Integer Containers");
    println!("========================================");
    
    let mut int_array = IntArray::with_capacity(10);
    println!("\n--- Integer Array ---");
    println!("Adding integers: 10, 20, 30, 40, 50");
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);
    
    print!("Array contents: ");
    for val in int_array.iter() {
        print!("{} ", val);
    }
    println!();
    
    let sum: i32 = int_array.iter().sum();
    println!("Sum: {}", sum);
    println!("Average: {:.2}", sum as f64 / int_array.len() as f64);
    
    let mut int_list = IntList::new();
    println!("\n--- Integer List ---");
    println!("Adding integers: 100, 200, 300, 400, 500");
    int_list.append(100);
    int_list.append(200);
    int_list.append(300);
    int_list.append(400);
    int_list.append(500);
    
    print!("List contents: ");
    for val in int_list.iter() {
        print!("{} ", val);
    }
    println!();
    
    let product: i64 = int_list.iter().map(|&x| x as i64).product();
    println!("Product: {}", product);
}

fn demo_double_containers() {
    println!();
    println!("========================================");
    println!("  DEMO 2: Double Containers");
    println!("========================================");
    
    let mut double_array = DoubleArray::with_capacity(5);
    println!("\n--- Double Array (Temperatures in Celsius) ---");
    
    let temps = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    
    print!("Adding temperatures: ");
    for &temp in &temps {
        double_array.push(temp);
        print!("{:.1} ", temp);
    }
    println!();
    
    let min_temp = double_array.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_temp = double_array.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let sum_temp: f64 = double_array.iter().sum();
    
    println!("Minimum: {:.1}°C", min_temp);
    println!("Maximum: {:.1}°C", max_temp);
    println!("Average: {:.1}°C", sum_temp / double_array.len() as f64);
    
    let mut price_list = DoubleList::new();
    println!("\n--- Double List (Product Prices) ---");
    
    let prices = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    
    print!("Adding prices: ");
    for &price in &prices {
        price_list.append(price);
        print!("${:.2} ", price);
    }
    println!();
    
    let total: f64 = price_list.iter().sum();
    let count_under_10 = price_list.iter().filter(|&&p| p < 10.0).count();
    
    println!("Total cost: ${:.2}", total);
    println!("Items under $10: {}", count_under_10);
}

fn demo_inventory_array() {
    println!();
    println!("========================================");
    println!("  DEMO 3: Inventory Array (Items)");
    println!("========================================");
    
    let mut inventory = InventoryArray::with_capacity(20);
    
    println!("\n--- Adding Items to Inventory ---");
    inventory.push(Item::new(1, "Laptop", "Electronics", 899.99, 15));
    inventory.push(Item::new(2, "Mouse", "Electronics", 24.99, 50));
    inventory.push(Item::new(3, "Keyboard", "Electronics", 79.99, 30));
    inventory.push(Item::new(4, "Monitor", "Electronics", 299.99, 20));
    inventory.push(Item::new(5, "Desk Chair", "Furniture", 199.99, 10));
    inventory.push(Item::new(6, "Desk", "Furniture", 349.99, 8));
    inventory.push(Item::new(7, "Notebook", "Office", 4.99, 100));
    inventory.push(Item::new(8, "Pen Set", "Office", 12.99, 75));
    inventory.push(Item::new(9, "USB Cable", "Electronics", 9.99, 60));
    inventory.push(Item::new(10, "Bookshelf", "Furniture", 149.99, 12));
    
    println!("Added {} items to inventory", inventory.len());
    
    println!("\n--- All Inventory Items ---");
    for item in inventory.iter() {
        item.print();
        println!();
    }
    
    inventory.calculate_stats();
    
    inventory.find_by_category("Electronics");
    inventory.find_by_category("Furniture");
    
    println!("\n--- Low Stock Items (< 20) ---");
    let low_stock: Vec<&Item> = inventory.iter().filter(|i| i.quantity < 20).collect();
    for item in &low_stock {
        item.print();
    }
    println!("Total low stock items: {}", low_stock.len());
}

fn demo_order_list() {
    println!();
    println!("========================================");
    println!("  DEMO 4: Order List (Orders)");
    println!("========================================");
    
    let mut orders = OrderList::new();
    
    println!("\n--- Adding Orders ---");
    orders.append(Order::new(1001, "Alice Johnson", 1249.95));
    orders.append(Order::new(1002, "Bob Smith", 89.99));
    orders.append(Order::new(1003, "Carol White", 549.98));
    orders.append(Order::new(1004, "David Brown", 24.99));
    orders.append(Order::new(1005, "Eve Davis", 899.99));
    orders.append(Order::new(1006, "Frank Miller", 374.97));
    orders.append(Order::new(1007, "Grace Lee", 159.98));
    orders.append(Order::new(1008, "Henry Wilson", 1099.99));
    
    println!("Added {} orders", orders.len());
    
    println!("\n--- All Orders ---");
    for order in orders.iter() {
        order.print();
    }
    
    orders.calculate_stats();
    
    println!("\n--- Large Orders (> $500) ---");
    let large_orders: Vec<&Order> = orders.iter().filter(|o| o.total_amount > 500.0).collect();
    for order in &large_orders {
        order.print();
    }
    let large_order_total: f64 = large_orders.iter().map(|o| o.total_amount).sum();
    println!("Total large orders: {}", large_orders.len());
    println!("Revenue from large orders: ${:.2}", large_order_total);
}

fn demo_mixed_operations() {
    println!();
    println!("========================================");
    println!("  DEMO 5: Mixed Operations");
    println!("========================================");
    
    let mut array_inventory = InventoryArray::with_capacity(10);
    let mut list_inventory = InventoryList::new();
    
    println!("\n--- Populating both Array and List ---");
    
    let items = [
        Item::new(1, "Smartphone", "Electronics", 699.99, 25),
        Item::new(2, "Tablet", "Electronics", 449.99, 18),
        Item::new(3, "Headphones", "Electronics", 149.99, 40),
        Item::new(4, "Smart Watch", "Electronics", 299.99, 22),
        Item::new(5, "Power Bank", "Electronics", 39.99, 55),
    ];
    
    for item in &items {
        array_inventory.push(item.clone());
        list_inventory.append(item.clone());
    }
    
    println!("Added {} items to both containers", items.len());
    
    println!("\n--- Iterating through Array ---");
    let array_count = array_inventory.iter().count();
    println!("Array iteration count: {}", array_count);
    
    println!("\n--- Iterating through List ---");
    let list_count = list_inventory.iter().count();
    println!("List iteration count: {}", list_count);
    
    let price_threshold = 200.0;
    
    println!("\n--- Items above ${:.2} (Array) ---", price_threshold);
    for item in array_inventory.iter().filter(|i| i.price >= price_threshold) {
        println!("  {}: ${:.2}", item.name, item.price);
    }
    
    println!("\n--- Items above ${:.2} (List) ---", price_threshold);
    for item in list_inventory.iter().filter(|i| i.price >= price_threshold) {
        println!("  {}: ${:.2}", item.name, item.price);
    }
}

fn main() {
    println!("╔════════════════════════════════════════╗");
    println!("║   GENERIC FOR_EACH MACRO DEMO         ║");
    println!("║   Demonstrating Generic Containers    ║");
    println!("╚════════════════════════════════════════╝");
    
    loop {
        print_menu();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        
        let choice: i32 = match input.trim().parse() {
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
            _ => println!("Invalid choice"),
        }
    }
}
