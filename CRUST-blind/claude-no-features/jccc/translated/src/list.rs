use std::any::Any;
/// A block in a linked list that holds multiple elements.
#[derive(Debug)]
pub struct ListBlock {
pub array: Vec<Box<dyn Any>>,
pub size: i32,
pub full: i32,
pub next: Option<Box<ListBlock>>,
}
/// A linked list structure consisting of blocks.
#[derive(Debug)]
pub struct List {
pub head: Option<Box<ListBlock>>,
/// In pure safe Rust, storing a raw pointer is discouraged. This is just
/// a placeholder to mimic C's design. An idiomatic approach would handle
/// linked traversal safely, potentially removing a raw tail pointer.
pub tail: Option<*mut ListBlock>,
pub blocksize: i32,
}
/// Retrieves an element from the list by index, if it exists.
pub fn lget_element(l: &mut List, index: i32) -> Option<&mut Box<dyn Any>> {
    if index < 0 {
        return None;
    }
    let mut remaining = index;
    let mut current = l.head.as_mut();
    while let Some(block) = current {
        if remaining < block.full {
            return block.array.get_mut(remaining as usize);
        }
        remaining -= block.size;
        current = block.next.as_mut();
    }
    None
}
/// Destroys the list and frees resources.
pub fn destroy_list(l: &mut List) -> i32 {
    // Walk the chain of blocks and drop each one to avoid recursive Drop on
    // very long lists. This mirrors the C `while` loop freeing each block.
    let mut current = l.head.take();
    while let Some(mut block) = current {
        current = block.next.take();
    }
    l.tail = None;
    0
}
/// Adds an element to the list.
pub fn ladd_element(l: &mut List, element: Box<dyn Any>) -> i32 {
    // If list is empty, create the first block.
    if l.head.is_none() {
        let block = new_block(l);
        l.head = Some(block);
    }

    // Walk to the last block.
    let mut current = l.head.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }

    if current.full < current.size {
        current.array.push(element);
        current.full += 1;
    } else {
        let new_b = new_block(l);
        // Re-walk to the tail since `new_block` doesn't actually link it.
        let mut current = l.head.as_mut().unwrap();
        while current.next.is_some() {
            current = current.next.as_mut().unwrap();
        }
        current.next = Some(new_b);
        let inserted = current.next.as_mut().unwrap();
        inserted.array.push(element);
        inserted.full += 1;
    }
    0
}
/// Allocates a new block and links it into the list.
pub fn new_block(l: &mut List) -> Box<ListBlock> {
    Box::new(ListBlock {
        array: Vec::with_capacity(l.blocksize.max(0) as usize),
        size: l.blocksize,
        full: 0,
        next: None,
    })
}
/// Iterates over the list with a provided function.
pub fn literate(l: &mut List, func: fn(&mut Box<dyn Any>) -> i32) -> i32 {
    let mut acc = 0;
    let mut current = l.head.as_mut();
    while let Some(block) = current {
        let full = block.full as usize;
        for i in 0..full {
            if let Some(elem) = block.array.get_mut(i) {
                acc += func(elem);
            }
        }
        current = block.next.as_mut();
    }
    acc
}
/// Finds and sets index variables for internal iteration.
/// Walks the list until the block containing index `*i` is found and adjusts
/// `*i` to be relative to that block. The `lb` out-parameter is left as `None`
/// because we can't yield a `Box<ListBlock>` without taking ownership of the
/// list contents.
pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    if *i < 0 {
        return -1;
    }
    *lb = None;
    let mut current = l.head.as_ref();
    while let Some(block) = current {
        if *i < block.size {
            return 0;
        }
        *i -= block.size;
        current = block.next.as_ref();
    }
    -1
}
/// Creates a new list with the specified blocksize.
pub fn create_list(blocksize: i32) -> List {
    List {
        head: None,
        tail: None,
        blocksize,
    }
}
/// Sets an element in the list by index.
pub fn lset_element(l: &mut List, index: i32, value: Box<dyn Any>) -> i32 {
    if index < 0 {
        return -1;
    }
    let mut remaining = index;
    let mut current = l.head.as_mut();
    while let Some(block) = current {
        if remaining < block.full {
            if let Some(slot) = block.array.get_mut(remaining as usize) {
                *slot = value;
                return 0;
            }
            return -1;
        }
        remaining -= block.size;
        current = block.next.as_mut();
    }
    -1
}
