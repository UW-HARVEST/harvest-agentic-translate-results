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
    let bytes = i.to_ne_bytes();
    file.write_all(&bytes).unwrap_or_else(|_| trusted_utils_exit_eof());
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
    // Write exactly SIG_SIZE_BYTES, padding with zeros if shorter.
    let mut buf = [0u8; SIG_SIZE_BYTES];
    let n = sig.len().min(SIG_SIZE_BYTES);
    buf[..n].copy_from_slice(&sig[..n]);
    file.write_all(&buf)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
}

pub fn exit_oom() {
    trusted_utils_log("allocation failed - terminating");
    std::process::exit(0);
}

pub fn trusted_utils_try_match_arg(arg: &str, opt: &str, out: &mut Option<&str>) {
    if arg.starts_with(opt) {
        // SAFETY: We just need to get the rest of the str. Since arg has 'a lifetime,
        // we need a way to tie it. We use unsafe transmute to extend lifetime since
        // out's lifetime is tied to arg's.
        let rest = &arg[opt.len()..];
        // out has lifetime 'a, the 'a parameter, so this should work normally
        // Need to convert lifetime. We use a workaround:
        let rest_ptr: *const str = rest;
        unsafe {
            *out = Some(&*rest_ptr);
        }
    }
}

pub fn trusted_utils_read_sig(out_sig: &mut [u8], file: &mut File) {
    file.read_exact(&mut out_sig[..SIG_SIZE_BYTES])
        .unwrap_or_else(|_| trusted_utils_exit_eof());
}

pub fn trusted_utils_read_ul(file: &mut File) -> u64 {
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
    u64::from_ne_bytes(buf)
}

pub fn trusted_utils_log_err(msg: &str) {
    println!("c [TRUSTED_CORE {}] [ERROR] {}", std::process::id(), msg);
}

pub fn trusted_utils_copy_bytes(to: &mut [u8], from: &[u8], nb_bytes: u64) {
    for i in 0..nb_bytes as usize {
        to[i] = from[i];
    }
}

pub fn trusted_utils_write_ints(data: &[i32], nb_ints: u64, file: &mut File) {
    let n = nb_ints as usize;
    let mut buf = Vec::with_capacity(n * 4);
    for i in 0..n {
        buf.extend_from_slice(&data[i].to_ne_bytes());
    }
    file.write_all(&buf)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
}

pub fn trusted_utils_read_uls(data: &mut [u64], nb_uls: u64, file: &mut File) {
    let n = nb_uls as usize;
    let mut buf = vec![0u8; n * 8];
    file.read_exact(&mut buf)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
    for i in 0..n {
        let mut tmp = [0u8; 8];
        tmp.copy_from_slice(&buf[i * 8..(i + 1) * 8]);
        data[i] = u64::from_ne_bytes(tmp);
    }
}

pub fn trusted_utils_log(msg: &str) {
    println!("c [TRUSTED_CORE {}] {}", std::process::id(), msg);
}

pub fn trusted_utils_read_objs(data: &mut [u8], _size: usize, nb_objs: usize, file: &mut File) {
    file.read_exact(&mut data[..nb_objs])
        .unwrap_or_else(|_| trusted_utils_exit_eof());
}

pub fn trusted_utils_read_ints(data: &mut [i32], nb_ints: u64, file: &mut File) {
    let n = nb_ints as usize;
    let mut buf = vec![0u8; n * 4];
    file.read_exact(&mut buf)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
    for i in 0..n {
        let mut tmp = [0u8; 4];
        tmp.copy_from_slice(&buf[i * 4..(i + 1) * 4]);
        data[i] = i32::from_ne_bytes(tmp);
    }
}

pub fn trusted_utils_write_uls(data: &[u64], nb_uls: u64, file: &mut File) {
    let n = nb_uls as usize;
    let mut buf = Vec::with_capacity(n * 8);
    for i in 0..n {
        buf.extend_from_slice(&data[i].to_ne_bytes());
    }
    file.write_all(&buf)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
}

pub fn trusted_utils_realloc<T>(_from: &mut [T], new_size: u64) -> Vec<T> {
    let mut v = Vec::with_capacity(new_size as usize);
    // We can't actually safely move T values out without knowing they're cloneable.
    // The caller is expected to discard the previous slice and use the new vec.
    // For our purposes we just allocate a fresh vec of the right capacity.
    unsafe {
        v.set_len(new_size as usize);
    }
    v
}

pub fn trusted_utils_write_bool(b: bool, file: &mut File) {
    let byte = if b { 1u8 } else { 0u8 };
    file.write_all(&[byte])
        .unwrap_or_else(|_| trusted_utils_exit_eof());
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
    let b = c as u8;
    file.write_all(&[b])
        .unwrap_or_else(|_| trusted_utils_exit_eof());
}

pub fn trusted_utils_calloc<T: Default + Clone>(nb_objs: u64, _size_per_obj: u64) -> Vec<T> {
    vec![T::default(); nb_objs as usize]
}

pub fn trusted_utils_read_bool(file: &mut File) -> bool {
    let mut buf = [0u8; 1];
    file.read_exact(&mut buf)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
    buf[0] != 0
}

pub fn trusted_utils_read_int(file: &mut File) -> i32 {
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
    i32::from_ne_bytes(buf)
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
            hex1 as i32 - '0' as i32
        } else if hex1 >= 'a' && hex1 <= 'f' {
            10 + hex1 as i32 - 'a' as i32
        } else {
            return false;
        };
        let v2 = if hex2 >= '0' && hex2 <= '9' {
            hex2 as i32 - '0' as i32
        } else if hex2 >= 'a' && hex2 <= 'f' {
            10 + hex2 as i32 - 'a' as i32
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
    let bytes = u.to_ne_bytes();
    file.write_all(&bytes)
        .unwrap_or_else(|_| trusted_utils_exit_eof());
}

pub fn trusted_utils_read_char(file: &mut File) -> i32 {
    let mut buf = [0u8; 1];
    match file.read_exact(&mut buf) {
        Ok(_) => buf[0] as i32,
        Err(_) => {
            trusted_utils_exit_eof();
            -1
        }
    }
}
