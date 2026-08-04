// FFI conformance tests: inventory functions (create_*, print_*, calculate_*,
// find_*). For functions that print, we capture stdout and require byte-for-byte
// equality between the C and Rust implementations.

mod common;

use common::*;
use libloading::Library;
use std::os::raw::{c_char, c_double, c_int};

type CreateItemFn =
    unsafe extern "C" fn(c_int, *const c_char, *const c_char, c_double, c_int) -> item_t;
type CreateOrderFn = unsafe extern "C" fn(c_int, *const c_char, c_double) -> order_t;

type PrintItemFn = unsafe extern "C" fn(item_t);
type PrintOrderFn = unsafe extern "C" fn(order_t);

type ArrItemCreate = unsafe extern "C" fn(size_t) -> *mut array_item_t_t;
type ArrItemDestroy = unsafe extern "C" fn(*mut array_item_t_t);
type ArrItemPush = unsafe extern "C" fn(*mut array_item_t_t, item_t) -> c_int;

type ListOrderCreate = unsafe extern "C" fn() -> *mut list_order_t_t;
type ListOrderDestroy = unsafe extern "C" fn(*mut list_order_t_t);
type ListOrderAppend = unsafe extern "C" fn(*mut list_order_t_t, order_t) -> c_int;

type ListItemCreate = unsafe extern "C" fn() -> *mut list_item_t_t;
type ListItemDestroy = unsafe extern "C" fn(*mut list_item_t_t);
type ListItemAppend = unsafe extern "C" fn(*mut list_item_t_t, item_t) -> c_int;

type CalcInventoryStats = unsafe extern "C" fn(*mut array_item_t_t);
type CalcOrderStats = unsafe extern "C" fn(*mut list_order_t_t);
type FindByCategory = unsafe extern "C" fn(*mut array_item_t_t, *const c_char);
type FindExpensive = unsafe extern "C" fn(*mut list_item_t_t, c_double);

fn item_fp(it: &item_t) -> (c_int, Vec<u8>, Vec<u8>, u64, c_int) {
    (
        it.id,
        cstr_slice(&it.name).to_vec(),
        cstr_slice(&it.category).to_vec(),
        it.price.to_bits(),
        it.quantity,
    )
}

fn order_fp(o: &order_t) -> (c_int, Vec<u8>, u64) {
    (o.customer_id, cstr_slice(&o.customer_name).to_vec(), o.total_amount.to_bits())
}

fn run_create_item(
    lib: &Library,
    id: c_int,
    name: &str,
    cat: &str,
    price: c_double,
    qty: c_int,
) -> (c_int, Vec<u8>, Vec<u8>, u64, c_int) {
    unsafe {
        let f = sym::<CreateItemFn>(lib, b"create_item");
        let cn = std::ffi::CString::new(name).unwrap();
        let cc = std::ffi::CString::new(cat).unwrap();
        let it = f(id, cn.as_ptr(), cc.as_ptr(), price, qty);
        item_fp(&it)
    }
}

#[test]
fn create_item_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();
    let cases: &[(c_int, &str, &str, c_double, c_int)] = &[
        (1, "Laptop", "Electronics", 899.99, 15),
        (2, "Mouse", "Electronics", 24.99, 50),
        (10, "Bookshelf", "Furniture", 149.99, 12),
        // Empty strings
        (0, "", "", 0.0, 0),
        // Negative price/quantity
        (-5, "Refund", "Misc", -1.5, -7),
        // Long names that will get truncated to MAX_NAME_LENGTH-1
        (
            42,
            "this is a really really really long item name that is more than sixty four characters",
            "this category name is also rather long, more than thirty two characters",
            12345.6789,
            999,
        ),
    ];
    for &(id, n, c, p, q) in cases {
        let c_b = run_create_item(&c_lib, id, n, c, p, q);
        let r_b = run_create_item(&r_lib, id, n, c, p, q);
        assert_eq!(c_b, r_b, "mismatch for case {:?}", (id, n, c, p, q));
    }
}

fn run_create_order(lib: &Library, id: c_int, name: &str, amt: c_double) -> (c_int, Vec<u8>, u64) {
    unsafe {
        let f = sym::<CreateOrderFn>(lib, b"create_order");
        let cn = std::ffi::CString::new(name).unwrap();
        let o = f(id, cn.as_ptr(), amt);
        order_fp(&o)
    }
}

