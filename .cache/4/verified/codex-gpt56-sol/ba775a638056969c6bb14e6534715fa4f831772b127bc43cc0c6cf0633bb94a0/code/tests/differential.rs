use libloading::Library;
use std::env;
use std::ffi::{c_char, c_double, c_int, c_long, c_void, CString};
use std::fs;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

const RANDOM_CASES: usize = 64;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Item {
    id: c_int,
    name: [c_char; 64],
    category: [c_char; 32],
    price: c_double,
    quantity: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Order {
    customer_id: c_int,
    customer_name: [c_char; 64],
    total_amount: c_double,
}

#[repr(C)]
struct Array<T> {
    data: *mut T,
    size: usize,
    capacity: usize,
}

#[repr(C)]
struct ListNode<T> {
    data: T,
    next: *mut ListNode<T>,
}

#[repr(C)]
struct List<T> {
    head: *mut ListNode<T>,
    tail: *mut ListNode<T>,
    size: usize,
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        Self {
            c: unsafe { Library::new(c_library_path()) }.expect("load C shared object"),
            rust: unsafe { Library::new(rust_library_path()) }.expect("load Rust shared object"),
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_root().join("c_src/build/libinventory_c.so")
}

fn rust_library_path() -> PathBuf {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root().join("target"));
    target.join("debug/libgeneric_for_each_demo.so")
}

unsafe fn symbol<T: Copy>(library: &Library, name: &str) -> T {
    *unsafe { library.get::<T>(name.as_bytes()) }
        .unwrap_or_else(|error| panic!("load symbol {name}: {error}"))
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_i32() as f64) / 65536.0
    }
}

trait TestValue: Copy {
    fn random(rng: &mut Rng) -> Self;
    fn assert_same(left: Self, right: Self);
}

impl TestValue for c_int {
    fn random(rng: &mut Rng) -> Self {
        rng.next_i32()
    }

    fn assert_same(left: Self, right: Self) {
        assert_eq!(left, right);
    }
}

impl TestValue for c_double {
    fn random(rng: &mut Rng) -> Self {
        rng.next_f64()
    }

    fn assert_same(left: Self, right: Self) {
        assert_eq!(left.to_bits(), right.to_bits());
    }
}

impl TestValue for Item {
    fn random(rng: &mut Rng) -> Self {
        Item {
            id: rng.next_i32(),
            name: random_c_array(rng),
            category: random_c_array(rng),
            price: rng.next_f64(),
            quantity: rng.next_i32(),
        }
    }

    fn assert_same(left: Self, right: Self) {
        assert_eq!(left.id, right.id);
        assert_eq!(left.name, right.name);
        assert_eq!(left.category, right.category);
        assert_eq!(left.price.to_bits(), right.price.to_bits());
        assert_eq!(left.quantity, right.quantity);
    }
}

impl TestValue for Order {
    fn random(rng: &mut Rng) -> Self {
        Order {
            customer_id: rng.next_i32(),
            customer_name: random_c_array(rng),
            total_amount: rng.next_f64(),
        }
    }

    fn assert_same(left: Self, right: Self) {
        assert_eq!(left.customer_id, right.customer_id);
        assert_eq!(left.customer_name, right.customer_name);
        assert_eq!(left.total_amount.to_bits(), right.total_amount.to_bits());
    }
}

fn random_c_array<const N: usize>(rng: &mut Rng) -> [c_char; N] {
    let mut value = [0; N];
    let length = (rng.next_u64() as usize) % N;
    for byte in value.iter_mut().take(length) {
        *byte = (b'a' + (rng.next_u64() % 26) as u8) as c_char;
    }
    value
}

unsafe fn assert_array_contents<T: TestValue>(
    left: *mut Array<T>,
    right: *mut Array<T>,
    get_left: unsafe extern "C" fn(*mut Array<T>, usize) -> T,
    get_right: unsafe extern "C" fn(*mut Array<T>, usize) -> T,
) {
    assert_eq!(unsafe { (*left).size }, unsafe { (*right).size });
    assert_eq!(unsafe { (*left).capacity }, unsafe { (*right).capacity });
    for index in 0..unsafe { (*left).size } {
        T::assert_same(unsafe { get_left(left, index) }, unsafe {
            get_right(right, index)
        });
    }
}

