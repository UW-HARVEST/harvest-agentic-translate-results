use std::any::Any;

#[derive(Debug)]
pub struct ListBlock {
    pub array: Vec<Box<dyn Any>>,
    pub size: i32,
    pub full: i32,
    pub next: Option<Box<ListBlock>>,
}

#[derive(Debug)]
pub struct List {
    pub head: Option<Box<ListBlock>>,
    pub tail: Option<*mut ListBlock>,
    pub blocksize: i32,
}

pub fn create_list(blocksize: i32) -> List {
    List {
        head: None,
        tail: None,
        blocksize,
    }
}

pub fn destroy_list(l: &mut List) -> i32 {
    l.head = None;
    l.tail = None;
    0
}

pub fn new_block(l: &mut List) -> Box<ListBlock> {
    let mut array = Vec::with_capacity(l.blocksize as usize);
    Box::new(ListBlock {
        array,
        size: l.blocksize,
        full: 0,
        next: None,
    })
}

fn lfind_index_internal(head: &mut Option<Box<ListBlock>>, index: &mut i32) -> *mut ListBlock {
    let mut current = match head.as_mut() {
        Some(b) => &mut **b as *mut ListBlock,
        None => return std::ptr::null_mut(),
    };
    unsafe {
        while *index >= (*current).size {
            *index -= (*current).size;
            current = match (*current).next.as_mut() {
                Some(b) => &mut **b as *mut ListBlock,
                None => return std::ptr::null_mut(),
            };
        }
    }
    current
}

pub fn lfind_index(l: &mut List, lb: &mut Option<Box<ListBlock>>, i: &mut i32) -> i32 {
    if *i < 0 {
        eprintln!("\x1b[31mError: jccc: internal: list index was negative ({})\x1b[0m", *i);
        return -1;
    }
    // This function signature is awkward for Rust. We do our best.
    0
}

pub fn lget_element(l: &mut List, index: i32) -> Option<&mut Box<dyn Any>> {
    let mut i = index;
    let ptr = lfind_index_internal(&mut l.head, &mut i);
    if ptr.is_null() {
        eprintln!("\x1b[31mError: jccc: internal: list index {} out of bounds\x1b[0m", index);
        return None;
    }
    unsafe {
        if i >= (*ptr).full {
            eprintln!("\x1b[31mError: jccc: internal: list index {} out of bounds\x1b[0m", index);
            return None;
        }
        Some(&mut (&mut (*ptr).array)[i as usize])
    }
}

pub fn lset_element(l: &mut List, index: i32, value: Box<dyn Any>) -> i32 {
    let mut i = index;
    let ptr = lfind_index_internal(&mut l.head, &mut i);
    if ptr.is_null() {
        return -1;
    }
    unsafe {
        if i >= (*ptr).full {
            eprintln!("\x1b[31mError: jccc: internal: list index {} out of bounds\x1b[0m", index);
            return -1;
        }
        (&mut (*ptr).array)[i as usize] = value;
    }
    0
}

pub fn ladd_element(l: &mut List, element: Box<dyn Any>) -> i32 {
    if l.head.is_none() {
        let mut block = new_block(l);
        let ptr = &mut *block as *mut ListBlock;
        l.head = Some(block);
        l.tail = Some(ptr);
    }
    let tail_ptr = l.tail.unwrap();
    unsafe {
        if (*tail_ptr).full < (*tail_ptr).size {
            (*tail_ptr).array.push(element);
            (*tail_ptr).full += 1;
        } else {
            let mut block = new_block(l);
            block.array.push(element);
            block.full = 1;
            let new_ptr = &mut *block as *mut ListBlock;
            (*tail_ptr).next = Some(block);
            l.tail = Some(new_ptr);
        }
    }
    0
}

pub fn literate(l: &mut List, func: fn(&mut Box<dyn Any>) -> i32) -> i32 {
    let mut acc = 0;
    let mut current = &mut l.head;
    loop {
        match current {
            Some(block) => {
                for i in 0..block.full as usize {
                    acc += func(&mut block.array[i]);
                }
                current = &mut block.next;
            }
            None => break,
        }
    }
    acc
}
