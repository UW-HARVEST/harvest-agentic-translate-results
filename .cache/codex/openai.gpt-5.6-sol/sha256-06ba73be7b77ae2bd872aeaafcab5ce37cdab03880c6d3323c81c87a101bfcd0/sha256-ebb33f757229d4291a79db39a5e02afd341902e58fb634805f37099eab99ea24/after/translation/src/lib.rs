use std::ffi::c_int;

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

    // SAFETY: As in the C implementation, the caller must provide a valid,
    // readable linked list whose non-null next pointers point to ListNode.
    let mut smallest = unsafe { (*head).value };
    while !unsafe { (*head).next }.is_null() {
        head = unsafe { (*head).next };
        if unsafe { (*head).value } < smallest {
            smallest = unsafe { (*head).value };
        }
    }
    smallest
}
