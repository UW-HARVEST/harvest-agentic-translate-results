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
    // Count up to first NUL byte if any, otherwise the whole length.
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == 0 {
            return i;
        }
    }
    bytes.len()
}
