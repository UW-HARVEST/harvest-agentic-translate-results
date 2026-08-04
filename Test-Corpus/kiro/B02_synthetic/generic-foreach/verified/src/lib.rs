use std::io::Write;
use std::os::raw::{c_char, c_int, c_double};
use std::ffi::CStr;

const MAX_NAME_LENGTH: usize = 64;
const MAX_CATEGORY_LENGTH: usize = 32;

// C-compatible structs with fixed-size char arrays
#[repr(C)]
#[derive(Clone, Copy)]
pub struct item_t {
    pub id: c_int,
    pub name: [c_char; MAX_NAME_LENGTH],
    pub category: [c_char; MAX_CATEGORY_LENGTH],
    pub price: c_double,
    pub quantity: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct order_t {
    pub customer_id: c_int,
    pub customer_name: [c_char; MAX_NAME_LENGTH],
    pub total_amount: c_double,
}

// Helper to copy a C string into a fixed buffer
fn copy_cstr_to_buf(dst: &mut [c_char], src: *const c_char) {
    let s = unsafe { CStr::from_ptr(src) };
    let bytes = s.to_bytes();
    let max = dst.len() - 1;
    let len = bytes.len().min(max);
    for i in 0..len {
        dst[i] = bytes[i] as c_char;
    }
    dst[len] = 0;
}

fn cstr_as_str(buf: &[c_char]) -> &str {
    unsafe { CStr::from_ptr(buf.as_ptr()).to_str().unwrap_or("") }
}

// ============================================================================
// GENERIC DYNAMIC ARRAY - using Vec internally but exposed as opaque pointer
// ============================================================================

macro_rules! define_array {
    ($T:ty, $prefix:ident) => {
        paste::paste! {
            // We can't use paste here, so we'll define each manually
        }
    };
}

// --- array_int ---
pub struct ArrayInt { data: Vec<c_int> }

#[no_mangle] pub extern "C" fn array_int_create(_cap: usize) -> *mut ArrayInt {
    Box::into_raw(Box::new(ArrayInt { data: Vec::new() }))
}
#[no_mangle] pub extern "C" fn array_int_destroy(arr: *mut ArrayInt) {
    if !arr.is_null() { unsafe { drop(Box::from_raw(arr)); } }
}
#[no_mangle] pub extern "C" fn array_int_push(arr: *mut ArrayInt, value: c_int) -> c_int {
    if arr.is_null() { return -1; }
    unsafe { (*arr).data.push(value); }
    0
}
#[no_mangle] pub extern "C" fn array_int_get(arr: *mut ArrayInt, index: usize) -> c_int {
    unsafe { (&(*arr).data)[index] }
}
#[no_mangle] pub extern "C" fn array_int_size(arr: *mut ArrayInt) -> usize {
    if arr.is_null() { return 0; }
    unsafe { (*arr).data.len() }
}
#[no_mangle] pub extern "C" fn array_int_clear(arr: *mut ArrayInt) {
    if !arr.is_null() { unsafe { (*arr).data.clear(); } }
}

// --- array_double ---
pub struct ArrayDouble { data: Vec<c_double> }

#[no_mangle] pub extern "C" fn array_double_create(_cap: usize) -> *mut ArrayDouble {
    Box::into_raw(Box::new(ArrayDouble { data: Vec::new() }))
}
#[no_mangle] pub extern "C" fn array_double_destroy(arr: *mut ArrayDouble) {
    if !arr.is_null() { unsafe { drop(Box::from_raw(arr)); } }
}
#[no_mangle] pub extern "C" fn array_double_push(arr: *mut ArrayDouble, value: c_double) -> c_int {
    if arr.is_null() { return -1; }
    unsafe { (*arr).data.push(value); }
    0
}
#[no_mangle] pub extern "C" fn array_double_get(arr: *mut ArrayDouble, index: usize) -> c_double {
    unsafe { (&(*arr).data)[index] }
}
#[no_mangle] pub extern "C" fn array_double_size(arr: *mut ArrayDouble) -> usize {
    if arr.is_null() { return 0; }
    unsafe { (*arr).data.len() }
}
#[no_mangle] pub extern "C" fn array_double_clear(arr: *mut ArrayDouble) {
    if !arr.is_null() { unsafe { (*arr).data.clear(); } }
}

