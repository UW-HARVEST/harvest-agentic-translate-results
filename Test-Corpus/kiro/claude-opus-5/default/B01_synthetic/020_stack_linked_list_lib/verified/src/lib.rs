// Rust translation of the SimpleList C library (c_src/).
//
// Original C library: Copyright 2025 MIT Lincoln Laboratory (MIT-style license,
// see c_src/include/simplestruct.h for the full notice).
//
// The complete public ABI of the C shared library consists of exactly one
// exported symbol, `smallestValue`, as declared in `c_src/include/simplestruct.h`.
// There are no namespace/renaming preprocessor macros in the public header, so
// the linker symbol name is identical to the source-level name.

#![allow(non_snake_case)]

use std::ffi::c_int;

/// Mirrors the C declaration:
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

/// Safe core of `smallestValue`, operating on a non-null head node.
///
/// Reproduces the exact C traversal and comparison order:
/// the running minimum is seeded from the head node, then the list is advanced
/// while `next` is non-null, replacing the minimum only on a strict `<`
/// comparison (so the earliest occurrence of the minimum is the one retained).
fn smallest_value_from(head: &ListNode) -> c_int {
    let mut node: &ListNode = head;
    let mut smallest: c_int = node.value;

    while !node.next.is_null() {
        // SAFETY: `node.next` was just checked to be non-null. As in the C
        // original, the caller is responsible for the pointer being a valid,
        // properly aligned `ListNode` that outlives this call.
        node = unsafe { &*node.next };

        if node.value < smallest {
            smallest = node.value;
        }
    }

    smallest
}

/// C signature: `int smallestValue (struct ListNode *date);`
///
/// Returns the smallest `value` in the singly linked list, or `-1` when the
/// head pointer is NULL. Note that `-1` is also a legitimate list value, so a
/// NULL list is indistinguishable from a list whose minimum is `-1`; this
/// behavior is preserved from the C original rather than "fixed".
///
/// # Safety
///
/// `date` must either be NULL or point to a valid, null-terminated chain of
/// `ListNode` values that remains valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn smallestValue(date: *mut ListNode) -> c_int {
    if !date.is_null() {
        // SAFETY: `date` was just checked to be non-null; validity is the
        // caller's responsibility, exactly as in the C original.
        let head: &ListNode = unsafe { &*date };
        smallest_value_from(head)
    } else {
        -1
    }
}
