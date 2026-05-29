use impcheck::trusted_utils::*;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[test]
fn test_constants() {
    assert_eq!(SIG_SIZE_BYTES, 16);
    assert_eq!(TRUSTED_CHK_MAX_BUF_SIZE, 1 << 14);
}

#[test]
fn test_sig_to_str_zeros() {
    let sig = [0u8; SIG_SIZE_BYTES];
    let mut out = String::new();
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "00000000000000000000000000000000");
}

#[test]
fn test_sig_to_str_ff() {
    let sig = [0xffu8; SIG_SIZE_BYTES];
    let mut out = String::new();
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "ffffffffffffffffffffffffffffffff");
}

#[test]
fn test_sig_to_str_sequential() {
    let mut sig = [0u8; SIG_SIZE_BYTES];
    for i in 0..SIG_SIZE_BYTES {
        sig[i] = i as u8;
    }
    let mut out = String::new();
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "000102030405060708090a0b0c0d0e0f");
}

#[test]
fn test_sig_to_str_clears_existing() {
    let sig = [0u8; SIG_SIZE_BYTES];
    let mut out = String::from("LEFTOVER");
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "00000000000000000000000000000000");
}

#[test]
fn test_str_to_sig_roundtrip() {
    let mut sig_in = [0u8; SIG_SIZE_BYTES];
    for i in 0..SIG_SIZE_BYTES {
        sig_in[i] = (i * 17) as u8;
    }
    let mut s = String::new();
    trusted_utils_sig_to_str(&sig_in, &mut s);

    let mut sig_out = [0u8; SIG_SIZE_BYTES];
    let ok = trusted_utils_str_to_sig(&s, &mut sig_out);
    assert!(ok);
    assert_eq!(sig_in, sig_out);
}

#[test]
fn test_str_to_sig_known_value() {
    let mut sig_out = [0u8; SIG_SIZE_BYTES];
    let ok = trusted_utils_str_to_sig("000102030405060708090a0b0c0d0e0f", &mut sig_out);
    assert!(ok);
    let expected: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf];
    assert_eq!(sig_out, expected);
}

#[test]
fn test_str_to_sig_too_short() {
    let mut sig_out = [0u8; SIG_SIZE_BYTES];
    let ok = trusted_utils_str_to_sig("abcd", &mut sig_out);
    assert!(!ok);
}

#[test]
fn test_str_to_sig_invalid_hex_returns_false() {
    let mut sig_out = [0u8; SIG_SIZE_BYTES];
    // 'z' is not a valid hex char.
    let ok = trusted_utils_str_to_sig("z000000000000000000000000000000z", &mut sig_out);
    assert!(!ok);
}

#[test]
fn test_equal_signatures_identical() {
    let a = [1u8; SIG_SIZE_BYTES];
    let b = [1u8; SIG_SIZE_BYTES];
    assert!(trusted_utils_equal_signatures(&a, &b));
}

#[test]
fn test_equal_signatures_different() {
    let a = [1u8; SIG_SIZE_BYTES];
    let mut b = [1u8; SIG_SIZE_BYTES];
    b[5] = 2;
    assert!(!trusted_utils_equal_signatures(&a, &b));
}

#[test]
fn test_equal_signatures_first_byte_differs() {
    let a = [0u8; SIG_SIZE_BYTES];
    let mut b = [0u8; SIG_SIZE_BYTES];
    b[0] = 1;
    assert!(!trusted_utils_equal_signatures(&a, &b));
}

#[test]
fn test_equal_signatures_last_byte_differs() {
    let a = [0u8; SIG_SIZE_BYTES];
    let mut b = [0u8; SIG_SIZE_BYTES];
    b[SIG_SIZE_BYTES - 1] = 1;
    assert!(!trusted_utils_equal_signatures(&a, &b));
}

#[test]
fn test_copy_bytes_full() {
    let from = [0xa, 0xb, 0xc, 0xd, 0xe];
    let mut to = [0u8; 5];
    trusted_utils_copy_bytes(&mut to, &from, 5);
    assert_eq!(to, [0xa, 0xb, 0xc, 0xd, 0xe]);
}

