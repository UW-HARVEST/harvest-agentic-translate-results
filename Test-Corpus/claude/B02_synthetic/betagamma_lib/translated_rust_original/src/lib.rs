// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from c_src/src/lib.c

use std::ffi::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct DataBlock {
    id: c_int,
    name: [u8; 32],
    flags: u8,
}

#[repr(C)]
struct MemoryBlock {
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

/// Allocate a MemoryBlock of `count` ints, each initialized to
/// `init_value + i` (with C-style integer wrap semantics).
/// Returns a raw pointer (or null on allocation failure / mimic C behavior).
fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    // Allocate the MemoryBlock struct on the heap.
    let mb_box = Box::new(MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    });
    let mb_ptr: *mut MemoryBlock = Box::into_raw(mb_box);

    // calloc(count, sizeof(int))
    // If count is 0, calloc may return null or a valid pointer; we return a valid (empty) pointer.
    let mut data_vec: Vec<c_int> = vec![0; count];

    // Initialize with init_value + i (C semantics: i promotes to size_t, addition wraps,
    // then narrowing conversion to int).
    for i in 0..count {
        // Faithful C reproduction: (size_t)init_value + i, then assigned to int.
        // On 64-bit systems this is equivalent to wrapping_add as i64 then truncate.
        let val = (init_value as i64).wrapping_add(i as i64) as c_int;
        data_vec[i] = val;
    }

    let data_ptr = data_vec.as_mut_ptr();
    // Leak the vector so the pointer remains valid; we will reconstruct
    // it later when freeing.
    let len = data_vec.len();
    std::mem::forget(data_vec);

    unsafe {
        (*mb_ptr).data = data_ptr;
        (*mb_ptr).size = len;
    }

    mb_ptr
}

/// Free a MemoryBlock allocated by `allocate_block`.
fn free_block(mb: *mut MemoryBlock) {
    if mb.is_null() {
        return;
    }
    unsafe {
        let data_ptr = (*mb).data;
        let size = (*mb).size;
        if !data_ptr.is_null() {
            // Reconstruct the Vec to free it.
            let _ = Vec::from_raw_parts(data_ptr, size, size);
        }
        // Reconstruct the Box to free the MemoryBlock struct.
        let _ = Box::from_raw(mb);
    }
}

/// Compute a hash based on pointer comparisons (mimics C pointer-compare semantics).
fn compute_hash(mb1: *const MemoryBlock, mb2: *const MemoryBlock) -> c_int {
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
