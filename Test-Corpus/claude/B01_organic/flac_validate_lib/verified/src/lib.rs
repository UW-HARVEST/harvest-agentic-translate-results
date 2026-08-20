//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` of the C shared object):
//!   * `tflac_size_memory`
//!   * `flac_validate`

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Types (include/lib.h)
// ---------------------------------------------------------------------------

/// `typedef uint8_t tflac_u8;`
pub type TflacU8 = u8;
/// `typedef uint32_t tflac_u32;`
pub type TflacU32 = u32;

/// `struct tflac` from `include/lib.h`.
///
/// Layout (verified against the C compiler): size 28, align 4,
/// `channel_mode` at offset 16, `cur_blocksize` at offset 24.
#[repr(C)]
pub struct Tflac {
    pub blocksize: TflacU32,
    pub samplerate: TflacU32,
    pub channels: TflacU32,
    pub bitdepth: TflacU32,
    pub channel_mode: TflacU8,
    pub max_rice_value: TflacU8,
    pub min_partition_order: TflacU8,
    pub max_partition_order: TflacU8,
    pub partition_order: TflacU8,
    pub cur_blocksize: TflacU32,
}

// Compile-time guarantee that the Rust layout matches the C one.
const _: () = {
    assert!(core::mem::size_of::<Tflac>() == 28);
    assert!(core::mem::align_of::<Tflac>() == 4);
    assert!(core::mem::offset_of!(Tflac, channel_mode) == 16);
    assert!(core::mem::offset_of!(Tflac, cur_blocksize) == 24);
};

// ---------------------------------------------------------------------------
// enum TFLAC_CHANNEL_MODE (src/lib.c)
// ---------------------------------------------------------------------------

pub const TFLAC_CHANNEL_INDEPENDENT: TflacU8 = 0;
pub const TFLAC_CHANNEL_LEFT_SIDE: TflacU8 = 1;
pub const TFLAC_CHANNEL_SIDE_RIGHT: TflacU8 = 2;
pub const TFLAC_CHANNEL_MID_SIDE: TflacU8 = 3;
pub const TFLAC_CHANNEL_MODE_COUNT: TflacU8 = 4;

// ---------------------------------------------------------------------------
// tflac_size_memory
// ---------------------------------------------------------------------------

/// ```c
/// tflac_u32 tflac_size_memory(tflac_u32 blocksize) {
///     return (tflac_u32)15U + (5U * ((15U + (blocksize * 4U)) & 0xFFFFFFF0U));
/// }
/// ```
///
/// All arithmetic is unsigned 32-bit and therefore wraps on overflow, exactly
/// as in C.
#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory(blocksize: TflacU32) -> TflacU32 {
    15u32.wrapping_add(
        5u32.wrapping_mul(15u32.wrapping_add(blocksize.wrapping_mul(4)) & 0xFFFF_FFF0u32),
    )
}

// ---------------------------------------------------------------------------
// flac_validate
// ---------------------------------------------------------------------------

/// ```c
/// int flac_validate(tflac *t);
/// ```
///
/// The order of the validation checks and every mutation of `*t` is preserved
/// verbatim from the C source.
///
/// # Safety
/// `t` must be a valid, aligned, non-null pointer to a `struct tflac`, just as
/// the C function requires (the C code dereferences it unconditionally).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flac_validate(t: *mut Tflac) -> c_int {
    let t: &mut Tflac = unsafe { &mut *t };

    if t.blocksize < 16 {
        return -1;
    }
    if t.blocksize > 65535 {
        return -1;
    }
    if t.samplerate == 0 {
        return -1;
    }
    if t.samplerate > 655350 {
        return -1;
    }
    if t.channels == 0 {
        return -1;
    }
    if t.channels > 8 {
        return -1;
    }
    if t.bitdepth == 0 {
        return -1;
    }
    if t.bitdepth > 32 {
        return -1;
    }
    if t.channel_mode != TFLAC_CHANNEL_INDEPENDENT && (t.channels != 2 || t.bitdepth == 32) {
        t.channel_mode = TFLAC_CHANNEL_INDEPENDENT;
    }
    if t.max_rice_value == 0 {
        if t.bitdepth <= 16 {
            t.max_rice_value = 14;
        } else {
            t.max_rice_value = 30;
        }
    } else if t.max_rice_value > 30 {
        return -1;
    }
    if t.max_partition_order > 15 {
        return -1;
    }
    if t.min_partition_order > t.max_partition_order {
        return -1;
    }
    t.partition_order = t.min_partition_order;
    // C: while ((t->blocksize % (1 << (t->partition_order + 1)) == 0) &&
    //            t->partition_order < t->max_partition_order)
    //
    // `1 << (t->partition_order + 1)` is a signed `int` shift; the `%` then
    // converts it to `unsigned int` because `t->blocksize` is `tflac_u32`.
    // `partition_order` never exceeds `max_partition_order <= 15` when the loop
    // body runs, so the shift amount stays within 1..=16 and no UB/overflow
    // occurs. `wrapping_shl` mirrors the C shift for those values.
    while {
        let divisor = 1i32.wrapping_shl(u32::from(t.partition_order).wrapping_add(1)) as u32;
        t.blocksize % divisor == 0 && t.partition_order < t.max_partition_order
    } {
        t.partition_order = t.partition_order.wrapping_add(1);
    }
    t.cur_blocksize = t.blocksize;
    0
}
