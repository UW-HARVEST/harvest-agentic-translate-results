use std::os::raw::c_int;

struct MemoryBlock {
    data: *mut c_int,
    size: usize,
}

fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    let mb = Box::into_raw(Box::new(MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    }));
    unsafe {
        let layout = std::alloc::Layout::array::<c_int>(count).unwrap();
        let ptr = std::alloc::alloc_zeroed(layout) as *mut c_int;
        if ptr.is_null() {
            drop(Box::from_raw(mb));
            return std::ptr::null_mut();
        }
        (*mb).data = ptr;
        (*mb).size = count;
        for i in 0..count {
            *ptr.add(i) = init_value + i as c_int;
        }
    }
    mb
}

unsafe fn free_block(mb: *mut MemoryBlock) {
    if !mb.is_null() {
        if !(*mb).data.is_null() {
            let layout = std::alloc::Layout::array::<c_int>((*mb).size).unwrap();
            std::alloc::dealloc((*mb).data as *mut u8, layout);
        }
        drop(Box::from_raw(mb));
    }
}

fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;
    unsafe {
        let d1 = (*mb1).data as usize;
        let d2 = (*mb2).data as usize;
        if d1 < d2 {
            hash += 100;
        } else if d1 > d2 {
            hash += 200;
        }

        let p1 = mb1 as usize;
        let p2 = mb2 as usize;
        if p1 < p2 {
            hash += 10;
        } else if p1 > p2 {
            hash += 20;
        }
    }
    hash
}

#[unsafe(no_mangle)]
pub extern "C" fn betagamma(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let block_ids: [c_int; 3] = [1, 2, 3];
    let block_flags: [u8; 3] = [0b10101010, 0b11001100, 0b11110000];

    for i in 0..3 {
        let flags = block_flags[i];
        let mut flag_contribution: c_int = 0;
        if flags & 0b00001111 != 0 {
            flag_contribution += param1;
        }
        if flags & 0b11110000 != 0 {
            flag_contribution += param2;
        }
        if flags & 0b10101010 != 0 {
            flag_contribution += param3;
        }
        if flags & 0b01010101 != 0 {
            flag_contribution += param4;
        }
        result += flag_contribution * block_ids[i];
    }

    let block_size = ((param1 % 10) + 5) as usize;
    let mem1 = allocate_block(block_size, param1);
    let mem2 = allocate_block(block_size, param2);

    if mem1.is_null() || mem2.is_null() {
        unsafe {
            free_block(mem1);
            free_block(mem2);
        }
        return -1;
    }

    let hash = compute_hash(mem1, mem2);
    result += hash;

    unsafe {
        let mut sum1: c_int = 0;
        let mut sum2: c_int = 0;
        for i in 0..(*mem1).size {
            sum1 += *(*mem1).data.add(i);
        }
        for i in 0..(*mem2).size {
            sum2 += *(*mem2).data.add(i);
        }
        result += (sum1 - sum2) / 10;

        // mem1->data != mem2->data is always true for separate allocations
        if (*mem1).data != (*mem2).data {
            result += 99; // special.id
        }

        // mem1->data > NULL && mem2->data > NULL — always true for valid allocs
        if ((*mem1).data as usize) > 0 && ((*mem2).data as usize) > 0 {
            result += 0b11111111_u8 as c_int; // special.flags = 0xFF
        }

        free_block(mem1);
        free_block(mem2);
    }

    result
}
