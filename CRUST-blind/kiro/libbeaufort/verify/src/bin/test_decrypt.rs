use libbeaufort::decrypt::beaufort_decrypt;
use libbeaufort::encrypt::beaufort_encrypt;
use libbeaufort::tableau::beaufort_tableau;

fn make_default_mat() -> Vec<Vec<u8>> {
    beaufort_tableau("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
}

fn roundtrip(src: &[u8], key: &[u8], mat: &[Vec<u8>]) {
    let refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let enc = beaufort_encrypt(src, key, &refs);
    let dec = beaufort_decrypt(&enc, key, &refs);
    assert_eq!(dec, src);
}

fn decrypt(src: &[u8], key: &[u8], mat: &[Vec<u8>]) -> Vec<u8> {
    let refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    beaufort_decrypt(src, key, &refs)
}

#[test]
fn test_decrypt_monkey() {
    let mat = make_default_mat();
    assert_eq!(decrypt(b"26004Fyuv AnK Cs9sqC8", b"monkey", &mat), b"kinkajous are awesome");
}

#[test]
fn test_decrypt_goodman() {
    let mat = make_default_mat();
    assert_eq!(decrypt(b"n7A \n24u \n22D0huq5", b"goodman", &mat), b"the \nbig \nlebowski");
}

#[test]
fn test_decrypt_groove() {
    let mat = make_default_mat();
    assert_eq!(decrypt(b"3n1Cs lg y7l 9ko!F :b", b"groove", &mat), b"d4nc3 t0 th3 mus!c :D");
}

#[test]
fn test_decrypt_all_passthrough() {
    let mat = make_default_mat();
    assert_eq!(decrypt(b"!@#", b"key", &mat), b"!@#");
}

#[test]
fn test_roundtrip_monkey() {
    roundtrip(b"kinkajous are awesome", b"monkey", &make_default_mat());
}

#[test]
fn test_roundtrip_goodman() {
    roundtrip(b"the \nbig \nlebowski", b"goodman", &make_default_mat());
}

#[test]
fn test_roundtrip_groove() {
    roundtrip(b"d4nc3 t0 th3 mus!c :D", b"groove", &make_default_mat());
}

#[test]
fn test_roundtrip_hello_world() {
    roundtrip(b"Hello World 123!", b"test", &make_default_mat());
}

#[test]
fn test_roundtrip_custom_alphabet() {
    let mat = beaufort_tableau("abc");
    let refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    let enc = beaufort_encrypt(b"abc", b"a", &refs);
    let dec = beaufort_decrypt(&enc, b"a", &refs);
    assert_eq!(dec, b"abc");
}

#[test]
fn test_decrypt_empty() {
    let mat = make_default_mat();
    assert_eq!(decrypt(b"", b"key", &mat), b"");
}

fn main() {}
