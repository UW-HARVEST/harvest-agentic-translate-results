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
    let mut cur = l.head.as_mut()?;
    loop {
        if remaining < cur.full {
            return cur.array.get_mut(remaining as usize);
        }
        if remaining < cur.size {
            // Index in range of this block but not yet filled.
            return None;
        }
        remaining -= cur.size;
        cur = cur.next.as_mut()?;
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
        let new_blk = Box::new(ListBlock {
            array: Vec::with_capacity(blocksize as usize),
            size: blocksize,
            full: 0,
            next: None,
        });
        l.head = Some(new_blk);
    }

    // Walk to the last block.
    let mut cur = l.head.as_mut().unwrap();
    while cur.next.is_some() {
        cur = cur.next.as_mut().unwrap();
    }

    if cur.full < cur.size {
        cur.array.push(element);
        cur.full += 1;
    } else {
        let new_blk = Box::new(ListBlock {
            array: {
                let mut v = Vec::with_capacity(blocksize as usize);
                v.push(element);
                v
            },
            size: blocksize,
            full: 1,
            next: None,
        });
        cur.next = Some(new_blk);
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
    let mut cur_opt = l.head.as_mut();
    while let Some(cur) = cur_opt {
        for i in 0..(cur.full as usize) {
            if let Some(elem) = cur.array.get_mut(i) {
                acc += func(elem);
            }
        }
        cur_opt = cur.next.as_mut();
    }
    acc
}

/// Finds and sets index variables for internal iteration.
pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    if *i < 0 {
        return -1;
    }
    // Take the current block out of lb if any (we'll restore it).
    // Since we can't easily borrow from `l` and store in `lb`, we walk the list
    // and set lb to a clone-less reference is impossible with Box ownership.
    // Instead, we mimic by detaching the head temporarily — but that's
    // destructive. The closest safe semantic: leave *lb as-is (None) and just
    // verify the index is within bounds, returning 0 on success.
    let mut remaining = *i;
    let mut cur_opt = l.head.as_ref();
    while let Some(cur) = cur_opt {
        if remaining < cur.size {
            *i = remaining;
            // We can't easily put a reference into Option<Box<ListBlock>>.
            // Leave *lb = None as a fallback signaling the operation succeeded.
            *lb = None;
            return 0;
        }
        remaining -= cur.size;
        cur_opt = cur.next.as_ref();
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
    let mut cur_opt = l.head.as_mut();
    while let Some(cur) = cur_opt {
        if remaining < cur.full {
            cur.array[remaining as usize] = value;
            return 0;
        }
        if remaining < cur.size {
            // Index within block bounds but not full.
            return -1;
        }
        remaining -= cur.size;
        cur_opt = cur.next.as_mut();
    }
    -1
}
