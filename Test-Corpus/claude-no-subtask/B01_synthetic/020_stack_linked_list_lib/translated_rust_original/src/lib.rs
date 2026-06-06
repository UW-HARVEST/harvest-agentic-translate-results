use std::ffi::c_int;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    if !head.is_null() {
        let mut current = head;
        let mut smallest = unsafe { (*current).value };
        while !unsafe { (*current).next }.is_null() {
            current = unsafe { (*current).next };
            let v = unsafe { (*current).value };
            if v < smallest {
                smallest = v;
            }
        }
        smallest
    } else {
        -1
    }
}
