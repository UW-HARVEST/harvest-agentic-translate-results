//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exposing a
//! single public symbol, `merge_sort`, plus three `static` (internal) helpers.
//! Behaviour — including the quirky comparison predicate, which the C code gets
//! "wrong" (the second `if` is unreachable in practice because the first `if`
//! already covers `a->sort_bits == b->sort_bits`) — is reproduced exactly.

#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_ulonglong, c_void};

extern "C" {
    /// `#include <string.h>` — the very same `memcpy` the C translation unit
    /// links against.
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

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
            copy_struct(a.offset(i as isize), b.offset(k as isize));
            i = i.wrapping_add(1);
        } else {
            copy_struct(a.offset(j as isize), b.offset(k as isize));
            j = j.wrapping_add(1);
        }
        k = k.wrapping_add(1);
    }
}

/// `*dst = *src` for `spritebatch_sprite_t`, byte for byte.
///
/// The C compiler implements the struct assignments `b[k] = a[i]` in
/// `spritebatch_internal_merge_sort_iteration` as a full `sizeof(struct)`
/// (16-byte) move — it copies the 4 trailing padding bytes along with the two
/// members. A plain Rust `*dst = *src` on the `#[repr(C)]` struct copies only
/// the two initialised fields and leaves the destination's padding bytes
/// untouched, which is observably different for a caller that inspects the
/// buffers as raw bytes. Copying raw bytes here reproduces the C exactly.
#[inline]
unsafe fn copy_struct(src: *const spritebatch_sprite_t, dst: *mut spritebatch_sprite_t) {
    // Read the full 16 bytes into a temporary first, then store them: this is
    // what gcc emits for the C struct assignment (`mov 0x8(%rax),%rdx; mov
    // (%rax),%rax; mov %rax,(%rcx); mov %rdx,0x8(%rcx)`) and it stays correct
    // when `src == dst` (the aliased `a == b` case). Unaligned accessors are
    // used because the C `mov`s do not require alignment either.
    let tmp: [u8; 16] = core::ptr::read_unaligned(src as *const [u8; 16]);
    core::ptr::write_unaligned(dst as *mut [u8; 16], tmp);
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
    // `sizeof(spritebatch_sprite_t)` has type `size_t`, so `size` (an `int`) is
    // converted to `size_t` *before* the multiplication: a negative `size`
    // becomes an enormous byte count (`(size_t)-1 * 16` mod 2^64), which makes
    // `memcpy` fault. The `as isize as usize` sign-extension plus
    // `wrapping_mul` reproduces that computation bit for bit.
    let bytes: usize = core::mem::size_of::<spritebatch_sprite_t>()
        .wrapping_mul(size as isize as usize);
    //
    // Delegate to libc's `memcpy`, exactly like the C does, instead of to
    // `ptr::copy_nonoverlapping`. This keeps the edge cases identical rather
    // than merely similar:
    //   * `size == 0` -> `memcpy(b, a, 0)`, which touches nothing even for
    //     null pointers;
    //   * `a == b`    -> glibc's `memcpy` still performs the (idempotent) copy
    //     instead of tripping Rust's non-overlapping requirement;
    //   * null / out-of-range pointers with a non-zero count fault in the same
    //     place, with the same signal, as the C.
    memcpy(b as *mut c_void, a as *const c_void, bytes);
    spritebatch_internal_merge_sort_recurse(b, 0, size, a);
}
