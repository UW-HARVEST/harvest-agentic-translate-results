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
//! demos.rs -- translation of main.c

#![allow(dead_code)]

use std::io::Write;

use libc::{c_char, c_int};

use crate::cio::{c_str_ptr, fadd_c, fgets, fmt_f, sscanf_d, w, CStdout};
use crate::generic_containers::{
    array_clear, array_create, array_destroy, array_foreach, array_get, array_push, array_size,
    list_append, list_clear, list_create, list_destroy, list_foreach, list_prepend, list_size,
    ArrayT, ListT,
};
use crate::inventory::{
    calculate_inventory_stats, calculate_order_stats, create_item, create_order,
    find_items_by_category, print_item, print_order, ItemT, OrderT,
};

/// Pointer to a NUL-terminated string literal, i.e. a C string literal.
macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

/// `void print_menu(void)`
pub fn print_menu(out: &mut dyn Write) {
    w(out, b"\n");
    w(out, b"========================================\n");
    w(out, b"  GENERIC FOR_EACH MACRO DEMO\n");
    w(out, b"========================================\n");
    w(out, b"1. Demo: Integer Containers\n");
    w(out, b"2. Demo: Double Containers\n");
    w(out, b"3. Demo: Inventory Array\n");
    w(out, b"4. Demo: Order List\n");
    w(out, b"5. Demo: Mixed Operations\n");
    w(out, b"6. Run All Demos\n");
    w(out, b"7. Exit\n");
    w(out, b"========================================\n");
    w(out, b"Choice: ");
}

/// `void demo_integer_containers(void)`
///
/// # Safety
/// Uses the raw container API, exactly as the C function does.
pub unsafe fn demo_integer_containers(out: &mut dyn Write) {
    w(out, b"\n");
    w(out, b"========================================\n");
    w(out, b"  DEMO 1: Integer Containers\n");
    w(out, b"========================================\n");

    // Create integer array
    let int_array: *mut ArrayT<c_int> = array_create(10);
    w(out, b"\n--- Integer Array ---\n");
    w(out, b"Adding integers: 10, 20, 30, 40, 50\n");
    array_push(int_array, 10);
    array_push(int_array, 20);
    array_push(int_array, 30);
    array_push(int_array, 40);
    array_push(int_array, 50);

    w(out, b"Array contents: ");
    array_foreach(int_array, |val: c_int| {
        cprintf!(out, "{} ", val);
    });
    w(out, b"\n");

    // Calculate sum using ARRAY_FOREACH
    let mut sum: c_int = 0;
    array_foreach(int_array, |val: c_int| {
        sum = sum.wrapping_add(val);
    });
    cprintf!(out, "Sum: {}\n", sum);
    cprintf!(
        out,
        "Average: {}\n",
        fmt_f((sum as f64) / ((*int_array).size as f64), 2)
    );

    // Create integer list
    let int_list: *mut ListT<c_int> = list_create();
    w(out, b"\n--- Integer List ---\n");
    w(out, b"Adding integers: 100, 200, 300, 400, 500\n");
    list_append(int_list, 100);
    list_append(int_list, 200);
    list_append(int_list, 300);
    list_append(int_list, 400);
    list_append(int_list, 500);

    w(out, b"List contents: ");
    list_foreach(int_list, |val: c_int| {
        cprintf!(out, "{} ", val);
    });
    w(out, b"\n");

    // Calculate product using LIST_FOREACH
    let mut product: i64 = 1;
    list_foreach(int_list, |val: c_int| {
        product = product.wrapping_mul(val as i64);
    });
    cprintf!(out, "Product: {}\n", product);

    // Cleanup
    array_destroy(int_array);
    list_destroy(int_list);
}

