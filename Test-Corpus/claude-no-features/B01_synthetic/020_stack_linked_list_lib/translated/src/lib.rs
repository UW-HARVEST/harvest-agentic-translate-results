// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of simplestruct.c

#![allow(non_snake_case)]

use std::ffi::c_int;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

/// # Safety
///
/// `head` must either be null or a valid pointer to a `ListNode`. The chain of
/// `next` pointers must terminate in a null pointer and each node along the
/// chain must be a valid pointer to a `ListNode`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    let mut head = head;
    if !head.is_null() {
        let mut smallest: c_int = (*head).value;
        while !(*head).next.is_null() {
            head = (*head).next;
            if (*head).value < smallest {
                smallest = (*head).value;
            }
        }
        smallest
    } else {
        -1
    }
}
