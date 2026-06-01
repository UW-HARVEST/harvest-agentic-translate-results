use rubiksolver::rubik_model::{
    adjacent_ccw, adjacent_cw, adjacent_left, adjacent_right, cube_compare_equal, cube_hash,
    find_entropy, new_cube_rotate_face, populate_initial, populate_specific, rear, rotate_face,
    Color, Cube, ECube, Rotation, COLOR_CODE, TOP,
};

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
fn test_top_constants() {
    assert_eq!(TOP[Color::Red as usize], Color::Green);
    assert_eq!(TOP[Color::Green as usize], Color::Blue);
    assert_eq!(TOP[Color::Blue as usize], Color::Red);
    assert_eq!(TOP[Color::Orange as usize], Color::White);
    assert_eq!(TOP[Color::Yellow as usize], Color::Orange);
    assert_eq!(TOP[Color::White as usize], Color::Yellow);
}

#[test]
fn test_rear() {
    assert_eq!(rear(Color::Red), Color::Orange);
    assert_eq!(rear(Color::Green), Color::Yellow);
    assert_eq!(rear(Color::Blue), Color::White);
    assert_eq!(rear(Color::Orange), Color::Red);
    assert_eq!(rear(Color::Yellow), Color::Green);
    assert_eq!(rear(Color::White), Color::Blue);
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
fn test_adjacent_left_right() {
    // Spot check using values that don't trigger the assert in cw/ccw
    // adjacent_left(RED, YELLOW): l_around = CYCLE_L(YELLOW)=ORANGE, REAR(RED)=ORANGE
    // since l_around==REAR, returns CYCLE_L(ORANGE)=BLUE
    assert_eq!(adjacent_left(Color::Red, Color::Yellow), Color::Blue);
    // adjacent_right(RED, YELLOW) -> r_around = CYCLE_R(YELLOW)=WHITE, REAR(RED)=ORANGE,
    // WHITE!=ORANGE, WHITE!=RED -> returns WHITE
    assert_eq!(adjacent_right(Color::Red, Color::Yellow), Color::White);
    // adjacent_left(RED, GREEN) -> l_around = CYCLE_L(GREEN)=RED, REAR(RED)=ORANGE,
    // RED==RED -> returns CYCLE_L(RED) = WHITE
    assert_eq!(adjacent_left(Color::Red, Color::Green), Color::White);
    // adjacent_right(RED, BLUE) -> r_around = CYCLE_R(BLUE)=ORANGE = REAR(RED) -> returns CYCLE_R(ORANGE)=YELLOW
    assert_eq!(adjacent_right(Color::Red, Color::Blue), Color::Yellow);
}

#[test]
fn test_populate_initial() {
    let cube = populate_initial();
    for face_idx in 0..6 {
        let face_color = Color::from_usize(face_idx);
        for pos in 0..8 {
            assert_eq!(cube[face_idx][pos], face_color);
        }
    }
}

#[test]
fn test_populate_specific() {
    let data: Cube = [
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
    let cube = populate_specific(&data);
    for face in 0..6 {
        for pos in 0..8 {
            assert_eq!(cube[face][pos], data[face][pos]);
        }
    }
}

#[test]
fn test_cube_compare_equal() {
    let initial = populate_initial();
    let initial2 = populate_initial();
    assert!(cube_compare_equal(&initial, &initial2));

    let mut modified = populate_initial();
    modified[0][0] = Color::Blue;
    assert!(!cube_compare_equal(&initial, &modified));
}

#[test]
fn test_cube_hash_solved() {
    let initial = populate_initial();
    assert_eq!(cube_hash(&initial), 0);
}

#[test]
fn test_cube_hash_specific() {
    let data: Cube = [
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
    let cube = populate_specific(&data);
    // Computed via running C code
    assert_eq!(cube_hash(&cube), 1900466);
}

#[test]
fn test_find_entropy() {
    let initial = populate_initial();
    assert_eq!(find_entropy(&initial), 0);

    let data: Cube = [
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
    let cube = populate_specific(&data);
    assert_eq!(find_entropy(&cube), 25);
}

#[test]
fn test_rotate_face_yellow_cw() {
    // Verify the rotate matches the expected output1 from C tests
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
    let mut cube = populate_specific(&test_data);
    rotate_face(&mut cube, Color::Yellow, Rotation::CW);

    let expected: Cube = [
        [
            Color::Red,
            Color::Red,
            Color::Red,
            Color::Red,
            Color::Blue,
            Color::Orange,
            Color::Orange,
            Color::Red,
        ],
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
            Color::Yellow,
            Color::Blue,
            Color::White,
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
            Color::Orange,
        ],
        [
            Color::Yellow,
            Color::Blue,
            Color::Orange,
            Color::Green,
            Color::Blue,
            Color::Yellow,
            Color::Yellow,
            Color::Yellow,
        ],
        [
            Color::Red,
            Color::Red,
            Color::Red,
            Color::White,
            Color::White,
            Color::White,
            Color::Green,
            Color::Orange,
        ],
    ];
    assert!(cube_compare_equal(&cube, &expected));

    // Now CCW back
    rotate_face(&mut cube, Color::Yellow, Rotation::CCW);
    let original = populate_specific(&test_data);
    assert!(cube_compare_equal(&cube, &original));
}

#[test]
fn test_new_cube_rotate_face_red_cw_solved() {
    let initial = populate_initial();
    let rotated = new_cube_rotate_face(&initial, Color::Red, Rotation::CW);
    // Initial cube unchanged
    assert!(cube_compare_equal(&initial, &populate_initial()));
    // Rotated has expected entropy and hash from C
    assert_eq!(find_entropy(&rotated), 12);
    assert_eq!(cube_hash(&rotated), 1278060);

    // Verify exact face data computed from C output:
    // face 0: 0 0 0 0 0 0 0 0
    // face 1: 2 1 1 1 1 1 2 2
    // face 2: 4 4 4 2 2 2 2 2
    // face 3: 3 3 3 3 3 3 3 3
    // face 4: 4 4 4 4 5 5 5 4
    // face 5: 5 5 1 1 1 5 5 5
    let expected: Cube = [
        [Color::Red; 8],
        [
            Color::Blue,
            Color::Green,
            Color::Green,
            Color::Green,
            Color::Green,
            Color::Green,
            Color::Blue,
            Color::Blue,
        ],
        [
            Color::Yellow,
            Color::Yellow,
            Color::Yellow,
            Color::Blue,
            Color::Blue,
            Color::Blue,
            Color::Blue,
            Color::Blue,
        ],
        [Color::Orange; 8],
        [
            Color::Yellow,
            Color::Yellow,
            Color::Yellow,
            Color::Yellow,
            Color::White,
            Color::White,
            Color::White,
            Color::Yellow,
        ],
        [
            Color::White,
            Color::White,
            Color::Green,
            Color::Green,
            Color::Green,
            Color::White,
            Color::White,
            Color::White,
        ],
    ];
    assert!(cube_compare_equal(&rotated, &expected));

    // Reverse CCW restores
    let restored = new_cube_rotate_face(&rotated, Color::Red, Rotation::CCW);
    assert!(cube_compare_equal(&restored, &initial));
}

#[test]
fn test_rotate_all_faces_solved_cube_hashes() {
    // Computed from C run for the initial solved cube.
    // CW results equal CCW results because of symmetry of solved cube.
    let initial = populate_initial();
    let test_cases = [
        (Color::Red, Rotation::CW, 12, 1278060u32),
        (Color::Red, Rotation::CCW, 12, 1278060u32),
        (Color::Green, Rotation::CW, 12, 25242630u32),
        (Color::Green, Rotation::CCW, 12, 25242630u32),
        (Color::Blue, Rotation::CW, 12, 20448960u32),
        (Color::Blue, Rotation::CCW, 12, 20448960u32),
        (Color::Orange, Rotation::CW, 12, 811410u32),
        (Color::Orange, Rotation::CCW, 12, 811410u32),
        (Color::Yellow, Rotation::CW, 12, 6345240u32),
        (Color::Yellow, Rotation::CCW, 12, 6345240u32),
        (Color::White, Rotation::CW, 12, 12982560u32),
        (Color::White, Rotation::CCW, 12, 12982560u32),
    ];
    for (face, dir, exp_entropy, exp_hash) in test_cases.iter() {
        let r = new_cube_rotate_face(&initial, *face, *dir);
        assert_eq!(find_entropy(&r), *exp_entropy, "entropy fail face={:?} dir={:?}", face, dir);
        assert_eq!(cube_hash(&r), *exp_hash, "hash fail face={:?} dir={:?}", face, dir);
    }
}

#[test]
fn test_ecube_new() {
    let initial = populate_initial();
    let ecube = ECube::new(initial);
    assert_eq!(ecube.entropy, 0);
    assert_eq!(ecube.hash, 0);

    // Now an interesting cube
    let rotated = new_cube_rotate_face(&populate_initial(), Color::Red, Rotation::CW);
    let ecube2 = ECube::new(rotated);
    assert_eq!(ecube2.entropy, 12);
    assert_eq!(ecube2.hash, 1278060);
}

fn main() {}
