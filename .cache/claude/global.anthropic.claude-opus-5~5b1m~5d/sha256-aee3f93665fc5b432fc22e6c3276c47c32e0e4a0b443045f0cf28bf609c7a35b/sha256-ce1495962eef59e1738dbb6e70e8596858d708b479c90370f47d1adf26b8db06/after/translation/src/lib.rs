//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared object):
//!   * `tflac_size_memory`  (defined in `src/lib.c`, not declared in the public header)
//!   * `flac_validate`      (declared in `include/lib.h`)
//!
//! Behaviour is reproduced exactly, including the original ordering of every
//! validation check and every in-place mutation of the `tflac` struct.

#![allow(non_camel_case_types)]

use core::ffi::c_int;

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;
/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// `struct tflac` from `include/lib.h`.
///
/// Verified against the C layout: size = 28, align = 4, with field offsets
/// 0, 4, 8, 12, 16, 17, 18, 19, 20 and 24 (three bytes of tail padding after
/// `partition_order` before `cur_blocksize`).
#[repr(C)]
pub struct tflac {
    pub blocksize: tflac_u32,
    pub samplerate: tflac_u32,
    pub channels: tflac_u32,
    pub bitdepth: tflac_u32,
    pub channel_mode: tflac_u8,
    pub max_rice_value: tflac_u8,
    pub min_partition_order: tflac_u8,
    pub max_partition_order: tflac_u8,
    pub partition_order: tflac_u8,
    pub cur_blocksize: tflac_u32,
}

/// `enum TFLAC_CHANNEL_MODE` from `src/lib.c`.
pub const TFLAC_CHANNEL_INDEPENDENT: tflac_u8 = 0;
pub const TFLAC_CHANNEL_LEFT_SIDE: tflac_u8 = 1;
pub const TFLAC_CHANNEL_SIDE_RIGHT: tflac_u8 = 2;
pub const TFLAC_CHANNEL_MID_SIDE: tflac_u8 = 3;
pub const TFLAC_CHANNEL_MODE_COUNT: tflac_u8 = 4;

/// ```c
/// tflac_u32 tflac_size_memory(tflac_u32 blocksize) {
///     return (tflac_u32)15U + (5U * ((15U + (blocksize * 4U)) & 0xFFFFFFF0U));
/// }
/// ```
///
/// All arithmetic is `unsigned int` arithmetic in C, i.e. it wraps modulo 2^32.
#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory(blocksize: tflac_u32) -> tflac_u32 {
    15u32.wrapping_add(5u32.wrapping_mul(
        15u32.wrapping_add(blocksize.wrapping_mul(4)) & 0xFFFF_FFF0,
    ))
}

/// ```c
/// int flac_validate(tflac *t);
/// ```
///
/// Returns `0` on success and `-1` on the first failed check. On success the
/// struct is updated in place (`channel_mode`, `max_rice_value`,
/// `partition_order` and `cur_blocksize`). Note that the in-place rewrites of
/// `channel_mode` and `max_rice_value` happen *before* the later checks, so a
/// rejected call can still leave those two fields modified — exactly as in C.
///
/// # Safety
/// `t` must be a valid, aligned, mutable pointer to a `tflac`. The C original
/// dereferences it unconditionally, so a null pointer is undefined behaviour
/// there too; the fields are therefore touched through the raw pointer (never
/// through a `&mut` reference) so that an invalid `t` faults exactly the way
/// the C does instead of tripping a Rust-side null-pointer assertion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flac_validate(t: *mut tflac) -> c_int {
    macro_rules! get {
        ($field:ident) => {
            core::ptr::read(core::ptr::addr_of!((*t).$field))
        };
    }
    macro_rules! set {
        ($field:ident, $value:expr) => {
            core::ptr::write(core::ptr::addr_of_mut!((*t).$field), $value)
        };
    }

    if get!(blocksize) < 16 {
        return -1;
    }
    if get!(blocksize) > 65535 {
        return -1;
    }
    if get!(samplerate) == 0 {
        return -1;
    }
    if get!(samplerate) > 655350 {
        return -1;
    }
    if get!(channels) == 0 {
        return -1;
    }
    if get!(channels) > 8 {
        return -1;
    }
    if get!(bitdepth) == 0 {
        return -1;
    }
    if get!(bitdepth) > 32 {
        return -1;
    }

    if get!(channel_mode) != TFLAC_CHANNEL_INDEPENDENT
        && (get!(channels) != 2 || get!(bitdepth) == 32)
    {
        set!(channel_mode, TFLAC_CHANNEL_INDEPENDENT);
    }

    if get!(max_rice_value) == 0 {
        if get!(bitdepth) <= 16 {
            set!(max_rice_value, 14);
        } else {
            set!(max_rice_value, 30);
        }
    } else if get!(max_rice_value) > 30 {
        return -1;
    }

    if get!(max_partition_order) > 15 {
        return -1;
    }
    if get!(min_partition_order) > get!(max_partition_order) {
        return -1;
    }

    set!(partition_order, get!(min_partition_order));
    // C: while ((t->blocksize % (1 << (t->partition_order + 1)) == 0) &&
    //           t->partition_order < t->max_partition_order)
    //
    // `1 << (partition_order + 1)` is `int` arithmetic that is then converted to
    // `unsigned int` for the `%`. Because the checks above guarantee
    // partition_order <= max_partition_order <= 15, the shift amount is at most
    // 16 and the divisor is never zero.
    while get!(blocksize) % 1u32.wrapping_shl(get!(partition_order) as u32 + 1) == 0
        && get!(partition_order) < get!(max_partition_order)
    {
        set!(partition_order, get!(partition_order).wrapping_add(1));
    }

    set!(cur_blocksize, get!(blocksize));
    0
}
