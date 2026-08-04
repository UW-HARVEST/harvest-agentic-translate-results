// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from c_src/src/lib.c

use std::ffi::{c_char, c_int};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    id: c_int,
    name: [u8; 32],
    flags: u8,
}

#[repr(C)]
pub struct MemoryBlock {
    data: *mut c_int,
    size: usize,
}

/// Build a DataBlock with name copied via C-string semantics (strcpy).
fn make_block(id: c_int, name: &[u8], flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id,
        name: [0u8; 32],
        flags,
    };
    // strcpy: copy bytes up to and including the null terminator.
    // The name slices passed in here include the null terminator.
    let n = name.len().min(32);
    block.name[..n].copy_from_slice(&name[..n]);
    block
}

/// C ABI export: DataBlock create_block(int id, const char *name, uint8_t flags)
/// Mimics strcpy: copies bytes from `name` up to and including the null terminator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id,
        name: [0u8; 32],
        flags,
    };
    if !name.is_null() {
        // strcpy copies bytes including the null terminator
        let mut i = 0usize;
        loop {
            let b = unsafe { *name.add(i) } as u8;
            // Note: strcpy in C does not bound check; we mirror its behavior up to 32
            // (writing past 32 would be UB in C anyway).
            if i < 32 {
                block.name[i] = b;
            }
            if b == 0 {
                break;
            }
            i += 1;
        }
    }
    block
}

/// Allocate a MemoryBlock of `count` ints, each initialized to
/// `init_value + i` (with C-style integer wrap semantics).
/// Returns a raw pointer (or null on allocation failure / mimic C behavior).
/// Uses libc malloc/calloc to match C allocator behavior precisely.
#[unsafe(no_mangle)]
pub extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    unsafe {
        let mb_ptr = libc::malloc(std::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
        if mb_ptr.is_null() {
            return std::ptr::null_mut();
        }

        let data_ptr = libc::calloc(count, std::mem::size_of::<c_int>()) as *mut c_int;
        if data_ptr.is_null() && count != 0 {
            libc::free(mb_ptr as *mut libc::c_void);
            return std::ptr::null_mut();
        }

        (*mb_ptr).data = data_ptr;
        (*mb_ptr).size = count;

        // Initialize with init_value + i. In C: mb->data[i] = init_value + i;
        // i is size_t, init_value is int -> int promoted to size_t (unsigned),
        // wraps, then narrows to int when assigned. On 64-bit: do as i64 then truncate.
        for i in 0..count {
            let val = (init_value as i64).wrapping_add(i as i64) as c_int;
            *data_ptr.add(i) = val;
        }

        mb_ptr
    }
}

/// Free a MemoryBlock allocated by `allocate_block`.
#[unsafe(no_mangle)]
pub extern "C" fn free_block(mb: *mut MemoryBlock) {
    if mb.is_null() {
        return;
    }
    unsafe {
        let data_ptr = (*mb).data;
        if !data_ptr.is_null() {
            libc::free(data_ptr as *mut libc::c_void);
        }
        libc::free(mb as *mut libc::c_void);
    }
}

/// Compute a hash based on pointer comparisons (mimics C pointer-compare semantics).
#[unsafe(no_mangle)]
pub extern "C" fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

    unsafe {
        let d1 = (*mb1).data as usize;
        let d2 = (*mb2).data as usize;
        if d1 < d2 {
            hash += 100;
        } else if d1 > d2 {
            hash += 200;
        }
    }

    let p1 = mb1 as usize;
    let p2 = mb2 as usize;
    if p1 < p2 {
        hash += 10;
    } else if p1 > p2 {
        hash += 20;
    }

    hash
}

#[unsafe(no_mangle)]
pub extern "C" fn betagamma(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let blocks: [DataBlock; 3] = [
        make_block(1, b"Block_Alpha\0", 0b10101010),
        make_block(2, b"Block_Beta\0", 0b11001100),
        make_block(3, b"Block_Gamma\0", 0b11110000),
    ];

    let num_blocks: c_int = blocks.len() as c_int;

    for i in 0..num_blocks {
        let current = &blocks[i as usize];

        // temp_name: the strcpy in C copies but is unused. No observable effect.
        let mut _temp_name = [0u8; 32];
        // Mimic strcpy by copying up to and including null terminator.
        for (j, &b) in current.name.iter().enumerate() {
            if j >= 32 {
                break;
            }
            _temp_name[j] = b;
            if b == 0 {
                break;
            }
        }

        let mut flag_contribution: c_int = 0;
        if (current.flags & 0b00001111) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param1);
        }
        if (current.flags & 0b11110000) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param2);
        }
        if (current.flags & 0b10101010) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param3);
        }
        if (current.flags & 0b01010101) != 0 {
            flag_contribution = flag_contribution.wrapping_add(param4);
        }

        result = result.wrapping_add(flag_contribution.wrapping_mul(current.id));
    }

    // size_t block_size = (param1 % 10) + 5;
    // Compute as int first (C semantics), then convert to size_t (which may wrap).
    let block_size_int: c_int = (param1 % 10).wrapping_add(5);
    // Conversion to size_t: int -> size_t. For negative values this becomes huge.
    let block_size: usize = block_size_int as usize;

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
    unsafe {
        let m1_size = (*mem1).size;
        let m1_data = (*mem1).data;
        for i in 0..m1_size {
            sum1 = sum1.wrapping_add(*m1_data.add(i));
        }
        let m2_size = (*mem2).size;
        let m2_data = (*mem2).data;
        for i in 0..m2_size {
            sum2 = sum2.wrapping_add(*m2_data.add(i));
        }
    }

    // C: result += (sum1 - sum2) / 10;
    result = result.wrapping_add(sum1.wrapping_sub(sum2) / 10);

    let mut special = DataBlock {
        id: 99,
        name: [0u8; 32],
        flags: 0b11111111,
    };
    // {99, "Special", 0b11111111} initialization - then strcpy(special.name, "Modified")
    // The final name is "Modified".
    let modified = b"Modified\0";
    for (j, &b) in modified.iter().enumerate() {
        if j >= 32 {
            break;
        }
        special.name[j] = b;
        if b == 0 {
            break;
        }
    }

    unsafe {
        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        // C: if (mem1->data > NULL && mem2->data > NULL) — comparison of pointer to NULL.
        // In C, comparing a pointer with > NULL is technically undefined behavior, but in
        // practice it tests whether the pointer's address (as unsigned) is greater than 0,
        // i.e., the pointer is non-null.
        let m1_nonzero = !(*mem1).data.is_null();
        let m2_nonzero = !(*mem2).data.is_null();
        if m1_nonzero && m2_nonzero {
            // special.flags is uint8_t (0xFF = 255). Promotes to int as 255 (zero-extended).
            result = result.wrapping_add(special.flags as c_int);
        }
    }

    free_block(mem1);
    free_block(mem2);

    result
}
