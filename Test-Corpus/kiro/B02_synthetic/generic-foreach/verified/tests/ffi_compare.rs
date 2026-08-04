use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;

const MAX_NAME_LENGTH: usize = 64;
const MAX_CATEGORY_LENGTH: usize = 32;

#[repr(C)]
#[derive(Clone, Copy)]
struct ItemT {
    id: c_int,
    name: [c_char; MAX_NAME_LENGTH],
    category: [c_char; MAX_CATEGORY_LENGTH],
    price: c_double,
    quantity: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct OrderT {
    customer_id: c_int,
    customer_name: [c_char; MAX_NAME_LENGTH],
    total_amount: c_double,
}

// Opaque pointer types
enum OpaqueArrayInt {}
enum OpaqueArrayDouble {}
enum OpaqueArrayItemT {}
enum OpaqueArrayOrderT {}
enum OpaqueListInt {}
enum OpaqueListDouble {}
enum OpaqueListItemT {}
enum OpaqueListOrderT {}

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn c_lib_path() -> String {
    std::env::current_dir().unwrap()
        .join("c_src/build/libgeneric_containers_c.so")
        .to_str().unwrap().to_string()
}

fn rust_lib_path() -> String {
    std::env::current_dir().unwrap()
        .join("target/debug/libgeneric_containers.so")
        .to_str().unwrap().to_string()
}

/// Capture stdout (fd 1) during a closure by redirecting to a pipe.
fn capture_stdout<F: FnOnce()>(f: F) -> String {
    std::io::stdout().flush().ok();
    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);

        f();

        // flush C and Rust stdout
        libc::fflush(std::ptr::null_mut());
        std::io::stdout().flush().ok();

        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut buf = String::new();
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        // Set non-blocking to avoid hanging, read what's available
        libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        // Small delay to let pipe buffer fill
        std::thread::sleep(std::time::Duration::from_millis(10));
        reader.read_to_string(&mut buf).ok();
        buf
    }
}

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

// ============================================================================
// LOW-LEVEL CONTAINER TESTS
// ============================================================================

#[test]
fn test_array_int_operations() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayInt> = c_lib.get(b"array_int_create").unwrap();
        let c_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt, c_int) -> c_int> = c_lib.get(b"array_int_push").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt, usize) -> c_int> = c_lib.get(b"array_int_get").unwrap();
        let c_size: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt) -> usize> = c_lib.get(b"array_int_size").unwrap();
        let c_clear: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt)> = c_lib.get(b"array_int_clear").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt)> = c_lib.get(b"array_int_destroy").unwrap();

        let r_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayInt> = r_lib.get(b"array_int_create").unwrap();
        let r_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt, c_int) -> c_int> = r_lib.get(b"array_int_push").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt, usize) -> c_int> = r_lib.get(b"array_int_get").unwrap();
        let r_size: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt) -> usize> = r_lib.get(b"array_int_size").unwrap();
        let r_clear: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt)> = r_lib.get(b"array_int_clear").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayInt)> = r_lib.get(b"array_int_destroy").unwrap();

        let ca = c_create(10);
        let ra = r_create(10);

        for v in [10, 20, 30, 40, 50] {
            assert_eq!(c_push(ca, v), r_push(ra, v));
        }
        assert_eq!(c_size(ca), r_size(ra));
        for i in 0..5 {
            assert_eq!(c_get(ca, i), r_get(ra, i), "mismatch at index {}", i);
        }
        c_clear(ca);
        r_clear(ra);
        assert_eq!(c_size(ca), r_size(ra));
        assert_eq!(c_size(ca), 0);

        c_destroy(ca);
        r_destroy(ra);
    }
}

#[test]
fn test_array_double_operations() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayDouble> = c_lib.get(b"array_double_create").unwrap();
        let c_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayDouble, c_double) -> c_int> = c_lib.get(b"array_double_push").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(*mut OpaqueArrayDouble, usize) -> c_double> = c_lib.get(b"array_double_get").unwrap();
        let c_size: Symbol<unsafe extern "C" fn(*mut OpaqueArrayDouble) -> usize> = c_lib.get(b"array_double_size").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayDouble)> = c_lib.get(b"array_double_destroy").unwrap();

        let r_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayDouble> = r_lib.get(b"array_double_create").unwrap();
        let r_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayDouble, c_double) -> c_int> = r_lib.get(b"array_double_push").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(*mut OpaqueArrayDouble, usize) -> c_double> = r_lib.get(b"array_double_get").unwrap();
        let r_size: Symbol<unsafe extern "C" fn(*mut OpaqueArrayDouble) -> usize> = r_lib.get(b"array_double_size").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayDouble)> = r_lib.get(b"array_double_destroy").unwrap();

        let ca = c_create(5);
        let ra = r_create(5);

        for v in [23.5, 25.0, 22.8, 26.3, 24.1] {
            assert_eq!(c_push(ca, v), r_push(ra, v));
        }
        assert_eq!(c_size(ca), r_size(ra));
        for i in 0..5 {
            assert_eq!(c_get(ca, i), r_get(ra, i), "mismatch at index {}", i);
        }

        c_destroy(ca);
        r_destroy(ra);
    }
}

