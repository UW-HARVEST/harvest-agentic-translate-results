// Translation of c_src/src/simplestruct.c to Rust.
// Preserves the exact C ABI: a `ListNode` struct with `int value` and a
// pointer to the next node, and a `smallestValue` function that returns
// the smallest value in the linked list, or -1 when given a null pointer.

use std::ffi::c_int;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

#[unsafe(no_mangle)]
pub extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    if head.is_null() {
        return -1;
    }
    // SAFETY: `head` is non-null and the caller guarantees it points to a
    // valid `ListNode`. We mirror the C pointer-walk exactly.
    unsafe {
        let mut cur: *mut ListNode = head;
        let mut smallest: c_int = (*cur).value;
        while !(*cur).next.is_null() {
            cur = (*cur).next;
            if (*cur).value < smallest {
                smallest = (*cur).value;
            }
        }
        smallest
    }
}
