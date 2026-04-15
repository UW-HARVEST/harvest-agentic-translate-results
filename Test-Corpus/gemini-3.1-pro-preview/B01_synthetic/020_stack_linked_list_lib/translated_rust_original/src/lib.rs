use std::os::raw::c_int;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

#[unsafe(no_mangle)]
pub extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    let mut current = unsafe { head.as_ref() };
    
    let mut smallest = match current {
        Some(node) => node.value,
        None => return -1,
    };
    
    while let Some(node) = current {
        if node.value < smallest {
            smallest = node.value;
        }
        current = unsafe { node.next.as_ref() };
    }
    
    smallest
}
