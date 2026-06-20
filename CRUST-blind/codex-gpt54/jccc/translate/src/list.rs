use std::any::Any;

fn make_block(blocksize: i32) -> Box<ListBlock> {
    Box::new(ListBlock {
        array: Vec::with_capacity(blocksize.max(0) as usize),
        size: blocksize.max(0),
        full: 0,
        next: None,
    })
}
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
    loop {
        if current.size <= 0 {
            return None;
        }
        if i < current.size {
            break;
        }
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
    if l.blocksize <= 0 {
        return -1;
    }
    let blocksize = l.blocksize;

    if l.head.is_none() {
        let mut block = make_block(blocksize);
        let raw = &mut *block as *mut ListBlock;
        l.tail = Some(raw);
        l.head = Some(block);
    }

    let mut current = match l.head.as_mut() {
        Some(current) => current,
        None => return -1,
    };

    while current.next.is_some() {
        current = match current.next.as_mut() {
            Some(next) => next,
            None => return -1,
        };
    }

    if current.full < current.size {
        current.array.push(element);
        current.full += 1;
        l.tail = Some(&mut **current as *mut ListBlock);
        return 0;
    }

    let mut block = make_block(blocksize);
    block.array.push(element);
    block.full = 1;
    let raw = &mut *block as *mut ListBlock;
    current.next = Some(block);
    l.tail = Some(raw);
    0
}
/// Allocates a new block and links it into the list.
pub fn new_block(l: &mut List) -> Box<ListBlock> {
    make_block(l.blocksize)
}
/// Iterates over the list with a provided function.
pub fn literate(l: &mut List, func: fn(&mut Box<dyn Any>) -> i32) -> i32 {
    let mut acc = 0;
    let mut current = l.head.as_mut();
    while let Some(block) = current {
        for item in block.array.iter_mut().take(block.full.max(0) as usize) {
            acc += func(item);
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

    let mut remaining = *i;
    let mut current = match l.head.as_ref() {
        Some(current) => current.as_ref(),
        None => return -1,
    };

    loop {
        if current.size <= 0 {
            return -1;
        }
        if remaining < current.size {
            *i = remaining;
            return 0;
        }
        remaining -= current.size;
        current = match current.next.as_ref() {
            Some(next) => next.as_ref(),
            None => return -1,
        };
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

    let mut i = index;
    let mut current = match l.head.as_mut() {
        Some(current) => current,
        None => return -1,
    };

    loop {
        if current.size <= 0 {
            return -1;
        }
        if i < current.size {
            break;
        }
        i -= current.size;
        current = match current.next.as_mut() {
            Some(next) => next,
            None => return -1,
        };
    }

    if i >= current.full {
        return -1;
    }

    if let Some(slot) = current.array.get_mut(i as usize) {
        *slot = value;
        0
    } else {
        -1
    }
}
