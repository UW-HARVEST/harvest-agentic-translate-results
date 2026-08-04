use rubiksolver::rubik_model::{
    new_cube_rotate_face, populate_initial, populate_specific, Color, Cube, ECube, Rotation,
};
use rubiksolver::solve_rubik::{ecube_compare_equal, hash_function, is_better};

#[test]
fn test_ecube_compare_equal_solved() {
    let a = ECube::new(populate_initial());
    let b = ECube::new(populate_initial());
    assert!(ecube_compare_equal(&a, &b));
}

#[test]
fn test_ecube_compare_equal_different_entropy() {
    let solved = ECube::new(populate_initial());
    let rotated = ECube::new(new_cube_rotate_face(
        &populate_initial(),
        Color::Red,
        Rotation::CW,
    ));
    assert!(!ecube_compare_equal(&solved, &rotated));
}

#[test]
fn test_ecube_compare_equal_same_entropy_different_cube() {
    // Two cubes with the same entropy but different states
    let r_cw = ECube::new(new_cube_rotate_face(
        &populate_initial(),
        Color::Red,
        Rotation::CW,
    ));
    let g_cw = ECube::new(new_cube_rotate_face(
        &populate_initial(),
        Color::Green,
        Rotation::CW,
    ));
    assert_eq!(r_cw.entropy, 12);
    assert_eq!(g_cw.entropy, 12);
    assert!(!ecube_compare_equal(&r_cw, &g_cw));
}

#[test]
fn test_is_better_lower_entropy() {
    let solved = ECube::new(populate_initial());
    let rotated = ECube::new(new_cube_rotate_face(
        &populate_initial(),
        Color::Red,
        Rotation::CW,
    ));
    assert!(is_better(&solved, &rotated));
    assert!(!is_better(&rotated, &solved));
    assert!(!is_better(&solved, &solved));
}

#[test]
fn test_hash_function_returns_precomputed() {
    let solved = ECube::new(populate_initial());
    assert_eq!(hash_function(&solved), 0);

    let test_data: Cube = [
        [Color::Red; 8],
        [
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Orange,
            Color::Orange,
            Color::Yellow,
            Color::Green,
            Color::Green,
        ],
        [
            Color::Blue,
            Color::Blue,
            Color::Blue,
            Color::Orange,
            Color::Orange,
            Color::Green,
            Color::Orange,
            Color::Blue,
        ],
        [
            Color::White,
            Color::White,
            Color::White,
            Color::Green,
            Color::Green,
            Color::White,
            Color::Yellow,
            Color::Blue,
        ],
        [
            Color::Orange,
            Color::Green,
            Color::Blue,
            Color::Yellow,
            Color::Yellow,
            Color::Yellow,
            Color::Yellow,
            Color::Blue,
        ],
        [
            Color::Yellow,
            Color::Orange,
            Color::White,
            Color::White,
            Color::White,
            Color::White,
            Color::Green,
            Color::Orange,
        ],
    ];
    let cube = populate_specific(&test_data);
    let ecube = ECube::new(cube);
    assert_eq!(hash_function(&ecube), 1900466);
    assert_eq!(ecube.entropy, 25);
}

fn main() {}
