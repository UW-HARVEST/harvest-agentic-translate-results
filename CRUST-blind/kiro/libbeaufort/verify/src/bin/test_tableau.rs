use libbeaufort::tableau::beaufort_tableau;

#[test]
fn test_tableau_abc() {
    let mat = beaufort_tableau("abc");
    assert_eq!(mat.len(), 3);
    assert_eq!(mat[0], b"acb");
    assert_eq!(mat[1], b"bac");
    assert_eq!(mat[2], b"cba");
}

#[test]
fn test_tableau_default_alpha() {
    let mat = beaufort_tableau("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz");
    assert_eq!(mat.len(), 62);
    assert_eq!(mat[0], b"0zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA987654321");
    assert_eq!(mat[1], b"10zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA98765432");
    for row in &mat {
        assert_eq!(row.len(), 62);
    }
}

#[test]
fn test_tableau_single_char() {
    let mat = beaufort_tableau("x");
    assert_eq!(mat.len(), 1);
    assert_eq!(mat[0], b"x");
}

fn main() {}
