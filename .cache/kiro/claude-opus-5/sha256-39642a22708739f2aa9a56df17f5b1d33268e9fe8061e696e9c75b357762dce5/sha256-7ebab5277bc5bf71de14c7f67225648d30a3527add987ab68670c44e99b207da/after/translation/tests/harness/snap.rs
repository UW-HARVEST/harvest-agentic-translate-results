//! Structural snapshots of the stb_ds data structures, taken by reading the
//! memory a `.so` produced.  Pointers are never compared directly (they differ
//! between the two allocations); instead the contents they point at are.
#![allow(dead_code)]

use std::ffi::{c_char, c_void};

pub const HEADER_SIZE: usize = 32;

// stbds_hash_index field byte offsets on a 64-bit LP64 target.
pub const HI_TEMP_KEY: usize = 0;
pub const HI_SLOT_COUNT: usize = 8;
pub const HI_USED_COUNT: usize = 16;
pub const HI_USED_COUNT_THRESHOLD: usize = 24;
pub const HI_USED_COUNT_SHRINK_THRESHOLD: usize = 32;
pub const HI_TOMBSTONE_COUNT: usize = 40;
pub const HI_TOMBSTONE_COUNT_THRESHOLD: usize = 48;
pub const HI_SEED: usize = 56;
pub const HI_SLOT_COUNT_LOG2: usize = 64;
pub const HI_STRING_STORAGE: usize = 72;
pub const HI_STRING_REMAINING: usize = 80;
pub const HI_STRING_BLOCK: usize = 88;
pub const HI_STRING_MODE: usize = 89;
pub const HI_STORAGE: usize = 96;

/// `sizeof(stbds_string_arena)`
pub const ARENA_SIZE: usize = 24;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HeaderSnap {
    pub length: usize,
    pub capacity: usize,
    pub has_hash_table: bool,
    pub temp: isize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ArenaSnap {
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
    /// number of blocks in the linked list
    pub chain_len: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HashIndexSnap {
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub arena: ArenaSnap,
    pub hashes: Vec<usize>,
    pub indices: Vec<isize>,
}

/// One element of a hash-map array, rendered so it can be compared across
/// libraries.  `Binary` keeps raw bytes; `Str` decodes the leading `char *`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ElemSnap {
    Binary(Vec<u8>),
    Str {
        key: Option<Vec<u8>>,
        rest: Vec<u8>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MapSnap {
    pub header: HeaderSnap,
    pub index: Option<HashIndexSnap>,
    pub elems: Vec<ElemSnap>,
    /// `stbds_temp_key` (the `temp_key` field), decoded as a C string.
    pub temp_key: Option<Vec<u8>>,
}

pub unsafe fn read_usize(p: *const u8, off: usize) -> usize {
    unsafe { (p.add(off) as *const usize).read_unaligned() }
}

pub unsafe fn read_isize(p: *const u8, off: usize) -> isize {
    unsafe { (p.add(off) as *const isize).read_unaligned() }
}

pub unsafe fn read_ptr(p: *const u8, off: usize) -> *mut u8 {
    unsafe { (p.add(off) as *const *mut u8).read_unaligned() }
}

pub unsafe fn cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let mut v = Vec::new();
        let mut i = 0;
        loop {
            let b = *(p.add(i) as *const u8);
            if b == 0 {
                break;
            }
            v.push(b);
            i += 1;
            assert!(i < 1 << 20, "runaway C string");
        }
        Some(v)
    }
}

/// Snapshot the array header sitting in front of `raw` (the *raw* array
/// pointer, i.e. the hash-map pointer minus `elemsize`).
pub unsafe fn snap_header(raw: *mut c_void) -> HeaderSnap {
    unsafe {
        let h = (raw as *const u8).sub(HEADER_SIZE);
        HeaderSnap {
            length: read_usize(h, 0),
            capacity: read_usize(h, 8),
            has_hash_table: !read_ptr(h, 16).is_null(),
            temp: read_isize(h, 24),
        }
    }
}

pub unsafe fn snap_arena_at(p: *const u8) -> ArenaSnap {
    unsafe {
        let mut chain_len = 0usize;
        let mut blk = read_ptr(p, 0);
        while !blk.is_null() {
            chain_len += 1;
            assert!(chain_len < 1 << 16, "runaway arena chain");
            blk = read_ptr(blk, 0);
        }
        ArenaSnap {
            remaining: read_usize(p, 8),
            block: *p.add(16),
            mode: *p.add(17),
            chain_len,
        }
    }
}

pub unsafe fn snap_hash_index(t: *const u8) -> HashIndexSnap {
    unsafe {
        let slot_count = read_usize(t, HI_SLOT_COUNT);
        assert!(
            slot_count.is_power_of_two() && slot_count <= (1 << 24),
            "implausible slot_count {slot_count} - struct offsets wrong?"
        );
        let storage = read_ptr(t, HI_STORAGE);
        let nbuckets = slot_count >> 3;
        let mut hashes = Vec::with_capacity(slot_count);
        let mut indices = Vec::with_capacity(slot_count);
        for b in 0..nbuckets {
            // stbds_hash_bucket = size_t hash[8]; ptrdiff_t index[8]; => 128 bytes
            let bucket = storage.add(b * 128);
            for j in 0..8 {
                hashes.push(read_usize(bucket, j * 8));
            }
            for j in 0..8 {
                indices.push(read_isize(bucket, 64 + j * 8));
            }
        }
        HashIndexSnap {
            slot_count,
            used_count: read_usize(t, HI_USED_COUNT),
            used_count_threshold: read_usize(t, HI_USED_COUNT_THRESHOLD),
            used_count_shrink_threshold: read_usize(t, HI_USED_COUNT_SHRINK_THRESHOLD),
            tombstone_count: read_usize(t, HI_TOMBSTONE_COUNT),
            tombstone_count_threshold: read_usize(t, HI_TOMBSTONE_COUNT_THRESHOLD),
            seed: read_usize(t, HI_SEED),
            slot_count_log2: read_usize(t, HI_SLOT_COUNT_LOG2),
            arena: snap_arena_at(t.add(HI_STRING_STORAGE)),
            hashes,
            indices,
        }
    }
}

/// How the key of an element should be rendered.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum KeyKind {
    /// keys live inline in the element: compare raw bytes
    Binary,
    /// element starts with a `char *`: compare the pointee string
    StrPtr,
}

