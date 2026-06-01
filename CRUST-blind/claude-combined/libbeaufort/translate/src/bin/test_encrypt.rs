use libbeaufort::{encrypt, tableau};

fn make_default_mat() -> Vec<Vec<u8>> {
    tableau::beaufort_tableau(
        std::str::from_utf8(libbeaufort::BEAUFORT_ALPHA).unwrap(),
    )
}

#[test]
fn test_encrypt_hello_with_secret() {
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"Hello", b"secret", &mat_refs);
    assert_eq!(out, b"b0r6q");
}

#[test]
fn test_encrypt_with_default_tableau_via_empty_mat() {
    // When mat is empty, the function should fall back to the default tableau.
    let out = encrypt::beaufort_encrypt(b"Hello", b"secret", &[]);
    assert_eq!(out, b"b0r6q");
}

#[test]
fn test_encrypt_with_punctuation_and_space() {
    // From C: "Hello, World!" with key "KEY" -> "3anZQ, 2WNnh!"
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"Hello, World!", b"KEY", &mat_refs);
    assert_eq!(out, b"3anZQ, 2WNnh!");
}

#[test]
fn test_encrypt_digits() {
    // "0123456789ABCDEF" with key "secret" -> "sdaoaomXUiUigROc"
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"0123456789ABCDEF", b"secret", &mat_refs);
    assert_eq!(out, b"sdaoaomXUiUigROc");
}

#[test]
fn test_encrypt_short_with_short_key() {
    // "ABCDEFG" with key "key" -> "aTmXQjU"
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"ABCDEFG", b"key", &mat_refs);
    assert_eq!(out, b"aTmXQjU");
}

#[test]
fn test_encrypt_empty_input() {
    // Empty input should produce empty output.
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"", b"key", &mat_refs);
    assert_eq!(out, b"");
}

#[test]
fn test_encrypt_only_unrecognized() {
    // characters not in alphabet are left unchanged
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"!@#$%", b"key", &mat_refs);
    assert_eq!(out, b"!@#$%");
}

#[test]
fn test_encrypt_small_alphabet_ab() {
    // alpha="AB", key="B"
    // Tableau: row0="AB", row1="BA"
    // src="AB":
    //   ch='A', x=0 (mat[0][0]='A'); k='B'; find y where mat[y][0]='B' -> y=1; out=mat[1][0]='B'
    //   ch='B', x=1 (mat[0][1]='B'); k='B'; find y where mat[y][1]='B' -> y=0; out=mat[0][0]='A'
    // C reference: "AB" -> "BA"
    let mat = tableau::beaufort_tableau("AB");
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"AB", b"B", &mat_refs);
    assert_eq!(out, b"BA");
}

#[test]
fn test_encrypt_small_alphabet_abc() {
    // From C reference: "CBA" with key "AB" alphabet "ABC" -> "BAA"
    let mat = tableau::beaufort_tableau("ABC");
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"CBA", b"AB", &mat_refs);
    assert_eq!(out, b"BAA");
}

#[test]
fn test_encrypt_unrecognized_char_does_not_advance_key() {
    // Validates the behavior that non-alphabet chars are passed through and do
    // not consume key positions. C output: "Hello, World!" key=KEY ->
    // "3anZQ, 2WNnh!". Ensure encrypting "Hello" alone with the same key
    // gives the same prefix "3anZQ", confirming that the comma/space did not
    // shift the key cycle.
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let part = encrypt::beaufort_encrypt(b"Hello", b"KEY", &mat_refs);
    assert_eq!(part, b"3anZQ");
}

#[test]
fn test_encrypt_key_longer_than_text() {
    // From C: "ABC" with key "ZZZZZZZZZZ" -> "PON"
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"ABC", b"ZZZZZZZZZZ", &mat_refs);
    assert_eq!(out, b"PON");
}

#[test]
fn test_encrypt_aaa_with_aaa() {
    // "AAA" with key "AAA" -> "000" (per C)
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let out = encrypt::beaufort_encrypt(b"AAA", b"AAA", &mat_refs);
    assert_eq!(out, b"000");
}

#[test]
fn test_encrypt_reciprocal_property() {
    // The Beaufort cipher is its own inverse (given the same key).
    // Encrypt twice should give back the original (for chars in the alphabet).
    let mat = make_default_mat();
    let mat_refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let original = b"ABCDE";
    let ct = encrypt::beaufort_encrypt(original, b"key", &mat_refs);
    assert_eq!(ct, b"aTmXQ");
    let pt = encrypt::beaufort_encrypt(&ct, b"key", &mat_refs);
    assert_eq!(pt, original);
}

fn main() {}
