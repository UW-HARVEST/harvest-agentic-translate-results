use std::io::{self, Write};

#[derive(Clone, Copy)]
pub struct Item {
    pub id: i32,
    pub name: &'static str,
    pub category: &'static str,
    pub price: f64,
    pub quantity: i32,
}

#[derive(Clone, Copy)]
pub struct Order {
    pub customer_id: i32,
    pub customer_name: &'static str,
    pub total_amount: f64,
}

pub fn create_item(
    id: i32,
    name: &'static str,
    category: &'static str,
    price: f64,
    quantity: i32,
) -> Item {
    Item {
        id,
        name,
        category,
        price,
        quantity,
    }
}

pub fn create_order(
    customer_id: i32,
    customer_name: &'static str,
    total_amount: f64,
) -> Order {
    Order {
        customer_id,
        customer_name,
        total_amount,
    }
}

pub fn print_item<W: Write>(out: &mut W, item: Item) -> io::Result<()> {
    writeln!(out, "  [{}] {}", item.id, item.name)?;
    writeln!(out, "      Category: {}", item.category)?;
    writeln!(out, "      Price: ${:.2}", item.price)?;
    writeln!(out, "      Quantity: {}", item.quantity)
}

pub fn print_order<W: Write>(out: &mut W, order: Order) -> io::Result<()> {
    writeln!(
        out,
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id, order.customer_name
    )?;
    writeln!(out, "          Total: ${:.2}", order.total_amount)
}

pub fn calculate_inventory_stats<W: Write>(out: &mut W, items: &[Item]) -> io::Result<()> {
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
    writeln!(out, "Total item count: {total_items}")?;
    writeln!(out, "Total inventory value: ${total_value:.2}")?;
    writeln!(
        out,
        "Average item price: ${:.2}",
        total_value / f64::from(total_items)
    )?;
    writeln!(out, "Most expensive item: ${max_price:.2}")?;
    writeln!(out, "Least expensive item: ${min_price:.2}")
}

pub fn calculate_order_stats<W: Write>(out: &mut W, orders: &[Order]) -> io::Result<()> {
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
    writeln!(out, "Total revenue: ${total_revenue:.2}")?;
    writeln!(
        out,
        "Average order value: ${:.2}",
        total_revenue / orders.len() as f64
    )?;
    writeln!(out, "Largest order: ${max_order:.2}")?;
    writeln!(out, "Smallest order: ${min_order:.2}")
}

pub fn find_items_by_category<W: Write>(
    out: &mut W,
    items: &[Item],
    category: &str,
) -> io::Result<()> {
    writeln!(out, "\n=== Items in category '{category}' ===")?;

    let mut found = 0;
    for &item in items {
        if item.category == category {
            print_item(out, item)?;
            found += 1;
        }
    }

    if found == 0 {
        writeln!(out, "No items found in this category")
    } else {
        writeln!(out, "Found {found} items")
    }
}
