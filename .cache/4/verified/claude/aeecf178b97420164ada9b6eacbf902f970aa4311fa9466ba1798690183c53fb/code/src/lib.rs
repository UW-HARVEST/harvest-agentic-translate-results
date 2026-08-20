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

// size_t on all supported targets.
#[allow(non_camel_case_types)]
type size_t = usize;

// The C code uses the platform allocator (malloc/calloc/free) directly.  The
// behaviour of `compute_hash` (and therefore of `betagamma`) depends on the
// *addresses* the allocator hands out, so we must go through the exact same
// libc entry points rather than through Rust's allocator.
unsafe extern "C" {
    fn malloc(size: size_t) -> *mut c_void;
    fn calloc(nmemb: size_t, size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    /// The C source calls `strcpy` (it imports `strcpy@GLIBC_2.2.5`).  We call
    /// the very same libc routine rather than reimplementing it in Rust so that
    /// *every* observable behaviour matches, including the SIGSEGV the C
    /// library produces for `create_block(id, NULL, flags)` — a hand-written
    /// Rust loop would instead trip rustc's null-pointer-dereference check and
    /// abort with SIGABRT.
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    /// Used to read struct fields through a possibly-invalid pointer exactly
    /// the way C does: an unchecked machine load.  A plain `(*p).field` in Rust
    /// is instrumented with a null check when `debug-assertions` are on, which
    /// would turn C's SIGSEGV into a Rust SIGABRT.
    fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
}

/// typedef struct { int id; char name[32]; uint8_t flags; } DataBlock;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub name: [c_char; 32],
    pub flags: u8,
}

/// typedef struct { int *data; size_t size; } MemoryBlock;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryBlock {
    pub data: *mut c_int,
    pub size: size_t,
}

/// `strcpy(dst, src)` — copies bytes up to and including the terminating NUL.
/// Delegates to libc so the behaviour is byte-for-byte the C library's,
/// including for a `NULL` source (SIGSEGV) and for a source longer than the
/// destination field (writes past it, exactly as the C code does).
#[inline]
unsafe fn strcpy_raw(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    unsafe { strcpy(dst, src) }
}

/// Read the `data` field of a `MemoryBlock` with a raw, *unchecked* machine
/// load — the C code does `mb->data` with no null check, so an invalid `mb`
/// must fault in the same way.
#[inline]
unsafe fn load_data_field(mb: *const MemoryBlock) -> *mut c_int {
    unsafe {
        let mut out: *mut c_int = core::ptr::null_mut();
        memcpy(
            &mut out as *mut *mut c_int as *mut c_void,
            (mb as *const u8).wrapping_add(core::mem::offset_of!(MemoryBlock, data))
                as *const c_void,
            core::mem::size_of::<*mut c_int>(),
        );
        out
    }
}

/// Build a `DataBlock` whose `name` field holds the given C string literal,
/// zero padded (exactly what a C `char name[32]` string initializer yields).
#[inline]
const fn name_field(s: &[u8]) -> [c_char; 32] {
    let mut out = [0 as c_char; 32];
    let mut i = 0;
    while i < s.len() && i < 32 {
        out[i] = s[i] as c_char;
        i += 1;
    }
    out
}

// -------------------------------------------------------------------------
// DataBlock create_block(int id, const char *name, uint8_t flags)
// -------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_block(id: c_int, name: *const c_char, flags: u8) -> DataBlock {
    let mut block = DataBlock {
        id: 0,
        name: [0 as c_char; 32],
        flags: 0,
    };
    block.id = id;
    unsafe {
        strcpy_raw(block.name.as_mut_ptr(), name);
    }
    block.flags = flags;
    block
}

