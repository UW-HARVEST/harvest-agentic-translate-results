// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_snake_case)]

use std::os::raw::c_int;

#[repr(C)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

/// Returns the smallest value in the singly-linked list pointed to by `head`.
/// If `head` is null, returns -1.
///
/// # Safety
///
/// `head` must either be a null pointer or point to a valid `ListNode`.
/// Each subsequent `next` pointer must either be null or point to a valid
/// `ListNode`. The list must be properly terminated with a null pointer
/// and must not contain cycles.
#[no_mangle]
pub unsafe extern "C" fn smallestValue(head: *mut ListNode) -> c_int {
    if !head.is_null() {
        let mut current = head;
        let mut smallest = (*current).value;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn null_returns_minus_one() {
        unsafe {
            assert_eq!(smallestValue(ptr::null_mut()), -1);
        }
    }

    #[test]
    fn single_node() {
        let mut node = ListNode {
            value: 42,
            next: ptr::null_mut(),
        };
        unsafe {
            assert_eq!(smallestValue(&mut node as *mut ListNode), 42);
        }
    }

    #[test]
    fn multiple_nodes() {
        let mut n3 = ListNode {
            value: 1,
            next: ptr::null_mut(),
        };
        let mut n2 = ListNode {
            value: 7,
            next: &mut n3 as *mut ListNode,
        };
        let mut n1 = ListNode {
            value: 4,
            next: &mut n2 as *mut ListNode,
        };
        unsafe {
            assert_eq!(smallestValue(&mut n1 as *mut ListNode), 1);
        }
    }
}
