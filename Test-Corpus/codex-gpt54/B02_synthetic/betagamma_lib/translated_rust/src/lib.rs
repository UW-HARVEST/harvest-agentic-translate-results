#![no_std]

use core::ffi::{c_char, c_int, c_void};
use core::panic::PanicInfo;
use core::ptr;

#[repr(C)]
#[derive(Copy, Clone)]
struct DataBlock {
    id: c_int,
    name: [c_char; 32],
    flags: u8,
}

#[repr(C)]
struct MemoryBlock {
    data: *mut c_int,
    size: usize,
}

unsafe extern "C" {
    fn abort() -> !;
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    unsafe { abort() }
}

unsafe fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    let mb = malloc(core::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
    if mb.is_null() {
        return ptr::null_mut();
    }

    (*mb).data = calloc(count, core::mem::size_of::<c_int>()) as *mut c_int;
    if (*mb).data.is_null() {
        free(mb.cast());
        return ptr::null_mut();
    }

    (*mb).size = count;

    let mut i = 0usize;
    while i < count {
        *(*mb).data.add(i) = init_value.wrapping_add(i as c_int);
        i += 1;
    }

    mb
}

unsafe fn free_block(mb: *mut MemoryBlock) {
    if !mb.is_null() {
        if !(*mb).data.is_null() {
            free((*mb).data.cast());
        }
        free(mb.cast());
    }
}

unsafe fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

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
            name: [0; 32],
            flags: 0b1010_1010,
        },
        DataBlock {
            id: 2,
            name: [0; 32],
            flags: 0b1100_1100,
        },
        DataBlock {
            id: 3,
            name: [0; 32],
            flags: 0b1111_0000,
        },
    ];

    strcpy(
        blocks[0].name.as_mut_ptr(),
        b"Block_Alpha\0".as_ptr().cast::<c_char>(),
    );
    strcpy(
        blocks[1].name.as_mut_ptr(),
        b"Block_Beta\0".as_ptr().cast::<c_char>(),
    );
    strcpy(
        blocks[2].name.as_mut_ptr(),
        b"Block_Gamma\0".as_ptr().cast::<c_char>(),
    );

    let num_blocks = (core::mem::size_of_val(&blocks) / core::mem::size_of::<DataBlock>()) as c_int;

    let mut i: c_int = 0;
    while i < num_blocks {
        let current = &*blocks.as_ptr().add(i as usize);

        let mut temp_name = [0 as c_char; 32];
        strcpy(temp_name.as_mut_ptr(), current.name.as_ptr());

        let mut flag_contribution: c_int = 0;
        if (current.flags & 0b0000_1111) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param1);
        }
        if (current.flags & 0b1111_0000) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param2);
        }
        if (current.flags & 0b1010_1010) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param3);
        }
        if (current.flags & 0b0101_0101) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param4);
        }

        result = result.wrapping_add(flag_contribution.wrapping_mul(current.id));
        i += 1;
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
    result = result.wrapping_add(hash);

    let mut sum1: c_int = 0;
    let mut sum2: c_int = 0;
    let mut idx = 0usize;
    while idx < (*mem1).size {
        sum1 = sum1.wrapping_add(*(*mem1).data.add(idx));
        idx += 1;
    }

    idx = 0;
    while idx < (*mem2).size {
        sum2 = sum2.wrapping_add(*(*mem2).data.add(idx));
        idx += 1;
    }

    result = result.wrapping_add((sum1.wrapping_sub(sum2)) / 10);

    let mut special = DataBlock {
        id: 99,
        name: [0; 32],
        flags: 0b1111_1111,
    };
    strcpy(
        special.name.as_mut_ptr(),
        b"Special\0".as_ptr().cast::<c_char>(),
    );
    strcpy(
        special.name.as_mut_ptr(),
        b"Modified\0".as_ptr().cast::<c_char>(),
    );

    if (*mem1).data != (*mem2).data {
        result = result.wrapping_add(special.id);
    }

    if ((*mem1).data as usize) > 0 && ((*mem2).data as usize) > 0 {
        result = result.wrapping_add(special.flags as c_int);
    }

    free_block(mem1);
    free_block(mem2);

    result
}