// -------------------------------------------------------------------------
// MemoryBlock* allocate_block(size_t count, int init_value)
// -------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_block(count: size_t, init_value: c_int) -> *mut MemoryBlock {
    unsafe {
        let mb = malloc(core::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
        if mb.is_null() {
            return core::ptr::null_mut();
        }

        (*mb).data = calloc(count, core::mem::size_of::<c_int>()) as *mut c_int;
        if (*mb).data.is_null() {
            free(mb as *mut c_void);
            return core::ptr::null_mut();
        }

        (*mb).size = count;

        let mut i: size_t = 0;
        while i < count {
            // C: mb->data[i] = init_value + i;
            // `init_value` is converted to size_t, added, then truncated back
            // to int on assignment.
            let v = (init_value as size_t).wrapping_add(i);
            *(*mb).data.add(i) = v as u32 as c_int;
            i += 1;
        }

        mb
    }
}

// -------------------------------------------------------------------------
// void free_block(MemoryBlock *mb)
// -------------------------------------------------------------------------
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

// -------------------------------------------------------------------------
// int compute_hash(MemoryBlock *mb1, MemoryBlock *mb2)
// -------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_hash(mb1: *mut MemoryBlock, mb2: *mut MemoryBlock) -> c_int {
    unsafe {
        let mut hash: c_int = 0;

        // C: `mb1->data < mb2->data` — an unchecked load followed by an
        // *unsigned* pointer comparison.
        let d1 = load_data_field(mb1) as usize;
        let d2 = load_data_field(mb2) as usize;
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
}

// -------------------------------------------------------------------------
// int betagamma(int param1, int param2, int param3, int param4)
// -------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn betagamma(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        let mut result: c_int = 0;

        let mut blocks: [DataBlock; 3] = [
            DataBlock {
                id: 1,
                name: name_field(b"Block_Alpha"),
                flags: 0b10101010,
            },
            DataBlock {
                id: 2,
                name: name_field(b"Block_Beta"),
                flags: 0b11001100,
            },
            DataBlock {
                id: 3,
                name: name_field(b"Block_Gamma"),
                flags: 0b11110000,
            },
        ];

        let num_blocks: c_int = blocks.len() as c_int;

        let mut i: c_int = 0;
        while i < num_blocks {
            let current: *mut DataBlock = blocks.as_mut_ptr().add(i as usize);

            // char temp_name[32]; strcpy(temp_name, current->name);
            let mut temp_name = [0 as c_char; 32];
            strcpy_raw(temp_name.as_mut_ptr(), (*current).name.as_ptr());
            let _ = &temp_name;

            let mut flag_contribution: c_int = 0;
            if (*current).flags & 0b00001111 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param1);
            }
            if (*current).flags & 0b11110000 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param2);
            }
            if (*current).flags & 0b10101010 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param3);
            }
            if (*current).flags & 0b01010101 != 0 {
                flag_contribution = flag_contribution.wrapping_add(param4);
            }

            result = result.wrapping_add(flag_contribution.wrapping_mul((*current).id));

            i += 1;
        }

        // size_t block_size = (param1 % 10) + 5;
        let block_size: size_t = ((param1 % 10).wrapping_add(5)) as size_t;
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
        let mut j: size_t = 0;
        while j < (*mem1).size {
            sum1 = sum1.wrapping_add(*(*mem1).data.add(j));
            j += 1;
        }
        let mut k: size_t = 0;
        while k < (*mem2).size {
            sum2 = sum2.wrapping_add(*(*mem2).data.add(k));
            k += 1;
        }

        result = result.wrapping_add(sum1.wrapping_sub(sum2) / 10);

        let mut special = DataBlock {
            id: 99,
            name: name_field(b"Special"),
            flags: 0b11111111,
        };
        strcpy_raw(special.name.as_mut_ptr(), b"Modified\0".as_ptr() as *const c_char);

        if (*mem1).data != (*mem2).data {
            result = result.wrapping_add(special.id);
        }

        // C: if (mem1->data > NULL && mem2->data > NULL)
        if !(*mem1).data.is_null() && !(*mem2).data.is_null() {
            result = result.wrapping_add(special.flags as c_int);
        }

        free_block(mem1);
        free_block(mem2);

        result
    }
}
