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

/// Internal: given a list and an index, walk blocks to find the right block and local index.
/// Returns (reference to block, local index) or None if out of bounds.
fn find_block_and_index(block: &mut Option<Box<ListBlock>>, mut index: i32) -> Option<(&mut ListBlock, i32)> {
    if index < 0 {
        return None;
    }
    let mut cur = block.as_mut()?;
    while index >= cur.size {
        index -= cur.size;
        cur = cur.next.as_mut()?;
    }
    Some((cur, index))
}

/// Retrieves an element from the list by index, if it exists.
pub fn lget_element(l: &mut List, index: i32) -> Option<&mut Box<dyn Any>> {
    let (block, local_i) = find_block_and_index(&mut l.head, index)?;
    if local_i >= block.full {
        return None;
    }
    Some(&mut block.array[local_i as usize])
}

/// Sets an element in the list by index.
pub fn lset_element(l: &mut List, index: i32, value: Box<dyn Any>) -> i32 {
    match find_block_and_index(&mut l.head, index) {
        Some((block, local_i)) => {
            if local_i >= block.full {
                return -1;
            }
            block.array[local_i as usize] = value;
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
        let ptr = &mut *block as *mut ListBlock;
        l.head = Some(block);
        l.tail = Some(ptr);
        return 0;
    }
    // Navigate to the tail block via the raw pointer
    let tail_ptr = l.tail.unwrap();
    let tail = unsafe { &mut *tail_ptr };
    if tail.full < tail.size {
        tail.array.push(element);
        tail.full += 1;
    } else {
        let mut nb = new_block(l);
        nb.array.push(element);
        nb.full = 1;
        let ptr = &mut *nb as *mut ListBlock;
        tail.next = Some(nb);
        l.tail = Some(ptr);
    }
    0
}

/// Iterates over the list with a provided function.
pub fn literate(l: &mut List, func: fn(&mut Box<dyn Any>) -> i32) -> i32 {
    let mut acc = 0;
    let mut cur = l.head.as_mut();
    while let Some(block) = cur {
        for i in 0..block.full as usize {
            acc += func(&mut block.array[i]);
        }
        cur = block.next.as_mut();
    }
    acc
}

/// Finds and sets index variables for internal iteration.
pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    *lb = None;
    if *i < 0 {
        return -1;
    }
    let mut cur = &l.head;
    while let Some(block) = cur {
        if *i < block.size {
            return 0;
        }
        *i -= block.size;
        cur = &block.next;
    }
    -1
}