// --- array_item_t ---
pub struct ArrayItemT { data: Vec<item_t> }

#[no_mangle] pub extern "C" fn array_item_t_create(_cap: usize) -> *mut ArrayItemT {
    Box::into_raw(Box::new(ArrayItemT { data: Vec::new() }))
}
#[no_mangle] pub extern "C" fn array_item_t_destroy(arr: *mut ArrayItemT) {
    if !arr.is_null() { unsafe { drop(Box::from_raw(arr)); } }
}
#[no_mangle] pub extern "C" fn array_item_t_push(arr: *mut ArrayItemT, value: item_t) -> c_int {
    if arr.is_null() { return -1; }
    unsafe { (*arr).data.push(value); }
    0
}
#[no_mangle] pub extern "C" fn array_item_t_get(arr: *mut ArrayItemT, index: usize) -> item_t {
    unsafe { (&(*arr).data)[index] }
}
#[no_mangle] pub extern "C" fn array_item_t_size(arr: *mut ArrayItemT) -> usize {
    if arr.is_null() { return 0; }
    unsafe { (*arr).data.len() }
}
#[no_mangle] pub extern "C" fn array_item_t_clear(arr: *mut ArrayItemT) {
    if !arr.is_null() { unsafe { (*arr).data.clear(); } }
}

// --- array_order_t ---
pub struct ArrayOrderT { data: Vec<order_t> }

#[no_mangle] pub extern "C" fn array_order_t_create(_cap: usize) -> *mut ArrayOrderT {
    Box::into_raw(Box::new(ArrayOrderT { data: Vec::new() }))
}
#[no_mangle] pub extern "C" fn array_order_t_destroy(arr: *mut ArrayOrderT) {
    if !arr.is_null() { unsafe { drop(Box::from_raw(arr)); } }
}
#[no_mangle] pub extern "C" fn array_order_t_push(arr: *mut ArrayOrderT, value: order_t) -> c_int {
    if arr.is_null() { return -1; }
    unsafe { (*arr).data.push(value); }
    0
}
#[no_mangle] pub extern "C" fn array_order_t_get(arr: *mut ArrayOrderT, index: usize) -> order_t {
    unsafe { (&(*arr).data)[index] }
}
#[no_mangle] pub extern "C" fn array_order_t_size(arr: *mut ArrayOrderT) -> usize {
    if arr.is_null() { return 0; }
    unsafe { (*arr).data.len() }
}
#[no_mangle] pub extern "C" fn array_order_t_clear(arr: *mut ArrayOrderT) {
    if !arr.is_null() { unsafe { (*arr).data.clear(); } }
}

// ============================================================================
// GENERIC LINKED LIST - using Vec internally (same iteration order)
// ============================================================================

// --- list_int ---
pub struct ListInt { data: Vec<c_int> }

#[no_mangle] pub extern "C" fn list_int_create() -> *mut ListInt {
    Box::into_raw(Box::new(ListInt { data: Vec::new() }))
}
#[no_mangle] pub extern "C" fn list_int_destroy(list: *mut ListInt) {
    if !list.is_null() { unsafe { drop(Box::from_raw(list)); } }
}
#[no_mangle] pub extern "C" fn list_int_append(list: *mut ListInt, value: c_int) -> c_int {
    if list.is_null() { return -1; }
    unsafe { (*list).data.push(value); }
    0
}
#[no_mangle] pub extern "C" fn list_int_prepend(list: *mut ListInt, value: c_int) -> c_int {
    if list.is_null() { return -1; }
    unsafe { (*list).data.insert(0, value); }
    0
}
#[no_mangle] pub extern "C" fn list_int_size(list: *mut ListInt) -> usize {
    if list.is_null() { return 0; }
    unsafe { (*list).data.len() }
}
#[no_mangle] pub extern "C" fn list_int_clear(list: *mut ListInt) {
    if !list.is_null() { unsafe { (*list).data.clear(); } }
}

