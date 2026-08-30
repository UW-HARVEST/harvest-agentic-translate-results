// Rust translation of c_src/src/simplestruct.c
//
// Original work: Copyright 2025 MIT Lincoln Laboratory (MIT license, see c_src).
//
// Behavior is reproduced exactly, including returning -1 for a NULL list head.

// The crate and the exported function keep the original C names so the
// linker symbol matches; that trips Rust's naming lints.
#![allow(non_snake_case)]

use std::ffi::c_int;

/// Mirrors `struct ListNode` from `include/simplestruct.h`.
///
/// `#[repr(C)]` guarantees the same layout (and therefore the same field
/// offsets) as the C definition, so callers built against the C header can
/// pass pointers to this type interchangeably.
#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

/// Returns the smallest `value` in the linked list starting at `head`,
/// or `-1` when `head` is NULL.
///
/// # Safety
///
/// `head` must either be NULL or point to a valid, NULL-terminated
/// `ListNode` chain that stays alive for the duration of the call. This
/// matches the contract of the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    // The C code branches on the pointer first; only a non-NULL head is
    // dereferenced.
    if head.is_null() {
        return -1;
    }

    // Past the NULL check the traversal is expressed with safe references,
    // re-borrowing through the raw `next` pointer only at each hop.
    let mut node: &ListNode = unsafe { &*head };
    let mut smallest = node.value;

    while !node.next.is_null() {
        node = unsafe { &*node.next };
        if node.value < smallest {
            smallest = node.value;
        }
    }

    smallest
}
