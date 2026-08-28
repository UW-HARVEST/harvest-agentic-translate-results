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

use std::ffi::{c_char, c_int, c_uchar, c_void};

// The C code's observable behaviour depends on the *identity and relative
// ordering* of pointers handed back by the platform allocator (see
// `compute_hash`, and the `mem1->data != mem2->data` / `> NULL` tests in
// `betagamma`). It also depends on `calloc` failing for absurd element counts,
// which is how a negative `param1 % 10` turns into the `-1` error return. Both
// require the real libc allocator rather than Rust's `Global`, so malloc /
// calloc / free are used directly.
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// `typedef struct { int id; char name[32]; uint8_t flags; } DataBlock;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: c_uchar,
}

/// `typedef struct { int *data; size_t size; } MemoryBlock;`
#[repr(C)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: usize,
}

/// `strcpy(dst, src)`: copy bytes up to and including the NUL terminator.
///
/// Reproduces the C exactly, including the fact that the callers do not bound
/// the copy against the 32-byte `name` field.
unsafe fn strcpy(dst: *mut c_char, src: *const c_char) {
    unsafe {
        let mut i = 0usize;
        loop {
            let ch = *src.add(i);
            *dst.add(i) = ch;
            if ch == 0 {
                return;
            }
            i += 1;
        }
    }
}

/// Build a `DataBlock` from an id, a NUL-terminated name and a flag byte.
///
/// Note that `block` is returned with `name` populated by `strcpy`, so any
/// bytes of `name` beyond the copied string keep whatever the (uninitialised)
/// stack slot held, exactly as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_block(id: c_int, name: *const c_char, flags: c_uchar) -> DataBlock {
    unsafe {
        // `DataBlock block;` -- deliberately uninitialised, as in the C.
        let mut block: DataBlock = std::mem::zeroed();
        block.id = id;
        strcpy(block.name.as_mut_ptr(), name);
        block.flags = flags;
        block
    }
}

/// Allocate a `MemoryBlock` holding `count` ints seeded with `init_value + i`.
///
/// Returns NULL if either allocation fails; a huge `count` (produced when
/// `param1 % 10 + 5` is negative and wraps through `size_t`) makes `calloc`
/// fail, which is the source of `betagamma`'s `-1` return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_block(count: usize, init_value: c_int) -> *mut MemoryBlock {
    unsafe {
        let mb = malloc(std::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
        if mb.is_null() {
            return std::ptr::null_mut();
        }

        let data = calloc(count, std::mem::size_of::<c_int>()) as *mut c_int;
        (*mb).data = data;
        if data.is_null() {
            free(mb as *mut c_void);
            return std::ptr::null_mut();
        }

        (*mb).size = count;

        // `mb->data[i] = init_value + i;` -- `init_value` is widened to
        // `size_t`, added, then truncated back to `int`, i.e. wrapping i32
        // arithmetic on the low 32 bits of `i`.
        for i in 0..count {
            *data.add(i) = init_value.wrapping_add(i as c_int);
        }

        mb
    }
}

/// Free a `MemoryBlock` and its payload. NULL-tolerant, as in the C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_block(mb: *mut MemoryBlock) {
    unsafe {
        if !mb.is_null() {
            if !(*mb).data.is_null() {
                free((*mb).data as *mut c_void);
            }
            free(mb as *mut c_void);
        }
    }
}

/// Score two blocks purely by the relative addresses of the payloads and of
/// the block headers themselves.
///
/// The C compares pointers from distinct allocations, which is formally
/// unspecified; the numeric comparison below matches what it does in practice
/// on a flat address space.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    unsafe {
        let mut hash: c_int = 0;

        let d1 = (*mb1).data as usize;
        let d2 = (*mb2).data as usize;
        if d1 < d2 {
            hash += 100;
        } else if d1 > d2 {
            hash += 200;
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
}

/// Byte-for-byte equivalent of the C `betagamma`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn betagamma(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        let mut result: c_int = 0;

        let blocks: [DataBlock; 3] = [
            make_block(1, b"Block_Alpha", 0b10101010),
            make_block(2, b"Block_Beta", 0b11001100),
            make_block(3, b"Block_Gamma", 0b11110000),
        ];

        for current in blocks.iter() {
            // `strcpy(temp_name, current->name);` -- the copy is never read,
            // but it is reproduced so the behaviour on an over-long name
            // matches.
            let mut temp_name: [c_char; 32] = [0; 32];
            strcpy(temp_name.as_mut_ptr(), current.name.as_ptr());
            let _ = &temp_name;

            let mut flag_contribution: c_int = 0;
            if current.flags & 0b00001111 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param1);
            }
            if current.flags & 0b11110000 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param2);
            }
            if current.flags & 0b10101010 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param3);
            }
            if current.flags & 0b01010101 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param4);
            }

            result = result.wrapping_add(flag_contribution.wrapping_mul(current.id));
        }

        // `size_t block_size = (param1 % 10) + 5;` -- computed as `int`, so a
        // negative value sign-extends into an enormous `size_t`.
        let block_size = ((param1 % 10) + 5) as isize as usize;
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
        for i in 0..(*mem1).size {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(i));
        }
        for i in 0..(*mem2).size {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(i));
        }

        // C integer division truncates toward zero, as does Rust's `/`.
        result = result.wrapping_add(sum1.wrapping_sub(sum2) / 10);

        let mut special = make_block(99, b"Special", 0b11111111);
        strcpy(special.name.as_mut_ptr(), c"Modified".as_ptr());

        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        // `mem1->data > NULL` -- true for any non-null pointer.
        if (*mem1).data as usize > 0 && (*mem2).data as usize > 0 {
            result = result.wrapping_add(special.flags as c_int);
        }

        free_block(mem1);
        free_block(mem2);

        result
    }
}

/// Build a `DataBlock` from a static initialiser, mirroring C's
/// `{id, "name", flags}` aggregate initialisation (which NUL-pads the rest of
/// the `name` array).
fn make_block(id: c_int, name: &[u8], flags: c_uchar) -> DataBlock {
    let mut b = DataBlock {
        id,
        name: [0; 32],
        flags,
    };
    for (dst, &src) in b.name.iter_mut().zip(name.iter()) {
        *dst = src as c_char;
    }
    b
}
