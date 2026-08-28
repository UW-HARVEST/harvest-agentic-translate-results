use std::io::{self, BufRead, Write};

#[derive(Clone, Copy)]
struct Item {
    id: i32,
    name: &'static str,
    category: &'static str,
    price: f64,
    quantity: i32,
}

#[derive(Clone, Copy)]
struct Order {
    customer_id: i32,
    customer_name: &'static str,
    total_amount: f64,
}

fn print_menu(out: &mut impl Write) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "========================================")?;
    writeln!(out, "  GENERIC FOR_EACH MACRO DEMO")?;
    writeln!(out, "========================================")?;
    writeln!(out, "1. Demo: Integer Containers")?;
    writeln!(out, "2. Demo: Double Containers")?;
    writeln!(out, "3. Demo: Inventory Array")?;
    writeln!(out, "4. Demo: Order List")?;
    writeln!(out, "5. Demo: Mixed Operations")?;
    writeln!(out, "6. Run All Demos")?;
    writeln!(out, "7. Exit")?;
    writeln!(out, "========================================")?;
    write!(out, "Choice: ")
}

fn print_item(out: &mut impl Write, item: Item) -> io::Result<()> {
    writeln!(out, "  [{}] {}", item.id, item.name)?;
    writeln!(out, "      Category: {}", item.category)?;
    writeln!(out, "      Price: ${:.2}", item.price)?;
    writeln!(out, "      Quantity: {}", item.quantity)
}

fn print_order(out: &mut impl Write, order: Order) -> io::Result<()> {
    writeln!(
        out,
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id, order.customer_name
    )?;
    writeln!(out, "          Total: ${:.2}", order.total_amount)
}

fn calculate_inventory_stats(out: &mut impl Write, items: &[Item]) -> io::Result<()> {
    if items.is_empty() {
        writeln!(out, "No items in inventory")?;
        return Ok(());
    }

    writeln!(out, "\n=== Inventory Statistics (Array) ===")?;

    let mut total_value = 0.0;
    let mut total_items = 0;
    let mut max_price = 0.0;
    let mut min_price = items[0].price;

    for item in items {
        total_value += item.price * f64::from(item.quantity);
        total_items += item.quantity;
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    writeln!(out, "Total unique items: {}", items.len())?;
    writeln!(out, "Total item count: {}", total_items)?;
    writeln!(out, "Total inventory value: ${:.2}", total_value)?;
    writeln!(
        out,
        "Average item price: ${:.2}",
        total_value / f64::from(total_items)
    )?;
    writeln!(out, "Most expensive item: ${:.2}", max_price)?;
    writeln!(out, "Least expensive item: ${:.2}", min_price)
}

fn calculate_order_stats(out: &mut impl Write, orders: &[Order]) -> io::Result<()> {
    if orders.is_empty() {
        writeln!(out, "No orders to analyze")?;
        return Ok(());
    }

    writeln!(out, "\n=== Order Statistics (List) ===")?;

    let mut total_revenue = 0.0;
    let mut max_order = 0.0;
    let mut min_order = -1.0;

    for order in orders {
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
    }

    writeln!(out, "Total orders: {}", orders.len())?;
    writeln!(out, "Total revenue: ${:.2}", total_revenue)?;
    writeln!(
        out,
        "Average order value: ${:.2}",
        total_revenue / orders.len() as f64
    )?;
    writeln!(out, "Largest order: ${:.2}", max_order)?;
    writeln!(out, "Smallest order: ${:.2}", min_order)
}

fn find_items_by_category(out: &mut impl Write, items: &[Item], category: &str) -> io::Result<()> {
    writeln!(out, "\n=== Items in category '{}' ===", category)?;

    let mut found = 0;
    for item in items {
        if item.category == category {
            print_item(out, *item)?;
            found += 1;
        }
    }

    if found == 0 {
        writeln!(out, "No items found in this category")
    } else {
        writeln!(out, "Found {} items", found)
    }
}

fn demo_integer_containers(out: &mut impl Write) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "========================================")?;
    writeln!(out, "  DEMO 1: Integer Containers")?;
    writeln!(out, "========================================")?;

    let int_array = vec![10, 20, 30, 40, 50];
    writeln!(out, "\n--- Integer Array ---")?;
    writeln!(out, "Adding integers: 10, 20, 30, 40, 50")?;

    write!(out, "Array contents: ")?;
    for val in &int_array {
        write!(out, "{} ", val)?;
    }
    writeln!(out)?;

    let mut sum = 0;
    for val in &int_array {
        sum += val;
    }
    writeln!(out, "Sum: {}", sum)?;
    writeln!(
        out,
        "Average: {:.2}",
        f64::from(sum) / int_array.len() as f64
    )?;

    let int_list = vec![100_i32, 200, 300, 400, 500];
    writeln!(out, "\n--- Integer List ---")?;
    writeln!(out, "Adding integers: 100, 200, 300, 400, 500")?;

    write!(out, "List contents: ")?;
    for val in &int_list {
        write!(out, "{} ", val)?;
    }
    writeln!(out)?;

    let mut product = 1_i64;
    for val in &int_list {
        product *= i64::from(*val);
    }
    writeln!(out, "Product: {}", product)
}

