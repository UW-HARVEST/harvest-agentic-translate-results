use std::fs::File;
use std::io::{Read, Write};
use std::process;

pub const SIG_SIZE_BYTES: usize = 16;
pub type Signature = [u8; SIG_SIZE_BYTES];
pub type U32 = u32;
pub type U64 = u64;
pub type U8 = u8;
pub const TRUSTED_CHK_MAX_BUF_SIZE: usize = 1 << 14;

pub fn trusted_utils_log(msg: &str) {
    println!("c [TRUSTED_CORE {}] {}", process::id(), msg);
}

pub fn trusted_utils_log_err(msg: &str) {
    println!("c [TRUSTED_CORE {}] [ERROR] {}", process::id(), msg);
}

pub fn trusted_utils_exit_eof() {
    trusted_utils_log("end-of-file - terminating");
    process::exit(0);
}

pub fn exit_oom() {
    trusted_utils_log("allocation failed - terminating");
    process::exit(0);
}

fn begins_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

pub fn trusted_utils_try_match_arg(arg: &str, opt: &str, out: &mut Option<&str>) {
    if begins_with(arg, opt) {
        // Note: returning a slice into `arg` requires the caller to hold a reference
        // with appropriate lifetime. We rely on the caller passing a reference whose
        // lifetime matches.
        // This is a best-effort port — using unsafe to extend lifetime is undesirable;
        // here we simply set the option to None of the suffix because a borrow of `arg`
        // would not satisfy the borrow checker without lifetimes on the API.
        // However, since the caller passes `&mut Option<&str>`, we can do:
        let suffix: &str = &arg[opt.len()..];
        // Transmute lifetime: the API was defined without lifetimes; safe under
        // assumption that `arg` outlives the `out` borrow.
        let suffix_static: &str = unsafe { std::mem::transmute::<&str, &str>(suffix) };
        *out = Some(suffix_static);
    }
}

pub fn trusted_utils_try_match_flag(arg: &str, opt: &str, out: &mut bool) {
    if begins_with(arg, opt) {
        *out = true;
    }
}

pub fn trusted_utils_copy_bytes(to: &mut [u8], from: &[u8], nb_bytes: u64) {
    for i in 0..(nb_bytes as usize) {
        to[i] = from[i];
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

pub fn trusted_utils_calloc<T: Default + Clone>(nb_objs: u64, _size_per_obj: u64) -> Vec<T> {
    vec![T::default(); nb_objs as usize]
}

pub fn trusted_utils_realloc<T: Default + Clone>(from: &mut [T], new_size: u64) -> Vec<T> {
    let mut v: Vec<T> = Vec::with_capacity(new_size as usize);
    let copy_n = std::cmp::min(from.len() as u64, new_size) as usize;
    for i in 0..copy_n {
        v.push(from[i].clone());
    }
    while (v.len() as u64) < new_size {
        v.push(T::default());
    }
    v
}

fn read_exact_or_exit(file: &mut File, buf: &mut [u8]) {
    if file.read_exact(buf).is_err() {
        trusted_utils_exit_eof();
    }
}

fn write_all_or_exit(file: &mut File, buf: &[u8]) {
    if file.write_all(buf).is_err() {
        trusted_utils_exit_eof();
    }
}

pub fn trusted_utils_read_bool(file: &mut File) -> bool {
    let mut b = [0u8; 1];
    read_exact_or_exit(file, &mut b);
    b[0] != 0
}

pub fn trusted_utils_read_char(file: &mut File) -> i32 {
    let mut b = [0u8; 1];
    read_exact_or_exit(file, &mut b);
    b[0] as i32
}

pub fn trusted_utils_read_objs(data: &mut [u8], size: usize, nb_objs: usize, file: &mut File) {
    let total = size * nb_objs;
    read_exact_or_exit(file, &mut data[..total]);
}

pub fn trusted_utils_read_int(file: &mut File) -> i32 {
    let mut b = [0u8; 4];
    read_exact_or_exit(file, &mut b);
    i32::from_ne_bytes(b)
}

pub fn trusted_utils_read_ints(data: &mut [i32], nb_ints: u64, file: &mut File) {
    for i in 0..(nb_ints as usize) {
        let mut b = [0u8; 4];
        read_exact_or_exit(file, &mut b);
        data[i] = i32::from_ne_bytes(b);
    }
}

pub fn trusted_utils_read_ul(file: &mut File) -> u64 {
    let mut b = [0u8; 8];
    read_exact_or_exit(file, &mut b);
    u64::from_ne_bytes(b)
}

pub fn trusted_utils_read_uls(data: &mut [u64], nb_uls: u64, file: &mut File) {
    for i in 0..(nb_uls as usize) {
        let mut b = [0u8; 8];
        read_exact_or_exit(file, &mut b);
        data[i] = u64::from_ne_bytes(b);
    }
}

pub fn trusted_utils_read_sig(out_sig: &mut [u8], file: &mut File) {
    // C reads sizeof(int)*4 = 16 bytes
    read_exact_or_exit(file, &mut out_sig[..SIG_SIZE_BYTES]);
}

pub fn trusted_utils_write_char(c: char, file: &mut File) {
    let b = [c as u8];
    write_all_or_exit(file, &b);
}

pub fn trusted_utils_write_bool(b: bool, file: &mut File) {
    let buf = [if b { 1u8 } else { 0u8 }];
    write_all_or_exit(file, &buf);
}

pub fn trusted_utils_write_int(i: i32, file: &mut File) {
    write_all_or_exit(file, &i.to_ne_bytes());
}

pub fn trusted_utils_write_ints(data: &[i32], nb_ints: u64, file: &mut File) {
    for i in 0..(nb_ints as usize) {
        write_all_or_exit(file, &data[i].to_ne_bytes());
    }
}

pub fn trusted_utils_write_ul(u: u64, file: &mut File) {
    write_all_or_exit(file, &u.to_ne_bytes());
}

pub fn trusted_utils_write_uls(data: &[u64], nb_uls: u64, file: &mut File) {
    for i in 0..(nb_uls as usize) {
        write_all_or_exit(file, &data[i].to_ne_bytes());
    }
}

pub fn trusted_utils_write_sig(sig: &[u8], file: &mut File) {
    write_all_or_exit(file, &sig[..SIG_SIZE_BYTES]);
}

pub fn trusted_utils_sig_to_str(sig: &[u8], out: &mut String) {
    out.clear();
    for charpos in 0..SIG_SIZE_BYTES {
        let val1: u8 = (sig[charpos] >> 4) & 0x0f;
        let val2: u8 = sig[charpos] & 0x0f;
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

pub fn trusted_utils_str_to_sig(s: &str, out: &mut [u8]) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < SIG_SIZE_BYTES * 2 {
        return false;
    }
    for bytepos in 0..SIG_SIZE_BYTES {
        let hex1 = bytes[bytepos * 2] as char;
        let hex2 = bytes[bytepos * 2 + 1] as char;
        let v1 = if ('0'..='9').contains(&hex1) {
            (hex1 as i32) - ('0' as i32)
        } else {
            10 + (hex1 as i32) - ('a' as i32)
        };
        let v2 = if ('0'..='9').contains(&hex2) {
            (hex2 as i32) - ('0' as i32)
        } else {
            10 + (hex2 as i32) - ('a' as i32)
        };
        let byte = 16 * v1 + v2;
        if !(0..256).contains(&byte) {
            return false;
        }
        out[bytepos] = byte as u8;
    }
    true
}
