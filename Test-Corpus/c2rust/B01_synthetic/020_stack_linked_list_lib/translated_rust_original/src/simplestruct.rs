#[derive(Copy, Clone)]
#[repr(C)]
pub struct ListNode {
    pub value: ::core::ffi::c_int,
    pub next: *mut ListNode,
}
#[no_mangle]
pub unsafe extern "C" fn smallestValue(mut head: *mut ListNode) -> ::core::ffi::c_int {
    if !head.is_null() {
        let mut smallest: ::core::ffi::c_int = (*head).value;
        while !(*head).next.is_null() {
            head = (*head).next;
            if (*head).value < smallest {
                smallest = (*head).value;
            }
        }
        return smallest;
    } else {
        return -(1 as ::core::ffi::c_int);
    };
}
