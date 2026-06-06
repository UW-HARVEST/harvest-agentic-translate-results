// Translation of c_src/src/lib.c to Rust.
// Preserves the original (sometimes buggy) behavior, including pointer
// comparisons and integer narrowing semantics, by using the C library's
// malloc/calloc/free for the allocations involved in pointer comparisons.

use std::ffi::c_int;
use std::os::raw::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

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

/// Equivalent of C's strcpy: copies bytes from `src` (a NUL-terminated byte
/// slice) into `dst`, including the terminating NUL byte. Caller must ensure
/// `dst` is large enough.
fn rust_strcpy(dst: &mut [u8; 32], src: &[u8]) {
    // Find NUL terminator length in `src` (at most dst.len()-1 will fit safely)
    let mut i = 0;
    while i < src.len() {
        let b = src[i];
        dst[i] = b;
        if b == 0 {
            return;
        }
        i += 1;
    }
    // If we ran off the end of `src` without finding NUL, do nothing further;
    // matches the C convention that the caller guarantees a NUL.
}

#[allow(dead_code)]
fn create_block(id: c_int, name: &[u8], flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id: 0,
        name: [0u8; 32],
        flags: 0,
    };
    block.id = id;
    rust_strcpy(&mut block.name, name);
    block.flags = flags;
    block
}

fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    unsafe {
        let mb = malloc(std::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
        if mb.is_null() {
            return std::ptr::null_mut();
        }

        let data = calloc(count, std::mem::size_of::<c_int>()) as *mut c_int;
        if data.is_null() {
            free(mb as *mut c_void);
            return std::ptr::null_mut();
        }

        (*mb).data = data;
        (*mb).size = count;

        // Reproduce C semantics: `mb->data[i] = init_value + i;` where i is
        // size_t. In C, init_value (int) is converted to size_t for the add,
        // then truncated back to int on assignment. With two's-complement
        // wrap-around this equals init_value.wrapping_add(i as i32) for any
        // i within i32 range.
        for i in 0..count {
            let v = (init_value as usize).wrapping_add(i) as c_int;
            *data.add(i) = v;
        }

        mb
    }
}

fn free_block(mb: *mut MemoryBlock) {
    unsafe {
        if !mb.is_null() {
            if !(*mb).data.is_null() {
                free((*mb).data as *mut c_void);
            }
            free(mb as *mut c_void);
        }
    }
}

fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;
    unsafe {
        let d1 = (*mb1).data;
        let d2 = (*mb2).data;
        if d1 < d2 {
            hash += 100;
        } else if d1 > d2 {
            hash += 200;
        }
    }
    if (mb1 as *const MemoryBlock) < (mb2 as *const MemoryBlock) {
        hash += 10;
    } else if (mb1 as *const MemoryBlock) > (mb2 as *const MemoryBlock) {
        hash += 20;
    }
    hash
}

#[unsafe(no_mangle)]
pub extern "C" fn betagamma(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut blocks: [DataBlock; 3] = [
        DataBlock {
            id: 1,
            name: [0u8; 32],
            flags: 0b1010_1010,
        },
        DataBlock {
            id: 2,
            name: [0u8; 32],
            flags: 0b1100_1100,
        },
        DataBlock {
            id: 3,
            name: [0u8; 32],
            flags: 0b1111_0000,
        },
    ];
    // Initialize names to match C string-literal initializers
    {
        let names: [&[u8]; 3] = [b"Block_Alpha\0", b"Block_Beta\0", b"Block_Gamma\0"];
        for (b, n) in blocks.iter_mut().zip(names.iter()) {
            rust_strcpy(&mut b.name, n);
        }
    }

    let num_blocks = blocks.len();

    for i in 0..num_blocks {
        let current = &blocks[i];

        // The C code has `char temp_name[32]; strcpy(temp_name, current->name);`
        // which has no observable side effect. We mirror that with a no-op
        // local copy so the loop structure is preserved.
        let mut temp_name = [0u8; 32];
        rust_strcpy(&mut temp_name, &current.name);
        let _ = temp_name;

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
    }

    // `block_size = (param1 % 10) + 5;` — int arithmetic, then implicit
    // conversion to size_t when used as a count. Preserve sign-extending
    // (i.e., reinterpret) cast to mirror C behavior on 64-bit platforms.
    let block_size_i32: c_int = (param1 % 10).wrapping_add(5);
    let block_size: usize = block_size_i32 as usize;

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
        for i in 0..(*mem1).size {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(i));
        }
        for i in 0..(*mem2).size {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(i));
        }
    }

    // C: `result += (sum1 - sum2) / 10;`  signed division truncates toward 0
    let diff = sum1.wrapping_sub(sum2);
    result = result.wrapping_add(diff / 10);

    let mut special = DataBlock {
        id: 99,
        name: [0u8; 32],
        flags: 0b1111_1111,
    };
    rust_strcpy(&mut special.name, b"Special\0");
    rust_strcpy(&mut special.name, b"Modified\0");

    unsafe {
        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        // C: `if (mem1->data > NULL && mem2->data > NULL)` — pointer > NULL
        // is true for any non-null pointer.
        if !(*mem1).data.is_null() && !(*mem2).data.is_null() {
            result = result.wrapping_add(special.flags as c_int);
        }
    }

    free_block(mem1);
    free_block(mem2);

    result
}
