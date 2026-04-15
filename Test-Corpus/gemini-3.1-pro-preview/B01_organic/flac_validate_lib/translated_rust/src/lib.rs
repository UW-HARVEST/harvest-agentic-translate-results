use std::os::raw::c_int;

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

pub const TFLAC_CHANNEL_INDEPENDENT: u8 = 0;
pub const TFLAC_CHANNEL_LEFT_SIDE: u8 = 1;
pub const TFLAC_CHANNEL_SIDE_RIGHT: u8 = 2;
pub const TFLAC_CHANNEL_MID_SIDE: u8 = 3;
pub const TFLAC_CHANNEL_MODE_COUNT: u8 = 4;

#[unsafe(no_mangle)]
pub extern "C" fn tflac_size_memory(blocksize: tflac_u32) -> tflac_u32 {
    15 + (5 * ((15 + (blocksize * 4)) & 0xFFFFFFF0))
}

#[unsafe(no_mangle)]
pub extern "C" fn flac_validate(t: *mut tflac) -> c_int {
    if t.is_null() {
        return -1;
    }
    let t_ref = unsafe { &mut *t };

    if t_ref.blocksize < 16 {
        return -1;
    }
    if t_ref.blocksize > 65535 {
        return -1;
    }
    if t_ref.samplerate == 0 {
        return -1;
    }
    if t_ref.samplerate > 655350 {
        return -1;
    }
    if t_ref.channels == 0 {
        return -1;
    }
    if t_ref.channels > 8 {
        return -1;
    }
    if t_ref.bitdepth == 0 {
        return -1;
    }
    if t_ref.bitdepth > 32 {
        return -1;
    }
    if t_ref.channel_mode != TFLAC_CHANNEL_INDEPENDENT {
        if t_ref.channels != 2 || t_ref.bitdepth == 32 {
            t_ref.channel_mode = TFLAC_CHANNEL_INDEPENDENT;
        }
    }
    if t_ref.max_rice_value == 0 {
        if t_ref.bitdepth <= 16 {
            t_ref.max_rice_value = 14;
        } else {
            t_ref.max_rice_value = 30;
        }
    } else if t_ref.max_rice_value > 30 {
        return -1;
    }
    if t_ref.max_partition_order > 15 {
        return -1;
    }
    if t_ref.min_partition_order > t_ref.max_partition_order {
        return -1;
    }
    t_ref.partition_order = t_ref.min_partition_order;
    while (t_ref.blocksize % (1u32 << (t_ref.partition_order + 1)) == 0) &&
          t_ref.partition_order < t_ref.max_partition_order {
        t_ref.partition_order += 1;
    }
    t_ref.cur_blocksize = t_ref.blocksize;
    0
}
