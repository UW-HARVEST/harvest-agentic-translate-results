use rubiksolver::rubik_model::*;

fn main() {
    println!("Running rubik_model tests");

    println!("Testing rear...");
    assert_eq!(rear(Color::Red), Color::Orange);
    assert_eq!(rear(Color::Green), Color::Yellow);
    assert_eq!(rear(Color::Blue), Color::White);
    assert_eq!(rear(rear(Color::Red)), Color::Red);
    assert_eq!(rear(rear(Color::Green)), Color::Green);
    assert_eq!(rear(rear(Color::Blue)), Color::Blue);

    // Test adjacent functions
    println!("Testing adjacent_cw...");
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

    println!("Testing adjacent_ccw...");
    assert_eq!(adjacent_ccw(Color::Red, Color::Blue), Color::Yellow);
    // C code: assert_verbose(adjacent_ccw(RED, GREEN == BLUE)) — GREEN==BLUE is 0 (RED),
    // assert just checks result is truthy (non-zero/non-RED)
    assert_ne!(adjacent_ccw(Color::Red, Color::Red), Color::Red);
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

    println!("Testing cube_compare_equal...");
    let test_cube_data: Cube = [
        [Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red],
        [Color::Green, Color::Yellow, Color::Blue, Color::Orange, Color::Orange, Color::Yellow, Color::Green, Color::Green],
        [Color::Blue, Color::Blue, Color::Blue, Color::Orange, Color::Orange, Color::Green, Color::Orange, Color::Blue],
        [Color::White, Color::White, Color::White, Color::Green, Color::Green, Color::White, Color::Yellow, Color::Blue],
        [Color::Orange, Color::Green, Color::Blue, Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::Blue],
        [Color::Yellow, Color::Orange, Color::White, Color::White, Color::White, Color::White, Color::Green, Color::Orange],
    ];
    let test_cube = populate_specific(&test_cube_data);
    let original = populate_specific(&test_cube_data);

    let output1_data: Cube = [
        [Color::Red, Color::Red, Color::Red, Color::Red, Color::Blue, Color::Orange, Color::Orange, Color::Red],
        [Color::Green, Color::Yellow, Color::Blue, Color::Orange, Color::Orange, Color::Yellow, Color::Green, Color::Green],
        [Color::Blue, Color::Blue, Color::Yellow, Color::Blue, Color::White, Color::Green, Color::Orange, Color::Blue],
        [Color::White, Color::White, Color::White, Color::Green, Color::Green, Color::White, Color::Yellow, Color::Orange],
        [Color::Yellow, Color::Blue, Color::Orange, Color::Green, Color::Blue, Color::Yellow, Color::Yellow, Color::Yellow],
        [Color::Red, Color::Red, Color::Red, Color::White, Color::White, Color::White, Color::Green, Color::Orange],
    ];
    let output1 = populate_specific(&output1_data);

    assert!(cube_compare_equal(&test_cube, &original));
    assert!(!cube_compare_equal(&test_cube, &output1));

    println!("Testing rotate_face...");
    let mut test_cube = test_cube;
    rotate_face(&mut test_cube, Color::Yellow, Rotation::CW);
    assert!(cube_compare_equal(&test_cube, &output1));
    rotate_face(&mut test_cube, Color::Yellow, Rotation::CCW);
    assert!(cube_compare_equal(&test_cube, &original));

    println!("Testing find_entropy...");
    assert_eq!(find_entropy(&original), 25);

    println!("rubik_model tests finished successfully");
}
