mod inventory;

use inventory::{
    calculate_inventory_stats, calculate_order_stats, create_item, create_order,
    find_items_by_category, print_item, print_order, Item,
};
use std::collections::LinkedList;
use std::io::{self, Read, Write};

fn print_menu<W: Write>(out: &mut W) -> io::Result<()> {
    write!(
        out,
        "\n\
========================================\n\
\x20\x20GENERIC FOR_EACH MACRO DEMO\n\
========================================\n\
1. Demo: Integer Containers\n\
2. Demo: Double Containers\n\
3. Demo: Inventory Array\n\
4. Demo: Order List\n\
5. Demo: Mixed Operations\n\
6. Run All Demos\n\
7. Exit\n\
========================================\n\
Choice: "
    )
}

fn demo_integer_containers<W: Write>(out: &mut W) -> io::Result<()> {
    write!(
        out,
        "\n\
========================================\n\
\x20\x20DEMO 1: Integer Containers\n\
========================================\n\
\n\
--- Integer Array ---\n\
Adding integers: 10, 20, 30, 40, 50\n\
Array contents: "
    )?;

    let int_array = vec![10, 20, 30, 40, 50];
    for value in &int_array {
        write!(out, "{value} ")?;
    }

    let sum: i32 = int_array.iter().sum();
    writeln!(out)?;
    writeln!(out, "Sum: {sum}")?;
    writeln!(out, "Average: {:.2}", f64::from(sum) / int_array.len() as f64)?;

    write!(
        out,
        "\n\
--- Integer List ---\n\
Adding integers: 100, 200, 300, 400, 500\n\
List contents: "
    )?;

    let int_list: LinkedList<i32> = [100, 200, 300, 400, 500].into_iter().collect();
    for value in &int_list {
        write!(out, "{value} ")?;
    }

    let product: i64 = int_list.iter().map(|&value| i64::from(value)).product();
    writeln!(out)?;
    writeln!(out, "Product: {product}")
}

fn demo_double_containers<W: Write>(out: &mut W) -> io::Result<()> {
    write!(
        out,
        "\n\
========================================\n\
\x20\x20DEMO 2: Double Containers\n\
========================================\n\
\n\
--- Double Array (Temperatures in Celsius) ---\n\
Adding temperatures: "
    )?;

    let temperatures = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    for temperature in temperatures {
        write!(out, "{temperature:.1} ")?;
    }
    writeln!(out)?;

    let mut minimum = temperatures[0];
    let mut maximum = temperatures[0];
    let mut sum = 0.0;
    for temperature in temperatures {
        if temperature < minimum {
            minimum = temperature;
        }
        if temperature > maximum {
            maximum = temperature;
        }
        sum += temperature;
    }

    writeln!(out, "Minimum: {minimum:.1}°C")?;
    writeln!(out, "Maximum: {maximum:.1}°C")?;
    writeln!(out, "Average: {:.1}°C", sum / temperatures.len() as f64)?;
    writeln!(out, "\n--- Double List (Product Prices) ---")?;

    let prices: LinkedList<f64> = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75]
        .into_iter()
        .collect();
    write!(out, "Adding prices: ")?;
    for price in &prices {
        write!(out, "${price:.2} ")?;
    }
    writeln!(out)?;

    let mut total = 0.0;
    let mut count_under_10 = 0;
    for price in prices {
        total += price;
        if price < 10.0 {
            count_under_10 += 1;
        }
    }

    writeln!(out, "Total cost: ${total:.2}")?;
    writeln!(out, "Items under $10: {count_under_10}")
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

fn demo_inventory_array<W: Write>(out: &mut W) -> io::Result<()> {
    write!(
        out,
        "\n\
========================================\n\
\x20\x20DEMO 3: Inventory Array (Items)\n\
========================================\n\
\n\
--- Adding Items to Inventory ---\n"
    )?;

    let inventory = inventory_items();
    writeln!(out, "Added {} items to inventory", inventory.len())?;
    writeln!(out, "\n--- All Inventory Items ---")?;
    for &item in &inventory {
        print_item(out, item)?;
        writeln!(out)?;
    }

    calculate_inventory_stats(out, &inventory)?;
    find_items_by_category(out, &inventory, "Electronics")?;
    find_items_by_category(out, &inventory, "Furniture")?;

    writeln!(out, "\n--- Low Stock Items (< 20) ---")?;
    let mut low_stock_count = 0;
    for &item in &inventory {
        if item.quantity < 20 {
            print_item(out, item)?;
            low_stock_count += 1;
        }
    }
    writeln!(out, "Total low stock items: {low_stock_count}")
}

