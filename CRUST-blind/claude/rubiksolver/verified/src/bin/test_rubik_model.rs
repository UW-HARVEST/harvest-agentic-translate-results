use rubiksolver::rubik_model::{
    adjacent_ccw, adjacent_cw, adjacent_left, adjacent_right, cube_compare_equal, cube_hash,
    find_entropy, new_cube_rotate_face, populate_initial, populate_specific, rear, rotate_face,
    Color, Cube, ECube, Rotation, COLOR_CODE, TOP,
};

fn c(n: usize) -> Color {
    Color::from_usize(n)
}

#[test]
fn test_color_from_usize() {
    assert_eq!(Color::from_usize(0), Color::Red);
    assert_eq!(Color::from_usize(1), Color::Green);
    assert_eq!(Color::from_usize(2), Color::Blue);
    assert_eq!(Color::from_usize(3), Color::Orange);
    assert_eq!(Color::from_usize(4), Color::Yellow);
    assert_eq!(Color::from_usize(5), Color::White);
}

#[test]
fn test_color_code() {
    assert_eq!(COLOR_CODE[0], 'R');
    assert_eq!(COLOR_CODE[1], 'G');
    assert_eq!(COLOR_CODE[2], 'B');
    assert_eq!(COLOR_CODE[3], 'O');
    assert_eq!(COLOR_CODE[4], 'Y');
    assert_eq!(COLOR_CODE[5], 'W');
}

#[test]
fn test_top_constant() {
    assert_eq!(TOP[0], Color::Green);
    assert_eq!(TOP[1], Color::Blue);
    assert_eq!(TOP[2], Color::Red);
    assert_eq!(TOP[3], Color::White);
    assert_eq!(TOP[4], Color::Orange);
    assert_eq!(TOP[5], Color::Yellow);
}

#[test]
fn test_rear() {
    assert_eq!(rear(Color::Red), Color::Orange);
    assert_eq!(rear(Color::Green), Color::Yellow);
    assert_eq!(rear(Color::Blue), Color::White);
    assert_eq!(rear(Color::Orange), Color::Red);
    assert_eq!(rear(Color::Yellow), Color::Green);
    assert_eq!(rear(Color::White), Color::Blue);
    // REAR(REAR(x)) == x
    for n in 0..6 {
        let x = Color::from_usize(n);
        assert_eq!(rear(rear(x)), x);
    }
}