/// Snapshot a hash map given the *user* pointer `t` (i.e. what
/// `stbds_hmput_key` returned).
pub unsafe fn snap_map(t: *mut c_void, elemsize: usize, kind: KeyKind) -> MapSnap {
    unsafe { snap_map_ex(t, elemsize, kind, false) }
}

/// As [`snap_map`], but also captures `stbds_temp_key`.  Only pass
/// `read_temp_key = true` when a string-mode put/get has definitely written it:
/// `stbds_make_hash_index` leaves the field uninitialised.
pub unsafe fn snap_map_ex(
    t: *mut c_void,
    elemsize: usize,
    kind: KeyKind,
    read_temp_key: bool,
) -> MapSnap {
    unsafe {
        if t.is_null() {
            return MapSnap {
                header: HeaderSnap {
                    length: 0,
                    capacity: 0,
                    has_hash_table: false,
                    temp: 0,
                },
                index: None,
                elems: Vec::new(),
                temp_key: None,
            };
        }
        let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
        let header = snap_header(raw);
        let ht = read_ptr((raw as *const u8).sub(HEADER_SIZE), 16);
        let index = if ht.is_null() {
            None
        } else {
            Some(snap_hash_index(ht))
        };
        let temp_key = if ht.is_null() || !read_temp_key {
            None
        } else {
            cstr(read_ptr(ht, HI_TEMP_KEY) as *const c_char)
        };

        let mut elems = Vec::with_capacity(header.length);
        for i in 0..header.length {
            let e = (raw as *const u8).add(elemsize * i);
            match kind {
                KeyKind::Binary => {
                    elems.push(ElemSnap::Binary(
                        std::slice::from_raw_parts(e, elemsize).to_vec(),
                    ));
                }
                KeyKind::StrPtr => {
                    let key = cstr(read_ptr(e, 0) as *const c_char);
                    elems.push(ElemSnap::Str {
                        key,
                        rest: std::slice::from_raw_parts(e.add(8), elemsize - 8).to_vec(),
                    });
                }
            }
        }
        MapSnap {
            header,
            index,
            elems,
            temp_key,
        }
    }
}