#[test]
fn test_copy_bytes_partial() {
    let from = [1u8, 2, 3, 4, 5];
    let mut to = [0u8; 5];
    trusted_utils_copy_bytes(&mut to, &from, 3);
    assert_eq!(to, [1, 2, 3, 0, 0]);
}

#[test]
fn test_copy_bytes_zero() {
    let from = [1u8, 2, 3];
    let mut to = [9u8, 9, 9];
    trusted_utils_copy_bytes(&mut to, &from, 0);
    assert_eq!(to, [9, 9, 9]);
}

#[test]
fn test_try_match_flag_match() {
    let mut flag = false;
    trusted_utils_try_match_flag("--verbose", "--verbose", &mut flag);
    assert!(flag);
}

#[test]
fn test_try_match_flag_prefix_match() {
    // C: begins_with returns true if str starts with prefix
    let mut flag = false;
    trusted_utils_try_match_flag("--verboseExtra", "--verbose", &mut flag);
    assert!(flag);
}

#[test]
fn test_try_match_flag_no_match() {
    let mut flag = false;
    trusted_utils_try_match_flag("--quiet", "--verbose", &mut flag);
    assert!(!flag);
}

#[test]
fn test_try_match_flag_empty_prefix() {
    let mut flag = false;
    trusted_utils_try_match_flag("anything", "", &mut flag);
    assert!(flag);
}

#[test]
fn test_try_match_arg_match() {
    let arg = "--out=value123";
    let mut out: Option<&str> = None;
    trusted_utils_try_match_arg(arg, "--out=", &mut out);
    assert_eq!(out, Some("value123"));
}

#[test]
fn test_try_match_arg_no_match() {
    let arg = "--no-out";
    let mut out: Option<&str> = None;
    trusted_utils_try_match_arg(arg, "--out=", &mut out);
    assert!(out.is_none());
}

#[test]
fn test_try_match_arg_exact_prefix_no_value() {
    let arg = "--out=";
    let mut out: Option<&str> = None;
    trusted_utils_try_match_arg(arg, "--out=", &mut out);
    assert_eq!(out, Some(""));
}

#[test]
fn test_calloc_default_initialized() {
    let v: Vec<i32> = trusted_utils_calloc(5, 4);
    assert_eq!(v.len(), 5);
    for x in v {
        assert_eq!(x, 0);
    }
}

