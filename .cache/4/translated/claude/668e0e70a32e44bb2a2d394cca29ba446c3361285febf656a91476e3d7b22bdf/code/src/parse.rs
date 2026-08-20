//! Translation of `int ima_parse(struct ima_info *info, const void *data)`
//! from `c_src/src/lib.c`.

use core::ffi::{c_int, c_void};

use crate::caf::{
    CAF_CHUNK_DATA, CAF_CHUNK_DESC, CAF_CHUNK_PAKT, CAF_FORMAT_IMA4, CAF_TYPE_CAFF, CAF_VERSION,
    caf_audio_description, caf_chunk, caf_data, caf_header, caf_packet_table,
};
use crate::endian::{ima_btoh16, ima_btoh32, ima_btoh64};
use crate::{ima_block, ima_f64_t, ima_info, ima_s64_t, ima_u16_t, ima_u32_t, ima_u64_t};

// ---------------------------------------------------------------------------
// Unaligned raw loads
//
// The C code reaches into the caller's buffer through `struct` pointers formed
// by pointer casts at arbitrary offsets. Reproducing that with `read_unaligned`
// keeps the observable behaviour (including a fault on a NULL/bad pointer)
// while avoiding Rust-level alignment UB that the C compiler happens to get
// away with on x86.
// ---------------------------------------------------------------------------

#[inline]
unsafe fn load_u16(base: *const u8, offset: usize) -> ima_u16_t {
    unsafe { base.wrapping_add(offset).cast::<ima_u16_t>().read_unaligned() }
}

#[inline]
unsafe fn load_u32(base: *const u8, offset: usize) -> ima_u32_t {
    unsafe { base.wrapping_add(offset).cast::<ima_u32_t>().read_unaligned() }
}

#[inline]
unsafe fn load_u64(base: *const u8, offset: usize) -> ima_u64_t {
    unsafe { base.wrapping_add(offset).cast::<ima_u64_t>().read_unaligned() }
}

#[inline]
unsafe fn load_f64(base: *const u8, offset: usize) -> ima_f64_t {
    unsafe { base.wrapping_add(offset).cast::<ima_f64_t>().read_unaligned() }
}

// ---------------------------------------------------------------------------
// x86-64 `double` -> `unsigned long long` conversion
//
// The C source does
//
//     conv64.u = desc->sample_rate;   /* NOT a bit cast: a value conversion! */
//     conv64.u = ima_btoh64(*(const ima_u64_t *)&conv64.u);
//     info->sample_rate = conv64.f;
//
// Assigning the `ima_f64_t` member to the `ima_u64_t` member of the union
// performs a floating-point-to-integer *conversion* (truncation toward zero),
// not a reinterpretation of the bits. Because `desc->sample_rate` holds a
// big-endian double read as if it were native, the value is essentially
// arbitrary, and for most inputs the conversion is out of `unsigned long long`
// range, which C leaves undefined.
//
// This is a bug in the original, but it must be reproduced exactly, so the
// conversion below emulates the code GCC emits on x86-64 (verified identical at
// -O0 and -O2):
//
//     comisd xmm0, 2^63
//     jae    .L2                 ; taken only when ordered and xmm0 >= 2^63
//     cvttsd2si rax, xmm0
//     jmp    .L3
// .L2:
//     subsd  xmm0, 2^63
//     cvttsd2si rax, xmm0
//     xor    rax, 0x8000000000000000
// .L3:
// ---------------------------------------------------------------------------

/// 2^63 as an exactly representable `f64`.
const TWO_POW_63: ima_f64_t = 9_223_372_036_854_775_808.0;

/// Emulates the x86 `CVTTSD2SI r64, xmm` instruction.
///
/// Truncates toward zero. When the source is NaN, infinite, or the truncated
/// result does not fit in a signed 64-bit integer, the hardware produces the
/// "integer indefinite" value `0x8000000000000000`. Rust's `as` cast saturates
/// instead, so the range check has to be explicit.
#[inline]
fn cvttsd2si(x: ima_f64_t) -> ima_s64_t {
    const INTEGER_INDEFINITE: ima_s64_t = ima_s64_t::MIN;

    if x.is_nan() {
        return INTEGER_INDEFINITE;
    }
    let truncated = x.trunc();
    // Valid iff truncated is in [-2^63, 2^63). Both bounds are exact in f64.
    if truncated >= -TWO_POW_63 && truncated < TWO_POW_63 {
        truncated as ima_s64_t
    } else {
        INTEGER_INDEFINITE
    }
}

/// Emulates GCC's x86-64 lowering of a C `double` -> `unsigned long long`
/// conversion, including the results it produces for the cases C calls
/// undefined.
#[inline]
fn f64_to_u64(x: ima_f64_t) -> ima_u64_t {
    // `comisd`/`jae` is only taken when the comparison is ordered *and*
    // x >= 2^63; NaN (unordered) sets CF and falls through to the direct
    // `cvttsd2si`, which then yields the indefinite value. `x >= TWO_POW_63`
    // is false for NaN in Rust too, so the mapping is direct.
    if x >= TWO_POW_63 {
        (cvttsd2si(x - TWO_POW_63) as ima_u64_t) ^ (1 << 63)
    } else {
        cvttsd2si(x) as ima_u64_t
    }
}

