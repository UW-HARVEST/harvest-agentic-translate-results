use std::fs::File;
use std::io::{Read, Write};
use std::process::exit;
pub const SIG_SIZE_BYTES: usize = 16;
pub type Signature = [u8; SIG_SIZE_BYTES];
pub type U32 = u32;
pub type U64 = u64;
pub type U8 = u8;
pub const TRUSTED_CHK_MAX_BUF_SIZE: usize = 1 << 14;
pub fn trusted_utils_sig_to_str(sig: &[u8], out: &mut String) {
    out.clear();
    for charpos in 0..SIG_SIZE_BYTES {
        let val1: u8 = (sig[charpos] >> 4) & 0x0f;
        let val2: u8 = sig[charpos] & 0x0f;
        let c1 = if val1 >= 10 { (b'a' + val1 - 10) as char } else { (b'0' + val1) as char };
        let c2 = if val2 >= 10 { (b'a' + val2 - 10) as char } else { (b'0' + val2) as char };
        out.push(c1);
        out.push(c2);
    }
}
pub fn trusted_utils_write_int(i: i32, file: &mut File) {
    let bytes = i.to_ne_bytes();
    if file.write_all(&bytes).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_equal_signatures(left: &[u8], right: &[u8]) -> bool {
    for i in 0..SIG_SIZE_BYTES {
        if left[i] != right[i] {
            return false;
        }
    }
    true
}
pub fn trusted_utils_write_sig(sig: &[u8], file: &mut File) {
    // C writes 4 ints (16 bytes)
    if file.write_all(&sig[..SIG_SIZE_BYTES]).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn exit_oom() {
    trusted_utils_log("allocation failed - terminating");
    exit(0);
}
pub fn trusted_utils_try_match_arg<'a>(arg: &'a str, opt: &str, out: &mut Option<&'a str>) {
    if arg.starts_with(opt) {
        *out = Some(&arg[opt.len()..]);
    }
}
pub fn trusted_utils_read_sig(out_sig: &mut [u8], file: &mut File) {
    trusted_utils_read_objs(&mut out_sig[..SIG_SIZE_BYTES], 1, SIG_SIZE_BYTES, file);
}
pub fn trusted_utils_read_ul(file: &mut File) -> u64 {
    let mut buf = [0u8; 8];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    u64::from_ne_bytes(buf)
}
pub fn trusted_utils_log_err(msg: &str) {
    println!("c [TRUSTED_CORE {}] [ERROR] {}", std::process::id(), msg);
}
pub fn trusted_utils_copy_bytes(to: &mut [u8], from: &[u8], nb_bytes: u64) {
    for i in 0..(nb_bytes as usize) {
        to[i] = from[i];
    }
}
pub fn trusted_utils_write_ints(data: &[i32], nb_ints: u64, file: &mut File) {
    for i in 0..(nb_ints as usize) {
        trusted_utils_write_int(data[i], file);
    }
}
pub fn trusted_utils_read_uls(data: &mut [u64], nb_uls: u64, file: &mut File) {
    for i in 0..(nb_uls as usize) {
        data[i] = trusted_utils_read_ul(file);
    }
}
pub fn trusted_utils_log(msg: &str) {
    println!("c [TRUSTED_CORE {}] {}", std::process::id(), msg);
}
pub fn trusted_utils_read_objs(data: &mut [u8], size: usize, nb_objs: usize, file: &mut File) {
    let total = size * nb_objs;
    if file.read_exact(&mut data[..total]).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_read_ints(data: &mut [i32], nb_ints: u64, file: &mut File) {
    for i in 0..(nb_ints as usize) {
        data[i] = trusted_utils_read_int(file);
    }
}
pub fn trusted_utils_write_uls(data: &[u64], nb_uls: u64, file: &mut File) {
    for i in 0..(nb_uls as usize) {
        trusted_utils_write_ul(data[i], file);
    }
}
pub fn trusted_utils_realloc<T: Clone + Default>(from: &mut [T], new_size: u64) -> Vec<T> {
    let mut v: Vec<T> = Vec::with_capacity(new_size as usize);
    let copy_len = std::cmp::min(from.len(), new_size as usize);
    for i in 0..copy_len {
        v.push(from[i].clone());
    }
    while v.len() < new_size as usize {
        v.push(T::default());
    }
    v
}
pub fn trusted_utils_write_bool(b: bool, file: &mut File) {
    let byte: u8 = if b { 1 } else { 0 };
    if file.write_all(&[byte]).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_try_match_flag(arg: &str, opt: &str, out: &mut bool) {
    if arg.starts_with(opt) {
        *out = true;
    }
}
pub fn trusted_utils_exit_eof() {
    trusted_utils_log("end-of-file - terminating");
    exit(0);
}
pub fn trusted_utils_write_char(c: char, file: &mut File) {
    let byte: u8 = c as u8;
    if file.write_all(&[byte]).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_calloc<T: Clone + Default>(nb_objs: u64, _size_per_obj: u64) -> Vec<T> {
    let mut v: Vec<T> = Vec::with_capacity(nb_objs as usize);
    for _ in 0..nb_objs {
        v.push(T::default());
    }
    v
}
pub fn trusted_utils_read_bool(file: &mut File) -> bool {
    let mut buf = [0u8; 1];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    buf[0] != 0
}
pub fn trusted_utils_read_int(file: &mut File) -> i32 {
    let mut buf = [0u8; 4];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    i32::from_ne_bytes(buf)
}
pub fn trusted_utils_str_to_sig(s: &str, out: &mut [u8]) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 * SIG_SIZE_BYTES {
        return false;
    }
    for bytepos in 0..SIG_SIZE_BYTES {
        let hex1 = bytes[bytepos * 2] as char;
        let hex2 = bytes[bytepos * 2 + 1] as char;
        let v1 = if hex1 >= '0' && hex1 <= '9' {
            (hex1 as i32) - ('0' as i32)
        } else {
            10 + (hex1 as i32) - ('a' as i32)
        };
        let v2 = if hex2 >= '0' && hex2 <= '9' {
            (hex2 as i32) - ('0' as i32)
        } else {
            10 + (hex2 as i32) - ('a' as i32)
        };
        let byte = 16 * v1 + v2;
        if byte < 0 || byte >= 256 {
            return false;
        }
        out[bytepos] = byte as u8;
    }
    true
}
pub fn trusted_utils_write_ul(u: u64, file: &mut File) {
    let bytes = u.to_ne_bytes();
    if file.write_all(&bytes).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_read_char(file: &mut File) -> i32 {
    let mut buf = [0u8; 1];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    buf[0] as i32
}
