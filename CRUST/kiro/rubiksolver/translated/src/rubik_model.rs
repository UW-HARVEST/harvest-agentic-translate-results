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
        match n {
            0 => Color::Red,
            1 => Color::Green,
            2 => Color::Blue,
            3 => Color::Orange,
            4 => Color::Yellow,
            5 => Color::White,
            _ => panic!("Invalid color index"),
        }
    }
}
/// Color codes used for printing.
pub const COLOR_CODE: [char; 6] = ['R', 'G', 'B', 'O', 'Y', 'W'];
/// Mapping for "top" face (as in the original C code).
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
/// An "entropy‐cube" combines a cube with its computed entropy and hash.
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
    for face in 0..6 {
        let c = Color::from_usize(face);
        cube[face] = [c; 8];
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
pub fn cube_hash(cube: &Cube) -> u32 {
    let mut hash: u32 = 0;
    for face in 0..6usize {
        let c = Color::from_usize(face);
        for pos in 0..4 {
            if cube[face][2 * pos] != c || cube[face][2 * pos + 1] != c {
                hash += 1;
            }
            hash <<= 1;
        }
    }
    hash
}
/// Counts the number of "misplaced" positions.
pub fn find_entropy(cube: &Cube) -> i32 {
    let mut count = 0;
    for face in 0..6usize {
        let c = Color::from_usize(face);
        for pos in 0..8 {
            if cube[face][pos] != c {
                count += 1;
            }
        }
    }
    count
}
// --- Helper functions for adjacent faces and rotations ---
fn cycle_l(color: Color) -> Color {
    Color::from_usize(((color as usize) + 5) % 6)
}
fn cycle_r(color: Color) -> Color {
    Color::from_usize(((color as usize) + 1) % 6)
}
pub fn rear(color: Color) -> Color {
    Color::from_usize(((color as usize) + 3) % 6)
}
/// Returns the "adjacent left" face.
pub fn adjacent_left(rotating_face: Color, around: Color) -> Color {
    let l = cycle_l(around);
    if l == rear(rotating_face) || l == rotating_face {
        cycle_l(l)
    } else {
        l
    }
}
/// Returns the "adjacent right" face.
pub fn adjacent_right(rotating_face: Color, around: Color) -> Color {
    let r = cycle_r(around);
    if r == rear(rotating_face) || r == rotating_face {
        cycle_r(r)
    } else {
        r
    }
}
/// Computes the adjacent face in the clockwise direction.
pub fn adjacent_cw(rotating_face: Color, around: Color) -> Color {
    if (rotating_face as u8) % 2 == 1 {
        adjacent_right(rotating_face, around)
    } else {
        adjacent_left(rotating_face, around)
    }
}
/// Computes the adjacent face in the counter‑clockwise direction.
pub fn adjacent_ccw(rotating_face: Color, around: Color) -> Color {
    if (rotating_face as u8) % 2 == 1 {
        adjacent_left(rotating_face, around)
    } else {
        adjacent_right(rotating_face, around)
    }
}
/// Rotates a face of the cube in place.
pub fn rotate_face(cube: &mut Cube, face: Color, direction: Rotation) {
    let f = face as usize;
    let g = TOP[f] as usize;
    let w = adjacent_cw(face, TOP[f]) as usize;
    let y = adjacent_cw(face, Color::from_usize(w)) as usize;
    let b = adjacent_cw(face, Color::from_usize(y)) as usize;

    if direction == Rotation::CW {
        // Swap G's (0,7,6) with W's (4,3,2)
        swap_pos(cube, g, 0, w, 4);
        swap_pos(cube, g, 7, w, 3);
        swap_pos(cube, g, 6, w, 2);
        // Swap B's (2,1,0) with G's (0,7,6)
        swap_pos(cube, b, 2, g, 0);
        swap_pos(cube, b, 1, g, 7);
        swap_pos(cube, b, 0, g, 6);
        // Swap Y's (6,5,4) with B's (2,1,0)
        swap_pos(cube, y, 6, b, 2);
        swap_pos(cube, y, 5, b, 1);
        swap_pos(cube, y, 4, b, 0);
        // Rotating face CW
        let temp = cube[f][0];
        cube[f][0] = cube[f][6];
        cube[f][6] = cube[f][4];
        cube[f][4] = cube[f][2];
        cube[f][2] = temp;
        let temp = cube[f][1];
        cube[f][1] = cube[f][7];
        cube[f][7] = cube[f][5];
        cube[f][5] = cube[f][3];
        cube[f][3] = temp;
    } else {
        // Swap G's (0,7,6) with B's (2,1,0)
        swap_pos(cube, g, 0, b, 2);
        swap_pos(cube, g, 7, b, 1);
        swap_pos(cube, g, 6, b, 0);
        // Swap W's (4,3,2) with G's (0,7,6)
        swap_pos(cube, w, 4, g, 0);
        swap_pos(cube, w, 3, g, 7);
        swap_pos(cube, w, 2, g, 6);
        // Swap Y's (6,5,4) with W's (4,3,2)
        swap_pos(cube, y, 6, w, 4);
        swap_pos(cube, y, 5, w, 3);
        swap_pos(cube, y, 4, w, 2);
        // Rotating face CCW
        let temp = cube[f][0];
        cube[f][0] = cube[f][2];
        cube[f][2] = cube[f][4];
        cube[f][4] = cube[f][6];
        cube[f][6] = temp;
        let temp = cube[f][1];
        cube[f][1] = cube[f][3];
        cube[f][3] = cube[f][5];
        cube[f][5] = cube[f][7];
        cube[f][7] = temp;
    }
}

fn swap_pos(cube: &mut Cube, f1: usize, p1: usize, f2: usize, p2: usize) {
    if f1 == f2 {
        cube[f1].swap(p1, p2);
    } else {
        let tmp = cube[f1][p1];
        cube[f1][p1] = cube[f2][p2];
        cube[f2][p2] = tmp;
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
    for face in 0..6usize {
        println!("{} {} {}", COLOR_CODE[cube[face][0] as usize], COLOR_CODE[cube[face][1] as usize], COLOR_CODE[cube[face][2] as usize]);
        println!("{} {} {}", COLOR_CODE[cube[face][7] as usize], COLOR_CODE[face], COLOR_CODE[cube[face][3] as usize]);
        println!("{} {} {}", COLOR_CODE[cube[face][6] as usize], COLOR_CODE[cube[face][5] as usize], COLOR_CODE[cube[face][4] as usize]);
        println!("-----");
    }
}
