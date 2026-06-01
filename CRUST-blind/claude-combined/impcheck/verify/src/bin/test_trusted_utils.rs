use impcheck::trusted_utils;

#[test]
fn test_sig_to_str_basic() {
    let mut sig = [0u8; 16];
    for i in 0..16 {
        sig[i] = (i as u8).wrapping_mul(17).wrapping_add(3);
    }
    let mut out = String::new();
    trusted_utils::trusted_utils_sig_to_str(&sig, &mut out);
    // Expected from running C
    assert_eq!(out, "031425364758697a8b9cadbecfe0f102");
}

#[test]
fn test_sig_to_str_zero() {
    let sig = [0u8; 16];
    let mut out = String::new();
    trusted_utils::trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "00000000000000000000000000000000");
}

#[test]
fn test_sig_to_str_all_ff() {
    let sig = [0xffu8; 16];
    let mut out = String::new();
    trusted_utils::trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "ffffffffffffffffffffffffffffffff");
}

#[test]
fn test_str_to_sig_roundtrip() {
    let mut sig = [0u8; 16];
    for i in 0..16 {
        sig[i] = (i as u8).wrapping_mul(17).wrapping_add(3);
    }
    let mut s = String::new();
    trusted_utils::trusted_utils_sig_to_str(&sig, &mut s);
    let mut back = [0u8; 16];
    let ok = trusted_utils::trusted_utils_str_to_sig(&s, &mut back);
    assert!(ok);
    assert_eq!(sig, back);
}

#[test]
fn test_str_to_sig_specific() {
    let s = "031425364758697a8b9cadbecfe0f102";
    let mut back = [0u8; 16];
    let ok = trusted_utils::trusted_utils_str_to_sig(s, &mut back);
    assert!(ok);
    let expected: [u8; 16] = [3, 20, 37, 54, 71, 88, 105, 122, 139, 156, 173, 190, 207, 224, 241, 2];
    assert_eq!(back, expected);
}

#[test]
fn test_equal_signatures_eq() {
    let a = [1u8; 16];
    let b = [1u8; 16];
    assert!(trusted_utils::trusted_utils_equal_signatures(&a, &b));
}

#[test]
fn test_equal_signatures_neq() {
    let mut a = [1u8; 16];
    let b = [1u8; 16];
    a[5] = 2;
    assert!(!trusted_utils::trusted_utils_equal_signatures(&a, &b));
}

#[test]
fn test_copy_bytes() {
    let mut to = [0u8; 8];
    let from = [10, 20, 30, 40, 50, 60, 70, 80u8];
    trusted_utils::trusted_utils_copy_bytes(&mut to, &from, 5);
    assert_eq!(&to, &[10, 20, 30, 40, 50, 0, 0, 0]);
}

#[test]
fn test_copy_bytes_full() {
    let mut to = [0u8; 8];
    let from = [10, 20, 30, 40, 50, 60, 70, 80u8];
    trusted_utils::trusted_utils_copy_bytes(&mut to, &from, 8);
    assert_eq!(&to, &from);
}

#[test]
fn test_try_match_flag_match() {
    let mut out = false;
    trusted_utils::trusted_utils_try_match_flag("-check-model", "-check-model", &mut out);
    assert_eq!(out, true);
}

#[test]
fn test_try_match_flag_no_match() {
    let mut out = false;
    trusted_utils::trusted_utils_try_match_flag("-other", "-check-model", &mut out);
    assert_eq!(out, false);
}

#[test]
fn test_try_match_arg() {
    let arg = String::from("-foo=bar");
    let mut out: Option<&str> = None;
    trusted_utils::trusted_utils_try_match_arg(&arg, "-foo=", &mut out);
    assert_eq!(out, Some("bar"));
}

#[test]
fn test_try_match_arg_no_match() {
    let arg = String::from("-other=stuff");
    let mut out: Option<&str> = None;
    trusted_utils::trusted_utils_try_match_arg(&arg, "-foo=", &mut out);
    assert_eq!(out, None);
}

#[test]
fn test_constants() {
    assert_eq!(trusted_utils::SIG_SIZE_BYTES, 16);
    assert_eq!(trusted_utils::TRUSTED_CHK_MAX_BUF_SIZE, 1 << 14);
}

#[test]
fn test_calloc() {
    let v: Vec<i32> = trusted_utils::trusted_utils_calloc(5, 4);
    assert_eq!(v.len(), 5);
    for x in &v {
        assert_eq!(*x, 0);
    }
}

