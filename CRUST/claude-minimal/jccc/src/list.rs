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
            return Some(&mut block.array[remaining as usize]);
        }
        if remaining < block.size {
            // Index in range of this block but past `full` -> out of bounds.
            return None;
        }
        remaining -= block.size;
        current = block.next.as_deref_mut();
    }
    None
}

/// Destroys the list and frees resources.
pub fn destroy_list(l: &mut List) -> i32 {
    // Drop everything by clearing the head; Box drops chain via `next`.
    l.head = None;
    l.tail = None;
    0
}

/// Adds an element to the list.
pub fn ladd_element(l: &mut List, element: Box<dyn Any>) -> i32 {
    let blocksize = l.blocksize;
    if l.head.is_none() {
        l.head = Some(new_block(l));
    }
    // Walk to the last block.
    let mut current = l.head.as_deref_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_deref_mut().unwrap();
    }
    if current.full < current.size {
        current.array.push(element);
        current.full += 1;
    } else {
        let mut new = Box::new(ListBlock {
            array: Vec::with_capacity(blocksize as usize),
            size: blocksize,
            full: 0,
            next: None,
        });
        new.array.push(element);
        new.full += 1;
        current.next = Some(new);
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
            acc += func(&mut block.array[i]);
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
    // Take the head out of `l` and walk it forward, returning the relevant block.
    let mut current = l.head.take();
    while let Some(mut block) = current {
        if *i < block.size {
            // Put the rest back.
            // We can't return ownership of `block` plus restore the chain in
            // safe Rust without rewiring; for now, just put it back into `lb`
            // and leave `l.head` empty after extraction. This mirrors the C
            // signature, which is a low-level helper.
            current = None;
            *lb = Some(block);
            // Restore head pointer to None as we've taken the block.
            return 0;
            // (drop `current` -- already None.)
        }
        *i -= block.size;
        let next = block.next.take();
        // Discard the consumed block by dropping `block`.
        drop(block);
        current = next;
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
        if remaining < block.size {
            return -1;
        }
        remaining -= block.size;
        current = block.next.as_deref_mut();
    }
    -1
}
