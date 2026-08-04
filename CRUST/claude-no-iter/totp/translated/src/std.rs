// Constants
pub const SIZE_MAX: usize = !0;
// Function Declarations
pub fn memset(s: &mut [u8], c: i32, n: usize) -> &mut [u8] {
    let byte = c as u8;
    for i in 0..n {
        s[i] = byte;
    }
    s
}
pub fn memcpy<'a>(dst: &'a mut [u8], src: &'a [u8], n: usize) -> &'a mut [u8] {
    for i in 0..n {
        dst[i] = src[i];
    }
    dst
}
pub fn strlen(s: &str) -> usize {
    // Mirror C's strlen: count bytes up to (but not including) the first NUL byte.
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] != 0 {
        i += 1;
    }
    i
}
