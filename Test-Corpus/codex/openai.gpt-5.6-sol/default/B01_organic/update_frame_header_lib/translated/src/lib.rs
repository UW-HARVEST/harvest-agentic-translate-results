#[repr(C)]
pub struct Tflac {
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub frame_header: u32,
    pub cur_blocksize: u32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_frame_header(t: *mut Tflac) {
    let t = unsafe { &mut *t };

    t.frame_header = 0xfff8_u32 << 16;
    t.frame_header |= match t.cur_blocksize {
        192 => 0x01 << 12,
        576 => 0x02 << 12,
        1152 => 0x03 << 12,
        2304 => 0x04 << 12,
        4608 => 0x05 << 12,
        256 => 0x08 << 12,
        512 => 0x09 << 12,
        1024 => 0x0a << 12,
        2048 => 0x0b << 12,
        4096 => 0x0c << 12,
        8192 => 0x0d << 12,
        16384 => 0x0e << 12,
        32768 => 0x0f << 12,
        blocksize if blocksize <= 256 => 0x06 << 12,
        _ => 0x07 << 12,
    };

    t.frame_header |= match t.samplerate {
        882000 => 0x01 << 8,
        176400 => 0x02 << 8,
        192000 => 0x03 << 8,
        8000 => 0x04 << 8,
        16000 => 0x05 << 8,
        22050 => 0x06 << 8,
        24000 => 0x07 << 8,
        32000 => 0x08 << 8,
        44100 => 0x09 << 8,
        48000 => 0x0a << 8,
        96000 => 0x0b << 8,
        samplerate if samplerate % 1000 == 0 && samplerate / 1000 < 256 => 0x0c << 8,
        samplerate if samplerate % 1000 != 0 && samplerate < 65536 => 0x0d << 8,
        samplerate
            if samplerate % 1000 != 0
                && samplerate >= 65536
                && samplerate % 10 == 0
                && samplerate / 10 < 65536 =>
        {
            0x0e << 8
        }
        _ => 0,
    };

    t.frame_header |= match t.channel_mode % 4 {
        0 => t.channels.wrapping_sub(1) << 4,
        1 => 0x08 << 4,
        2 => 0x09 << 4,
        3 => 0x0a << 4,
        _ => 0,
    };

    t.frame_header |= match t.bitdepth {
        8 => 1 << 1,
        12 => 2 << 1,
        16 => 4 << 1,
        20 => 5 << 1,
        24 => 6 << 1,
        32 => 7 << 1,
        _ => 0,
    };
}
