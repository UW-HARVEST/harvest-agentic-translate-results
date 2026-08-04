#![no_std]

use core::ffi::{c_char, c_int, c_void};
use core::mem;
use core::panic::PanicInfo;
use core::ptr;

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
    fn abort() -> !;
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    unsafe { abort() }
}

const BLOCK_ALPHA: [c_char; 32] = c_name_32(b"Block_Alpha\0");
const BLOCK_BETA: [c_char; 32] = c_name_32(b"Block_Beta\0");
const BLOCK_GAMMA: [c_char; 32] = c_name_32(b"Block_Gamma\0");
const SPECIAL: [c_char; 32] = c_name_32(b"Special\0");
const MODIFIED: &[u8] = b"Modified\0";

const fn c_name_32(bytes: &[u8]) -> [c_char; 32] {
    let mut out = [0 as c_char; 32];
    let mut i = 0;
    while i < bytes.len() {
        out[i] = bytes[i] as c_char;
        i += 1;
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id,
        name: [0; 32],
        flags,
    };

    unsafe {
        strcpy(block.name.as_mut_ptr(), name);
    }

    block
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    let mb = unsafe { malloc(mem::size_of::<MemoryBlock>()) as *mut MemoryBlock };
    if mb.is_null() {
        return ptr::null_mut();
    }

    let data = unsafe { calloc(count, mem::size_of::<c_int>()) as *mut c_int };
    if data.is_null() {
        unsafe {
            free(mb as *mut c_void);
        }
        return ptr::null_mut();
    }

    unsafe {
        (*mb).data = data;
        (*mb).size = count;
    }

    for i in 0..count {
        let value = (init_value as usize).wrapping_add(i) as c_int;
        unsafe {
            *data.add(i) = value;
        }
    }

    mb
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_block(mb: *mut MemoryBlock) {
    if !mb.is_null() {
        unsafe {
            if !(*mb).data.is_null() {
                free((*mb).data as *mut c_void);
            }
            free(mb as *mut c_void);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

    unsafe {
        if ((*mb1).data as usize) < ((*mb2).data as usize) {
            hash = hash.wrapping_add(100);
        } else if ((*mb1).data as usize) > ((*mb2).data as usize) {
            hash = hash.wrapping_add(200);
        }

        if (mb1 as usize) < (mb2 as usize) {
            hash = hash.wrapping_add(10);
        } else if (mb1 as usize) > (mb2 as usize) {
            hash = hash.wrapping_add(20);
        }
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
    let mut result: c_int = 0;

    let mut blocks = [
        DataBlock {
            id: 1,
            name: BLOCK_ALPHA,
            flags: 0b1010_1010,
        },
        DataBlock {
            id: 2,
            name: BLOCK_BETA,
            flags: 0b1100_1100,
        },
        DataBlock {
            id: 3,
            name: BLOCK_GAMMA,
            flags: 0b1111_0000,
        },
    ];

    let num_blocks = blocks.len() as c_int;

    for i in 0..num_blocks {
        let current = &mut blocks[i as usize] as *mut DataBlock;

        let mut temp_name = [0 as c_char; 32];
        unsafe {
            strcpy(temp_name.as_mut_ptr(), (*current).name.as_ptr());
        }

        let mut flag_contribution: c_int = 0;
        unsafe {
            if (*current).flags & 0b0000_1111 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param1);
            }
            if (*current).flags & 0b1111_0000 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param2);
            }
            if (*current).flags & 0b1010_1010 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param3);
            }
            if (*current).flags & 0b0101_0101 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param4);
            }

            result = result.wrapping_add(flag_contribution.wrapping_mul((*current).id));
        }
    }

    let block_size = param1.wrapping_rem(10).wrapping_add(5) as usize;
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
        for i in 0..(*mem1).size {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(i));
        }
        for i in 0..(*mem2).size {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(i));
        }
    }

    result = result.wrapping_add(sum1.wrapping_sub(sum2) / 10);

    let mut special = DataBlock {
        id: 99,
        name: SPECIAL,
        flags: 0b1111_1111,
    };
    unsafe {
        strcpy(special.name.as_mut_ptr(), MODIFIED.as_ptr() as *const c_char);
    }

    unsafe {
        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        if !(*mem1).data.is_null() && !(*mem2).data.is_null() {
            result = result.wrapping_add(special.flags as c_int);
        }

        free_block(mem1);
        free_block(mem2);
    }

    result
}
