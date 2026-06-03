use std::ffi::c_uint;

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
pub unsafe extern "C" fn hdr_bitrate(h: *const u8) -> c_uint {
    let h1 = unsafe { *h.add(1) };
    let h2 = unsafe { *h.add(2) };

    // !!((h[1]) & 0x8)
    let i: usize = if (h1 & 0x8) != 0 { 1 } else { 0 };
    // (((h[1]) >> 1) & 3) - 1
    // In C, this is computed as int; if the result is -1 (when bits 1..2 are 0),
    // the original C invokes undefined behaviour. Real MP3 headers never have
    // layer == 0 (reserved), so this branch is unreachable for valid input.
    let j: usize = (((h1 >> 1) & 3) as i32 - 1) as usize;
    // ((h[2]) >> 4)
    let k: usize = (h2 >> 4) as usize;

    (2u32 * HALFRATE[i][j][k] as u32) as c_uint
}
