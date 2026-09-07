// Rust translation of c_src/src/lib.c
//
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
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::MaybeUninit;

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C code relies on the platform allocator both for its observable
// side effects (calloc(0, n) returning a unique non-NULL pointer, calloc
// overflow returning NULL) and, in `compute_hash`, for the *relative
// ordering* of the returned addresses.  Reproducing that behaviour
// byte-for-byte requires using exactly the same allocator as the C build,
// so malloc/calloc/free are used directly rather than Rust's allocator.
// ---------------------------------------------------------------------------
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
}

// typedef struct {
//     int id;
//     char name[32];
//     uint8_t flags;
// } DataBlock;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: u8,
}

// typedef struct {
//     int *data;
//     size_t size;
// } MemoryBlock;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

/// Build a `DataBlock` literal the way the C initialisers do:
/// `{id, "literal", flags}` zero-fills the remainder of `name`.
const fn data_block_lit(id: c_int, name: &[u8], flags: u8) -> DataBlock {
    let mut buf = [0 as c_char; 32];
    let mut i = 0usize;
    while i < name.len() {
        buf[i] = name[i] as c_char;
        i += 1;
    }
    DataBlock {
        id,
        name: buf,
        flags,
    }
}

// ---------------------------------------------------------------------------
// DataBlock create_block(int id, const char *name, uint8_t flags)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    // DataBlock block;  -- deliberately left uninitialised, exactly as in C.
    let mut block: MaybeUninit<DataBlock> = MaybeUninit::uninit();
    let p = block.as_mut_ptr();

    // block.id = id;
    core::ptr::addr_of_mut!((*p).id).write(id);

    // strcpy(block.name, name);
    strcpy(core::ptr::addr_of_mut!((*p).name) as *mut c_char, name);

    // block.flags = flags;
    core::ptr::addr_of_mut!((*p).flags).write(flags);

    // return block;
    block.assume_init()
}

// ---------------------------------------------------------------------------
// MemoryBlock* allocate_block(size_t count, int init_value)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    let mb = malloc(core::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
    if mb.is_null() {
        return core::ptr::null_mut();
    }

    let data = calloc(count, core::mem::size_of::<c_int>()) as *mut c_int;
    core::ptr::addr_of_mut!((*mb).data).write(data);
    if data.is_null() {
        free(mb as *mut c_void);
        return core::ptr::null_mut();
    }

    core::ptr::addr_of_mut!((*mb).size).write(count);

    // for (size_t i = 0; i < count; i++) mb->data[i] = init_value + i;
    //
    // `init_value + i` is computed in size_t (the int operand is converted
    // to the unsigned type) and then truncated on assignment to int.
    let mut i: usize = 0;
    while i < count {
        let v = (init_value as usize).wrapping_add(i);
        data.add(i).write(v as u32 as c_int);
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
        let data = core::ptr::addr_of!((*mb).data).read();
        if !data.is_null() {
            free(data as *mut c_void);
        }
        free(mb as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// int compute_hash(MemoryBlock *mb1, MemoryBlock *mb2)
//
// Note: both pointers are dereferenced without a NULL check, mirroring the C.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    let mut hash: c_int = 0;

    let d1 = core::ptr::addr_of!((*mb1).data).read();
    let d2 = core::ptr::addr_of!((*mb2).data).read();

    if d1 < d2 {
        hash = hash.wrapping_add(100);
    } else if d1 > d2 {
        hash = hash.wrapping_add(200);
    }

    if (mb1 as *const MemoryBlock) < (mb2 as *const MemoryBlock) {
        hash = hash.wrapping_add(10);
    } else if (mb1 as *const MemoryBlock) > (mb2 as *const MemoryBlock) {
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
        data_block_lit(1, b"Block_Alpha", 0b1010_1010),
        data_block_lit(2, b"Block_Beta", 0b1100_1100),
        data_block_lit(3, b"Block_Gamma", 0b1111_0000),
    ];

    let num_blocks: c_int = blocks.len() as c_int;

    let mut i: c_int = 0;
    while i < num_blocks {
        let current: *mut DataBlock = blocks.as_mut_ptr().add(i as usize);

        // char temp_name[32]; strcpy(temp_name, current->name);
        let mut temp_name: MaybeUninit<[c_char; 32]> = MaybeUninit::uninit();
        strcpy(
            temp_name.as_mut_ptr() as *mut c_char,
            core::ptr::addr_of!((*current).name) as *const c_char,
        );
        let _ = core::ptr::read_volatile(temp_name.as_ptr());

        let flags = core::ptr::addr_of!((*current).flags).read();

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

        let id = core::ptr::addr_of!((*current).id).read();
        result = result.wrapping_add(flag_contribution.wrapping_mul(id));

        i += 1;
    }

    // size_t block_size = (param1 % 10) + 5;
    // The int expression is sign-extended when converted to size_t, so a
    // negative value becomes an enormous count (and calloc then fails).
    let block_size: usize = (param1 % 10).wrapping_add(5) as isize as usize;

    let mem1 = allocate_block(block_size, param1);
    let mem2 = allocate_block(block_size, param2);

    if mem1.is_null() || mem2.is_null() {
        free_block(mem1);
        free_block(mem2);
        return -1;
    }

    let hash = compute_hash(mem1, mem2);
    result = result.wrapping_add(hash);

    let (mut sum1, mut sum2): (c_int, c_int) = (0, 0);

    let m1_size = core::ptr::addr_of!((*mem1).size).read();
    let m1_data = core::ptr::addr_of!((*mem1).data).read();
    let mut k: usize = 0;
    while k < m1_size {
        sum1 = sum1.wrapping_add(m1_data.add(k).read());
        k += 1;
    }

    let m2_size = core::ptr::addr_of!((*mem2).size).read();
    let m2_data = core::ptr::addr_of!((*mem2).data).read();
    let mut k: usize = 0;
    while k < m2_size {
        sum2 = sum2.wrapping_add(m2_data.add(k).read());
        k += 1;
    }

    // result += (sum1 - sum2) / 10;  (C division truncates toward zero)
    result = result.wrapping_add(sum1.wrapping_sub(sum2).wrapping_div(10));

    // DataBlock special = {99, "Special", 0b11111111};
    let mut special = data_block_lit(99, b"Special", 0b1111_1111);
    // strcpy(special.name, "Modified");
    strcpy(
        special.name.as_mut_ptr(),
        b"Modified\0".as_ptr() as *const c_char,
    );

    if m1_data != m2_data {
        result = result.wrapping_add(special.id);
    }

    // if (mem1->data > NULL && mem2->data > NULL)
    if !m1_data.is_null() && !m2_data.is_null() {
        result = result.wrapping_add(special.flags as c_int);
    }

    free_block(mem1);
    free_block(mem2);

    result
}
