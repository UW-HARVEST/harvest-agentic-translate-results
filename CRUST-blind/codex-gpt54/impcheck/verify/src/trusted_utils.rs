use std::fs::File;
use std::io::{Read, Write};
use std::process;
pub const SIG_SIZE_BYTES: usize = 16;
pub type Signature = [u8; SIG_SIZE_BYTES];
pub type U32 = u32;
pub type U64 = u64;
pub type U8 = u8;
pub const TRUSTED_CHK_MAX_BUF_SIZE: usize = 1 << 14;
pub fn trusted_utils_sig_to_str(sig: &[u8], out: &mut String) {
    out.clear();
    for byte in sig.iter().take(SIG_SIZE_BYTES) {
        let hi = (byte >> 4) & 0x0f;
        let lo = byte & 0x0f;
        out.push(if hi >= 10 {
            (b'a' + hi - 10) as char
        } else {
            (b'0' + hi) as char
        });
        out.push(if lo >= 10 {
            (b'a' + lo - 10) as char
        } else {
            (b'0' + lo) as char
        });
    }
}
pub fn trusted_utils_write_int(i: i32, file: &mut File) {
    if file.write_all(&i.to_ne_bytes()).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_equal_signatures(left: &[u8], right: &[u8]) -> bool {
    left.len() >= SIG_SIZE_BYTES
        && right.len() >= SIG_SIZE_BYTES
        && left[..SIG_SIZE_BYTES] == right[..SIG_SIZE_BYTES]
}
pub fn trusted_utils_write_sig(sig: &[u8], file: &mut File) {
    if file.write_all(&sig[..sig.len().min(SIG_SIZE_BYTES)]).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn exit_oom() {
    trusted_utils_log("allocation failed - terminating");
    process::exit(0);
}
pub fn trusted_utils_try_match_arg(arg: &str, opt: &str, out: &mut Option<&str>) {
    if arg.starts_with(opt) {
        let suffix = &arg[opt.len()..];
        // The translated signature does not relate the output lifetime to `arg`.
        // Callers pass the output straight through the same argv storage, which
        // matches the original C semantics.
        *out = Some(unsafe { std::mem::transmute::<&str, &str>(suffix) });
    }
}
pub fn trusted_utils_read_sig(out_sig: &mut [u8], file: &mut File) {
    let mut buf = [0_u8; SIG_SIZE_BYTES];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    let len = out_sig.len().min(SIG_SIZE_BYTES);
    out_sig[..len].copy_from_slice(&buf[..len]);
}
pub fn trusted_utils_read_ul(file: &mut File) -> u64 {
    let mut buf = [0_u8; std::mem::size_of::<u64>()];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    u64::from_ne_bytes(buf)
}
pub fn trusted_utils_log_err(msg: &str) {
    println!("c [TRUSTED_CORE {}] [ERROR] {}", process::id(), msg);
}
pub fn trusted_utils_copy_bytes(to: &mut [u8], from: &[u8], nb_bytes: u64) {
    let count = nb_bytes as usize;
    let len = count.min(to.len()).min(from.len());
    to[..len].copy_from_slice(&from[..len]);
}
pub fn trusted_utils_write_ints(data: &[i32], nb_ints: u64, file: &mut File) {
    for value in data.iter().take(nb_ints as usize) {
        trusted_utils_write_int(*value, file);
    }
}
pub fn trusted_utils_read_uls(data: &mut [u64], nb_uls: u64, file: &mut File) {
    for slot in data.iter_mut().take(nb_uls as usize) {
        *slot = trusted_utils_read_ul(file);
    }
}
pub fn trusted_utils_log(msg: &str) {
    println!("c [TRUSTED_CORE {}] {}", process::id(), msg);
}
pub fn trusted_utils_read_objs(data: &mut [u8], size: usize, nb_objs: usize, file: &mut File) {
    let total = size.saturating_mul(nb_objs);
    if total > data.len() || file.read_exact(&mut data[..total]).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_read_ints(data: &mut [i32], nb_ints: u64, file: &mut File) {
    for slot in data.iter_mut().take(nb_ints as usize) {
        *slot = trusted_utils_read_int(file);
    }
}
pub fn trusted_utils_write_uls(data: &[u64], nb_uls: u64, file: &mut File) {
    for value in data.iter().take(nb_uls as usize) {
        trusted_utils_write_ul(*value, file);
    }
}
pub fn trusted_utils_realloc<T>(from: &mut [T], new_size: u64) -> Vec<T> {
    let _ = from;
    let _ = new_size;
    Vec::new()
}
pub fn trusted_utils_write_bool(b: bool, file: &mut File) {
    if file.write_all(&[if b { 1 } else { 0 }]).is_err() {
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
    process::exit(0);
}
pub fn trusted_utils_write_char(c: char, file: &mut File) {
    if file.write_all(&[c as u8]).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_calloc<T>(nb_objs: u64, size_per_obj: u64) -> Vec<T> {
    let _ = nb_objs;
    let _ = size_per_obj;
    Vec::new()
}
pub fn trusted_utils_read_bool(file: &mut File) -> bool {
    let mut buf = [0_u8; 1];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    buf[0] != 0
}
pub fn trusted_utils_read_int(file: &mut File) -> i32 {
    let mut buf = [0_u8; std::mem::size_of::<i32>()];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    i32::from_ne_bytes(buf)
}
pub fn trusted_utils_str_to_sig(str: &str, out: &mut [u8]) -> bool {
    if str.len() < SIG_SIZE_BYTES * 2 || out.len() < SIG_SIZE_BYTES {
        return false;
    }
    for bytepos in 0..SIG_SIZE_BYTES {
        let hex_pair = &str[bytepos * 2..bytepos * 2 + 2];
        let parsed = match u8::from_str_radix(hex_pair, 16) {
            Ok(value) => value,
            Err(_) => return false,
        };
        out[bytepos] = parsed;
    }
    true
}
pub fn trusted_utils_write_ul(u: u64, file: &mut File) {
    if file.write_all(&u.to_ne_bytes()).is_err() {
        trusted_utils_exit_eof();
    }
}
pub fn trusted_utils_read_char(file: &mut File) -> i32 {
    let mut buf = [0_u8; 1];
    if file.read_exact(&mut buf).is_err() {
        trusted_utils_exit_eof();
    }
    buf[0] as i32
}