unsafe fn exercise_array<T: TestValue>(libraries: &Libraries, prefix: &str, seed: u64) {
    type Create<T> = unsafe extern "C" fn(usize) -> *mut Array<T>;
    type Destroy<T> = unsafe extern "C" fn(*mut Array<T>);
    type Push<T> = unsafe extern "C" fn(*mut Array<T>, T) -> c_int;
    type Get<T> = unsafe extern "C" fn(*mut Array<T>, usize) -> T;
    type Size<T> = unsafe extern "C" fn(*mut Array<T>) -> usize;
    type Clear<T> = unsafe extern "C" fn(*mut Array<T>);

    let create_c: Create<T> = unsafe { symbol(&libraries.c, &format!("array_{prefix}_create")) };
    let create_r: Create<T> = unsafe { symbol(&libraries.rust, &format!("array_{prefix}_create")) };
    let destroy_c: Destroy<T> = unsafe { symbol(&libraries.c, &format!("array_{prefix}_destroy")) };
    let destroy_r: Destroy<T> =
        unsafe { symbol(&libraries.rust, &format!("array_{prefix}_destroy")) };
    let push_c: Push<T> = unsafe { symbol(&libraries.c, &format!("array_{prefix}_push")) };
    let push_r: Push<T> = unsafe { symbol(&libraries.rust, &format!("array_{prefix}_push")) };
    let get_c: Get<T> = unsafe { symbol(&libraries.c, &format!("array_{prefix}_get")) };
    let get_r: Get<T> = unsafe { symbol(&libraries.rust, &format!("array_{prefix}_get")) };
    let size_c: Size<T> = unsafe { symbol(&libraries.c, &format!("array_{prefix}_size")) };
    let size_r: Size<T> = unsafe { symbol(&libraries.rust, &format!("array_{prefix}_size")) };
    let clear_c: Clear<T> = unsafe { symbol(&libraries.c, &format!("array_{prefix}_clear")) };
    let clear_r: Clear<T> = unsafe { symbol(&libraries.rust, &format!("array_{prefix}_clear")) };

    let mut rng = Rng::new(seed);
    let empty_left = unsafe { create_c(0) };
    let empty_right = unsafe { create_r(0) };
    unsafe {
        clear_c(empty_left);
        clear_r(empty_right);
    }
    assert_eq!(unsafe { size_c(empty_left) }, unsafe {
        size_r(empty_right)
    });
    unsafe {
        destroy_c(empty_left);
        destroy_r(empty_right);
    }

    for case in 0..RANDOM_CASES {
        let capacity = if case % 8 == 0 {
            0
        } else {
            1 + (rng.next_u64() as usize % 9)
        };
        let left = unsafe { create_c(capacity) };
        let right = unsafe { create_r(capacity) };
        assert_eq!(left.is_null(), right.is_null());
        assert!(!left.is_null());
        assert_eq!(unsafe { size_c(left) }, unsafe { size_r(right) });
        assert_eq!(unsafe { (*left).capacity }, unsafe { (*right).capacity });

        let count = 18 + (rng.next_u64() as usize % 23);
        for _ in 0..count {
            let value = T::random(&mut rng);
            assert_eq!(unsafe { push_c(left, value) }, unsafe {
                push_r(right, value)
            });
            unsafe { assert_array_contents(left, right, get_c, get_r) };
        }

        unsafe {
            clear_c(left);
            clear_r(right);
        }
        assert_eq!(unsafe { size_c(left) }, 0);
        assert_eq!(unsafe { size_c(left) }, unsafe { size_r(right) });
        let reuse = T::random(&mut rng);
        assert_eq!(unsafe { push_c(left, reuse) }, unsafe {
            push_r(right, reuse)
        });
        unsafe { assert_array_contents(left, right, get_c, get_r) };
        unsafe {
            destroy_c(left);
            destroy_r(right);
        }
    }
}

unsafe fn collect_list<T: Copy>(list: *mut List<T>) -> Vec<T> {
    let mut values = Vec::new();
    let mut node = unsafe { (*list).head };
    while !node.is_null() {
        values.push(unsafe { (*node).data });
        node = unsafe { (*node).next };
    }
    values
}

unsafe fn assert_list_contents<T: TestValue>(left: *mut List<T>, right: *mut List<T>) {
    assert_eq!(unsafe { (*left).size }, unsafe { (*right).size });
    let left_values = unsafe { collect_list(left) };
    let right_values = unsafe { collect_list(right) };
    assert_eq!(left_values.len(), unsafe { (*left).size });
    assert_eq!(right_values.len(), unsafe { (*right).size });
    for (left_value, right_value) in left_values.into_iter().zip(right_values) {
        T::assert_same(left_value, right_value);
    }
    assert_eq!(unsafe { (*left).tail.is_null() }, unsafe {
        (*right).tail.is_null()
    });
    if !unsafe { (*left).tail.is_null() } {
        assert!(unsafe { (*(*left).tail).next.is_null() });
        assert!(unsafe { (*(*right).tail).next.is_null() });
    }
}

