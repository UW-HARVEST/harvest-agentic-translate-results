// inventory.rs - Translation of inventory.c / inventory.h
// Provides item_t, order_t types and inventory management functions.

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

#[derive(Clone, Copy)]
pub struct Item {
    pub id: i32,
    pub name: [u8; MAX_NAME_LENGTH],
    pub category: [u8; MAX_CATEGORY_LENGTH],
    pub price: f64,
    pub quantity: i32,
}

#[derive(Clone, Copy)]
pub struct Order {
    pub customer_id: i32,
    pub customer_name: [u8; MAX_NAME_LENGTH],
    pub total_amount: f64,
}

// ============================================================================
// Helper functions: emulate strncpy / null-terminated buffer printing
// ============================================================================

fn copy_str_to_buf(dst: &mut [u8], src: &str) {
    // emulate strncpy + explicit null termination of last byte
    let bytes = src.as_bytes();
    let n = dst.len();
    // zero out dst first to be safe
    for b in dst.iter_mut() {
        *b = 0;
    }
    let copy_len = if bytes.len() > n - 1 { n - 1 } else { bytes.len() };
    dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
    dst[n - 1] = 0;
}

pub fn buf_to_str(buf: &[u8]) -> &str {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    // Safe because input came from valid UTF-8 strings via copy_str_to_buf.
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}

pub fn buf_eq_str(buf: &[u8], s: &str) -> bool {
    buf_to_str(buf) == s
}

// ============================================================================
// Constructors / printers
// ============================================================================

pub fn create_item(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Item {
    let mut item = Item {
        id,
        name: [0u8; MAX_NAME_LENGTH],
        category: [0u8; MAX_CATEGORY_LENGTH],
        price,
        quantity,
    };
    copy_str_to_buf(&mut item.name, name);
    copy_str_to_buf(&mut item.category, category);
    item
}

pub fn create_order(customer_id: i32, customer_name: &str, total_amount: f64) -> Order {
    let mut order = Order {
        customer_id,
        customer_name: [0u8; MAX_NAME_LENGTH],
        total_amount,
    };
    copy_str_to_buf(&mut order.customer_name, customer_name);
    order
}

pub fn print_item(item: &Item) {
    println!("  [{}] {}", item.id, buf_to_str(&item.name));
    println!("      Category: {}", buf_to_str(&item.category));
    println!("      Price: ${:.2}", item.price);
    println!("      Quantity: {}", item.quantity);
}

pub fn print_order(order: &Order) {
    println!(
        "  Order - Customer ID: {}, Name: {}",
        order.customer_id,
        buf_to_str(&order.customer_name)
    );
    println!("          Total: ${:.2}", order.total_amount);
}

// ============================================================================
// Statistics & query functions
// ============================================================================

pub fn calculate_inventory_stats(items: &[Item]) {
    if items.is_empty() {
        println!("No items in inventory");
        return;
    }

    println!("\n=== Inventory Statistics (Array) ===");

    let mut total_value: f64 = 0.0;
    let mut total_items: i32 = 0;
    let mut max_price: f64 = 0.0;
    let mut min_price: f64 = items[0].price;

    for item in items.iter() {
        total_value += item.price * item.quantity as f64;
        total_items += item.quantity;
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
    }

    println!("Total unique items: {}", items.len());
    println!("Total item count: {}", total_items);
    println!("Total inventory value: ${:.2}", total_value);
    println!(
        "Average item price: ${:.2}",
        total_value / total_items as f64
    );
    println!("Most expensive item: ${:.2}", max_price);
    println!("Least expensive item: ${:.2}", min_price);
}

pub fn calculate_order_stats(orders: &LinkedList<Order>) {
    if orders.is_empty() {
        println!("No orders to analyze");
        return;
    }

    println!("\n=== Order Statistics (List) ===");

    let mut total_revenue: f64 = 0.0;
    let mut max_order: f64 = 0.0;
    let mut min_order: f64 = -1.0;

    for order in orders.iter() {
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0.0 || order.total_amount < min_order {
            min_order = order.total_amount;
        }
    }

    println!("Total orders: {}", orders.len());
    println!("Total revenue: ${:.2}", total_revenue);
    println!(
        "Average order value: ${:.2}",
        total_revenue / orders.len() as f64
    );
    println!("Largest order: ${:.2}", max_order);
    println!("Smallest order: ${:.2}", min_order);
}

pub fn find_items_by_category(items: &[Item], category: &str) {
    println!("\n=== Items in category '{}' ===", category);

    let mut found = 0;
    for item in items.iter() {
        if buf_eq_str(&item.category, category) {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        println!("No items found in this category");
    } else {
        println!("Found {} items", found);
    }
}

#[allow(dead_code)]
pub fn find_expensive_items(items: &LinkedList<Item>, min_price: f64) {
    println!("\n=== Items priced above ${:.2} ===", min_price);

    let mut found = 0;
    for item in items.iter() {
        if item.price >= min_price {
            print_item(item);
            found += 1;
        }
    }

    if found == 0 {
        println!("No items found above this price");
    } else {
        println!("Found {} items", found);
    }
}

// ============================================================================
// LinkedList — replacement for the C `list_TYPE_t` macro container.
// Stores values as a singly linked list, preserving append order.
// ============================================================================

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

struct Node<T> {
    data: T,
    next: Option<Box<Node<T>>>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        LinkedList {
            head: None,
            size: 0,
        }
    }

    pub fn append(&mut self, value: T) {
        let new_node = Box::new(Node {
            data: value,
            next: None,
        });
        // Walk to the tail and append
        match self.head.as_mut() {
            None => {
                self.head = Some(new_node);
            }
            Some(mut cur) => {
                while cur.next.is_some() {
                    cur = cur.next.as_mut().unwrap();
                }
                cur.next = Some(new_node);
            }
        }
        self.size += 1;
    }

    #[allow(dead_code)]
    pub fn prepend(&mut self, value: T) {
        let new_node = Box::new(Node {
            data: value,
            next: self.head.take(),
        });
        self.head = Some(new_node);
        self.size += 1;
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        // Iteratively drop to avoid recursive Drop blowing the stack.
        let mut cur = self.head.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
        self.size = 0;
    }

    pub fn iter(&self) -> LinkedListIter<'_, T> {
        LinkedListIter {
            current: self.head.as_deref(),
        }
    }
}

impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        // Iteratively drop to avoid stack overflow on very long lists.
        let mut cur = self.head.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
    }
}

pub struct LinkedListIter<'a, T> {
    current: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for LinkedListIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        match self.current {
            Some(node) => {
                self.current = node.next.as_deref();
                Some(&node.data)
            }
            None => None,
        }
    }
}
