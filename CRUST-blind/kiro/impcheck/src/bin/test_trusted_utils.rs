use impcheck::trusted_utils::*;

#[test]
fn test_sig_to_str_basic() {
    let sig: [u8; 16] = [0x00, 0x01, 0x0a, 0x0f, 0x10, 0xff, 0xab, 0xcd,
                          0xef, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde];
    let mut out = String::new();
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, "00010a0f10ffabcdef123456789abcde");
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
fn test_str_to_sig_roundtrip() {
    let mut sig = [0u8; 16];
    let ok = trusted_utils_str_to_sig("00010a0f10ffabcdef123456789abcde", &mut sig);
    assert!(ok);
    assert_eq!(sig, [0x00, 0x01, 0x0a, 0x0f, 0x10, 0xff, 0xab, 0xcd,
                      0xef, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde]);
}

#[test]
fn test_str_to_sig_then_sig_to_str() {
    let input = "00010a0f10ffabcdef123456789abcde";
    let mut sig = [0u8; 16];
    assert!(trusted_utils_str_to_sig(input, &mut sig));
    let mut out = String::new();
    trusted_utils_sig_to_str(&sig, &mut out);
    assert_eq!(out, input);
}

#[test]
fn test_equal_signatures_same() {
    let a: [u8; 16] = [0x00, 0x01, 0x0a, 0x0f, 0x10, 0xff, 0xab, 0xcd,
                        0xef, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde];
    assert!(trusted_utils_equal_signatures(&a, &a));
}

#[test]
fn test_equal_signatures_different() {
    let a: [u8; 16] = [0x00, 0x01, 0x0a, 0x0f, 0x10, 0xff, 0xab, 0xcd,
                        0xef, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde];
    let b = [0u8; 16];
    assert!(!trusted_utils_equal_signatures(&a, &b));
}

#[test]
fn test_copy_bytes() {
    let src: [u8; 16] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16];
    let mut dst = [0u8; 16];
    trusted_utils_copy_bytes(&mut dst, &src, 16);
    assert_eq!(dst, src);
}

#[test]
fn test_copy_bytes_partial() {
    let src: [u8; 16] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16];
    let mut dst = [0u8; 16];
    trusted_utils_copy_bytes(&mut dst, &src, 4);
    assert_eq!(&dst[..4], &[1,2,3,4]);
    assert_eq!(&dst[4..], &[0u8; 12]);
}

#[test]
fn test_try_match_arg_match() {
    let mut out: Option<&str> = None;
    trusted_utils_try_match_arg("-foo=bar", "-foo=", &mut out);
    assert_eq!(out, Some("bar"));
}

#[test]
fn test_try_match_arg_no_match() {
    let mut out: Option<&str> = None;
    trusted_utils_try_match_arg("-baz=qux", "-foo=", &mut out);
    assert_eq!(out, None);
}

#[test]
fn test_try_match_flag_match() {
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

fn main() {}