#[test]
fn test_list_int_operations() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListInt> = c_lib.get(b"list_int_create").unwrap();
        let c_append: Symbol<unsafe extern "C" fn(*mut OpaqueListInt, c_int) -> c_int> = c_lib.get(b"list_int_append").unwrap();
        let c_prepend: Symbol<unsafe extern "C" fn(*mut OpaqueListInt, c_int) -> c_int> = c_lib.get(b"list_int_prepend").unwrap();
        let c_size: Symbol<unsafe extern "C" fn(*mut OpaqueListInt) -> usize> = c_lib.get(b"list_int_size").unwrap();
        let c_clear: Symbol<unsafe extern "C" fn(*mut OpaqueListInt)> = c_lib.get(b"list_int_clear").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListInt)> = c_lib.get(b"list_int_destroy").unwrap();

        let r_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListInt> = r_lib.get(b"list_int_create").unwrap();
        let r_append: Symbol<unsafe extern "C" fn(*mut OpaqueListInt, c_int) -> c_int> = r_lib.get(b"list_int_append").unwrap();
        let r_prepend: Symbol<unsafe extern "C" fn(*mut OpaqueListInt, c_int) -> c_int> = r_lib.get(b"list_int_prepend").unwrap();
        let r_size: Symbol<unsafe extern "C" fn(*mut OpaqueListInt) -> usize> = r_lib.get(b"list_int_size").unwrap();
        let r_clear: Symbol<unsafe extern "C" fn(*mut OpaqueListInt)> = r_lib.get(b"list_int_clear").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListInt)> = r_lib.get(b"list_int_destroy").unwrap();

        let cl = c_create();
        let rl = r_create();

        assert_eq!(c_append(cl, 10), r_append(rl, 10));
        assert_eq!(c_append(cl, 20), r_append(rl, 20));
        assert_eq!(c_prepend(cl, 5), r_prepend(rl, 5));
        assert_eq!(c_size(cl), r_size(rl));
        assert_eq!(c_size(cl), 3);

        c_clear(cl);
        r_clear(rl);
        assert_eq!(c_size(cl), r_size(rl));
        assert_eq!(c_size(cl), 0);

        c_destroy(cl);
        r_destroy(rl);
    }
}

#[test]
fn test_list_double_operations() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListDouble> = c_lib.get(b"list_double_create").unwrap();
        let c_append: Symbol<unsafe extern "C" fn(*mut OpaqueListDouble, c_double) -> c_int> = c_lib.get(b"list_double_append").unwrap();
        let c_size: Symbol<unsafe extern "C" fn(*mut OpaqueListDouble) -> usize> = c_lib.get(b"list_double_size").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListDouble)> = c_lib.get(b"list_double_destroy").unwrap();

        let r_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListDouble> = r_lib.get(b"list_double_create").unwrap();
        let r_append: Symbol<unsafe extern "C" fn(*mut OpaqueListDouble, c_double) -> c_int> = r_lib.get(b"list_double_append").unwrap();
        let r_size: Symbol<unsafe extern "C" fn(*mut OpaqueListDouble) -> usize> = r_lib.get(b"list_double_size").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListDouble)> = r_lib.get(b"list_double_destroy").unwrap();

        let cl = c_create();
        let rl = r_create();

        for v in [9.99, 14.50, 7.25] {
            assert_eq!(c_append(cl, v), r_append(rl, v));
        }
        assert_eq!(c_size(cl), r_size(rl));

        c_destroy(cl);
        r_destroy(rl);
    }
}

