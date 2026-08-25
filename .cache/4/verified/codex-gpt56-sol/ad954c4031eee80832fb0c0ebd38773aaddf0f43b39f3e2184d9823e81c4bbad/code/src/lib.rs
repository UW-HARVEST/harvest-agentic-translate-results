use std::ffi::c_int;
use std::ptr;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn smallestValue(mut head: *mut ListNode) -> c_int {
    if head.is_null() {
        return -1;
    }

    // SAFETY: As in the C implementation, callers must provide a valid,
    // null-terminated linked list.
    unsafe {
        let mut smallest = ptr::read(ptr::addr_of!((*head).value));
        while !ptr::read(ptr::addr_of!((*head).next)).is_null() {
            head = ptr::read(ptr::addr_of!((*head).next));
            let value = ptr::read(ptr::addr_of!((*head).value));
            if value < smallest {
                smallest = value;
            }
        }
        smallest
    }
}
