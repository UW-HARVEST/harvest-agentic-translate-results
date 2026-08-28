//! Rust translation of `c_src/src/lib.c` (spritebatch merge sort).
//!
//! Behaviour is preserved exactly, including the dead second comparison in
//! `spritebatch_internal_sprite_less_than_or_equal` (the first `<=` test makes
//! the `==` test unreachable). That quirk is part of the original code and is
//! intentionally *not* fixed here.

use std::ffi::c_int;

/// Mirrors:
/// ```c
/// typedef struct spritebatch_sprite_t {
///     unsigned long long texture_id;
///     int sort_bits;
/// } spritebatch_sprite_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spritebatch_sprite_t {
    pub texture_id: u64,
    pub sort_bits: c_int,
}

/// `static int spritebatch_internal_sprite_less_than_or_equal(...)`
///
/// The second `if` is unreachable in the C original; kept verbatim.
#[inline]
fn sprite_less_than_or_equal(a: &spritebatch_sprite_t, b: &spritebatch_sprite_t) -> bool {
    if a.sort_bits <= b.sort_bits {
        return true;
    }
    if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id {
        return true;
    }
    false
}

/// `static void spritebatch_internal_merge_sort_iteration(a, lo, split, hi, b)`
///
/// Reads from `src` (the C `a`) and writes into `dst` (the C `b`).
fn merge_sort_iteration(
    src: &[spritebatch_sprite_t],
    lo: c_int,
    split: c_int,
    hi: c_int,
    dst: &mut [spritebatch_sprite_t],
) {
    let mut i = lo;
    let mut j = split;
    let mut k = lo;
    while k < hi {
        if i < split && (j >= hi || sprite_less_than_or_equal(&src[i as usize], &src[j as usize])) {
            dst[k as usize] = src[i as usize];
            i += 1;
        } else {
            dst[k as usize] = src[j as usize];
            j += 1;
        }
        k += 1;
    }
}

/// `static void spritebatch_internal_merge_sort_recurse(b, lo, hi, a)`
///
/// Note how the two buffers swap roles on every level of recursion, exactly as
/// in the C original.
fn merge_sort_recurse(
    b: &mut [spritebatch_sprite_t],
    lo: c_int,
    hi: c_int,
    a: &mut [spritebatch_sprite_t],
) {
    if hi - lo <= 1 {
        return;
    }
    let split = (lo + hi) / 2;
    merge_sort_recurse(a, lo, split, b);
    merge_sort_recurse(a, split, hi, b);
    merge_sort_iteration(&*b, lo, split, hi, a);
}

/// `void merge_sort(spritebatch_sprite_t *a, spritebatch_sprite_t *b, int size);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    if a.is_null() || b.is_null() || size <= 0 {
        // size == 0 makes the C body a no-op; guard the degenerate cases
        // instead of constructing invalid slices.
        return;
    }

    let len = size as usize;
    let a = unsafe { std::slice::from_raw_parts_mut(a, len) };
    let b = unsafe { std::slice::from_raw_parts_mut(b, len) };

    // memcpy(b, a, sizeof(spritebatch_sprite_t) * size);
    b.copy_from_slice(a);

    merge_sort_recurse(b, 0, size, a);
}
