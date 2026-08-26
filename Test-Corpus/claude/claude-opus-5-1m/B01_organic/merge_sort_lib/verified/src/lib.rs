//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` of the C shared library exactly):
//!   * `merge_sort`
//!
//! The C source declares its helpers `static`, so they are not exported; they
//! are translated here as private Rust functions with identical semantics.
//!
//! Behavioural quirks of the original C are reproduced verbatim (they are not
//! "fixed"), most notably `spritebatch_internal_sprite_less_than_or_equal`,
//! whose second `if` is unreachable because the first `if` already covers the
//! `a->sort_bits == b->sort_bits` case.

use core::ffi::c_int;

/// `typedef struct spritebatch_sprite_t { unsigned long long texture_id; int sort_bits; }`
///
/// `#[repr(C)]` gives the same layout as the C struct: size 16, align 8 with
/// 4 bytes of tail padding on the usual LP64 targets.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spritebatch_sprite_t {
    pub texture_id: u64,
    pub sort_bits: c_int,
}

/// ```c
/// static int spritebatch_internal_sprite_less_than_or_equal(
///     spritebatch_sprite_t *a, spritebatch_sprite_t *b) {
///     if (a->sort_bits <= b->sort_bits) return 1;
///     if (a->sort_bits == b->sort_bits && a->texture_id <= b->texture_id) return 1;
///     return 0;
/// }
/// ```
///
/// Note: the second comparison is dead code in the original C (an equal
/// `sort_bits` already returns 1 from the first check). It is preserved as-is so
/// the observable ordering is bit-for-bit identical to the C implementation.
#[inline]
fn spritebatch_internal_sprite_less_than_or_equal(
    a: &spritebatch_sprite_t,
    b: &spritebatch_sprite_t,
) -> c_int {
    if a.sort_bits <= b.sort_bits {
        return 1;
    }
    if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id {
        return 1;
    }
    0
}

/// ```c
/// static void spritebatch_internal_merge_sort_iteration(
///     spritebatch_sprite_t *a, int lo, int split, int hi, spritebatch_sprite_t *b);
/// ```
///
/// Merges the runs `a[lo..split)` and `a[split..hi)` into `b[lo..hi)`.
///
/// # Safety
/// `a` and `b` must be valid for the index ranges touched below, exactly as
/// required by the C original.
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
        // `i < split && (j >= hi || less_than_or_equal(a + i, a + j))`
        let take_i = i < split
            && (j >= hi
                || unsafe {
                    spritebatch_internal_sprite_less_than_or_equal(
                        &*a.offset(i as isize),
                        &*a.offset(j as isize),
                    )
                } != 0);

        // `b[k] = a[i];` — a whole-struct assignment. C compilers copy all
        // `sizeof` bytes, so the 4 tail padding bytes propagate too. Using an
        // explicit element-sized `copy_nonoverlapping` guarantees the same
        // 16-byte copy instead of leaving padding handling up to the optimiser.
        if take_i {
            unsafe { core::ptr::copy_nonoverlapping(a.offset(i as isize), b.offset(k as isize), 1) };
            i = i.wrapping_add(1);
        } else {
            unsafe { core::ptr::copy_nonoverlapping(a.offset(j as isize), b.offset(k as isize), 1) };
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
/// Keeps the C parameter names (and therefore the C's alternating source /
/// scratch buffer roles) so the recursion pattern is identical.
///
/// # Safety
/// Same requirements as the C original.
unsafe fn spritebatch_internal_merge_sort_recurse(
    b: *mut spritebatch_sprite_t,
    lo: c_int,
    hi: c_int,
    a: *mut spritebatch_sprite_t,
) {
    if hi.wrapping_sub(lo) <= 1 {
        return;
    }
    // `int split = (lo + hi) / 2;` — wrapping mirrors what the C compiler emits.
    let split = lo.wrapping_add(hi) / 2;
    unsafe {
        spritebatch_internal_merge_sort_recurse(a, lo, split, b);
        spritebatch_internal_merge_sort_recurse(a, split, hi, b);
        spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);
    }
}

/// ```c
/// void merge_sort(spritebatch_sprite_t *a, spritebatch_sprite_t *b, int size) {
///     memcpy(b, a, sizeof(spritebatch_sprite_t) * size);
///     spritebatch_internal_merge_sort_recurse(b, 0, size, a);
/// }
/// ```
///
/// Sorts `a[0..size)` in place using `b[0..size)` as scratch space.
///
/// # Safety
/// `a` and `b` must each point to at least `size` writable, initialised
/// `spritebatch_sprite_t` elements and must not overlap — the same contract the
/// C function imposes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    // `sizeof(spritebatch_sprite_t) * size`: `size` is converted to `size_t`,
    // so a negative `size` wraps to a huge byte count just like in C.
    let bytes = core::mem::size_of::<spritebatch_sprite_t>().wrapping_mul(size as usize);
    if bytes != 0 {
        unsafe { core::ptr::copy_nonoverlapping(a as *const u8, b as *mut u8, bytes) };
    }

    unsafe { spritebatch_internal_merge_sort_recurse(b, 0, size, a) };
}