unsafe fn exercise_list<T: TestValue>(libraries: &Libraries, prefix: &str, seed: u64) {
    type Create<T> = unsafe extern "C" fn() -> *mut List<T>;
    type Destroy<T> = unsafe extern "C" fn(*mut List<T>);
    type Insert<T> = unsafe extern "C" fn(*mut List<T>, T) -> c_int;
    type Size<T> = unsafe extern "C" fn(*mut List<T>) -> usize;
    type Clear<T> = unsafe extern "C" fn(*mut List<T>);

    let create_c: Create<T> = unsafe { symbol(&libraries.c, &format!("list_{prefix}_create")) };
    let create_r: Create<T> = unsafe { symbol(&libraries.rust, &format!("list_{prefix}_create")) };
    let destroy_c: Destroy<T> = unsafe { symbol(&libraries.c, &format!("list_{prefix}_destroy")) };
    let destroy_r: Destroy<T> =
        unsafe { symbol(&libraries.rust, &format!("list_{prefix}_destroy")) };
    let append_c: Insert<T> = unsafe { symbol(&libraries.c, &format!("list_{prefix}_append")) };
    let append_r: Insert<T> = unsafe { symbol(&libraries.rust, &format!("list_{prefix}_append")) };
    let prepend_c: Insert<T> = unsafe { symbol(&libraries.c, &format!("list_{prefix}_prepend")) };
    let prepend_r: Insert<T> =
        unsafe { symbol(&libraries.rust, &format!("list_{prefix}_prepend")) };
    let size_c: Size<T> = unsafe { symbol(&libraries.c, &format!("list_{prefix}_size")) };
    let size_r: Size<T> = unsafe { symbol(&libraries.rust, &format!("list_{prefix}_size")) };
    let clear_c: Clear<T> = unsafe { symbol(&libraries.c, &format!("list_{prefix}_clear")) };
    let clear_r: Clear<T> = unsafe { symbol(&libraries.rust, &format!("list_{prefix}_clear")) };

    let mut rng = Rng::new(seed);
    let empty_left = unsafe { create_c() };
    let empty_right = unsafe { create_r() };
    unsafe {
        clear_c(empty_left);
        clear_r(empty_right);
    }
    assert_eq!(unsafe { size_c(empty_left) }, unsafe {
        size_r(empty_right)
    });
    unsafe {
        destroy_c(empty_left);
        destroy_r(empty_right);
    }

    for _ in 0..RANDOM_CASES {
        let left = unsafe { create_c() };
        let right = unsafe { create_r() };
        assert_eq!(left.is_null(), right.is_null());
        assert!(!left.is_null());
        assert_eq!(unsafe { size_c(left) }, unsafe { size_r(right) });

        for _ in 0..(16 + rng.next_u64() as usize % 17) {
            let value = T::random(&mut rng);
            let (left_result, right_result) = if rng.next_u64() & 1 == 0 {
                (unsafe { append_c(left, value) }, unsafe {
                    append_r(right, value)
                })
            } else {
                (unsafe { prepend_c(left, value) }, unsafe {
                    prepend_r(right, value)
                })
            };
            assert_eq!(left_result, right_result);
            assert_eq!(unsafe { size_c(left) }, unsafe { size_r(right) });
            unsafe { assert_list_contents(left, right) };
        }

        unsafe {
            clear_c(left);
            clear_r(right);
        }
        assert_eq!(unsafe { size_c(left) }, 0);
        assert_eq!(unsafe { size_c(left) }, unsafe { size_r(right) });
        let reuse = T::random(&mut rng);
        assert_eq!(unsafe { append_c(left, reuse) }, unsafe {
            append_r(right, reuse)
        });
        unsafe { assert_list_contents(left, right) };
        unsafe {
            destroy_c(left);
            destroy_r(right);
        }
    }
}

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn tmpfile() -> *mut c_void;
    fn fileno(stream: *mut c_void) -> c_int;
    fn fseek(stream: *mut c_void, offset: c_long, origin: c_int) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn rewind(stream: *mut c_void);
    fn fread(
        destination: *mut c_void,
        element_size: usize,
        count: usize,
        stream: *mut c_void,
    ) -> usize;
    fn fclose(stream: *mut c_void) -> c_int;
}

fn capture_stdout(action: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;
    const SEEK_END: c_int = 2;

    let _guard = STDOUT_LOCK.lock().expect("stdout lock");
    unsafe {
        fflush(ptr::null_mut());
        let saved = dup(STDOUT_FILENO);
        assert!(saved >= 0);
        let temporary = tmpfile();
        assert!(!temporary.is_null());
        assert_eq!(dup2(fileno(temporary), STDOUT_FILENO), STDOUT_FILENO);

        action();

        fflush(ptr::null_mut());
        assert_eq!(dup2(saved, STDOUT_FILENO), STDOUT_FILENO);
        close(saved);
        assert_eq!(fseek(temporary, 0, SEEK_END), 0);
        let length = ftell(temporary);
        assert!(length >= 0);
        rewind(temporary);
        let mut bytes = vec![0_u8; length as usize];
        assert_eq!(
            fread(bytes.as_mut_ptr().cast(), 1, bytes.len(), temporary),
            bytes.len()
        );
        fclose(temporary);
        bytes
    }
}

unsafe fn build_item_array(library: &Library, values: &[Item]) -> *mut Array<Item> {
    let create: unsafe extern "C" fn(usize) -> *mut Array<Item> =
        unsafe { symbol(library, "array_item_t_create") };
    let push: unsafe extern "C" fn(*mut Array<Item>, Item) -> c_int =
        unsafe { symbol(library, "array_item_t_push") };
    let array = unsafe { create(values.len().max(1)) };
    assert!(!array.is_null());
    for &value in values {
        assert_eq!(unsafe { push(array, value) }, 0);
    }
    array
}

unsafe fn build_item_list(library: &Library, values: &[Item]) -> *mut List<Item> {
    let create: unsafe extern "C" fn() -> *mut List<Item> =
        unsafe { symbol(library, "list_item_t_create") };
    let append: unsafe extern "C" fn(*mut List<Item>, Item) -> c_int =
        unsafe { symbol(library, "list_item_t_append") };
    let list = unsafe { create() };
    assert!(!list.is_null());
    for &value in values {
        assert_eq!(unsafe { append(list, value) }, 0);
    }
    list
}

unsafe fn build_order_list(library: &Library, values: &[Order]) -> *mut List<Order> {
    let create: unsafe extern "C" fn() -> *mut List<Order> =
        unsafe { symbol(library, "list_order_t_create") };
    let append: unsafe extern "C" fn(*mut List<Order>, Order) -> c_int =
        unsafe { symbol(library, "list_order_t_append") };
    let list = unsafe { create() };
    assert!(!list.is_null());
    for &value in values {
        assert_eq!(unsafe { append(list, value) }, 0);
    }
    list
}

