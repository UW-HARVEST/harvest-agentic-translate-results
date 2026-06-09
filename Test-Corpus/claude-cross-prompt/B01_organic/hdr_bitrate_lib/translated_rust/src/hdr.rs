// Translation of c_src/src/lib.c

pub fn hdr_bitrate(h: &[u8]) -> u32 {
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
    // Replicates: 2 * halfrate[!!((h[1]) & 0x8)][(((h[1]) >> 1) & 3) - 1][((h[2]) >> 4)]
    // Note: if (((h[1]) >> 1) & 3) == 0, the C code performs an out-of-bounds
    // access (index -1). We reproduce a panic in that case in Rust (preserving
    // the "incorrect behavior" rule by not silently fixing it).
    let dim0 = if (h[1] & 0x8) != 0 { 1usize } else { 0usize };
    let dim1_raw = ((h[1] >> 1) & 0x3) as i32;
    let dim1 = (dim1_raw - 1) as usize;
    let dim2 = (h[2] >> 4) as usize;
    2u32 * HALFRATE[dim0][dim1][dim2] as u32
}
