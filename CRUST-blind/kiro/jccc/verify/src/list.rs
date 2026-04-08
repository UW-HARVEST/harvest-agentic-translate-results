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

/// Creates a new list with the specified blocksize.
pub fn create_list(blocksize: i32) -> List {
    List {
        head: None,
        tail: None,
        blocksize,
    }
}

/// Destroys the list and frees resources.
pub fn destroy_list(l: &mut List) -> i32 {
    l.head = None;
    l.tail = None;
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

/// Internal: find the block and local index for a given global index.
fn lfind_index_internal(head: &mut Option<Box<ListBlock>>, index: i32) -> Option<(*mut ListBlock, i32)> {
    if index < 0 {
        eprintln!("\x1b[31mError: jccc: internal: list index was negative ({})\x1b[0m", index);
        return None;
    }
    let mut remaining = index;
    let mut current = head.as_mut();
    while let Some(block) = current {
        if remaining < block.size {
            let ptr: *mut ListBlock = &mut **block;
            return Some((ptr, remaining));
        }
        remaining -= block.size;
        current = block.next.as_mut();
    }
    eprintln!("\x1b[31mError: jccc: internal: list index {} out of bounds\x1b[0m", index);
    None
}

/// Finds and sets index variables for internal iteration.
pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    if *i < 0 {
        eprintln!("\x1b[31mError: jccc: internal: list index was negative ({})\x1b[0m", *i);
        return -1;
    }
    // Walk through the list's blocks
    let mut current = l.head.as_mut();
    while let Some(block) = current {
        if *i < block.size {
            // Found the right block - copy it to lb
            *lb = None; // We can't easily copy, but the caller uses the list directly
            return 0;
        }
        *i -= block.size;
        current = block.next.as_mut();
    }
    eprintln!("\x1b[31mError: jccc: internal: list index {} out of bounds\x1b[0m", *i);
    -1
}

/// Retrieves an element from the list by index, if it exists.
pub fn lget_element(l: &mut List, index: i32) -> Option<&mut Box<dyn Any>> {
    match lfind_index_internal(&mut l.head, index) {
        Some((ptr, local_idx)) => {
            let block = unsafe { &mut *ptr };
            if local_idx >= block.full {
                eprintln!("\x1b[31mError: jccc: internal: list index {} out of bounds\x1b[0m", index);
                return None;
            }
            Some(&mut block.array[local_idx as usize])
        }
        None => None,
    }
}

/// Sets an element in the list by index.
pub fn lset_element(l: &mut List, index: i32, value: Box<dyn Any>) -> i32 {
    match lfind_index_internal(&mut l.head, index) {
        Some((ptr, local_idx)) => {
            let block = unsafe { &mut *ptr };
            if local_idx >= block.full {
                eprintln!("\x1b[31mError: jccc: internal: list index {} out of bounds\x1b[0m", index);
                return -1;
            }
            block.array[local_idx as usize] = value;
            0
        }
        None => -1,
    }
}

/// Adds an element to the list.
pub fn ladd_element(l: &mut List, element: Box<dyn Any>) -> i32 {
    if l.head.is_none() {
        let mut block = new_block(l);
        block.array.push(element);
        block.full = 1;
        let ptr: *mut ListBlock = &mut *block;
        l.head = Some(block);
        l.tail = Some(ptr);
        return 0;
    }

    // Get the tail block
    if let Some(tail_ptr) = l.tail {
        let tail = unsafe { &mut *tail_ptr };
        if tail.full < tail.size {
            tail.array.push(element);
            tail.full += 1;
        } else {
            let mut new_blk = new_block(l);
            new_blk.array.push(element);
            new_blk.full = 1;
            let new_ptr: *mut ListBlock = &mut *new_blk;
            tail.next = Some(new_blk);
            l.tail = Some(new_ptr);
        }
    }
    0
}

/// Iterates over the list with a provided function.
pub fn literate(l: &mut List, func: fn(&mut Box<dyn Any>) -> i32) -> i32 {
    let mut acc = 0;
    let mut current = l.head.as_mut();
    while let Some(block) = current {
        for i in 0..block.full as usize {
            acc += func(&mut block.array[i]);
        }
        current = block.next.as_mut();
    }
    acc
}
