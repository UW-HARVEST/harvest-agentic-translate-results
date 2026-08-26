use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::ffi::{c_char, c_int};
use std::ptr;

#[repr(C)]
pub struct DataBlock {
    id: c_int,
    name: [c_char; 32],
    flags: u8,
}

#[repr(C)]
pub struct MemoryBlock {
    data: *mut c_int,
    size: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id,
        name: [0; 32],
        flags,
    };
    unsafe {
        let mut i = 0;
        while i < 31 && *name.add(i) != 0 {
            block.name[i] = *name.add(i);
            i += 1;
        }
        block.name[i] = 0;
    }
    block
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    unsafe {
        let layout_mb = Layout::new::<MemoryBlock>();
        let mb = alloc_zeroed(layout_mb) as *mut MemoryBlock;
        if mb.is_null() {
            return ptr::null_mut();
        }

        let layout_data = match Layout::array::<c_int>(count) {
            Ok(l) => l,
            Err(_) => {
                dealloc(mb as *mut u8, layout_mb);
                return ptr::null_mut();
            }
        };
        let data = alloc_zeroed(layout_data) as *mut c_int;
        if data.is_null() {
            dealloc(mb as *mut u8, layout_mb);
            return ptr::null_mut();
        }

        (*mb).data = data;
        (*mb).size = count;

        for i in 0..count {
            *data.add(i) = init_value + i as c_int;
        }

        mb
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_block(mb: *mut MemoryBlock) {
    unsafe {
        if !mb.is_null() {
            if !(*mb).data.is_null() {
                let layout_data = Layout::array::<c_int>((*mb).size).unwrap();
                dealloc((*mb).data as *mut u8, layout_data);
            }
            dealloc(mb as *mut u8, Layout::new::<MemoryBlock>());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    unsafe {
        let mut hash: c_int = 0;

        if ((*mb1).data as usize) < ((*mb2).data as usize) {
            hash += 100;
        } else if ((*mb1).data as usize) > ((*mb2).data as usize) {
            hash += 200;
        }

        if (mb1 as usize) < (mb2 as usize) {
            hash += 10;
        } else if (mb1 as usize) > (mb2 as usize) {
            hash += 20;
        }

        hash
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn betagamma(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let block_ids: [c_int; 3] = [1, 2, 3];
    let block_flags: [u8; 3] = [0b10101010, 0b11001100, 0b11110000];

    for i in 0..3 {
        let flags = block_flags[i];
        let id = block_ids[i];

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

        result += flag_contribution * id;
    }

    let block_size = ((param1 % 10) + 5) as usize;
    let mem1 = allocate_block(block_size, param1);
    let mem2 = allocate_block(block_size, param2);

    if mem1.is_null() || mem2.is_null() {
        free_block(mem1);
        free_block(mem2);
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

        let special_id: c_int = 99;
        let special_flags: u8 = 0b11111111;

        // mem1->data != mem2->data (pointer comparison, always true for separate allocs)
        if (*mem1).data != (*mem2).data {
            result += special_id;
        }

        // mem1->data > NULL && mem2->data > NULL (always true for non-null)
        if !(*mem1).data.is_null() && !(*mem2).data.is_null() {
            result += special_flags as c_int;
        }
    }

    free_block(mem1);
    free_block(mem2);

    result
}
