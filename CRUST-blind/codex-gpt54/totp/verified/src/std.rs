// Constants
pub const SIZE_MAX: usize = !0;
// Function Declarations
pub fn memset(s: &mut [u8], c: i32, n: usize) -> &mut [u8] {
    let count = n.min(s.len());
    s[..count].fill(c as u8);
    s
}
pub fn memcpy<'a>(dst: &'a mut [u8], src: &'a [u8], n: usize) -> &'a mut [u8] {
    let count = n.min(dst.len()).min(src.len());
    dst[..count].copy_from_slice(&src[..count]);
    dst
}
pub fn strlen(s: &str) -> usize {
    s.as_bytes()
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(s.len())
}