/// `void demo_double_containers(void)`
///
/// # Safety
/// Uses the raw container API, exactly as the C function does.
pub unsafe fn demo_double_containers(out: &mut dyn Write) {
    w(out, b"\n");
    w(out, b"========================================\n");
    w(out, b"  DEMO 2: Double Containers\n");
    w(out, b"========================================\n");

    // Create double array
    let double_array: *mut ArrayT<f64> = array_create(5);
    w(out, b"\n--- Double Array (Temperatures in Celsius) ---\n");

    let temps: [f64; 7] = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    let num_temps: c_int = temps.len() as c_int;

    w(out, b"Adding temperatures: ");
    let mut i: c_int = 0;
    while i < num_temps {
        array_push(double_array, temps[i as usize]);
        cprintf!(out, "{} ", fmt_f(temps[i as usize], 1));
        i += 1;
    }
    w(out, b"\n");

    // Find min, max, average using ARRAY_FOREACH
    let mut min_temp: f64 = temps[0];
    let mut max_temp: f64 = temps[0];
    let mut sum_temp: f64 = 0.0;

    array_foreach(double_array, |temp: f64| {
        if temp < min_temp {
            min_temp = temp;
        }
        if temp > max_temp {
            max_temp = temp;
        }
        sum_temp = fadd_c(sum_temp, temp);
    });

    cprintf!(out, "Minimum: {}\u{b0}C\n", fmt_f(min_temp, 1));
    cprintf!(out, "Maximum: {}\u{b0}C\n", fmt_f(max_temp, 1));
    cprintf!(
        out,
        "Average: {}\u{b0}C\n",
        fmt_f(sum_temp / ((*double_array).size as f64), 1)
    );

    // Create double list
    let price_list: *mut ListT<f64> = list_create();
    w(out, b"\n--- Double List (Product Prices) ---\n");

    let prices: [f64; 6] = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    let num_prices: c_int = prices.len() as c_int;

    w(out, b"Adding prices: ");
    let mut i: c_int = 0;
    while i < num_prices {
        list_append(price_list, prices[i as usize]);
        cprintf!(out, "${} ", fmt_f(prices[i as usize], 2));
        i += 1;
    }
    w(out, b"\n");

    // Calculate total and find items under $10 using LIST_FOREACH
    let mut total: f64 = 0.0;
    let mut count_under_10: c_int = 0;

    list_foreach(price_list, |temp: f64| {
        total = fadd_c(total, temp);
        if temp < 10.0 {
            count_under_10 += 1;
        }
    });

    cprintf!(out, "Total cost: ${}\n", fmt_f(total, 2));
    cprintf!(out, "Items under $10: {}\n", count_under_10);

    // Cleanup
    array_destroy(double_array);
    list_destroy(price_list);
}

/// `void demo_inventory_array(void)`
///
/// # Safety
/// Uses the raw container API, exactly as the C function does.
pub unsafe fn demo_inventory_array(out: &mut dyn Write) {
    w(out, b"\n");
    w(out, b"========================================\n");
    w(out, b"  DEMO 3: Inventory Array (Items)\n");
    w(out, b"========================================\n");

    // Create inventory array
    let inventory: *mut ArrayT<ItemT> = array_create(20);

    // Add items
    w(out, b"\n--- Adding Items to Inventory ---\n");
    array_push(
        inventory,
        create_item(1, cstr!("Laptop"), cstr!("Electronics"), 899.99, 15),
    );
    array_push(
        inventory,
        create_item(2, cstr!("Mouse"), cstr!("Electronics"), 24.99, 50),
    );
    array_push(
        inventory,
        create_item(3, cstr!("Keyboard"), cstr!("Electronics"), 79.99, 30),
    );
    array_push(
        inventory,
        create_item(4, cstr!("Monitor"), cstr!("Electronics"), 299.99, 20),
    );
    array_push(
        inventory,
        create_item(5, cstr!("Desk Chair"), cstr!("Furniture"), 199.99, 10),
    );
    array_push(
        inventory,
        create_item(6, cstr!("Desk"), cstr!("Furniture"), 349.99, 8),
    );
    array_push(
        inventory,
        create_item(7, cstr!("Notebook"), cstr!("Office"), 4.99, 100),
    );
    array_push(
        inventory,
        create_item(8, cstr!("Pen Set"), cstr!("Office"), 12.99, 75),
    );
    array_push(
        inventory,
        create_item(9, cstr!("USB Cable"), cstr!("Electronics"), 9.99, 60),
    );
    array_push(
        inventory,
        create_item(10, cstr!("Bookshelf"), cstr!("Furniture"), 149.99, 12),
    );

    cprintf!(out, "Added {} items to inventory\n", (*inventory).size);

    // Display all items using ARRAY_FOREACH
    w(out, b"\n--- All Inventory Items ---\n");
    array_foreach(inventory, |item: ItemT| {
        print_item(out, &item);
        w(out, b"\n");
    });

    // Calculate statistics
    calculate_inventory_stats(out, inventory);

    // Find items by category
    find_items_by_category(out, inventory, cstr!("Electronics"));
    find_items_by_category(out, inventory, cstr!("Furniture"));

    // Find low stock items using ARRAY_FOREACH
    w(out, b"\n--- Low Stock Items (< 20) ---\n");
    let mut low_stock_count: c_int = 0;
    array_foreach(inventory, |item: ItemT| {
        if item.quantity < 20 {
            print_item(out, &item);
            low_stock_count += 1;
        }
    });
    cprintf!(out, "Total low stock items: {}\n", low_stock_count);

    // Cleanup
    array_destroy(inventory);
}

