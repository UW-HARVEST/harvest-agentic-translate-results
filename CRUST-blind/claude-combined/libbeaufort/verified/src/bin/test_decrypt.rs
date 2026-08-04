use libbeaufort::{decrypt, tableau};

fn make_default_mat() -> Vec<Vec<u8>> {
    tableau::beaufort_tableau(
        std::str::from_utf8(libbeaufort::BEAUFORT_ALPHA).unwrap(),
    )
}

#[test]
fn test_decrypt_hello_with_secret() {
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"b0r6q", b"secret", &mat_refs);
    assert_eq!(out, b"Hello");
}

#[test]
fn test_decrypt_default_tableau_via_empty_mat() {
    let out = decrypt::beaufort_decrypt(b"b0r6q", b"secret", &[]);
    assert_eq!(out, b"Hello");
}

#[test]
fn test_decrypt_short_with_short_key() {
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"aTmXQjU", b"key", &mat_refs);
    assert_eq!(out, b"ABCDEFG");
}

#[test]
fn test_decrypt_digits() {
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"sdaoaomXUiUigROc", b"secret", &mat_refs);
    assert_eq!(out, b"0123456789ABCDEF");
}

#[test]
fn test_decrypt_with_long_key() {
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"PON", b"ZZZZZZZZZZ", &mat_refs);
    assert_eq!(out, b"ABC");
}

#[test]
fn test_decrypt_with_punctuation_and_space() {
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"3anZQ, 2WNnh!", b"KEY", &mat_refs);
    assert_eq!(out, b"Hello, World!");
}

#[test]
fn test_decrypt_empty_input() {
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"", b"key", &mat_refs);
    assert_eq!(out, b"");
}

#[test]
fn test_decrypt_only_unrecognized() {
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"!@#$%", b"key", &mat_refs);
    assert_eq!(out, b"!@#$%");
}

#[test]
fn test_decrypt_small_alphabet_ab() {
    let mat = tableau::beaufort_tableau("AB");
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"BA", b"B", &mat_refs);
    assert_eq!(out, b"AB");
}

#[test]
fn test_decrypt_small_alphabet_abc() {
    // From C: encrypt "CBA" with key "AB", alpha "ABC" -> "BAA"
    // So decrypt "BAA" with key "AB", alpha "ABC" -> "CBA"
    let mat = tableau::beaufort_tableau("ABC");
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"BAA", b"AB", &mat_refs);
    assert_eq!(out, b"CBA");
}

#[test]
fn test_decrypt_aaa_with_aaa() {
    // Encrypt "AAA" with key "AAA" gives "000"; thus decrypt "000" -> "AAA"
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = decrypt::beaufort_decrypt(b"000", b"AAA", &mat_refs);
    assert_eq!(out, b"AAA");
}

#[test]
fn test_decrypt_reciprocal_property() {
    // Encrypting a ciphertext yields the original plaintext (Beaufort is its own inverse).
    use libbeaufort::encrypt;
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let pt = b"HelloWorld";
    let ct = encrypt::beaufort_encrypt(pt, b"key", &mat_refs);
    let back = decrypt::beaufort_decrypt(&ct, b"key", &mat_refs);
    assert_eq!(back.as_slice(), pt);
}

#[test]
fn test_decrypt_unrecognized_does_not_advance_key() {
    // Should produce same prefix when only alphabet chars are passed.
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    // From C: "3anZQ" decoded with "KEY" should be "Hello"
    let out = decrypt::beaufort_decrypt(b"3anZQ", b"KEY", &mat_refs);
    assert_eq!(out, b"Hello");
}

fn main() {}
