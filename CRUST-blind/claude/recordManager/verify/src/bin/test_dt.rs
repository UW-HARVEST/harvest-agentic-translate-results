use recordManager::dt::{Bool, FALSE, TRUE};

#[test]
fn test_bool_constants() {
    let t: Bool = TRUE;
    let f: Bool = FALSE;
    assert!(t);
    assert!(!f);
    assert_eq!(t, true);
    assert_eq!(f, false);
}

#[test]
fn test_bool_logic() {
    assert_eq!(TRUE && TRUE, true);
    assert_eq!(TRUE && FALSE, false);
    assert_eq!(TRUE || FALSE, true);
    assert_eq!(FALSE || FALSE, false);
    assert_eq!(!TRUE, false);
    assert_eq!(!FALSE, true);
}

fn main() {}