/// `void demo_order_list(void)`
///
/// # Safety
/// Uses the raw container API, exactly as the C function does.
pub unsafe fn demo_order_list(out: &mut dyn Write) {
    w(out, b"\n");
    w(out, b"========================================\n");
    w(out, b"  DEMO 4: Order List (Orders)\n");
    w(out, b"========================================\n");

    // Create order list
    let orders: *mut ListT<OrderT> = list_create();

    // Add orders
    w(out, b"\n--- Adding Orders ---\n");
    list_append(orders, create_order(1001, cstr!("Alice Johnson"), 1249.95));
    list_append(orders, create_order(1002, cstr!("Bob Smith"), 89.99));
    list_append(orders, create_order(1003, cstr!("Carol White"), 549.98));
    list_append(orders, create_order(1004, cstr!("David Brown"), 24.99));
    list_append(orders, create_order(1005, cstr!("Eve Davis"), 899.99));
    list_append(orders, create_order(1006, cstr!("Frank Miller"), 374.97));
    list_append(orders, create_order(1007, cstr!("Grace Lee"), 159.98));
    list_append(orders, create_order(1008, cstr!("Henry Wilson"), 1099.99));

    cprintf!(out, "Added {} orders\n", (*orders).size);

    // Display all orders using LIST_FOREACH
    w(out, b"\n--- All Orders ---\n");
    list_foreach(orders, |order: OrderT| {
        print_order(out, &order);
    });

    // Calculate statistics
    calculate_order_stats(out, orders);

    // Find large orders using LIST_FOREACH
    w(out, b"\n--- Large Orders (> $500) ---\n");
    let mut large_order_count: c_int = 0;
    let mut large_order_total: f64 = 0.0;

    list_foreach(orders, |order: OrderT| {
        if order.total_amount > 500.0 {
            print_order(out, &order);
            large_order_count += 1;
            large_order_total = fadd_c(large_order_total, order.total_amount);
        }
    });

    cprintf!(out, "Total large orders: {}\n", large_order_count);
    cprintf!(
        out,
        "Revenue from large orders: ${}\n",
        fmt_f(large_order_total, 2)
    );

    // Cleanup
    list_destroy(orders);
}

