
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ListNode {
    pub value: ::core::ffi::c_int,
    pub next: *mut ListNode,
}
#[no_mangle]
pub fn smallestValue(head: *mut ListNode) -> i32 {
    let mut current = head;
    let mut smallest = -1;

    while let Some(node) = unsafe { current.as_ref() } {
        if smallest == -1 || node.value < smallest {
            smallest = node.value;
        }
        current = node.next;
    }

    smallest
}

