use std::ffi::c_int;

const TFLAC_CHANNEL_INDEPENDENT: u8 = 0;

#[repr(C)]
pub struct Tflac {
    pub blocksize: u32,
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub max_rice_value: u8,
    pub min_partition_order: u8,
    pub max_partition_order: u8,
    pub partition_order: u8,
    pub cur_blocksize: u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory(blocksize: u32) -> u32 {
    15_u32.wrapping_add(
        5_u32.wrapping_mul(15_u32.wrapping_add(blocksize.wrapping_mul(4)) & 0xffff_fff0),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flac_validate(t: *mut Tflac) -> c_int {
    let t = unsafe { &mut *t };

    if t.blocksize < 16 {
        return -1;
    }
    if t.blocksize > 65_535 {
        return -1;
    }
    if t.samplerate == 0 {
        return -1;
    }
    if t.samplerate > 655_350 {
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
    while t.blocksize % (1_u32 << (t.partition_order + 1)) == 0
        && t.partition_order < t.max_partition_order
    {
        t.partition_order += 1;
    }
    t.cur_blocksize = t.blocksize;
    0
}
