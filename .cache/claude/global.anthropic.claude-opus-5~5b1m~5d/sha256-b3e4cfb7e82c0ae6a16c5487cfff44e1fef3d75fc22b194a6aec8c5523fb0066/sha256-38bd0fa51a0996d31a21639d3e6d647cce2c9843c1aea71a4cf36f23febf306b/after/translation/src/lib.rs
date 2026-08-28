//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exposing a
//! single public symbol, `merge_sort`, plus three `static` (internal) helpers.
//! Behaviour — including the quirky comparison predicate, which the C code gets
//! "wrong" (the second `if` is unreachable in practice because the first `if`
//! already covers `a->sort_bits == b->sort_bits`) — is reproduced exactly.

#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_ulonglong};

/// ```c
/// typedef struct spritebatch_sprite_t {
///     unsigned long long texture_id;
///     int sort_bits;
/// } spritebatch_sprite_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spritebatch_sprite_t {
    pub texture_id: c_ulonglong,
    pub sort_bits: c_int,
}

/// ```c
/// static int spritebatch_internal_sprite_less_than_or_equal(
///     spritebatch_sprite_t *a, spritebatch_sprite_t *b);
/// ```
///
/// Kept verbatim: the redundant second test is preserved even though the first
/// test subsumes it.
unsafe fn spritebatch_internal_sprite_less_than_or_equal(
    a: *const spritebatch_sprite_t,
    b: *const spritebatch_sprite_t,
) -> c_int {
    if (*a).sort_bits <= (*b).sort_bits {
        return 1;
    }
    if (*a).sort_bits == (*b).sort_bits && (*a).texture_id <= (*b).texture_id {
        return 1;
    }
    0
}

/// ```c
/// static void spritebatch_internal_merge_sort_iteration(
///     spritebatch_sprite_t *a, int lo, int split, int hi, spritebatch_sprite_t *b);
/// ```
unsafe fn spritebatch_internal_merge_sort_iteration(
    a: *const spritebatch_sprite_t,
    lo: c_int,
    split: c_int,
    hi: c_int,
    b: *mut spritebatch_sprite_t,
) {
    let mut i: c_int = lo;
    let mut j: c_int = split;
    let mut k: c_int = lo;
    while k < hi {
        if i < split
            && (j >= hi
                || spritebatch_internal_sprite_less_than_or_equal(
                    a.offset(i as isize),
                    a.offset(j as isize),
                ) != 0)
        {
            *b.offset(k as isize) = *a.offset(i as isize);
            i = i.wrapping_add(1);
        } else {
            *b.offset(k as isize) = *a.offset(j as isize);
            j = j.wrapping_add(1);
        }
        k = k.wrapping_add(1);
    }
}

/// ```c
/// static void spritebatch_internal_merge_sort_recurse(
///     spritebatch_sprite_t *b, int lo, int hi, spritebatch_sprite_t *a);
/// ```
///
/// Note the deliberate swap of the buffer arguments in the recursive calls,
/// exactly as in the C source.
unsafe fn spritebatch_internal_merge_sort_recurse(
    b: *mut spritebatch_sprite_t,
    lo: c_int,
    hi: c_int,
    a: *mut spritebatch_sprite_t,
) {
    if hi.wrapping_sub(lo) <= 1 {
        return;
    }
    let split: c_int = lo.wrapping_add(hi) / 2;
    spritebatch_internal_merge_sort_recurse(a, lo, split, b);
    spritebatch_internal_merge_sort_recurse(a, split, hi, b);
    spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);
}

/// ```c
/// void merge_sort(spritebatch_sprite_t *a, spritebatch_sprite_t *b, int size);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    // memcpy(b, a, sizeof(spritebatch_sprite_t) * size);
    //
    // The C code converts `size` (int) to size_t before multiplying, so a
    // negative size becomes an enormous byte count; the wrapping arithmetic
    // below reproduces that computation bit for bit.
    let bytes: usize = core::mem::size_of::<spritebatch_sprite_t>()
        .wrapping_mul(size as isize as usize);
    if bytes != 0 {
        std::ptr::copy_nonoverlapping(a as *const u8, b as *mut u8, bytes);
    }
    spritebatch_internal_merge_sort_recurse(b, 0, size, a);
}