#[test]
fn create_order_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();
    let cases: &[(c_int, &str, c_double)] = &[
        (1001, "Alice Johnson", 1249.95),
        (1002, "Bob Smith", 89.99),
        (0, "", 0.0),
        (-1, "Refund", -50.0),
        (
            7777,
            "this customer has an absurdly long name that won't fit in 64 bytes inclusive of NUL",
            999999.999,
        ),
    ];
    for &(id, n, a) in cases {
        let c_b = run_create_order(&c_lib, id, n, a);
        let r_b = run_create_order(&r_lib, id, n, a);
        assert_eq!(c_b, r_b);
    }
}

// --- print_item / print_order: stdout match -------------------------------

fn make_item_via(lib: &Library, id: c_int, name: &str, cat: &str, price: c_double, qty: c_int)
    -> item_t
{
    unsafe {
        let f = sym::<CreateItemFn>(lib, b"create_item");
        let cn = std::ffi::CString::new(name).unwrap();
        let cc = std::ffi::CString::new(cat).unwrap();
        f(id, cn.as_ptr(), cc.as_ptr(), price, qty)
    }
}

fn make_order_via(lib: &Library, id: c_int, name: &str, amt: c_double) -> order_t {
    unsafe {
        let f = sym::<CreateOrderFn>(lib, b"create_order");
        let cn = std::ffi::CString::new(name).unwrap();
        f(id, cn.as_ptr(), amt)
    }
}

#[test]
fn print_item_stdout_matches() {
    let c_lib = load_c();
    let r_lib = load_rust();

    let cases: &[(c_int, &str, &str, c_double, c_int)] = &[
        (1, "Laptop", "Electronics", 899.99, 15),
        (10, "Bookshelf", "Furniture", 149.99, 12),
        (-5, "Refund", "Misc", -1.5, -7),
        (0, "", "", 0.0, 0),
    ];

    for &(id, n, c, p, q) in cases {
        // Build the item using the C lib so we know exactly what bytes are
        // in the buffer.
        let it = make_item_via(&c_lib, id, n, c, p, q);

        let c_out = capture_stdout(|| unsafe {
            let f = sym::<PrintItemFn>(&c_lib, b"print_item");
            f(it);
        });
        let r_out = capture_stdout(|| unsafe {
            let f = sym::<PrintItemFn>(&r_lib, b"print_item");
            f(it);
        });
        assert_eq!(c_out, r_out, "print_item mismatch for {:?}", (id, n, c, p, q));
    }
}

#[test]
fn print_order_stdout_matches() {
    let c_lib = load_c();
    let r_lib = load_rust();

    let cases: &[(c_int, &str, c_double)] = &[
        (1001, "Alice Johnson", 1249.95),
        (1004, "David Brown", 24.99),
        (-1, "Refund", -50.0),
        (0, "", 0.0),
    ];
    for &(id, n, a) in cases {
        let o = make_order_via(&c_lib, id, n, a);
        let c_out = capture_stdout(|| unsafe {
            let f = sym::<PrintOrderFn>(&c_lib, b"print_order");
            f(o);
        });
        let r_out = capture_stdout(|| unsafe {
            let f = sym::<PrintOrderFn>(&r_lib, b"print_order");
            f(o);
        });
        assert_eq!(c_out, r_out, "print_order mismatch for {:?}", (id, n, a));
    }
}

// --- calculate_inventory_stats --------------------------------------------

fn populate_array(lib: &Library, items: &[(c_int, &str, &str, c_double, c_int)])
    -> *mut array_item_t_t
{
    unsafe {
        let create = sym::<ArrItemCreate>(lib, b"array_item_t_create");
        let push = sym::<ArrItemPush>(lib, b"array_item_t_push");
        let arr = create(items.len().max(1));
        for &(id, n, c, p, q) in items {
            let it = make_item_via(lib, id, n, c, p, q);
            assert_eq!(push(arr, it), 0);
        }
        arr
    }
}

