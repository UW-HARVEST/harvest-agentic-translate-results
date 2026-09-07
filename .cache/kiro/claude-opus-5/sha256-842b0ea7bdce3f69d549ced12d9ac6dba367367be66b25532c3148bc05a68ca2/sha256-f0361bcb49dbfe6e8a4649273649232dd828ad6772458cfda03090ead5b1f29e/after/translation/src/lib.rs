//! Rust translation of the C library in `c_src/`.
//!
//! The C library (`c_src/src/lib.c`, `c_src/include/lib.h`) exports exactly one
//! public symbol: `merge_sort`. Everything else in the C translation unit is
//! `static` and therefore has internal linkage (verified with
//! `nm -D --defined-only` on the C `.so`, which lists only `T merge_sort`).
//!
//! The translation is intentionally literal: quirks of the original are
//! preserved, not fixed. In particular:
//!
//! * `spritebatch_internal_sprite_less_than_or_equal` returns early on
//!   `a->sort_bits <= b->sort_bits`, which makes its second `if` unreachable
//!   dead code (the `texture_id` tiebreak never runs). Reproduced as-is.
//! * `merge_sort` computes the `memcpy` length as
//!   `sizeof(spritebatch_sprite_t) * size` where `size` is an `int` implicitly
//!   converted to `size_t`; a negative `size` therefore yields a huge unsigned
//!   byte count. Reproduced with wrapping arithmetic.
//! * `int` arithmetic (`hi - lo`, `lo + hi`) uses wrapping semantics to match
//!   what the C compiler emits on this target.

// The crate name is derived from the C library's name, which is not snake case.
#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::c_int;

/// Mirror of the C `spritebatch_sprite_t`.
///
/// ```c
/// typedef struct spritebatch_sprite_t {
///     unsigned long long texture_id;
///     int sort_bits;
/// } spritebatch_sprite_t;
/// ```
///
/// `#[repr(C)]` gives the same layout as the C struct: size 16, align 8 on the
/// LP64 targets this library is built for (4 bytes of tail padding after
/// `sort_bits`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spritebatch_sprite_t {
    pub texture_id: u64,
    pub sort_bits: c_int,
}

/// Size in bytes of one sprite, i.e. C's `sizeof(spritebatch_sprite_t)`.
const SPRITE_SIZE: usize = core::mem::size_of::<spritebatch_sprite_t>();

/// Translation of `spritebatch_internal_sprite_less_than_or_equal`.
///
/// Kept returning `c_int` (1 / 0) rather than `bool` to stay faithful to the
/// original; the value is only ever used as a truth value.
///
/// # Safety
///
/// `a` and `b` must be valid, aligned, readable pointers to a
/// `spritebatch_sprite_t`.
unsafe fn spritebatch_internal_sprite_less_than_or_equal(
    a: *const spritebatch_sprite_t,
    b: *const spritebatch_sprite_t,
) -> c_int {
    let a = unsafe { &*a };
    let b = unsafe { &*b };

    if a.sort_bits <= b.sort_bits {
        return 1;
    }
    // Dead code in the original as well: the branch above already covers
    // `a->sort_bits == b->sort_bits`. Preserved deliberately.
    if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id {
        return 1;
    }
    0
}

/// Byte-for-byte copy of one sprite, matching the code gcc generates for the C
/// struct assignment `b[k] = a[i]`.
///
/// This deliberately copies all `SPRITE_SIZE` (16) bytes, including the 4 bytes
/// of tail padding after `sort_bits`. A plain Rust `*dst = *src` would only copy
/// the two fields (12 bytes) and leave the destination's padding untouched,
/// whereas gcc emits a 16-byte `movdqu`/`movups` pair that carries the padding
/// across. When callers pass arrays whose padding holds indeterminate values,
/// that distinction is externally visible to anything comparing raw bytes, so it
/// is reproduced here.
///
/// # Safety
///
/// `src` and `dst` must be valid for a 16-byte read / write respectively and
/// must not overlap.
#[inline(always)]
unsafe fn copy_sprite(dst: *mut spritebatch_sprite_t, src: *const spritebatch_sprite_t) {
    unsafe { core::ptr::copy_nonoverlapping(src.cast::<u8>(), dst.cast::<u8>(), SPRITE_SIZE) };
}

/// Translation of `spritebatch_internal_merge_sort_iteration`.
///
/// Merges the two adjacent, already-sorted runs `a[lo..split)` and
/// `a[split..hi)` into `b[lo..hi)`.
///
/// # Safety
///
/// `a` and `b` must be valid for reads / writes respectively over the index
/// range touched by the loop, exactly as required by the C original.
unsafe fn spritebatch_internal_merge_sort_iteration(
    a: *const spritebatch_sprite_t,
    lo: c_int,
    split: c_int,
    hi: c_int,
    b: *mut spritebatch_sprite_t,
) {
    let mut i = lo;
    let mut j = split;

    let mut k = lo;
    while k < hi {
        // `i < split && (j >= hi || leq(a + i, a + j))` — note the C code
        // short-circuits, so `a + j` is only dereferenced when `j < hi`.
        let take_left = i < split
            && (j >= hi
                || unsafe {
                    spritebatch_internal_sprite_less_than_or_equal(
                        a.offset(i as isize),
                        a.offset(j as isize),
                    )
                } != 0);

        if take_left {
            unsafe { copy_sprite(b.offset(k as isize), a.offset(i as isize)) };
            i = i.wrapping_add(1);
        } else {
            unsafe { copy_sprite(b.offset(k as isize), a.offset(j as isize)) };
            j = j.wrapping_add(1);
        }

        k = k.wrapping_add(1);
    }
}

/// Translation of `spritebatch_internal_merge_sort_recurse`.
///
/// Note the deliberate parameter swap on the recursive calls: the C original
/// alternates the roles of the two buffers on each level so that the final
/// merge at this level reads from `b` and writes into `a`.
///
/// # Safety
///
/// `b` and `a` must both be valid for the index range `[lo, hi)`.
unsafe fn spritebatch_internal_merge_sort_recurse(
    b: *mut spritebatch_sprite_t,
    lo: c_int,
    hi: c_int,
    a: *mut spritebatch_sprite_t,
) {
    if hi.wrapping_sub(lo) <= 1 {
        return;
    }

    let split = lo.wrapping_add(hi) / 2;

    unsafe {
        spritebatch_internal_merge_sort_recurse(a, lo, split, b);
        spritebatch_internal_merge_sort_recurse(a, split, hi, b);
        spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);
    }
}

/// Translation of the library's only exported function:
///
/// ```c
/// void merge_sort(spritebatch_sprite_t *a, spritebatch_sprite_t *b, int size);
/// ```
///
/// Sorts `a[0..size)` in place, using `b` as scratch space of the same length.
///
/// # Safety
///
/// Same contract as the C function: `a` and `b` must be valid, non-overlapping
/// arrays of at least `size` `spritebatch_sprite_t` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    // `memcpy(b, a, sizeof(spritebatch_sprite_t) * size);`
    //
    // `size` is an `int` widened to `size_t`, so a negative `size` produces an
    // enormous byte count here just as it does in C. The multiplication is done
    // with wrapping semantics to reproduce that faithfully.
    let bytes = SPRITE_SIZE.wrapping_mul(size as usize);
    if bytes != 0 {
        unsafe { core::ptr::copy_nonoverlapping(a.cast::<u8>(), b.cast::<u8>(), bytes) };
    }

    unsafe { spritebatch_internal_merge_sort_recurse(b, 0, size, a) };
}
