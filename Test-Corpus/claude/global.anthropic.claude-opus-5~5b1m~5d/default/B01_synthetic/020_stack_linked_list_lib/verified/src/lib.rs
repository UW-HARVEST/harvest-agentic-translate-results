// Rust translation of c_src/src/simplestruct.c (MIT Lincoln Laboratory, 2025).
//
// The C library exposes a single public symbol, `smallestValue`, operating on a
// singly linked list of `int`. This translation reproduces the exact ABI,
// signature, traversal order and return values of the original.

#![allow(non_snake_case)]

use std::ffi::c_int;

/// Mirrors:
/// ```c
/// struct ListNode {
///     int value;
///     struct ListNode* next;
/// };
/// ```
#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

// ABI guard: the C struct is `{ int; struct ListNode*; }`, so on every target the
// C compiler lays it out as value@0, next@sizeof(ptr), size 2*sizeof(ptr). Pin
// that here so any accidental field reordering or type change is a COMPILE error
// instead of silent memory corruption when a C-built chain is passed in.
// `tests/layout.rs` independently confirms these are the offsets the real C
// compiler produces for `c_src/include/simplestruct.h`.
const _: () = {
    assert!(core::mem::offset_of!(ListNode, value) == 0);
    assert!(core::mem::offset_of!(ListNode, next) == core::mem::size_of::<*mut ListNode>());
    assert!(core::mem::size_of::<ListNode>() == 2 * core::mem::size_of::<*mut ListNode>());
    assert!(core::mem::align_of::<ListNode>() == core::mem::align_of::<*mut ListNode>());
    assert!(core::mem::size_of::<c_int>() == 4);
};

/// Returns the smallest `value` in the list starting at `head`, or `-1` when
/// `head` is NULL.
///
/// Faithful to the C original:
/// * NULL `head` yields `-1` (indistinguishable from a list whose minimum is
///   `-1`; this quirk is preserved intentionally).
/// * The head's value seeds the running minimum, then each `next` node is
///   visited exactly once via the `while (head->next)` loop, so a strictly
///   smaller value replaces the minimum (ties keep the earlier value).
///
/// # Safety
/// `head` must be NULL or point to a valid, NULL-terminated, non-cyclic chain
/// of `ListNode` values — the same contract the C function imposes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    if !head.is_null() {
        let mut head = head;
        let mut smallest = (*head).value;
        while !(*head).next.is_null() {
            head = (*head).next;
            if (*head).value < smallest {
                smallest = (*head).value;
            }
        }
        smallest
    } else {
        -1
    }
}