#[test]
fn test_create_item() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = c_lib.get(b"create_item").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = r_lib.get(b"create_item").unwrap();

        let name = cstr("Laptop");
        let cat = cstr("Electronics");
        let ci = c_fn(1, name.as_ptr(), cat.as_ptr(), 899.99, 15);
        let ri = r_fn(1, name.as_ptr(), cat.as_ptr(), 899.99, 15);

        assert_eq!(ci.id, ri.id);
        assert_eq!(ci.price, ri.price);
        assert_eq!(ci.quantity, ri.quantity);
        assert_eq!(&ci.name[..], &ri.name[..]);
        assert_eq!(&ci.category[..], &ri.category[..]);
    }
}

#[test]
fn test_create_order() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, *const c_char, c_double) -> OrderT> = c_lib.get(b"create_order").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, *const c_char, c_double) -> OrderT> = r_lib.get(b"create_order").unwrap();

        let name = cstr("Alice Johnson");
        let co = c_fn(1001, name.as_ptr(), 1249.95);
        let ro = r_fn(1001, name.as_ptr(), 1249.95);

        assert_eq!(co.customer_id, ro.customer_id);
        assert_eq!(co.total_amount, ro.total_amount);
        assert_eq!(&co.customer_name[..], &ro.customer_name[..]);
    }
}

#[test]
fn test_array_item_t_operations() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayItemT> = c_lib.get(b"array_item_t_create").unwrap();
        let c_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, ItemT) -> c_int> = c_lib.get(b"array_item_t_push").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, usize) -> ItemT> = c_lib.get(b"array_item_t_get").unwrap();
        let c_size: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT) -> usize> = c_lib.get(b"array_item_t_size").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = c_lib.get(b"array_item_t_destroy").unwrap();
        let c_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = c_lib.get(b"create_item").unwrap();

        let r_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayItemT> = r_lib.get(b"array_item_t_create").unwrap();
        let r_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, ItemT) -> c_int> = r_lib.get(b"array_item_t_push").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, usize) -> ItemT> = r_lib.get(b"array_item_t_get").unwrap();
        let r_size: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT) -> usize> = r_lib.get(b"array_item_t_size").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = r_lib.get(b"array_item_t_destroy").unwrap();
        let r_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = r_lib.get(b"create_item").unwrap();

        let ca = c_create(10);
        let ra = r_create(10);

        let name = cstr("Laptop");
        let cat = cstr("Electronics");
        let ci = c_create_item(1, name.as_ptr(), cat.as_ptr(), 899.99, 15);
        let ri = r_create_item(1, name.as_ptr(), cat.as_ptr(), 899.99, 15);

        assert_eq!(c_push(ca, ci), r_push(ra, ri));
        assert_eq!(c_size(ca), r_size(ra));

        let cg = c_get(ca, 0);
        let rg = r_get(ra, 0);
        assert_eq!(cg.id, rg.id);
        assert_eq!(cg.price, rg.price);
        assert_eq!(&cg.name[..], &rg.name[..]);

        c_destroy(ca);
        r_destroy(ra);
    }
}

#[test]
fn test_list_order_t_operations() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListOrderT> = c_lib.get(b"list_order_t_create").unwrap();
        let c_append: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT, OrderT) -> c_int> = c_lib.get(b"list_order_t_append").unwrap();
        let c_size: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT) -> usize> = c_lib.get(b"list_order_t_size").unwrap();
        let c_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT)> = c_lib.get(b"list_order_t_destroy").unwrap();
        let c_create_order: Symbol<unsafe extern "C" fn(c_int, *const c_char, c_double) -> OrderT> = c_lib.get(b"create_order").unwrap();

        let r_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListOrderT> = r_lib.get(b"list_order_t_create").unwrap();
        let r_append: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT, OrderT) -> c_int> = r_lib.get(b"list_order_t_append").unwrap();
        let r_size: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT) -> usize> = r_lib.get(b"list_order_t_size").unwrap();
        let r_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT)> = r_lib.get(b"list_order_t_destroy").unwrap();
        let r_create_order: Symbol<unsafe extern "C" fn(c_int, *const c_char, c_double) -> OrderT> = r_lib.get(b"create_order").unwrap();

        let cl = c_create();
        let rl = r_create();

        let name = cstr("Alice Johnson");
        let co = c_create_order(1001, name.as_ptr(), 1249.95);
        let ro = r_create_order(1001, name.as_ptr(), 1249.95);

        assert_eq!(c_append(cl, co), r_append(rl, ro));
        assert_eq!(c_size(cl), r_size(rl));

        c_destroy(cl);
        r_destroy(rl);
    }
}

