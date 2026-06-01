// Translation of c_src/src/lib.c to Rust.
// Preserves the same observable behavior, including reliance on libc
// malloc/calloc/free so pointer comparisons in compute_hash match the C
// implementation.

use std::ffi::c_int;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Copy, Clone)]
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

unsafe fn strcpy_into(dst: *mut c_char, src: *const c_char) {
    // Mirror C's strcpy: copy until NUL (inclusive), no bounds check.
    let mut i = 0isize;
    loop {
        let b = unsafe { *src.offset(i) };
        unsafe {
            *dst.offset(i) = b;
        }
        if b == 0 {
            break;
        }
        i += 1;
    }
}

fn make_name(s: &str) -> [c_char; 32] {
    let mut buf = [0 as c_char; 32];
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        buf[i] = b as c_char;
    }
    buf
}

#[unsafe(no_mangle)]
pub extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id: 0,
        name: [0 as c_char; 32],
        flags: 0,
    };
    block.id = id;
    unsafe {
        strcpy_into(block.name.as_mut_ptr(), name);
    }
    block.flags = flags;
    block
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    unsafe {
        let mb = libc::malloc(std::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
        if mb.is_null() {
            return std::ptr::null_mut();
        }

        let data = libc::calloc(count, std::mem::size_of::<c_int>()) as *mut c_int;
        if data.is_null() {
            libc::free(mb as *mut libc::c_void);
            return std::ptr::null_mut();
        }

        (*mb).data = data;
        (*mb).size = count;

        let mut i: usize = 0;
        while i < count {
            // C: mb->data[i] = init_value + i;
            // (init_value + (int)i) computed in int with wraparound.
            let val = (init_value as i32).wrapping_add(i as i32);
            *data.add(i) = val as c_int;
            i += 1;
        }

        mb
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_block(mb: *mut MemoryBlock) {
    unsafe {
        if !mb.is_null() {
            if !(*mb).data.is_null() {
                libc::free((*mb).data as *mut libc::c_void);
            }
            libc::free(mb as *mut libc::c_void);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

    unsafe {
        let d1 = (*mb1).data;
        let d2 = (*mb2).data;
        if (d1 as usize) < (d2 as usize) {
            hash += 100;
        } else if (d1 as usize) > (d2 as usize) {
            hash += 200;
        }
    }

    if (mb1 as usize) < (mb2 as usize) {
        hash += 10;
    } else if (mb1 as usize) > (mb2 as usize) {
        hash += 20;
    }

    hash
}

#[unsafe(no_mangle)]
pub extern "C" fn betagamma(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let blocks: [DataBlock; 3] = [
        DataBlock {
            id: 1,
            name: make_name("Block_Alpha"),
            flags: 0b1010_1010,
        },
        DataBlock {
            id: 2,
            name: make_name("Block_Beta"),
            flags: 0b1100_1100,
        },
        DataBlock {
            id: 3,
            name: make_name("Block_Gamma"),
            flags: 0b1111_0000,
        },
    ];

    let num_blocks = blocks.len();

    for i in 0..num_blocks {
        let current = &blocks[i];

        // Mirror the (unused) strcpy of name into temp_name.
        let mut temp_name = [0 as c_char; 32];
        unsafe {
            strcpy_into(temp_name.as_mut_ptr(), current.name.as_ptr());
        }
        let _ = temp_name;

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

    // size_t block_size = (param1 % 10) + 5;
    // (param1 % 10) is signed int; +5 done as int; result implicitly converted
    // to size_t. Negative values become huge size_t values, causing
    // allocate_block to return NULL.
    let modv = (param1 as i32) % 10;
    let block_size_signed: i32 = modv.wrapping_add(5);
    let block_size: usize = block_size_signed as isize as usize;

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
        let mut i: usize = 0;
        while i < (*mem1).size {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(i));
            i += 1;
        }
        let mut j: usize = 0;
        while j < (*mem2).size {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(j));
            j += 1;
        }
    }

    result = result.wrapping_add((sum1.wrapping_sub(sum2)) / 10);

    let mut special = DataBlock {
        id: 99,
        name: make_name("Special"),
        flags: 0b1111_1111,
    };
    // strcpy(special.name, "Modified");
    let modified = b"Modified\0";
    for (k, &b) in modified.iter().enumerate() {
        special.name[k] = b as c_char;
    }

    unsafe {
        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        // Original: if (mem1->data > NULL && mem2->data > NULL)
        // i.e., both pointers non-null.
        if !(*mem1).data.is_null() && !(*mem2).data.is_null() {
            result = result.wrapping_add(special.flags as c_int);
        }
    }

    free_block(mem1);
    free_block(mem2);

    result
}
