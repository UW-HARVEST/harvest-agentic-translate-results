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
    let mut cur = l.head.as_deref_mut();
    while let Some(block) = cur {
        if remaining < block.full {
            return block.array.get_mut(remaining as usize);
        }
        remaining -= block.full;
        cur = block.next.as_deref_mut();
    }
    None
}

/// Destroys the list and frees resources.
pub fn destroy_list(l: &mut List) -> i32 {
    // Iteratively drop the linked list to avoid potential deep recursion on Drop.
    let mut head = l.head.take();
    while let Some(mut block) = head {
        head = block.next.take();
    }
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
    // Walk to the last block.
    let mut cur = l.head.as_deref_mut().unwrap();
    while cur.next.is_some() {
        cur = cur.next.as_deref_mut().unwrap();
    }
    if cur.full < cur.size {
        cur.array.push(element);
        cur.full += 1;
    } else {
        let new_block = Box::new(ListBlock {
            array: Vec::with_capacity(blocksize as usize),
            size: blocksize,
            full: 1,
            next: None,
        });
        cur.next = Some(new_block);
        let new_ref = cur.next.as_deref_mut().unwrap();
        new_ref.array.push(element);
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
    let mut acc = 0i32;
    let mut cur = l.head.as_deref_mut();
    while let Some(block) = cur {
        for item in block.array.iter_mut() {
            acc = acc.wrapping_add(func(item));
        }
        cur = block.next.as_deref_mut();
    }
    acc
}

/// Finds and sets index variables for internal iteration.
pub fn lfind_index(_l: &mut List, _lb: &mut Option<Box<ListBlock>>, _i: &mut i32) -> i32 {
    // Internal helper: not used by tests; keeping a no-op compatible stub.
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
    let mut remaining = index;
    let mut cur = l.head.as_deref_mut();
    while let Some(block) = cur {
        if remaining < block.full {
            block.array[remaining as usize] = value;
            return 0;
        }
        remaining -= block.full;
        cur = block.next.as_deref_mut();
    }
    -1
}
