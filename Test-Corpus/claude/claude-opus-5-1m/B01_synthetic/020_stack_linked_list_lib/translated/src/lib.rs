// Rust translation of the C library in c_src/ (MIT Lincoln Laboratory, 2025).
//
// Original C sources:
//   c_src/include/simplestruct.h
//   c_src/src/simplestruct.c
//
// The C build (CMakeLists.txt) globs all of c_src/src into a single shared
// library `libSimpleList.so`, whose complete exported public ABI is:
//   * smallestValue
//
// Behavior is reproduced exactly, including the `-1` sentinel returned for a
// NULL list head (which is indistinguishable from a list whose smallest value
// is genuinely -1 -- an original-code quirk that is intentionally preserved).

#![allow(non_snake_case)]

use std::ffi::c_int;

/// C: `struct ListNode { int value; struct ListNode* next; };`
///
/// `#[repr(C)]` keeps the field order/offsets/alignment identical to the C
/// declaration so that pointers handed to us by C callers are laid out the same.
#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

/// C:
/// ```c
/// int smallestValue (struct ListNode *head) {
///     if (head) {
///         int smallest = head->value;
///         while (head->next) {
///             head = head->next;
///             if (head->value < smallest) {
///                 smallest = head->value;
///             }
///         }
///         return smallest;
///     }
///     else return -1;
/// }
/// ```
///
/// Walks the singly linked list starting at `head` and returns the smallest
/// `value` found. Returns `-1` when `head` is NULL.
///
/// # Safety
///
/// `head` must either be NULL or point to a valid, properly aligned
/// `ListNode` whose `next` chain is NULL-terminated and consists of valid
/// nodes, exactly as the original C function requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    // `if (head)` -- NULL head yields the -1 sentinel.
    if head.is_null() {
        return -1;
    }

    let mut node: &ListNode = &*head;

    // `int smallest = head->value;`
    let mut smallest: c_int = node.value;

    // `while (head->next) { head = head->next; ... }`
    //
    // Note the C loop tests the *next* pointer before advancing, so the first
    // node's value is only ever read via the initialization above; every
    // subsequent node is compared after the advance. This ordering (rather
    // than a simple for-each) is preserved verbatim.
    while !node.next.is_null() {
        node = &*node.next;
        if node.value < smallest {
            smallest = node.value;
        }
    }

    smallest
}
