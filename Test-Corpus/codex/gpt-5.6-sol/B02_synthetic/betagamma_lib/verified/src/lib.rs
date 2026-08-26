use std::ffi::{c_char, c_int, c_void};
use std::ptr;

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

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn strcpy(destination: *mut c_char, source: *const c_char) -> *mut c_char;
}

fn fixed_name(value: &[u8]) -> [c_char; 32] {
    let mut name = [0; 32];
    for (destination, source) in name.iter_mut().zip(value) {
        *destination = *source as c_char;
    }
    name
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id,
        name: [0; 32],
        flags: 0,
    };

    unsafe {
        strcpy(block.name.as_mut_ptr(), name);
    }
    block.flags = flags;

    block
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    let block = unsafe { malloc(size_of::<MemoryBlock>()) }.cast::<MemoryBlock>();
    if block.is_null() {
        return ptr::null_mut();
    }

    let data = unsafe { calloc(count, size_of::<c_int>()) }.cast::<c_int>();
    unsafe {
        (*block).data = data;
    }
    if data.is_null() {
        unsafe {
            free(block.cast());
        }
        return ptr::null_mut();
    }

    unsafe {
        (*block).size = count;
    }

    for index in 0..count {
        unsafe {
            *data.add(index) = init_value.wrapping_add(index as c_int);
        }
    }

    block
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_block(block: *mut MemoryBlock) {
    if !block.is_null() {
        let data = unsafe { (*block).data };
        if !data.is_null() {
            unsafe {
                free(data.cast());
            }
        }
        unsafe {
            free(block.cast());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_hash(block1: *mut MemoryBlock, block2: *mut MemoryBlock) -> c_int {
    if block1.is_null() || block2.is_null() {
        unsafe {
            strcpy(ptr::null_mut(), c"x".as_ptr());
        }
    }

    let mut hash: c_int = 0;
    let data1 = unsafe { (*block1).data } as usize;
    let data2 = unsafe { (*block2).data } as usize;

    if data1 < data2 {
        hash += 100;
    } else if data1 > data2 {
        hash += 200;
    }

    if (block1 as usize) < (block2 as usize) {
        hash += 10;
    } else if (block1 as usize) > (block2 as usize) {
        hash += 20;
    }

    hash
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn betagamma(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let blocks = [
        DataBlock {
            id: 1,
            name: fixed_name(b"Block_Alpha"),
            flags: 0b1010_1010,
        },
        DataBlock {
            id: 2,
            name: fixed_name(b"Block_Beta"),
            flags: 0b1100_1100,
        },
        DataBlock {
            id: 3,
            name: fixed_name(b"Block_Gamma"),
            flags: 0b1111_0000,
        },
    ];

    let mut result: c_int = 0;

    for current in &blocks {
        let mut temp_name: [c_char; 32] = [0; 32];
        unsafe {
            strcpy(temp_name.as_mut_ptr(), current.name.as_ptr());
        }

        let mut flag_contribution: c_int = 0;
        if current.flags & 0b0000_1111 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param1);
        }
        if current.flags & 0b1111_0000 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param2);
        }
        if current.flags & 0b1010_1010 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param3);
        }
        if current.flags & 0b0101_0101 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param4);
        }

        result = result.wrapping_add(flag_contribution.wrapping_mul(current.id));
    }

    let block_size = ((param1 % 10).wrapping_add(5)) as usize;
    let mem1 = unsafe { allocate_block(block_size, param1) };
    let mem2 = unsafe { allocate_block(block_size, param2) };

    if mem1.is_null() || mem2.is_null() {
        unsafe {
            free_block(mem1);
            free_block(mem2);
        }
        return -1;
    }

    let hash = unsafe { compute_hash(mem1, mem2) };
    result = result.wrapping_add(hash);

    let mut sum1: c_int = 0;
    let mut sum2: c_int = 0;
    unsafe {
        for index in 0..(*mem1).size {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(index));
        }
        for index in 0..(*mem2).size {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(index));
        }
    }

    result = result.wrapping_add(sum1.wrapping_sub(sum2) / 10);

    let mut special = DataBlock {
        id: 99,
        name: fixed_name(b"Special"),
        flags: 0b1111_1111,
    };
    unsafe {
        strcpy(special.name.as_mut_ptr(), c"Modified".as_ptr());
    }

    unsafe {
        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        if ((*mem1).data as usize) > 0 && ((*mem2).data as usize) > 0 {
            result = result.wrapping_add(special.flags as c_int);
        }

        free_block(mem1);
        free_block(mem2);
    }

    result
}