#[test]
fn test_adjacent_cw_documented() {
    // The C tests in rubik_model.c
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
fn test_adjacent_ccw_documented() {
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
fn test_adjacent_cw_complete() {
    // From C ground truth (excluded REAR pairs)
    // (face, around, expected)
    let cases: &[(usize, usize, usize)] = &[
        (0, 0, 5),
        (0, 1, 5),
        (0, 2, 1),
        (0, 4, 2),
        (0, 5, 4),
        (1, 0, 2),
        (1, 1, 2),
        (1, 2, 3),
        (1, 3, 5),
        (1, 5, 0),
        (2, 0, 4),
        (2, 1, 0),
        (2, 2, 1),
        (2, 3, 1),
        (2, 4, 3),
        (3, 1, 2),
        (3, 2, 4),
        (3, 3, 4),
        (3, 4, 5),
        (3, 5, 1),
        (4, 0, 5),
        (4, 2, 0),
        (4, 3, 2),
        (4, 4, 3),
        (4, 5, 3),
        (5, 0, 1),
        (5, 1, 3),
        (5, 3, 4),
        (5, 4, 0),
        (5, 5, 0),
    ];
    for (f, a, e) in cases {
        assert_eq!(
            adjacent_cw(c(*f), c(*a)),
            c(*e),
            "adjacent_cw({}, {}) should be {}",
            f,
            a,
            e
        );
    }
}

#[test]
fn test_adjacent_ccw_complete() {
    let cases: &[(usize, usize, usize)] = &[
        (0, 0, 1),
        (0, 1, 2),
        (0, 2, 4),
        (0, 4, 5),
        (0, 5, 1),
        (1, 0, 5),
        (1, 1, 0),
        (1, 2, 0),
        (1, 3, 2),
        (1, 5, 3),
        (2, 0, 1),
        (2, 1, 3),
        (2, 2, 3),
        (2, 3, 4),
        (2, 4, 0),
        (3, 1, 5),
        (3, 2, 1),
        (3, 3, 2),
        (3, 4, 2),
        (3, 5, 4),
        (4, 0, 2),
        (4, 2, 3),
        (4, 3, 5),
        (4, 4, 5),
        (4, 5, 0),
        (5, 0, 4),
        (5, 1, 0),
        (5, 3, 1),
        (5, 4, 3),
        (5, 5, 4),
    ];
    for (f, a, e) in cases {
        assert_eq!(
            adjacent_ccw(c(*f), c(*a)),
            c(*e),
            "adjacent_ccw({}, {}) should be {}",
            f,
            a,
            e
        );
    }
}

#[test]
fn test_adjacent_left_complete() {
    let cases: &[(usize, usize, usize)] = &[
        (0, 0, 5),
        (0, 1, 5),
        (0, 2, 1),
        (0, 3, 2),
        (0, 4, 2),
        (0, 5, 4),
        (1, 0, 5),
        (1, 1, 0),
        (1, 2, 0),
        (1, 3, 2),
        (1, 4, 3),
        (1, 5, 3),
        (2, 0, 4),
        (2, 1, 0),
        (2, 2, 1),
        (2, 3, 1),
        (2, 4, 3),
        (2, 5, 4),
        (3, 0, 5),
        (3, 1, 5),
        (3, 2, 1),
        (3, 3, 2),
        (3, 4, 2),
        (3, 5, 4),
        (4, 0, 5),
        (4, 1, 0),
        (4, 2, 0),
        (4, 3, 2),
        (4, 4, 3),
        (4, 5, 3),
        (5, 0, 4),
        (5, 1, 0),
        (5, 2, 1),
        (5, 3, 1),
        (5, 4, 3),
        (5, 5, 4),
    ];
    for (f, a, e) in cases {
        assert_eq!(
            adjacent_left(c(*f), c(*a)),
            c(*e),
            "adjacent_left({}, {}) should be {}",
            f,
            a,
            e
        );
    }
}

#[test]
fn test_adjacent_right_complete() {
    let cases: &[(usize, usize, usize)] = &[
        (0, 0, 1),
        (0, 1, 2),
        (0, 2, 4),
        (0, 3, 4),
        (0, 4, 5),
        (0, 5, 1),
        (1, 0, 2),
        (1, 1, 2),
        (1, 2, 3),
        (1, 3, 5),
        (1, 4, 5),
        (1, 5, 0),
        (2, 0, 1),
        (2, 1, 3),
        (2, 2, 3),
        (2, 3, 4),
        (2, 4, 0),
        (2, 5, 0),
        (3, 0, 1),
        (3, 1, 2),
        (3, 2, 4),
        (3, 3, 4),
        (3, 4, 5),
        (3, 5, 1),
        (4, 0, 2),
        (4, 1, 2),
        (4, 2, 3),
        (4, 3, 5),
        (4, 4, 5),
        (4, 5, 0),
        (5, 0, 1),
        (5, 1, 3),
        (5, 2, 3),
        (5, 3, 4),
        (5, 4, 0),
        (5, 5, 0),
    ];
    for (f, a, e) in cases {
        assert_eq!(
            adjacent_right(c(*f), c(*a)),
            c(*e),
            "adjacent_right({}, {}) should be {}",
            f,
            a,
            e
        );
    }
}

#[test]
fn test_populate_initial_solved() {
    let cube = populate_initial();
    for face in 0..6 {
        for pos in 0..8 {
            assert_eq!(cube[face][pos], Color::from_usize(face));
        }
    }
}

#[test]
fn test_populate_specific_copies_data() {
    let test_cube_data: Cube = [
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
    let cube = populate_specific(&test_cube_data);
    assert!(cube_compare_equal(&cube, &test_cube_data));
}

#[test]
fn test_cube_compare_equal_self_and_diff() {
    let solved = populate_initial();
    let solved2 = populate_initial();
    assert!(cube_compare_equal(&solved, &solved2));

    let mut altered = populate_initial();
    altered[0][0] = Color::Blue;
    assert!(!cube_compare_equal(&solved, &altered));
}

#[test]
fn test_find_entropy_solved() {
    let solved = populate_initial();
    assert_eq!(find_entropy(&solved), 0);
}

#[test]
fn test_find_entropy_test_cube() {
    let test_cube_data: Cube = [
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
    let cube = populate_specific(&test_cube_data);
    assert_eq!(find_entropy(&cube), 25);
}

#[test]
fn test_cube_hash_solved() {
    let solved = populate_initial();
    assert_eq!(cube_hash(&solved), 0);
}

#[test]
fn test_cube_hash_test_cube() {
    let test_cube_data: Cube = [
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
    let cube = populate_specific(&test_cube_data);
    assert_eq!(cube_hash(&cube), 1900466);
}

#[test]
fn test_ecube_init_solved() {
    let solved = populate_initial();
    let ecube = ECube::new(solved);
    assert_eq!(ecube.entropy, 0);
    assert_eq!(ecube.hash, 0);
    assert!(cube_compare_equal(&ecube.cube, &solved));
}

#[test]
fn test_ecube_init_scrambled() {
    let test_cube_data: Cube = [
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
    let cube = populate_specific(&test_cube_data);
    let ecube = ECube::new(cube);
    assert_eq!(ecube.entropy, 25);
    assert_eq!(ecube.hash, 1900466);
    assert!(cube_compare_equal(&ecube.cube, &test_cube_data));
}

// Helper to build expected cube from rows of u8
fn cube_from_data(rows: [[u8; 8]; 6]) -> Cube {
    let mut cube: Cube = [[Color::Red; 8]; 6];
    for f in 0..6 {
        for p in 0..8 {
            cube[f][p] = Color::from_usize(rows[f][p] as usize);
        }
    }
    cube
}

#[test]
fn test_rotate_face_red_cw() {
    let solved = populate_initial();
    let rotated = new_cube_rotate_face(&solved, Color::Red, Rotation::CW);
    let expected = cube_from_data([
        [0, 0, 0, 0, 0, 0, 0, 0],
        [2, 1, 1, 1, 1, 1, 2, 2],
        [4, 4, 4, 2, 2, 2, 2, 2],
        [3, 3, 3, 3, 3, 3, 3, 3],
        [4, 4, 4, 4, 5, 5, 5, 4],
        [5, 5, 1, 1, 1, 5, 5, 5],
    ]);
    assert!(cube_compare_equal(&rotated, &expected));
    assert_eq!(find_entropy(&rotated), 12);
    assert_eq!(cube_hash(&rotated), 1278060);
}

#[test]
fn test_rotate_face_red_ccw() {
    let solved = populate_initial();
    let rotated = new_cube_rotate_face(&solved, Color::Red, Rotation::CCW);
    let expected = cube_from_data([
        [0, 0, 0, 0, 0, 0, 0, 0],
        [5, 1, 1, 1, 1, 1, 5, 5],
        [1, 1, 1, 2, 2, 2, 2, 2],
        [3, 3, 3, 3, 3, 3, 3, 3],
        [4, 4, 4, 4, 2, 2, 2, 4],
        [5, 5, 4, 4, 4, 5, 5, 5],
    ]);
    assert!(cube_compare_equal(&rotated, &expected));
    assert_eq!(find_entropy(&rotated), 12);
    assert_eq!(cube_hash(&rotated), 1278060);
}

#[test]
fn test_rotate_face_green_cw() {
    let solved = populate_initial();
    let rotated = new_cube_rotate_face(&solved, Color::Green, Rotation::CW);
    let expected = cube_from_data([
        [5, 5, 5, 0, 0, 0, 0, 0],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [0, 2, 2, 2, 2, 2, 0, 0],
        [3, 3, 2, 2, 2, 3, 3, 3],
        [4, 4, 4, 4, 4, 4, 4, 4],
        [5, 5, 5, 5, 3, 3, 3, 5],
    ]);
    assert!(cube_compare_equal(&rotated, &expected));
    assert_eq!(find_entropy(&rotated), 12);
    assert_eq!(cube_hash(&rotated), 25242630);
}

#[test]
fn test_rotate_face_blue_cw() {
    let solved = populate_initial();
    let rotated = new_cube_rotate_face(&solved, Color::Blue, Rotation::CW);
    let expected = cube_from_data([
        [1, 0, 0, 0, 0, 0, 1, 1],
        [3, 3, 3, 1, 1, 1, 1, 1],
        [2, 2, 2, 2, 2, 2, 2, 2],
        [3, 3, 3, 3, 4, 4, 4, 3],
        [4, 4, 0, 0, 0, 4, 4, 4],
        [5, 5, 5, 5, 5, 5, 5, 5],
    ]);
    assert!(cube_compare_equal(&rotated, &expected));
    assert_eq!(find_entropy(&rotated), 12);
    assert_eq!(cube_hash(&rotated), 20448960);
}

#[test]
fn test_rotate_face_orange_cw() {
    let solved = populate_initial();
    let rotated = new_cube_rotate_face(&solved, Color::Orange, Rotation::CW);
    let expected = cube_from_data([
        [0, 0, 0, 0, 0, 0, 0, 0],
        [1, 1, 5, 5, 5, 1, 1, 1],
        [2, 2, 2, 2, 1, 1, 1, 2],
        [3, 3, 3, 3, 3, 3, 3, 3],
        [2, 2, 2, 4, 4, 4, 4, 4],
        [4, 5, 5, 5, 5, 5, 4, 4],
    ]);
    assert!(cube_compare_equal(&rotated, &expected));
    assert_eq!(find_entropy(&rotated), 12);
    assert_eq!(cube_hash(&rotated), 811410);
}

#[test]
fn test_rotate_face_yellow_cw() {
    let solved = populate_initial();
    let rotated = new_cube_rotate_face(&solved, Color::Yellow, Rotation::CW);
    let expected = cube_from_data([
        [0, 0, 0, 0, 2, 2, 2, 0],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [2, 2, 3, 3, 3, 2, 2, 2],
        [5, 3, 3, 3, 3, 3, 5, 5],
        [4, 4, 4, 4, 4, 4, 4, 4],
        [0, 0, 0, 5, 5, 5, 5, 5],
    ]);
    assert!(cube_compare_equal(&rotated, &expected));
    assert_eq!(find_entropy(&rotated), 12);
    assert_eq!(cube_hash(&rotated), 6345240);
}

#[test]
fn test_rotate_face_white_cw() {
    let solved = populate_initial();
    let rotated = new_cube_rotate_face(&solved, Color::White, Rotation::CW);
    let expected = cube_from_data([
        [0, 0, 4, 4, 4, 0, 0, 0],
        [1, 1, 1, 1, 0, 0, 0, 1],
        [2, 2, 2, 2, 2, 2, 2, 2],
        [1, 1, 1, 3, 3, 3, 3, 3],
        [3, 4, 4, 4, 4, 4, 3, 3],
        [5, 5, 5, 5, 5, 5, 5, 5],
    ]);
    assert!(cube_compare_equal(&rotated, &expected));
    assert_eq!(find_entropy(&rotated), 12);
    assert_eq!(cube_hash(&rotated), 12982560);
}

#[test]
fn test_rotate_face_test_cube_yellow_cw() {
    // Test the documented rotate_face test case
    let test_cube_data: Cube = [
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
    let mut cube = populate_specific(&test_cube_data);
    let original = populate_specific(&test_cube_data);

    let expected_after_yellow_cw = cube_from_data([
        [0, 0, 0, 0, 2, 3, 3, 0],
        [1, 4, 2, 3, 3, 4, 1, 1],
        [2, 2, 4, 2, 5, 1, 3, 2],
        [5, 5, 5, 1, 1, 5, 4, 3],
        [4, 2, 3, 1, 2, 4, 4, 4],
        [0, 0, 0, 5, 5, 5, 1, 3],
    ]);

    rotate_face(&mut cube, Color::Yellow, Rotation::CW);
    assert!(cube_compare_equal(&cube, &expected_after_yellow_cw));
    assert_eq!(find_entropy(&cube), 28);
    assert_eq!(cube_hash(&cube), 8191962);

    // Reverse it (CCW)
    rotate_face(&mut cube, Color::Yellow, Rotation::CCW);
    assert!(cube_compare_equal(&cube, &original));
}

#[test]
fn test_rotate_4x_cw_identity() {
    let solved = populate_initial();
    for face_idx in 0..6 {
        let face = Color::from_usize(face_idx);
        let mut c = solved;
        rotate_face(&mut c, face, Rotation::CW);
        rotate_face(&mut c, face, Rotation::CW);
        rotate_face(&mut c, face, Rotation::CW);
        rotate_face(&mut c, face, Rotation::CW);
        assert!(
            cube_compare_equal(&c, &solved),
            "4xCW for face {} should be identity",
            face_idx
        );
    }
}

#[test]
fn test_rotate_4x_ccw_identity() {
    let solved = populate_initial();
    for face_idx in 0..6 {
        let face = Color::from_usize(face_idx);
        let mut c = solved;
        rotate_face(&mut c, face, Rotation::CCW);
        rotate_face(&mut c, face, Rotation::CCW);
        rotate_face(&mut c, face, Rotation::CCW);
        rotate_face(&mut c, face, Rotation::CCW);
        assert!(
            cube_compare_equal(&c, &solved),
            "4xCCW for face {} should be identity",
            face_idx
        );
    }
}

#[test]
fn test_rotate_cw_then_ccw_identity() {
    let solved = populate_initial();
    for face_idx in 0..6 {
        let face = Color::from_usize(face_idx);
        let mut c = solved;
        rotate_face(&mut c, face, Rotation::CW);
        rotate_face(&mut c, face, Rotation::CCW);
        assert!(
            cube_compare_equal(&c, &solved),
            "CW then CCW for face {} should be identity",
            face_idx
        );
    }
}

#[test]
fn test_new_cube_rotate_face_does_not_mutate_input() {
    let solved = populate_initial();
    let _rotated = new_cube_rotate_face(&solved, Color::Red, Rotation::CW);
    // solved should remain unchanged
    assert_eq!(find_entropy(&solved), 0);
    let solved2 = populate_initial();
    assert!(cube_compare_equal(&solved, &solved2));
}

fn main() {}
