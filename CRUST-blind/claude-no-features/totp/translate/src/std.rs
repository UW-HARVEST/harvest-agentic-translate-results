// Constants
pub const SIZE_MAX: usize = !0;
// Function Declarations
pub fn memset(s: &mut [u8], c: i32, n: usize) -> &mut [u8] {
    let len = if n < s.len() { n } else { s.len() };
    for i in 0..len {
        s[i] = c as u8;
    }
    s
}
pub fn memcpy<'a>(dst: &'a mut [u8], src: &'a [u8], n: usize) -> &'a mut [u8] {
    let mut len = n;
    if dst.len() < len {
        len = dst.len();
    }
    if src.len() < len {
        len = src.len();
    }
    for i in 0..len {
        dst[i] = src[i];
    }
    dst
}
pub fn strlen(s: &str) -> usize {
    // C strlen: counts bytes until a NUL terminator. For Rust &str without
    // an embedded NUL, this is just the byte length.
    match s.as_bytes().iter().position(|&b| b == 0) {
        Some(idx) => idx,
        None => s.len(),
    }
}
