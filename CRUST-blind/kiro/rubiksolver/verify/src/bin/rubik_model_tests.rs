use rubiksolver::rubik_model::*;

#[test]
fn test_rear() {
    // REAR = [3,4,5,0,1,2]
    assert_eq!(rear(Color::Red) as u8, 3);     // Orange
    assert_eq!(rear(Color::Green) as u8, 4);    // Yellow
    assert_eq!(rear(Color::Blue) as u8, 5);     // White
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
    assert!(cube_compare_equal(&c1, &c2));
    let initial = populate_initial();
    assert!(!cube_compare_equal(&c1, &initial));
}

#[test]
fn test_rotate_face_yellow_cw() {
    let mut cube = test_cube_data();
    let expected: Cube = [
        [Color::Red, Color::Red, Color::Red, Color::Red, Color::Blue, Color::Orange, Color::Orange, Color::Red],
        [Color::Green, Color::Yellow, Color::Blue, Color::Orange, Color::Orange, Color::Yellow, Color::Green, Color::Green],
        [Color::Blue, Color::Blue, Color::Yellow, Color::Blue, Color::White, Color::Green, Color::Orange, Color::Blue],
        [Color::White, Color::White, Color::White, Color::Green, Color::Green, Color::White, Color::Yellow, Color::Orange],
        [Color::Yellow, Color::Blue, Color::Orange, Color::Green, Color::Blue, Color::Yellow, Color::Yellow, Color::Yellow],
        [Color::Red, Color::Red, Color::Red, Color::White, Color::White, Color::White, Color::Green, Color::Orange],
    ];
    rotate_face(&mut cube, Color::Yellow, Rotation::CW);
    assert!(cube_compare_equal(&cube, &expected));
}

#[test]
fn test_rotate_face_yellow_cw_then_ccw_roundtrip() {
    let original = test_cube_data();
    let mut cube = test_cube_data();
    rotate_face(&mut cube, Color::Yellow, Rotation::CW);
    rotate_face(&mut cube, Color::Yellow, Rotation::CCW);
    assert!(cube_compare_equal(&cube, &original));
}

#[test]
fn test_rotate_face_red_cw_on_initial() {
    let mut cube = populate_initial();
    rotate_face(&mut cube, Color::Red, Rotation::CW);
    // Ground truth from C:
    // face 0: 0 0 0 0 0 0 0 0
    // face 1: 2 1 1 1 1 1 2 2
    // face 2: 4 4 4 2 2 2 2 2
    // face 3: 3 3 3 3 3 3 3 3
    // face 4: 4 4 4 4 5 5 5 4
    // face 5: 5 5 1 1 1 5 5 5
    let expected: Cube = [
        [Color::Red; 8],
        [Color::Blue, Color::Green, Color::Green, Color::Green, Color::Green, Color::Green, Color::Blue, Color::Blue],
        [Color::Yellow, Color::Yellow, Color::Yellow, Color::Blue, Color::Blue, Color::Blue, Color::Blue, Color::Blue],
        [Color::Orange; 8],
        [Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::White, Color::White, Color::White, Color::Yellow],
        [Color::White, Color::White, Color::Green, Color::Green, Color::Green, Color::White, Color::White, Color::White],
    ];
    assert!(cube_compare_equal(&cube, &expected));
    assert_eq!(find_entropy(&cube), 12);
    assert_eq!(cube_hash(&cube), 1278060);
}

#[test]
fn test_rotate_face_red_ccw_on_initial() {
    let mut cube = populate_initial();
    rotate_face(&mut cube, Color::Red, Rotation::CCW);
    // face 0: 0 0 0 0 0 0 0 0
    // face 1: 5 1 1 1 1 1 5 5
    // face 2: 1 1 1 2 2 2 2 2
    // face 3: 3 3 3 3 3 3 3 3
    // face 4: 4 4 4 4 2 2 2 4
    // face 5: 5 5 4 4 4 5 5 5
    let expected: Cube = [
        [Color::Red; 8],
        [Color::White, Color::Green, Color::Green, Color::Green, Color::Green, Color::Green, Color::White, Color::White],
        [Color::Green, Color::Green, Color::Green, Color::Blue, Color::Blue, Color::Blue, Color::Blue, Color::Blue],
        [Color::Orange; 8],
        [Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::Blue, Color::Blue, Color::Blue, Color::Yellow],
        [Color::White, Color::White, Color::Yellow, Color::Yellow, Color::Yellow, Color::White, Color::White, Color::White],
    ];
    assert!(cube_compare_equal(&cube, &expected));
}

#[test]
fn test_rotate_red_cw_ccw_roundtrip_initial() {
    let initial = populate_initial();
    let mut cube = populate_initial();
    rotate_face(&mut cube, Color::Red, Rotation::CW);
    rotate_face(&mut cube, Color::Red, Rotation::CCW);
    assert!(cube_compare_equal(&cube, &initial));
}

#[test]
fn test_new_cube_rotate_face() {
    let cube = test_cube_data();
    let rotated = new_cube_rotate_face(&cube, Color::Yellow, Rotation::CW);
    assert_eq!(cube_hash(&rotated), 8191962);
    assert_eq!(find_entropy(&rotated), 28);
    // Original should be unchanged
    assert_eq!(cube_hash(&cube), 1900466);
}

#[test]
fn test_rotate_green_cw_on_initial() {
    let mut cube = populate_initial();
    rotate_face(&mut cube, Color::Green, Rotation::CW);
    // face 0: 5 5 5 0 0 0 0 0
    // face 1: 1 1 1 1 1 1 1 1
    // face 2: 0 2 2 2 2 2 0 0
    // face 3: 3 3 2 2 2 3 3 3
    // face 4: 4 4 4 4 4 4 4 4
    // face 5: 5 5 5 5 3 3 3 5
    let expected: Cube = [
        [Color::White, Color::White, Color::White, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red],
        [Color::Green; 8],
        [Color::Red, Color::Blue, Color::Blue, Color::Blue, Color::Blue, Color::Blue, Color::Red, Color::Red],
        [Color::Orange, Color::Orange, Color::Blue, Color::Blue, Color::Blue, Color::Orange, Color::Orange, Color::Orange],
        [Color::Yellow; 8],
        [Color::White, Color::White, Color::White, Color::White, Color::Orange, Color::Orange, Color::Orange, Color::White],
    ];
    assert!(cube_compare_equal(&cube, &expected));
}

#[test]
fn test_ecube_new() {
    let cube = test_cube_data();
    let ecube = ECube::new(cube);
    assert_eq!(ecube.entropy, 25);
    assert_eq!(ecube.hash, 1900466);
}

#[test]
fn test_populate_specific() {
    let data = test_cube_data();
    let cube = populate_specific(&data);
    assert!(cube_compare_equal(&cube, &data));
}

fn main() {}
