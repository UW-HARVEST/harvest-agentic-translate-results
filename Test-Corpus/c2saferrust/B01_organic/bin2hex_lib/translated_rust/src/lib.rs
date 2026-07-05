
extern "C" {
    fn abort() -> !;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
#[no_mangle]
pub fn bin2hex(bin: &[u8]) -> String {
    let mut hex = String::with_capacity(bin.len() * 2);
    for &byte in bin {
        let high = byte >> 4;
        let low = byte & 0x0f;

        hex.push(if high < 10 {
            (b'0' + high) as char
        } else {
            (b'a' + (high - 10)) as char
        });

        hex.push(if low < 10 {
            (b'0' + low) as char
        } else {
            (b'a' + (low - 10)) as char
        });
    }
    hex
}

