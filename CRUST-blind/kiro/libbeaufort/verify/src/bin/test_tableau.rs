use libbeaufort::tableau::beaufort_tableau;

#[test]
fn test_abc_tableau() {
    let mat = beaufort_tableau("abc");
    assert_eq!(mat.len(), 3);
    assert_eq!(mat[0], b"acb");
    assert_eq!(mat[1], b"bac");
    assert_eq!(mat[2], b"cba");
}

#[test]
fn test_single_char() {
    let mat = beaufort_tableau("X");
    assert_eq!(mat.len(), 1);
    assert_eq!(mat[0], b"X");
}

#[test]
fn test_default_alpha_dimensions() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = beaufort_tableau(alpha);
    assert_eq!(mat.len(), 62);
    for row in &mat {
        assert_eq!(row.len(), 62);
    }
}

#[test]
fn test_default_alpha_first_row() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = beaufort_tableau(alpha);
    // First row starts with '0', ends with '1' (shifted)
    assert_eq!(mat[0][0], b'0');
    assert_eq!(mat[0][1], b'z');
    assert_eq!(mat[0][61], b'1');
}

#[test]
fn test_default_alpha_second_row() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = beaufort_tableau(alpha);
    assert_eq!(mat[1][0], b'1');
    assert_eq!(mat[1][1], b'0');
}

#[test]
fn test_two_char() {
    let mat = beaufort_tableau("ab");
    assert_eq!(mat[0], b"ab");
    assert_eq!(mat[1], b"ba");
}

fn main() {}
