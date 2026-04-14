use std::os::raw::c_int;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

#[unsafe(no_mangle)]
pub extern "C" fn smallestValue(mut head: *mut ListNode) -> c_int {
    if head.is_null() {
        return -1;
    }

    unsafe {
        let mut smallest = (*head).value;
        while !(*head).next.is_null() {
            head = (*head).next;
            if (*head).value < smallest {
                smallest = (*head).value;
            }
        }
        smallest
    }
}
