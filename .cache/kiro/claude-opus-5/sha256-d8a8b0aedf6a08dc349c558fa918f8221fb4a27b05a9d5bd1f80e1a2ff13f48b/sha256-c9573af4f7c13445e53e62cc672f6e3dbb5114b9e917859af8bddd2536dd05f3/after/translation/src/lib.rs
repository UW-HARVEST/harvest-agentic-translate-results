// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/lib.c`.
//!
//! Fidelity notes:
//!
//! * `compute_hash` (and therefore `betagamma`) observes raw heap addresses.
//!   To reproduce the C library's results bit-for-bit we deliberately route all
//!   allocation through the platform C allocator (`malloc`/`calloc`/`free`)
//!   using exactly the same request sizes and ordering as the C code, and we
//!   perform no Rust heap allocation anywhere in this crate. That keeps the
//!   process heap evolution identical to the C build.
//! * Signed arithmetic in the C code can overflow (formally UB). The reference
//!   build wraps, so every arithmetic operation here uses explicit wrapping
//!   semantics.
//! * `strcpy` is used verbatim (including the unbounded copies the C code
//!   performs) rather than being "fixed" into a bounded copy.
//! * `(param1 % 10) + 5` is computed in `int` and then converted to `size_t`,
//!   so negative results become huge unsigned counts whose `calloc` fails.
//!   That path returns `-1`, exactly as in C.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::MaybeUninit;
use core::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
}

/// ```c
/// typedef struct {
///     int id;
///     char name[32];
///     uint8_t flags;
/// } DataBlock;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: u8,
}

/// ```c
/// typedef struct {
///     int *data;
///     size_t size;
/// } MemoryBlock;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

/// Build a NUL-padded `char[32]` from a string literal, mirroring C's
/// `char name[32] = "..."` aggregate initialization (remaining bytes zeroed).
const fn name32(src: &[u8]) -> [c_char; 32] {
    let mut out = [0 as c_char; 32];
    let mut i = 0;
    while i < src.len() {
        out[i] = src[i] as c_char;
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// DataBlock create_block(int id, const char *name, uint8_t flags)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    // `DataBlock block;` is uninitialized in C; only the three fields are
    // assigned, so trailing padding keeps whatever was on the stack.
    let mut block: MaybeUninit<DataBlock> = MaybeUninit::uninit();
    let p = block.as_mut_ptr();

    (*p).id = id;
    // Faithful to the C `strcpy(block.name, name)`, overflow behaviour included.
    strcpy(ptr::addr_of_mut!((*p).name) as *mut c_char, name);
    (*p).flags = flags;

    block.assume_init()
}

// ---------------------------------------------------------------------------
// MemoryBlock* allocate_block(size_t count, int init_value)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    let mb = malloc(core::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
    if mb.is_null() {
        return ptr::null_mut();
    }

    (*mb).data = calloc(count, core::mem::size_of::<c_int>()) as *mut c_int;
    if (*mb).data.is_null() {
        free(mb as *mut c_void);
        return ptr::null_mut();
    }

    (*mb).size = count;

    let data = (*mb).data;
    let mut i: usize = 0;
    while i < count {
        // C: `mb->data[i] = init_value + i;` -- `int` is converted to `size_t`,
        // the addition happens in `size_t`, then the result is truncated to
        // `int` on assignment.
        *data.add(i) = (init_value as isize as usize).wrapping_add(i) as u32 as c_int;
        i += 1;
    }

    mb
}

// ---------------------------------------------------------------------------
// void free_block(MemoryBlock *mb)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_block(mb: *mut MemoryBlock) {
    if !mb.is_null() {
        if !(*mb).data.is_null() {
            free((*mb).data as *mut c_void);
        }
        free(mb as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// int compute_hash(MemoryBlock *mb1, MemoryBlock *mb2)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

    // Raw (unsigned) address comparisons, matching C pointer relationals.
    let d1 = (*mb1).data as usize;
    let d2 = (*mb2).data as usize;
    if d1 < d2 {
        hash = hash.wrapping_add(100);
    } else if d1 > d2 {
        hash = hash.wrapping_add(200);
    }

    let p1 = mb1 as usize;
    let p2 = mb2 as usize;
    if p1 < p2 {
        hash = hash.wrapping_add(10);
    } else if p1 > p2 {
        hash = hash.wrapping_add(20);
    }

    hash
}

// ---------------------------------------------------------------------------
// int betagamma(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn betagamma(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let mut blocks: [DataBlock; 3] = [
        DataBlock {
            id: 1,
            name: name32(b"Block_Alpha"),
            flags: 0b1010_1010,
        },
        DataBlock {
            id: 2,
            name: name32(b"Block_Beta"),
            flags: 0b1100_1100,
        },
        DataBlock {
            id: 3,
            name: name32(b"Block_Gamma"),
            flags: 0b1111_0000,
        },
    ];

    let num_blocks: c_int =
        (core::mem::size_of_val(&blocks) / core::mem::size_of::<DataBlock>()) as c_int;

    let mut i: c_int = 0;
    while i < num_blocks {
        let current: *mut DataBlock = &mut blocks[i as usize];

        let mut temp_name = [0 as c_char; 32];
        strcpy(
            temp_name.as_mut_ptr(),
            ptr::addr_of!((*current).name) as *const c_char,
        );
        let _ = &temp_name; // dead in C as well

        let flags = (*current).flags as c_int;

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

        result = result.wrapping_add(flag_contribution.wrapping_mul((*current).id));

        i += 1;
    }

    // `int` arithmetic, then converted (sign-extended) to `size_t`.
    let block_size: usize = param1.wrapping_rem(10).wrapping_add(5) as isize as usize;
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
    let mut k: usize = 0;
    while k < (*mem1).size {
        sum1 = sum1.wrapping_add(*(*mem1).data.add(k));
        k += 1;
    }
    let mut k: usize = 0;
    while k < (*mem2).size {
        sum2 = sum2.wrapping_add(*(*mem2).data.add(k));
        k += 1;
    }

    result = result.wrapping_add(sum1.wrapping_sub(sum2).wrapping_div(10));

    let mut special = DataBlock {
        id: 99,
        name: name32(b"Special"),
        flags: 0b1111_1111,
    };
    strcpy(
        special.name.as_mut_ptr(),
        b"Modified\0".as_ptr() as *const c_char,
    );

    if (*mem1).data != (*mem2).data {
        result = result.wrapping_add(special.id);
    }

    // C: `mem1->data > NULL && mem2->data > NULL` -- unsigned address > 0.
    if ((*mem1).data as usize) > 0 && ((*mem2).data as usize) > 0 {
        result = result.wrapping_add(special.flags as c_int);
    }

    free_block(mem1);
    free_block(mem2);

    result
}
