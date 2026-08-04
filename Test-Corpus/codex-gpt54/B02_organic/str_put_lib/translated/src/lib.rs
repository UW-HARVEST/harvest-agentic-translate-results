use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

const STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[repr(C)]
struct StringBlock {
    next: *mut StringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
struct StringArena {
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

impl Default for StringArena {
    fn default() -> Self {
        Self {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(dst: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

static mut BUFFER: [c_char; 256] = [0; 256];
static A_KEY: [u8; 2] = *b"a\0";
static PRINTF_FMT: [u8; 7] = *b"%s %d\n\0";
static STRKEY_FMT: [u8; 8] = *b"test_%d\0";

unsafe fn alloc_block(size: usize) -> *mut StringBlock {
    unsafe { malloc(size) as *mut StringBlock }
}

unsafe fn stbds_stralloc(arena: &mut StringArena, src: *const c_char) -> *mut c_char {
    let len = {
        let mut n = 0usize;
        unsafe {
            while *src.add(n) != 0 {
                n += 1;
            }
        }
        n + 1
    };

    if len > arena.remaining {
        let mut blocksize = arena.block as usize;
        blocksize = STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STRING_ARENA_BLOCKSIZE_MAX {
            arena.block = arena.block.wrapping_add(1);
        }

        if len > blocksize {
            let alloc_size = size_of::<StringBlock>() - 8 + len;
            unsafe {
                let sb = alloc_block(alloc_size);
                ptr::copy_nonoverlapping(src.cast::<u8>(), (*sb).storage.as_mut_ptr().cast::<u8>(), len);
                if !arena.storage.is_null() {
                    (*sb).next = (*arena.storage).next;
                    (*arena.storage).next = sb;
                } else {
                    (*sb).next = ptr::null_mut();
                    arena.storage = sb;
                    arena.remaining = 0;
                }
                return (*sb).storage.as_mut_ptr();
            }
        }

        let alloc_size = size_of::<StringBlock>() - 8 + blocksize;
        let sb = unsafe { alloc_block(alloc_size) };
        unsafe {
            (*sb).next = arena.storage;
        }
        arena.storage = sb;
        arena.remaining = blocksize;
    }

    assert!(len <= arena.remaining);
    let dst = unsafe {
        (*arena.storage)
            .storage
            .as_mut_ptr()
            .cast::<u8>()
            .add(arena.remaining - len)
            .cast::<c_char>()
    };
    arena.remaining -= len;
    unsafe {
        ptr::copy_nonoverlapping(src.cast::<u8>(), dst.cast::<u8>(), len);
    }
    dst
}

unsafe fn stbds_strreset(arena: &mut StringArena) {
    let mut current = arena.storage;
    while !current.is_null() {
        let next = unsafe { (*current).next };
        unsafe {
            free(current.cast::<c_void>());
        }
        current = next;
    }
    *arena = StringArena::default();
}

unsafe fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        sprintf(
            std::ptr::addr_of_mut!(BUFFER).cast::<c_char>(),
            STRKEY_FMT.as_ptr().cast::<c_char>(),
            n,
        );
    }
    std::ptr::addr_of_mut!(BUFFER).cast::<c_char>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    let mut strmap: Vec<StrMapEntry> = Vec::new();
    let mut sa = StringArena::default();

    for i in 0..num {
        let key = unsafe { strkey(i) };
        let _ = unsafe { stbds_stralloc(&mut sa, key) };
    }
    unsafe {
        stbds_strreset(&mut sa);
    }

    let s = StrMapEntry {
        key: A_KEY.as_ptr().cast::<c_char>() as *mut c_char,
        value: num,
    };
    strmap.push(s);

    assert!(unsafe { *strmap[0].key } == b'a' as c_char);
    assert!(strmap[0].key == s.key);
    assert!(strmap[0].value == s.value);

    for z in 0..strmap.len() {
        let entry = strmap[z];
        unsafe {
            printf(PRINTF_FMT.as_ptr().cast::<c_char>(), entry.key, entry.value);
        }
    }
}
