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
    for (face_ix, face) in cube.iter_mut().enumerate() {
        let color = Color::from_usize(face_ix);
        for slot in face.iter_mut() {
            *slot = color;
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
/// For each face and for positions in pairs, if the face’s color does not match the expected one, increment the hash.
pub fn cube_hash(cube: &Cube) -> u32 {
    let mut hash = 0;
    for face in 0..6 {
        let face_color = Color::from_usize(face);
        for pos in 0..4 {
            if cube[face][2 * pos] != face_color || cube[face][2 * pos + 1] != face_color {
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
    let l_around = cycle_l(around);
    if l_around == rear(rotating_face) || l_around == rotating_face {
        cycle_l(l_around)
    } else {
        l_around
    }
}
/// Returns the “adjacent right” face.
pub fn adjacent_right(rotating_face: Color, around: Color) -> Color {
    let r_around = cycle_r(around);
    if r_around == rear(rotating_face) || r_around == rotating_face {
        cycle_r(r_around)
    } else {
        r_around
    }
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

fn swap_stickers(cube: &mut Cube, a: (usize, usize), b: (usize, usize)) {
    let tmp = cube[a.0][a.1];
    cube[a.0][a.1] = cube[b.0][b.1];
    cube[b.0][b.1] = tmp;
}

/// Rotates a face of the cube in place.
/// The swapping operations mimic the series of SWAP_COLOR macros.
pub fn rotate_face(cube: &mut Cube, face: Color, direction: Rotation) {
    let face_ix = face as usize;
    let g = TOP[face_ix] as usize;
    let w = adjacent_cw(face, TOP[face_ix]) as usize;
    let y = adjacent_cw(face, Color::from_usize(w)) as usize;
    let b = adjacent_cw(face, Color::from_usize(y)) as usize;

    match direction {
        Rotation::CW => {
            swap_stickers(cube, (g, 0), (w, 4));
            swap_stickers(cube, (g, 7), (w, 3));
            swap_stickers(cube, (g, 6), (w, 2));

            swap_stickers(cube, (b, 2), (g, 0));
            swap_stickers(cube, (b, 1), (g, 7));
            swap_stickers(cube, (b, 0), (g, 6));

            swap_stickers(cube, (y, 6), (b, 2));
            swap_stickers(cube, (y, 5), (b, 1));
            swap_stickers(cube, (y, 4), (b, 0));

            let temp = cube[face_ix][0];
            cube[face_ix][0] = cube[face_ix][6];
            cube[face_ix][6] = cube[face_ix][4];
            cube[face_ix][4] = cube[face_ix][2];
            cube[face_ix][2] = temp;

            let temp = cube[face_ix][1];
            cube[face_ix][1] = cube[face_ix][7];
            cube[face_ix][7] = cube[face_ix][5];
            cube[face_ix][5] = cube[face_ix][3];
            cube[face_ix][3] = temp;
        }
        Rotation::CCW => {
            swap_stickers(cube, (g, 0), (b, 2));
            swap_stickers(cube, (g, 7), (b, 1));
            swap_stickers(cube, (g, 6), (b, 0));

            swap_stickers(cube, (w, 4), (g, 0));
            swap_stickers(cube, (w, 3), (g, 7));
            swap_stickers(cube, (w, 2), (g, 6));

            swap_stickers(cube, (y, 6), (w, 4));
            swap_stickers(cube, (y, 5), (w, 3));
            swap_stickers(cube, (y, 4), (w, 2));

            let temp = cube[face_ix][0];
            cube[face_ix][0] = cube[face_ix][2];
            cube[face_ix][2] = cube[face_ix][4];
            cube[face_ix][4] = cube[face_ix][6];
            cube[face_ix][6] = temp;

            let temp = cube[face_ix][1];
            cube[face_ix][1] = cube[face_ix][3];
            cube[face_ix][3] = cube[face_ix][5];
            cube[face_ix][5] = cube[face_ix][7];
            cube[face_ix][7] = temp;
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
    for face in 0..6 {
        println!(
            "{} {} {}\n{} {} {}\n{} {} {}\n-----",
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
    }
}
