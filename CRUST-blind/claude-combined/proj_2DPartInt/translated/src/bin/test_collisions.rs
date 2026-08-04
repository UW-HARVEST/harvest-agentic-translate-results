use twoDPartInt::collisions;
use twoDPartInt::data::{Contact, Particle};

fn make_particle(x: f64, y: f64, r: f64, idx: i32) -> Particle {
    Particle {
        x_coordinate: x,
        y_coordinate: y,
        radius: r,
        next: None,
        idx,
    }
}

#[test]
fn test_find_col_inside() {
    assert_eq!(collisions::find_col(0.0, 4, 100.0), 2);
    assert_eq!(collisions::find_col(-200.0, 4, 100.0), 0);
    assert_eq!(collisions::find_col(199.0, 4, 100.0), 3);
    assert_eq!(collisions::find_col(-100.0, 4, 100.0), 1);
    assert_eq!(collisions::find_col(50.0, 4, 100.0), 2);
    assert_eq!(collisions::find_col(100.0, 4, 100.0), 3);
}

#[test]
fn test_find_col_left_boundary() {
    assert_eq!(collisions::find_col(-201.0, 4, 100.0), -2);
}

#[test]
fn test_find_col_right_boundary() {
    assert_eq!(collisions::find_col(200.0, 4, 100.0), -1);
}

#[test]
fn test_find_row_inside() {
    assert_eq!(collisions::find_row(0.0, 5, 50.0), 0);
    assert_eq!(collisions::find_row(49.0, 5, 50.0), 0);
    assert_eq!(collisions::find_row(50.0, 5, 50.0), 1);
    assert_eq!(collisions::find_row(200.0, 5, 50.0), 4);
    assert_eq!(collisions::find_row(249.0, 5, 50.0), 4);
}

#[test]
fn test_find_row_above() {
    assert_eq!(collisions::find_row(250.0, 5, 50.0), -1);
}

#[test]
fn test_find_row_below() {
    assert_eq!(collisions::find_row(-1.0, 5, 50.0), -2);
}

#[test]
fn test_find_square_basic() {
    assert_eq!(collisions::find_square(0.0, 0.0, 4, 5, 100.0), 2);
    assert_eq!(collisions::find_square(-200.0, 0.0, 4, 5, 100.0), 0);
    assert_eq!(collisions::find_square(0.0, 100.0, 4, 5, 100.0), 6);
    assert_eq!(collisions::find_square(199.0, 499.0, 4, 5, 100.0), 19);
}

#[test]
fn test_find_square_outside() {
    assert_eq!(collisions::find_square(-300.0, 50.0, 4, 5, 100.0), -1);
    assert_eq!(collisions::find_square(-400.0, 0.0, 4, 5, 100.0), -1);
    assert_eq!(collisions::find_square(0.0, 500.0, 4, 5, 100.0), -1);
    assert_eq!(collisions::find_square(0.0, -1.0, 4, 5, 100.0), -1);
    assert_eq!(collisions::find_square(-201.0, 0.0, 4, 5, 100.0), -1);
    assert_eq!(collisions::find_square(200.0, 0.0, 4, 5, 100.0), -1);
}

#[test]
fn test_compute_contacts_two_overlap() {
    // Two overlapping particles in the same square.
    let p0 = make_particle(50.0, 50.0, 30.0, 0);
    let p1 = make_particle(60.0, 50.0, 30.0, 1);

    let x_squares = 2;
    let y_squares = 2;
    let square_length = 100.0;

    // Square layout: 4 squares total. Particles fall in square (row=0,col=1) -> idx=1
    // For x=50 with x_left=-100: diff=-100-50=-150, |diff|=150 -> col=1, row=0 -> sq=1
    let s0: Vec<&Particle> = vec![];
    let s1: Vec<&Particle> = vec![&p0, &p1];
    let s2: Vec<&Particle> = vec![];
    let s3: Vec<&Particle> = vec![];
    let grid: Vec<&Vec<&Particle>> = vec![&s0, &s1, &s2, &s3];

    let mut contacts = vec![Contact { p1_idx: 0, p2_idx: 0, overlap: 0.0 }; 8];
    let n = collisions::compute_contacts(&grid, x_squares, y_squares, square_length, &mut contacts);
    // 2 contacts: (0->1) and (1->0). Each direction is recorded.
    assert_eq!(n, 2);

    // Sort contacts for a stable assertion.
    let mut found_01 = false;
    let mut found_10 = false;
    for i in 0..n {
        let c = &contacts[i];
        if c.p1_idx == 0 && c.p2_idx == 1 {
            assert!((c.overlap - 50.0).abs() < 1e-9);
            found_01 = true;
        } else if c.p1_idx == 1 && c.p2_idx == 0 {
            assert!((c.overlap - 50.0).abs() < 1e-9);
            found_10 = true;
        }
    }
    assert!(found_01);
    assert!(found_10);
}

#[test]
fn test_compute_contacts_no_overlap() {
    let p0 = make_particle(10.0, 10.0, 5.0, 0);
    let p1 = make_particle(80.0, 80.0, 5.0, 1);

    let x_squares = 2;
    let y_squares = 2;
    let square_length = 100.0;

    // p0: x=10 -> diff=-100-10=-110, col = 1; y=10 -> row=0 -> idx=1
    // p1: x=80 -> diff=-100-80=-180, col=1, y=80 -> row=0 -> idx=1
    let s0: Vec<&Particle> = vec![];
    let s1: Vec<&Particle> = vec![&p0, &p1];
    let s2: Vec<&Particle> = vec![];
    let s3: Vec<&Particle> = vec![];
    let grid: Vec<&Vec<&Particle>> = vec![&s0, &s1, &s2, &s3];

    let mut contacts = vec![Contact { p1_idx: 0, p2_idx: 0, overlap: 0.0 }; 8];
    let n = collisions::compute_contacts(&grid, x_squares, y_squares, square_length, &mut contacts);
    assert_eq!(n, 0);
}

#[test]
fn test_compute_contacts_empty_grid() {
    let x_squares = 2;
    let y_squares = 2;
    let square_length = 100.0;
    let s0: Vec<&Particle> = vec![];
    let s1: Vec<&Particle> = vec![];
    let s2: Vec<&Particle> = vec![];
    let s3: Vec<&Particle> = vec![];
    let grid: Vec<&Vec<&Particle>> = vec![&s0, &s1, &s2, &s3];
    let mut contacts = vec![Contact { p1_idx: 0, p2_idx: 0, overlap: 0.0 }; 4];
    let n = collisions::compute_contacts(&grid, x_squares, y_squares, square_length, &mut contacts);
    assert_eq!(n, 0);
}

fn main() {}