/// `void demo_mixed_operations(void)`
///
/// # Safety
/// Uses the raw container API, exactly as the C function does.
pub unsafe fn demo_mixed_operations(out: &mut dyn Write) {
    w(out, b"\n");
    w(out, b"========================================\n");
    w(out, b"  DEMO 5: Mixed Operations\n");
    w(out, b"========================================\n");

    // Create both array and list with items
    let array_inventory: *mut ArrayT<ItemT> = array_create(10);
    let list_inventory: *mut ListT<ItemT> = list_create();

    w(out, b"\n--- Populating both Array and List ---\n");

    // Add same items to both
    let items: [ItemT; 5] = [
        create_item(1, cstr!("Smartphone"), cstr!("Electronics"), 699.99, 25),
        create_item(2, cstr!("Tablet"), cstr!("Electronics"), 449.99, 18),
        create_item(3, cstr!("Headphones"), cstr!("Electronics"), 149.99, 40),
        create_item(4, cstr!("Smart Watch"), cstr!("Electronics"), 299.99, 22),
        create_item(5, cstr!("Power Bank"), cstr!("Electronics"), 39.99, 55),
    ];

    let num_items: c_int = items.len() as c_int;

    let mut i: c_int = 0;
    while i < num_items {
        array_push(array_inventory, items[i as usize]);
        list_append(list_inventory, items[i as usize]);
        i += 1;
    }

    cprintf!(out, "Added {} items to both containers\n", num_items);

    // Compare iteration performance (conceptually)
    w(out, b"\n--- Iterating through Array ---\n");
    let mut array_count: c_int = 0;
    array_foreach(array_inventory, |_item: ItemT| {
        array_count += 1;
    });
    cprintf!(out, "Array iteration count: {}\n", array_count);

    w(out, b"\n--- Iterating through List ---\n");
    let mut list_count: c_int = 0;
    list_foreach(list_inventory, |_item: ItemT| {
        list_count += 1;
    });
    cprintf!(out, "List iteration count: {}\n", list_count);

    // Find items above certain price in both
    let price_threshold: f64 = 200.0;

    cprintf!(
        out,
        "\n--- Items above ${} (Array) ---\n",
        fmt_f(price_threshold, 2)
    );
    array_foreach(array_inventory, |item: ItemT| {
        if item.price >= price_threshold {
            w(out, b"  ");
            // printf("  %s: $%.2f\n", item.name, item.price)
            w(out, c_str_ptr(item.name.as_ptr() as *const c_char));
            cprintf!(out, ": ${}\n", fmt_f(item.price, 2));
        }
    });

    cprintf!(
        out,
        "\n--- Items above ${} (List) ---\n",
        fmt_f(price_threshold, 2)
    );
    list_foreach(list_inventory, |item: ItemT| {
        if item.price >= price_threshold {
            w(out, b"  ");
            // printf("  %s: $%.2f\n", item.name, item.price)
            w(out, c_str_ptr(item.name.as_ptr() as *const c_char));
            cprintf!(out, ": ${}\n", fmt_f(item.price, 2));
        }
    });

    // Cleanup
    array_destroy(array_inventory);
    list_destroy(list_inventory);
}

/// `int main(void)`
///
/// # Safety
/// Drives the raw container API and reads file descriptor 0, like C's `main`.
pub unsafe fn c_main() -> c_int {
    let mut stdout = CStdout;
    let out: &mut dyn Write = &mut stdout;

    w(
        out,
        "\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}\n".as_bytes(),
    );
    w(
        out,
        "\u{2551}   GENERIC FOR_EACH MACRO DEMO         \u{2551}\n".as_bytes(),
    );
    w(
        out,
        "\u{2551}   Demonstrating Generic Containers    \u{2551}\n".as_bytes(),
    );
    w(
        out,
        "\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}\n".as_bytes(),
    );

    loop {
        print_menu(out);

        // char input[256]; if (!fgets(input, sizeof(input), stdin)) break;
        let input = match fgets(256) {
            Some(line) => line,
            None => break,
        };

        let choice = match sscanf_d(&input) {
            Some(value) => value,
            None => {
                w(out, b"Invalid input\n");
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
                w(out, b"\n=== Running All Demos ===\n");
                demo_integer_containers(out);
                demo_double_containers(out);
                demo_inventory_array(out);
                demo_order_list(out);
                demo_mixed_operations(out);
                w(out, b"\n========================================\n");
                w(out, b"  All demos completed successfully!\n");
                w(out, b"========================================\n");
            }

            7 => {
                w(out, b"\nGoodbye!\n");
                return 0;
            }

            _ => {
                w(out, b"Invalid choice\n");
            }
        }
    }

    0
}

// Referenced so that the full container surface stays exercised by the crate
// even where `main.c` itself never calls a particular generated function.
#[allow(unused)]
unsafe fn _unused_container_api() {
    let a: *mut ArrayT<c_int> = array_create(1);
    let _ = array_get(a, 0);
    let _ = array_size(a);
    array_clear(a);
    array_destroy(a);
    let l: *mut ListT<c_int> = list_create();
    let _ = list_prepend(l, 1);
    let _ = list_size(l);
    list_clear(l);
    list_destroy(l);
}
