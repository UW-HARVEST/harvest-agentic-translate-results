use libbeaufort::encrypt::beaufort_encrypt;
use libbeaufort::tableau::beaufort_tableau;

#[test]
fn test_encrypt_monkey() {
    // C test: kinkajous are awesome with key "monkey"
    let out = beaufort_encrypt(b"kinkajous are awesome", b"monkey", &[]);
    assert_eq!(out, b"26004Fyuv AnK Cs9sqC8");
}

#[test]
fn test_encrypt_goodman() {
    let out = beaufort_encrypt(b"the \nbig \nlebowski", b"goodman", &[]);
    assert_eq!(out, b"n7A \n24u \n22D0huq5");
}

#[test]
fn test_encrypt_groove() {
    let out = beaufort_encrypt(b"d4nc3 t0 th3 mus!c :D", b"groove", &[]);
    assert_eq!(out, b"3n1Cs lg y7l 9ko!F :b");
}

#[test]
fn test_encrypt_empty() {
    let out = beaufort_encrypt(b"", b"key", &[]);
    assert_eq!(out, b"");
}

#[test]
fn test_encrypt_short_key() {
    // C: enc_short_key='36zzw'
    let out = beaufort_encrypt(b"hello", b"k", &[]);
    assert_eq!(out, b"36zzw");
}

#[test]
fn test_encrypt_punct_only() {
    // C: enc_punct='!@#$%^'
    let out = beaufort_encrypt(b"!@#$%^", b"key", &[]);
    assert_eq!(out, b"!@#$%^");
}

#[test]
fn test_encrypt_mixed_chars() {
    // C: enc_mixed='Jxrpn 6mkrx' for "Hello World", "abc"
    let out = beaufort_encrypt(b"Hello World", b"abc", &[]);
    assert_eq!(out, b"Jxrpn 6mkrx");
}

#[test]
fn test_encrypt_with_custom_tableau() {
    // C: enc_small='aabbcc' for "abcabc", "ab" with abc tableau
    let mat_owned = beaufort_tableau("abc");
    let mat_refs: Vec<&[u8]> = mat_owned.iter().map(|r| r.as_slice()).collect();
    let out = beaufort_encrypt(b"abcabc", b"ab", &mat_refs);
    assert_eq!(out, b"aabbcc");
}

#[test]
fn test_encrypt_round_trip_key() {
    // C: rt_e='T0Dzq, SwnD7!' for "Hello, World!", "key"
    let out = beaufort_encrypt(b"Hello, World!", b"key", &[]);
    assert_eq!(out, b"T0Dzq, SwnD7!");
}

#[test]
fn test_encrypt_single_char_a_b() {
    // C: enc_A_B='1'
    let out = beaufort_encrypt(b"A", b"B", &[]);
    assert_eq!(out, b"1");
}

#[test]
fn test_encrypt_single_char_0_0() {
    // C: enc_0_0='0'
    let out = beaufort_encrypt(b"0", b"0", &[]);
    assert_eq!(out, b"0");
}

#[test]
fn test_encrypt_space_key() {
    // C: enc_space_key='Hello' -- key " " is not in the alphabet
    let out = beaufort_encrypt(b"Hello", b" ", &[]);
    assert_eq!(out, b"Hello");
}

#[test]
fn test_encrypt_mixed_key() {
    // C: enc_mixed_key='abc' for "abc", " a"
    // The space in key is invalid; the rotation index decrements only
    // when the *current src char* is in the alphabet but the key char
    // is not. So with key=" a", first src char 'a' tries key[0]=' ' which
    // is not found in column 'a' so j-- (back to 0). Next src char 'b'
    // tries key[0]=' ' again which fails again.
    let out = beaufort_encrypt(b"abc", b" a", &[]);
    assert_eq!(out, b"abc");
}

#[test]
fn test_encrypt_alt() {
    // C: enc_alt='NN!!LL' for "ab!!cd", "xy"
    let out = beaufort_encrypt(b"ab!!cd", b"xy", &[]);
    assert_eq!(out, b"NN!!LL");
}

#[test]
fn test_encrypt_long_a() {
    // C: enc_long_a='qqqqqqqqqqqqqqqq' for "AAAAAAAAAAAAAAAA", "0"
    let out = beaufort_encrypt(b"AAAAAAAAAAAAAAAA", b"0", &[]);
    assert_eq!(out, b"qqqqqqqqqqqqqqqq");
}

#[test]
fn test_encrypt_a_a() {
    // C: enc_a_a='0'
    let out = beaufort_encrypt(b"a", b"a", &[]);
    assert_eq!(out, b"0");
}

#[test]
fn test_encrypt_z_capital_z() {
    // C: enc_z_Z='a'
    let out = beaufort_encrypt(b"z", b"Z", &[]);
    assert_eq!(out, b"a");
}

#[test]
fn test_encrypt_invalid_key() {
    // C: enc_invalid_key='abc'
    let out = beaufort_encrypt(b"abc", b"!", &[]);
    assert_eq!(out, b"abc");
}

#[test]
fn test_encrypt_period() {
    // C: enc_period='QQOOMMKKII' for "abcdefghij", "01"
    let out = beaufort_encrypt(b"abcdefghij", b"01", &[]);
    assert_eq!(out, b"QQOOMMKKII");
}

#[test]
fn test_encrypt_digits() {
    // C: enc_digits='9876543210' for "0123456789", "9"
    let out = beaufort_encrypt(b"0123456789", b"9", &[]);
    assert_eq!(out, b"9876543210");
}

#[test]
fn test_encrypt_custom_tableau_six() {
    // From C: tableau("abcdef"), encrypt("abcdef", "ab") => "aaeecc"
    let mat_owned = beaufort_tableau("abcdef");
    let mat_refs: Vec<&[u8]> = mat_owned.iter().map(|r| r.as_slice()).collect();
    let out = beaufort_encrypt(b"abcdef", b"ab", &mat_refs);
    assert_eq!(out, b"aaeecc");
}

fn main() {}
