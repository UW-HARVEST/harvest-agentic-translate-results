/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
//! main.rs -- translation of main.c

/// `printf`-alike: writes to the buffered stdout handle and, like `printf`,
/// silently ignores write errors instead of panicking.
macro_rules! cprintf {
    ($out:expr, $($arg:tt)*) => {{
        let _ = std::io::Write::write_fmt($out, format_args!($($arg)*));
    }};
}

mod generic_containers;
mod inventory;

use std::io::{BufReader, BufWriter, Read, Stdin, Write};

use generic_containers::{Array, List};
use inventory::{
    calculate_inventory_stats, calculate_order_stats, create_item, create_order,
    find_items_by_category, print_item, print_order, ItemT, OrderT,
};

/// `void print_menu(void)`
fn print_menu(out: &mut dyn Write) {
    cprintf!(out, "\n");
    cprintf!(out, "========================================\n");
    cprintf!(out, "  GENERIC FOR_EACH MACRO DEMO\n");
    cprintf!(out, "========================================\n");
    cprintf!(out, "1. Demo: Integer Containers\n");
    cprintf!(out, "2. Demo: Double Containers\n");
    cprintf!(out, "3. Demo: Inventory Array\n");
    cprintf!(out, "4. Demo: Order List\n");
    cprintf!(out, "5. Demo: Mixed Operations\n");
    cprintf!(out, "6. Run All Demos\n");
    cprintf!(out, "7. Exit\n");
    cprintf!(out, "========================================\n");
    cprintf!(out, "Choice: ");
}

/// `void demo_integer_containers(void)`
fn demo_integer_containers(out: &mut dyn Write) {
    cprintf!(out, "\n");
    cprintf!(out, "========================================\n");
    cprintf!(out, "  DEMO 1: Integer Containers\n");
    cprintf!(out, "========================================\n");

    // Create integer array
    let mut int_array: Array<i32> = Array::create(10);
    cprintf!(out, "\n--- Integer Array ---\n");
    cprintf!(out, "Adding integers: 10, 20, 30, 40, 50\n");
    int_array.push(10);
    int_array.push(20);
    int_array.push(30);
    int_array.push(40);
    int_array.push(50);

    cprintf!(out, "Array contents: ");
    for val in int_array.iter() {
        cprintf!(out, "{} ", val);
    }
    cprintf!(out, "\n");

    // Calculate sum using ARRAY_FOREACH
    let mut sum: i32 = 0;
    for val in int_array.iter() {
        sum = sum.wrapping_add(*val);
    }
    cprintf!(out, "Sum: {}\n", sum);
    cprintf!(
        out,
        "Average: {:.2}\n",
        (sum as f64) / (int_array.size() as f64)
    );

    // Create integer list
    let mut int_list: List<i32> = List::create();
    cprintf!(out, "\n--- Integer List ---\n");
    cprintf!(out, "Adding integers: 100, 200, 300, 400, 500\n");
    int_list.append(100);
    int_list.append(200);
    int_list.append(300);
    int_list.append(400);
    int_list.append(500);

    cprintf!(out, "List contents: ");
    for val in int_list.iter() {
        cprintf!(out, "{} ", val);
    }
    cprintf!(out, "\n");

    // Calculate product using LIST_FOREACH
    let mut product: i64 = 1;
    for val in int_list.iter() {
        product = product.wrapping_mul(*val as i64);
    }
    cprintf!(out, "Product: {}\n", product);
}

/// `void demo_double_containers(void)`
fn demo_double_containers(out: &mut dyn Write) {
    cprintf!(out, "\n");
    cprintf!(out, "========================================\n");
    cprintf!(out, "  DEMO 2: Double Containers\n");
    cprintf!(out, "========================================\n");

    // Create double array
    let mut double_array: Array<f64> = Array::create(5);
    cprintf!(out, "\n--- Double Array (Temperatures in Celsius) ---\n");

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    let num_temps: i32 = temps.len() as i32;

    cprintf!(out, "Adding temperatures: ");
    for i in 0..num_temps {
        double_array.push(temps[i as usize]);
        cprintf!(out, "{:.1} ", temps[i as usize]);
    }
    cprintf!(out, "\n");

    // Find min, max, average using ARRAY_FOREACH
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

    cprintf!(out, "Minimum: {:.1}\u{b0}C\n", min_temp);
    cprintf!(out, "Maximum: {:.1}\u{b0}C\n", max_temp);
    cprintf!(
        out,
        "Average: {:.1}\u{b0}C\n",
        sum_temp / (double_array.size() as f64)
    );

    // Create double list
    let mut price_list: List<f64> = List::create();
    cprintf!(out, "\n--- Double List (Product Prices) ---\n");

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    let num_prices: i32 = prices.len() as i32;

    cprintf!(out, "Adding prices: ");
    for i in 0..num_prices {
        price_list.append(prices[i as usize]);
        cprintf!(out, "${:.2} ", prices[i as usize]);
    }
    cprintf!(out, "\n");

    // Calculate total and find items under $10 using LIST_FOREACH
    let mut total: f64 = 0.0;
    let mut count_under_10: i32 = 0;

    for temp in price_list.iter() {
        total += *temp;
        if *temp < 10.0 {
            count_under_10 += 1;
        }
    }

    cprintf!(out, "Total cost: ${:.2}\n", total);
    cprintf!(out, "Items under $10: {}\n", count_under_10);
}