#[test]
fn all_dynamic_symbols_load() {
    let symbols = fs::read_to_string(crate_root().join("SYMBOLS.md")).expect("read SYMBOLS.md");
    let names: Vec<_> = symbols
        .lines()
        .filter_map(|line| {
            let marker = line.find("`")?;
            let rest = &line[marker + 1..];
            let end = rest.find("`")?;
            let name = &rest[..end];
            (line.starts_with("| ") && !name.contains(' ')).then_some(name)
        })
        .collect();
    assert_eq!(names.len(), 56);

    let libraries = unsafe { Libraries::load() };
    for name in names {
        unsafe {
            libraries
                .c
                .get::<*mut c_void>(name.as_bytes())
                .unwrap_or_else(|error| panic!("C symbol {name}: {error}"));
            libraries
                .rust
                .get::<*mut c_void>(name.as_bytes())
                .unwrap_or_else(|error| panic!("Rust symbol {name}: {error}"));
        }
    }
}

#[test]
fn arrays_match_randomized() {
    let libraries = unsafe { Libraries::load() };
    unsafe {
        exercise_array::<c_int>(&libraries, "int", 0x1020_3040_5060_7080);
        exercise_array::<c_double>(&libraries, "double", 0x8877_6655_4433_2211);
        exercise_array::<Item>(&libraries, "item_t", 0x1234_5678_9abc_def0);
        exercise_array::<Order>(&libraries, "order_t", 0x0fed_cba9_8765_4321);
    }
}

#[test]
fn lists_match_randomized() {
    let libraries = unsafe { Libraries::load() };
    unsafe {
        exercise_list::<c_int>(&libraries, "int", 0xa102_3040_5060_7080);
        exercise_list::<c_double>(&libraries, "double", 0xb877_6655_4433_2211);
        exercise_list::<Item>(&libraries, "item_t", 0xc234_5678_9abc_def0);
        exercise_list::<Order>(&libraries, "order_t", 0xdfed_cba9_8765_4321);
    }
}

fn string_of_length(length: usize, offset: u8) -> CString {
    CString::new(
        (0..length)
            .map(|index| b'a' + ((index as u8).wrapping_add(offset) % 26))
            .collect::<Vec<_>>(),
    )
    .expect("generated C string")
}

#[test]
fn constructors_match_randomized() {
    type CreateItem =
        unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> Item;
    type CreateOrder = unsafe extern "C" fn(c_int, *const c_char, c_double) -> Order;

    let libraries = unsafe { Libraries::load() };
    let item_c: CreateItem = unsafe { symbol(&libraries.c, "create_item") };
    let item_r: CreateItem = unsafe { symbol(&libraries.rust, "create_item") };
    let order_c: CreateOrder = unsafe { symbol(&libraries.c, "create_order") };
    let order_r: CreateOrder = unsafe { symbol(&libraries.rust, "create_order") };
    let lengths = [0, 1, 30, 31, 32, 62, 63, 64, 127];
    let specials = [
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7ff8_1234_5678_9abc),
    ];
    let mut rng = Rng::new(0x3141_5926_5358_9793);

    for case in 0..RANDOM_CASES {
        for &name_length in &lengths {
            for &category_length in &lengths {
                let name = string_of_length(name_length, case as u8);
                let category = string_of_length(category_length, (case * 3) as u8);
                let price = if case < specials.len() {
                    specials[case]
                } else {
                    rng.next_f64()
                };
                let id = rng.next_i32();
                let quantity = rng.next_i32();
                Item::assert_same(
                    unsafe { item_c(id, name.as_ptr(), category.as_ptr(), price, quantity) },
                    unsafe { item_r(id, name.as_ptr(), category.as_ptr(), price, quantity) },
                );
            }

            let name = string_of_length(name_length, case as u8);
            let total = if case < specials.len() {
                specials[case]
            } else {
                rng.next_f64()
            };
            let customer_id = rng.next_i32();
            Order::assert_same(
                unsafe { order_c(customer_id, name.as_ptr(), total) },
                unsafe { order_r(customer_id, name.as_ptr(), total) },
            );
        }
    }
}

#[test]
fn print_functions_match_randomized() {
    type PrintItem = unsafe extern "C" fn(Item);
    type PrintOrder = unsafe extern "C" fn(Order);

    let libraries = unsafe { Libraries::load() };
    let print_item_c: PrintItem = unsafe { symbol(&libraries.c, "print_item") };
    let print_item_r: PrintItem = unsafe { symbol(&libraries.rust, "print_item") };
    let print_order_c: PrintOrder = unsafe { symbol(&libraries.c, "print_order") };
    let print_order_r: PrintOrder = unsafe { symbol(&libraries.rust, "print_order") };
    let mut rng = Rng::new(0x2718_2818_2845_9045);

    for _ in 0..RANDOM_CASES {
        let item = Item::random(&mut rng);
        assert_eq!(
            capture_stdout(|| unsafe { print_item_c(item) }),
            capture_stdout(|| unsafe { print_item_r(item) })
        );
        let order = Order::random(&mut rng);
        assert_eq!(
            capture_stdout(|| unsafe { print_order_c(order) }),
            capture_stdout(|| unsafe { print_order_r(order) })
        );
    }
}

