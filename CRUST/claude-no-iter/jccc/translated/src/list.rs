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
    let mut node = l.head.as_deref_mut()?;
    loop {
        if remaining < node.full {
            return node.array.get_mut(remaining as usize);
        }
        if remaining < node.size {
            // We're inside this block but past the filled portion.
            return None;
        }
        remaining -= node.size;
        node = node.next.as_deref_mut()?;
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
    if l.head.is_none() {
        l.head = Some(new_block(l));
    }
    let blocksize = l.blocksize;
    // Walk to the tail block.
    let mut node = l.head.as_deref_mut().unwrap();
    while node.next.is_some() {
        node = node.next.as_deref_mut().unwrap();
    }
    if node.full < node.size {
        node.array.push(element);
        node.full += 1;
    } else {
        let new_b = Box::new(ListBlock {
            array: Vec::with_capacity(blocksize.max(0) as usize),
            size: blocksize,
            full: 0,
            next: None,
        });
        node.next = Some(new_b);
        let nxt = node.next.as_deref_mut().unwrap();
        nxt.array.push(element);
        nxt.full += 1;
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
    let mut acc = 0i32;
    let mut node = l.head.as_deref_mut();
    while let Some(n) = node {
        for elem in n.array.iter_mut() {
            acc += func(elem);
        }
        node = n.next.as_deref_mut();
    }
    acc
}

/// Finds and sets index variables for internal iteration. Provided for API
/// completeness; mirrors the C `lfind_index` helper at a high level.
pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    if *i < 0 {
        return -1;
    }
    // Detach the head into the out-parameter to mirror the C API semantics.
    *lb = l.head.take();
    let mut working: Option<Box<ListBlock>> = lb.take();
    while let Some(mut node) = working {
        if *i < node.size {
            *lb = Some(node);
            return 0;
        }
        *i -= node.size;
        working = node.next.take();
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
    let mut node = match l.head.as_deref_mut() {
        Some(h) => h,
        None => return -1,
    };
    loop {
        if remaining < node.full {
            node.array[remaining as usize] = value;
            return 0;
        }
        if remaining < node.size {
            return -1;
        }
        remaining -= node.size;
        node = match node.next.as_deref_mut() {
            Some(n) => n,
            None => return -1,
        };
    }
}
