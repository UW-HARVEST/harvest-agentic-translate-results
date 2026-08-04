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

fn find_block_mut(
    head: &mut Option<Box<ListBlock>>,
    mut index: i32,
) -> Option<(&mut ListBlock, usize)> {
    let mut current = head.as_deref_mut()?;
    if index < 0 {
        return None;
    }

    while index >= current.size {
        index -= current.size;
        current = current.next.as_deref_mut()?;
    }

    Some((current, index as usize))
}

/// Retrieves an element from the list by index, if it exists.
pub fn lget_element(l: &mut List, index: i32) -> Option<&mut Box<dyn Any>> {
    let (block, inner_index) = find_block_mut(&mut l.head, index)?;
    if inner_index >= block.full as usize {
        return None;
    }
    block.array.get_mut(inner_index)
}
/// Destroys the list and frees resources.
pub fn destroy_list(l: &mut List) -> i32 {
    l.head = None;
    l.tail = None;
    0
}
/// Adds an element to the list.
pub fn ladd_element(l: &mut List, element: Box<dyn Any>) -> i32 {
    let blocksize = l.blocksize;

    if l.head.is_none() {
        let mut block = new_block(l);
        block.array.push(element);
        block.full = 1;
        let raw_tail = &mut *block as *mut ListBlock;
        l.head = Some(block);
        l.tail = Some(raw_tail);
        return 0;
    }

    let mut current = l.head.as_deref_mut().expect("head checked above");
    while current.next.is_some() {
        current = current.next.as_deref_mut().expect("next checked above");
    }

    if current.full < current.size {
        current.array.push(element);
        current.full += 1;
        l.tail = Some(current as *mut ListBlock);
        return 0;
    }

    let mut block = Box::new(ListBlock {
        array: vec![element],
        size: blocksize,
        full: 1,
        next: None,
    });
    let raw_tail = &mut *block as *mut ListBlock;
    current.next = Some(block);
    l.tail = Some(raw_tail);
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
    let mut current = l.head.as_deref_mut();
    while let Some(block) = current {
        for value in block.array.iter_mut().take(block.full as usize) {
            acc += func(value);
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

    let mut current = l.head.as_deref();
    while let Some(block) = current {
        if *i < block.size {
            *lb = None;
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
    let Some((block, inner_index)) = find_block_mut(&mut l.head, index) else {
        return -1;
    };
    if inner_index >= block.full as usize {
        return -1;
    }
    block.array[inner_index] = value;
    0
}