/// `void demo_inventory_array(void)`
fn demo_inventory_array(out: &mut dyn Write) {
    cprintf!(out, "\n");
    cprintf!(out, "========================================\n");
    cprintf!(out, "  DEMO 3: Inventory Array (Items)\n");
    cprintf!(out, "========================================\n");

    // Create inventory array
    let mut inventory: Array<ItemT> = Array::create(20);

    // Add items
    cprintf!(out, "\n--- Adding Items to Inventory ---\n");
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

    cprintf!(out, "Added {} items to inventory\n", inventory.size());

    // Display all items using ARRAY_FOREACH
    cprintf!(out, "\n--- All Inventory Items ---\n");
    for item in inventory.iter() {
        print_item(out, item);
        cprintf!(out, "\n");
    }

    // Calculate statistics
    calculate_inventory_stats(out, &inventory);

    // Find items by category
    find_items_by_category(out, &inventory, "Electronics");
    find_items_by_category(out, &inventory, "Furniture");

    // Find low stock items using ARRAY_FOREACH
    cprintf!(out, "\n--- Low Stock Items (< 20) ---\n");
    let mut low_stock_count: i32 = 0;
    for item in inventory.iter() {
        if item.quantity < 20 {
            print_item(out, item);
            low_stock_count += 1;
        }
    }
    cprintf!(out, "Total low stock items: {}\n", low_stock_count);
}

/// `void demo_order_list(void)`
fn demo_order_list(out: &mut dyn Write) {
    cprintf!(out, "\n");
    cprintf!(out, "========================================\n");
    cprintf!(out, "  DEMO 4: Order List (Orders)\n");
    cprintf!(out, "========================================\n");

    // Create order list
    let mut orders: List<OrderT> = List::create();

    // Add orders
    cprintf!(out, "\n--- Adding Orders ---\n");
    orders.append(create_order(1001, "Alice Johnson", 1249.95));
    orders.append(create_order(1002, "Bob Smith", 89.99));
    orders.append(create_order(1003, "Carol White", 549.98));
    orders.append(create_order(1004, "David Brown", 24.99));
    orders.append(create_order(1005, "Eve Davis", 899.99));
    orders.append(create_order(1006, "Frank Miller", 374.97));
    orders.append(create_order(1007, "Grace Lee", 159.98));
    orders.append(create_order(1008, "Henry Wilson", 1099.99));

    cprintf!(out, "Added {} orders\n", orders.size());

    // Display all orders using LIST_FOREACH
    cprintf!(out, "\n--- All Orders ---\n");
    for order in orders.iter() {
        print_order(out, order);
    }

    // Calculate statistics
    calculate_order_stats(out, &orders);

    // Find large orders using LIST_FOREACH
    cprintf!(out, "\n--- Large Orders (> $500) ---\n");
    let mut large_order_count: i32 = 0;
    let mut large_order_total: f64 = 0.0;

    for order in orders.iter() {
        if order.total_amount > 500.0 {
            print_order(out, order);
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    cprintf!(out, "Total large orders: {}\n", large_order_count);
    cprintf!(
        out,
        "Revenue from large orders: ${:.2}\n",
        large_order_total
    );
}

/// `void demo_mixed_operations(void)`
fn demo_mixed_operations(out: &mut dyn Write) {
    cprintf!(out, "\n");
    cprintf!(out, "========================================\n");
    cprintf!(out, "  DEMO 5: Mixed Operations\n");
    cprintf!(out, "========================================\n");

    // Create both array and list with items
    let mut array_inventory: Array<ItemT> = Array::create(10);
    let mut list_inventory: List<ItemT> = List::create();

    cprintf!(out, "\n--- Populating both Array and List ---\n");

    // Add same items to both
    let items: [ItemT; 5] = [
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55),
    ];

    let num_items: i32 = items.len() as i32;

    for i in 0..num_items {
        array_inventory.push(items[i as usize]);
        list_inventory.append(items[i as usize]);
    }

    cprintf!(out, "Added {} items to both containers\n", num_items);

    // Compare iteration performance (conceptually)
    cprintf!(out, "\n--- Iterating through Array ---\n");
    let mut array_count: i32 = 0;
    for _item in array_inventory.iter() {
        array_count += 1;
    }
    cprintf!(out, "Array iteration count: {}\n", array_count);

    cprintf!(out, "\n--- Iterating through List ---\n");
    let mut list_count: i32 = 0;
    for _item in list_inventory.iter() {
        list_count += 1;
    }
    cprintf!(out, "List iteration count: {}\n", list_count);

    // Find items above certain price in both
    let price_threshold: f64 = 200.0;

    cprintf!(
        out,
        "\n--- Items above ${:.2} (Array) ---\n",
        price_threshold
    );
    for item in array_inventory.iter() {
        if item.price >= price_threshold {
            cprintf!(
                out,
                "  {}: ${:.2}\n",
                String::from_utf8_lossy(c_str(&item.name)),
                item.price
            );
        }
    }

    cprintf!(
        out,
        "\n--- Items above ${:.2} (List) ---\n",
        price_threshold
    );
    for item in list_inventory.iter() {
        if item.price >= price_threshold {
            cprintf!(
                out,
                "  {}: ${:.2}\n",
                String::from_utf8_lossy(c_str(&item.name)),
                item.price
            );
        }
    }
}

/// Bytes of a NUL-terminated fixed buffer, as `printf("%s", ...)` sees them.
fn c_str(buf: &[u8]) -> &[u8] {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..end]
}

