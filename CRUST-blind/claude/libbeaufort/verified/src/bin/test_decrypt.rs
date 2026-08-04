use libbeaufort::decrypt::beaufort_decrypt;
use libbeaufort::tableau::beaufort_tableau;

#[test]
fn test_decrypt_monkey() {
    // C: dec1='kinkajous are awesome'
    let out = beaufort_decrypt(b"26004Fyuv AnK Cs9sqC8", b"monkey", &[]);
    assert_eq!(out, b"kinkajous are awesome");
}

#[test]
fn test_decrypt_goodman() {
    // C: dec2='the \nbig \nlebowski'
    let out = beaufort_decrypt(b"n7A \n24u \n22D0huq5", b"goodman", &[]);
    assert_eq!(out, b"the \nbig \nlebowski");
}

#[test]
fn test_decrypt_groove() {
    // C: dec3='d4nc3 t0 th3 mus!c :D'
    let out = beaufort_decrypt(b"3n1Cs lg y7l 9ko!F :b", b"groove", &[]);
    assert_eq!(out, b"d4nc3 t0 th3 mus!c :D");
}

#[test]
fn test_decrypt_empty() {
    let out = beaufort_decrypt(b"", b"key", &[]);
    assert_eq!(out, b"");
}

#[test]
fn test_decrypt_mixed_chars() {
    // C: dec_mixed='Jxrpn 6mkrx' for "Hello World", "abc"
    let out = beaufort_decrypt(b"Hello World", b"abc", &[]);
    assert_eq!(out, b"Jxrpn 6mkrx");
}

#[test]
fn test_decrypt_with_custom_tableau() {
    // C: dec_small='abcabc' for "aabbcc", "ab", abc tableau
    let mat_owned = beaufort_tableau("abc");
    let mat_refs: Vec<&[u8]> = mat_owned.iter().map(|r| r.as_slice()).collect();
    let out = beaufort_decrypt(b"aabbcc", b"ab", &mat_refs);
    assert_eq!(out, b"abcabc");
}

#[test]
fn test_decrypt_round_trip_key() {
    // C: rt_d='Hello, World!' for "T0Dzq, SwnD7!", "key"
    let out = beaufort_decrypt(b"T0Dzq, SwnD7!", b"key", &[]);
    assert_eq!(out, b"Hello, World!");
}

#[test]
fn test_decrypt_period() {
    // C: dec_period='abcdefghij' for "QQOOMMKKII", "01"
    let out = beaufort_decrypt(b"QQOOMMKKII", b"01", &[]);
    assert_eq!(out, b"abcdefghij");
}

#[test]
fn test_decrypt_digits() {
    // C: dec_digits='0123456789' for "9876543210", "9"
    let out = beaufort_decrypt(b"9876543210", b"9", &[]);
    assert_eq!(out, b"0123456789");
}

#[test]
fn test_decrypt_punct_only() {
    // chars not in alphabet should be passed through unchanged
    let out = beaufort_decrypt(b"!@#$%^", b"key", &[]);
    assert_eq!(out, b"!@#$%^");
}

#[test]
fn test_decrypt_invalid_key_chars_only() {
    // key="!" -> not in alphabet, src chars in alphabet but no key match
    let out = beaufort_decrypt(b"abc", b"!", &[]);
    assert_eq!(out, b"abc");
}

#[test]
fn test_decrypt_round_trip_long() {
    // Round-trip: encrypt then decrypt with same key returns original
    use libbeaufort::encrypt::beaufort_encrypt;
    let plain = b"The quick brown fox jumps over 13 lazy dogs.";
    let key = b"secret";
    let cipher = beaufort_encrypt(plain, key, &[]);
    let back = beaufort_decrypt(&cipher, key, &[]);
    assert_eq!(back, plain);
}

#[test]
fn test_decrypt_round_trip_with_custom_tableau() {
    use libbeaufort::encrypt::beaufort_encrypt;
    let mat_owned = beaufort_tableau("abcdef");
    let mat_refs: Vec<&[u8]> = mat_owned.iter().map(|r| r.as_slice()).collect();
    let plain = b"abcdef";
    let key = b"ab";
    let cipher = beaufort_encrypt(plain, key, &mat_refs);
    assert_eq!(cipher, b"aaeecc");
    let back = beaufort_decrypt(&cipher, key, &mat_refs);
    assert_eq!(back, plain);
}

#[test]
fn test_decrypt_round_trip_alt() {
    // Round-trip "ab!!cd" with key "xy"
    use libbeaufort::encrypt::beaufort_encrypt;
    let plain = b"ab!!cd";
    let key = b"xy";
    let cipher = beaufort_encrypt(plain, key, &[]);
    assert_eq!(cipher, b"NN!!LL");
    let back = beaufort_decrypt(&cipher, key, &[]);
    assert_eq!(back, plain);
}

fn main() {}