// --- list_double ---
pub struct ListDouble { data: Vec<c_double> }

#[no_mangle] pub extern "C" fn list_double_create() -> *mut ListDouble {
    Box::into_raw(Box::new(ListDouble { data: Vec::new() }))
}
#[no_mangle] pub extern "C" fn list_double_destroy(list: *mut ListDouble) {
    if !list.is_null() { unsafe { drop(Box::from_raw(list)); } }
}
#[no_mangle] pub extern "C" fn list_double_append(list: *mut ListDouble, value: c_double) -> c_int {
    if list.is_null() { return -1; }
    unsafe { (*list).data.push(value); }
    0
}
#[no_mangle] pub extern "C" fn list_double_prepend(list: *mut ListDouble, value: c_double) -> c_int {
    if list.is_null() { return -1; }
    unsafe { (*list).data.insert(0, value); }
    0
}
#[no_mangle] pub extern "C" fn list_double_size(list: *mut ListDouble) -> usize {
    if list.is_null() { return 0; }
    unsafe { (*list).data.len() }
}
#[no_mangle] pub extern "C" fn list_double_clear(list: *mut ListDouble) {
    if !list.is_null() { unsafe { (*list).data.clear(); } }
}

// --- list_item_t ---
pub struct ListItemT { data: Vec<item_t> }

#[no_mangle] pub extern "C" fn list_item_t_create() -> *mut ListItemT {
    Box::into_raw(Box::new(ListItemT { data: Vec::new() }))
}
#[no_mangle] pub extern "C" fn list_item_t_destroy(list: *mut ListItemT) {
    if !list.is_null() { unsafe { drop(Box::from_raw(list)); } }
}
#[no_mangle] pub extern "C" fn list_item_t_append(list: *mut ListItemT, value: item_t) -> c_int {
    if list.is_null() { return -1; }
    unsafe { (*list).data.push(value); }
    0
}
#[no_mangle] pub extern "C" fn list_item_t_prepend(list: *mut ListItemT, value: item_t) -> c_int {
    if list.is_null() { return -1; }
    unsafe { (*list).data.insert(0, value); }
    0
}
#[no_mangle] pub extern "C" fn list_item_t_size(list: *mut ListItemT) -> usize {
    if list.is_null() { return 0; }
    unsafe { (*list).data.len() }
}
#[no_mangle] pub extern "C" fn list_item_t_clear(list: *mut ListItemT) {
    if !list.is_null() { unsafe { (*list).data.clear(); } }
}

// --- list_order_t ---
pub struct ListOrderT { data: Vec<order_t> }

#[no_mangle] pub extern "C" fn list_order_t_create() -> *mut ListOrderT {
    Box::into_raw(Box::new(ListOrderT { data: Vec::new() }))
}
#[no_mangle] pub extern "C" fn list_order_t_destroy(list: *mut ListOrderT) {
    if !list.is_null() { unsafe { drop(Box::from_raw(list)); } }
}
#[no_mangle] pub extern "C" fn list_order_t_append(list: *mut ListOrderT, value: order_t) -> c_int {
    if list.is_null() { return -1; }
    unsafe { (*list).data.push(value); }
    0
}
#[no_mangle] pub extern "C" fn list_order_t_prepend(list: *mut ListOrderT, value: order_t) -> c_int {
    if list.is_null() { return -1; }
    unsafe { (*list).data.insert(0, value); }
    0
}
#[no_mangle] pub extern "C" fn list_order_t_size(list: *mut ListOrderT) -> usize {
    if list.is_null() { return 0; }
    unsafe { (*list).data.len() }
}
#[no_mangle] pub extern "C" fn list_order_t_clear(list: *mut ListOrderT) {
    if !list.is_null() { unsafe { (*list).data.clear(); } }
}

// ============================================================================
// INVENTORY FUNCTIONS
// ============================================================================