#[test]
fn calculate_inventory_stats_matches() {
    let c_lib = load_c();
    let r_lib = load_rust();

    let cases: Vec<Vec<(c_int, &str, &str, c_double, c_int)>> = vec![
        // Empty
        vec![],
        // The same items as the C demo
        vec![
            (1, "Laptop", "Electronics", 899.99, 15),
            (2, "Mouse", "Electronics", 24.99, 50),
            (3, "Keyboard", "Electronics", 79.99, 30),
            (4, "Monitor", "Electronics", 299.99, 20),
            (5, "Desk Chair", "Furniture", 199.99, 10),
            (6, "Desk", "Furniture", 349.99, 8),
            (7, "Notebook", "Office", 4.99, 100),
            (8, "Pen Set", "Office", 12.99, 75),
            (9, "USB Cable", "Electronics", 9.99, 60),
            (10, "Bookshelf", "Furniture", 149.99, 12),
        ],
        // Single item with quantity 1
        vec![(1, "Solo", "Solo", 5.5, 1)],
    ];

    for items in &cases {
        let c_arr = populate_array(&c_lib, items);
        let r_arr = populate_array(&r_lib, items);

        let c_out = capture_stdout(|| unsafe {
            let f = sym::<CalcInventoryStats>(&c_lib, b"calculate_inventory_stats");
            f(c_arr);
        });
        let r_out = capture_stdout(|| unsafe {
            let f = sym::<CalcInventoryStats>(&r_lib, b"calculate_inventory_stats");
            f(r_arr);
        });
        assert_eq!(c_out, r_out, "calculate_inventory_stats mismatch (n={})", items.len());

        unsafe {
            let cdest = sym::<ArrItemDestroy>(&c_lib, b"array_item_t_destroy");
            cdest(c_arr);
            let rdest = sym::<ArrItemDestroy>(&r_lib, b"array_item_t_destroy");
            rdest(r_arr);
        }
    }

    // NULL array
    let c_out = capture_stdout(|| unsafe {
        let f = sym::<CalcInventoryStats>(&c_lib, b"calculate_inventory_stats");
        f(std::ptr::null_mut());
    });
    let r_out = capture_stdout(|| unsafe {
        let f = sym::<CalcInventoryStats>(&r_lib, b"calculate_inventory_stats");
        f(std::ptr::null_mut());
    });
    assert_eq!(c_out, r_out);
}

// --- calculate_order_stats -------------------------------------------------

fn populate_order_list(lib: &Library, orders: &[(c_int, &str, c_double)]) -> *mut list_order_t_t {
    unsafe {
        let create = sym::<ListOrderCreate>(lib, b"list_order_t_create");
        let append = sym::<ListOrderAppend>(lib, b"list_order_t_append");
        let list = create();
        for &(id, n, a) in orders {
            let o = make_order_via(lib, id, n, a);
            assert_eq!(append(list, o), 0);
        }
        list
    }
}

#[test]
fn calculate_order_stats_matches() {
    let c_lib = load_c();
    let r_lib = load_rust();

    let cases: Vec<Vec<(c_int, &str, c_double)>> = vec![
        vec![],
        vec![
            (1001, "Alice Johnson", 1249.95),
            (1002, "Bob Smith", 89.99),
            (1003, "Carol White", 549.98),
            (1004, "David Brown", 24.99),
            (1005, "Eve Davis", 899.99),
            (1006, "Frank Miller", 374.97),
            (1007, "Grace Lee", 159.98),
            (1008, "Henry Wilson", 1099.99),
        ],
        vec![(1, "Solo", 100.0)],
    ];

    for orders in &cases {
        let c_l = populate_order_list(&c_lib, orders);
        let r_l = populate_order_list(&r_lib, orders);

        let c_out = capture_stdout(|| unsafe {
            let f = sym::<CalcOrderStats>(&c_lib, b"calculate_order_stats");
            f(c_l);
        });
        let r_out = capture_stdout(|| unsafe {
            let f = sym::<CalcOrderStats>(&r_lib, b"calculate_order_stats");
            f(r_l);
        });
        assert_eq!(c_out, r_out, "calculate_order_stats mismatch (n={})", orders.len());

        unsafe {
            let cdest = sym::<ListOrderDestroy>(&c_lib, b"list_order_t_destroy");
            cdest(c_l);
            let rdest = sym::<ListOrderDestroy>(&r_lib, b"list_order_t_destroy");
            rdest(r_l);
        }
    }

    // NULL list
    let c_out = capture_stdout(|| unsafe {
        let f = sym::<CalcOrderStats>(&c_lib, b"calculate_order_stats");
        f(std::ptr::null_mut());
    });
    let r_out = capture_stdout(|| unsafe {
        let f = sym::<CalcOrderStats>(&r_lib, b"calculate_order_stats");
        f(std::ptr::null_mut());
    });
    assert_eq!(c_out, r_out);
}

// --- find_items_by_category -----------------------------------------------

