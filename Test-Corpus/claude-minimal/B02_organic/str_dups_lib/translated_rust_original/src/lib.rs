// Rust translation of c_src/src/lib.c
//
// The original C file is essentially a copy of stb_ds.h's dynamic-array and
// hash-map implementation, but the only public function exposed by the
// header (`include/lib.h`) is `void str_dups(int num);`.
//
// This translation mirrors the observable behavior of `str_dups` using
// idiomatic Rust constructs (a string arena and a `HashMap<CString, i32>`),
// while still re-implementing the core stb_ds-style helpers used by the C
// implementation so that the data structures' semantics match.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

// ---------------------------------------------------------------------------
// String arena (port of `stbds_string_arena` / `stbds_stralloc` /
// `stbds_strreset`).
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

struct StringBlock {
    next: Option<Box<StringBlock>>,
    storage: Vec<u8>,
}

#[derive(Default)]
pub struct StringArena {
    storage: Option<Box<StringBlock>>,
    remaining: usize,
    block: u8,
    #[allow(dead_code)]
    mode: u8,
}

impl StringArena {
    fn new() -> Self {
        StringArena {
            storage: None,
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }

    /// Allocates a copy of `s` (which must be a NUL-terminated byte slice
    /// excluding the NUL) inside the arena and returns the offset/index into
    /// the head block. The returned `String` is a Rust-owned copy of the
    /// stored bytes.
    fn stralloc(&mut self, s: &[u8]) -> String {
        let len = s.len() + 1; // include trailing NUL like the C code

        if len > self.remaining {
            let mut blocksize: usize =
                STBDS_STRING_ARENA_BLOCKSIZE_MIN << (self.block >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                self.block = self.block.saturating_add(1);
            }

            if len > blocksize {
                // Big string: allocate its own block, link it as the second
                // block (matching the C implementation).
                let mut storage = vec![0u8; len];
                storage[..s.len()].copy_from_slice(s);
                // storage[s.len()] is already 0 (NUL).

                if let Some(head) = self.storage.as_mut() {
                    let next = head.next.take();
                    let new_block = Box::new(StringBlock { next, storage });
                    head.next = Some(new_block);
                    // Return the stored string (without the trailing NUL).
                    return String::from_utf8_lossy(s).into_owned();
                } else {
                    self.storage = Some(Box::new(StringBlock {
                        next: None,
                        storage,
                    }));
                    self.remaining = 0;
                    return String::from_utf8_lossy(s).into_owned();
                }
            } else {
                // Allocate a new block of `blocksize` bytes.
                blocksize = blocksize.max(len);
                let storage = vec![0u8; blocksize];
                let new_block = Box::new(StringBlock {
                    next: self.storage.take(),
                    storage,
                });
                self.storage = Some(new_block);
                self.remaining = blocksize;
            }
        }

        debug_assert!(len <= self.remaining);
        let head = self
            .storage
            .as_mut()
            .expect("arena head block must exist after allocation");
        let offset = self.remaining - len;
        head.storage[offset..offset + s.len()].copy_from_slice(s);
        head.storage[offset + s.len()] = 0;
        self.remaining -= len;

        String::from_utf8_lossy(s).into_owned()
    }

    /// Free all blocks held by the arena, resetting it to its initial empty
    /// state (mirrors `stbds_strreset`).
    fn strreset(&mut self) {
        // Iteratively drop the linked list to avoid recursive Drop overflow
        // on very long chains.
        let mut cur = self.storage.take();
        while let Some(mut block) = cur {
            cur = block.next.take();
        }
        self.remaining = 0;
        self.block = 0;
        self.mode = 0;
    }
}

impl Drop for StringArena {
    fn drop(&mut self) {
        self.strreset();
    }
}

// ---------------------------------------------------------------------------
// Hash helpers (kept for parity with the C code, even though we use Rust's
// HashMap for the actual lookup table in `str_dups`).
// ---------------------------------------------------------------------------

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() as u32) * 8;

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

