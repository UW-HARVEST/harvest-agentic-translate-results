//! Phase B — exported low-level helper differential tests. CONFIGS.md rows 274-313.
//!
//! `utf.c`, `strbuffer.c`, `hashtable.c`, `memory.c`, `strconv.c`, `error.c` and
//! `version.c` all export real dynamic symbols (see SYMBOLS.md), so every one of
//! them is called directly through the two `.so` handles.
//!
//! The `#[repr(C)]` structs below mirror `c_src/src/strbuffer.h`,
//! `c_src/src/hashtable.h` and `c_src/include/jansson_private.h` exactly. Each
//! one is handed to the library inside a heap slab that is deliberately larger
//! than the mirrored layout and pre-filled with a 0xAA canary, so a layout
//! mismatch in either library shows up as a clobbered canary rather than as
//! silent corruption.
//!
//! The C library is ground truth.

// Comparing the allocator hook pointers is exactly the point of rows 307-309:
// the value handed to `json_set_alloc_funcs*` must come back out of the getter.
#![allow(unpredictable_function_pointer_comparisons)]
#![allow(unused_doc_comments)]

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void};
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

// ================================================================ mirrored layouts

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct strbuffer_t {
    value: *mut c_char,
    length: usize,
    size: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct hashtable_list {
    prev: *mut hashtable_list,
    next: *mut hashtable_list,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct hashtable_bucket {
    first: *mut hashtable_list,
    last: *mut hashtable_list,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct hashtable_t {
    size: usize,
    buckets: *mut hashtable_bucket,
    order: usize,
    list: hashtable_list,
    ordered_list: hashtable_list,
}

/// `#define LOOP_KEY_LEN (2 + (sizeof(json_t *) * 2) + 1)` (jansson_private.h:93)
const LOOP_KEY_LEN: usize = 2 + (std::mem::size_of::<*mut json_t>() * 2) + 1;

/// Extra canary bytes appended after every mirrored struct.
const SLACK: usize = 128;

/// A heap block that holds one mirrored struct followed by a 0xAA canary tail.
/// Backed by `Vec<u64>` so the base address is 8-byte aligned.
struct Slab {
    mem: Vec<u64>,
    used: usize,
}

impl Slab {
    fn new(used: usize) -> Slab {
        let words = (used + SLACK + 7) / 8;
        Slab { mem: vec![0xAAAA_AAAA_AAAA_AAAAu64; words], used }
    }
    fn ptr<T>(&mut self) -> *mut T {
        self.mem.as_mut_ptr() as *mut T
    }
    /// True while every byte past the mirrored struct is still the canary.
    fn tail_intact(&self) -> bool {
        let all = unsafe {
            std::slice::from_raw_parts(self.mem.as_ptr() as *const u8, self.mem.len() * 8)
        };
        all[self.used..].iter().all(|&b| b == 0xAA)
    }
}

fn sb_slab() -> Slab {
    Slab::new(std::mem::size_of::<strbuffer_t>())
}
fn ht_slab() -> Slab {
    Slab::new(std::mem::size_of::<hashtable_t>())
}

// ================================================================ fn types

type FnUtf8Encode = unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int;
type FnUtf8CheckFirst = unsafe extern "C" fn(c_char) -> usize;
type FnUtf8CheckFull = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize;
type FnUtf8Iterate = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char;
type FnUtf8CheckString = unsafe extern "C" fn(*const c_char, usize) -> c_int;

type FnSbInit = unsafe extern "C" fn(*mut strbuffer_t) -> c_int;
type FnSbVoid = unsafe extern "C" fn(*mut strbuffer_t);
type FnSbValue = unsafe extern "C" fn(*const strbuffer_t) -> *const c_char;
type FnSbSteal = unsafe extern "C" fn(*mut strbuffer_t) -> *mut c_char;
type FnSbAppendByte = unsafe extern "C" fn(*mut strbuffer_t, c_char) -> c_int;
type FnSbAppendBytes = unsafe extern "C" fn(*mut strbuffer_t, *const c_char, usize) -> c_int;
type FnSbPop = unsafe extern "C" fn(*mut strbuffer_t) -> c_char;

type FnHtInit = unsafe extern "C" fn(*mut hashtable_t) -> c_int;
type FnHtVoid = unsafe extern "C" fn(*mut hashtable_t);
type FnHtSet = unsafe extern "C" fn(*mut hashtable_t, *const c_char, usize, *mut json_t) -> c_int;
type FnHtGet = unsafe extern "C" fn(*mut hashtable_t, *const c_char, usize) -> *mut c_void;
type FnHtDel = unsafe extern "C" fn(*mut hashtable_t, *const c_char, usize) -> c_int;
type FnHtIter = unsafe extern "C" fn(*mut hashtable_t) -> *mut c_void;
type FnHtIterAt = unsafe extern "C" fn(*mut hashtable_t, *const c_char, usize) -> *mut c_void;
type FnHtIterNext = unsafe extern "C" fn(*mut hashtable_t, *mut c_void) -> *mut c_void;
type FnHtIterKey = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type FnHtIterKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
type FnHtIterValue = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type FnHtIterSet = unsafe extern "C" fn(*mut c_void, *mut json_t);

type FnStrtod = unsafe extern "C" fn(*mut strbuffer_t, *mut c_double) -> c_int;
type FnDtostr = unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int;
type FnStrndup = unsafe extern "C" fn(*const c_char, usize) -> *mut c_char;
type FnJMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
type FnJFree = unsafe extern "C" fn(*mut c_void);
type FnJRealloc = unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void;
type FnLoopCheck =
    unsafe extern "C" fn(*mut hashtable_t, *const json_t, *mut c_char, usize, *mut usize) -> c_int;
type FnSeed = unsafe extern "C" fn(usize);
type FnVersionStr = unsafe extern "C" fn() -> *const c_char;
type FnVersionCmp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

// allocator hooks (jansson.h:404-414)
type JsonMalloc = unsafe extern "C" fn(usize) -> *mut c_void;
type JsonRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type JsonFree = unsafe extern "C" fn(*mut c_void);
type FnSetAlloc = unsafe extern "C" fn(Option<JsonMalloc>, Option<JsonFree>);
type FnGetAlloc = unsafe extern "C" fn(*mut Option<JsonMalloc>, *mut Option<JsonFree>);
type FnSetAlloc2 =
    unsafe extern "C" fn(Option<JsonMalloc>, Option<JsonRealloc>, Option<JsonFree>);
type FnGetAlloc2 = unsafe extern "C" fn(
    *mut Option<JsonMalloc>,
    *mut Option<JsonRealloc>,
    *mut Option<JsonFree>,
);

// error.c (rows 312-313)
type FnErrInit = unsafe extern "C" fn(*mut json_error_t, *const c_char);
type FnErrSetSource = unsafe extern "C" fn(*mut json_error_t, *const c_char);
type FnErrSet =
    unsafe extern "C" fn(*mut json_error_t, c_int, c_int, usize, c_int, *const c_char, ...);

// ================================================================ global-state guard
//
// `json_set_alloc_funcs*` mutates a GLOBAL in the loaded library. cargo runs the
// `#[test]` fns of one binary on several threads, so the allocator test takes an
// exclusive lock while everything else takes a shared one.

fn alloc_lock() -> &'static RwLock<()> {
    static L: OnceLock<RwLock<()>> = OnceLock::new();
    L.get_or_init(|| RwLock::new(()))
}
fn shared_alloc() -> RwLockReadGuard<'static, ()> {
    alloc_lock().read().unwrap_or_else(|e| e.into_inner())
}
fn exclusive_alloc() -> RwLockWriteGuard<'static, ()> {
    alloc_lock().write().unwrap_or_else(|e| e.into_inner())
}

// ================================================================ helpers

unsafe fn n_int(lib: &Library, v: i64) -> *mut json_t {
    let f: Symbol<FnInt> = sym(lib, "json_integer");
    f(v)
}

/// `json_integer_value` of a `void*` returned by the hashtable, or `None` for NULL.
unsafe fn ival(lib: &Library, v: *mut c_void) -> Option<(c_int, json_int_t)> {
    if v.is_null() {
        return None;
    }
    let f: Symbol<FnIntVal> = sym(lib, "json_integer_value");
    Some(((*(v as *const json_t)).type_, f(v as *const json_t)))
}

/// Snapshot of a `strbuffer_t`: everything except the (necessarily different)
/// heap address.
#[derive(PartialEq, Debug)]
struct SbSnap {
    length: usize,
    size: usize,
    value_null: bool,
    /// `strbuffer_value()` read as `length` raw bytes (NUL-safe).
    raw: Vec<u8>,
    /// the NUL terminator the implementation is required to maintain
    nul_terminated: bool,
    tail_intact: bool,
}

unsafe fn sb_snap(lib: &Library, slab: &Slab, sb: *const strbuffer_t) -> SbSnap {
    let val: Symbol<FnSbValue> = sym(lib, "strbuffer_value");
    let p = val(sb);
    let length = (*sb).length;
    SbSnap {
        length,
        size: (*sb).size,
        value_null: p.is_null(),
        raw: if p.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(p as *const u8, length).to_vec()
        },
        nul_terminated: if p.is_null() { false } else { *p.add(length) == 0 },
        tail_intact: slab.tail_intact(),
    }
}

/// Insertion-ordered `(key bytes, key_len, value)` walk of a hashtable.
unsafe fn ht_order(lib: &Library, ht: *mut hashtable_t) -> Vec<(Vec<u8>, usize, Option<(c_int, json_int_t)>)> {
    let it: Symbol<FnHtIter> = sym(lib, "hashtable_iter");
    let nx: Symbol<FnHtIterNext> = sym(lib, "hashtable_iter_next");
    let ky: Symbol<FnHtIterKey> = sym(lib, "hashtable_iter_key");
    let kl: Symbol<FnHtIterKeyLen> = sym(lib, "hashtable_iter_key_len");
    let vl: Symbol<FnHtIterValue> = sym(lib, "hashtable_iter_value");
    let mut out = Vec::new();
    let mut iter = it(ht);
    while !iter.is_null() {
        let n = kl(iter);
        let k = ky(iter);
        out.push((
            std::slice::from_raw_parts(k as *const u8, n).to_vec(),
            n,
            ival(lib, vl(iter)),
        ));
        iter = nx(ht, iter);
    }
    out
}

