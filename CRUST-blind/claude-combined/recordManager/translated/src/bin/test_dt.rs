use recordManager::dt::{Bool, TRUE, FALSE};

#[test]
fn test_true_is_true() {
    let t: Bool = TRUE;
    assert_eq!(t, true);
}

#[test]
fn test_false_is_false() {
    let f: Bool = FALSE;
    assert_eq!(f, false);
}

#[test]
fn test_true_not_false() {
    assert_ne!(TRUE, FALSE);
}

fn main() {}
