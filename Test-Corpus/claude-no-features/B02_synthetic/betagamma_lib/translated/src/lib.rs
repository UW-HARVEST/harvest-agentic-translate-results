use libc::{c_void, calloc, free, malloc, size_t};
use std::ffi::c_int;
use std::ptr;

#[repr(C)]
struct DataBlock {
    id: c_int,
    name: [u8; 32],
    flags: u8,
}

#[repr(C)]
struct MemoryBlock {
    data: *mut c_int,
    size: size_t,
}

const fn make_name(s: &[u8]) -> [u8; 32] {
    let mut arr = [0u8; 32];
    let mut i = 0;
    while i < s.len() && i < 32 {
        arr[i] = s[i];
        i += 1;
    }
    arr
}

#[allow(dead_code)]
unsafe fn create_block(id: c_int, name: *const u8, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id,
        name: [0u8; 32],
        flags,
    };
    // strcpy: copy until null terminator (inclusive)
    let mut i = 0;
    loop {
        let b = unsafe { *name.add(i) };
        block.name[i] = b;
        if b == 0 {
            break;
        }
        i += 1;
    }
    block
}

unsafe fn allocate_block(count: size_t, init_value: c_int) -> *mut MemoryBlock {
    let mb = unsafe { malloc(std::mem::size_of::<MemoryBlock>()) } as *mut MemoryBlock;
    if mb.is_null() {
        return ptr::null_mut();
    }

    let data = unsafe { calloc(count, std::mem::size_of::<c_int>()) } as *mut c_int;
    if data.is_null() {
        unsafe { free(mb as *mut c_void) };
        return ptr::null_mut();
    }
    unsafe {
        (*mb).data = data;
        (*mb).size = count;
    }

    let mut i: size_t = 0;
    while i < count {
        unsafe {
            *(*mb).data.add(i) = init_value + i as c_int;
        }
        i += 1;
    }

    mb
}

unsafe fn free_block(mb: *mut MemoryBlock) {
    if !mb.is_null() {
        unsafe {
            if !(*mb).data.is_null() {
                free((*mb).data as *mut c_void);
            }
            free(mb as *mut c_void);
        }
    }
}

unsafe fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

    let d1 = unsafe { (*mb1).data };
    let d2 = unsafe { (*mb2).data };
    if (d1 as usize) < (d2 as usize) {
        hash += 100;
    } else if (d1 as usize) > (d2 as usize) {
        hash += 200;
    }

    if (mb1 as usize) < (mb2 as usize) {
        hash += 10;
    } else if (mb1 as usize) > (mb2 as usize) {
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
    let mut result: c_int = 0;

    let blocks: [DataBlock; 3] = [
        DataBlock {
            id: 1,
            name: make_name(b"Block_Alpha"),
            flags: 0b10101010,
        },
        DataBlock {
            id: 2,
            name: make_name(b"Block_Beta"),
            flags: 0b11001100,
        },
        DataBlock {
            id: 3,
            name: make_name(b"Block_Gamma"),
            flags: 0b11110000,
        },
    ];

    let num_blocks = blocks.len() as c_int;

    let mut i: c_int = 0;
    while i < num_blocks {
        let current = &blocks[i as usize];

        // strcpy(temp_name, current->name) — no observable effect; preserved
        let mut temp_name = [0u8; 32];
        let mut j = 0usize;
        loop {
            let b = current.name[j];
            temp_name[j] = b;
            if b == 0 {
                break;
            }
            j += 1;
            if j >= 32 {
                break;
            }
        }
        let _ = temp_name;

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

        result = result.wrapping_add(flag_contribution.wrapping_mul(current.id));

        i += 1;
    }

    // (param1 % 10) + 5 computed as int, then converted to size_t (preserve C behavior).
    let block_size_int: c_int = (param1 % 10) + 5;
    let block_size: size_t = block_size_int as size_t;

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
    let size1 = unsafe { (*mem1).size };
    let size2 = unsafe { (*mem2).size };
    let mut k: size_t = 0;
    while k < size1 {
        unsafe {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(k));
        }
        k += 1;
    }
    let mut k: size_t = 0;
    while k < size2 {
        unsafe {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(k));
        }
        k += 1;
    }

    result = result.wrapping_add((sum1.wrapping_sub(sum2)) / 10);

    let mut special = DataBlock {
        id: 99,
        name: make_name(b"Special"),
        flags: 0b11111111,
    };
    // strcpy(special.name, "Modified")
    let modified = b"Modified";
    let mut idx = 0usize;
    while idx < modified.len() {
        special.name[idx] = modified[idx];
        idx += 1;
    }
    special.name[idx] = 0;

    let d1 = unsafe { (*mem1).data };
    let d2 = unsafe { (*mem2).data };
    if d1 != d2 {
        result = result.wrapping_add(special.id);
    }

    // mem1->data > NULL && mem2->data > NULL
    if (d1 as usize) > 0 && (d2 as usize) > 0 {
        result = result.wrapping_add(special.flags as c_int);
    }

    unsafe {
        free_block(mem1);
        free_block(mem2);
    }

    result
}
