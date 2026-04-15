use std::os::raw::c_uint;

#[unsafe(no_mangle)]
pub extern "C" fn hdr_bitrate(h: *const u8) -> c_uint {
    const HALFRATE: [[[u8; 15]; 3]; 2] = [
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

    let h1 = unsafe { *h.add(1) };
    let h2 = unsafe { *h.add(2) };

    let idx0 = if (h1 & 0x8) != 0 { 1 } else { 0 };
    let idx1 = (((h1 >> 1) & 3) as usize).wrapping_sub(1);
    let idx2 = (h2 >> 4) as usize;

    2 * (HALFRATE[idx0][idx1][idx2] as c_uint)
}