#[test]
fn statistics_match_randomized() {
    type InventoryStats = unsafe extern "C" fn(*mut Array<Item>);
    type OrderStats = unsafe extern "C" fn(*mut List<Order>);

    let libraries = unsafe { Libraries::load() };
    let inventory_c: InventoryStats = unsafe { symbol(&libraries.c, "calculate_inventory_stats") };
    let inventory_r: InventoryStats =
        unsafe { symbol(&libraries.rust, "calculate_inventory_stats") };
    let orders_c: OrderStats = unsafe { symbol(&libraries.c, "calculate_order_stats") };
    let orders_r: OrderStats = unsafe { symbol(&libraries.rust, "calculate_order_stats") };
    let destroy_items_c: unsafe extern "C" fn(*mut Array<Item>) =
        unsafe { symbol(&libraries.c, "array_item_t_destroy") };
    let destroy_items_r: unsafe extern "C" fn(*mut Array<Item>) =
        unsafe { symbol(&libraries.rust, "array_item_t_destroy") };
    let destroy_orders_c: unsafe extern "C" fn(*mut List<Order>) =
        unsafe { symbol(&libraries.c, "list_order_t_destroy") };
    let destroy_orders_r: unsafe extern "C" fn(*mut List<Order>) =
        unsafe { symbol(&libraries.rust, "list_order_t_destroy") };
    let mut rng = Rng::new(0x1618_0339_8874_9894);

    for case in 0..RANDOM_CASES {
        let count = case % 17;
        let mut items = Vec::with_capacity(count);
        let mut orders = Vec::with_capacity(count);
        for index in 0..count {
            let mut item = Item::random(&mut rng);
            item.quantity = match (case + index) % 7 {
                0 => 0,
                1 => -1,
                _ => 1 + (rng.next_u64() % 1000) as i32,
            };
            item.price = match (case + index) % 11 {
                0 => 0.0,
                1 => -rng.next_f64().abs(),
                _ => rng.next_f64().abs(),
            };
            items.push(item);

            let mut order = Order::random(&mut rng);
            order.total_amount = match (case + index) % 5 {
                0 => -rng.next_f64().abs(),
                1 => 0.0,
                _ => rng.next_f64().abs(),
            };
            orders.push(order);
        }
        if count >= 3 {
            let low_index = match case % 3 {
                0 => 0,
                1 => count / 2,
                _ => count - 1,
            };
            let high_index = match case % 3 {
                0 => count - 1,
                1 => 0,
                _ => count / 2,
            };
            items[low_index].price = -1_000_000.0;
            items[high_index].price = 1_000_000.0;
            orders[low_index].total_amount = -1_000_000.0;
            orders[high_index].total_amount = 1_000_000.0;
        }

        let item_array_c = unsafe { build_item_array(&libraries.c, &items) };
        let item_array_r = unsafe { build_item_array(&libraries.rust, &items) };
        assert_eq!(
            capture_stdout(|| unsafe { inventory_c(item_array_c) }),
            capture_stdout(|| unsafe { inventory_r(item_array_r) })
        );
        unsafe {
            destroy_items_c(item_array_c);
            destroy_items_r(item_array_r);
        }

        let order_list_c = unsafe { build_order_list(&libraries.c, &orders) };
        let order_list_r = unsafe { build_order_list(&libraries.rust, &orders) };
        assert_eq!(
            capture_stdout(|| unsafe { orders_c(order_list_c) }),
            capture_stdout(|| unsafe { orders_r(order_list_r) })
        );
        unsafe {
            destroy_orders_c(order_list_c);
            destroy_orders_r(order_list_r);
        }
    }
}

#[test]
fn find_functions_match_randomized() {
    type FindCategory = unsafe extern "C" fn(*mut Array<Item>, *const c_char);
    type FindExpensive = unsafe extern "C" fn(*mut List<Item>, c_double);

    let libraries = unsafe { Libraries::load() };
    let category_c: FindCategory = unsafe { symbol(&libraries.c, "find_items_by_category") };
    let category_r: FindCategory = unsafe { symbol(&libraries.rust, "find_items_by_category") };
    let expensive_c: FindExpensive = unsafe { symbol(&libraries.c, "find_expensive_items") };
    let expensive_r: FindExpensive = unsafe { symbol(&libraries.rust, "find_expensive_items") };
    let destroy_array_c: unsafe extern "C" fn(*mut Array<Item>) =
        unsafe { symbol(&libraries.c, "array_item_t_destroy") };
    let destroy_array_r: unsafe extern "C" fn(*mut Array<Item>) =
        unsafe { symbol(&libraries.rust, "array_item_t_destroy") };
    let destroy_list_c: unsafe extern "C" fn(*mut List<Item>) =
        unsafe { symbol(&libraries.c, "list_item_t_destroy") };
    let destroy_list_r: unsafe extern "C" fn(*mut List<Item>) =
        unsafe { symbol(&libraries.rust, "list_item_t_destroy") };
    let mut rng = Rng::new(0x1414_2135_6237_3095);
    let categories = ["", "a", "alpha", "ALPHA", "category-with-31-bytes-xxxxxxx"];
    let missing_category = CString::new("__guaranteed_missing__").unwrap();

    for case in 0..RANDOM_CASES {
        let category = CString::new(categories[case % categories.len()]).unwrap();
        let mut items = Vec::new();
        for index in 0..(case % 13) {
            let mut item = Item::random(&mut rng);
            if index % 3 == 0 {
                item.category = [0; 32];
                let bytes = category.as_bytes();
                for (slot, byte) in item.category.iter_mut().zip(bytes.iter().copied()) {
                    *slot = byte as c_char;
                }
            }
            item.price = (index as f64) - 3.0;
            items.push(item);
        }

        let array_c = unsafe { build_item_array(&libraries.c, &items) };
        let array_r = unsafe { build_item_array(&libraries.rust, &items) };
        assert_eq!(
            capture_stdout(|| unsafe { category_c(array_c, category.as_ptr()) }),
            capture_stdout(|| unsafe { category_r(array_r, category.as_ptr()) })
        );
        assert_eq!(
            capture_stdout(|| unsafe { category_c(array_c, missing_category.as_ptr()) }),
            capture_stdout(|| unsafe { category_r(array_r, missing_category.as_ptr()) })
        );
        unsafe {
            destroy_array_c(array_c);
            destroy_array_r(array_r);
        }

        let threshold = match case % 8 {
            0 => f64::NEG_INFINITY,
            1 => f64::INFINITY,
            2 => f64::NAN,
            3 => -3.0,
            _ => (case as f64 % 9.0) - 4.0,
        };
        let list_c = unsafe { build_item_list(&libraries.c, &items) };
        let list_r = unsafe { build_item_list(&libraries.rust, &items) };
        assert_eq!(
            capture_stdout(|| unsafe { expensive_c(list_c, threshold) }),
            capture_stdout(|| unsafe { expensive_r(list_r, threshold) })
        );
        unsafe {
            destroy_list_c(list_c);
            destroy_list_r(list_r);
        }
    }
}

