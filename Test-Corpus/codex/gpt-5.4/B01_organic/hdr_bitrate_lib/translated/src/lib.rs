use std::ffi::{c_uchar, c_uint};

static HALFRATE: [[[u8; 15]; 3]; 2] = [
    [
        [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
        [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
        [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128],
    ],
    [
        [0, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160],
        [0, 16, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192],
        [
            0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224,
        ],
    ],
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const c_uchar) -> c_uint {
    let h1 = *h.add(1);
    let h2 = *h.add(2);

    let version_index = usize::from((h1 & 0x8) != 0);
    let layer_index = (((h1 >> 1) & 3).wrapping_sub(1)) as usize;
    let bitrate_index = usize::from(h2 >> 4);

    2 * u32::from(*HALFRATE
        .get_unchecked(version_index)
        .get_unchecked(layer_index)
        .get_unchecked(bitrate_index))
}
