use libbeaufort::tableau::beaufort_tableau;

#[test]
fn test_tableau_default_alphabet_dimensions() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = beaufort_tableau(alpha);
    assert_eq!(mat.len(), 62);
    for row in &mat {
        assert_eq!(row.len(), 62);
    }
}

#[test]
fn test_tableau_default_first_row() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = beaufort_tableau(alpha);
    let row0 = std::str::from_utf8(&mat[0]).unwrap();
    assert_eq!(row0, "0zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA987654321");
}

#[test]
fn test_tableau_default_second_row() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = beaufort_tableau(alpha);
    let row1 = std::str::from_utf8(&mat[1]).unwrap();
    assert_eq!(row1, "10zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA98765432");
}

#[test]
fn test_tableau_default_third_row() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = beaufort_tableau(alpha);
    let row2 = std::str::from_utf8(&mat[2]).unwrap();
    assert_eq!(row2, "210zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA9876543");
}

#[test]
fn test_tableau_default_last_row() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = beaufort_tableau(alpha);
    let row61 = std::str::from_utf8(&mat[61]).unwrap();
    assert_eq!(row61, "zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA9876543210");
}

#[test]
fn test_tableau_abc() {
    // Mirrors the C test case
    let mat = beaufort_tableau("abc");
    assert_eq!(mat.len(), 3);
    assert_eq!(mat[0].len(), 3);
    assert_eq!(mat[0][0], b'a');
    assert_eq!(mat[0][1], b'c');
    assert_eq!(mat[0][2], b'b');

    assert_eq!(mat[1][0], b'b');
    assert_eq!(mat[1][1], b'a');
    assert_eq!(mat[1][2], b'c');

    assert_eq!(mat[2][0], b'c');
    assert_eq!(mat[2][1], b'b');
    assert_eq!(mat[2][2], b'a');
}

#[test]
fn test_tableau_single_char_alphabet() {
    let mat = beaufort_tableau("a");
    assert_eq!(mat.len(), 1);
    assert_eq!(mat[0].len(), 1);
    assert_eq!(mat[0][0], b'a');
}

#[test]
fn test_tableau_two_char_alphabet() {
    let mat = beaufort_tableau("ab");
    assert_eq!(mat.len(), 2);
    assert_eq!(mat[0].len(), 2);
    assert_eq!(std::str::from_utf8(&mat[0]).unwrap(), "ab");
    assert_eq!(std::str::from_utf8(&mat[1]).unwrap(), "ba");
}

#[test]
fn test_tableau_six_char_alphabet() {
    let mat = beaufort_tableau("abcdef");
    assert_eq!(mat.len(), 6);
    assert_eq!(std::str::from_utf8(&mat[0]).unwrap(), "afedcb");
    assert_eq!(std::str::from_utf8(&mat[1]).unwrap(), "bafedc");
    assert_eq!(std::str::from_utf8(&mat[2]).unwrap(), "cbafed");
    assert_eq!(std::str::from_utf8(&mat[5]).unwrap(), "fedcba");
}

#[test]
fn test_tableau_xyz() {
    let mat = beaufort_tableau("xyz");
    assert_eq!(mat.len(), 3);
    assert_eq!(std::str::from_utf8(&mat[0]).unwrap(), "xzy");
    assert_eq!(std::str::from_utf8(&mat[1]).unwrap(), "yxz");
    assert_eq!(std::str::from_utf8(&mat[2]).unwrap(), "zyx");
}

#[test]
fn test_tableau_abcd_uppercase() {
    let mat = beaufort_tableau("ABCD");
    assert_eq!(mat.len(), 4);
    assert_eq!(std::str::from_utf8(&mat[0]).unwrap(), "ADCB");
    assert_eq!(std::str::from_utf8(&mat[1]).unwrap(), "BADC");
    assert_eq!(std::str::from_utf8(&mat[2]).unwrap(), "CBAD");
    assert_eq!(std::str::from_utf8(&mat[3]).unwrap(), "DCBA");
}

fn main() {}