// ============================================================================
// STDOUT-CAPTURING TESTS for print/stats/demo functions
// ============================================================================

#[test]
fn test_print_item() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = c_lib.get(b"create_item").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(ItemT)> = c_lib.get(b"print_item").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(ItemT)> = r_lib.get(b"print_item").unwrap();

        let name = cstr("Laptop");
        let cat = cstr("Electronics");
        let item = c_create_item(1, name.as_ptr(), cat.as_ptr(), 899.99, 15);

        let c_out = capture_stdout(|| { c_print(item); });
        let r_out = capture_stdout(|| { r_print(item); });
        assert_eq!(c_out, r_out, "print_item output mismatch:\nC:  {:?}\nRust: {:?}", c_out, r_out);
    }
}

#[test]
fn test_print_order() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_create_order: Symbol<unsafe extern "C" fn(c_int, *const c_char, c_double) -> OrderT> = c_lib.get(b"create_order").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(OrderT)> = c_lib.get(b"print_order").unwrap();
        let r_print: Symbol<unsafe extern "C" fn(OrderT)> = r_lib.get(b"print_order").unwrap();

        let name = cstr("Alice Johnson");
        let order = c_create_order(1001, name.as_ptr(), 1249.95);

        let c_out = capture_stdout(|| { c_print(order); });
        let r_out = capture_stdout(|| { r_print(order); });
        assert_eq!(c_out, r_out, "print_order output mismatch:\nC:  {:?}\nRust: {:?}", c_out, r_out);
    }
}

#[test]
fn test_print_menu() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn()> = c_lib.get(b"print_menu").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r_lib.get(b"print_menu").unwrap();

        let c_out = capture_stdout(|| { c_fn(); });
        let r_out = capture_stdout(|| { r_fn(); });
        assert_eq!(c_out, r_out, "print_menu output mismatch");
    }
}

#[test]
fn test_demo_integer_containers() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn()> = c_lib.get(b"demo_integer_containers").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r_lib.get(b"demo_integer_containers").unwrap();

        let c_out = capture_stdout(|| { c_fn(); });
        let r_out = capture_stdout(|| { r_fn(); });
        assert_eq!(c_out, r_out, "demo_integer_containers output mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);
    }
}

#[test]
fn test_demo_double_containers() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn()> = c_lib.get(b"demo_double_containers").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r_lib.get(b"demo_double_containers").unwrap();

        let c_out = capture_stdout(|| { c_fn(); });
        let r_out = capture_stdout(|| { r_fn(); });
        assert_eq!(c_out, r_out, "demo_double_containers output mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);
    }
}

#[test]
fn test_demo_inventory_array() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn()> = c_lib.get(b"demo_inventory_array").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r_lib.get(b"demo_inventory_array").unwrap();

        let c_out = capture_stdout(|| { c_fn(); });
        let r_out = capture_stdout(|| { r_fn(); });
        assert_eq!(c_out, r_out, "demo_inventory_array output mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);
    }
}

#[test]
fn test_demo_order_list() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn()> = c_lib.get(b"demo_order_list").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r_lib.get(b"demo_order_list").unwrap();

        let c_out = capture_stdout(|| { c_fn(); });
        let r_out = capture_stdout(|| { r_fn(); });
        assert_eq!(c_out, r_out, "demo_order_list output mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);
    }
}

#[test]
fn test_demo_mixed_operations() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn()> = c_lib.get(b"demo_mixed_operations").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn()> = r_lib.get(b"demo_mixed_operations").unwrap();

        let c_out = capture_stdout(|| { c_fn(); });
        let r_out = capture_stdout(|| { r_fn(); });
        assert_eq!(c_out, r_out, "demo_mixed_operations output mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);
    }
}

