// FFI conformance tests: generic linked-list container.

mod common;

use common::*;
use libloading::Library;
use std::os::raw::{c_double, c_int};

type ListIntCreate = unsafe extern "C" fn() -> *mut list_int_t;
type ListIntDestroy = unsafe extern "C" fn(*mut list_int_t);
type ListIntAppend = unsafe extern "C" fn(*mut list_int_t, c_int) -> c_int;
type ListIntPrepend = unsafe extern "C" fn(*mut list_int_t, c_int) -> c_int;
type ListIntSize = unsafe extern "C" fn(*mut list_int_t) -> size_t;
type ListIntClear = unsafe extern "C" fn(*mut list_int_t);

fn run_list_int(lib: &Library) -> (Vec<c_int>, size_t) {
    unsafe {
        let create = sym::<ListIntCreate>(lib, b"list_int_create");
        let destroy = sym::<ListIntDestroy>(lib, b"list_int_destroy");
        let append = sym::<ListIntAppend>(lib, b"list_int_append");
        let prepend = sym::<ListIntPrepend>(lib, b"list_int_prepend");
        let size = sym::<ListIntSize>(lib, b"list_int_size");
        let clear = sym::<ListIntClear>(lib, b"list_int_clear");

        let list = create();
        assert!(!list.is_null());
        // Append 100, 200, 300, 400, 500
        for v in [100, 200, 300, 400, 500] {
            assert_eq!(append(list, v), 0);
        }
        // Prepend 0 and -1 (so list becomes -1, 0, 100, 200, 300, 400, 500)
        assert_eq!(prepend(list, 0), 0);
        assert_eq!(prepend(list, -1), 0);

        let n = size(list);

        // Walk the list and gather values.
        let mut walked = Vec::new();
        let mut node = (*list).head;
        while !node.is_null() {
            walked.push((*node).data);
            node = (*node).next;
        }

        // Exercise clear(): list should be empty afterwards but still usable.
        clear(list);
        assert_eq!(size(list), 0);
        assert!((*list).head.is_null());
        assert!((*list).tail.is_null());

        // Append to cleared list and verify head/tail are linked again.
        assert_eq!(append(list, 42), 0);
        assert_eq!(size(list), 1);
        assert_eq!((*(*list).head).data, 42);
        assert!(std::ptr::eq((*list).head, (*list).tail));

        destroy(list);

        // Null safety
        assert_eq!(size(std::ptr::null_mut()), 0);
        assert_eq!(append(std::ptr::null_mut(), 1), -1);
        assert_eq!(prepend(std::ptr::null_mut(), 1), -1);

        (walked, n)
    }
}

#[test]
fn list_int_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();
    let c_r = run_list_int(&c_lib);
    let r_r = run_list_int(&r_lib);
    assert_eq!(c_r, r_r);
}

type ListDoubleCreate = unsafe extern "C" fn() -> *mut list_double_t;
type ListDoubleDestroy = unsafe extern "C" fn(*mut list_double_t);
type ListDoubleAppend = unsafe extern "C" fn(*mut list_double_t, c_double) -> c_int;
type ListDoublePrepend = unsafe extern "C" fn(*mut list_double_t, c_double) -> c_int;
type ListDoubleSize = unsafe extern "C" fn(*mut list_double_t) -> size_t;

fn run_list_double(lib: &Library) -> Vec<u64> {
    unsafe {
        let create = sym::<ListDoubleCreate>(lib, b"list_double_create");
        let destroy = sym::<ListDoubleDestroy>(lib, b"list_double_destroy");
        let append = sym::<ListDoubleAppend>(lib, b"list_double_append");
        let prepend = sym::<ListDoublePrepend>(lib, b"list_double_prepend");
        let size = sym::<ListDoubleSize>(lib, b"list_double_size");

        let list = create();
        for v in [9.99, 14.50, 7.25] {
            assert_eq!(append(list, v), 0);
        }
        assert_eq!(prepend(list, -1.0), 0);
        assert_eq!(size(list), 4);

        let mut bits = Vec::new();
        let mut node = (*list).head;
        while !node.is_null() {
            bits.push((*node).data.to_bits());
            node = (*node).next;
        }

        destroy(list);
        bits
    }
}

#[test]
fn list_double_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();
    assert_eq!(run_list_double(&c_lib), run_list_double(&r_lib));
}

// -- item_t list -------------------------------------------------------------