fn demo_double_containers(out: &mut impl Write) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "========================================")?;
    writeln!(out, "  DEMO 2: Double Containers")?;
    writeln!(out, "========================================")?;

    let temps = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    let mut double_array = Vec::with_capacity(5);
    writeln!(out, "\n--- Double Array (Temperatures in Celsius) ---")?;

    write!(out, "Adding temperatures: ")?;
    for temp in temps {
        double_array.push(temp);
        write!(out, "{:.1} ", temp)?;
    }
    writeln!(out)?;

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
        sum_temp += temp;
    }

    writeln!(out, "Minimum: {:.1}°C", min_temp)?;
    writeln!(out, "Maximum: {:.1}°C", max_temp)?;
    writeln!(
        out,
        "Average: {:.1}°C",
        sum_temp / double_array.len() as f64
    )?;

    let prices = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    let mut price_list = Vec::new();
    writeln!(out, "\n--- Double List (Product Prices) ---")?;

    write!(out, "Adding prices: ")?;
    for price in prices {
        price_list.push(price);
        write!(out, "${:.2} ", price)?;
    }
    writeln!(out)?;

    let mut total = 0.0;
    let mut count_under_10 = 0;
    for price in &price_list {
        total += price;
        if *price < 10.0 {
            count_under_10 += 1;
        }
    }

    writeln!(out, "Total cost: ${:.2}", total)?;
    writeln!(out, "Items under $10: {}", count_under_10)
}

fn inventory_items() -> Vec<Item> {
    vec![
        Item {
            id: 1,
            name: "Laptop",
            category: "Electronics",
            price: 899.99,
            quantity: 15,
        },
        Item {
            id: 2,
            name: "Mouse",
            category: "Electronics",
            price: 24.99,
            quantity: 50,
        },
        Item {
            id: 3,
            name: "Keyboard",
            category: "Electronics",
            price: 79.99,
            quantity: 30,
        },
        Item {
            id: 4,
            name: "Monitor",
            category: "Electronics",
            price: 299.99,
            quantity: 20,
        },
        Item {
            id: 5,
            name: "Desk Chair",
            category: "Furniture",
            price: 199.99,
            quantity: 10,
        },
        Item {
            id: 6,
            name: "Desk",
            category: "Furniture",
            price: 349.99,
            quantity: 8,
        },
        Item {
            id: 7,
            name: "Notebook",
            category: "Office",
            price: 4.99,
            quantity: 100,
        },
        Item {
            id: 8,
            name: "Pen Set",
            category: "Office",
            price: 12.99,
            quantity: 75,
        },
        Item {
            id: 9,
            name: "USB Cable",
            category: "Electronics",
            price: 9.99,
            quantity: 60,
        },
        Item {
            id: 10,
            name: "Bookshelf",
            category: "Furniture",
            price: 149.99,
            quantity: 12,
        },
    ]
}

