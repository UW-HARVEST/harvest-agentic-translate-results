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
    let mut current = l.head.as_mut()?;
    loop {
        if remaining < current.full {
            return current.array.get_mut(remaining as usize);
        }
        if remaining < current.size {
            // Index in this block but past `full` is out of bounds
            return None;
        }
        remaining -= current.size;
        match current.next.as_mut() {
            Some(n) => current = n,
            None => return None,
        }
    }
}

/// Destroys the list and frees resources.
pub fn destroy_list(l: &mut List) -> i32 {
    // Iteratively drop the chain to avoid recursion-induced stack overflow.
    let mut head = l.head.take();
    while let Some(mut block) = head {
        head = block.next.take();
    }
    l.tail = None;
    0
}

/// Adds an element to the list.
pub fn ladd_element(l: &mut List, element: Box<dyn Any>) -> i32 {
    if l.head.is_none() {
        let block = new_block(l);
        l.head = Some(block);
    }

    // Walk to the last block.
    let mut node = l.head.as_mut().unwrap();
    while node.next.is_some() {
        node = node.next.as_mut().unwrap();
    }

    if node.full < node.size {
        node.array.push(element);
        node.full += 1;
        0
    } else {
        let blocksize = l.blocksize;
        let mut new = Box::new(ListBlock {
            array: Vec::with_capacity(blocksize.max(0) as usize),
            size: blocksize,
            full: 0,
            next: None,
        });
        new.array.push(element);
        new.full += 1;
        node.next = Some(new);
        0
    }
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
        for elem in block.array.iter_mut() {
            acc += func(elem);
        }
        current = block.next.as_mut();
    }
    acc
}

/// Finds and sets index variables for internal iteration.
pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    // Take the head out of the list to allow exclusive ownership transfer to `lb`.
    *lb = l.head.take();
    if *i < 0 {
        return -1;
    }
    loop {
        let block = match lb.as_mut() {
            Some(b) => b,
            None => return -1,
        };
        if *i < block.size {
            return 0;
        }
        let next = block.next.take();
        match next {
            Some(n) => {
                *i -= block.size;
                *lb = Some(n);
            }
            None => return -1,
        }
    }
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
    let mut current = match l.head.as_mut() {
        Some(c) => c,
        None => return -1,
    };
    loop {
        if remaining < current.full {
            current.array[remaining as usize] = value;
            return 0;
        }
        if remaining < current.size {
            return -1;
        }
        remaining -= current.size;
        match current.next.as_mut() {
            Some(n) => current = n,
            None => return -1,
        }
    }
}