#[derive(PartialEq, Debug)]
struct HtSnap {
    size: usize,
    order: usize,
    buckets_null: bool,
    /// every bucket slot that still points at the `&hashtable->list` sentinel
    empty_buckets: usize,
    list_self_linked: bool,
    order_list_self_linked: bool,
    tail_intact: bool,
    order_walk: Vec<(Vec<u8>, usize, Option<(c_int, json_int_t)>)>,
}

unsafe fn ht_snap(lib: &Library, slab: &Slab, ht: *mut hashtable_t) -> HtSnap {
    let sentinel = &mut (*ht).list as *mut hashtable_list;
    let nbuckets = 1usize << (*ht).order;
    let mut empty = 0usize;
    if !(*ht).buckets.is_null() {
        for i in 0..nbuckets {
            let b = (*ht).buckets.add(i);
            if (*b).first == sentinel && (*b).last == sentinel {
                empty += 1;
            }
        }
    }
    HtSnap {
        size: (*ht).size,
        order: (*ht).order,
        buckets_null: (*ht).buckets.is_null(),
        empty_buckets: empty,
        list_self_linked: (*ht).list.next == sentinel && (*ht).list.prev == sentinel,
        order_list_self_linked: (*ht).ordered_list.next
            == &mut (*ht).ordered_list as *mut hashtable_list
            && (*ht).ordered_list.prev == &mut (*ht).ordered_list as *mut hashtable_list,
        tail_intact: slab.tail_intact(),
        order_walk: ht_order(lib, ht),
    }
}

// ================================================================ rows 274-276

#[test]
fn rows274_276_utf8_encode() {
    let _g = shared_alloc();

    // Every length boundary, both surrogate halves and both rejection paths.
    let cps: &[i32] = &[
        i32::MIN,
        -1000,
        -1,
        0,
        1,
        0x41,
        0x7f,
        0x80,
        0x81,
        0x7ff,
        0x800,
        0x801,
        0xd7ff,
        0xd800, // ACCEPTED by utf8_encode (asymmetric with utf8_check_full)
        0xdbff,
        0xdc00,
        0xdfff,
        0xe000,
        0xfffd,
        0xffff,
        0x10000,
        0x10001,
        0x10ffff,
        0x110000,
        0x1f_ffff,
        0x7fff_ffff,
        i32::MAX,
    ];

    diff("rows274_276/utf8_encode", move |lib: &Library| unsafe {
        let f: Symbol<FnUtf8Encode> = sym(lib, "utf8_encode");
        let mut out = Vec::new();
        for &cp in cps {
            let mut buf = [0xCCu8; 8];
            let mut size: usize = 0xDEAD_BEEF;
            let r = f(cp, buf.as_mut_ptr() as *mut c_char, &mut size);
            out.push((cp, r, size, buf));
        }
        // Spot-check the exact encodings so this row cannot pass vacuously.
        let enc = |cp: i32| -> (c_int, usize, Vec<u8>) {
            let mut b = [0xCCu8; 8];
            let mut n: usize = 0xDEAD_BEEF;
            let r = f(cp, b.as_mut_ptr() as *mut c_char, &mut n);
            (r, n, if r == 0 { b[..n].to_vec() } else { Vec::new() })
        };
        assert_eq!(enc(0x00), (0, 1, vec![0x00]));
        assert_eq!(enc(0x7f), (0, 1, vec![0x7f]));
        assert_eq!(enc(0x80), (0, 2, vec![0xc2, 0x80]));
        assert_eq!(enc(0x7ff), (0, 2, vec![0xdf, 0xbf]));
        assert_eq!(enc(0x800), (0, 3, vec![0xe0, 0xa0, 0x80]));
        assert_eq!(enc(0xffff), (0, 3, vec![0xef, 0xbf, 0xbf]));
        assert_eq!(enc(0x10000), (0, 4, vec![0xf0, 0x90, 0x80, 0x80]));
        assert_eq!(enc(0x10ffff), (0, 4, vec![0xf4, 0x8f, 0xbf, 0xbf]));
        // row 275: surrogates are ACCEPTED and encode as 3 bytes
        assert_eq!(enc(0xd800), (0, 3, vec![0xed, 0xa0, 0x80]));
        assert_eq!(enc(0xdfff), (0, 3, vec![0xed, 0xbf, 0xbf]));
        // row 276: out-of-range codepoints are rejected and *size is untouched
        assert_eq!(enc(-1), (-1, 0xDEAD_BEEF, vec![]));
        assert_eq!(enc(0x110000), (-1, 0xDEAD_BEEF, vec![]));
        out
    });

    // Cross-check: every codepoint utf8_encode accepts round-trips through
    // utf8_check_full / utf8_iterate EXCEPT the surrogate range, which
    // utf8_check_full rejects (CONFIGS "notable findings" #3).
    diff_n("rows274_276/encode-then-check round trip", 0x1200, |lib: &Library, i: u64| unsafe {
        let enc: Symbol<FnUtf8Encode> = sym(lib, "utf8_encode");
        let full: Symbol<FnUtf8CheckFull> = sym(lib, "utf8_check_full");
        let iter: Symbol<FnUtf8Iterate> = sym(lib, "utf8_iterate");
        let first: Symbol<FnUtf8CheckFirst> = sym(lib, "utf8_check_first");
        // stride over the whole Unicode range plus a little beyond
        let cp = (i.wrapping_mul(0x97)) as i32 % 0x11_2000;
        let mut buf = [0xCCu8; 8];
        let mut size: usize = 0;
        let r = enc(cp, buf.as_mut_ptr() as *mut c_char, &mut size);
        if r != 0 {
            return (cp, r, size, buf, 0usize, -12345i32, 0usize, -1i64);
        }
        let mut back: i32 = -12345;
        let ok = full(buf.as_ptr() as *const c_char, size, &mut back);
        let cf = first(buf[0] as c_char);
        let mut it_cp: i32 = -999;
        let end = iter(buf.as_ptr() as *const c_char, size, &mut it_cp);
        let off = if end.is_null() {
            -1i64
        } else {
            end as i64 - buf.as_ptr() as i64
        };
        (cp, r, size, buf, ok, back, cf, off)
    });
}

// ================================================================ row 277

#[test]
fn row277_utf8_check_first_all_256_bytes() {
    let _g = shared_alloc();
    diff("row277/utf8_check_first x 256", |lib: &Library| unsafe {
        let f: Symbol<FnUtf8CheckFirst> = sym(lib, "utf8_check_first");
        let all: Vec<(u8, usize)> =
            (0u16..256).map(|b| (b as u8, f(b as u8 as c_char))).collect();
        // The documented classification (utf.c:38-67), asserted directly so the
        // row cannot pass on empty data.
        for (b, n) in &all {
            let expect = match *b {
                0x00..=0x7f => 1,
                0x80..=0xbf => 0,
                0xc0 | 0xc1 => 0,
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf4 => 4,
                _ => 0,
            };
            assert_eq!(*n, expect, "utf8_check_first(0x{:02x})", b);
        }
        all
    });
}

// ================================================================ rows 278-279

#[test]
fn rows278_279_utf8_check_full() {
    let _g = shared_alloc();

    // (bytes, size) pairs. The buffer handed to the library is always 8 bytes
    // wide (0x5A padded) so an out-of-range `size` cannot read past our slab.
    let cases: &[(&[u8], usize)] = &[
        // valid 2/3/4 byte sequences
        (b"\xc3\xa9", 2),
        (b"\xc2\x80", 2),
        (b"\xdf\xbf", 2),
        (b"\xe0\xa0\x80", 3),
        (b"\xe2\x82\xac", 3),
        (b"\xef\xbf\xbf", 3),
        (b"\xf0\x90\x8d\x88", 4),
        (b"\xf4\x8f\xbf\xbf", 4),
        // size 0, 1, 5, 6, 7 -> immediate 0
        (b"\xc3\xa9", 0),
        (b"a", 1),
        (b"\xc3\xa9", 1),
        (b"\xf0\x90\x8d\x88", 5),
        (b"\xf0\x90\x8d\x88", 6),
        (b"\xf0\x90\x8d\x88", 7),
        // overlong
        (b"\xc0\x80", 2),
        (b"\xc1\xbf", 2),
        (b"\xe0\x80\xa9", 3),
        (b"\xe0\x9f\xbf", 3),
        (b"\xf0\x80\x80\x80", 4),
        (b"\xf0\x8f\xbf\xbf", 4),
        // surrogate halves
        (b"\xed\xa0\x80", 3),
        (b"\xed\xbf\xbf", 3),
        // > 0x10FFFF
        (b"\xf4\x90\x80\x80", 4),
        (b"\xf4\xbf\xbf\xbf", 4),
        (b"\xf7\xbf\xbf\xbf", 4),
        // bad continuation bytes in each position
        (b"\xc3\x41", 2),
        (b"\xc3\xc3", 2),
        (b"\xe2\x41\xac", 3),
        (b"\xe2\x82\x41", 3),
        (b"\xe2\x82\xff", 3),
        (b"\xf0\x41\x8d\x88", 4),
        (b"\xf0\x90\x41\x88", 4),
        (b"\xf0\x90\x8d\x41", 4),
        (b"\x00\x80", 2),
        (b"\x41\x80", 2),
    ];

    diff("rows278_279/utf8_check_full", move |lib: &Library| unsafe {
        let f: Symbol<FnUtf8CheckFull> = sym(lib, "utf8_check_full");
        let mut out = Vec::new();
        for (bytes, size) in cases {
            let mut buf = [0x5Au8; 8];
            buf[..bytes.len()].copy_from_slice(bytes);
            // codepoint out-param present
            let mut cp: i32 = -12345;
            let r1 = f(buf.as_ptr() as *const c_char, *size, &mut cp);
            // codepoint out-param NULL
            let r2 = f(buf.as_ptr() as *const c_char, *size, std::ptr::null_mut());
            out.push((*bytes, *size, r1, cp, r2));
        }
        out
    });
}