#[test]
fn find_items_by_category_matches() {
    let c_lib = load_c();
    let r_lib = load_rust();

    let items = vec![
        (1, "Laptop", "Electronics", 899.99, 15),
        (2, "Mouse", "Electronics", 24.99, 50),
        (5, "Desk Chair", "Furniture", 199.99, 10),
        (6, "Desk", "Furniture", 349.99, 8),
        (7, "Notebook", "Office", 4.99, 100),
    ];
    let c_arr = populate_array(&c_lib, &items);
    let r_arr = populate_array(&r_lib, &items);

    for &cat in &["Electronics", "Furniture", "Office", "DoesNotExist", ""] {
        let cn = std::ffi::CString::new(cat).unwrap();

        let c_out = capture_stdout(|| unsafe {
            let f = sym::<FindByCategory>(&c_lib, b"find_items_by_category");
            f(c_arr, cn.as_ptr());
        });
        let r_out = capture_stdout(|| unsafe {
            let f = sym::<FindByCategory>(&r_lib, b"find_items_by_category");
            f(r_arr, cn.as_ptr());
        });
        assert_eq!(c_out, r_out, "category {:?}", cat);
    }

    unsafe {
        let cdest = sym::<ArrItemDestroy>(&c_lib, b"array_item_t_destroy");
        cdest(c_arr);
        let rdest = sym::<ArrItemDestroy>(&r_lib, b"array_item_t_destroy");
        rdest(r_arr);
    }
}

// --- find_expensive_items --------------------------------------------------

fn populate_item_list(lib: &Library, items: &[(c_int, &str, &str, c_double, c_int)])
    -> *mut list_item_t_t
{
    unsafe {
        let create = sym::<ListItemCreate>(lib, b"list_item_t_create");
        let append = sym::<ListItemAppend>(lib, b"list_item_t_append");
        let list = create();
        for &(id, n, c, p, q) in items {
            let it = make_item_via(lib, id, n, c, p, q);
            assert_eq!(append(list, it), 0);
        }
        list
    }
}

#[test]
fn find_expensive_items_matches() {
    let c_lib = load_c();
    let r_lib = load_rust();

    let items = vec![
        (1, "Laptop", "Electronics", 899.99, 15),
        (2, "Mouse", "Electronics", 24.99, 50),
        (5, "Desk Chair", "Furniture", 199.99, 10),
        (6, "Desk", "Furniture", 349.99, 8),
        (7, "Notebook", "Office", 4.99, 100),
    ];
    let c_l = populate_item_list(&c_lib, &items);
    let r_l = populate_item_list(&r_lib, &items);

    for &thresh in &[0.0_f64, 100.0, 200.0, 1000.0, -10.0] {
        let c_out = capture_stdout(|| unsafe {
            let f = sym::<FindExpensive>(&c_lib, b"find_expensive_items");
            f(c_l, thresh);
        });
        let r_out = capture_stdout(|| unsafe {
            let f = sym::<FindExpensive>(&r_lib, b"find_expensive_items");
            f(r_l, thresh);
        });
        assert_eq!(c_out, r_out, "thresh {}", thresh);
    }

    unsafe {
        let cdest = sym::<ListItemDestroy>(&c_lib, b"list_item_t_destroy");
        cdest(c_l);
        let rdest = sym::<ListItemDestroy>(&r_lib, b"list_item_t_destroy");
        rdest(r_l);
    }
}

// --- symbol parity --------------------------------------------------------

#[test]
fn rust_lib_exports_all_c_symbols() {
    use std::process::Command;
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    let collect = |p: &std::path::PathBuf| -> Vec<String> {
        let out = Command::new("nm").arg("-D").arg("--defined-only").arg(p).output();
        let out = match out {
            Ok(o) => o,
            Err(e) => panic!("nm failed: {}", e),
        };
        if !out.status.success() {
            panic!("nm failed");
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[1] == "T" {
                    Some(parts[2].to_string())
                } else {
                    None
                }
            })
            .filter(|s| !matches!(s.as_str(), "_init" | "_fini"))
            .collect()
    };
    let c_syms: std::collections::BTreeSet<String> = collect(&c_path).into_iter().collect();
    let r_syms: std::collections::BTreeSet<String> = collect(&r_path).into_iter().collect();
    let missing: Vec<&String> = c_syms.difference(&r_syms).collect();
    assert!(missing.is_empty(), "Rust .so missing C symbols: {:?}", missing);
}

#[allow(dead_code)]
fn _silence_unused_imports() {
    let _ = MAX_NAME_LENGTH;
    let _ = MAX_CATEGORY_LENGTH;
    let _: c_char = 0;
}
