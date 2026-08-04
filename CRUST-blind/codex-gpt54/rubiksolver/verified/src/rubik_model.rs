use std::fmt;
/// The six colors (and faces) of the cube.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Red = 0,
    Green = 1,
    Blue = 2,
    Orange = 3,
    Yellow = 4,
    White = 5,
}
impl Color {
    /// Create a Color from a usize (0–5).
    pub fn from_usize(n: usize) -> Self {
        assert!(n < 6, "Invalid color index");
        [
            Color::Red,
            Color::Green,
            Color::Blue,
            Color::Orange,
            Color::Yellow,
            Color::White,
        ][n]
    }
}
/// Color codes used for printing.
pub const COLOR_CODE: [char; 6] = ['R', 'G', 'B', 'O', 'Y', 'W'];
/// Mapping for “top” face (as in the original C code).
/// For a given face, TOP[face] is used to determine adjacent faces.
pub const TOP: [Color; 6] = [
    Color::Green,
    Color::Blue,
    Color::Red,
    Color::White,
    Color::Orange,
    Color::Yellow,
];
/// Rotation direction.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Rotation {
    CW,
    CCW,
}
/// A cube is represented as 6 faces, each with 8 positions.
/// (In the C code, cube_t was a pointer‐to‑array; here we use a fixed-size 2D array.)
pub type Cube = [[Color; 8]; 6];
/// An “entropy‐cube” combines a cube with its computed entropy and hash.
#[derive(Clone, Debug)]
pub struct ECube {
    pub cube: Cube,
    pub entropy: i32,
    pub hash: u32,
}
impl ECube {
    /// Initializes an ECube from a given cube.
    pub fn new(cube: Cube) -> Self {
        let entropy = find_entropy(&cube);
        let hash = cube_hash(&cube);
        ECube {
            cube,
            entropy,
            hash,
        }
    }
}
/// Returns a cube in its initial (solved) state.
pub fn populate_initial() -> Cube {
    let mut cube = [[Color::Red; 8]; 6];
    for (face, face_data) in cube.iter_mut().enumerate() {
        let color = Color::from_usize(face);
        face_data.fill(color);
    }
    cube
}
/// Creates a cube from provided data.
pub fn populate_specific(data: &Cube) -> Cube {
    *data
}
/// Compares two cubes for equality.
pub fn cube_compare_equal(c1: &Cube, c2: &Cube) -> bool {
    c1 == c2
}
/// Computes a hash value for a cube.
/// For each face and for positions in pairs, if the face’s color does not match the expected one, increment the hash.
pub fn cube_hash(cube: &Cube) -> u32 {
    let mut hash = 0_u32;
    for (face, face_data) in cube.iter().enumerate() {
        let expected = Color::from_usize(face);
        for pos in 0..4 {
            if face_data[2 * pos] != expected || face_data[2 * pos + 1] != expected {
                hash += 1;
            }
            hash <<= 1;
        }
    }
    hash
}
/// Counts the number of “misplaced” positions.
pub fn find_entropy(cube: &Cube) -> i32 {
    let mut count = 0;
    for (face, face_data) in cube.iter().enumerate() {
        let expected = Color::from_usize(face);
        for sticker in face_data {
            if *sticker != expected {
                count += 1;
            }
        }
    }
    count
}
// --- Helper functions for adjacent faces and rotations ---
fn cycle_l(color: Color) -> Color {
    Color::from_usize((color as usize + 5) % 6)
}
fn cycle_r(color: Color) -> Color {
    Color::from_usize((color as usize + 1) % 6)
}
pub fn rear(color: Color) -> Color {
    Color::from_usize((color as usize + 3) % 6)
}
/// Returns the “adjacent left” face.
pub fn adjacent_left(rotating_face: Color, around: Color) -> Color {
    let left_around = cycle_l(around);
    if left_around == rear(rotating_face) || left_around == rotating_face {
        cycle_l(left_around)
    } else {
        left_around
    }
}
/// Returns the “adjacent right” face.
pub fn adjacent_right(rotating_face: Color, around: Color) -> Color {
    let right_around = cycle_r(around);
    if right_around == rear(rotating_face) || right_around == rotating_face {
        cycle_r(right_around)
    } else {
        right_around
    }
}
/// Computes the adjacent face in the clockwise direction.
pub fn adjacent_cw(rotating_face: Color, around: Color) -> Color {
    assert!(around != rear(rotating_face));
    if (rotating_face as u8) % 2 == 1 {
        adjacent_right(rotating_face, around)
    } else {
        adjacent_left(rotating_face, around)
    }
}
/// Computes the adjacent face in the counter‑clockwise direction.
pub fn adjacent_ccw(rotating_face: Color, around: Color) -> Color {
    assert!(around != rear(rotating_face));
    if (rotating_face as u8) % 2 == 1 {
        adjacent_left(rotating_face, around)
    } else {
        adjacent_right(rotating_face, around)
    }
}
/// Rotates a face of the cube in place.
/// The swapping operations mimic the series of SWAP_COLOR macros.
pub fn rotate_face(cube: &mut Cube, face: Color, direction: Rotation) {
    let face_ix = face as usize;
    let g = TOP[face_ix];
    let w = adjacent_cw(face, g);
    let y = adjacent_cw(face, w);
    let b = adjacent_cw(face, y);

    let g_ix = g as usize;
    let w_ix = w as usize;
    let y_ix = y as usize;
    let b_ix = b as usize;

    match direction {
        Rotation::CW => {
            swap_cells(cube, g_ix, 0, w_ix, 4);
            swap_cells(cube, g_ix, 7, w_ix, 3);
            swap_cells(cube, g_ix, 6, w_ix, 2);

            swap_cells(cube, b_ix, 2, g_ix, 0);
            swap_cells(cube, b_ix, 1, g_ix, 7);
            swap_cells(cube, b_ix, 0, g_ix, 6);

            swap_cells(cube, y_ix, 6, b_ix, 2);
            swap_cells(cube, y_ix, 5, b_ix, 1);
            swap_cells(cube, y_ix, 4, b_ix, 0);

            cube[face_ix].swap(0, 6);
            cube[face_ix].swap(6, 4);
            cube[face_ix].swap(4, 2);
            cube[face_ix].swap(1, 7);
            cube[face_ix].swap(7, 5);
            cube[face_ix].swap(5, 3);
        }
        Rotation::CCW => {
            swap_cells(cube, g_ix, 0, b_ix, 2);
            swap_cells(cube, g_ix, 7, b_ix, 1);
            swap_cells(cube, g_ix, 6, b_ix, 0);

            swap_cells(cube, w_ix, 4, g_ix, 0);
            swap_cells(cube, w_ix, 3, g_ix, 7);
            swap_cells(cube, w_ix, 2, g_ix, 6);

            swap_cells(cube, y_ix, 6, w_ix, 4);
            swap_cells(cube, y_ix, 5, w_ix, 3);
            swap_cells(cube, y_ix, 4, w_ix, 2);

            cube[face_ix].swap(0, 2);
            cube[face_ix].swap(2, 4);
            cube[face_ix].swap(4, 6);
            cube[face_ix].swap(1, 3);
            cube[face_ix].swap(3, 5);
            cube[face_ix].swap(5, 7);
        }
    }
}
/// Returns a new cube that is the result of rotating a face.
pub fn new_cube_rotate_face(cube: &Cube, face: Color, direction: Rotation) -> Cube {
    let mut new_cube = *cube;
    rotate_face(&mut new_cube, face, direction);
    new_cube
}
/// Prints the cube in a formatted way.
pub fn print_cube(cube: &Cube) {
    for (face, face_data) in cube.iter().enumerate() {
        println!(
            "{} {} {}\n{} {} {}\n{} {} {}",
            COLOR_CODE[face_data[0] as usize],
            COLOR_CODE[face_data[1] as usize],
            COLOR_CODE[face_data[2] as usize],
            COLOR_CODE[face_data[7] as usize],
            COLOR_CODE[face],
            COLOR_CODE[face_data[3] as usize],
            COLOR_CODE[face_data[6] as usize],
            COLOR_CODE[face_data[5] as usize],
            COLOR_CODE[face_data[4] as usize]
        );
        println!("-----");
    }
}

fn swap_cells(cube: &mut Cube, face1: usize, pos1: usize, face2: usize, pos2: usize) {
    let tmp = cube[face1][pos1];
    cube[face1][pos1] = cube[face2][pos2];
    cube[face2][pos2] = tmp;
}
