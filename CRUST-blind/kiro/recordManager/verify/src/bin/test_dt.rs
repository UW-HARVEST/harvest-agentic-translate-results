use recordManager::dt::{Bool, TRUE, FALSE};

#[test]
fn test_bool_type() {
    let t: Bool = TRUE;
    let f: Bool = FALSE;
    assert_eq!(t, true);
    assert_eq!(f, false);
}

#[test]
fn test_bool_operations() {
    assert_eq!(TRUE && TRUE, true);
    assert_eq!(TRUE && FALSE, false);
    assert_eq!(TRUE || FALSE, true);
    assert_eq!(FALSE || FALSE, false);
    assert_eq!(!FALSE, true);
    assert_eq!(!TRUE, false);
}

fn main() {}
