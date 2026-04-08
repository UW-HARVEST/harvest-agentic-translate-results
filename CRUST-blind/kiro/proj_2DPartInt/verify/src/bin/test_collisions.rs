use twoDPartInt::collisions;

#[test]
fn test_find_col_center() {
    assert_eq!(collisions::find_col(0.0, 4, 100.0), 2);
}

#[test]
fn test_find_col_left_boundary() {
    assert_eq!(collisions::find_col(-250.0, 4, 100.0), -2);
}

#[test]
fn test_find_col_right_boundary() {
    assert_eq!(collisions::find_col(250.0, 4, 100.0), -1);
}

#[test]
fn test_find_col_near_left() {
    assert_eq!(collisions::find_col(-199.0, 4, 100.0), 0);
}

#[test]
fn test_find_col_near_right() {
    assert_eq!(collisions::find_col(199.0, 4, 100.0), 3);
}

#[test]
fn test_find_col_at_minus100() {
    assert_eq!(collisions::find_col(-100.0, 4, 100.0), 1);
}

#[test]
fn test_find_row_middle() {
    assert_eq!(collisions::find_row(50.0, 4, 100.0), 0);
}

#[test]
fn test_find_row_below() {
    assert_eq!(collisions::find_row(-10.0, 4, 100.0), -2);
}

#[test]
fn test_find_row_above() {
    assert_eq!(collisions::find_row(450.0, 4, 100.0), -1);
}

#[test]
fn test_find_row_zero() {
    assert_eq!(collisions::find_row(0.0, 4, 100.0), 0);
}

#[test]
fn test_find_row_top() {
    assert_eq!(collisions::find_row(399.0, 4, 100.0), 3);
}

#[test]
fn test_find_square_valid() {
    assert_eq!(collisions::find_square(0.0, 50.0, 4, 4, 100.0), 2);
}

#[test]
fn test_find_square_outside_left() {
    assert_eq!(collisions::find_square(-250.0, 50.0, 4, 4, 100.0), -1);
}

#[test]
fn test_find_square_outside_below() {
    assert_eq!(collisions::find_square(0.0, -10.0, 4, 4, 100.0), -1);
}

fn main() {}