type ListItemCreate = unsafe extern "C" fn() -> *mut list_item_t_t;
type ListItemDestroy = unsafe extern "C" fn(*mut list_item_t_t);
type ListItemAppend = unsafe extern "C" fn(*mut list_item_t_t, item_t) -> c_int;
type ListItemPrepend = unsafe extern "C" fn(*mut list_item_t_t, item_t) -> c_int;
#[allow(dead_code)]
type ListItemSize = unsafe extern "C" fn(*mut list_item_t_t) -> size_t;

type CreateItemFn = unsafe extern "C" fn(
    c_int,
    *const std::os::raw::c_char,
    *const std::os::raw::c_char,
    c_double,
    c_int,
) -> item_t;

fn item_fp(it: &item_t) -> (c_int, Vec<u8>, Vec<u8>, u64, c_int) {
    (
        it.id,
        cstr_slice(&it.name).to_vec(),
        cstr_slice(&it.category).to_vec(),
        it.price.to_bits(),
        it.quantity,
    )
}

fn run_list_item(lib: &Library) -> Vec<(c_int, Vec<u8>, Vec<u8>, u64, c_int)> {
    unsafe {
        let create = sym::<ListItemCreate>(lib, b"list_item_t_create");
        let destroy = sym::<ListItemDestroy>(lib, b"list_item_t_destroy");
        let append = sym::<ListItemAppend>(lib, b"list_item_t_append");
        let prepend = sym::<ListItemPrepend>(lib, b"list_item_t_prepend");
        let create_item = sym::<CreateItemFn>(lib, b"create_item");

        let list = create();
        let cn1 = std::ffi::CString::new("Laptop").unwrap();
        let cc1 = std::ffi::CString::new("Electronics").unwrap();
        let it1 = create_item(1, cn1.as_ptr(), cc1.as_ptr(), 899.99, 15);
        let cn2 = std::ffi::CString::new("Pen").unwrap();
        let cc2 = std::ffi::CString::new("Office").unwrap();
        let it2 = create_item(2, cn2.as_ptr(), cc2.as_ptr(), 1.99, 200);

        assert_eq!(append(list, it1), 0);
        assert_eq!(append(list, it2), 0);
        let cn3 = std::ffi::CString::new("Desk").unwrap();
        let cc3 = std::ffi::CString::new("Furniture").unwrap();
        let it3 = create_item(3, cn3.as_ptr(), cc3.as_ptr(), 349.99, 8);
        assert_eq!(prepend(list, it3), 0);

        let mut fps = Vec::new();
        let mut node = (*list).head;
        while !node.is_null() {
            fps.push(item_fp(&(*node).data));
            node = (*node).next;
        }

        destroy(list);
        fps
    }
}

#[test]
fn list_item_t_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();
    assert_eq!(run_list_item(&c_lib), run_list_item(&r_lib));
}

// -- order_t list ------------------------------------------------------------

type ListOrderCreate = unsafe extern "C" fn() -> *mut list_order_t_t;
type ListOrderDestroy = unsafe extern "C" fn(*mut list_order_t_t);
type ListOrderAppend = unsafe extern "C" fn(*mut list_order_t_t, order_t) -> c_int;

type CreateOrderFn =
    unsafe extern "C" fn(c_int, *const std::os::raw::c_char, c_double) -> order_t;

fn order_fp(o: &order_t) -> (c_int, Vec<u8>, u64) {
    (o.customer_id, cstr_slice(&o.customer_name).to_vec(), o.total_amount.to_bits())
}

fn run_list_order(lib: &Library) -> Vec<(c_int, Vec<u8>, u64)> {
    unsafe {
        let create = sym::<ListOrderCreate>(lib, b"list_order_t_create");
        let destroy = sym::<ListOrderDestroy>(lib, b"list_order_t_destroy");
        let append = sym::<ListOrderAppend>(lib, b"list_order_t_append");
        let create_order = sym::<CreateOrderFn>(lib, b"create_order");

        let list = create();
        let pairs = [
            (1001, "Alice Johnson", 1249.95),
            (1002, "Bob Smith", 89.99),
            (1003, "Carol White", 549.98),
        ];
        for (id, name, amt) in pairs {
            let cn = std::ffi::CString::new(name).unwrap();
            let o = create_order(id, cn.as_ptr(), amt);
            assert_eq!(append(list, o), 0);
        }

        let mut fps = Vec::new();
        let mut node = (*list).head;
        while !node.is_null() {
            fps.push(order_fp(&(*node).data));
            node = (*node).next;
        }

        destroy(list);
        fps
    }
}

#[test]
fn list_order_t_matches_c() {
    let c_lib = load_c();
    let r_lib = load_rust();
    assert_eq!(run_list_order(&c_lib), run_list_order(&r_lib));
}