#[test]
fn test_calloc_zero() {
    let v: Vec<i32> = trusted_utils_calloc(0, 4);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_realloc_grow() {
    let mut from: Vec<i32> = vec![1, 2, 3];
    let new = trusted_utils_realloc(&mut from, 5);
    assert_eq!(new.len(), 5);
    assert_eq!(new[0], 1);
    assert_eq!(new[1], 2);
    assert_eq!(new[2], 3);
    assert_eq!(new[3], 0);
    assert_eq!(new[4], 0);
}

#[test]
fn test_realloc_shrink() {
    let mut from: Vec<i32> = vec![1, 2, 3, 4, 5];
    let new = trusted_utils_realloc(&mut from, 3);
    assert_eq!(new.len(), 3);
    assert_eq!(new[0], 1);
    assert_eq!(new[1], 2);
    assert_eq!(new[2], 3);
}

#[test]
fn test_realloc_zero() {
    let mut from: Vec<i32> = vec![1, 2, 3];
    let new = trusted_utils_realloc(&mut from, 0);
    assert_eq!(new.len(), 0);
}

// ---- File I/O tests ----

fn tmp_path(name: &str) -> String {
    format!("{}/impcheck_test_{}_{}", std::env::temp_dir().display(), std::process::id(), name)
}

#[test]
fn test_write_int_then_read_int() {
    let path = tmp_path("write_int");
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_int(0x12345678, &mut f);
    }
    {
        let mut f = File::open(&path).unwrap();
        let v = trusted_utils_read_int(&mut f);
        assert_eq!(v, 0x12345678);
    }
    // Also verify on-disk bytes are little-endian.
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, vec![0x78, 0x56, 0x34, 0x12]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_int_negative() {
    let path = tmp_path("write_int_neg");
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_int(-1, &mut f);
    }
    {
        let mut f = File::open(&path).unwrap();
        let v = trusted_utils_read_int(&mut f);
        assert_eq!(v, -1);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, vec![0xff, 0xff, 0xff, 0xff]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_ul_roundtrip() {
    let path = tmp_path("write_ul");
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_ul(0x0123456789abcdefu64, &mut f);
    }
    {
        let mut f = File::open(&path).unwrap();
        let v = trusted_utils_read_ul(&mut f);
        assert_eq!(v, 0x0123456789abcdefu64);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        bytes,
        vec![0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_bool_roundtrip() {
    let path = tmp_path("write_bool");
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_bool(true, &mut f);
        trusted_utils_write_bool(false, &mut f);
    }
    {
        let mut f = File::open(&path).unwrap();
        assert_eq!(trusted_utils_read_bool(&mut f), true);
        assert_eq!(trusted_utils_read_bool(&mut f), false);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, vec![1, 0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_char_roundtrip() {
    let path = tmp_path("write_char");
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_char('A', &mut f);
        trusted_utils_write_char('z', &mut f);
    }
    {
        let mut f = File::open(&path).unwrap();
        assert_eq!(trusted_utils_read_char(&mut f), 'A' as i32);
        assert_eq!(trusted_utils_read_char(&mut f), 'z' as i32);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, vec![b'A', b'z']);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_ints_roundtrip() {
    let path = tmp_path("write_ints");
    let data = [1i32, -2, 0x12345678, -0x12345678, 0];
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_ints(&data, 5, &mut f);
    }
    {
        let mut f = File::open(&path).unwrap();
        let mut out = [0i32; 5];
        trusted_utils_read_ints(&mut out, 5, &mut f);
        assert_eq!(out, data);
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_uls_roundtrip() {
    let path = tmp_path("write_uls");
    let data = [0u64, 1, 0xdeadbeefu64, u64::MAX, 12345];
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_uls(&data, 5, &mut f);
    }
    {
        let mut f = File::open(&path).unwrap();
        let mut out = [0u64; 5];
        trusted_utils_read_uls(&mut out, 5, &mut f);
        assert_eq!(out, data);
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_sig_roundtrip() {
    let path = tmp_path("write_sig");
    let mut sig = [0u8; SIG_SIZE_BYTES];
    for i in 0..SIG_SIZE_BYTES {
        sig[i] = (i * 13 + 7) as u8;
    }
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_sig(&sig, &mut f);
    }
    {
        let mut f = File::open(&path).unwrap();
        let mut out = [0u8; SIG_SIZE_BYTES];
        trusted_utils_read_sig(&mut out, &mut f);
        assert_eq!(out, sig);
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_objs_basic() {
    let path = tmp_path("read_objs");
    {
        let mut f = File::create(&path).unwrap();
        let bytes = [0x10u8, 0x11, 0x12, 0x13];
        std::io::Write::write_all(&mut f, &bytes).unwrap();
    }
    {
        let mut f = File::open(&path).unwrap();
        let mut buf = [0u8; 4];
        trusted_utils_read_objs(&mut buf, 1, 4, &mut f);
        assert_eq!(buf, [0x10, 0x11, 0x12, 0x13]);
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_int_then_seek_re_read() {
    let path = tmp_path("write_int_seek");
    {
        let mut f = File::create(&path).unwrap();
        trusted_utils_write_int(42, &mut f);
        trusted_utils_write_int(99, &mut f);
    }
    let mut f = File::open(&path).unwrap();
    let mut buf = [0u8; 8];
    f.read_exact(&mut buf).unwrap();
    let a = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let b = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    assert_eq!(a, 42);
    assert_eq!(b, 99);
    f.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(trusted_utils_read_int(&mut f), 42);
    assert_eq!(trusted_utils_read_int(&mut f), 99);
    let _ = std::fs::remove_file(&path);
}

fn main() {}
