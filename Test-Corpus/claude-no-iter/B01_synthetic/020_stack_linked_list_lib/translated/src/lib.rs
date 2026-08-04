#![allow(non_snake_case)]

use std::ffi::c_int;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

#[unsafe(no_mangle)]
pub extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    if !head.is_null() {
        // SAFETY: caller guarantees `head` points to a valid ListNode if non-null;
        // each `next` pointer is either null or points to a valid ListNode.
        unsafe {
            let mut current = head;
            let mut smallest = (*current).value;
            while !(*current).next.is_null() {
                current = (*current).next;
                if (*current).value < smallest {
                    smallest = (*current).value;
                }
            }
            smallest
        }
    } else {
        -1
    }
}
