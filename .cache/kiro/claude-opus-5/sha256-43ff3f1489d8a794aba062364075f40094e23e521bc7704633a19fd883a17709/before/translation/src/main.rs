//! Translation of `main.c`.

mod cio;
mod containers;
mod cstr;
mod inventory;

use std::io::Write;

use cio::{sscanf_int, Stdin};
use containers::{Array, List};
use inventory::{
    calculate_inventory_stats, calculate_order_stats, create_item, create_order,
    find_items_by_category, print_item, print_order, Item,
};

fn print_menu(out: &mut impl Write) {
    let _ = write!(out, "\n");
    let _ = write!(out, "========================================\n");
    let _ = write!(out, "  GENERIC FOR_EACH MACRO DEMO\n");
    let _ = write!(out, "========================================\n");
    let _ = write!(out, "1. Demo: Integer Containers\n");
    let _ = write!(out, "2. Demo: Double Containers\n");
    let _ = write!(out, "3. Demo: Inventory Array\n");
    let _ = write!(out, "4. Demo: Order List\n");
    let _ = write!(out, "5. Demo: Mixed Operations\n");
    let _ = write!(out, "6. Run All Demos\n");
    let _ = write!(out, "7. Exit\n");
    let _ = write!(out, "========================================\n");
    let _ = write!(out, "Choice: ");
}

fn demo_integer_containers(out: &mut impl Write) {
    let _ = write!(out, "\n");
    let _ = write!(out, "========================================\n");
    let _ = write!(out, "  DEMO 1: Integer Containers\n");
    let _ = write!(out, "========================================\n");

    let mut int_array: Array<i32> = Array::create(10);
    let _ = write!(out, "\n--- Integer Array ---\n");
    let _ = write!(out, "Adding integers: 10, 20, 30, 40, 50\n");
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);

    let _ = write!(out, "Array contents: ");
    for val in int_array.iter() {
        let _ = write!(out, "{} ", val);
    }
    let _ = write!(out, "\n");

    // Sum accumulated in an `int`, so wrap on overflow like C's -fwrapv-less
    // gcc does in practice.
    let mut sum: i32 = 0;
    for val in int_array.iter() {
        sum = sum.wrapping_add(*val);
    }
    let _ = write!(out, "Sum: {}\n", sum);
    let _ = write!(
        out,
        "Average: {:.2}\n",
        f64::from(sum) / int_array.size() as f64
    );

    let mut int_list: List<i32> = List::create();
    let _ = write!(out, "\n--- Integer List ---\n");
    let _ = write!(out, "Adding integers: 100, 200, 300, 400, 500\n");
    int_list.append(100);
    int_list.append(200);
    int_list.append(300);
    int_list.append(400);
    int_list.append(500);

    let _ = write!(out, "List contents: ");
    for val in int_list.iter() {
        let _ = write!(out, "{} ", val);
    }
    let _ = write!(out, "\n");

    // `long long product`
    let mut product: i64 = 1;
    for val in int_list.iter() {
        product = product.wrapping_mul(i64::from(*val));
    }
    let _ = write!(out, "Product: {}\n", product);

    int_array.destroy();
    int_list.destroy();
}

fn demo_double_containers(out: &mut impl Write) {
    let _ = write!(out, "\n");
    let _ = write!(out, "========================================\n");
    let _ = write!(out, "  DEMO 2: Double Containers\n");
    let _ = write!(out, "========================================\n");

    let mut double_array: Array<f64> = Array::create(5);
    let _ = write!(
        out,
        "\n--- Double Array (Temperatures in Celsius) ---\n"
    );

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];

    let _ = write!(out, "Adding temperatures: ");
    for t in temps.iter() {
        double_array.push(*t);
        let _ = write!(out, "{:.1} ", t);
    }
    let _ = write!(out, "\n");

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

    let _ = write!(out, "Minimum: {:.1}\u{b0}C\n", min_temp);
    let _ = write!(out, "Maximum: {:.1}\u{b0}C\n", max_temp);
    let _ = write!(
        out,
        "Average: {:.1}\u{b0}C\n",
        sum_temp / double_array.size() as f64
    );

    let mut price_list: List<f64> = List::create();
    let _ = write!(out, "\n--- Double List (Product Prices) ---\n");

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];

    let _ = write!(out, "Adding prices: ");
    for p in prices.iter() {
        price_list.append(*p);
        let _ = write!(out, "${:.2} ", p);
    }
    let _ = write!(out, "\n");

    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;

    for temp in price_list.iter() {
        total += *temp;
        if *temp < 10.0 {
            count_under_10 += 1;
        }
    }

    let _ = write!(out, "Total cost: ${:.2}\n", total);
    let _ = write!(out, "Items under $10: {}\n", count_under_10);

    double_array.destroy();
    price_list.destroy();
}

fn demo_inventory_array(out: &mut impl Write) {
    let _ = write!(out, "\n");
    let _ = write!(out, "========================================\n");
    let _ = write!(out, "  DEMO 3: Inventory Array (Items)\n");
    let _ = write!(out, "========================================\n");

    let mut inventory: Array<Item> = Array::create(20);

    let _ = write!(out, "\n--- Adding Items to Inventory ---\n");
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

    let _ = write!(out, "Added {} items to inventory\n", inventory.size());

    let _ = write!(out, "\n--- All Inventory Items ---\n");
    for item in inventory.iter() {
        print_item(out, item);
        let _ = write!(out, "\n");
    }

    calculate_inventory_stats(out, &inventory);

    find_items_by_category(out, &inventory, "Electronics");
    find_items_by_category(out, &inventory, "Furniture");

    let _ = write!(out, "\n--- Low Stock Items (< 20) ---\n");
    let mut low_stock_count: i32 = 0;
    for item in inventory.iter() {
        if item.quantity < 20 {
            print_item(out, item);
            low_stock_count += 1;
        }
    }
    let _ = write!(out, "Total low stock items: {}\n", low_stock_count);

    inventory.destroy();
}