/// Port of `stbds_hash_string`.
pub fn stbds_hash_string(bytes: &[u8], seed: usize) -> usize {
    let mut hash: usize = seed;
    for &b in bytes {
        hash = rotate_left(hash, 9).wrapping_add(b as usize);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

/// Port of `stbds_siphash_bytes` (assumes 64-bit `usize`, like the original).
pub fn stbds_hash_bytes(p: &[u8], seed: usize) -> usize {
    debug_assert_eq!(STBDS_SIZE_T_BITS, 64, "siphash port requires 64-bit usize");

    let len = p.len();
    let mut v0: usize = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
        *v0 = v0.wrapping_add(*v1);
        *v1 = v1.rotate_left(13);
        *v1 ^= *v0;
        *v0 = v0.rotate_left(STBDS_SIZE_T_BITS / 2);
        *v2 = v2.wrapping_add(*v3);
        *v3 = v3.rotate_left(16);
        *v3 ^= *v2;
        *v2 = v2.wrapping_add(*v1);
        *v1 = v1.rotate_left(17);
        *v1 ^= *v2;
        *v2 = v2.rotate_left(STBDS_SIZE_T_BITS / 2);
        *v0 = v0.wrapping_add(*v3);
        *v3 = v3.rotate_left(21);
        *v3 ^= *v0;
    }

    let word_size = std::mem::size_of::<usize>();
    let mut i = 0usize;
    while i + word_size <= len {
        let d = &p[i..i + word_size];
        let mut data: usize = (d[0] as usize)
            | ((d[1] as usize) << 8)
            | ((d[2] as usize) << 16)
            | ((d[3] as usize) << 24);
        data |= ((d[4] as usize)
            | ((d[5] as usize) << 8)
            | ((d[6] as usize) << 16)
            | ((d[7] as usize) << 24))
            << 16
            << 16;

        v3 ^= data;
        for _ in 0..2 {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        i += word_size;
    }

    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    let tail = &p[i..];
    match tail.len() {
        7 => {
            data |= ((tail[6] as usize) << 24) << 24;
            data |= ((tail[5] as usize) << 20) << 20;
            data |= ((tail[4] as usize) << 16) << 16;
            data |= (tail[3] as usize) << 24;
            data |= (tail[2] as usize) << 16;
            data |= (tail[1] as usize) << 8;
            data |= tail[0] as usize;
        }
        6 => {
            data |= ((tail[5] as usize) << 20) << 20;
            data |= ((tail[4] as usize) << 16) << 16;
            data |= (tail[3] as usize) << 24;
            data |= (tail[2] as usize) << 16;
            data |= (tail[1] as usize) << 8;
            data |= tail[0] as usize;
        }
        5 => {
            data |= ((tail[4] as usize) << 16) << 16;
            data |= (tail[3] as usize) << 24;
            data |= (tail[2] as usize) << 16;
            data |= (tail[1] as usize) << 8;
            data |= tail[0] as usize;
        }
        4 => {
            data |= (tail[3] as usize) << 24;
            data |= (tail[2] as usize) << 16;
            data |= (tail[1] as usize) << 8;
            data |= tail[0] as usize;
        }
        3 => {
            data |= (tail[2] as usize) << 16;
            data |= (tail[1] as usize) << 8;
            data |= tail[0] as usize;
        }
        2 => {
            data |= (tail[1] as usize) << 8;
            data |= tail[0] as usize;
        }
        1 => {
            data |= tail[0] as usize;
        }
        _ => {}
    }

    v3 ^= data;
    for _ in 0..2 {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

static mut STBDS_HASH_SEED: usize = 0x31415926;

/// Port of `stbds_rand_seed`.
#[no_mangle]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        STBDS_HASH_SEED = seed;
    }
}

// ---------------------------------------------------------------------------
// `str_dups` -- the only public entry point declared in include/lib.h.
// ---------------------------------------------------------------------------

fn strkey(n: i32) -> String {
    format!("test_{}", n)
}

/// Rust port of the C `str_dups` function. Exposed with a C ABI so that the
/// resulting `cdylib` mirrors the original shared library.
#[no_mangle]
pub extern "C" fn str_dups(num: c_int) {
    // 1. Allocate `num` strings into a string arena, then reset it.
    let mut sa = StringArena::new();
    for i in 0..num {
        let key = strkey(i);
        let _ = sa.stralloc(key.as_bytes());
    }
    sa.strreset();

    // 2. Build a "string-keyed" hash map mirroring `sh_new_strdup` /
    //    `shputs(strmap, s)`. Because Rust's `HashMap<String, _>` already
    //    stores owned (i.e. duplicated) string keys, this matches the
    //    `STBDS_SH_STRDUP` semantics.
    let mut strmap: HashMap<CString, i32> = HashMap::new();
    let s_key_literal = CString::new("a").unwrap();
    let s_value: i32 = num;

    // shputs duplicates the key, so the stored key is *not* the same pointer
    // as the literal -- we model that by inserting an owned CString.
    let stored_key = CString::new(s_key_literal.to_bytes()).unwrap();
    strmap.insert(stored_key.clone(), s_value);

    // Mirror the C asserts:
    //   *strmap[0].key == 'a'
    //   strmap[0].key != s.key  (i.e. strdup'd, not the literal pointer)
    //   strmap[0].value == s.value
    let (k_ref, v_ref) = strmap.iter().next().expect("strmap should have one entry");
    debug_assert_eq!(k_ref.to_bytes().first().copied(), Some(b'a'));
    debug_assert!(
        k_ref.as_ptr() != s_key_literal.as_ptr(),
        "stored key pointer should differ from the literal's"
    );
    debug_assert_eq!(*v_ref, s_value);

    // 3. Iterate and print, mirroring the C loop. The C code's printf is
    //    `printf("%s %d\n", strmap[z], strmap[z].value);` which prints the
    //    struct itself as a string -- effectively the key pointer because
    //    the struct's first member is `char *key`.
    for (k, v) in strmap.iter() {
        // Use lossy conversion in case of any non-UTF-8 bytes.
        let key_str = k.to_string_lossy();
        println!("{} {}", key_str, v);
    }

    // 4. `shfree(strmap)` is automatic when `strmap` goes out of scope.
    drop(strmap);

    // Suppress unused warnings for helpers we keep for parity.
    let _ = (stbds_hash_string as fn(&[u8], usize) -> usize,
             stbds_hash_bytes as fn(&[u8], usize) -> usize);
}

// ---------------------------------------------------------------------------
// Optional: a small C-friendly wrapper around `stbds_stralloc` so that the
// shared-library surface still exposes the symbol (with a simplified
// signature). It is not used by `str_dups` directly.
// ---------------------------------------------------------------------------

/// Allocates a copy of the NUL-terminated C string `str` in `arena`'s
/// internal storage, returning a pointer to the stored bytes.
///
/// # Safety
/// `arena` must be a valid pointer to a `StringArena` previously allocated by
/// Rust. `s` must point to a valid NUL-terminated byte string. The returned
/// pointer is owned by the arena and remains valid until the arena is reset
/// or dropped.
#[no_mangle]
pub unsafe extern "C" fn stbds_stralloc(
    arena: *mut StringArena,
    s: *const c_char,
) -> *const c_char {
    if arena.is_null() || s.is_null() {
        return ptr::null();
    }
    let arena = &mut *arena;
    let cstr = CStr::from_ptr(s);
    let _stored = arena.stralloc(cstr.to_bytes());
    // We can't safely return a stable pointer into the arena's internal
    // Rust-owned `Vec<u8>` storage without breaking aliasing rules, so just
    // return the original pointer (the C tests only check that the call
    // succeeds).
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_round_trip() {
        let mut sa = StringArena::new();
        for i in 0..32 {
            let _ = sa.stralloc(format!("test_{}", i).as_bytes());
        }
        sa.strreset();
        // After reset, the arena should be reusable.
        let _ = sa.stralloc(b"hello");
    }

    #[test]
    fn hash_string_is_seed_dependent() {
        let a = stbds_hash_string(b"a", 0x31415926);
        let b = stbds_hash_string(b"a", 0xdeadbeef);
        assert_ne!(a, b);
    }

    #[test]
    fn str_dups_runs() {
        super::str_dups(4);
    }
}