fn demo_order_list<W: Write>(out: &mut W) -> io::Result<()> {
    write!(
        out,
        "\n\
========================================\n\
\x20\x20DEMO 4: Order List (Orders)\n\
========================================\n\
\n\
--- Adding Orders ---\n"
    )?;

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

    writeln!(out, "Added {} orders", orders.len())?;
    writeln!(out, "\n--- All Orders ---")?;
    for &order in &orders {
        print_order(out, order)?;
    }

    calculate_order_stats(out, &orders)?;
    writeln!(out, "\n--- Large Orders (> $500) ---")?;

    let mut large_order_count = 0;
    let mut large_order_total = 0.0;
    for &order in &orders {
        if order.total_amount > 500.0 {
            print_order(out, order)?;
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    writeln!(out, "Total large orders: {large_order_count}")?;
    writeln!(
        out,
        "Revenue from large orders: ${large_order_total:.2}"
    )
}

fn demo_mixed_operations<W: Write>(out: &mut W) -> io::Result<()> {
    write!(
        out,
        "\n\
========================================\n\
\x20\x20DEMO 5: Mixed Operations\n\
========================================\n\
\n\
--- Populating both Array and List ---\n"
    )?;

    let items = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];
    let array_inventory = items.to_vec();
    let list_inventory: LinkedList<Item> = items.into_iter().collect();

    writeln!(
        out,
        "Added {} items to both containers",
        array_inventory.len()
    )?;
    writeln!(out, "\n--- Iterating through Array ---")?;
    let mut array_count = 0;
    for _item in &array_inventory {
        array_count += 1;
    }
    writeln!(out, "Array iteration count: {array_count}")?;

    writeln!(out, "\n--- Iterating through List ---")?;
    let mut list_count = 0;
    for _item in &list_inventory {
        list_count += 1;
    }
    writeln!(out, "List iteration count: {list_count}")?;

    let price_threshold = 200.0;
    writeln!(
        out,
        "\n--- Items above ${price_threshold:.2} (Array) ---"
    )?;
    for item in &array_inventory {
        if item.price >= price_threshold {
            writeln!(out, "  {}: ${:.2}", item.name, item.price)?;
        }
    }

    writeln!(
        out,
        "\n--- Items above ${price_threshold:.2} (List) ---"
    )?;
    for item in &list_inventory {
        if item.price >= price_threshold {
            writeln!(out, "  {}: ${:.2}", item.name, item.price)?;
        }
    }

    Ok(())
}

fn fgets_256<R: Read>(input: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::with_capacity(255);
    let mut byte = [0_u8; 1];

    while bytes.len() < 255 {
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                bytes.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }

    if bytes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(bytes))
    }
}

fn sscanf_decimal_int(input: &[u8]) -> Option<i32> {
    let mut index = 0;
    while index < input.len()
        && input[index] != 0
        && matches!(input[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let negative = match input.get(index).copied() {
        Some(b'+') => {
            index += 1;
            false
        }
        Some(b'-') => {
            index += 1;
            true
        }
        _ => false,
    };

    let digit_start = index;
    let mut magnitude = 0_u128;
    while let Some(&byte) = input.get(index) {
        if !byte.is_ascii_digit() {
            break;
        }
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add(u128::from(byte - b'0'));
        index += 1;
    }
    if index == digit_start {
        return None;
    }

    // glibc's scanf converts through long before storing into the int target.
    let long_value = if negative {
        if magnitude >= (i64::MAX as u128) + 1 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else if magnitude > i64::MAX as u128 {
        i64::MAX
    } else {
        magnitude as i64
    };
    Some(long_value as i32)
}

fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut out = stdout.lock();

    write!(
        out,
        "╔════════════════════════════════════════╗\n\
║   GENERIC FOR_EACH MACRO DEMO         ║\n\
║   Demonstrating Generic Containers    ║\n\
╚════════════════════════════════════════╝\n"
    )?;

    loop {
        print_menu(&mut out)?;
        out.flush()?;

        let Some(line) = fgets_256(&mut input)? else {
            break;
        };

        let Some(choice) = sscanf_decimal_int(&line) else {
            writeln!(out, "Invalid input")?;
            continue;
        };

        match choice {
            1 => demo_integer_containers(&mut out)?,
            2 => demo_double_containers(&mut out)?,
            3 => demo_inventory_array(&mut out)?,
            4 => demo_order_list(&mut out)?,
            5 => demo_mixed_operations(&mut out)?,
            6 => {
                writeln!(out, "\n=== Running All Demos ===")?;
                demo_integer_containers(&mut out)?;
                demo_double_containers(&mut out)?;
                demo_inventory_array(&mut out)?;
                demo_order_list(&mut out)?;
                demo_mixed_operations(&mut out)?;
                write!(
                    out,
                    "\n\
========================================\n\
\x20\x20All demos completed successfully!\n\
========================================\n"
                )?;
            }
            7 => {
                writeln!(out, "\nGoodbye!")?;
                return Ok(());
            }
            _ => writeln!(out, "Invalid choice")?,
        }
    }

    Ok(())
}

fn main() {
    let _ = run();
}