fn demo_order_list(out: &mut impl Write) {
    let _ = write!(out, "\n");
    let _ = write!(out, "========================================\n");
    let _ = write!(out, "  DEMO 4: Order List (Orders)\n");
    let _ = write!(out, "========================================\n");

    let mut orders = List::create();

    let _ = write!(out, "\n--- Adding Orders ---\n");
    orders.append(create_order(1001, "Alice Johnson", 1249.95));
    orders.append(create_order(1002, "Bob Smith", 89.99));
    orders.append(create_order(1003, "Carol White", 549.98));
    orders.append(create_order(1004, "David Brown", 24.99));
    orders.append(create_order(1005, "Eve Davis", 899.99));
    orders.append(create_order(1006, "Frank Miller", 374.97));
    orders.append(create_order(1007, "Grace Lee", 159.98));
    orders.append(create_order(1008, "Henry Wilson", 1099.99));

    let _ = write!(out, "Added {} orders\n", orders.size());

    let _ = write!(out, "\n--- All Orders ---\n");
    for order in orders.iter() {
        print_order(out, order);
    }

    calculate_order_stats(out, &orders);

    let _ = write!(out, "\n--- Large Orders (> $500) ---\n");
    let mut large_order_count: i32 = 0;
    let mut large_order_total: f64 = 0.0;

    for order in orders.iter() {
        if order.total_amount > 500.0 {
            print_order(out, order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    let _ = write!(out, "Total large orders: {}\n", large_order_count);
    let _ = write!(
        out,
        "Revenue from large orders: ${:.2}\n",
        large_order_total
    );

    orders.destroy();
}

fn demo_mixed_operations(out: &mut impl Write) {
    let _ = write!(out, "\n");
    let _ = write!(out, "========================================\n");
    let _ = write!(out, "  DEMO 5: Mixed Operations\n");
    let _ = write!(out, "========================================\n");

    let mut array_inventory: Array<Item> = Array::create(10);
    let mut list_inventory: List<Item> = List::create();

    let _ = write!(out, "\n--- Populating both Array and List ---\n");

    let items: [Item; 5] = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items: i32 = items.len() as i32;

    for item in items.iter() {
        array_inventory.push(*item);
        list_inventory.append(*item);
    }

    let _ = write!(out, "Added {} items to both containers\n", num_items);

    let _ = write!(out, "\n--- Iterating through Array ---\n");
    let mut array_count: i32 = 0;
    for _item in array_inventory.iter() {
        array_count += 1;
    }
    let _ = write!(out, "Array iteration count: {}\n", array_count);

    let _ = write!(out, "\n--- Iterating through List ---\n");
    let mut list_count: i32 = 0;
    for _item in list_inventory.iter() {
        list_count += 1;
    }
    let _ = write!(out, "List iteration count: {}\n", list_count);

    let price_threshold: f64 = 200.0;

    let _ = write!(
        out,
        "\n--- Items above ${:.2} (Array) ---\n",
        price_threshold
    );
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            let _ = write!(
                out,
                "  {}: ${:.2}\n",
                cstr::cstr_str(&item.name),
                item.price
            );
        }
    }

    let _ = write!(
        out,
        "\n--- Items above ${:.2} (List) ---\n",
        price_threshold
    );
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            let _ = write!(
                out,
                "  {}: ${:.2}\n",
                cstr::cstr_str(&item.name),
                item.price
            );
        }
    }

    array_inventory.destroy();
    list_inventory.destroy();
}

fn main() {
    // stdout is written through one buffer and flushed before each read, which
    // reproduces C's ordering while keeping the byte stream identical.
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let mut stdin = Stdin::new();

    let _ = write!(out, "\u{2554}{}\u{2557}\n", "\u{2550}".repeat(40));
    let _ = write!(out, "\u{2551}   GENERIC FOR_EACH MACRO DEMO         \u{2551}\n");
    let _ = write!(out, "\u{2551}   Demonstrating Generic Containers    \u{2551}\n");
    let _ = write!(out, "\u{255a}{}\u{255d}\n", "\u{2550}".repeat(40));

    loop {
        print_menu(&mut out);
        let _ = out.flush();

        // `char input[256]` -> fgets(input, sizeof(input), stdin)
        let input = match stdin.fgets(256) {
            Some(line) => line,
            None => break,
        };

        let (matched, choice) = sscanf_int(&input);
        if matched != 1 {
            let _ = write!(out, "Invalid input\n");
            continue;
        }

        match choice {
            1 => demo_integer_containers(&mut out),
            2 => demo_double_containers(&mut out),
            3 => demo_inventory_array(&mut out),
            4 => demo_order_list(&mut out),
            5 => demo_mixed_operations(&mut out),
            6 => {
                let _ = write!(out, "\n=== Running All Demos ===\n");
                demo_integer_containers(&mut out);
                demo_double_containers(&mut out);
                demo_inventory_array(&mut out);
                demo_order_list(&mut out);
                demo_mixed_operations(&mut out);
                let _ = write!(out, "\n========================================\n");
                let _ = write!(out, "  All demos completed successfully!\n");
                let _ = write!(out, "========================================\n");
            }
            7 => {
                let _ = write!(out, "\nGoodbye!\n");
                let _ = out.flush();
                std::process::exit(0);
            }
            _ => {
                let _ = write!(out, "Invalid choice\n");
            }
        }
    }

    let _ = out.flush();
    std::process::exit(0);
}