#[test]
fn null_and_oversized_error_paths_match() {
    let libraries = unsafe { Libraries::load() };

    macro_rules! null_array_paths {
        ($type:ty, $prefix:literal, $value:expr) => {{
            let push_c: unsafe extern "C" fn(*mut Array<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.c, concat!("array_", $prefix, "_push")) };
            let push_r: unsafe extern "C" fn(*mut Array<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.rust, concat!("array_", $prefix, "_push")) };
            let size_c: unsafe extern "C" fn(*mut Array<$type>) -> usize =
                unsafe { symbol(&libraries.c, concat!("array_", $prefix, "_size")) };
            let size_r: unsafe extern "C" fn(*mut Array<$type>) -> usize =
                unsafe { symbol(&libraries.rust, concat!("array_", $prefix, "_size")) };
            let clear_c: unsafe extern "C" fn(*mut Array<$type>) =
                unsafe { symbol(&libraries.c, concat!("array_", $prefix, "_clear")) };
            let clear_r: unsafe extern "C" fn(*mut Array<$type>) =
                unsafe { symbol(&libraries.rust, concat!("array_", $prefix, "_clear")) };
            let destroy_c: unsafe extern "C" fn(*mut Array<$type>) =
                unsafe { symbol(&libraries.c, concat!("array_", $prefix, "_destroy")) };
            let destroy_r: unsafe extern "C" fn(*mut Array<$type>) =
                unsafe { symbol(&libraries.rust, concat!("array_", $prefix, "_destroy")) };
            let create_c: unsafe extern "C" fn(usize) -> *mut Array<$type> =
                unsafe { symbol(&libraries.c, concat!("array_", $prefix, "_create")) };
            let create_r: unsafe extern "C" fn(usize) -> *mut Array<$type> =
                unsafe { symbol(&libraries.rust, concat!("array_", $prefix, "_create")) };

            assert_eq!(unsafe { push_c(ptr::null_mut(), $value) }, unsafe {
                push_r(ptr::null_mut(), $value)
            });
            assert_eq!(unsafe { size_c(ptr::null_mut()) }, unsafe {
                size_r(ptr::null_mut())
            });
            unsafe {
                clear_c(ptr::null_mut());
                clear_r(ptr::null_mut());
                destroy_c(ptr::null_mut());
                destroy_r(ptr::null_mut());
            }
            assert_eq!(unsafe { create_c(usize::MAX).is_null() }, unsafe {
                create_r(usize::MAX).is_null()
            });
        }};
    }

    macro_rules! null_list_paths {
        ($type:ty, $prefix:literal, $value:expr) => {{
            let append_c: unsafe extern "C" fn(*mut List<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_append")) };
            let append_r: unsafe extern "C" fn(*mut List<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_append")) };
            let prepend_c: unsafe extern "C" fn(*mut List<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_prepend")) };
            let prepend_r: unsafe extern "C" fn(*mut List<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_prepend")) };
            let size_c: unsafe extern "C" fn(*mut List<$type>) -> usize =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_size")) };
            let size_r: unsafe extern "C" fn(*mut List<$type>) -> usize =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_size")) };
            let clear_c: unsafe extern "C" fn(*mut List<$type>) =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_clear")) };
            let clear_r: unsafe extern "C" fn(*mut List<$type>) =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_clear")) };
            let destroy_c: unsafe extern "C" fn(*mut List<$type>) =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_destroy")) };
            let destroy_r: unsafe extern "C" fn(*mut List<$type>) =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_destroy")) };

            assert_eq!(unsafe { append_c(ptr::null_mut(), $value) }, unsafe {
                append_r(ptr::null_mut(), $value)
            });
            assert_eq!(unsafe { prepend_c(ptr::null_mut(), $value) }, unsafe {
                prepend_r(ptr::null_mut(), $value)
            });
            assert_eq!(unsafe { size_c(ptr::null_mut()) }, unsafe {
                size_r(ptr::null_mut())
            });
            unsafe {
                clear_c(ptr::null_mut());
                clear_r(ptr::null_mut());
                destroy_c(ptr::null_mut());
                destroy_r(ptr::null_mut());
            }
        }};
    }

    let item = Item::random(&mut Rng::new(1));
    let order = Order::random(&mut Rng::new(2));
    null_array_paths!(c_int, "int", 1);
    null_array_paths!(c_double, "double", 1.0);
    null_array_paths!(Item, "item_t", item);
    null_array_paths!(Order, "order_t", order);
    null_list_paths!(c_int, "int", 1);
    null_list_paths!(c_double, "double", 1.0);
    null_list_paths!(Item, "item_t", item);
    null_list_paths!(Order, "order_t", order);

    let inventory_c: unsafe extern "C" fn(*mut Array<Item>) =
        unsafe { symbol(&libraries.c, "calculate_inventory_stats") };
    let inventory_r: unsafe extern "C" fn(*mut Array<Item>) =
        unsafe { symbol(&libraries.rust, "calculate_inventory_stats") };
    assert_eq!(
        capture_stdout(|| unsafe { inventory_c(ptr::null_mut()) }),
        capture_stdout(|| unsafe { inventory_r(ptr::null_mut()) })
    );
    let orders_c: unsafe extern "C" fn(*mut List<Order>) =
        unsafe { symbol(&libraries.c, "calculate_order_stats") };
    let orders_r: unsafe extern "C" fn(*mut List<Order>) =
        unsafe { symbol(&libraries.rust, "calculate_order_stats") };
    assert_eq!(
        capture_stdout(|| unsafe { orders_c(ptr::null_mut()) }),
        capture_stdout(|| unsafe { orders_r(ptr::null_mut()) })
    );

    let category_c: unsafe extern "C" fn(*mut Array<Item>, *const c_char) =
        unsafe { symbol(&libraries.c, "find_items_by_category") };
    let category_r: unsafe extern "C" fn(*mut Array<Item>, *const c_char) =
        unsafe { symbol(&libraries.rust, "find_items_by_category") };
    let expensive_c: unsafe extern "C" fn(*mut List<Item>, c_double) =
        unsafe { symbol(&libraries.c, "find_expensive_items") };
    let expensive_r: unsafe extern "C" fn(*mut List<Item>, c_double) =
        unsafe { symbol(&libraries.rust, "find_expensive_items") };
    let category = CString::new("category").unwrap();
    let empty_c = unsafe { build_item_array(&libraries.c, &[]) };
    let empty_r = unsafe { build_item_array(&libraries.rust, &[]) };
    assert_eq!(
        capture_stdout(|| unsafe { category_c(ptr::null_mut(), category.as_ptr()) }),
        capture_stdout(|| unsafe { category_r(ptr::null_mut(), category.as_ptr()) })
    );
    assert_eq!(
        capture_stdout(|| unsafe { category_c(empty_c, ptr::null()) }),
        capture_stdout(|| unsafe { category_r(empty_r, ptr::null()) })
    );
    assert_eq!(
        capture_stdout(|| unsafe { expensive_c(ptr::null_mut(), 1.0) }),
        capture_stdout(|| unsafe { expensive_r(ptr::null_mut(), 1.0) })
    );
    let destroy_c: unsafe extern "C" fn(*mut Array<Item>) =
        unsafe { symbol(&libraries.c, "array_item_t_destroy") };
    let destroy_r: unsafe extern "C" fn(*mut Array<Item>) =
        unsafe { symbol(&libraries.rust, "array_item_t_destroy") };
    unsafe {
        destroy_c(empty_c);
        destroy_r(empty_r);
    }
}

fn allocator_shim_path() -> PathBuf {
    crate_root().join("target/fail_alloc.so")
}

fn build_allocator_shim(path: &Path) {
    let output = Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(crate_root().join("tests/fail_alloc.c"))
        .args(["-ldl", "-o"])
        .arg(path)
        .output()
        .expect("run C compiler for allocator shim");
    assert!(
        output.status.success(),
        "allocator shim build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn allocation_failures_match() {
    const CHILD_ENV: &str = "DIFFERENTIAL_ALLOCATOR_CHILD";
    let shim = allocator_shim_path();
    if env::var_os(CHILD_ENV).is_none() {
        build_allocator_shim(&shim);
        let output = Command::new(env::current_exe().expect("current test executable"))
            .args(["--exact", "allocation_failures_match", "--nocapture"])
            .env(CHILD_ENV, "1")
            .env("LD_PRELOAD", &shim)
            .output()
            .expect("run allocator-failure child");
        assert!(
            output.status.success(),
            "allocator child failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let libraries = unsafe { Libraries::load() };
    let shim_library = unsafe { Library::new(&shim) }.expect("open preloaded allocator shim");
    let fail_at: unsafe extern "C" fn(c_long) = unsafe { symbol(&shim_library, "fail_alloc_at") };
    let disable: unsafe extern "C" fn() = unsafe { symbol(&shim_library, "fail_alloc_disable") };

    macro_rules! allocation_array_paths {
        ($type:ty, $prefix:literal, $value:expr) => {{
            let create_c: unsafe extern "C" fn(usize) -> *mut Array<$type> =
                unsafe { symbol(&libraries.c, concat!("array_", $prefix, "_create")) };
            let create_r: unsafe extern "C" fn(usize) -> *mut Array<$type> =
                unsafe { symbol(&libraries.rust, concat!("array_", $prefix, "_create")) };
            let push_c: unsafe extern "C" fn(*mut Array<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.c, concat!("array_", $prefix, "_push")) };
            let push_r: unsafe extern "C" fn(*mut Array<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.rust, concat!("array_", $prefix, "_push")) };
            let destroy_c: unsafe extern "C" fn(*mut Array<$type>) =
                unsafe { symbol(&libraries.c, concat!("array_", $prefix, "_destroy")) };
            let destroy_r: unsafe extern "C" fn(*mut Array<$type>) =
                unsafe { symbol(&libraries.rust, concat!("array_", $prefix, "_destroy")) };

            unsafe { fail_at(0) };
            let metadata_c = unsafe { create_c(4) };
            unsafe { fail_at(0) };
            let metadata_r = unsafe { create_r(4) };
            assert!(metadata_c.is_null() && metadata_r.is_null());

            unsafe { fail_at(1) };
            let data_c = unsafe { create_c(4) };
            unsafe { fail_at(1) };
            let data_r = unsafe { create_r(4) };
            assert!(data_c.is_null() && data_r.is_null());

            unsafe { disable() };
            let array_c = unsafe { create_c(1) };
            let array_r = unsafe { create_r(1) };
            assert_eq!(unsafe { push_c(array_c, $value) }, 0);
            assert_eq!(unsafe { push_r(array_r, $value) }, 0);
            unsafe { fail_at(0) };
            let realloc_c = unsafe { push_c(array_c, $value) };
            unsafe { fail_at(0) };
            let realloc_r = unsafe { push_r(array_r, $value) };
            assert_eq!(realloc_c, -1);
            assert_eq!(realloc_c, realloc_r);
            assert_eq!(unsafe { (*array_c).size }, unsafe { (*array_r).size });
            assert_eq!(unsafe { (*array_c).capacity }, unsafe {
                (*array_r).capacity
            });
            unsafe {
                disable();
                destroy_c(array_c);
                destroy_r(array_r);
            }
        }};
    }

    macro_rules! allocation_list_paths {
        ($type:ty, $prefix:literal, $value:expr) => {{
            let create_c: unsafe extern "C" fn() -> *mut List<$type> =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_create")) };
            let create_r: unsafe extern "C" fn() -> *mut List<$type> =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_create")) };
            let append_c: unsafe extern "C" fn(*mut List<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_append")) };
            let append_r: unsafe extern "C" fn(*mut List<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_append")) };
            let prepend_c: unsafe extern "C" fn(*mut List<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_prepend")) };
            let prepend_r: unsafe extern "C" fn(*mut List<$type>, $type) -> c_int =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_prepend")) };
            let destroy_c: unsafe extern "C" fn(*mut List<$type>) =
                unsafe { symbol(&libraries.c, concat!("list_", $prefix, "_destroy")) };
            let destroy_r: unsafe extern "C" fn(*mut List<$type>) =
                unsafe { symbol(&libraries.rust, concat!("list_", $prefix, "_destroy")) };

            unsafe { fail_at(0) };
            let create_failure_c = unsafe { create_c() };
            unsafe { fail_at(0) };
            let create_failure_r = unsafe { create_r() };
            assert!(create_failure_c.is_null() && create_failure_r.is_null());

            unsafe { disable() };
            let list_c = unsafe { create_c() };
            let list_r = unsafe { create_r() };
            unsafe { fail_at(0) };
            let append_failure_c = unsafe { append_c(list_c, $value) };
            unsafe { fail_at(0) };
            let append_failure_r = unsafe { append_r(list_r, $value) };
            assert_eq!(append_failure_c, -1);
            assert_eq!(append_failure_c, append_failure_r);
            assert_eq!(unsafe { (*list_c).size }, unsafe { (*list_r).size });

            unsafe { fail_at(0) };
            let prepend_failure_c = unsafe { prepend_c(list_c, $value) };
            unsafe { fail_at(0) };
            let prepend_failure_r = unsafe { prepend_r(list_r, $value) };
            assert_eq!(prepend_failure_c, -1);
            assert_eq!(prepend_failure_c, prepend_failure_r);
            assert_eq!(unsafe { (*list_c).size }, unsafe { (*list_r).size });
            unsafe {
                disable();
                destroy_c(list_c);
                destroy_r(list_r);
            }
        }};
    }

    let item = Item::random(&mut Rng::new(3));
    let order = Order::random(&mut Rng::new(4));
    allocation_array_paths!(c_int, "int", 7);
    allocation_array_paths!(c_double, "double", 7.0);
    allocation_array_paths!(Item, "item_t", item);
    allocation_array_paths!(Order, "order_t", order);
    allocation_list_paths!(c_int, "int", 7);
    allocation_list_paths!(c_double, "double", 7.0);
    allocation_list_paths!(Item, "item_t", item);
    allocation_list_paths!(Order, "order_t", order);
    unsafe { disable() };
}

#[test]
fn ffi_layouts_match_c_platform_expectations() {
    assert_eq!(size_of::<Item>(), 120);
    assert_eq!(size_of::<Order>(), 80);
    assert_eq!(size_of::<Array<c_int>>(), 24);
    assert_eq!(size_of::<List<c_int>>(), 24);
}
