use recordManager::dt::{Bool, TRUE, FALSE};

#[test]
fn test_bool_type() {
    let t: Bool = TRUE;
    let f: Bool = FALSE;
    assert!(t);
    assert!(!f);
    assert_eq!(TRUE, true);
    assert_eq!(FALSE, false);
}

fn main() {}
