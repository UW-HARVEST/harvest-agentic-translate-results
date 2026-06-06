#![allow(non_camel_case_types)]

use std::ffi::c_int;

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

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

// enum TFLAC_CHANNEL_MODE
const TFLAC_CHANNEL_INDEPENDENT: tflac_u8 = 0;

#[allow(dead_code)]
fn tflac_size_memory(blocksize: tflac_u32) -> tflac_u32 {
    15u32.wrapping_add(
        5u32.wrapping_mul(
            15u32
                .wrapping_add(blocksize.wrapping_mul(4u32))
                & 0xFFFFFFF0u32,
        ),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flac_validate(t: *mut tflac) -> c_int {
    let t = &mut *t;

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
    if t.channel_mode != TFLAC_CHANNEL_INDEPENDENT {
        if t.channels != 2 || t.bitdepth == 32 {
            t.channel_mode = TFLAC_CHANNEL_INDEPENDENT;
        }
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
    // The C code uses (1 << (partition_order + 1)) where 1 is `int`.
    // Reproduce as i32 shift; partition_order is u8, max value here goes up to max_partition_order (<=15),
    // so partition_order+1 can be up to 16 -> 1<<16 = 65536, safe in i32.
    while {
        let shift = (t.partition_order as i32) + 1;
        let divisor = 1i32 << shift;
        // C: blocksize is tflac_u32 (unsigned), divisor is int. The modulo
        // converts both to unsigned if the unsigned operand is at least as wide;
        // tflac_u32 is uint32_t and int is at least 32 bits, so the int operand
        // is converted to unsigned int. blocksize % (unsigned)divisor.
        (t.blocksize % (divisor as u32)) == 0 && t.partition_order < t.max_partition_order
    } {
        t.partition_order += 1;
    }
    t.cur_blocksize = t.blocksize;
    0
}
