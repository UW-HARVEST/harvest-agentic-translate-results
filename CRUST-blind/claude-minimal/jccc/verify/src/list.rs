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
    let mut current = l.head.as_deref_mut();
    while let Some(block) = current {
        if remaining < block.full {
            return block.array.get_mut(remaining as usize);
        }
        remaining -= block.size;
        current = block.next.as_deref_mut();
    }
    None
}
/// Destroys the list and frees resources.
pub fn destroy_list(l: &mut List) -> i32 {
    // In Rust, dropping the head will recursively free all blocks.
    l.head = None;
    l.tail = None;
    0
}
/// Adds an element to the list.
pub fn ladd_element(l: &mut List, element: Box<dyn Any>) -> i32 {
    // If the list is empty, create the first block.
    if l.head.is_none() {
        let mut block = new_block(l);
        block.array.push(element);
        block.full += 1;
        l.head = Some(block);
        return 0;
    }
    // Walk to the last block and add there. If the last block is full,
    // create a new block and append it.
    let blocksize = l.blocksize;
    let mut current = l.head.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    if current.full < current.size {
        current.array.push(element);
        current.full += 1;
    } else {
        let mut new_b = Box::new(ListBlock {
            array: Vec::with_capacity(blocksize as usize),
            size: blocksize,
            full: 0,
            next: None,
        });
        new_b.array.push(element);
        new_b.full += 1;
        current.next = Some(new_b);
    }
    0
}
/// Allocates a new block and links it into the list.
pub fn new_block(l: &mut List) -> Box<ListBlock> {
    Box::new(ListBlock {
        array: Vec::with_capacity(l.blocksize as usize),
        size: l.blocksize,
        full: 0,
        next: None,
    })
}
/// Iterates over the list with a provided function.
pub fn literate(l: &mut List, func: fn(&mut Box<dyn Any>) -> i32) -> i32 {
    let mut acc = 0;
    let mut current = l.head.as_deref_mut();
    while let Some(block) = current {
        for i in 0..block.full as usize {
            if let Some(elem) = block.array.get_mut(i) {
                acc += func(elem);
            }
        }
        current = block.next.as_deref_mut();
    }
    acc
}
/// Finds and sets index variables for internal iteration.
pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    if *i < 0 {
        return -1;
    }
    // We can't really return a reference to the internal block; instead
    // we mirror the C API by walking the head and adjusting `i` to be the
    // index relative to the block that contains it.
    let mut current = l.head.as_deref();
    while let Some(block) = current {
        if *i < block.size {
            // Found the block; store nothing into lb (kept None to avoid
            // ownership issues) and leave `i` set to the local index.
            let _ = lb;
            return 0;
        }
        *i -= block.size;
        current = block.next.as_deref();
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
    let mut current = l.head.as_deref_mut();
    while let Some(block) = current {
        if remaining < block.full {
            block.array[remaining as usize] = value;
            return 0;
        }
        remaining -= block.size;
        current = block.next.as_deref_mut();
    }
    -1
}
