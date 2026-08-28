//! Test-only harness mirroring `tests/harness/c_harness.c`.
//!
//! The pristine translation (`translation/src/lib.rs`) is textually included so
//! that its private helpers and `static mut` state become reachable from this
//! crate, exactly as `#include "lib.c"` does on the C side. The shipped crate
//! itself is not modified.

#![allow(unused_imports)]

include!("../../../../src/lib.rs");

use std::ffi::c_uchar;

#[no_mangle]
pub extern "C" fn h_reset() {
    node_count_set(0);
}

#[no_mangle]
pub extern "C" fn h_node_count() -> c_int {
    node_count_get()
}

#[no_mangle]
pub extern "C" fn h_set_node_count(n: c_int) {
    node_count_set(n);
}

/// Returns the index of the matching node, or -1 for NULL.
#[no_mangle]
pub extern "C" fn h_find_node_by_id(id: c_int) -> c_int {
    let p = find_node_by_id(id);
    if p.is_null() {
        return -1;
    }
    let base = node_storage_ptr();
    unsafe { p.offset_from(base) as c_int }
}

#[no_mangle]
pub extern "C" fn h_add_node(id: c_int, parent_id: c_int, value: c_double) -> c_int {
    add_node(id, parent_id, value)
}

#[no_mangle]
pub unsafe extern "C" fn h_process_backward(
    array: *mut c_int,
    size: usize,
    start_offset: c_int,
) -> c_int {
    process_backward(array, size, start_offset)
}

#[no_mangle]
pub unsafe extern "C" fn h_compute_size_metric(s: *const c_char) -> c_int {
    compute_size_metric(s)
}

#[no_mangle]
pub extern "C" fn h_safe_double_to_int(value: c_double) -> c_int {
    safe_double_to_int(value)
}

#[no_mangle]
pub extern "C" fn h_initialize_test_data() {
    initialize_test_data();
}

/// Reads one slot of `NODE_STORAGE`. Returns 0 on success, -1 if out of range.
#[no_mangle]
pub unsafe extern "C" fn h_get_node(
    index: c_int,
    id: *mut c_int,
    parent_id: *mut c_int,
    value: *mut c_double,
    data_out4: *mut c_int,
) -> c_int {
    if index < 0 || index as usize >= MAX_NODES {
        return -1;
    }
    let slot = node_storage_ptr().offset(index as isize);
    *id = (*slot).id;
    *parent_id = (*slot).parent_id;
    *value = (*slot).value;
    for i in 0..4 {
        *data_out4.add(i) = (*slot).data[i];
    }
    0
}

/// Raw byte view of one node, to compare struct layout/padding exactly.
#[no_mangle]
pub unsafe extern "C" fn h_node_bytes(index: c_int, out: *mut c_uchar, out_len: usize) -> c_int {
    let n = std::mem::size_of::<Node>();
    if index < 0 || index as usize >= MAX_NODES || out_len < n {
        return -1;
    }
    let src = node_storage_ptr().offset(index as isize) as *const c_uchar;
    for i in 0..n {
        *out.add(i) = *src.add(i);
    }
    n as c_int
}

#[no_mangle]
pub extern "C" fn h_sizeof_node() -> usize {
    std::mem::size_of::<Node>()
}

#[no_mangle]
pub extern "C" fn h_status_ok() -> c_int {
    STATUS_OK
}
#[no_mangle]
pub extern "C" fn h_status_warning() -> c_int {
    STATUS_WARNING
}
#[no_mangle]
pub extern "C" fn h_status_error() -> c_int {
    STATUS_ERROR
}
#[no_mangle]
pub extern "C" fn h_status_critical() -> c_int {
    STATUS_CRITICAL
}
#[no_mangle]
pub extern "C" fn h_max_nodes() -> c_int {
    MAX_NODES as c_int
}
