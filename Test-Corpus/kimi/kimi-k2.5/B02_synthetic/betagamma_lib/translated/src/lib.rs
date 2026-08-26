use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::slice;

#[repr(C)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: u8,
}

#[repr(C)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

pub fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id,
        name: [0; 32],
        flags,
    };
    unsafe {
        let name_len = libc::strlen(name);
        let copy_len = name_len.min(31);
        ptr::copy_nonoverlapping(name, block.name.as_mut_ptr(), copy_len);
        block.name[copy_len] = 0;
    }
    block
}

pub fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    unsafe {
        let layout = Layout::new::<MemoryBlock>();
        let mb = alloc(layout) as *mut MemoryBlock;
        if mb.is_null() {
            return ptr::null_mut();
        }

        let data_layout = Layout::array::<c_int>(count).unwrap();
        let data = alloc(data_layout) as *mut c_int;
        if data.is_null() {
            dealloc(mb as *mut u8, layout);
            return ptr::null_mut();
        }

        ptr::write_bytes(data, 0, count);

        for i in 0..count {
            *data.add(i) = init_value + i as c_int;
        }

        (*mb).data = data;
        (*mb).size = count;

        mb
    }
}

pub fn free_block(mb: *mut MemoryBlock) {
    unsafe {
        if !mb.is_null() {
            if !(*mb).data.is_null() {
                let data_layout = Layout::array::<c_int>((*mb).size).unwrap();
                dealloc((*mb).data as *mut u8, data_layout);
            }
            let layout = Layout::new::<MemoryBlock>();
            dealloc(mb as *mut u8, layout);
        }
    }
}

pub fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

    unsafe {
        if (*mb1).data < (*mb2).data {
            hash += 100;
        } else if (*mb1).data > (*mb2).data {
            hash += 200;
        }

        if mb1 < mb2 {
            hash += 10;
        } else if mb1 > mb2 {
            hash += 20;
        }
    }

    hash
}

#[unsafe(no_mangle)]
pub extern "C" fn betagamma(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut blocks: [DataBlock; 3] = [
        DataBlock {
            id: 1,
            name: [0; 32],
            flags: 0b10101010,
        },
        DataBlock {
            id: 2,
            name: [0; 32],
            flags: 0b11001100,
        },
        DataBlock {
            id: 3,
            name: [0; 32],
            flags: 0b11110000,
        },
    ];

    unsafe {
        let name1 = b"Block_Alpha\0".as_ptr() as *const c_char;
        let name2 = b"Block_Beta\0".as_ptr() as *const c_char;
        let name3 = b"Block_Gamma\0".as_ptr() as *const c_char;
        ptr::copy_nonoverlapping(name1, blocks[0].name.as_mut_ptr(), 12);
        ptr::copy_nonoverlapping(name2, blocks[1].name.as_mut_ptr(), 11);
        ptr::copy_nonoverlapping(name3, blocks[2].name.as_mut_ptr(), 12);
    }

    for i in 0..3 {
        let current = &blocks[i];

        let mut flag_contribution: c_int = 0;
        if current.flags & 0b00001111 != 0 {
            flag_contribution += param1;
        }
        if current.flags & 0b11110000 != 0 {
            flag_contribution += param2;
        }
        if current.flags & 0b10101010 != 0 {
            flag_contribution += param3;
        }
        if current.flags & 0b01010101 != 0 {
            flag_contribution += param4;
        }

        result += flag_contribution * current.id;
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

    let mut sum1: c_int = 0;
    let mut sum2: c_int = 0;
    unsafe {
        let slice1 = slice::from_raw_parts((*mem1).data, (*mem1).size);
        let slice2 = slice::from_raw_parts((*mem2).data, (*mem2).size);
        for &val in slice1 {
            sum1 += val;
        }
        for &val in slice2 {
            sum2 += val;
        }
    }

    result += (sum1 - sum2) / 10;

    let mut special = DataBlock {
        id: 99,
        name: [0; 32],
        flags: 0b11111111,
    };
    unsafe {
        let special_name = b"Modified\0".as_ptr() as *const c_char;
        ptr::copy_nonoverlapping(special_name, special.name.as_mut_ptr(), 9);
    }

    unsafe {
        if (*mem1).data != (*mem2).data {
            result += special.id;
        }

        if !(*mem1).data.is_null() && !(*mem2).data.is_null() {
            result += special.flags as c_int;
        }
    }

    free_block(mem1);
    free_block(mem2);

    result
}
