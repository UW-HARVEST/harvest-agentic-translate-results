use std::fs::File;
use std::io::{Read, Write};
pub const SIG_SIZE_BYTES: usize = 16;
pub type Signature = [u8; SIG_SIZE_BYTES];
pub type U32 = u32;
pub type U64 = u64;
pub type U8 = u8;
pub const TRUSTED_CHK_MAX_BUF_SIZE: usize = 1 << 14;

pub fn trusted_utils_sig_to_str(sig: &[u8], out: &mut String) {
    out.clear();
    for charpos in 0..SIG_SIZE_BYTES {
        let val1 = (sig[charpos] >> 4) & 0x0f;
        let val2 = sig[charpos] & 0x0f;
        let c1 = if val1 >= 10 {
            (b'a' + val1 - 10) as char
        } else {
            (b'0' + val1) as char
        };
        let c2 = if val2 >= 10 {
            (b'a' + val2 - 10) as char
        } else {
            (b'0' + val2) as char
        };
        out.push(c1);
        out.push(c2);
    }
}

pub fn trusted_utils_write_int(i: i32, file: &mut File) {
    let bytes = i.to_le_bytes();
    if file.write_all(&bytes).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_equal_signatures(left: &[u8], right: &[u8]) -> bool {
    if left.len() < SIG_SIZE_BYTES || right.len() < SIG_SIZE_BYTES {
        return false;
    }
    for i in 0..SIG_SIZE_BYTES {
        if left[i] != right[i] {
            return false;
        }
    }
    true
}

pub fn trusted_utils_write_sig(sig: &[u8], file: &mut File) {
    if file.write_all(&sig[..SIG_SIZE_BYTES]).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn exit_oom() {
    trusted_utils_log("allocation failed - terminating");
    std::process::exit(0);
}

pub fn trusted_utils_try_match_arg<'a>(arg: &'a str, opt: &str, out: &mut Option<&'a str>) {
    if arg.starts_with(opt) {
        *out = Some(&arg[opt.len()..]);
    }
}

pub fn trusted_utils_read_sig(out_sig: &mut [u8], file: &mut File) {
    let n = SIG_SIZE_BYTES.min(out_sig.len());
    let mut buf = [0u8; SIG_SIZE_BYTES];
    if file.read_exact(&mut buf[..n]).is_err() {
        trusted_utils_exit_eof();
    }
    out_sig[..n].copy_from_slice(&buf[..n]);
}

pub fn trusted_utils_read_ul(file: &mut File) -> u64 {
    let mut buf = [0u8; 8];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    u64::from_le_bytes(buf)
}

pub fn trusted_utils_log_err(msg: &str) {
    println!("c [TRUSTED_CORE {}] [ERROR] {}", std::process::id(), msg);
}

pub fn trusted_utils_copy_bytes(to: &mut [u8], from: &[u8], nb_bytes: u64) {
    let n = nb_bytes as usize;
    for i in 0..n {
        to[i] = from[i];
    }
}

pub fn trusted_utils_write_ints(data: &[i32], nb_ints: u64, file: &mut File) {
    let n = nb_ints as usize;
    let mut buf = Vec::with_capacity(n * 4);
    for i in 0..n {
        buf.extend_from_slice(&data[i].to_le_bytes());
    }
    if file.write_all(&buf).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_read_uls(data: &mut [u64], nb_uls: u64, file: &mut File) {
    let n = nb_uls as usize;
    for i in 0..n {
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
    let n = nb_ints as usize;
    for i in 0..n {
        data[i] = trusted_utils_read_int(file);
    }
}

pub fn trusted_utils_write_uls(data: &[u64], nb_uls: u64, file: &mut File) {
    let n = nb_uls as usize;
    let mut buf = Vec::with_capacity(n * 8);
    for i in 0..n {
        buf.extend_from_slice(&data[i].to_le_bytes());
    }
    if file.write_all(&buf).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_realloc<T>(from: &mut [T], new_size: u64) -> Vec<T>
where
    T: Default + Clone,
{
    let n = new_size as usize;
    let mut v: Vec<T> = Vec::with_capacity(n);
    let copy_n = n.min(from.len());
    for i in 0..copy_n {
        // move element out by replacing with default
        let mut tmp = T::default();
        std::mem::swap(&mut tmp, &mut from[i]);
        v.push(tmp);
    }
    while v.len() < n {
        v.push(T::default());
    }
    v
}

pub fn trusted_utils_write_bool(b: bool, file: &mut File) {
    let byte = if b { 1u8 } else { 0u8 };
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
    std::process::exit(0);
}

pub fn trusted_utils_write_char(c: char, file: &mut File) {
    let byte = c as u32 as u8;
    if file.write_all(&[byte]).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_calloc<T>(nb_objs: u64, _size_per_obj: u64) -> Vec<T>
where
    T: Default + Clone,
{
    let n = nb_objs as usize;
    let mut v: Vec<T> = Vec::with_capacity(n);
    for _ in 0..n {
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
    i32::from_le_bytes(buf)
}

pub fn trusted_utils_str_to_sig(s: &str, out: &mut [u8]) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < SIG_SIZE_BYTES * 2 {
        return false;
    }
    for bytepos in 0..SIG_SIZE_BYTES {
        let hex1 = bytes[bytepos * 2] as char;
        let hex2 = bytes[bytepos * 2 + 1] as char;
        let v1 = if hex1 >= '0' && hex1 <= '9' {
            (hex1 as i32) - ('0' as i32)
        } else if hex1 >= 'a' && hex1 <= 'f' {
            10 + (hex1 as i32) - ('a' as i32)
        } else {
            return false;
        };
        let v2 = if hex2 >= '0' && hex2 <= '9' {
            (hex2 as i32) - ('0' as i32)
        } else if hex2 >= 'a' && hex2 <= 'f' {
            10 + (hex2 as i32) - ('a' as i32)
        } else {
            return false;
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
    let bytes = u.to_le_bytes();
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
