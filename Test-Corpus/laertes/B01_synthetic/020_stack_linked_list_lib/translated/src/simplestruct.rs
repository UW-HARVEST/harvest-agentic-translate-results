#[derive(Copy, Clone)]
#[repr(C)]
pub struct ListNode {
    pub value: libc::c_int,
    pub next: *mut ListNode,
}
#[no_mangle]
pub unsafe extern "C" fn smallestValue(mut head: *mut ListNode) -> libc::c_int {
    if !head.is_null() {
        let mut smallest: libc::c_int = (*head).value;
        while !(*head).next.is_null() {
            head = (*head).next;
            if (*head).value < smallest {
                smallest = (*head).value;
            }
        }
        return smallest;
    } else {
        return -(1 as libc::c_int);
    };
}