/// ```c
/// int ima_parse(struct ima_info *info, const void *data);
/// ```
///
/// Parses a CAF container holding IMA4 audio.
///
/// Returns `0` on success, `-1` if the file type is not `caff`, `-2` if the
/// header version is not 1, and `-3` if the audio description does not
/// advertise the `ima4` format.
///
/// # Safety
///
/// Mirrors the C contract exactly: `info` must point to a writable
/// `struct ima_info` and `data` must point to a well-formed CAF stream. As in
/// the C original there is no bounds checking, no NULL checking, and no
/// termination condition for the chunk walk other than finding a `data` chunk,
/// so malformed input can fault or loop forever.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ima_parse(info: *mut ima_info, data: *const c_void) -> c_int {
    unsafe {
        // const struct caf_header *header = (const struct caf_header *)data;
        let header: *const u8 = data.cast();
        // const struct caf_chunk *chunk = (const struct caf_chunk *)&header[1];
        let mut chunk: *const u8 = header.wrapping_add(caf_header::SIZE);
        // const struct caf_audio_description *desc = NULL;
        let mut desc: *const u8 = core::ptr::null();
        // const struct caf_packet_table *pakt = NULL;
        let mut pakt: *const u8 = core::ptr::null();
        // const struct ima_block *blocks = NULL;
        let blocks: *const u8;

        // if (ima_btoh32(header->type) != 'caff') return -1;
        if ima_btoh32(load_u32(header, caf_header::OFF_TYPE)) != CAF_TYPE_CAFF {
            return -1;
        }
        // if (ima_btoh16(header->version) != 1) return -2;
        if ima_btoh16(load_u16(header, caf_header::OFF_VERSION)) != CAF_VERSION {
            return -2;
        }

        // The `data` chunk's size is what ends up in `info->size`; because the
        // C code declares `chunk_size` outside the loop and only breaks out of
        // it in the `data` case, this is the last value assigned.
        let mut chunk_size: ima_s64_t;
        loop {
            let chunk_type = ima_btoh32(load_u32(chunk, caf_chunk::OFF_TYPE));
            chunk_size = ima_btoh64(load_u64(chunk, caf_chunk::OFF_SIZE)) as ima_s64_t;

            if chunk_type == CAF_CHUNK_DESC {
                // desc = (const struct caf_audio_description *)&chunk[1];
                desc = chunk.wrapping_add(caf_chunk::SIZE);
            } else if chunk_type == CAF_CHUNK_PAKT {
                // pakt = (const struct caf_packet_table *)&chunk[1];
                pakt = chunk.wrapping_add(caf_chunk::SIZE);
            } else if chunk_type == CAF_CHUNK_DATA {
                // blocks = (const struct ima_block *)
                //              &((const struct caf_data *)&chunk[1])[1];
                blocks = chunk
                    .wrapping_add(caf_chunk::SIZE)
                    .wrapping_add(caf_data::SIZE);
                break;
            }

            // chunk = (const struct caf_chunk *)
            //             ((const ima_u8_t *)&chunk[1] + chunk_size);
            chunk = chunk
                .wrapping_add(caf_chunk::SIZE)
                .wrapping_offset(chunk_size as isize);
        }

        // if (ima_btoh32(desc->format_id) != 'ima4') return -3;
        if ima_btoh32(load_u32(desc, caf_audio_description::OFF_FORMAT_ID)) != CAF_FORMAT_IMA4 {
            return -3;
        }

        // info->blocks = blocks;
        (*info).blocks = blocks as *const ima_block;
        // info->size = chunk_size;  (ima_s64_t -> ima_u64_t, bits preserved)
        (*info).size = chunk_size as ima_u64_t;
        // info->frame_count = ima_btoh64(pakt->frame_count);
        (*info).frame_count = ima_btoh64(load_u64(pakt, caf_packet_table::OFF_FRAME_COUNT));
        // info->channel_count = ima_btoh32(desc->channels_per_frame);
        (*info).channel_count =
            ima_btoh32(load_u32(desc, caf_audio_description::OFF_CHANNELS_PER_FRAME));

        // conv64.u = desc->sample_rate;                       /* f64 -> u64 */
        // conv64.u = ima_btoh64(*(const ima_u64_t *)&conv64.u);
        // info->sample_rate = conv64.f;                       /* bit cast */
        let converted = f64_to_u64(load_f64(desc, caf_audio_description::OFF_SAMPLE_RATE));
        (*info).sample_rate = ima_f64_t::from_bits(ima_btoh64(converted));

        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cvttsd2si_edge_cases() {
        assert_eq!(cvttsd2si(0.0), 0);
        assert_eq!(cvttsd2si(-0.0), 0);
        assert_eq!(cvttsd2si(1.9), 1);
        assert_eq!(cvttsd2si(-1.9), -1);
        assert_eq!(cvttsd2si(f64::NAN), i64::MIN);
        assert_eq!(cvttsd2si(f64::INFINITY), i64::MIN);
        assert_eq!(cvttsd2si(f64::NEG_INFINITY), i64::MIN);
        assert_eq!(cvttsd2si(TWO_POW_63), i64::MIN);
        assert_eq!(cvttsd2si(-TWO_POW_63), i64::MIN);
        // Subnormals truncate to zero, which is the common real-world case:
        // a big-endian 44100.0 read as a native double is a tiny subnormal.
        assert_eq!(cvttsd2si(f64::from_bits(0x0000_0000_8088_e540)), 0);
    }

    #[test]
    fn f64_to_u64_edge_cases() {
        assert_eq!(f64_to_u64(0.0), 0);
        assert_eq!(f64_to_u64(42.75), 42);
        assert_eq!(f64_to_u64(f64::NAN), 0x8000_0000_0000_0000);
        assert_eq!(f64_to_u64(TWO_POW_63), 0x8000_0000_0000_0000);
        assert_eq!(f64_to_u64(f64::INFINITY), 0);
    }
}
