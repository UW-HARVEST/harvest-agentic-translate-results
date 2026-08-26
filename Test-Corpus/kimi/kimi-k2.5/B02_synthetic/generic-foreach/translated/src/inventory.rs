use std::fmt;

#[derive(Clone)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub price: f64,
    pub quantity: i32,
}

impl Item {
    pub fn new(id: i32, name: &str, category: &str, price: f64, quantity: i32) -> Self {
        Self {
            id,
            name: name.to_string(),
            category: category.to_string(),
            price,
            quantity,
        }
    }

    pub fn print(&self) {
        println!("  [{}] {}", self.id, self.name);
        println!("      Category: {}", self.category);
        println!("      Price: ${:.2}", self.price);
        println!("      Quantity: {}", self.quantity);
    }
}

#[derive(Clone)]
pub struct Order {
    pub customer_id: i32,
    pub customer_name: String,
    pub total_amount: f64,
}

impl Order {
    pub fn new(customer_id: i32, customer_name: &str, total_amount: f64) -> Self {
        Self {
            customer_id,
            customer_name: customer_name.to_string(),
            total_amount,
        }
    }

    pub fn print(&self) {
        println!("  Order - Customer ID: {}, Name: {}", self.customer_id, self.customer_name);
        println!("          Total: ${:.2}", self.total_amount);
    }
}

pub struct IntArray {
    data: Vec<i32>,
}

impl IntArray {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: i32) {
        self.data.push(value);
    }

    pub fn get(&self, index: usize) -> Option<&i32> {
        self.data.get(index)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::slice::Iter<i32> {
        self.data.iter()
    }
}

pub struct DoubleArray {
    data: Vec<f64>,
}

impl DoubleArray {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: f64) {
        self.data.push(value);
    }

    pub fn get(&self, index: usize) -> Option<&f64> {
        self.data.get(index)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::slice::Iter<f64> {
        self.data.iter()
    }
}

pub struct InventoryArray {
    data: Vec<Item>,
}

impl InventoryArray {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: Item) {
        self.data.push(value);
    }

    pub fn get(&self, index: usize) -> Option<&Item> {
        self.data.get(index)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::slice::Iter<Item> {
        self.data.iter()
    }

    pub fn calculate_stats(&self) {
        if self.data.is_empty() {
            println!("No items in inventory");
            return;
        }

        println!("\n=== Inventory Statistics (Array) ===");

        let total_value: f64 = self.data.iter().map(|i| i.price * i.quantity as f64).sum();
        let total_items: i32 = self.data.iter().map(|i| i.quantity).sum();
        let max_price = self.data.iter().map(|i| i.price).fold(f64::NEG_INFINITY, f64::max);
        let min_price = self.data.iter().map(|i| i.price).fold(f64::INFINITY, f64::min);

        println!("Total unique items: {}", self.data.len());
        println!("Total item count: {}", total_items);
        println!("Total inventory value: ${:.2}", total_value);
        println!("Average item price: ${:.2}", total_value / total_items as f64);
        println!("Most expensive item: ${:.2}", max_price);
        println!("Least expensive item: ${:.2}", min_price);
    }

    pub fn find_by_category(&self, category: &str) {
        println!("\n=== Items in category '{}' ===", category);

        let found: Vec<&Item> = self.data.iter().filter(|i| i.category == category).collect();

        if found.is_empty() {
            println!("No items found in this category");
        } else {
            for item in &found {
                item.print();
            }
            println!("Found {} items", found.len());
        }
    }
}

pub struct IntList {
    data: Vec<i32>,
}

impl IntList {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn append(&mut self, value: i32) {
        self.data.push(value);
    }

    pub fn prepend(&mut self, value: i32) {
        self.data.insert(0, value);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::slice::Iter<i32> {
        self.data.iter()
    }
}

pub struct DoubleList {
    data: Vec<f64>,
}

impl DoubleList {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn append(&mut self, value: f64) {
        self.data.push(value);
    }

    pub fn prepend(&mut self, value: f64) {
        self.data.insert(0, value);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::slice::Iter<f64> {
        self.data.iter()
    }
}

pub struct InventoryList {
    data: Vec<Item>,
}

impl InventoryList {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn append(&mut self, value: Item) {
        self.data.push(value);
    }

    pub fn prepend(&mut self, value: Item) {
        self.data.insert(0, value);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::slice::Iter<Item> {
        self.data.iter()
    }

    pub fn find_expensive(&self, min_price: f64) {
        println!("\n=== Items priced above ${:.2} ===", min_price);

        let found: Vec<&Item> = self.data.iter().filter(|i| i.price >= min_price).collect();

        if found.is_empty() {
            println!("No items found above this price");
        } else {
            for item in &found {
                item.print();
            }
            println!("Found {} items", found.len());
        }
    }
}

pub struct OrderList {
    data: Vec<Order>,
}

impl OrderList {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn append(&mut self, value: Order) {
        self.data.push(value);
    }

    pub fn prepend(&mut self, value: Order) {
        self.data.insert(0, value);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> std::slice::Iter<Order> {
        self.data.iter()
    }

    pub fn calculate_stats(&self) {
        if self.data.is_empty() {
            println!("No orders to analyze");
            return;
        }

        println!("\n=== Order Statistics (List) ===");

        let total_revenue: f64 = self.data.iter().map(|o| o.total_amount).sum();
        let max_order = self.data.iter().map(|o| o.total_amount).fold(f64::NEG_INFINITY, f64::max);
        let min_order = self.data.iter().map(|o| o.total_amount).fold(f64::INFINITY, f64::min);

        println!("Total orders: {}", self.data.len());
        println!("Total revenue: ${:.2}", total_revenue);
        println!("Average order value: ${:.2}", total_revenue / self.data.len() as f64);
        println!("Largest order: ${:.2}", max_order);
        println!("Smallest order: ${:.2}", min_order);
    }
}
