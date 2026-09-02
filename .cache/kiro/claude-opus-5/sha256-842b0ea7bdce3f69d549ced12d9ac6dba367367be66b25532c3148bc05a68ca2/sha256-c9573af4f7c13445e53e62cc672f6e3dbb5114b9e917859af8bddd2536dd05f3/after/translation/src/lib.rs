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
//!   byte count. Reproduced with wrapping arithmetic, and the copy is performed
//!   by a direct call to libc `memcpy` so that the byte-level outcome (and the
//!   fatal-signal behaviour for very large lengths) is literally the same code.
//! * `int` arithmetic (`hi - lo`, `lo + hi`) uses wrapping semantics to match
//!   what the C compiler emits on this target.
//!
//! # Faithfulness to the C's memory model
//!
//! The C accesses `spritebatch_sprite_t` through plain `mov` instructions and
//! indexes with plain address arithmetic, so it tolerates inputs that Rust's
//! reference and `copy_nonoverlapping` rules do not: misaligned pointers,
//! aliasing `a` and `b`, and indices outside the caller's allocation (reachable
//! whenever `size` disagrees with the real buffer length). To behave identically
//! on those inputs rather than relying on "it happens to work with
//! optimizations on", this translation:
//!
//! * never forms a Rust reference to a sprite — fields are read with
//!   `read_unaligned`;
//! * copies a sprite as two unaligned `u64` loads followed by two unaligned
//!   `u64` stores, which is exactly what gcc emits for `b[k] = a[i]`
//!   (`mov 0x8(%rax),%rdx; mov (%rax),%rax; mov %rax,(%rcx); mov %rdx,0x8(%rcx)`)
//!   — both loads precede both stores, and there is no non-overlap requirement;
//! * indexes with `wrapping_offset` (gcc's `cltq; shl $4; add`), which has no
//!   in-bounds precondition;
//! * calls libc `memcpy` for the bulk copy, mirroring the C's
//!   `U memcpy@GLIBC_2.14` import.

// The crate name is derived from the C library's name, which is not snake case.
#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_int, c_void};

