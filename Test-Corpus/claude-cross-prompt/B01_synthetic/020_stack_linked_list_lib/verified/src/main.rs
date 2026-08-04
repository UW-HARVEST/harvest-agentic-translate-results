// Translation of c_src/src/simplestruct.c
//
// The original C file defines a single library function `smallestValue`
// that walks a singly-linked list (`struct ListNode`) and returns the
// smallest value in the list. The C package has no `main` function and
// performs no I/O, so this Rust executable mirrors that: no input is
// consumed and no output is produced. The translated function below is
// preserved (as `smallest_value`) for byte-identical behavior should it
// be wired into a larger program.

#[derive(Debug)]
struct ListNode {
    value: i32,
    next: Option<Box<ListNode>>,
}

/// Returns the smallest `value` in the linked list. If the list is empty
/// (i.e. `head` is `None`), returns -1, matching the C implementation.
fn smallest_value(head: Option<&ListNode>) -> i32 {
    if let Some(mut node) = head {
        let mut smallest = node.value;
        while let Some(next) = node.next.as_deref() {
            node = next;
            if node.value < smallest {
                smallest = node.value;
            }
        }
        smallest
    } else {
        -1
    }
}

fn main() {
    // The C source provides no `main` (it builds a shared library), so the
    // executable produces no output for any input. We still reference
    // `smallest_value` here through a no-op so the function is exercised by
    // the type-checker.
    let _ = smallest_value(None::<&ListNode>);
}
