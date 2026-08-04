// Constants
pub const SIZE_MAX: usize = !0;
// Function Declarations
pub fn memset(s: &mut [u8], c: i32, n: usize) -> &mut [u8] {
    for byte in &mut s[..n] {
        *byte = c as u8;
    }
    s
}
pub fn memcpy<'a>(dst: &'a mut [u8], src: &'a [u8], n: usize) -> &'a mut [u8] {
    dst[..n].copy_from_slice(&src[..n]);
    dst
}
pub fn strlen(s: &str) -> usize {
    s.bytes().position(|byte| byte == 0).unwrap_or(s.len())
}
