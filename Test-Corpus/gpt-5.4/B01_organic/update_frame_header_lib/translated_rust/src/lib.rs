use std::os::raw::c_uchar;

pub type tflac_u8 = u8;
pub type tflac_u32 = u32;

#[repr(C)]
pub struct tflac {
    pub samplerate: tflac_u32,
    pub channels: tflac_u32,
    pub bitdepth: tflac_u32,
    pub channel_mode: tflac_u8,
    pub frame_header: tflac_u32,
    pub cur_blocksize: tflac_u32,
}

#[repr(u8)]
enum TflacChannelMode {
    TflacChannelIndependent = 0,
    TflacChannelLeftSide = 1,
    TflacChannelSideRight = 2,
    TflacChannelMidSide = 3,
}

#[unsafe(no_mangle)]
pub extern "C" fn update_frame_header(t: *mut tflac) {
    let t = unsafe { t.as_mut() };
    let Some(t) = t else {
        return;
    };

    t.frame_header = 0xFFF8_u32 << 16;

    match t.cur_blocksize {
        192 => t.frame_header |= 0x01_u32 << 12,
        576 => t.frame_header |= 0x02_u32 << 12,
        1152 => t.frame_header |= 0x03_u32 << 12,
        2304 => t.frame_header |= 0x04_u32 << 12,
        4608 => t.frame_header |= 0x05_u32 << 12,
        256 => t.frame_header |= 0x08_u32 << 12,
        512 => t.frame_header |= 0x09_u32 << 12,
        1024 => t.frame_header |= 0x0A_u32 << 12,
        2048 => t.frame_header |= 0x0B_u32 << 12,
        4096 => t.frame_header |= 0x0C_u32 << 12,
        8192 => t.frame_header |= 0x0D_u32 << 12,
        16384 => t.frame_header |= 0x0E_u32 << 12,
        32768 => t.frame_header |= 0x0F_u32 << 12,
        _ => {
            t.frame_header |= if t.cur_blocksize <= 256 {
                0x06_u32 << 12
            } else {
                0x07_u32 << 12
            };
        }
    }

    match t.samplerate {
        882000 => t.frame_header |= 0x01_u32 << 8,
        176400 => t.frame_header |= 0x02_u32 << 8,
        192000 => t.frame_header |= 0x03_u32 << 8,
        8000 => t.frame_header |= 0x04_u32 << 8,
        16000 => t.frame_header |= 0x05_u32 << 8,
        22050 => t.frame_header |= 0x06_u32 << 8,
        24000 => t.frame_header |= 0x07_u32 << 8,
        32000 => t.frame_header |= 0x08_u32 << 8,
        44100 => t.frame_header |= 0x09_u32 << 8,
        48000 => t.frame_header |= 0x0A_u32 << 8,
        96000 => t.frame_header |= 0x0B_u32 << 8,
        _ => {
            if t.samplerate % 1000 == 0 {
                if t.samplerate / 1000 < 256 {
                    t.frame_header |= 0x0C_u32 << 8;
                }
            } else if t.samplerate < 65536 {
                t.frame_header |= 0x0D_u32 << 8;
            } else if t.samplerate % 10 == 0 {
                if t.samplerate / 10 < 65536 {
                    t.frame_header |= 0x0E_u32 << 8;
                }
            }
        }
    }

    let mode: c_uchar = (t.channel_mode % 4) as c_uchar;
    match mode {
        x if x == TflacChannelMode::TflacChannelIndependent as u8 => {
            t.frame_header |= (t.channels - 1) << 4;
        }
        x if x == TflacChannelMode::TflacChannelLeftSide as u8 => {
            t.frame_header |= 0x08_u32 << 4;
        }
        x if x == TflacChannelMode::TflacChannelSideRight as u8 => {
            t.frame_header |= 0x09_u32 << 4;
        }
        x if x == TflacChannelMode::TflacChannelMidSide as u8 => {
            t.frame_header |= 0x0A_u32 << 4;
        }
        _ => {}
    }

    match t.bitdepth {
        8 => t.frame_header |= 1_u32 << 1,
        12 => t.frame_header |= 2_u32 << 1,
        16 => t.frame_header |= 4_u32 << 1,
        20 => t.frame_header |= 5_u32 << 1,
        24 => t.frame_header |= 6_u32 << 1,
        32 => t.frame_header |= 7_u32 << 1,
        _ => {}
    }
}
