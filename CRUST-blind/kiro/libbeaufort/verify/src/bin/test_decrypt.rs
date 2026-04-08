use libbeaufort::decrypt::beaufort_decrypt;
use libbeaufort::encrypt::beaufort_encrypt;
use libbeaufort::tableau::beaufort_tableau;

fn default_mat() -> Vec<Vec<u8>> {
    beaufort_tableau("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
}

fn mat_refs(mat: &[Vec<u8>]) -> Vec<&[u8]> {
    mat.iter().map(|r| r.as_slice()).collect()
}

#[test]
fn test_decrypt_monkey() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_decrypt(b"26004Fyuv AnK Cs9sqC8", b"monkey", &r);
    assert_eq!(out, b"kinkajous are awesome");
}

#[test]
fn test_decrypt_goodman() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_decrypt(b"n7A \n24u \n22D0huq5", b"goodman", &r);
    assert_eq!(out, b"the \nbig \nlebowski");
}

#[test]
fn test_decrypt_groove() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_decrypt(b"3n1Cs lg y7l 9ko!F :b", b"groove", &r);
    assert_eq!(out, b"d4nc3 t0 th3 mus!c :D");
}

#[test]
fn test_decrypt_special_chars_passthrough() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_decrypt(b"30Dzq!", b"key", &r);
    assert_eq!(out, b"hello!");
}

#[test]
fn test_decrypt_roundtrip() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let plain = b"kinkajous are awesome";
    let enc = beaufort_encrypt(plain, b"monkey", &r);
    let dec = beaufort_decrypt(&enc, b"monkey", &r);
    assert_eq!(dec, plain);
}

#[test]
fn test_decrypt_empty() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_decrypt(b"", b"key", &r);
    assert_eq!(out, b"");
}

fn main() {}
