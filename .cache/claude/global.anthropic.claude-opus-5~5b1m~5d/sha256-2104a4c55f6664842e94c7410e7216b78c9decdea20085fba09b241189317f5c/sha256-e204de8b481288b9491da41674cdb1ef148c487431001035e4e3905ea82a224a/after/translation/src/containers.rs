// containers.rs
//
// Rust translation of generic_containers.h
//
// The C header uses token-pasting macros (DECLARE_ARRAY / DEFINE_ARRAY and
// DECLARE_LIST / DEFINE_LIST) to instantiate a dynamic array and a singly
// linked list for each element type.  Rust generics give the same result
// without macros, so `Array<T>` stands in for every `array_TYPE_t` and
// `List<T>` for every `list_TYPE_t`.

#![allow(dead_code)]

// ============================================================================
// GENERIC DYNAMIC ARRAY  (DECLARE_ARRAY / DEFINE_ARRAY)
// ============================================================================

pub struct Array<T> {
    data: Vec<T>,
}

impl<T: Copy> Array<T> {
    /// array_TYPE_create: a capacity of 0 is replaced by 16, as in the C macro.
    pub fn create(initial_capacity: usize) -> Array<T> {
        let capacity = if initial_capacity > 0 {
            initial_capacity
        } else {
            16
        };
        Array {
            data: Vec::with_capacity(capacity),
        }
    }

    /// array_TYPE_push
    pub fn push(&mut self, value: T) -> i32 {
        self.data.push(value);
        0
    }

    /// array_TYPE_get
    pub fn get(&self, index: usize) -> T {
        self.data[index]
    }

    /// array_TYPE_size
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// array_TYPE_clear
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// ARRAY_FOREACH
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data
    }
}

// ============================================================================
// GENERIC LINKED LIST  (DECLARE_LIST / DEFINE_LIST)
// ============================================================================

struct ListNode<T> {
    data: T,
    next: Option<Box<ListNode<T>>>,
}

pub struct List<T> {
    head: Option<Box<ListNode<T>>>,
    size: usize,
}

impl<T: Copy> List<T> {
    /// list_TYPE_create
    pub fn create() -> List<T> {
        List {
            head: None,
            size: 0,
        }
    }

    /// list_TYPE_append
    pub fn append(&mut self, value: T) -> i32 {
        let node = Box::new(ListNode {
            data: value,
            next: None,
        });
        // Walk to the tail.  The C version keeps a tail pointer; the observable
        // behaviour (insertion order) is identical.
        let mut cursor = &mut self.head;
        while cursor.is_some() {
            cursor = &mut cursor.as_mut().unwrap().next;
        }
        *cursor = Some(node);
        self.size += 1;
        0
    }

    /// list_TYPE_prepend
    pub fn prepend(&mut self, value: T) -> i32 {
        let node = Box::new(ListNode {
            data: value,
            next: self.head.take(),
        });
        self.head = Some(node);
        self.size += 1;
        0
    }

    /// list_TYPE_size
    pub fn size(&self) -> usize {
        self.size
    }

    /// list_TYPE_clear
    pub fn clear(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
        self.size = 0;
    }

    /// LIST_FOREACH
    pub fn iter(&self) -> ListIter<'_, T> {
        ListIter {
            node: self.head.as_deref(),
        }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        // Iterative teardown so that long lists do not blow the stack.
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}

pub struct ListIter<'a, T> {
    node: Option<&'a ListNode<T>>,
}

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match self.node {
            Some(n) => {
                self.node = n.next.as_deref();
                Some(&n.data)
            }
            None => None,
        }
    }
}
