use std::ffi::c_uint;

#[repr(C)]
struct HalfrateStorage {
    prefix: [u8; 15],
    table: [[[u8; 15]; 3]; 2],
    suffix: [u8; 1],
}

static HALFRATE: HalfrateStorage = HalfrateStorage {
    prefix: [0; 15],
    table: [
        [
            [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
            [0, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80],
            [0, 16, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128],
        ],
        [
            [0, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160],
            [
                0, 16, 24, 28, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192,
            ],
            [
                0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224,
            ],
        ],
    ],
    suffix: [0; 1],
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hdr_bitrate(h: *const u8) -> c_uint {
    let byte1 = unsafe { *h.add(1) };
    let byte2 = unsafe { *h.add(2) };
    let version = usize::from(byte1 & 0x08 != 0);
    let layer = isize::from((byte1 >> 1) & 3) - 1;
    let bitrate = isize::from(byte2 >> 4);
    let offset = version as isize * 45 + layer * 15 + bitrate;

    2 * c_uint::from(unsafe { *HALFRATE.table.as_ptr().cast::<u8>().offset(offset) })
}
