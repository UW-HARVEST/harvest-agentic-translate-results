pub type TflacU8 = u8;
pub type TflacU32 = u32;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Tflac {
    pub samplerate: TflacU32,
    pub channels: TflacU32,
    pub bitdepth: TflacU32,
    pub channel_mode: TflacU8,
    pub frame_header: TflacU32,
    pub cur_blocksize: TflacU32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TflacChannelMode {
    Independent = 0,
    LeftSide = 1,
    SideRight = 2,
    MidSide = 3,
}

impl TflacChannelMode {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TflacChannelMode::Independent),
            1 => Some(TflacChannelMode::LeftSide),
            2 => Some(TflacChannelMode::SideRight),
            3 => Some(TflacChannelMode::MidSide),
            _ => None,
        }
    }
}

pub fn update_frame_header(t: &mut Tflac) {
    t.frame_header = 0xFFF8u32 << 16;

    match t.cur_blocksize {
        192 => t.frame_header |= 0x01u32 << 12,
        576 => t.frame_header |= 0x02u32 << 12,
        1152 => t.frame_header |= 0x03u32 << 12,
        2304 => t.frame_header |= 0x04u32 << 12,
        4608 => t.frame_header |= 0x05u32 << 12,
        256 => t.frame_header |= 0x08u32 << 12,
        512 => t.frame_header |= 0x09u32 << 12,
        1024 => t.frame_header |= 0x0Au32 << 12,
        2048 => t.frame_header |= 0x0Bu32 << 12,
        4096 => t.frame_header |= 0x0Cu32 << 12,
        8192 => t.frame_header |= 0x0Du32 << 12,
        16384 => t.frame_header |= 0x0Eu32 << 12,
        32768 => t.frame_header |= 0x0Fu32 << 12,
        _ => {
            t.frame_header |= if t.cur_blocksize <= 256 {
                0x06u32 << 12
            } else {
                0x07u32 << 12
            };
        }
    }

    match t.samplerate {
        882000 => t.frame_header |= 0x01u32 << 8,
        176400 => t.frame_header |= 0x02u32 << 8,
        192000 => t.frame_header |= 0x03u32 << 8,
        8000 => t.frame_header |= 0x04u32 << 8,
        16000 => t.frame_header |= 0x05u32 << 8,
        22050 => t.frame_header |= 0x06u32 << 8,
        24000 => t.frame_header |= 0x07u32 << 8,
        32000 => t.frame_header |= 0x08u32 << 8,
        44100 => t.frame_header |= 0x09u32 << 8,
        48000 => t.frame_header |= 0x0Au32 << 8,
        96000 => t.frame_header |= 0x0Bu32 << 8,
        _ => {
            if t.samplerate % 1000 == 0 {
                if t.samplerate / 1000 < 256 {
                    t.frame_header |= 0x0Cu32 << 8;
                }
            } else if t.samplerate < 65536 {
                t.frame_header |= 0x0Du32 << 8;
            } else if t.samplerate % 10 == 0 {
                if t.samplerate / 10 < 65536 {
                    t.frame_header |= 0x0Eu32 << 8;
                }
            }
        }
    }

    let mode = t.channel_mode % 4;
    if let Some(channel_mode) = TflacChannelMode::from_u8(mode) {
        match channel_mode {
            TflacChannelMode::Independent => {
                t.frame_header |= (t.channels - 1) << 4;
            }
            TflacChannelMode::LeftSide => {
                t.frame_header |= 0x08u32 << 4;
            }
            TflacChannelMode::SideRight => {
                t.frame_header |= 0x09u32 << 4;
            }
            TflacChannelMode::MidSide => {
                t.frame_header |= 0x0Au32 << 4;
            }
        }
    }

    match t.bitdepth {
        8 => t.frame_header |= 1u32 << 1,
        12 => t.frame_header |= 2u32 << 1,
        16 => t.frame_header |= 4u32 << 1,
        20 => t.frame_header |= 5u32 << 1,
        24 => t.frame_header |= 6u32 << 1,
        32 => t.frame_header |= 7u32 << 1,
        _ => {}
    }
}

#[no_mangle]
pub extern "C" fn update_frame_header_ffi(t: *mut Tflac) {
    if t.is_null() {
        return;
    }
    unsafe {
        update_frame_header(&mut *t);
    }
}
