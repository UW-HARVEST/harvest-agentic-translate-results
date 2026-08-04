use std::os::raw::{c_uchar, c_uint};

static HALFRATE: [[[u8; 15]; 3]; 2] = [
    [
        [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
        [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
        [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128],
    ],
    [
        [0, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160],
        [0, 16, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192],
        [0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224],
    ],
];

#[unsafe(no_mangle)]
pub extern "C" fn hdr_bitrate(h: *const c_uchar) -> c_uint {
    let h = unsafe { std::slice::from_raw_parts(h, 4) };
    let idx1 = if (h[1] & 0x8) != 0 { 1 } else { 0 };
    let idx2 = (((h[1]) >> 1) & 3) as usize - 1;
    let idx3 = ((h[2]) >> 4) as usize;
    2 * HALFRATE[idx1][idx2][idx3] as c_uint
}