/// `fgets(input, size, stdin)`: reads at most `size - 1` bytes, stopping after a
/// newline (which is kept) or at EOF. Returns `None` for the `NULL` result,
/// i.e. when nothing at all could be read.
fn fgets(reader: &mut BufReader<Stdin>, size: usize) -> Option<Vec<u8>> {
    let max = size - 1;
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < max {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// `sscanf(input, "%d", &choice)`: `Some(value)` when the conversion succeeds
/// (return value 1), `None` for a matching failure or input failure.
///
/// Reproduces glibc's behavior of accumulating into a `long` (saturating at
/// `LONG_MIN`/`LONG_MAX`) and then storing the truncated `int`.
fn sscanf_d(input: &[u8]) -> Option<i32> {
    // A C string ends at the first NUL byte.
    let s = {
        let end = input.iter().position(|&b| b == 0).unwrap_or(input.len());
        &input[..end]
    };

    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let start_digits = i;
    let mut acc: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = (s[i] - b'0') as i64;
        acc = if negative {
            acc.saturating_mul(10).saturating_sub(digit)
        } else {
            acc.saturating_mul(10).saturating_add(digit)
        };
        i += 1;
    }

    if i == start_digits {
        // No digits: matching failure (0) or, if the input was exhausted while
        // skipping whitespace, an input failure (EOF). Neither equals 1.
        return None;
    }

    Some(acc as i32)
}

fn main() {
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let out: &mut dyn Write = &mut out;

    cprintf!(out, "\u{2554}{}\u{2557}\n", "\u{2550}".repeat(40));
    cprintf!(out, "\u{2551}   GENERIC FOR_EACH MACRO DEMO         \u{2551}\n");
    cprintf!(out, "\u{2551}   Demonstrating Generic Containers    \u{2551}\n");
    cprintf!(out, "\u{255a}{}\u{255d}\n", "\u{2550}".repeat(40));

    let mut reader = BufReader::new(std::io::stdin());

    loop {
        print_menu(out);

        // Show the prompt before blocking on input (glibc flushes line-buffered
        // stdout when reading from an interactive stdin).
        let _ = out.flush();

        let input = match fgets(&mut reader, 256) {
            Some(line) => line,
            None => break,
        };

        let choice = match sscanf_d(&input) {
            Some(value) => value,
            None => {
                cprintf!(out, "Invalid input\n");
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
                cprintf!(out, "\n=== Running All Demos ===\n");
                demo_integer_containers(out);
                demo_double_containers(out);
                demo_inventory_array(out);
                demo_order_list(out);
                demo_mixed_operations(out);
                cprintf!(out, "\n========================================\n");
                cprintf!(out, "  All demos completed successfully!\n");
                cprintf!(out, "========================================\n");
            }

            7 => {
                cprintf!(out, "\nGoodbye!\n");
                let _ = out.flush();
                std::process::exit(0);
            }

            _ => {
                cprintf!(out, "Invalid choice\n");
            }
        }
    }

    let _ = out.flush();
    std::process::exit(0);
}
