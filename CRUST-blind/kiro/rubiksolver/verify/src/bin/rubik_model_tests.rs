use rubiksolver::rubik_model::*;

fn test_cube_data() -> Cube {
    [
        [Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red],
        [Color::Green, Color::Yellow, Color::Blue, Color::Orange, Color::Orange, Color::Yellow, Color::Green, Color::Green],
        [Color::Blue, Color::Blue, Color::Blue, Color::Orange, Color::Orange, Color::Green, Color::Orange, Color::Blue],
        [Color::White, Color::White, Color::White, Color::Green, Color::Green, Color::White, Color::Yellow, Color::Blue],
        [Color::Orange, Color::Green, Color::Blue, Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::Blue],
        [Color::Yellow, Color::Orange, Color::White, Color::White, Color::White, Color::White, Color::Green, Color::Orange],
    ]
}

fn yellow_cw_output() -> Cube {
    [
        [Color::Red, Color::Red, Color::Red, Color::Red, Color::Blue, Color::Orange, Color::Orange, Color::Red],
        [Color::Green, Color::Yellow, Color::Blue, Color::Orange, Color::Orange, Color::Yellow, Color::Green, Color::Green],
        [Color::Blue, Color::Blue, Color::Yellow, Color::Blue, Color::White, Color::Green, Color::Orange, Color::Blue],
        [Color::White, Color::White, Color::White, Color::Green, Color::Green, Color::White, Color::Yellow, Color::Orange],
        [Color::Yellow, Color::Blue, Color::Orange, Color::Green, Color::Blue, Color::Yellow, Color::Yellow, Color::Yellow],
        [Color::Red, Color::Red, Color::Red, Color::White, Color::White, Color::White, Color::Green, Color::Orange],
    ]
}

#[test]
fn test_rear() {
    assert_eq!(rear(Color::Red), Color::Orange);
    assert_eq!(rear(Color::Green), Color::Yellow);
    assert_eq!(rear(Color::Blue), Color::White);
    assert_eq!(rear(rear(Color::Red)), Color::Red);
    assert_eq!(rear(rear(Color::Green)), Color::Green);
    assert_eq!(rear(rear(Color::Blue)), Color::Blue);
}

#[test]
fn test_adjacent_cw() {
    assert_eq!(adjacent_cw(Color::Red, Color::Yellow), Color::Blue);
    assert_eq!(adjacent_cw(Color::Red, Color::Blue), Color::Green);
    assert_eq!(adjacent_cw(Color::Red, Color::White), Color::Yellow);
    assert_eq!(adjacent_cw(Color::Red, Color::Green), Color::White);
    assert_eq!(adjacent_cw(Color::Green, Color::Red), Color::Blue);
    assert_eq!(adjacent_cw(Color::Green, Color::White), Color::Red);
    assert_eq!(adjacent_cw(Color::Green, Color::Orange), Color::White);
    assert_eq!(adjacent_cw(Color::Green, Color::Blue), Color::Orange);
    assert_eq!(adjacent_cw(Color::White, Color::Orange), Color::Yellow);
    assert_eq!(adjacent_cw(Color::White, Color::Green), Color::Orange);
    assert_eq!(adjacent_cw(Color::Yellow, Color::White), Color::Orange);
    assert_eq!(adjacent_cw(Color::Yellow, Color::Red), Color::White);
}

#[test]
fn test_adjacent_ccw() {
    assert_eq!(adjacent_ccw(Color::Red, Color::Blue), Color::Yellow);
    assert_eq!(adjacent_ccw(Color::Red, Color::Yellow), Color::White);
    assert_eq!(adjacent_ccw(Color::Red, Color::White), Color::Green);
    assert_eq!(adjacent_ccw(Color::Green, Color::Blue), Color::Red);
    assert_eq!(adjacent_ccw(Color::Green, Color::Red), Color::White);
    assert_eq!(adjacent_ccw(Color::Green, Color::White), Color::Orange);
    assert_eq!(adjacent_ccw(Color::Green, Color::Orange), Color::Blue);
    assert_eq!(adjacent_ccw(Color::White, Color::Yellow), Color::Orange);
    assert_eq!(adjacent_ccw(Color::White, Color::Orange), Color::Green);
    assert_eq!(adjacent_ccw(Color::Yellow, Color::Orange), Color::White);
    assert_eq!(adjacent_ccw(Color::Yellow, Color::White), Color::Red);
}

#[test]
fn test_populate_initial() {
    let cube = populate_initial();
    for face in 0..6 {
        let c = Color::from_usize(face);
        for pos in 0..8 {
            assert_eq!(cube[face][pos], c);
        }
    }
}