fn demo_inventory_array(out: &mut impl Write) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "========================================")?;
    writeln!(out, "  DEMO 3: Inventory Array (Items)")?;
    writeln!(out, "========================================")?;

    writeln!(out, "\n--- Adding Items to Inventory ---")?;
    let inventory = inventory_items();
    writeln!(out, "Added {} items to inventory", inventory.len())?;

    writeln!(out, "\n--- All Inventory Items ---")?;
    for item in &inventory {
        print_item(out, *item)?;
        writeln!(out)?;
    }

    calculate_inventory_stats(out, &inventory)?;
    find_items_by_category(out, &inventory, "Electronics")?;
    find_items_by_category(out, &inventory, "Furniture")?;

    writeln!(out, "\n--- Low Stock Items (< 20) ---")?;
    let mut low_stock_count = 0;
    for item in &inventory {
        if item.quantity < 20 {
            print_item(out, *item)?;
            low_stock_count += 1;
        }
    }
    writeln!(out, "Total low stock items: {}", low_stock_count)
}

fn orders() -> Vec<Order> {
    vec![
        Order {
            customer_id: 1001,
            customer_name: "Alice Johnson",
            total_amount: 1249.95,
        },
        Order {
            customer_id: 1002,
            customer_name: "Bob Smith",
            total_amount: 89.99,
        },
        Order {
            customer_id: 1003,
            customer_name: "Carol White",
            total_amount: 549.98,
        },
        Order {
            customer_id: 1004,
            customer_name: "David Brown",
            total_amount: 24.99,
        },
        Order {
            customer_id: 1005,
            customer_name: "Eve Davis",
            total_amount: 899.99,
        },
        Order {
            customer_id: 1006,
            customer_name: "Frank Miller",
            total_amount: 374.97,
        },
        Order {
            customer_id: 1007,
            customer_name: "Grace Lee",
            total_amount: 159.98,
        },
        Order {
            customer_id: 1008,
            customer_name: "Henry Wilson",
            total_amount: 1099.99,
        },
    ]
}

fn demo_order_list(out: &mut impl Write) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "========================================")?;
    writeln!(out, "  DEMO 4: Order List (Orders)")?;
    writeln!(out, "========================================")?;

    writeln!(out, "\n--- Adding Orders ---")?;
    let orders = orders();
    writeln!(out, "Added {} orders", orders.len())?;

    writeln!(out, "\n--- All Orders ---")?;
    for order in &orders {
        print_order(out, *order)?;
    }

    calculate_order_stats(out, &orders)?;

    writeln!(out, "\n--- Large Orders (> $500) ---")?;
    let mut large_order_count = 0;
    let mut large_order_total = 0.0;
    for order in &orders {
        if order.total_amount > 500.0 {
            print_order(out, *order)?;
            large_order_count += 1;
            large_order_total += order.total_amount;
        }
    }

    writeln!(out, "Total large orders: {}", large_order_count)?;
    writeln!(out, "Revenue from large orders: ${:.2}", large_order_total)
}

fn mixed_items() -> Vec<Item> {
    vec![
        Item {
            id: 1,
            name: "Smartphone",
            category: "Electronics",
            price: 699.99,
            quantity: 25,
        },
        Item {
            id: 2,
            name: "Tablet",
            category: "Electronics",
            price: 449.99,
            quantity: 18,
        },
        Item {
            id: 3,
            name: "Headphones",
            category: "Electronics",
            price: 149.99,
            quantity: 40,
        },
        Item {
            id: 4,
            name: "Smart Watch",
            category: "Electronics",
            price: 299.99,
            quantity: 22,
        },
        Item {
            id: 5,
            name: "Power Bank",
            category: "Electronics",
            price: 39.99,
            quantity: 55,
        },
    ]
}

