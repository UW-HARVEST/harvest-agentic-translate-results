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
    let mut i = index;
    let mut current = l.head.as_mut()?;
    while i >= current.size {
        i -= current.size;
        current = current.next.as_mut()?;
    }
    if i >= current.full {
        return None;
    }
    current.array.get_mut(i as usize)
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
        let blocksize = l.blocksize;
        l.head = Some(Box::new(ListBlock {
            array: Vec::with_capacity(blocksize as usize),
            size: blocksize,
            full: 0,
            next: None,
        }));
    }
    // Walk to last block
    let mut current = l.head.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    if current.full < current.size {
        current.array.push(element);
        current.full += 1;
    } else {
        let blocksize = current.size;
        let new = Box::new(ListBlock {
            array: Vec::with_capacity(blocksize as usize),
            size: blocksize,
            full: 0,
            next: None,
        });
        current.next = Some(new);
        let next = current.next.as_mut().unwrap();
        next.array.push(element);
        next.full += 1;
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
    let mut current = l.head.as_mut();
    while let Some(block) = current {
        for i in 0..block.full as usize {
            acc += func(&mut block.array[i]);
        }
        current = block.next.as_mut();
    }
    acc
}
/// Finds and sets index variables for internal iteration.
pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    *lb = None;
    if *i < 0 {
        return -1;
    }
    let mut current = match l.head.as_ref() {
        Some(c) => c.as_ref(),
        None => return -1,
    };
    while *i >= current.size {
        *i -= current.size;
        match current.next.as_ref() {
            Some(c) => current = c.as_ref(),
            None => return -1,
        }
    }
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
    let mut i = index;
    let mut current = match l.head.as_mut() {
        Some(c) => c,
        None => return -1,
    };
    while i >= current.size {
        i -= current.size;
        current = match current.next.as_mut() {
            Some(c) => c,
            None => return -1,
        };
    }
    if i >= current.full {
        return -1;
    }
    current.array[i as usize] = value;
    0
}
