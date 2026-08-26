// Rust translation of c_src/src/lib.c
//
// The original C source defines a small image library used as a shared
// library; it does not perform any I/O and has no `main`. This translation
// preserves the same observable behavior.

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug)]
#[repr(C)]
pub struct CpImage {
    pub w: i32,
    pub h: i32,
    pub pix: Vec<CpPixel>,
}

/// Flip the image's rows top-to-bottom.
///
/// Note: despite the function being named `flip_horizontal`, the original C
/// implementation actually performs a vertical flip (it swaps row `i` with row
/// `h - i - 1`). The translation preserves this exact behavior — including
/// the misleading name — because we must reproduce the C code as written
/// rather than fix any apparent bugs.
pub fn flip_horizontal(img: &mut CpImage) {
    let w = img.w as usize;
    let h = img.h as usize;
    let flips = h / 2;
    for i in 0..flips {
        let top_start = w * i;
        let bottom_start = w * (h - i - 1);
        for j in 0..w {
            img.pix.swap(top_start + j, bottom_start + j);
        }
    }
}
