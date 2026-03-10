use std::os::raw::c_int;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

#[unsafe(no_mangle)]
pub extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    unsafe {
        if !head.is_null() {
            let mut smallest = (*head).value;
            let mut current = head;
            while !(*current).next.is_null() {
                current = (*current).next;
                if (*current).value < smallest {
                    smallest = (*current).value;
                }
            }
            smallest
        } else {
            -1
        }
    }
}
