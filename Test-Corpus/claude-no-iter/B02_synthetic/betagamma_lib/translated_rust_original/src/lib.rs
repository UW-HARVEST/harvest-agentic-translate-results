// Translation of c_src/src/lib.c to Rust.
//
// Goal: produce byte-identical output for the same inputs.
//
// Notes on faithful preservation of the original C behavior:
//   * The C implementation uses malloc/calloc/free from libc and performs
//     pointer comparisons between the resulting allocations. To mirror that
//     allocator behavior (and the resulting pointer ordering observed in
//     `compute_hash` and elsewhere) we also use libc's malloc/calloc/free.
//   * Signed integer arithmetic is performed using `wrapping_*` helpers so
//     that overflow does not panic (matching C's two's-complement wrap on
//     virtually all targets).
//   * The size_t conversion of `(param1 % 10) + 5` is preserved, including
//     its bug-prone behavior when param1 is negative (e.g. param1 = -7 makes
//     the size_t value SIZE_MAX - 1, which causes calloc to fail and the
//     function to return -1 — same as the original C).

use std::ffi::c_int;
use std::ptr;

use libc::{c_void, calloc, free, malloc, size_t};

#[repr(C)]
struct MemoryBlock {
    data: *mut c_int,
    size: size_t,
}

/// Mirrors the C `allocate_block` helper.
unsafe fn allocate_block(count: size_t, init_value: c_int) -> *mut MemoryBlock {
    let mb = unsafe { malloc(std::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock };
    if mb.is_null() {
        return ptr::null_mut();
    }

    let data = unsafe { calloc(count, std::mem::size_of::<c_int>()) as *mut c_int };
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
        // C does:  mb->data[i] = init_value + i;
        // where init_value is `int` and i is `size_t`. In C the addition is
        // performed in size_t (the larger type) and the result is truncated
        // back to int on assignment. Reduced modulo 2^32 this is identical
        // to performing the addition with wrapping i32 arithmetic after
        // truncating `i` to i32 first.
        unsafe {
            *(*mb).data.add(i) = init_value.wrapping_add(i as c_int);
        }
        i += 1;
    }

    mb
}

/// Mirrors the C `free_block` helper.
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

/// Mirrors the C `compute_hash` helper. Pointer order is compared by address.
unsafe fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

    let (d1, d2) = unsafe { ((*mb1).data, (*mb2).data) };
    if d1 < d2 {
        hash = hash.wrapping_add(100);
    } else if d1 > d2 {
        hash = hash.wrapping_add(200);
    }

    if mb1 < mb2 {
        hash = hash.wrapping_add(10);
    } else if mb1 > mb2 {
        hash = hash.wrapping_add(20);
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

    // The DataBlock array in C only contributes (id, flags) to the
    // computation — name/strcpy/etc. have no observable effect on the
    // returned value. So we only model the fields we need.
    let blocks: [(c_int, u8); 3] = [
        (1, 0b1010_1010),
        (2, 0b1100_1100),
        (3, 0b1111_0000),
    ];

    for &(id, flags) in blocks.iter() {
        let mut flag_contribution: c_int = 0;
        if flags & 0b0000_1111 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param1);
        }
        if flags & 0b1111_0000 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param2);
        }
        if flags & 0b1010_1010 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param3);
        }
        if flags & 0b0101_0101 != 0 {
            flag_contribution = flag_contribution.wrapping_add(param4);
        }

        result = result.wrapping_add(flag_contribution.wrapping_mul(id));
    }

    // size_t block_size = (param1 % 10) + 5;
    // In C this is computed as `int` then implicitly converted to size_t.
    // We replicate that exact path: signed remainder, signed +5 with
    // wrapping, then a sign-preserving (mod 2^N) cast to size_t.
    let block_size_signed: c_int = (param1 % 10).wrapping_add(5);
    let block_size: size_t = block_size_signed as size_t;

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
        let size1 = (*mem1).size;
        let mut i: size_t = 0;
        while i < size1 {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(i));
            i += 1;
        }

        let size2 = (*mem2).size;
        let mut j: size_t = 0;
        while j < size2 {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(j));
            j += 1;
        }
    }

    // Signed integer division, truncating toward zero — same as C's `/`.
    result = result.wrapping_add(sum1.wrapping_sub(sum2) / 10);

    // `special` is built locally in C: id=99, flags=0xFF. The strcpy on its
    // name has no observable impact on the returned value, so we just keep
    // the constants.
    let special_id: c_int = 99;
    let special_flags: u8 = 0b1111_1111;

    unsafe {
        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special_id);
        }

        // C: `if (mem1->data > NULL && mem2->data > NULL)` — i.e. both
        // pointers compare strictly greater than the null pointer. For any
        // successfully-allocated pointer this is true, so the contribution
        // is added.
        if !(*mem1).data.is_null() && !(*mem2).data.is_null() {
            result = result.wrapping_add(special_flags as c_int);
        }
    }

    unsafe {
        free_block(mem1);
        free_block(mem2);
    }

    result
}