#[no_mangle]
pub extern "C" fn create_item(id: c_int, name: *const c_char, category: *const c_char, price: c_double, quantity: c_int) -> item_t {
    let mut item = item_t { id, name: [0; MAX_NAME_LENGTH], category: [0; MAX_CATEGORY_LENGTH], price, quantity };
    copy_cstr_to_buf(&mut item.name, name);
    copy_cstr_to_buf(&mut item.category, category);
    item
}

#[no_mangle]
pub extern "C" fn create_order(customer_id: c_int, customer_name: *const c_char, total_amount: c_double) -> order_t {
    let mut order = order_t { customer_id, customer_name: [0; MAX_NAME_LENGTH], total_amount };
    copy_cstr_to_buf(&mut order.customer_name, customer_name);
    order
}

#[no_mangle]
pub extern "C" fn print_item(item: item_t) {
    let name = cstr_as_str(&item.name);
    let category = cstr_as_str(&item.category);
    print!("  [{}] {}\n", item.id, name);
    print!("      Category: {}\n", category);
    print!("      Price: ${:.2}\n", item.price);
    print!("      Quantity: {}\n", item.quantity);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn print_order(order: order_t) {
    let name = cstr_as_str(&order.customer_name);
    print!("  Order - Customer ID: {}, Name: {}\n", order.customer_id, name);
    print!("          Total: ${:.2}\n", order.total_amount);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn calculate_inventory_stats(items: *mut ArrayItemT) {
    let data = if items.is_null() { &[] as &[item_t] } else { unsafe { &(*items).data } };
    if data.is_empty() {
        print!("No items in inventory\n");
        std::io::stdout().flush().ok();
        return;
    }
    print!("\n=== Inventory Statistics (Array) ===\n");
    let mut total_value = 0.0_f64;
    let mut total_items = 0_i32;
    let mut max_price = 0.0_f64;
    let mut min_price = data[0].price;
    for item in data {
        total_value += item.price * item.quantity as f64;
        total_items += item.quantity;
        if item.price > max_price { max_price = item.price; }
        if item.price < min_price { min_price = item.price; }
    }
    print!("Total unique items: {}\n", data.len());
    print!("Total item count: {}\n", total_items);
    print!("Total inventory value: ${:.2}\n", total_value);
    print!("Average item price: ${:.2}\n", total_value / total_items as f64);
    print!("Most expensive item: ${:.2}\n", max_price);
    print!("Least expensive item: ${:.2}\n", min_price);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn calculate_order_stats(orders: *mut ListOrderT) {
    let data = if orders.is_null() { &[] as &[order_t] } else { unsafe { &(*orders).data } };
    if data.is_empty() {
        print!("No orders to analyze\n");
        std::io::stdout().flush().ok();
        return;
    }
    print!("\n=== Order Statistics (List) ===\n");
    let mut total_revenue = 0.0_f64;
    let mut max_order = 0.0_f64;
    let mut min_order = -1.0_f64;
    for order in data {
        total_revenue += order.total_amount;
        if order.total_amount > max_order { max_order = order.total_amount; }
        if min_order < 0.0 || order.total_amount < min_order { min_order = order.total_amount; }
    }
    print!("Total orders: {}\n", data.len());
    print!("Total revenue: ${:.2}\n", total_revenue);
    print!("Average order value: ${:.2}\n", total_revenue / data.len() as f64);
    print!("Largest order: ${:.2}\n", max_order);
    print!("Smallest order: ${:.2}\n", min_order);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn find_items_by_category(items: *mut ArrayItemT, category: *const c_char) {
    if items.is_null() || category.is_null() { return; }
    let cat = unsafe { CStr::from_ptr(category).to_str().unwrap_or("") };
    let data = unsafe { &(*items).data };
    print!("\n=== Items in category '{}' ===\n", cat);
    let mut found = 0_i32;
    for item in data {
        if cstr_as_str(&item.category) == cat {
            print_item(*item);
            found += 1;
        }
    }
    if found == 0 {
        print!("No items found in this category\n");
    } else {
        print!("Found {} items\n", found);
    }
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn find_expensive_items(items: *mut ListItemT, min_price: c_double) {
    if items.is_null() { return; }
    let data = unsafe { &(*items).data };
    print!("\n=== Items priced above ${:.2} ===\n", min_price);
    let mut found = 0_i32;
    for item in data {
        if item.price >= min_price {
            print_item(*item);
            found += 1;
        }
    }
    if found == 0 {
        print!("No items found above this price\n");
    } else {
        print!("Found {} items\n", found);
    }
    std::io::stdout().flush().ok();
}

// ============================================================================
// DEMO FUNCTIONS (from main.c)
// ============================================================================

#[no_mangle]
pub extern "C" fn print_menu() {
    print!("\n");
    print!("========================================\n");
    print!("  GENERIC FOR_EACH MACRO DEMO\n");
    print!("========================================\n");
    print!("1. Demo: Integer Containers\n");
    print!("2. Demo: Double Containers\n");
    print!("3. Demo: Inventory Array\n");
    print!("4. Demo: Order List\n");
    print!("5. Demo: Mixed Operations\n");
    print!("6. Run All Demos\n");
    print!("7. Exit\n");
    print!("========================================\n");
    print!("Choice: ");
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn demo_integer_containers() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 1: Integer Containers\n");
    print!("========================================\n");

    let int_array = array_int_create(10);
    print!("\n--- Integer Array ---\n");
    print!("Adding integers: 10, 20, 30, 40, 50\n");
    for &v in &[10, 20, 30, 40, 50] {
        array_int_push(int_array, v);
    }
    print!("Array contents: ");
    let sz = array_int_size(int_array);
    for i in 0..sz {
        print!("{} ", array_int_get(int_array, i));
    }
    print!("\n");

    let mut sum: c_int = 0;
    for i in 0..sz { sum += array_int_get(int_array, i); }
    print!("Sum: {}\n", sum);
    print!("Average: {:.2}\n", sum as f64 / sz as f64);

    let int_list = list_int_create();
    print!("\n--- Integer List ---\n");
    print!("Adding integers: 100, 200, 300, 400, 500\n");
    for &v in &[100, 200, 300, 400, 500] {
        list_int_append(int_list, v);
    }
    print!("List contents: ");
    let lsz = list_int_size(int_list);
    // List doesn't have get, so we access data directly
    let list_data = unsafe { &(*int_list).data };
    for v in list_data { print!("{} ", v); }
    print!("\n");

    let mut product: i64 = 1;
    for v in list_data { product *= *v as i64; }
    print!("Product: {}\n", product);

    array_int_destroy(int_array);
    list_int_destroy(int_list);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn demo_double_containers() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 2: Double Containers\n");
    print!("========================================\n");

    let double_array = array_double_create(5);
    print!("\n--- Double Array (Temperatures in Celsius) ---\n");
    let temps = [23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5];
    print!("Adding temperatures: ");
    for &t in &temps {
        array_double_push(double_array, t);
        print!("{:.1} ", t);
    }
    print!("\n");

    let sz = array_double_size(double_array);
    let mut min_temp = array_double_get(double_array, 0);
    let mut max_temp = min_temp;
    let mut sum_temp = 0.0_f64;
    for i in 0..sz {
        let t = array_double_get(double_array, i);
        if t < min_temp { min_temp = t; }
        if t > max_temp { max_temp = t; }
        sum_temp += t;
    }
    print!("Minimum: {:.1}°C\n", min_temp);
    print!("Maximum: {:.1}°C\n", max_temp);
    print!("Average: {:.1}°C\n", sum_temp / sz as f64);

    let price_list = list_double_create();
    print!("\n--- Double List (Product Prices) ---\n");
    let prices = [9.99, 14.50, 7.25, 22.00, 5.99, 18.75];
    print!("Adding prices: ");
    for &p in &prices {
        list_double_append(price_list, p);
        print!("${:.2} ", p);
    }
    print!("\n");

    let pdata = unsafe { &(*price_list).data };
    let mut total = 0.0_f64;
    let mut count_under_10 = 0_i32;
    for &p in pdata {
        total += p;
        if p < 10.0 { count_under_10 += 1; }
    }
    print!("Total cost: ${:.2}\n", total);
    print!("Items under $10: {}\n", count_under_10);

    array_double_destroy(double_array);
    list_double_destroy(price_list);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn demo_inventory_array() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 3: Inventory Array (Items)\n");
    print!("========================================\n");

    let inventory = array_item_t_create(20);
    print!("\n--- Adding Items to Inventory ---\n");

    let names: &[(&[u8], &[u8], f64, i32)] = &[
        (b"Laptop\0", b"Electronics\0", 899.99, 15),
        (b"Mouse\0", b"Electronics\0", 24.99, 50),
        (b"Keyboard\0", b"Electronics\0", 79.99, 30),
        (b"Monitor\0", b"Electronics\0", 299.99, 20),
        (b"Desk Chair\0", b"Furniture\0", 199.99, 10),
        (b"Desk\0", b"Furniture\0", 349.99, 8),
        (b"Notebook\0", b"Office\0", 4.99, 100),
        (b"Pen Set\0", b"Office\0", 12.99, 75),
        (b"USB Cable\0", b"Electronics\0", 9.99, 60),
        (b"Bookshelf\0", b"Furniture\0", 149.99, 12),
    ];
    for (i, &(name, cat, price, qty)) in names.iter().enumerate() {
        let item = create_item((i + 1) as c_int, name.as_ptr() as *const c_char, cat.as_ptr() as *const c_char, price, qty);
        array_item_t_push(inventory, item);
    }
    let sz = array_item_t_size(inventory);
    print!("Added {} items to inventory\n", sz);

    print!("\n--- All Inventory Items ---\n");
    for i in 0..sz {
        print_item(array_item_t_get(inventory, i));
        print!("\n");
    }

    calculate_inventory_stats(inventory);
    find_items_by_category(inventory, b"Electronics\0".as_ptr() as *const c_char);
    find_items_by_category(inventory, b"Furniture\0".as_ptr() as *const c_char);

    print!("\n--- Low Stock Items (< 20) ---\n");
    let mut low_stock_count = 0_i32;
    for i in 0..sz {
        let item = array_item_t_get(inventory, i);
        if item.quantity < 20 {
            print_item(item);
            low_stock_count += 1;
        }
    }
    print!("Total low stock items: {}\n", low_stock_count);

    array_item_t_destroy(inventory);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn demo_order_list() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 4: Order List (Orders)\n");
    print!("========================================\n");

    let orders = list_order_t_create();
    print!("\n--- Adding Orders ---\n");

    let order_data: &[(i32, &[u8], f64)] = &[
        (1001, b"Alice Johnson\0", 1249.95),
        (1002, b"Bob Smith\0", 89.99),
        (1003, b"Carol White\0", 549.98),
        (1004, b"David Brown\0", 24.99),
        (1005, b"Eve Davis\0", 899.99),
        (1006, b"Frank Miller\0", 374.97),
        (1007, b"Grace Lee\0", 159.98),
        (1008, b"Henry Wilson\0", 1099.99),
    ];
    for &(cid, name, amt) in order_data {
        let o = create_order(cid, name.as_ptr() as *const c_char, amt);
        list_order_t_append(orders, o);
    }
    let sz = list_order_t_size(orders);
    print!("Added {} orders\n", sz);

    print!("\n--- All Orders ---\n");
    let odata = unsafe { &(*orders).data };
    for o in odata { print_order(*o); }

    calculate_order_stats(orders);

    print!("\n--- Large Orders (> $500) ---\n");
    let mut large_order_count = 0_i32;
    let mut large_order_total = 0.0_f64;
    let odata = unsafe { &(*orders).data };
    for o in odata {
        if o.total_amount > 500.0 {
            print_order(*o);
            large_order_count += 1;
            large_order_total += o.total_amount;
        }
    }
    print!("Total large orders: {}\n", large_order_count);
    print!("Revenue from large orders: ${:.2}\n", large_order_total);

    list_order_t_destroy(orders);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn demo_mixed_operations() {
    print!("\n");
    print!("========================================\n");
    print!("  DEMO 5: Mixed Operations\n");
    print!("========================================\n");

    let array_inventory = array_item_t_create(10);
    let list_inventory = list_item_t_create();

    print!("\n--- Populating both Array and List ---\n");

    let items_data: &[(&[u8], &[u8], f64, i32)] = &[
        (b"Smartphone\0", b"Electronics\0", 699.99, 25),
        (b"Tablet\0", b"Electronics\0", 449.99, 18),
        (b"Headphones\0", b"Electronics\0", 149.99, 40),
        (b"Smart Watch\0", b"Electronics\0", 299.99, 22),
        (b"Power Bank\0", b"Electronics\0", 39.99, 55),
    ];
    for (i, &(name, cat, price, qty)) in items_data.iter().enumerate() {
        let item = create_item((i + 1) as c_int, name.as_ptr() as *const c_char, cat.as_ptr() as *const c_char, price, qty);
        array_item_t_push(array_inventory, item);
        list_item_t_append(list_inventory, item);
    }
    print!("Added {} items to both containers\n", items_data.len());

    print!("\n--- Iterating through Array ---\n");
    let mut array_count = 0_i32;
    let asz = array_item_t_size(array_inventory);
    for _ in 0..asz { array_count += 1; }
    print!("Array iteration count: {}\n", array_count);

    print!("\n--- Iterating through List ---\n");
    let mut list_count = 0_i32;
    let ldata = unsafe { &(*list_inventory).data };
    for _ in ldata { list_count += 1; }
    print!("List iteration count: {}\n", list_count);

    let price_threshold = 200.0_f64;

    print!("\n--- Items above ${:.2} (Array) ---\n", price_threshold);
    for i in 0..asz {
        let item = array_item_t_get(array_inventory, i);
        if item.price >= price_threshold {
            print!("  {}: ${:.2}\n", cstr_as_str(&item.name), item.price);
        }
    }

    print!("\n--- Items above ${:.2} (List) ---\n", price_threshold);
    let ldata = unsafe { &(*list_inventory).data };
    for item in ldata {
        if item.price >= price_threshold {
            print!("  {}: ${:.2}\n", cstr_as_str(&item.name), item.price);
        }
    }

    array_item_t_destroy(array_inventory);
    list_item_t_destroy(list_inventory);
    std::io::stdout().flush().ok();
}

#[no_mangle]
pub extern "C" fn main() -> c_int {
    print!("╔════════════════════════════════════════╗\n");
    print!("║   GENERIC FOR_EACH MACRO DEMO         ║\n");
    print!("║   Demonstrating Generic Containers    ║\n");
    print!("╚════════════════════════════════════════╝\n");
    std::io::stdout().flush().ok();

    let stdin = std::io::stdin();
    let mut input = String::new();

    loop {
        print_menu();
        input.clear();
        if stdin.read_line(&mut input).unwrap_or(0) == 0 { break; }
        let choice: c_int = match input.trim().parse() {
            Ok(v) => v,
            Err(_) => { print!("Invalid input\n"); std::io::stdout().flush().ok(); continue; }
        };
        match choice {
            1 => demo_integer_containers(),
            2 => demo_double_containers(),
            3 => demo_inventory_array(),
            4 => demo_order_list(),
            5 => demo_mixed_operations(),
            6 => {
                print!("\n=== Running All Demos ===\n");
                demo_integer_containers();
                demo_double_containers();
                demo_inventory_array();
                demo_order_list();
                demo_mixed_operations();
                print!("\n========================================\n");
                print!("  All demos completed successfully!\n");
                print!("========================================\n");
                std::io::stdout().flush().ok();
            }
            7 => { print!("\nGoodbye!\n"); std::io::stdout().flush().ok(); return 0; }
            _ => { print!("Invalid choice\n"); std::io::stdout().flush().ok(); }
        }
    }
    0
}
