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
    let mut cube: Cube = [[Color::Red; 8]; 6];
    for face in 0..6 {
        let c = Color::from_usize(face);
        for pos in 0..8 {
            cube[face][pos] = c;
        }
    }
    cube
}
/// Creates a cube from provided data.
pub fn populate_specific(data: &Cube) -> Cube {
    *data
}
/// Compares two cubes for equality.
pub fn cube_compare_equal(c1: &Cube, c2: &Cube) -> bool {
    for face in 0..6 {
        for pos in 0..8 {
            if c1[face][pos] != c2[face][pos] {
                return false;
            }
        }
    }
    true
}
/// Computes a hash value for a cube.
/// For each face and for positions in pairs, if the face's color does not match the expected one, increment the hash.
pub fn cube_hash(cube: &Cube) -> u32 {
    let mut hash: u32 = 0;
    for face in 0..6 {
        let face_color = Color::from_usize(face);
        for pos in 0..4 {
            if cube[face][2 * pos] != face_color || cube[face][2 * pos + 1] != face_color {
                hash = hash.wrapping_add(1);
            }
            hash = hash.wrapping_shl(1);
        }
    }
    hash
}
/// Counts the number of "misplaced" positions.
pub fn find_entropy(cube: &Cube) -> i32 {
    let mut count = 0;
    for face in 0..6 {
        let face_color = Color::from_usize(face);
        for pos in 0..8 {
            if cube[face][pos] != face_color {
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
    let l_around = cycle_l(around);
    if l_around == rear(rotating_face) || l_around == rotating_face {
        return cycle_l(l_around);
    }
    l_around
}
/// Returns the "adjacent right" face.
pub fn adjacent_right(rotating_face: Color, around: Color) -> Color {
    let r_around = cycle_r(around);
    if r_around == rear(rotating_face) || r_around == rotating_face {
        return cycle_r(r_around);
    }
    r_around
}
/// Computes the adjacent face in the clockwise direction.
pub fn adjacent_cw(rotating_face: Color, around: Color) -> Color {
    assert!(around != rear(rotating_face));
    if (rotating_face as usize) % 2 == 1 {
        adjacent_right(rotating_face, around)
    } else {
        adjacent_left(rotating_face, around)
    }
}
/// Computes the adjacent face in the counter‑clockwise direction.
pub fn adjacent_ccw(rotating_face: Color, around: Color) -> Color {
    assert!(around != rear(rotating_face));
    if (rotating_face as usize) % 2 == 1 {
        adjacent_left(rotating_face, around)
    } else {
        adjacent_right(rotating_face, around)
    }
}
/// Rotates a face of the cube in place.
/// The swapping operations mimic the series of SWAP_COLOR macros.
pub fn rotate_face(cube: &mut Cube, face: Color, direction: Rotation) {
    let g = TOP[face as usize];
    let w = adjacent_cw(face, g);
    let y = adjacent_cw(face, w);
    let b = adjacent_cw(face, y);
    let f = face as usize;
    let gi = g as usize;
    let wi = w as usize;
    let yi = y as usize;
    let bi = b as usize;
    if direction == Rotation::CW {
        // Swap G's (0,7,6) with W's (4,3,2)
        let t = cube[gi][0]; cube[gi][0] = cube[wi][4]; cube[wi][4] = t;
        let t = cube[gi][7]; cube[gi][7] = cube[wi][3]; cube[wi][3] = t;
        let t = cube[gi][6]; cube[gi][6] = cube[wi][2]; cube[wi][2] = t;

        // Swap B's (2,1,0) with G's (0,7,6)
        let t = cube[bi][2]; cube[bi][2] = cube[gi][0]; cube[gi][0] = t;
        let t = cube[bi][1]; cube[bi][1] = cube[gi][7]; cube[gi][7] = t;
        let t = cube[bi][0]; cube[bi][0] = cube[gi][6]; cube[gi][6] = t;

        // Swap Y's (6,5,4) with B's (2,1,0)
        let t = cube[yi][6]; cube[yi][6] = cube[bi][2]; cube[bi][2] = t;
        let t = cube[yi][5]; cube[yi][5] = cube[bi][1]; cube[bi][1] = t;
        let t = cube[yi][4]; cube[yi][4] = cube[bi][0]; cube[bi][0] = t;

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
        let t = cube[gi][0]; cube[gi][0] = cube[bi][2]; cube[bi][2] = t;
        let t = cube[gi][7]; cube[gi][7] = cube[bi][1]; cube[bi][1] = t;
        let t = cube[gi][6]; cube[gi][6] = cube[bi][0]; cube[bi][0] = t;

        // Swap W's (4,3,2) with G's (0,7,6)
        let t = cube[wi][4]; cube[wi][4] = cube[gi][0]; cube[gi][0] = t;
        let t = cube[wi][3]; cube[wi][3] = cube[gi][7]; cube[gi][7] = t;
        let t = cube[wi][2]; cube[wi][2] = cube[gi][6]; cube[gi][6] = t;

        // Swap Y's (6,5,4) with W's (4,3,2)
        let t = cube[yi][6]; cube[yi][6] = cube[wi][4]; cube[wi][4] = t;
        let t = cube[yi][5]; cube[yi][5] = cube[wi][3]; cube[wi][3] = t;
        let t = cube[yi][4]; cube[yi][4] = cube[wi][2]; cube[wi][2] = t;

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
/// Returns a new cube that is the result of rotating a face.
pub fn new_cube_rotate_face(cube: &Cube, face: Color, direction: Rotation) -> Cube {
    let mut new_cube = *cube;
    rotate_face(&mut new_cube, face, direction);
    new_cube
}
/// Prints the cube in a formatted way.
pub fn print_cube(cube: &Cube) {
    for face in 0..6 {
        println!(
            "{} {} {}\n{} {} {}\n{} {} {}",
            COLOR_CODE[cube[face][0] as usize],
            COLOR_CODE[cube[face][1] as usize],
            COLOR_CODE[cube[face][2] as usize],
            COLOR_CODE[cube[face][7] as usize],
            COLOR_CODE[face],
            COLOR_CODE[cube[face][3] as usize],
            COLOR_CODE[cube[face][6] as usize],
            COLOR_CODE[cube[face][5] as usize],
            COLOR_CODE[cube[face][4] as usize],
        );
        println!("-----");
    }
}