fn demo_mixed_operations(out: &mut impl Write) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "========================================")?;
    writeln!(out, "  DEMO 5: Mixed Operations")?;
    writeln!(out, "========================================")?;

    writeln!(out, "\n--- Populating both Array and List ---")?;
    let array_inventory = mixed_items();
    let list_inventory = array_inventory.clone();
    writeln!(
        out,
        "Added {} items to both containers",
        array_inventory.len()
    )?;

    writeln!(out, "\n--- Iterating through Array ---")?;
    let mut array_count = 0;
    for _ in &array_inventory {
        array_count += 1;
    }
    writeln!(out, "Array iteration count: {}", array_count)?;

    writeln!(out, "\n--- Iterating through List ---")?;
    let mut list_count = 0;
    for _ in &list_inventory {
        list_count += 1;
    }
    writeln!(out, "List iteration count: {}", list_count)?;

    let price_threshold = 200.0;
    writeln!(out, "\n--- Items above ${:.2} (Array) ---", price_threshold)?;
    for item in &array_inventory {
        if item.price >= price_threshold {
            writeln!(out, "  {}: ${:.2}", item.name, item.price)?;
        }
    }

    writeln!(out, "\n--- Items above ${:.2} (List) ---", price_threshold)?;
    for item in &list_inventory {
        if item.price >= price_threshold {
            writeln!(out, "  {}: ${:.2}", item.name, item.price)?;
        }
    }

    Ok(())
}

// fgets(input, 256, stdin) returns at most 255 bytes and leaves a longer
// physical line for the next call.
fn fgets_256(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut result = Vec::with_capacity(255);

    while result.len() < 255 {
        let available = input.fill_buf()?;
        if available.is_empty() {
            return if result.is_empty() {
                Ok(None)
            } else {
                Ok(Some(result))
            };
        }

        let room = 255 - result.len();
        let available = &available[..available.len().min(room)];
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        let found_newline = available[take - 1] == b'\n';
        result.extend_from_slice(&available[..take]);
        input.consume(take);

        if found_newline {
            break;
        }
    }

    Ok(Some(result))
}

fn sscanf_decimal_i32(input: &[u8]) -> Option<i32> {
    let nul = input
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(input.len());
    let input = &input[..nul];
    let mut index = 0;

    while index < input.len() && matches!(input[index], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    {
        index += 1;
    }

    let mut negative = false;
    if index < input.len() && (input[index] == b'+' || input[index] == b'-') {
        negative = input[index] == b'-';
        index += 1;
    }

    let digit_start = index;
    let mut magnitude = 0_u64;
    while index < input.len() && input[index].is_ascii_digit() {
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add(u64::from(input[index] - b'0'));
        index += 1;
    }
    if index == digit_start {
        return None;
    }

    let signed = if negative {
        const I64_MIN_MAGNITUDE: u64 = 1_u64 << 63;
        if magnitude >= I64_MIN_MAGNITUDE {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else {
        magnitude.min(i64::MAX as u64) as i64
    };
    Some(signed as i32)
}

fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut out = stdout.lock();

    writeln!(out, "╔════════════════════════════════════════╗")?;
    writeln!(out, "║   GENERIC FOR_EACH MACRO DEMO         ║")?;
    writeln!(out, "║   Demonstrating Generic Containers    ║")?;
    writeln!(out, "╚════════════════════════════════════════╝")?;

    loop {
        print_menu(&mut out)?;
        out.flush()?;

        let Some(line) = fgets_256(&mut input)? else {
            break;
        };

        let Some(choice) = sscanf_decimal_i32(&line) else {
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
                writeln!(out, "\n========================================")?;
                writeln!(out, "  All demos completed successfully!")?;
                writeln!(out, "========================================")?;
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
    if let Err(error) = run() {
        if error.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("{error}");
        }
    }
}