#[test]
fn test_read_write_int() {
    use std::fs::File;

    let path = std::env::temp_dir().join("test_ru_int.bin");
    let _ = std::fs::remove_file(&path);
    let mut f = File::create(&path).unwrap();
    trusted_utils::trusted_utils_write_int(42, &mut f);
    trusted_utils::trusted_utils_write_int(-7, &mut f);
    drop(f);
    let mut f = File::open(&path).unwrap();
    let a = trusted_utils::trusted_utils_read_int(&mut f);
    let b = trusted_utils::trusted_utils_read_int(&mut f);
    assert_eq!(a, 42);
    assert_eq!(b, -7);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_write_ul() {
    use std::fs::File;
    let path = std::env::temp_dir().join("test_ru_ul.bin");
    let _ = std::fs::remove_file(&path);
    let mut f = File::create(&path).unwrap();
    trusted_utils::trusted_utils_write_ul(0xDEADBEEF12345678, &mut f);
    trusted_utils::trusted_utils_write_ul(0, &mut f);
    drop(f);
    let mut f = File::open(&path).unwrap();
    let a = trusted_utils::trusted_utils_read_ul(&mut f);
    let b = trusted_utils::trusted_utils_read_ul(&mut f);
    assert_eq!(a, 0xDEADBEEF12345678);
    assert_eq!(b, 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_write_bool() {
    use std::fs::File;
    let path = std::env::temp_dir().join("test_ru_bool.bin");
    let _ = std::fs::remove_file(&path);
    let mut f = File::create(&path).unwrap();
    trusted_utils::trusted_utils_write_bool(true, &mut f);
    trusted_utils::trusted_utils_write_bool(false, &mut f);
    drop(f);
    let mut f = File::open(&path).unwrap();
    let a = trusted_utils::trusted_utils_read_bool(&mut f);
    let b = trusted_utils::trusted_utils_read_bool(&mut f);
    assert_eq!(a, true);
    assert_eq!(b, false);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_write_ints() {
    use std::fs::File;
    let path = std::env::temp_dir().join("test_ru_ints.bin");
    let _ = std::fs::remove_file(&path);
    let mut f = File::create(&path).unwrap();
    let data: [i32; 4] = [10, -20, 30, -40];
    trusted_utils::trusted_utils_write_ints(&data, 4, &mut f);
    drop(f);
    let mut f = File::open(&path).unwrap();
    let mut out = [0i32; 4];
    trusted_utils::trusted_utils_read_ints(&mut out, 4, &mut f);
    assert_eq!(out, data);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_write_uls() {
    use std::fs::File;
    let path = std::env::temp_dir().join("test_ru_uls.bin");
    let _ = std::fs::remove_file(&path);
    let mut f = File::create(&path).unwrap();
    let data: [u64; 3] = [1, 1u64 << 40, 0xFFFF_FFFF_FFFF_FFFF];
    trusted_utils::trusted_utils_write_uls(&data, 3, &mut f);
    drop(f);
    let mut f = File::open(&path).unwrap();
    let mut out = [0u64; 3];
    trusted_utils::trusted_utils_read_uls(&mut out, 3, &mut f);
    assert_eq!(out, data);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_write_sig() {
    use std::fs::File;
    let path = std::env::temp_dir().join("test_ru_sig.bin");
    let _ = std::fs::remove_file(&path);
    let mut f = File::create(&path).unwrap();
    let mut sig = [0u8; 16];
    for i in 0..16 {
        sig[i] = (i as u8) * 11 + 1;
    }
    trusted_utils::trusted_utils_write_sig(&sig, &mut f);
    drop(f);
    let mut f = File::open(&path).unwrap();
    let mut out = [0u8; 16];
    trusted_utils::trusted_utils_read_sig(&mut out, &mut f);
    assert_eq!(out, sig);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_write_char() {
    use std::fs::File;
    let path = std::env::temp_dir().join("test_ru_char.bin");
    let _ = std::fs::remove_file(&path);
    let mut f = File::create(&path).unwrap();
    trusted_utils::trusted_utils_write_char('A', &mut f);
    trusted_utils::trusted_utils_write_char('B', &mut f);
    drop(f);
    let mut f = File::open(&path).unwrap();
    let a = trusted_utils::trusted_utils_read_char(&mut f);
    let b = trusted_utils::trusted_utils_read_char(&mut f);
    assert_eq!(a, 'A' as i32);
    assert_eq!(b, 'B' as i32);
    let _ = std::fs::remove_file(&path);
}

fn main() {}