// ================================================================ row 280

#[test]
fn row280_utf8_iterate() {
    let _g = shared_alloc();

    let cases: &[(&[u8], usize)] = &[
        (b"abc", 0),          // bufsize 0 -> buffer returned unchanged, cp untouched
        (b"", 0),
        (b"a", 1),            // 1-byte
        (b"\x00", 1),
        (b"\x7f", 1),
        (b"\xc3\xa9", 2),     // 2-byte
        (b"\xc3\xa9", 1),     // truncated -> NULL
        (b"\xe2\x82\xac", 3), // 3-byte
        (b"\xe2\x82\xac", 2), // truncated -> NULL
        (b"\xe2\x82\xac", 1),
        (b"\xf0\x90\x8d\x88", 4),
        (b"\xf0\x90\x8d\x88", 3),
        (b"\xf0\x90\x8d\x88", 1),
        (b"\x80abc", 4),      // continuation byte as lead -> NULL
        (b"\xff abc", 5),     // invalid lead -> NULL
        (b"\xc0\x80ab", 4),   // overlong -> NULL
        (b"\xed\xa0\x80a", 4),// surrogate -> NULL
        (b"\xc3\xa9xyz", 5),  // returns buffer+2
        (b"abc", 3),          // returns buffer+1
    ];

    diff("row280/utf8_iterate", move |lib: &Library| unsafe {
        let f: Symbol<FnUtf8Iterate> = sym(lib, "utf8_iterate");
        let mut out = Vec::new();
        for (bytes, size) in cases {
            let mut buf = [0x5Au8; 12];
            buf[..bytes.len()].copy_from_slice(bytes);
            let base = buf.as_ptr();
            let mut cp: i32 = -12345;
            let end = f(base as *const c_char, *size, &mut cp);
            let off = if end.is_null() { -1i64 } else { end as i64 - base as i64 };
            // codepoint == NULL variant
            let end2 = f(base as *const c_char, *size, std::ptr::null_mut());
            let off2 = if end2.is_null() { -1i64 } else { end2 as i64 - base as i64 };
            out.push((*bytes, *size, off, cp, off2));
        }
        out
    });
}

// ================================================================ row 281

#[test]
fn row281_utf8_check_string() {
    let _g = shared_alloc();

    let cases: &[(&[u8], usize)] = &[
        (b"", 0),
        (b"abc", 0),                     // length 0 is always valid
        (b"abc", 3),
        (b"a\x00c", 3),                  // embedded NUL is VALID
        (b"\x00\x00\x00", 3),
        (b"\xc3\xa9", 2),
        (b"\xc3\xa9", 1),                // truncated by length -> invalid
        (b"a\xc3\xa9b", 4),
        (b"a\xc3\xa9b", 2),              // sequence cut by length -> invalid
        (b"\xe2\x82\xac", 3),
        (b"\xe2\x82\xac", 2),
        (b"\xf0\x90\x8d\x88", 4),
        (b"\xf0\x90\x8d\x88", 3),
        (b"\xf0\x90\x8d\x88x", 5),
        (b"\x80", 1),
        (b"\xff", 1),
        (b"\xc0\x80", 2),
        (b"\xed\xa0\x80", 3),
        (b"\xf4\x90\x80\x80", 4),
        (b"mixed \xc3\xa9 \xe2\x82\xac \xf0\x90\x8d\x88 end", 27),
        (b"\x01\x1f\x7f", 3),
    ];

    diff("row281/utf8_check_string", move |lib: &Library| unsafe {
        let f: Symbol<FnUtf8CheckString> = sym(lib, "utf8_check_string");
        cases
            .iter()
            .map(|(bytes, len)| {
                let mut buf = [0x5Au8; 40];
                buf[..bytes.len()].copy_from_slice(bytes);
                (*bytes, *len, f(buf.as_ptr() as *const c_char, *len))
            })
            .collect::<Vec<_>>()
    });
}

// ================================================================ randomized utf8

/// 1..6 random bytes, heavily biased towards lead/continuation byte values so a
/// large fraction of the samples land in `utf8_check_full`'s interesting cases.
fn rand_utf8ish(rng: &mut Rng, maxlen: usize) -> Vec<u8> {
    let n = 1 + rng.below(maxlen as u64) as usize;
    (0..n)
        .map(|_| match rng.below(12) {
            0..=2 => 0xC0u8.wrapping_add(rng.below(0x40) as u8), // 2/3/4-byte leads
            3..=6 => 0x80u8.wrapping_add(rng.below(0x40) as u8), // continuations
            7..=8 => rng.below(0x80) as u8,                      // ASCII (incl. NUL)
            9 => 0xF0u8.wrapping_add(rng.below(0x10) as u8),     // F0..FF
            _ => rng.next_u64() as u8,                           // anything at all
        })
        .collect()
}

