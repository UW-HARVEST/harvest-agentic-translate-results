use libbeaufort::encrypt::beaufort_encrypt;
use libbeaufort::tableau::beaufort_tableau;

fn make_default_mat() -> Vec<Vec<u8>> {
    beaufort_tableau("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
}

fn encrypt(src: &[u8], key: &[u8], mat: &[Vec<u8>]) -> Vec<u8> {
    let refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    beaufort_encrypt(src, key, &refs)
}

#[test]
fn test_encrypt_monkey() {
    let mat = make_default_mat();
    assert_eq!(encrypt(b"kinkajous are awesome", b"monkey", &mat), b"26004Fyuv AnK Cs9sqC8");
}

#[test]
fn test_encrypt_goodman() {
    let mat = make_default_mat();
    assert_eq!(encrypt(b"the \nbig \nlebowski", b"goodman", &mat), b"n7A \n24u \n22D0huq5");
}

#[test]
fn test_encrypt_groove() {
    let mat = make_default_mat();
    assert_eq!(encrypt(b"d4nc3 t0 th3 mus!c :D", b"groove", &mat), b"3n1Cs lg y7l 9ko!F :b");
}

#[test]
fn test_encrypt_passthrough() {
    let mat = make_default_mat();
    assert_eq!(encrypt(b"hello!", b"key", &mat), b"30Dzq!");
}

#[test]
fn test_encrypt_empty() {
    let mat = make_default_mat();
    assert_eq!(encrypt(b"", b"key", &mat), b"");
}

#[test]
fn test_encrypt_single_char_key() {
    let mat = make_default_mat();
    assert_eq!(encrypt(b"abc", b"a", &mat), b"0zy");
}

#[test]
fn test_encrypt_custom_alphabet() {
    let mat = beaufort_tableau("abc");
    let refs: Vec<&[u8]> = mat.iter().map(|r| r.as_slice()).collect();
    assert_eq!(beaufort_encrypt(b"abc", b"a", &refs), b"acb");
}

#[test]
fn test_encrypt_hello_world() {
    let mat = make_default_mat();
    assert_eq!(encrypt(b"Hello World 123!", b"test", &mat), b"c0785 84281 rrq!");
}

fn main() {}