#[test]
fn test_cube_hash_initial() {
    let cube = populate_initial();
    assert_eq!(cube_hash(&cube), 0);
}

#[test]
fn test_find_entropy_initial() {
    let cube = populate_initial();
    assert_eq!(find_entropy(&cube), 0);
}

#[test]
fn test_cube_hash_test_cube() {
    let cube = test_cube_data();
    assert_eq!(cube_hash(&cube), 1900466);
}

#[test]
fn test_find_entropy_test_cube() {
    let cube = test_cube_data();
    assert_eq!(find_entropy(&cube), 25);
}

#[test]
fn test_cube_compare_equal() {
    let c1 = test_cube_data();
    let c2 = test_cube_data();
    assert_eq!(cube_compare_equal(&c1, &c2), true);
    let out = yellow_cw_output();
    assert_eq!(cube_compare_equal(&c1, &out), false);
}

#[test]
fn test_rotate_yellow_cw() {
    let mut cube = test_cube_data();
    let expected = yellow_cw_output();
    rotate_face(&mut cube, Color::Yellow, Rotation::CW);
    assert_eq!(cube_compare_equal(&cube, &expected), true);
    assert_eq!(cube_hash(&cube), 8191962);
    assert_eq!(find_entropy(&cube), 28);
}

#[test]
fn test_rotate_yellow_cw_then_ccw_roundtrip() {
    let original = test_cube_data();
    let mut cube = test_cube_data();
    rotate_face(&mut cube, Color::Yellow, Rotation::CW);
    rotate_face(&mut cube, Color::Yellow, Rotation::CCW);
    assert_eq!(cube_compare_equal(&cube, &original), true);
}

#[test]
fn test_rotate_red_cw_on_initial() {
    let mut cube = populate_initial();
    rotate_face(&mut cube, Color::Red, Rotation::CW);
    assert_eq!(cube_hash(&cube), 1278060);
    assert_eq!(find_entropy(&cube), 12);
    // Verify exact face data from C ground truth
    let expected: Cube = [
        [Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red],
        [Color::Blue, Color::Green, Color::Green, Color::Green, Color::Green, Color::Green, Color::Blue, Color::Blue],
        [Color::Yellow, Color::Yellow, Color::Yellow, Color::Blue, Color::Blue, Color::Blue, Color::Blue, Color::Blue],
        [Color::Orange, Color::Orange, Color::Orange, Color::Orange, Color::Orange, Color::Orange, Color::Orange, Color::Orange],
        [Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::White, Color::White, Color::White, Color::Yellow],
        [Color::White, Color::White, Color::Green, Color::Green, Color::Green, Color::White, Color::White, Color::White],
    ];
    assert_eq!(cube_compare_equal(&cube, &expected), true);
}

#[test]
fn test_rotate_green_ccw_on_initial() {
    let mut cube = populate_initial();
    rotate_face(&mut cube, Color::Green, Rotation::CCW);
    assert_eq!(cube_hash(&cube), 25242630);
    assert_eq!(find_entropy(&cube), 12);
    let expected: Cube = [
        [Color::Blue, Color::Blue, Color::Blue, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red],
        [Color::Green, Color::Green, Color::Green, Color::Green, Color::Green, Color::Green, Color::Green, Color::Green],
        [Color::Orange, Color::Blue, Color::Blue, Color::Blue, Color::Blue, Color::Blue, Color::Orange, Color::Orange],
        [Color::Orange, Color::Orange, Color::White, Color::White, Color::White, Color::Orange, Color::Orange, Color::Orange],
        [Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow],
        [Color::White, Color::White, Color::White, Color::White, Color::Red, Color::Red, Color::Red, Color::White],
    ];
    assert_eq!(cube_compare_equal(&cube, &expected), true);
}

#[test]
fn test_new_cube_rotate_face() {
    let cube = test_cube_data();
    let rotated = new_cube_rotate_face(&cube, Color::Yellow, Rotation::CW);
    let expected = yellow_cw_output();
    assert_eq!(cube_compare_equal(&rotated, &expected), true);
    // Original unchanged
    assert_eq!(cube_compare_equal(&cube, &test_cube_data()), true);
}

#[test]
fn test_ecube_new() {
    let cube = test_cube_data();
    let ec = ECube::new(cube);
    assert_eq!(ec.entropy, 25);
    assert_eq!(ec.hash, 1900466);
    assert_eq!(cube_compare_equal(&ec.cube, &cube), true);
}

fn main() {}
