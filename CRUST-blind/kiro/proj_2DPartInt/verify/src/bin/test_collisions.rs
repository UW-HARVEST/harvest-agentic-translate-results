use twoDPartInt::collisions;

// --- find_col ---

#[test]
fn test_find_col_inside() {
    // x_squares=4, square_length=10, x_left_limit=-20
    assert_eq!(collisions::find_col(-19.0, 4, 10.0), 0);
    assert_eq!(collisions::find_col(-15.0, 4, 10.0), 0);
    assert_eq!(collisions::find_col(-10.0, 4, 10.0), 1);
    assert_eq!(collisions::find_col(-5.0, 4, 10.0), 1);
    assert_eq!(collisions::find_col(0.0, 4, 10.0), 2);
    assert_eq!(collisions::find_col(5.0, 4, 10.0), 2);
    assert_eq!(collisions::find_col(10.0, 4, 10.0), 3);
    assert_eq!(collisions::find_col(15.0, 4, 10.0), 3);
    assert_eq!(collisions::find_col(19.0, 4, 10.0), 3);
}

#[test]
fn test_find_col_boundary() {
    assert_eq!(collisions::find_col(-20.0, 4, 10.0), 0); // exact left edge
    assert_eq!(collisions::find_col(20.0, 4, 10.0), -1);  // right boundary
    assert_eq!(collisions::find_col(-21.0, 4, 10.0), -2); // left boundary
}

// --- find_row ---

#[test]
fn test_find_row_inside() {
    assert_eq!(collisions::find_row(0.0, 4, 10.0), 0);
    assert_eq!(collisions::find_row(5.0, 4, 10.0), 0);
    assert_eq!(collisions::find_row(10.0, 4, 10.0), 1);
    assert_eq!(collisions::find_row(15.0, 4, 10.0), 1);
    assert_eq!(collisions::find_row(25.0, 4, 10.0), 2);
    assert_eq!(collisions::find_row(39.0, 4, 10.0), 3);
}

#[test]
fn test_find_row_boundary() {
    assert_eq!(collisions::find_row(40.0, 4, 10.0), -1); // top boundary
    assert_eq!(collisions::find_row(-1.0, 4, 10.0), -2);  // below zero
}

// --- find_square ---

#[test]
fn test_find_square_inside() {
    assert_eq!(collisions::find_square(-15.0, 5.0, 4, 4, 10.0), 0);
    assert_eq!(collisions::find_square(5.0, 15.0, 4, 4, 10.0), 6);
}

#[test]
fn test_find_square_outside() {
    assert_eq!(collisions::find_square(-21.0, 5.0, 4, 4, 10.0), -1);
    assert_eq!(collisions::find_square(5.0, -1.0, 4, 4, 10.0), -1);
    assert_eq!(collisions::find_square(5.0, 45.0, 4, 4, 10.0), -1);
}

fn main() {}
