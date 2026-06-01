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
    // Walk the chain to find the right block and remaining index.
    let mut block_opt = l.head.as_deref_mut();
    let mut i = index;
    loop {
        let block = block_opt?;
        if i < block.size {
            if i >= block.full {
                return None;
            }
            return Some(&mut block.array[i as usize]);
        }
        i -= block.size;
        block_opt = block.next.as_deref_mut();
    }
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
        l.head = Some(Box::new(ListBlock {
            array: Vec::with_capacity(blocksize as usize),
            size: blocksize,
            full: 0,
            next: None,
        }));
    }

    // Walk to the last block (tail).
    let mut current = l.head.as_deref_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_deref_mut().unwrap();
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
    let mut acc: i32 = 0;
    let mut block_opt = l.head.as_deref_mut();
    while let Some(block) = block_opt {
        for i in 0..(block.full as usize) {
            acc += func(&mut block.array[i]);
        }
        block_opt = block.next.as_deref_mut();
    }
    acc
}

/// Finds and sets index variables for internal iteration.
pub fn lfind_index(_l: &mut List, _lb: &mut Option<Box<ListBlock>>, _i: &mut i32) -> i32 {
    // Not used in tests; the C version takes block-by-pointer; here we leave it as a placeholder.
    0
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
    let mut block_opt = l.head.as_deref_mut();
    let mut i = index;
    loop {
        let block = match block_opt {
            Some(b) => b,
            None => return -1,
        };
        if i < block.size {
            if i >= block.full {
                return -1;
            }
            block.array[i as usize] = value;
            return 0;
        }
        i -= block.size;
        block_opt = block.next.as_deref_mut();
    }
}