#[test]
fn rows278_281_utf8_randomized() {
    let _g = shared_alloc();

    diff_n("rows278_281/randomized 1-6 byte strings", 2600, |lib: &Library, it: u64| unsafe {
        let first: Symbol<FnUtf8CheckFirst> = sym(lib, "utf8_check_first");
        let full: Symbol<FnUtf8CheckFull> = sym(lib, "utf8_check_full");
        let iter: Symbol<FnUtf8Iterate> = sym(lib, "utf8_iterate");
        let string: Symbol<FnUtf8CheckString> = sym(lib, "utf8_check_string");

        let mut rng = Rng::new(0xF00D_0001 ^ it.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let b = rand_utf8ish(&mut rng, 6);
        // 16-byte slab: 0x5A padding keeps every out-of-range `size` in bounds
        let mut buf = [0x5Au8; 16];
        buf[..b.len()].copy_from_slice(&b);
        let base = buf.as_ptr() as *const c_char;

        let cf = first(buf[0] as c_char);

        // utf8_check_full at the length check_first asked for, at the real
        // length, and at every size 0..=6
        let mut fulls = Vec::new();
        for size in 0..=6usize {
            let mut cp: i32 = -12345;
            let r = full(base, size, &mut cp);
            let r_nullcp = full(base, size, std::ptr::null_mut());
            fulls.push((size, r, cp, r_nullcp));
        }

        // utf8_iterate at every bufsize 0..=6
        let mut iters = Vec::new();
        for size in 0..=6usize {
            let mut cp: i32 = -12345;
            let end = iter(base, size, &mut cp);
            let off = if end.is_null() { -1i64 } else { end as i64 - base as i64 };
            iters.push((size, off, cp));
        }

        // utf8_check_string at every length 0..=6, and full-buffer walks
        let mut strs = Vec::new();
        for len in 0..=6usize {
            strs.push((len, string(base, len)));
        }

        // Iterate the whole buffer the way load.c/dump.c do, collecting the
        // codepoint stream (or the failure position).
        let mut walk: Vec<i32> = Vec::new();
        let mut pos = 0usize;
        let mut fail_at: i64 = -1;
        while pos < b.len() {
            let mut cp: i32 = -12345;
            let end = iter(base.add(pos), b.len() - pos, &mut cp);
            if end.is_null() {
                fail_at = pos as i64;
                break;
            }
            walk.push(cp);
            let adv = end as usize - (base as usize + pos);
            if adv == 0 {
                break;
            }
            pos += adv;
        }

        (b, cf, fulls, iters, strs, walk, fail_at)
    });

    diff_n("rows278_281/randomized long strings", 1200, |lib: &Library, it: u64| unsafe {
        let string: Symbol<FnUtf8CheckString> = sym(lib, "utf8_check_string");
        let mut rng = Rng::new(0xBEEF_0002 ^ it.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        // mix valid UTF-8 fragments with raw garbage
        let mut b: Vec<u8> = Vec::new();
        for _ in 0..rng.below(6) {
            b.extend_from_slice(rng.utf8_string(4).as_bytes());
            b.extend_from_slice(&rand_utf8ish(&mut rng, 3));
        }
        b.extend_from_slice(&[0x5A; 8]);
        let n = b.len();
        let base = b.as_ptr() as *const c_char;
        let per_len: Vec<(usize, c_int)> = (0..=n).map(|l| (l, string(base, l))).collect();
        (b.clone(), per_len)
    });
}

// ================================================================ rows 282-289

#[test]
fn rows282_289_strbuffer() {
    let _g = shared_alloc();

    // row 282 + 287 (fresh) + 289 (close without steal)
    diff("row282/strbuffer_init", |lib: &Library| unsafe {
        let init: Symbol<FnSbInit> = sym(lib, "strbuffer_init");
        let close: Symbol<FnSbVoid> = sym(lib, "strbuffer_close");
        let value: Symbol<FnSbValue> = sym(lib, "strbuffer_value");
        let mut slab = sb_slab();
        let sb: *mut strbuffer_t = slab.ptr();
        let r = init(sb);
        let snap = sb_snap(lib, &slab, sb);
        let first_byte_nul = *value(sb) == 0;
        // row 282, verbatim: size 16, length 0, value[0] == '\0'
        assert_eq!(r, 0, "strbuffer_init");
        assert_eq!((*sb).size, 16, "STRBUFFER_MIN_SIZE");
        assert_eq!((*sb).length, 0, "fresh length");
        assert!(!(*sb).value.is_null(), "fresh value");
        assert!(first_byte_nul, "value[0] == '\\0'");
        assert!(slab.tail_intact(), "strbuffer_t layout is wider than mirrored");
        close(sb);
        let after = ((*sb).length, (*sb).size, (*sb).value.is_null(), slab.tail_intact());
        (
            r,
            snap,
            first_byte_nul,
            std::mem::size_of::<strbuffer_t>(),
            after,
        )
    });

    // row 283: append_byte crossing 16 -> 32 -> 64 -> 128
    diff("row283/append_byte growth", |lib: &Library| unsafe {
        let init: Symbol<FnSbInit> = sym(lib, "strbuffer_init");
        let close: Symbol<FnSbVoid> = sym(lib, "strbuffer_close");
        let ab: Symbol<FnSbAppendByte> = sym(lib, "strbuffer_append_byte");
        let mut slab = sb_slab();
        let sb: *mut strbuffer_t = slab.ptr();
        assert_eq!(init(sb), 0);
        let mut steps = Vec::new();
        for i in 0..140u32 {
            let byte = b'a' + (i % 26) as u8;
            let r = ab(sb, byte as c_char);
            steps.push((i, r, (*sb).length, (*sb).size));
        }
        let snap = sb_snap(lib, &slab, sb);
        // a NUL byte can be appended and is counted in `length`
        let r_nul = ab(sb, 0);
        let snap_nul = sb_snap(lib, &slab, sb);
        close(sb);
        (steps, snap, r_nul, snap_nul)
    });

    // row 284: append_bytes with size 0 / exactly-filling / overflow guards
    diff("row284/append_bytes", |lib: &Library| unsafe {
        let init: Symbol<FnSbInit> = sym(lib, "strbuffer_init");
        let close: Symbol<FnSbVoid> = sym(lib, "strbuffer_close");
        let abs: Symbol<FnSbAppendBytes> = sym(lib, "strbuffer_append_bytes");
        let mut out = Vec::new();

        // size 0 on a fresh buffer (no growth, NUL still written)
        {
            let mut slab = sb_slab();
            let sb: *mut strbuffer_t = slab.ptr();
            assert_eq!(init(sb), 0);
            let r = abs(sb, b"ignored".as_ptr() as *const c_char, 0);
            out.push(("size0", r, sb_snap(lib, &slab, sb)));
            // NULL data with size 0 (memcpy(_, NULL, 0))
            let r2 = abs(sb, std::ptr::null(), 0);
            out.push(("size0-null", r2, sb_snap(lib, &slab, sb)));
            close(sb);
        }

        // 15 bytes fit in the 16-byte buffer; the 16th grows it
        for n in [14usize, 15, 16, 17, 31, 32, 33, 63, 64, 100] {
            let mut slab = sb_slab();
            let sb: *mut strbuffer_t = slab.ptr();
            assert_eq!(init(sb), 0);
            let data: Vec<u8> = (0..n).map(|i| b'0' + (i % 10) as u8).collect();
            let r = abs(sb, data.as_ptr() as *const c_char, n);
            out.push(("fill", r, sb_snap(lib, &slab, sb)));
            // then one more byte
            let r2 = abs(sb, b"!".as_ptr() as *const c_char, 1);
            out.push(("fill+1", r2, sb_snap(lib, &slab, sb)));
            close(sb);
        }

        // embedded NUL bytes
        {
            let mut slab = sb_slab();
            let sb: *mut strbuffer_t = slab.ptr();
            assert_eq!(init(sb), 0);
            let r = abs(sb, b"a\0b\0c".as_ptr() as *const c_char, 5);
            out.push(("nuls", r, sb_snap(lib, &slab, sb)));
            close(sb);
        }

        // overflow guards (strbuffer.c:66-69) — each returns -1 without touching
        // the buffer. `size == SIZE_MAX` trips the second clause, and
        // `length > SIZE_MAX-1-size` trips the third.
        {
            let mut slab = sb_slab();
            let sb: *mut strbuffer_t = slab.ptr();
            assert_eq!(init(sb), 0);
            abs(sb, b"12345".as_ptr() as *const c_char, 5);
            let r_max = abs(sb, b"x".as_ptr() as *const c_char, usize::MAX);
            let s1 = sb_snap(lib, &slab, sb);
            let r_max1 = abs(sb, b"x".as_ptr() as *const c_char, usize::MAX - 5);
            let s2 = sb_snap(lib, &slab, sb);
            // huge-but-legal request: realloc fails, -1, buffer preserved
            let r_huge = abs(sb, b"x".as_ptr() as *const c_char, 1usize << 62);
            let s3 = sb_snap(lib, &slab, sb);
            out.push(("guard-max", r_max, s1));
            out.push(("guard-max-len", r_max1, s2));
            out.push(("guard-huge", r_huge, s3));
            close(sb);
        }
        out
    });

    // rows 285, 286, 287
    diff("rows285_287/pop, clear, value", |lib: &Library| unsafe {
        let init: Symbol<FnSbInit> = sym(lib, "strbuffer_init");
        let close: Symbol<FnSbVoid> = sym(lib, "strbuffer_close");
        let clear: Symbol<FnSbVoid> = sym(lib, "strbuffer_clear");
        let abs: Symbol<FnSbAppendBytes> = sym(lib, "strbuffer_append_bytes");
        let pop: Symbol<FnSbPop> = sym(lib, "strbuffer_pop");
        let value: Symbol<FnSbValue> = sym(lib, "strbuffer_value");

        let mut slab = sb_slab();
        let sb: *mut strbuffer_t = slab.ptr();
        assert_eq!(init(sb), 0);

        // pop on a brand-new (EMPTY) buffer
        let p_empty = (pop(sb) as u8, (*sb).length, (*sb).size);
        let p_empty2 = (pop(sb) as u8, (*sb).length);
        let fresh_value = (value(sb).is_null(), cstr_to_string(value(sb)));

        const FILL: &[u8] = b"hello world, this is longer than sixteen";
        abs(sb, FILL.as_ptr() as *const c_char, FILL.len());
        let populated = sb_snap(lib, &slab, sb);
        let populated_value = cstr_to_string(value(sb));

        // pop every byte back off, one at a time
        let mut popped = Vec::new();
        while (*sb).length > 0 {
            popped.push((pop(sb) as u8, (*sb).length, (*sb).size));
        }
        let drained = sb_snap(lib, &slab, sb);
        let p_after_drain = (pop(sb) as u8, (*sb).length);

        // clear on a populated buffer keeps the capacity
        const REFILL: &[u8] = b"refilled after drain";
        abs(sb, REFILL.as_ptr() as *const c_char, REFILL.len());
        let before_clear = sb_snap(lib, &slab, sb);
        clear(sb);
        let after_clear = sb_snap(lib, &slab, sb);
        let cleared_value = cstr_to_string(value(sb));
        // clear is idempotent, and the buffer is reusable afterwards
        clear(sb);
        const REUSE: &[u8] = b"reused";
        abs(sb, REUSE.as_ptr() as *const c_char, REUSE.len());
        let reused = sb_snap(lib, &slab, sb);
        close(sb);

        (
            (p_empty, p_empty2, fresh_value),
            (populated, populated_value),
            (popped, drained, p_after_drain),
            (before_clear, after_clear, cleared_value, reused),
        )
    });

    // rows 288, 289
    diff("rows288_289/steal_value then close", |lib: &Library| unsafe {
        let init: Symbol<FnSbInit> = sym(lib, "strbuffer_init");
        let close: Symbol<FnSbVoid> = sym(lib, "strbuffer_close");
        let abs: Symbol<FnSbAppendBytes> = sym(lib, "strbuffer_append_bytes");
        let steal: Symbol<FnSbSteal> = sym(lib, "strbuffer_steal_value");
        let jfree: Symbol<FnJFree> = sym(lib, "jsonp_free");

        // populated buffer
        let mut slab = sb_slab();
        let sb: *mut strbuffer_t = slab.ptr();
        assert_eq!(init(sb), 0);
        const PAYLOAD: &[u8] = b"stolen payload\0tail";
        abs(sb, PAYLOAD.as_ptr() as *const c_char, PAYLOAD.len());
        let before = sb_snap(lib, &slab, sb);
        let p = steal(sb);
        let stolen = (
            p.is_null(),
            std::slice::from_raw_parts(p as *const u8, PAYLOAD.len()).to_vec(),
            cstr_to_string(p),
            // steal only clears `value`; length/size are left as they were
            (*sb).length,
            (*sb).size,
            (*sb).value.is_null(),
            slab.tail_intact(),
        );
        // close after steal must not double-free
        close(sb);
        let after_close = ((*sb).length, (*sb).size, (*sb).value.is_null(), slab.tail_intact());
        jfree(p as *mut c_void);

        // steal from a fresh (empty) buffer
        let mut slab2 = sb_slab();
        let sb2: *mut strbuffer_t = slab2.ptr();
        assert_eq!(init(sb2), 0);
        let p2 = steal(sb2);
        let stolen_empty = (p2.is_null(), cstr_to_string(p2), (*sb2).length, (*sb2).size);
        close(sb2);
        jfree(p2 as *mut c_void);
        // close twice (value already NULL) is a no-op
        close(sb2);

        (before, stolen, after_close, stolen_empty)
    });
}

// ================================================================ rows 290-296

/// Keys covering hashlittle tail cases, key_len 0 and embedded NULs.
const HT_KEYS: &[&[u8]] = &[
    b"",
    b"a",
    b"bb",
    b"ccc",
    b"abcd",
    b"abcde",
    b"0123456789a",
    b"0123456789ab",
    b"0123456789abc",
    b"0123456789abcdef",
    b"0123456789abcdef01234567",
    b"0123456789abcdef012345678",
    b"n\0l",
    b"n\0l\0",
    b"\0",
    b"\0\0",
    b"prefix",
    b"prefixed",
    b"k00",
    b"k01",
    b"k02",
    b"k03",
    b"k04",
    b"k05",
];

#[test]
fn rows290_296_hashtable() {
    let _g = shared_alloc();

    // row 290
    diff("row290/hashtable_init+close", |lib: &Library| unsafe {
        let init: Symbol<FnHtInit> = sym(lib, "hashtable_init");
        let close: Symbol<FnHtVoid> = sym(lib, "hashtable_close");
        let mut slab = ht_slab();
        let ht: *mut hashtable_t = slab.ptr();
        let r = init(ht);
        let snap = ht_snap(lib, &slab, ht);
        // row 290, verbatim: order 3 => pow(2,3) == 8 buckets, all empty, size 0
        assert_eq!(r, 0, "hashtable_init");
        assert_eq!(snap.order, 3, "INITIAL_HASHTABLE_ORDER");
        assert_eq!(snap.size, 0, "fresh size");
        assert_eq!(snap.empty_buckets, 8, "8 buckets, all pointing at &ht->list");
        assert!(snap.list_self_linked && snap.order_list_self_linked, "list_init");
        assert!(snap.tail_intact, "hashtable_t layout is wider than mirrored");
        close(ht);
        (
            r,
            snap,
            std::mem::size_of::<hashtable_t>(),
            std::mem::size_of::<hashtable_list>(),
            std::mem::size_of::<hashtable_bucket>(),
            slab.tail_intact(),
        )
    });

    // rows 291, 292: set new / existing / rehash boundaries, key_len 0, NUL keys
    diff("rows291_292/hashtable_set", |lib: &Library| unsafe {
        let init: Symbol<FnHtInit> = sym(lib, "hashtable_init");
        let close: Symbol<FnHtVoid> = sym(lib, "hashtable_close");
        let set: Symbol<FnHtSet> = sym(lib, "hashtable_set");
        let mut slab = ht_slab();
        let ht: *mut hashtable_t = slab.ptr();
        assert_eq!(init(ht), 0);
        let mut steps = Vec::new();
        // 24 distinct keys: order 3 -> 4 at the 9th, 4 -> 5 at the 17th
        for (i, k) in HT_KEYS.iter().enumerate() {
            let r = set(ht, k.as_ptr() as *const c_char, k.len(), n_int(lib, i as i64));
            steps.push((i, r, (*ht).size, (*ht).order));
            // rehash happens on the way IN, when size >= hashsize(order):
            // keys 1..8 stay at order 3, the 9th moves to 4, the 17th to 5.
            let expect_order = if i < 8 {
                3
            } else if i < 16 {
                4
            } else {
                5
            };
            assert_eq!(r, 0, "hashtable_set #{}", i);
            assert_eq!((*ht).size, i + 1, "size after #{}", i);
            assert_eq!((*ht).order, expect_order, "order after #{}", i);
        }
        let full = ht_snap(lib, &slab, ht);
        // re-set existing keys: value replaced, ordinal position kept
        let mut resets = Vec::new();
        for (i, k) in HT_KEYS.iter().enumerate().filter(|(i, _)| i % 5 == 0) {
            let r = set(ht, k.as_ptr() as *const c_char, k.len(), n_int(lib, 1000 + i as i64));
            resets.push((i, r, (*ht).size, (*ht).order));
        }
        let after = ht_snap(lib, &slab, ht);
        close(ht);
        (steps, full, resets, after, slab.tail_intact())
    });

    // row 293: get hit / miss / key_len mismatch on an equal prefix
    diff("row293/hashtable_get", |lib: &Library| unsafe {
        let init: Symbol<FnHtInit> = sym(lib, "hashtable_init");
        let close: Symbol<FnHtVoid> = sym(lib, "hashtable_close");
        let set: Symbol<FnHtSet> = sym(lib, "hashtable_set");
        let get: Symbol<FnHtGet> = sym(lib, "hashtable_get");
        let mut slab = ht_slab();
        let ht: *mut hashtable_t = slab.ptr();
        assert_eq!(init(ht), 0);
        for (i, k) in HT_KEYS.iter().enumerate() {
            set(ht, k.as_ptr() as *const c_char, k.len(), n_int(lib, i as i64));
        }
        let mut out = Vec::new();
        // every inserted key at its own length, one shorter and one longer
        for k in HT_KEYS {
            let p = get(ht, k.as_ptr() as *const c_char, k.len());
            out.push((*k, k.len(), ival(lib, p)));
            if k.len() > 0 {
                let p2 = get(ht, k.as_ptr() as *const c_char, k.len() - 1);
                out.push((*k, k.len() - 1, ival(lib, p2)));
            }
        }
        // prefix/suffix confusion: "prefix" vs "prefixed"
        let long = b"prefixed";
        let mismatches = (
            ival(lib, get(ht, long.as_ptr() as *const c_char, 6)), // -> "prefix"
            ival(lib, get(ht, long.as_ptr() as *const c_char, 8)), // -> "prefixed"
            ival(lib, get(ht, long.as_ptr() as *const c_char, 7)), // -> miss
            ival(lib, get(ht, b"absent".as_ptr() as *const c_char, 6)),
            ival(lib, get(ht, b"".as_ptr() as *const c_char, 0)),
            ival(lib, get(ht, b"n\0l\0x".as_ptr() as *const c_char, 3)),
            ival(lib, get(ht, b"n\0l\0x".as_ptr() as *const c_char, 4)),
            ival(lib, get(ht, b"n\0l\0x".as_ptr() as *const c_char, 5)),
        );
        close(ht);
        (out, mismatches, slab.tail_intact())
    });

    // row 294: del hit / miss / all three relink branches
    diff("row294/hashtable_del", |lib: &Library| unsafe {
        let init: Symbol<FnHtInit> = sym(lib, "hashtable_init");
        let close: Symbol<FnHtVoid> = sym(lib, "hashtable_close");
        let set: Symbol<FnHtSet> = sym(lib, "hashtable_set");
        let del: Symbol<FnHtDel> = sym(lib, "hashtable_del");
        let mut out = Vec::new();
        // three tables, deleted forward / backward / middle-out so every
        // bucket->first / bucket->last / sole-entry relink branch is taken
        for mode in 0..3usize {
            let mut slab = ht_slab();
            let ht: *mut hashtable_t = slab.ptr();
            assert_eq!(init(ht), 0);
            for (i, k) in HT_KEYS.iter().enumerate() {
                set(ht, k.as_ptr() as *const c_char, k.len(), n_int(lib, i as i64));
            }
            let mut idx: Vec<usize> = (0..HT_KEYS.len()).collect();
            match mode {
                0 => {}
                1 => idx.reverse(),
                _ => {
                    let mid = idx.len() / 2;
                    idx.rotate_left(mid);
                }
            }
            let mut steps = Vec::new();
            for i in idx {
                let k = HT_KEYS[i];
                let r = del(ht, k.as_ptr() as *const c_char, k.len());
                let r_again = del(ht, k.as_ptr() as *const c_char, k.len());
                steps.push((i, r, r_again, (*ht).size, (*ht).order, ht_order(lib, ht)));
            }
            let r_miss = del(ht, b"absent".as_ptr() as *const c_char, 6);
            out.push((mode, steps, r_miss, ht_snap(lib, &slab, ht)));
            close(ht);
        }
        out
    });

    // row 295
    diff("row295/hashtable_clear then reuse", |lib: &Library| unsafe {
        let init: Symbol<FnHtInit> = sym(lib, "hashtable_init");
        let close: Symbol<FnHtVoid> = sym(lib, "hashtable_close");
        let set: Symbol<FnHtSet> = sym(lib, "hashtable_set");
        let clear: Symbol<FnHtVoid> = sym(lib, "hashtable_clear");
        let get: Symbol<FnHtGet> = sym(lib, "hashtable_get");
        let mut slab = ht_slab();
        let ht: *mut hashtable_t = slab.ptr();
        assert_eq!(init(ht), 0);
        for (i, k) in HT_KEYS.iter().enumerate() {
            set(ht, k.as_ptr() as *const c_char, k.len(), n_int(lib, i as i64));
        }
        let before = ht_snap(lib, &slab, ht);
        clear(ht);
        let cleared = ht_snap(lib, &slab, ht);
        let miss = ival(lib, get(ht, b"abcd".as_ptr() as *const c_char, 4));
        // reuse: the order (i.e. bucket count) is retained by hashtable_clear
        for (i, k) in HT_KEYS.iter().enumerate().take(10) {
            set(ht, k.as_ptr() as *const c_char, k.len(), n_int(lib, 500 + i as i64));
        }
        let reused = ht_snap(lib, &slab, ht);
        clear(ht);
        clear(ht);
        let twice = ht_snap(lib, &slab, ht);
        close(ht);
        (before, cleared, miss, reused, twice)
    });

    // row 296: the whole iterator family
    diff("row296/hashtable iterators", |lib: &Library| unsafe {
        let init: Symbol<FnHtInit> = sym(lib, "hashtable_init");
        let close: Symbol<FnHtVoid> = sym(lib, "hashtable_close");
        let set: Symbol<FnHtSet> = sym(lib, "hashtable_set");
        let it: Symbol<FnHtIter> = sym(lib, "hashtable_iter");
        let at: Symbol<FnHtIterAt> = sym(lib, "hashtable_iter_at");
        let nx: Symbol<FnHtIterNext> = sym(lib, "hashtable_iter_next");
        let ky: Symbol<FnHtIterKey> = sym(lib, "hashtable_iter_key");
        let kl: Symbol<FnHtIterKeyLen> = sym(lib, "hashtable_iter_key_len");
        let vl: Symbol<FnHtIterValue> = sym(lib, "hashtable_iter_value");
        let iset: Symbol<FnHtIterSet> = sym(lib, "hashtable_iter_set");

        let mut out = Vec::new();
        for n in [0usize, 1, 2, 8, 9, 16, 17, 24] {
            let mut slab = ht_slab();
            let ht: *mut hashtable_t = slab.ptr();
            assert_eq!(init(ht), 0);
            for (i, k) in HT_KEYS.iter().enumerate().take(n) {
                set(ht, k.as_ptr() as *const c_char, k.len(), n_int(lib, i as i64));
            }
            // empty table -> hashtable_iter returns NULL
            let head_null = it(ht).is_null();
            let order = ht_order(lib, ht);
            // iterate to exhaustion
            let mut steps = 0usize;
            let mut iter = it(ht);
            while !iter.is_null() {
                iter = nx(ht, iter);
                steps += 1;
            }
            // iter_at hit and miss
            let hit = if n > 4 {
                let k = HT_KEYS[3];
                let i = at(ht, k.as_ptr() as *const c_char, k.len());
                if i.is_null() {
                    None
                } else {
                    Some((
                        std::slice::from_raw_parts(ky(i) as *const u8, kl(i)).to_vec(),
                        kl(i),
                        ival(lib, vl(i)),
                    ))
                }
            } else {
                None
            };
            let miss = at(ht, b"absent".as_ptr() as *const c_char, 6).is_null();
            let miss_len = at(ht, b"abcd".as_ptr() as *const c_char, 3).is_null();
            // iter_set replaces the value in place
            let after_set = if n > 4 {
                let k = HT_KEYS[2];
                let i = at(ht, k.as_ptr() as *const c_char, k.len());
                iset(i, n_int(lib, -99));
                Some(ht_order(lib, ht))
            } else {
                None
            };
            out.push((n, head_null, order, steps, hit, miss, miss_len, after_set));
            close(ht);
        }
        out
    });
}

#[test]
fn rows291_296_hashtable_randomized() {
    let _g = shared_alloc();

    diff_n("rows291-296/randomized set/get/del/iterate", 340, |lib: &Library, iter: u64| unsafe {
        let init: Symbol<FnHtInit> = sym(lib, "hashtable_init");
        let close: Symbol<FnHtVoid> = sym(lib, "hashtable_close");
        let set: Symbol<FnHtSet> = sym(lib, "hashtable_set");
        let get: Symbol<FnHtGet> = sym(lib, "hashtable_get");
        let del: Symbol<FnHtDel> = sym(lib, "hashtable_del");
        let clear: Symbol<FnHtVoid> = sym(lib, "hashtable_clear");
        let at: Symbol<FnHtIterAt> = sym(lib, "hashtable_iter_at");
        let ky: Symbol<FnHtIterKey> = sym(lib, "hashtable_iter_key");
        let kl: Symbol<FnHtIterKeyLen> = sym(lib, "hashtable_iter_key_len");
        let vl: Symbol<FnHtIterValue> = sym(lib, "hashtable_iter_value");
        let iset: Symbol<FnHtIterSet> = sym(lib, "hashtable_iter_set");

        let mut rng = Rng::new(0x4A17_0055 ^ iter.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut slab = ht_slab();
        let ht: *mut hashtable_t = slab.ptr();
        assert_eq!(init(ht), 0);

        let mut log = Vec::new();
        for step in 0..30u64 {
            let op = rng.below(6);
            let ki = rng.below(HT_KEYS.len() as u64) as usize;
            let k = HT_KEYS[ki];
            let kp = k.as_ptr() as *const c_char;
            let ret: i64 = match op {
                0 | 1 => set(ht, kp, k.len(), n_int(lib, (step * 100 + ki as u64) as i64)) as i64,
                2 => del(ht, kp, k.len()) as i64,
                3 => match ival(lib, get(ht, kp, k.len())) {
                    None => -1,
                    Some((_, v)) => v,
                },
                4 => {
                    let i = at(ht, kp, k.len());
                    if i.is_null() {
                        -1
                    } else {
                        iset(i, n_int(lib, -(step as i64) - 1));
                        kl(i) as i64
                    }
                }
                _ => {
                    if rng.below(6) == 0 {
                        clear(ht);
                        -777
                    } else {
                        // random-length prefix of a pool key
                        let l = rng.below(k.len() as u64 + 2) as usize;
                        if l <= k.len() {
                            set(ht, kp, l, n_int(lib, 900 + l as i64)) as i64
                        } else {
                            -888
                        }
                    }
                }
            };
            log.push((step, op, ki, ret, (*ht).size, (*ht).order, ht_order(lib, ht)));
        }

        // final full walk through the raw iterator accessors
        let tail = ht_snap(lib, &slab, ht);
        let _ = (ky, vl);
        close(ht);
        (log, tail)
    });
}

// ================================================================ row 297

#[test]
fn row297_jsonp_strtod() {
    let _g = shared_alloc();

    // Every input must be consumed in full by strtod (strconv.c:53 asserts it).
    let texts: &[&str] = &[
        "0",
        "-0",
        "1",
        "-1",
        "123",
        "-123",
        "1.5",
        "-1.5",
        "0.0",
        "-0.0",
        "0.0001",
        "1e2",
        "1E2",
        "1e+2",
        "1e-2",
        "1E-2",
        "3.141592653589793",
        "2.2250738585072014e-308",
        "4.9406564584124654e-324",
        "1.7976931348623157e308",
        "9007199254740993",
        "1e308",
        "1e-308",
        "1e-999",  // underflow: 0.0 with ERANGE, still accepted (value != HUGE_VAL)
        "-1e-999",
        "1e999",   // overflow: HUGE_VAL + ERANGE -> rejected -1
        "-1e999",
        "1e310",
        "-1e310",
    ];

    diff("row297/jsonp_strtod", move |lib: &Library| unsafe {
        let init: Symbol<FnSbInit> = sym(lib, "strbuffer_init");
        let close: Symbol<FnSbVoid> = sym(lib, "strbuffer_close");
        let abs: Symbol<FnSbAppendBytes> = sym(lib, "strbuffer_append_bytes");
        let strtod: Symbol<FnStrtod> = sym(lib, "jsonp_strtod");
        let mut out = Vec::new();
        for t in texts {
            let mut slab = sb_slab();
            let sb: *mut strbuffer_t = slab.ptr();
            assert_eq!(init(sb), 0);
            assert_eq!(abs(sb, t.as_ptr() as *const c_char, t.len()), 0);
            // sentinel so we can see whether *out is written on the error path
            let mut d: c_double = f64::from_bits(0x7ff8_0000_dead_beef);
            let r = strtod(sb, &mut d);
            out.push((*t, r, d.to_bits(), (*sb).length, cstr_to_string((*sb).value)));
            close(sb);
        }
        out
    });
}

// ================================================================ rows 298-299

/// Values chosen to straddle `decpt <= -4` and `decpt > 16` (strconv.c:86).
const DTOSTR_VALUES: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    1.5,
    -1.5,
    0.5,
    100.0,
    1e-1,
    1e-2,
    1e-3,
    1e-4, // decpt == -3 -> fixed
    1e-5, // decpt == -4 -> exponent
    1e-6,
    1e14,
    1e15, // decpt == 16 -> fixed
    1e16, // decpt == 17 -> exponent
    1e17,
    9.9e15,
    1.0000000000000002e15,
    1234567890123456.0,
    12345678901234567.0,
    5e-324,
    2.2250738585072014e-308,
    1.7976931348623157e308,
    -1.7976931348623157e308,
    3.141592653589793,
    2.718281828459045,
    1e100,
    1e-100,
    1e300,
    1e-300,
    0.1,
    0.2,
    0.3,
    1.0 / 3.0,
    123456789.123456789,
];

#[test]
fn rows298_299_jsonp_dtostr() {
    let _g = shared_alloc();

    // precision 0 (dtoa mode 0) vs 1..17 (mode 2), across a generous buffer
    diff("rows298_299/dtostr precision sweep", |lib: &Library| unsafe {
        let f: Symbol<FnDtostr> = sym(lib, "jsonp_dtostr");
        let mut out = Vec::new();
        for &v in DTOSTR_VALUES {
            for p in 0..=31i32 {
                let mut buf = [0xCCu8; 64];
                let r = f(buf.as_mut_ptr() as *mut c_char, 64, v, p);
                out.push((v.to_bits(), p, r, buf));
            }
        }
        out
    });

    // buffer exactly large enough vs too small
    diff("rows298_299/dtostr buffer sizes", |lib: &Library| unsafe {
        let f: Symbol<FnDtostr> = sym(lib, "jsonp_dtostr");
        let mut out = Vec::new();
        for &v in DTOSTR_VALUES {
            for p in [0i32, 1, 6, 17] {
                for size in 0..=34usize {
                    // 64-byte slab, but only `size` is offered to the library
                    let mut buf = [0xCCu8; 64];
                    let r = f(buf.as_mut_ptr() as *mut c_char, size, v, p);
                    out.push((v.to_bits(), p, size, r, buf));
                }
            }
        }
        out
    });
}

#[test]
fn rows298_299_jsonp_dtostr_randomized() {
    let _g = shared_alloc();

    diff_n("rows298_299/dtostr randomized", 560, |lib: &Library, it: u64| unsafe {
        let f: Symbol<FnDtostr> = sym(lib, "jsonp_dtostr");
        let mut rng = Rng::new(0xD70_5712 ^ it.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        // mix of raw-bit doubles and "human" magnitudes
        let v = match it % 4 {
            0 => rng.f64_finite(),
            1 => (rng.i64() % 1_000_000_007) as f64 / (1 + rng.below(1_000_000)) as f64,
            2 => {
                let m = (rng.below(1u64 << 52) as f64) / (1u64 << 52) as f64;
                let e = rng.below(60) as i32 - 30;
                m * 10f64.powi(e)
            }
            _ => {
                let d = rng.f64_bits();
                if d.is_finite() {
                    d
                } else {
                    rng.f64_finite()
                }
            }
        };
        let mut out = Vec::new();
        for p in 0..=17i32 {
            let mut buf = [0xCCu8; 64];
            let r = f(buf.as_mut_ptr() as *mut c_char, 64, v, p);
            out.push((p, r, buf));
        }
        (v.to_bits(), out)
    });
}

// ================================================================ rows 300-302

#[test]
fn rows300_302_memory_helpers() {
    let _g = shared_alloc();

    // row 300
    diff("row300/jsonp_strndup", |lib: &Library| unsafe {
        let dup: Symbol<FnStrndup> = sym(lib, "jsonp_strndup");
        let jfree: Symbol<FnJFree> = sym(lib, "jsonp_free");
        let mut out = Vec::new();
        let cases: &[(&[u8], usize)] = &[
            (b"", 0),
            (b"hello", 0),
            (b"hello", 1),
            (b"hello", 4),
            (b"hello", 5),
            (b"a\0b\0c", 5),
            (b"a\0b\0c", 3),
            (b"\0\0\0", 3),
            (b"\xff\xfe\xfd", 3),
        ];
        for (data, len) in cases {
            let p = dup(data.as_ptr() as *const c_char, *len);
            let bytes = if p.is_null() {
                None
            } else {
                // len bytes plus the NUL jsonp_strndup is required to append
                Some(std::slice::from_raw_parts(p as *const u8, *len + 1).to_vec())
            };
            out.push((*data, *len, p.is_null(), bytes, cstr_to_string(p)));
            jfree(p as *mut c_void);
        }
        out
    });

    // rows 301, 302
    diff("rows301_302/jsonp_malloc + jsonp_free", |lib: &Library| unsafe {
        let jmalloc: Symbol<FnJMalloc> = sym(lib, "jsonp_malloc");
        let jfree: Symbol<FnJFree> = sym(lib, "jsonp_free");
        // size 0 returns NULL by design (memory.c:26-27)
        let zero = jmalloc(0).is_null();
        let mut sizes = Vec::new();
        for n in [1usize, 7, 8, 16, 100, 4096, 1 << 20] {
            let p = jmalloc(n);
            let usable = if p.is_null() {
                false
            } else {
                // prove the block is really writable
                std::ptr::write_bytes(p as *mut u8, 0x5A, n);
                *(p as *const u8).add(n - 1) == 0x5A
            };
            sizes.push((n, p.is_null(), usable));
            jfree(p);
        }
        // jsonp_free(NULL) is a documented no-op
        jfree(std::ptr::null_mut());
        jfree(std::ptr::null_mut());
        (zero, sizes)
    });
}

// ================================================================ rows 303, 307-309

/// Thin wrappers over libc so swapping them in cannot break anything else that
/// is running in this process.
extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

unsafe extern "C" fn my_malloc(n: usize) -> *mut c_void {
    malloc(n)
}
unsafe extern "C" fn my_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    realloc(p, n)
}
unsafe extern "C" fn my_free(p: *mut c_void) {
    free(p)
}

#[test]
fn rows303_307_309_allocator_hooks_and_realloc() {
    // EXCLUSIVE: json_set_alloc_funcs* mutates library globals.
    let _g = exclusive_alloc();

    diff("rows303_307_309/alloc funcs + jsonp_realloc", |lib: &Library| unsafe {
        let get2: Symbol<FnGetAlloc2> = sym(lib, "json_get_alloc_funcs2");
        let get1: Symbol<FnGetAlloc> = sym(lib, "json_get_alloc_funcs");
        let set1: Symbol<FnSetAlloc> = sym(lib, "json_set_alloc_funcs");
        let set2: Symbol<FnSetAlloc2> = sym(lib, "json_set_alloc_funcs2");
        let jrealloc: Symbol<FnJRealloc> = sym(lib, "jsonp_realloc");
        let jmalloc: Symbol<FnJMalloc> = sym(lib, "jsonp_malloc");
        let jfree: Symbol<FnJFree> = sym(lib, "jsonp_free");

        // ---- save the originals so this test restores global state
        let mut om: Option<JsonMalloc> = None;
        let mut or_: Option<JsonRealloc> = None;
        let mut of: Option<JsonFree> = None;
        get2(&mut om, &mut or_, &mut of);
        let defaults_present = (om.is_some(), or_.is_some(), of.is_some());

        // ---- row 309: json_get_alloc_funcs agrees with json_get_alloc_funcs2
        let mut m1: Option<JsonMalloc> = None;
        let mut f1: Option<JsonFree> = None;
        get1(&mut m1, &mut f1);
        let one_matches_two = (m1 == om, f1 == of);

        // ---- row 309: each out-param individually NULL leaves ours untouched
        let mut probe_m: Option<JsonMalloc> = Some(my_malloc);
        let mut probe_r: Option<JsonRealloc> = Some(my_realloc);
        let mut probe_f: Option<JsonFree> = Some(my_free);
        get2(std::ptr::null_mut(), &mut probe_r, &mut probe_f);
        let skip_m = (probe_m == Some(my_malloc), probe_r == or_, probe_f == of);
        probe_m = Some(my_malloc);
        probe_r = Some(my_realloc);
        probe_f = Some(my_free);
        get2(&mut probe_m, std::ptr::null_mut(), &mut probe_f);
        let skip_r = (probe_m == om, probe_r == Some(my_realloc), probe_f == of);
        probe_m = Some(my_malloc);
        probe_r = Some(my_realloc);
        probe_f = Some(my_free);
        get2(&mut probe_m, &mut probe_r, std::ptr::null_mut());
        let skip_f = (probe_m == om, probe_r == or_, probe_f == Some(my_free));
        // all three NULL is a legal no-op
        get2(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        get1(std::ptr::null_mut(), std::ptr::null_mut());
        let mut only_m: Option<JsonMalloc> = None;
        get1(&mut only_m, std::ptr::null_mut());
        let mut only_f: Option<JsonFree> = None;
        get1(std::ptr::null_mut(), &mut only_f);
        let get1_partial = (only_m == om, only_f == of);

        // ---- row 303 (hook mode): do_realloc is the libc realloc by default
        let hook_mode = {
            let p = jmalloc(16) as *mut u8;
            std::ptr::write_bytes(p, 0x11, 16);
            let q = jrealloc(p as *mut c_void, 16, 64) as *mut u8;
            let grown = (
                q.is_null(),
                std::slice::from_raw_parts(q, 16).to_vec(),
            );
            let s = jrealloc(q as *mut c_void, 64, 8) as *mut u8;
            let shrunk = (s.is_null(), std::slice::from_raw_parts(s, 8).to_vec());
            // newSize 0 in hook mode: glibc realloc frees and returns NULL
            let z = jrealloc(s as *mut c_void, 8, 0);
            let fresh = jrealloc(std::ptr::null_mut(), 0, 32);
            let fresh_null = fresh.is_null();
            jfree(fresh);
            (grown, shrunk, z.is_null(), fresh_null)
        };

        // ---- row 308: custom malloc + realloc + free
        set2(Some(my_malloc), Some(my_realloc), Some(my_free));
        let mut cm: Option<JsonMalloc> = None;
        let mut cr: Option<JsonRealloc> = None;
        let mut cf: Option<JsonFree> = None;
        get2(&mut cm, &mut cr, &mut cf);
        let set2_roundtrip = (
            cm == Some(my_malloc),
            cr == Some(my_realloc),
            cf == Some(my_free),
        );
        // the library really uses them
        let with_set2 = {
            let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
            let oset: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
            let o = obj();
            for i in 0..12 {
                oset(o, cs(&format!("k{}", i)).as_ptr(), n_int(lib, i));
            }
            let d = dumps_to_string(lib, o, JSON_SORT_KEYS);
            decref(lib, o);
            d
        };

        // ---- row 307: json_set_alloc_funcs forces do_realloc to NULL, so every
        // jsonp_realloc goes through the malloc+memcpy+free emulation.
        set1(Some(my_malloc), Some(my_free));
        let mut em: Option<JsonMalloc> = None;
        let mut er: Option<JsonRealloc> = None;
        let mut ef: Option<JsonFree> = None;
        get2(&mut em, &mut er, &mut ef);
        let set1_roundtrip = (em == Some(my_malloc), er.is_none(), ef == Some(my_free));

        let emulation = {
            let p = jmalloc(16) as *mut u8;
            std::ptr::write_bytes(p, 0x22, 16);
            let q = jrealloc(p as *mut c_void, 16, 64) as *mut u8;
            let grown = (q.is_null(), std::slice::from_raw_parts(q, 16).to_vec());
            let s = jrealloc(q as *mut c_void, 64, 8) as *mut u8;
            let shrunk = (s.is_null(), std::slice::from_raw_parts(s, 8).to_vec());
            // newSize 0 in emulation mode: frees ptr, returns NULL
            let z = jrealloc(s as *mut c_void, 8, 0).is_null();
            let z_null = jrealloc(std::ptr::null_mut(), 0, 0).is_null();
            // newSize != 0 with ptr == NULL: plain malloc, no memcpy
            let fresh = jrealloc(std::ptr::null_mut(), 0, 24);
            let fresh_null = fresh.is_null();
            jfree(fresh);
            (grown, shrunk, z, z_null, fresh_null)
        };
        // ... and the whole library still works on the emulation path
        let with_set1 = {
            let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
            let app: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
            let a = arr();
            for i in 0..40 {
                app(a, n_int(lib, i));
            }
            let d = dumps_to_string(lib, a, JSON_COMPACT);
            decref(lib, a);
            d
        };

        // ---- restore the original global allocator trio
        set2(om, or_, of);
        let mut rm: Option<JsonMalloc> = None;
        let mut rr: Option<JsonRealloc> = None;
        let mut rf: Option<JsonFree> = None;
        get2(&mut rm, &mut rr, &mut rf);
        let restored = (rm == om, rr == or_, rf == of);

        (
            (defaults_present, one_matches_two),
            (skip_m, skip_r, skip_f, get1_partial),
            hook_mode,
            (set2_roundtrip, with_set2),
            (set1_roundtrip, emulation, with_set1),
            restored,
        )
    });
}

// ================================================================ row 304

#[test]
fn row304_jsonp_loop_check() {
    let _g = shared_alloc();

    diff("row304/jsonp_loop_check", |lib: &Library| unsafe {
        let init: Symbol<FnHtInit> = sym(lib, "hashtable_init");
        let close: Symbol<FnHtVoid> = sym(lib, "hashtable_close");
        let del: Symbol<FnHtDel> = sym(lib, "hashtable_del");
        let lc: Symbol<FnLoopCheck> = sym(lib, "jsonp_loop_check");
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");

        let mut slab = ht_slab();
        let ht: *mut hashtable_t = slab.ptr();
        assert_eq!(init(ht), 0);

        let a = obj();
        let b = obj();

        // Key buffer of exactly LOOP_KEY_LEN, prefilled so we can prove the
        // library only writes the "%p" text plus a NUL.
        let mut key = [b'Z'; LOOP_KEY_LEN];
        let mut key_len: usize = 0xDEAD_BEEF;
        let r1 = lc(ht, a, key.as_mut_ptr() as *mut c_char, LOOP_KEY_LEN, &mut key_len);
        // The pointer text itself must differ between the two libraries, so only
        // its SHAPE is compared.
        let strlen1 = key.iter().position(|&c| c == 0).unwrap_or(LOOP_KEY_LEN);
        let shape1 = (
            key_len,
            strlen1,
            key[0] as char == '0' && key[1] as char == 'x',
            key[2..strlen1].iter().all(|c| (*c as char).is_ascii_hexdigit()),
            key[strlen1 + 1..].iter().all(|&c| c == b'Z'),
            (*ht).size,
        );

        // second visit to the same json_t -> -1, table unchanged
        let mut key2 = [b'Z'; LOOP_KEY_LEN];
        let mut key_len2: usize = 0;
        let r2 = lc(ht, a, key2.as_mut_ptr() as *mut c_char, LOOP_KEY_LEN, &mut key_len2);
        let same_key = key2 == key;

        // a different json_t is a first visit again
        let mut key3 = [b'Z'; LOOP_KEY_LEN];
        let mut key_len3: usize = 0;
        let r3 = lc(ht, b, key3.as_mut_ptr() as *mut c_char, LOOP_KEY_LEN, &mut key_len3);
        let distinct = key3 != key;

        // key_len_out == NULL is allowed
        let mut key4 = [b'Z'; LOOP_KEY_LEN];
        let r4 = lc(ht, b, key4.as_mut_ptr() as *mut c_char, LOOP_KEY_LEN, std::ptr::null_mut());

        // after deleting the recorded key, `a` is a first visit once more
        del(ht, key.as_ptr() as *const c_char, key_len);
        let mut key5 = [b'Z'; LOOP_KEY_LEN];
        let mut key_len5: usize = 0;
        let r5 = lc(ht, a, key5.as_mut_ptr() as *mut c_char, LOOP_KEY_LEN, &mut key_len5);

        // truncating key_size: snprintf still reports the full length, and the
        // bytes past `key_size` in OUR buffer stay untouched.
        let mut key6 = [b'Z'; LOOP_KEY_LEN];
        let mut key_len6: usize = 0;
        let r6 = lc(ht, b, key6.as_mut_ptr() as *mut c_char, 8, &mut key_len6);
        let trunc = (
            key_len6,
            key6[7] == 0,
            key6[8..].iter().all(|&c| c == b'Z'),
            key6[0] as char == '0' && key6[1] as char == 'x',
        );

        let final_snap = ((*ht).size, (*ht).order);
        close(ht);
        decref(lib, a);
        decref(lib, b);

        (
            (r1, shape1),
            (r2, key_len2, same_key),
            (r3, key_len3, distinct),
            r4,
            (r5, key_len5),
            (r6, trunc),
            final_snap,
            LOOP_KEY_LEN,
        )
    });
}

// ================================================================ rows 305-306

#[test]
fn rows305_306_object_seed() {
    let _g = shared_alloc();

    diff("rows305_306/json_object_seed + hashtable_seed", |lib: &Library| unsafe {
        let seed: Symbol<FnSeed> = sym(lib, "json_object_seed");
        let hs: Symbol<*mut u32> = sym(lib, "hashtable_seed");
        // The harness already called json_object_seed(FIXED_SEED) at dlopen time,
        // so the "read as 0 before seeding" half of row 306 is not observable
        // from here; what IS observable is that the seed is now the fixed value.
        let before = **hs;
        // row 305: a second seeding is ignored once hashtable_seed != 0
        seed(0x1234_5678);
        let after_explicit = **hs;
        seed(0);
        let after_zero = **hs;
        (
            before,
            after_explicit,
            after_zero,
            before == FIXED_SEED as u32,
            before != 0,
        )
    });
}

// ================================================================ rows 310-311

#[test]
fn rows310_311_version() {
    let _g = shared_alloc();

    diff("row310/jansson_version_str", |lib: &Library| unsafe {
        let f: Symbol<FnVersionStr> = sym(lib, "jansson_version_str");
        let s = cstr_to_string(f());
        assert_eq!(s, "2.15.0", "jansson_version_str");
        (s, f().is_null())
    });

    diff("row311/jansson_version_cmp", |lib: &Library| unsafe {
        let f: Symbol<FnVersionCmp> = sym(lib, "jansson_version_cmp");
        // 2.15.0 exactly
        assert_eq!(f(2, 15, 0), 0, "cmp(2,15,0)");
        assert!(f(1, 15, 0) > 0 && f(3, 15, 0) < 0, "major");
        assert!(f(2, 14, 0) > 0 && f(2, 16, 0) < 0, "minor");
        assert!(f(2, 15, 1) < 0 && f(2, 15, -1) > 0, "micro");
        let mut out = Vec::new();
        for major in [-1i32, 0, 1, 2, 3, 100] {
            for minor in [-1i32, 0, 14, 15, 16, 100] {
                for micro in [-1i32, 0, 1, 2, 100] {
                    out.push((major, minor, micro, f(major, minor, micro)));
                }
            }
        }
        out.push((2, 15, 0, f(2, 15, 0)));
        out.push((i32::MAX, 0, 0, f(i32::MAX, 0, 0)));
        out.push((2, i32::MAX, 0, f(2, i32::MAX, 0)));
        out.push((2, 15, i32::MAX, f(2, 15, i32::MAX)));
        out
    });
}

// ================================================================ rows 312-313

#[test]
fn rows312_313_error_helpers() {
    let _g = shared_alloc();

    diff("rows312_313/jsonp_error_* and json_error_code", |lib: &Library| unsafe {
        let init: Symbol<FnErrInit> = sym(lib, "jsonp_error_init");
        let set_src: Symbol<FnErrSetSource> = sym(lib, "jsonp_error_set_source");
        let set: Symbol<FnErrSet> = sym(lib, "jsonp_error_set");

        let mut out = Vec::new();

        // error == NULL is a no-op for all of them
        init(std::ptr::null_mut(), cs("src").as_ptr());
        set_src(std::ptr::null_mut(), cs("src").as_ptr());
        set(
            std::ptr::null_mut(),
            1,
            2,
            3,
            JSON_ERROR_INVALID_SYNTAX,
            cs("%s").as_ptr(),
            cs("x").as_ptr(),
        );

        // init with a source, then with source == NULL
        let mut e = json_error_t::new();
        init(&mut e, cs("myfile.json").as_ptr());
        out.push(("init-src", e.snapshot()));
        init(&mut e, std::ptr::null());
        out.push(("init-nullsrc", e.snapshot()));

        // set_source with an exactly-79, exactly-80 and 200 char path
        for n in [1usize, 78, 79, 80, 81, 200] {
            let mut e = json_error_t::new();
            init(&mut e, std::ptr::null());
            let path: String = (0..n).map(|i| (b'a' + (i % 26) as u8) as char).collect();
            set_src(&mut e, cs(&path).as_ptr());
            out.push(("set_source", e.snapshot()));
        }
        // source == NULL is ignored by set_source
        let mut e = json_error_t::new();
        init(&mut e, cs("keepme").as_ptr());
        set_src(&mut e, std::ptr::null());
        out.push(("set_source-null", e.snapshot()));

        // row 313: the code is smuggled into text[JSON_ERROR_TEXT_LENGTH-1]
        for code in [
            JSON_ERROR_UNKNOWN,
            JSON_ERROR_INVALID_UTF8,
            JSON_ERROR_NULL_BYTE_IN_KEY,
            JSON_ERROR_INDEX_OUT_OF_RANGE,
        ] {
            let mut e = json_error_t::new();
            init(&mut e, cs("s").as_ptr());
            set(&mut e, 11, 22, 33usize, code, cs("msg %s %d").as_ptr(), cs("abc").as_ptr(), 7);
            let first = e.snapshot();
            // row 312: a second set on an already-set error is IGNORED
            set(
                &mut e,
                99,
                98,
                97usize,
                JSON_ERROR_STACK_OVERFLOW,
                cs("second").as_ptr(),
            );
            out.push(("set", first));
            out.push(("set-again-ignored", e.snapshot()));
        }

        // a message longer than JSON_ERROR_TEXT_LENGTH is truncated to 158 bytes
        let mut e = json_error_t::new();
        init(&mut e, std::ptr::null());
        let long: String = (0..400).map(|i| (b'A' + (i % 26) as u8) as char).collect();
        set(
            &mut e,
            1,
            1,
            1usize,
            JSON_ERROR_INVALID_FORMAT,
            cs("%s").as_ptr(),
            cs(&long).as_ptr(),
        );
        out.push(("set-truncated", e.snapshot()));
        let tail = (e.text[JSON_ERROR_TEXT_LENGTH - 2], e.code(), e.text_str().len());
        (out, tail)
    });
}
