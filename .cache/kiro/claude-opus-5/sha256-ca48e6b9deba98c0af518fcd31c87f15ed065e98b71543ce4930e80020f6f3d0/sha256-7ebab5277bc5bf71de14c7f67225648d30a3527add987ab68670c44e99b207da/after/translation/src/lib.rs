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
///
/// The C struct is 16 bytes: 8 for `texture_id`, 4 for `sort_bits`, and 4 bytes
/// of tail padding to reach the 8-byte alignment. That padding is modelled as
/// an explicit `_pad` field rather than left implicit, because the C code moves
/// elements with whole-struct assignment (`b[k] = a[i]`), which every tested
/// compiler configuration lowers to a full 16-byte copy — padding included.
/// With implicit padding, Rust's struct assignment is free to move only the two
/// named fields, which leaves stale padding bytes behind and is observable by
/// any caller that compares the buffers byte-for-byte.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spritebatch_sprite_t {
    pub texture_id: u64,
    pub sort_bits: c_int,
    /// Tail padding of the C struct; carried so element copies are byte-exact.
    pub _pad: u32,
}

const _: () = {
    assert!(size_of::<spritebatch_sprite_t>() == 16);
    assert!(align_of::<spritebatch_sprite_t>() == 8);
};

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
            i = i.wrapping_add(1);
        } else {
            dst[k as usize] = src[j as usize];
            j = j.wrapping_add(1);
        }
        k = k.wrapping_add(1);
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
    if hi.wrapping_sub(lo) <= 1 {
        return;
    }
    // `wrapping_add` matches the C `(lo + hi) / 2` on overflow instead of
    // panicking in debug builds. Unreachable for any allocatable `size`.
    let split = lo.wrapping_add(hi) / 2;
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