unsafe extern "C" {
    /// The same `memcpy` the C library imports (`U memcpy@GLIBC_2.14`).
    ///
    /// Called instead of `core::ptr::copy_nonoverlapping` so that
    /// `merge_sort`'s bulk copy is byte-for-byte the same operation as the C's,
    /// including for the degenerate lengths a negative `size` produces and for
    /// callers that pass overlapping buffers.
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

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

const _: () = assert!(SPRITE_SIZE == 16);
const _: () = assert!(core::mem::align_of::<spritebatch_sprite_t>() == 8);

/// Byte offset of `texture_id` within the struct.
const OFF_TEXTURE_ID: usize = 0;
/// Byte offset of `sort_bits` within the struct.
const OFF_SORT_BITS: usize = 8;

/// Read `p->sort_bits` the way the C does (`mov 0x8(%rax),%eax`): a plain
/// 4-byte load with no alignment requirement and without forming a reference.
///
/// # Safety
///
/// `p` must be readable for `SPRITE_SIZE` bytes, exactly as the C requires.
#[inline(always)]
unsafe fn load_sort_bits(p: *const spritebatch_sprite_t) -> c_int {
    unsafe { p.cast::<u8>().add(OFF_SORT_BITS).cast::<c_int>().read_unaligned() }
}

/// Read `p->texture_id` the way the C does (`mov (%rax),%rax`).
///
/// # Safety
///
/// `p` must be readable for `SPRITE_SIZE` bytes, exactly as the C requires.
#[inline(always)]
unsafe fn load_texture_id(p: *const spritebatch_sprite_t) -> u64 {
    unsafe { p.cast::<u8>().add(OFF_TEXTURE_ID).cast::<u64>().read_unaligned() }
}

/// `base + index` with C's address arithmetic (`cltq; shl $4; add`).
///
/// `wrapping_offset` is used rather than `offset` because the C imposes no
/// in-bounds requirement: a caller whose `size` exceeds the real buffer length
/// makes the C compute out-of-range addresses, and the translation must compute
/// the same ones instead of invoking Rust-level UB.
#[inline(always)]
fn elem(base: *const spritebatch_sprite_t, index: c_int) -> *const spritebatch_sprite_t {
    base.wrapping_offset(index as isize)
}

#[inline(always)]
fn elem_mut(base: *mut spritebatch_sprite_t, index: c_int) -> *mut spritebatch_sprite_t {
    base.wrapping_offset(index as isize)
}

/// Translation of `spritebatch_internal_sprite_less_than_or_equal`.
///
/// Kept returning `c_int` (1 / 0) rather than `bool` to stay faithful to the
/// original; the value is only ever used as a truth value.
///
/// # Safety
///
/// `a` and `b` must be readable for `SPRITE_SIZE` bytes. Unlike the previous
/// formulation, they need *not* be aligned and need not be distinct — matching
/// the C, which just issues `mov`s.
unsafe fn spritebatch_internal_sprite_less_than_or_equal(
    a: *const spritebatch_sprite_t,
    b: *const spritebatch_sprite_t,
) -> c_int {
    if unsafe { load_sort_bits(a) } <= unsafe { load_sort_bits(b) } {
        return 1;
    }
    // Dead code in the original as well: the branch above already covers
    // `a->sort_bits == b->sort_bits`. Preserved deliberately — gcc emits it and
    // it is likewise unreachable there (`a > b` implies `a != b`).
    if unsafe { load_sort_bits(a) } == unsafe { load_sort_bits(b) }
        && unsafe { load_texture_id(a) } <= unsafe { load_texture_id(b) }
    {
        return 1;
    }
    0
}

/// Copy of one sprite, matching the code gcc generates for the C struct
/// assignment `b[k] = a[i]`:
///
/// ```text
/// mov 0x8(%rax),%rdx   ; load high 8 bytes
/// mov (%rax),%rax      ; load low 8 bytes
/// mov %rax,(%rcx)      ; store low 8 bytes
/// mov %rdx,0x8(%rcx)   ; store high 8 bytes
/// ```
///
/// All 16 bytes move, including the 4 bytes of tail padding after `sort_bits`
/// (a field-wise Rust assignment would move only 12 and leave the destination's
/// padding stale, which is externally visible to a caller comparing raw bytes).
/// Both loads precede both stores, and unaligned accessors are used, so this is
/// well-defined for misaligned and aliasing pointers just like the C.
///
/// # Safety
///
/// `src` and `dst` must be valid for a 16-byte read / write respectively.
#[inline(always)]
unsafe fn copy_sprite(dst: *mut spritebatch_sprite_t, src: *const spritebatch_sprite_t) {
    unsafe {
        let s = src.cast::<u8>();
        let d = dst.cast::<u8>();
        let hi = s.add(8).cast::<u64>().read_unaligned();
        let lo = s.cast::<u64>().read_unaligned();
        d.cast::<u64>().write_unaligned(lo);
        d.add(8).cast::<u64>().write_unaligned(hi);
    }
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
                || unsafe { spritebatch_internal_sprite_less_than_or_equal(elem(a, i), elem(a, j)) }
                    != 0);

        if take_left {
            unsafe { copy_sprite(elem_mut(b, k), elem(a, i)) };
            i = i.wrapping_add(1);
        } else {
            unsafe { copy_sprite(elem_mut(b, k), elem(a, j)) };
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

    // gcc: `add; mov; shr $0x1f; add; sar $1` — wrapping addition followed by a
    // divide that truncates toward zero, which is what Rust's `/` does too.
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
/// Same contract as the C function: `a` and `b` must be valid arrays of at
/// least `size` `spritebatch_sprite_t` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    // `memcpy(b, a, sizeof(spritebatch_sprite_t) * size);`
    //
    // `size` is an `int` widened to `size_t` (gcc: `cltq; shl $4`), so a
    // negative `size` produces an enormous byte count here just as it does in
    // C. The multiplication wraps, and the call goes to the very same libc
    // `memcpy`, so degenerate lengths behave identically down to the signal.
    let bytes = SPRITE_SIZE.wrapping_mul(size as usize);
    unsafe { memcpy(b.cast::<c_void>(), a.cast::<c_void>(), bytes) };

    unsafe { spritebatch_internal_merge_sort_recurse(b, 0, size, a) };
}
