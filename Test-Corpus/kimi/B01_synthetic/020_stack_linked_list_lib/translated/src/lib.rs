use std::os::raw::{c_int, c_void};

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
    unsafe {
        let mut smallest = (*head).value;
        let mut current = (*head).next;
        while !current.is_null() {
            if (*current).value < smallest {
                smallest = (*current).value;
            }
            current = (*current).next;
        }
        smallest
    }
}