#[test]
fn test_calculate_inventory_stats() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_arr_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayItemT> = c_lib.get(b"array_item_t_create").unwrap();
        let c_arr_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, ItemT) -> c_int> = c_lib.get(b"array_item_t_push").unwrap();
        let c_arr_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = c_lib.get(b"array_item_t_destroy").unwrap();
        let c_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = c_lib.get(b"create_item").unwrap();
        let c_stats: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = c_lib.get(b"calculate_inventory_stats").unwrap();

        let r_arr_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayItemT> = r_lib.get(b"array_item_t_create").unwrap();
        let r_arr_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, ItemT) -> c_int> = r_lib.get(b"array_item_t_push").unwrap();
        let r_arr_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = r_lib.get(b"array_item_t_destroy").unwrap();
        let r_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = r_lib.get(b"create_item").unwrap();
        let r_stats: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = r_lib.get(b"calculate_inventory_stats").unwrap();

        let ca = c_arr_create(10);
        let ra = r_arr_create(10);

        let items_data: &[(i32, &str, &str, f64, i32)] = &[
            (1, "Laptop", "Electronics", 899.99, 15),
            (2, "Mouse", "Electronics", 24.99, 50),
            (3, "Desk", "Furniture", 349.99, 8),
        ];
        for &(id, name, cat, price, qty) in items_data {
            let n = cstr(name);
            let c = cstr(cat);
            c_arr_push(ca, c_create_item(id, n.as_ptr(), c.as_ptr(), price, qty));
            r_arr_push(ra, r_create_item(id, n.as_ptr(), c.as_ptr(), price, qty));
        }

        let c_out = capture_stdout(|| { c_stats(ca); });
        let r_out = capture_stdout(|| { r_stats(ra); });
        assert_eq!(c_out, r_out, "calculate_inventory_stats mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);

        c_arr_destroy(ca);
        r_arr_destroy(ra);
    }
}

#[test]
fn test_calculate_order_stats() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_list_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListOrderT> = c_lib.get(b"list_order_t_create").unwrap();
        let c_list_append: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT, OrderT) -> c_int> = c_lib.get(b"list_order_t_append").unwrap();
        let c_list_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT)> = c_lib.get(b"list_order_t_destroy").unwrap();
        let c_create_order: Symbol<unsafe extern "C" fn(c_int, *const c_char, c_double) -> OrderT> = c_lib.get(b"create_order").unwrap();
        let c_stats: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT)> = c_lib.get(b"calculate_order_stats").unwrap();

        let r_list_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListOrderT> = r_lib.get(b"list_order_t_create").unwrap();
        let r_list_append: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT, OrderT) -> c_int> = r_lib.get(b"list_order_t_append").unwrap();
        let r_list_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT)> = r_lib.get(b"list_order_t_destroy").unwrap();
        let r_create_order: Symbol<unsafe extern "C" fn(c_int, *const c_char, c_double) -> OrderT> = r_lib.get(b"create_order").unwrap();
        let r_stats: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT)> = r_lib.get(b"calculate_order_stats").unwrap();

        let cl = c_list_create();
        let rl = r_list_create();

        let orders_data: &[(i32, &str, f64)] = &[
            (1001, "Alice", 1249.95),
            (1002, "Bob", 89.99),
        ];
        for &(id, name, amt) in orders_data {
            let n = cstr(name);
            c_list_append(cl, c_create_order(id, n.as_ptr(), amt));
            r_list_append(rl, r_create_order(id, n.as_ptr(), amt));
        }

        let c_out = capture_stdout(|| { c_stats(cl); });
        let r_out = capture_stdout(|| { r_stats(rl); });
        assert_eq!(c_out, r_out, "calculate_order_stats mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);

        c_list_destroy(cl);
        r_list_destroy(rl);
    }
}

#[test]
fn test_find_items_by_category() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_arr_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayItemT> = c_lib.get(b"array_item_t_create").unwrap();
        let c_arr_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, ItemT) -> c_int> = c_lib.get(b"array_item_t_push").unwrap();
        let c_arr_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = c_lib.get(b"array_item_t_destroy").unwrap();
        let c_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = c_lib.get(b"create_item").unwrap();
        let c_find: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, *const c_char)> = c_lib.get(b"find_items_by_category").unwrap();

        let r_arr_create: Symbol<unsafe extern "C" fn(usize) -> *mut OpaqueArrayItemT> = r_lib.get(b"array_item_t_create").unwrap();
        let r_arr_push: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, ItemT) -> c_int> = r_lib.get(b"array_item_t_push").unwrap();
        let r_arr_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = r_lib.get(b"array_item_t_destroy").unwrap();
        let r_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = r_lib.get(b"create_item").unwrap();
        let r_find: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT, *const c_char)> = r_lib.get(b"find_items_by_category").unwrap();

        let ca = c_arr_create(10);
        let ra = r_arr_create(10);

        let n1 = cstr("Laptop"); let c1 = cstr("Electronics");
        let n2 = cstr("Desk"); let c2 = cstr("Furniture");
        c_arr_push(ca, c_create_item(1, n1.as_ptr(), c1.as_ptr(), 899.99, 15));
        c_arr_push(ca, c_create_item(2, n2.as_ptr(), c2.as_ptr(), 349.99, 8));
        r_arr_push(ra, r_create_item(1, n1.as_ptr(), c1.as_ptr(), 899.99, 15));
        r_arr_push(ra, r_create_item(2, n2.as_ptr(), c2.as_ptr(), 349.99, 8));

        let cat = cstr("Electronics");
        let c_out = capture_stdout(|| { c_find(ca, cat.as_ptr()); });
        let r_out = capture_stdout(|| { r_find(ra, cat.as_ptr()); });
        assert_eq!(c_out, r_out, "find_items_by_category mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);

        c_arr_destroy(ca);
        r_arr_destroy(ra);
    }
}

