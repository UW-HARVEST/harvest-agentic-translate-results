use libbeaufort::encrypt::beaufort_encrypt;
use libbeaufort::tableau::beaufort_tableau;

fn default_mat() -> Vec<Vec<u8>> {
    beaufort_tableau("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
}

fn mat_refs(mat: &[Vec<u8>]) -> Vec<&[u8]> {
    mat.iter().map(|r| r.as_slice()).collect()
}

#[test]
fn test_encrypt_monkey() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_encrypt(b"kinkajous are awesome", b"monkey", &r);
    assert_eq!(out, b"26004Fyuv AnK Cs9sqC8");
}

#[test]
fn test_encrypt_goodman() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_encrypt(b"the \nbig \nlebowski", b"goodman", &r);
    assert_eq!(out, b"n7A \n24u \n22D0huq5");
}

#[test]
fn test_encrypt_groove() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_encrypt(b"d4nc3 t0 th3 mus!c :D", b"groove", &r);
    assert_eq!(out, b"3n1Cs lg y7l 9ko!F :b");
}

#[test]
fn test_encrypt_special_chars_passthrough() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_encrypt(b"hello!", b"key", &r);
    assert_eq!(out, b"30Dzq!");
}

#[test]
fn test_encrypt_empty() {
    let mat = default_mat();
    let r = mat_refs(&mat);
    let out = beaufort_encrypt(b"", b"key", &r);
    assert_eq!(out, b"");
}

fn main() {}
