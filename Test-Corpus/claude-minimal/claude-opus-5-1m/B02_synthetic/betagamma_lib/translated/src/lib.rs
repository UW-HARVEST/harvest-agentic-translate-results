// Rust translation of c_src/src/lib.c

use std::os::raw::c_char;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: i32,
    pub name: [c_char; 32],
    pub flags: u8,
}

#[repr(C)]
pub struct MemoryBlock {
    pub data: *mut i32,
    pub size: usize,
}

/// Copy a C-string (null-terminated) into a fixed-size byte array. The
/// destination must be large enough to hold the string plus a NUL terminator.
unsafe fn copy_cstr(dest: &mut [c_char; 32], src: &str) {
    // Zero out destination first
    for byte in dest.iter_mut() {
        *byte = 0;
    }
    let bytes = src.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i >= dest.len() - 1 {
            break;
        }
        dest[i] = *b as c_char;
    }
}

pub fn create_block(id: i32, name: &str, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id,
        name: [0; 32],
        flags,
    };
    unsafe {
        copy_cstr(&mut block.name, name);
    }
    block
}

pub fn allocate_block(count: usize, init_value: i32) -> *mut MemoryBlock {
    unsafe {
        let layout = std::alloc::Layout::new::<MemoryBlock>();
        let mb = std::alloc::alloc(layout) as *mut MemoryBlock;
        if mb.is_null() {
            return ptr::null_mut();
        }

        // Allocate zero-initialized memory for `count` i32 values.
        let data_layout = match std::alloc::Layout::array::<i32>(count) {
            Ok(l) => l,
            Err(_) => {
                std::alloc::dealloc(mb as *mut u8, layout);
                return ptr::null_mut();
            }
        };

        let data_ptr = if count == 0 {
            ptr::null_mut()
        } else {
            std::alloc::alloc_zeroed(data_layout) as *mut i32
        };

        if count != 0 && data_ptr.is_null() {
            std::alloc::dealloc(mb as *mut u8, layout);
            return ptr::null_mut();
        }

        (*mb).data = data_ptr;
        (*mb).size = count;

        for i in 0..count {
            *data_ptr.add(i) = init_value.wrapping_add(i as i32);
        }

        mb
    }
}

pub fn free_block(mb: *mut MemoryBlock) {
    unsafe {
        if !mb.is_null() {
            if !(*mb).data.is_null() {
                let count = (*mb).size;
                if count != 0 {
                    if let Ok(data_layout) = std::alloc::Layout::array::<i32>(count) {
                        std::alloc::dealloc((*mb).data as *mut u8, data_layout);
                    }
                }
            }
            let layout = std::alloc::Layout::new::<MemoryBlock>();
            std::alloc::dealloc(mb as *mut u8, layout);
        }
    }
}

pub fn compute_hash(mb1: *const MemoryBlock, mb2: *const MemoryBlock) -> i32 {
    let mut hash = 0i32;
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

pub fn betagamma(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result: i32 = 0;

    let blocks: [DataBlock; 3] = [
        create_block(1, "Block_Alpha", 0b1010_1010),
        create_block(2, "Block_Beta", 0b1100_1100),
        create_block(3, "Block_Gamma", 0b1111_0000),
    ];

    let num_blocks = blocks.len();

    for i in 0..num_blocks {
        let current = &blocks[i];

        // Equivalent of `char temp_name[32]; strcpy(temp_name, current->name);`
        let mut temp_name: [c_char; 32] = [0; 32];
        unsafe {
            ptr::copy_nonoverlapping(current.name.as_ptr(), temp_name.as_mut_ptr(), 32);
        }
        let _ = temp_name;

        let mut flag_contribution: i32 = 0;
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

    let block_size = ((param1.rem_euclid(10)) + 5) as usize;
    let mem1 = allocate_block(block_size, param1);
    let mem2 = allocate_block(block_size, param2);

    if mem1.is_null() || mem2.is_null() {
        free_block(mem1);
        free_block(mem2);
        return -1;
    }

    let hash = compute_hash(mem1 as *const MemoryBlock, mem2 as *const MemoryBlock);
    result = result.wrapping_add(hash);

    let mut sum1: i32 = 0;
    let mut sum2: i32 = 0;
    unsafe {
        let size1 = (*mem1).size;
        for i in 0..size1 {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(i));
        }
        let size2 = (*mem2).size;
        for i in 0..size2 {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(i));
        }
    }

    result = result.wrapping_add((sum1.wrapping_sub(sum2)) / 10);

    let mut special = create_block(99, "Special", 0b1111_1111);
    unsafe {
        copy_cstr(&mut special.name, "Modified");
    }

    unsafe {
        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        // In C: `mem1->data > NULL && mem2->data > NULL` — true when both pointers
        // are non-null (pointer comparison treats NULL as the lowest address).
        if !(*mem1).data.is_null() && !(*mem2).data.is_null() {
            result = result.wrapping_add(special.flags as i32);
        }
    }

    free_block(mem1);
    free_block(mem2);

    result
}

/// C-ABI wrapper that mirrors the original `int betagamma(int, int, int, int)`.
#[no_mangle]
pub extern "C" fn betagamma_c(a: i32, b: i32, c: i32, d: i32) -> i32 {
    betagamma(a, b, c, d)
}
