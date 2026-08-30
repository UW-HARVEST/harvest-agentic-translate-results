// Rust translation of c_src/src/simplestruct.c
//
// Original C:
//   Copyright 2025 MIT Lincoln Laboratory (MIT license, see c_src headers)
//
// Behavior is reproduced exactly, including returning -1 for a NULL head
// (which is indistinguishable from a list whose smallest value is -1).

// The C library is named `SimpleList` (see c_src/CMakeLists.txt), which is not
// snake case; keep the name to match the produced `libSimpleList.so`.
#![allow(non_snake_case)]

use std::ffi::c_int;

/// Mirrors `struct ListNode` from include/simplestruct.h:
///
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

/// `int smallestValue(struct ListNode *head);`
///
/// Returns the smallest `value` in the singly linked list starting at `head`,
/// or -1 if `head` is NULL.
///
/// # Safety
///
/// `head` must either be NULL or point to a valid, NUL-terminated (i.e.
/// `next`-terminated) chain of `ListNode` values. The list must not contain
/// cycles, otherwise this loops forever, matching the C original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    // `if (head)` in the C source.
    if head.is_null() {
        // `else return -1;`
        return -1;
    }

    let mut node: &ListNode = unsafe { &*head };
    let mut smallest: c_int = node.value;

    // `while (head->next) { head = head->next; ... }`
    while !node.next.is_null() {
        node = unsafe { &*node.next };
        if node.value < smallest {
            smallest = node.value;
        }
    }

    smallest
}
