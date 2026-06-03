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
        let c1 = if val1 >= 10 { (b'a' + val1 - 10) as char } else { (b'0' + val1) as char };
        let c2 = if val2 >= 10 { (b'a' + val2 - 10) as char } else { (b'0' + val2) as char };
        out.push(c1);
        out.push(c2);
    }
}

pub fn trusted_utils_write_int(i: i32, file: &mut File) {
    if file.write_all(&i.to_ne_bytes()).is_err() {
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
    if file.write_all(&sig[..SIG_SIZE_BYTES]).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn exit_oom() {
    trusted_utils_log("allocation failed - terminating");
    std::process::exit(0);
}

pub fn trusted_utils_try_match_arg(arg: &str, opt: &str, out: &mut Option<&str>) {
    if arg.starts_with(opt) {
        // Need to extend lifetime — we leak the substring as 'static.
        let s: String = arg[opt.len()..].to_string();
        let leaked: &'static str = Box::leak(s.into_boxed_str());
        *out = Some(leaked);
    }
}

pub fn trusted_utils_read_sig(out_sig: &mut [u8], file: &mut File) {
    if file.read_exact(&mut out_sig[..SIG_SIZE_BYTES]).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_read_ul(file: &mut File) -> u64 {
    let mut buf = [0u8; 8];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
        return 0;
    }
    u64::from_ne_bytes(buf)
}

pub fn trusted_utils_log_err(msg: &str) {
    println!("c [TRUSTED_CORE {}] [ERROR] {}", std::process::id(), msg);
}

pub fn trusted_utils_copy_bytes(to: &mut [u8], from: &[u8], nb_bytes: u64) {
    let n = nb_bytes as usize;
    to[..n].copy_from_slice(&from[..n]);
}

pub fn trusted_utils_write_ints(data: &[i32], nb_ints: u64, file: &mut File) {
    let n = nb_ints as usize;
    let mut buf = Vec::with_capacity(n * 4);
    for i in 0..n {
        buf.extend_from_slice(&data[i].to_ne_bytes());
    }
    if file.write_all(&buf).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_read_uls(data: &mut [u64], nb_uls: u64, file: &mut File) {
    let n = nb_uls as usize;
    let mut buf = vec![0u8; n * 8];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
        return;
    }
    for i in 0..n {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&buf[i * 8..(i + 1) * 8]);
        data[i] = u64::from_ne_bytes(bytes);
    }
}

pub fn trusted_utils_log(msg: &str) {
    println!("c [TRUSTED_CORE {}] {}", std::process::id(), msg);
}

pub fn trusted_utils_read_objs(data: &mut [u8], size: usize, nb_objs: usize, file: &mut File) {
    let n = size * nb_objs;
    if file.read_exact(&mut data[..n]).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_read_ints(data: &mut [i32], nb_ints: u64, file: &mut File) {
    let n = nb_ints as usize;
    let mut buf = vec![0u8; n * 4];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
        return;
    }
    for i in 0..n {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&buf[i * 4..(i + 1) * 4]);
        data[i] = i32::from_ne_bytes(bytes);
    }
}

pub fn trusted_utils_write_uls(data: &[u64], nb_uls: u64, file: &mut File) {
    let n = nb_uls as usize;
    let mut buf = Vec::with_capacity(n * 8);
    for i in 0..n {
        buf.extend_from_slice(&data[i].to_ne_bytes());
    }
    if file.write_all(&buf).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_realloc<T>(from: &mut [T], new_size: u64) -> Vec<T> {
    // Stub implementation: not used directly from Rust code
    let _ = from;
    Vec::with_capacity(new_size as usize)
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
    std::process::exit(0);
}

pub fn trusted_utils_write_char(c: char, file: &mut File) {
    let byte = c as u8;
    if file.write_all(&[byte]).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_calloc<T>(nb_objs: u64, size_per_obj: u64) -> Vec<T> {
    // Stub implementation: not used directly from Rust code
    let _ = size_per_obj;
    Vec::with_capacity(nb_objs as usize)
}

pub fn trusted_utils_read_bool(file: &mut File) -> bool {
    let mut buf = [0u8; 1];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
        return false;
    }
    buf[0] != 0
}

pub fn trusted_utils_read_int(file: &mut File) -> i32 {
    let mut buf = [0u8; 4];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
        return 0;
    }
    i32::from_ne_bytes(buf)
}

pub fn trusted_utils_str_to_sig(str: &str, out: &mut [u8]) -> bool {
    let bytes = str.as_bytes();
    if bytes.len() < SIG_SIZE_BYTES * 2 {
        return false;
    }
    for bytepos in 0..SIG_SIZE_BYTES {
        let hex1 = bytes[bytepos * 2];
        let hex2 = bytes[bytepos * 2 + 1];
        let v1: i32 = if hex1 >= b'0' && hex1 <= b'9' {
            (hex1 - b'0') as i32
        } else {
            10 + (hex1 as i32) - (b'a' as i32)
        };
        let v2: i32 = if hex2 >= b'0' && hex2 <= b'9' {
            (hex2 - b'0') as i32
        } else {
            10 + (hex2 as i32) - (b'a' as i32)
        };
        let byte = 16 * v1 + v2;
        if !(0..256).contains(&byte) {
            return false;
        }
        out[bytepos] = byte as u8;
    }
    true
}

pub fn trusted_utils_write_ul(u: u64, file: &mut File) {
    if file.write_all(&u.to_ne_bytes()).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_read_char(file: &mut File) -> i32 {
    let mut buf = [0u8; 1];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
        return 0;
    }
    buf[0] as i32
}
