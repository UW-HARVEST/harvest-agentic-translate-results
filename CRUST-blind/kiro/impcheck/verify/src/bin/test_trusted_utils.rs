use impcheck::trusted_utils::*;
use std::io::{Seek, SeekFrom};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_file() -> std::fs::File {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = format!("/tmp/test_tu_{}_{}.bin", std::process::id(), id);
    let f = std::fs::File::options().read(true).write(true).create(true).truncate(true).open(&path).unwrap();
    std::fs::remove_file(&path).ok();
    f
}

#[test]
fn test_sig_to_str() {
    let sig: [u8; 16] = [0x56, 0x5d, 0x01, 0xd1, 0x70, 0xb0, 0x0d, 0x28,
                          0xa8, 0xdf, 0x19, 0x16, 0x86, 0x3a, 0x15, 0xd3];
    let mut out = String::new();
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "565d01d170b00d28a8df1916863a15d3");
}

#[test]
fn test_str_to_sig_roundtrip() {
    let sig: [u8; 16] = [0x56, 0x5d, 0x01, 0xd1, 0x70, 0xb0, 0x0d, 0x28,
                          0xa8, 0xdf, 0x19, 0x16, 0x86, 0x3a, 0x15, 0xd3];
    let mut out_str = String::new();
    trusted_utils_sig_to_str(&sig, &mut out_str);
    let mut sig2 = [0u8; 16];
    let ok = trusted_utils_str_to_sig(&out_str, &mut sig2);
    assert!(ok);
    assert!(trusted_utils_equal_signatures(&sig, &sig2));
}

#[test]
fn test_str_to_sig_known() {
    let mut sig = [0u8; 16];
    let ok = trusted_utils_str_to_sig("00ff0a0b0c0d0e0f1011121314151617", &mut sig);
    assert!(ok);
    assert_eq!(sig, [0, 255, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23]);
}

#[test]
fn test_equal_signatures_same() {
    let sig: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    assert!(trusted_utils_equal_signatures(&sig, &sig));
}

#[test]
fn test_equal_signatures_diff() {
    let sig1: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let sig2: [u8; 16] = [0; 16];
    assert!(!trusted_utils_equal_signatures(&sig1, &sig2));
}

#[test]
fn test_copy_bytes() {
    let src: [u8; 16] = [0x56, 0x5d, 0x01, 0xd1, 0x70, 0xb0, 0x0d, 0x28,
                          0xa8, 0xdf, 0x19, 0x16, 0x86, 0x3a, 0x15, 0xd3];
    let mut dst = [0u8; 16];
    trusted_utils_copy_bytes(&mut dst, &src, 16);
    assert!(trusted_utils_equal_signatures(&dst, &src));
}

#[test]
fn test_try_match_arg() {
    let mut out: Option<&str> = None;
    trusted_utils_try_match_arg("-formula-input=/tmp/test.cnf", "-formula-input=", &mut out);
    assert_eq!(out, Some("/tmp/test.cnf"));
}

#[test]
fn test_try_match_arg_no_match() {
    let mut out: Option<&str> = None;
    trusted_utils_try_match_arg("-other=value", "-formula-input=", &mut out);
    assert_eq!(out, None);
}

#[test]
fn test_try_match_flag() {
    let mut flag = false;
    trusted_utils_try_match_flag("-check-model", "-check-model", &mut flag);
    assert!(flag);
}

#[test]
fn test_try_match_flag_no_match() {
    let mut flag = false;
    trusted_utils_try_match_flag("-other", "-check-model", &mut flag);
    assert!(!flag);
}

#[test]
fn test_sig_size_bytes() {
    assert_eq!(SIG_SIZE_BYTES, 16);
}

#[test]
fn test_max_buf_size() {
    assert_eq!(TRUSTED_CHK_MAX_BUF_SIZE, 1 << 14);
}

#[test]
fn test_sig_to_str_all_zeros() {
    let sig = [0u8; 16];
    let mut out = String::new();
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "00000000000000000000000000000000");
}

#[test]
fn test_sig_to_str_all_ff() {
    let sig = [0xffu8; 16];
    let mut out = String::new();
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "ffffffffffffffffffffffffffffffff");
}

#[test]
fn test_write_and_read_int() {
    let mut f = tmp_file();
    trusted_utils_write_int(42, &mut f);
    trusted_utils_write_int(-1, &mut f);
    f.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(trusted_utils_read_int(&mut f), 42);
    assert_eq!(trusted_utils_read_int(&mut f), -1);
}

#[test]
fn test_write_and_read_ul() {
    let mut f = tmp_file();
    trusted_utils_write_ul(123456789u64, &mut f);
    trusted_utils_write_ul(u64::MAX, &mut f);
    f.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(trusted_utils_read_ul(&mut f), 123456789u64);
    assert_eq!(trusted_utils_read_ul(&mut f), u64::MAX);
}

#[test]
fn test_write_and_read_bool() {
    let mut f = tmp_file();
    trusted_utils_write_bool(true, &mut f);
    trusted_utils_write_bool(false, &mut f);
    f.seek(SeekFrom::Start(0)).unwrap();
    assert!(trusted_utils_read_bool(&mut f));
    assert!(!trusted_utils_read_bool(&mut f));
}

#[test]
fn test_write_and_read_sig() {
    let sig: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut f = tmp_file();
    trusted_utils_write_sig(&sig, &mut f);
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut out = [0u8; 16];
    trusted_utils_read_sig(&mut out, &mut f);
    assert_eq!(out, sig);
}

#[test]
fn test_write_and_read_ints() {
    let data = [10i32, -20, 30, 0, -100];
    let mut f = tmp_file();
    trusted_utils_write_ints(&data, 5, &mut f);
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut out = [0i32; 5];
    trusted_utils_read_ints(&mut out, 5, &mut f);
    assert_eq!(out, data);
}

#[test]
fn test_write_and_read_uls() {
    let data = [100u64, 200, u64::MAX, 0];
    let mut f = tmp_file();
    trusted_utils_write_uls(&data, 4, &mut f);
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut out = [0u64; 4];
    trusted_utils_read_uls(&mut out, 4, &mut f);
    assert_eq!(out, data);
}

fn main() {}