#[test]
fn test_find_expensive_items() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_list_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListItemT> = c_lib.get(b"list_item_t_create").unwrap();
        let c_list_append: Symbol<unsafe extern "C" fn(*mut OpaqueListItemT, ItemT) -> c_int> = c_lib.get(b"list_item_t_append").unwrap();
        let c_list_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListItemT)> = c_lib.get(b"list_item_t_destroy").unwrap();
        let c_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = c_lib.get(b"create_item").unwrap();
        let c_find: Symbol<unsafe extern "C" fn(*mut OpaqueListItemT, c_double)> = c_lib.get(b"find_expensive_items").unwrap();

        let r_list_create: Symbol<unsafe extern "C" fn() -> *mut OpaqueListItemT> = r_lib.get(b"list_item_t_create").unwrap();
        let r_list_append: Symbol<unsafe extern "C" fn(*mut OpaqueListItemT, ItemT) -> c_int> = r_lib.get(b"list_item_t_append").unwrap();
        let r_list_destroy: Symbol<unsafe extern "C" fn(*mut OpaqueListItemT)> = r_lib.get(b"list_item_t_destroy").unwrap();
        let r_create_item: Symbol<unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> ItemT> = r_lib.get(b"create_item").unwrap();
        let r_find: Symbol<unsafe extern "C" fn(*mut OpaqueListItemT, c_double)> = r_lib.get(b"find_expensive_items").unwrap();

        let cl = c_list_create();
        let rl = r_list_create();

        let n1 = cstr("Laptop"); let c1 = cstr("Electronics");
        let n2 = cstr("Mouse"); let c2 = cstr("Electronics");
        c_list_append(cl, c_create_item(1, n1.as_ptr(), c1.as_ptr(), 899.99, 15));
        c_list_append(cl, c_create_item(2, n2.as_ptr(), c2.as_ptr(), 24.99, 50));
        r_list_append(rl, r_create_item(1, n1.as_ptr(), c1.as_ptr(), 899.99, 15));
        r_list_append(rl, r_create_item(2, n2.as_ptr(), c2.as_ptr(), 24.99, 50));

        let c_out = capture_stdout(|| { c_find(cl, 100.0); });
        let r_out = capture_stdout(|| { r_find(rl, 100.0); });
        assert_eq!(c_out, r_out, "find_expensive_items mismatch:\nC:\n{}\nRust:\n{}", c_out, r_out);

        c_list_destroy(cl);
        r_list_destroy(rl);
    }
}

#[test]
fn test_calculate_inventory_stats_empty() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_stats: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = c_lib.get(b"calculate_inventory_stats").unwrap();
        let r_stats: Symbol<unsafe extern "C" fn(*mut OpaqueArrayItemT)> = r_lib.get(b"calculate_inventory_stats").unwrap();

        // Test with NULL pointer
        let c_out = capture_stdout(|| { c_stats(std::ptr::null_mut()); });
        let r_out = capture_stdout(|| { r_stats(std::ptr::null_mut()); });
        assert_eq!(c_out, r_out, "calculate_inventory_stats(NULL) mismatch");
    }
}

#[test]
fn test_calculate_order_stats_empty() {
    let _lock = TEST_MUTEX.lock().unwrap();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_stats: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT)> = c_lib.get(b"calculate_order_stats").unwrap();
        let r_stats: Symbol<unsafe extern "C" fn(*mut OpaqueListOrderT)> = r_lib.get(b"calculate_order_stats").unwrap();

        let c_out = capture_stdout(|| { c_stats(std::ptr::null_mut()); });
        let r_out = capture_stdout(|| { r_stats(std::ptr::null_mut()); });
        assert_eq!(c_out, r_out, "calculate_order_stats(NULL) mismatch");
    }
}
