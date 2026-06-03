use std::ffi::c_int;

pub type TflacU8 = u8;
pub type TflacU32 = u32;

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

const TFLAC_CHANNEL_INDEPENDENT: TflacU8 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory(blocksize: TflacU32) -> TflacU32 {
    15u32.wrapping_add(
        5u32.wrapping_mul(
            15u32
                .wrapping_add(blocksize.wrapping_mul(4u32))
                & 0xFFFFFFF0u32,
        ),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flac_validate(t: *mut Tflac) -> c_int {
    let t = unsafe { &mut *t };

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
    // C: while ((t->blocksize % (1 << (t->partition_order + 1)) == 0) &&
    //            t->partition_order < t->max_partition_order)
    // In C, the shift is done in `int` after integer promotion of the u8.
    while {
        let shift = (t.partition_order as c_int) + 1;
        let divisor = (1i32 << shift) as u32;
        t.blocksize % divisor == 0 && t.partition_order < t.max_partition_order
    } {
        t.partition_order += 1;
    }
    t.cur_blocksize = t.blocksize;
    0
}
