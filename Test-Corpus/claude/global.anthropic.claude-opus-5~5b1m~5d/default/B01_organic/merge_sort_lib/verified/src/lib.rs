//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exposing a
//! single public symbol, `merge_sort`, plus three `static` (internal) helpers.
//! Behaviour — including the quirky comparison predicate, which the C code gets
//! "wrong" (the second `if` is unreachable in practice because the first `if`
//! already covers `a->sort_bits == b->sort_bits`) — is reproduced exactly.

#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_ulonglong, c_void};

// `c_src/src/lib.c` includes <string.h> and calls `memcpy` directly. Binding the
// libc symbol rather than using `core::ptr::copy*` keeps the translation exact:
// the same glibc routine sees the same byte count, so out-of-domain inputs (null
// pointers, the ~2^64 byte count produced by a negative `size`, and a caller
// that aliases `a` with `b`) reproduce the C's behaviour bit for bit in EVERY
// build profile. `core::ptr::copy_nonoverlapping` would instead trip its own
// debug-only "non-null / non-overlapping" preconditions and abort where the C
// segfaults or silently succeeds.
extern "C" {
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

/// Byte-exact equivalent of the C struct assignment `*dst = *src`.
///
/// `spritebatch_sprite_t` has 4 bytes of trailing padding (offsets 12..16). A
/// plain Rust `*dst = *src` is NOT equivalent to the C assignment: rustc/LLVM
/// treat padding as `undef` and only copy the two real fields, so the
/// destination keeps whatever padding it already had. GCC, by contrast, compiles
/// `b[k] = a[i]` into two 8-byte `mov`s that span all 16 bytes, so the padding
/// travels with the element and is observable in the output buffers.
///
/// The two 8-byte loads are both performed *before* either store, mirroring
/// GCC's instruction sequence exactly. That also makes the fully-aliased case
/// (`dst == src`, reachable via `merge_sort(p, p, n)`) behave identically
/// instead of relying on `copy_nonoverlapping`, whose no-overlap precondition
/// such a call would violate.
#[inline]
unsafe fn sprite_assign(dst: *mut spritebatch_sprite_t, src: *const spritebatch_sprite_t) {
    // `MaybeUninit` is used because the second word spans the 4 padding bytes,
    // which a caller is free to leave uninitialised; the C reads them regardless,
    // so they must be forwarded without asserting that they are initialised.
    use core::mem::MaybeUninit;
    // mov (%rax),%rax  /  mov 0x8(%rax),%rdx
    let w0 = (src as *const MaybeUninit<u64>).read();
    let w1 = (src as *const MaybeUninit<u64>).add(1).read();
    // mov %rax,(%rcx)  /  mov %rdx,0x8(%rcx)
    (dst as *mut MaybeUninit<u64>).write(w0);
    (dst as *mut MaybeUninit<u64>).add(1).write(w1);
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
            // b[k] = a[i];
            sprite_assign(b.offset(k as isize), a.offset(i as isize));
            i = i.wrapping_add(1);
        } else {
            // b[k] = a[j];
            sprite_assign(b.offset(k as isize), a.offset(j as isize));
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
    // `sizeof(T)` has type size_t, so `size` is converted to size_t before the
    // multiply: gcc emits `cltq; shl $0x4` (sign-extend the int to 64 bits, then
    // multiply by 16). `size as isize as usize` reproduces that sign extension,
    // so a negative `size` yields the same enormous byte count the C produces
    // (rustc emits the identical `movslq; shl $0x4`). The call is made
    // unconditionally, exactly as in the C.
    let bytes: usize = core::mem::size_of::<spritebatch_sprite_t>()
        .wrapping_mul(size as isize as usize);
    memcpy(b as *mut c_void, a as *const c_void, bytes);
    spritebatch_internal_merge_sort_recurse(b, 0, size, a);
}
