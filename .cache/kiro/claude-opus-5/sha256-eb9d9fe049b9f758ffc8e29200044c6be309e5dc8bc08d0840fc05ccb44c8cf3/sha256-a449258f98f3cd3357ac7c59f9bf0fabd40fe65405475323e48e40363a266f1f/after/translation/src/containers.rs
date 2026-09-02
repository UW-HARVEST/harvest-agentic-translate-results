// containers.rs
//
// Safe-Rust translation of the DECLARE_ARRAY / DEFINE_ARRAY and
// DECLARE_LIST / DEFINE_LIST macro families from
// c_src/include/generic_containers.h.
//
// The C macros generate, for every type `T`:
//   * a growable array (`data`, `size`, `capacity`) with doubling growth,
//   * a singly linked list (`head`, `tail`, `size`).
//
// Rust generics replace the token-pasting macros. `Array<T>` keeps an explicit
// `capacity` field so that the C growth bookkeeping is mirrored exactly, and
// `List<T>` is a node arena with `head`/`tail` indices so that the linked
// structure (and therefore iteration order) is preserved without `unsafe`.

#![allow(dead_code)]

// ============================================================================
// GENERIC DYNAMIC ARRAY
// ============================================================================

pub struct Array<T> {
    data: Vec<T>,
    capacity: usize,
}

impl<T: Copy> Array<T> {
    /// `array_TYPE_create`: a zero initial capacity is bumped to 16.
    pub fn create(initial_capacity: usize) -> Array<T> {
        let capacity = if initial_capacity > 0 {
            initial_capacity
        } else {
            16
        };
        Array {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// `array_TYPE_push`: doubles the capacity once `size >= capacity`.
    pub fn push(&mut self, value: T) {
        if self.data.len() >= self.capacity {
            let new_capacity = self.capacity * 2;
            self.data.reserve_exact(new_capacity - self.data.len());
            self.capacity = new_capacity;
        }
        self.data.push(value);
    }

    /// `array_TYPE_get`: unchecked in C; indexing panics in Rust instead.
    pub fn get(&self, index: usize) -> T {
        self.data[index]
    }

    /// `array_TYPE_size`
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// `array_TYPE_clear`: resets `size`, keeps the buffer.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Backing storage, equivalent to reading `arr->data` in C.
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// `ARRAY_FOREACH(TYPE, var, arr)`
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }
}

// ============================================================================
// GENERIC LINKED LIST
// ============================================================================

struct ListNode<T> {
    data: T,
    next: Option<usize>,
}

pub struct List<T> {
    nodes: Vec<ListNode<T>>,
    head: Option<usize>,
    tail: Option<usize>,
}

impl<T: Copy> List<T> {
    /// `list_TYPE_create`
    pub fn create() -> List<T> {
        List {
            nodes: Vec::new(),
            head: None,
            tail: None,
        }
    }

    /// `list_TYPE_append`
    pub fn append(&mut self, value: T) {
        let idx = self.nodes.len();
        self.nodes.push(ListNode {
            data: value,
            next: None,
        });
        match self.tail {
            None => {
                self.head = Some(idx);
                self.tail = Some(idx);
            }
            Some(tail) => {
                self.nodes[tail].next = Some(idx);
                self.tail = Some(idx);
            }
        }
    }

    /// `list_TYPE_prepend`
    pub fn prepend(&mut self, value: T) {
        let idx = self.nodes.len();
        let old_head = self.head;
        self.nodes.push(ListNode {
            data: value,
            next: old_head,
        });
        self.head = Some(idx);
        if self.tail.is_none() {
            self.tail = Some(idx);
        }
    }

    /// `list_TYPE_size`
    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    /// `list_TYPE_clear`
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.head = None;
        self.tail = None;
    }

    /// `LIST_FOREACH(TYPE, var, list)`: walks the `next` chain from `head`.
    pub fn iter(&self) -> ListIter<'_, T> {
        ListIter {
            list: self,
            current: self.head,
        }
    }
}

pub struct ListIter<'a, T> {
    list: &'a List<T>,
    current: Option<usize>,
}

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let idx = self.current?;
        let node = &self.list.nodes[idx];
        self.current = node.next;
        Some(&node.data)
    }
}
