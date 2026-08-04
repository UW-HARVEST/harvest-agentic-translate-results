use crate::hash::Hash;
use crate::heap::Heap;
use crate::rubik_model::{
    cube_compare_equal, new_cube_rotate_face, populate_specific, Color, ECube, Rotation,
};

/// Compares two ECubes for equality.
/// They are equal if their entropies match and their cubes compare equal.
fn ecube_compare_equal(a: &ECube, b: &ECube) -> bool {
    if a.entropy != b.entropy {
        return false;
    }
    cube_compare_equal(&a.cube, &b.cube)
}

fn main() {
    let cube_data: [[Color; 8]; 6] = [
        [
            Color::Red,
            Color::Red,
            Color::Red,
            Color::Red,
            Color::Red,
            Color::Red,
            Color::Red,
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

    let mut count_explored: i32 = 0;
    let mut count_unexplored: i32 = 1;

    let cube = populate_specific(&cube_data);
    let ecube = ECube::new(cube);

    let mut unexplored_hash: Hash<ECube> = Hash::new(33554432, |e: &ECube| e.hash);
    let mut unexplored: Heap<ECube> =
        Heap::new(5000, |e1: &ECube, e2: &ECube| e1.entropy < e2.entropy);

    unexplored.insert(ecube.clone());
    unexplored_hash.insert(ecube, ecube_compare_equal);
    let mut explored_hash: Hash<ECube> = Hash::new(33554432, |e: &ECube| e.hash);

    while !unexplored.is_empty() {
        println!("Unexplored nodes = {:6}", count_unexplored);
        let x = unexplored.delete_min().unwrap();
        unexplored_hash.delete(&x, ecube_compare_equal);
        count_unexplored -= 1;
        println!("Entropy of x: {}", x.entropy);

        if x.entropy == 0 {
            println!("Found goal.");
            std::process::exit(0);
        }

        // Insert x into explored, but we still need to iterate over rotations using x.cube.
        let x_cube = x.cube;
        explored_hash.insert(x, ecube_compare_equal);
        count_explored += 1;
        println!("Explored nodes = {:6}", count_explored);

        for face_idx in 0..=5usize {
            let face = Color::from_usize(face_idx);
            for dir in &[Rotation::CW, Rotation::CCW] {
                let new_cube = new_cube_rotate_face(&x_cube, face, *dir);
                let y = ECube::new(new_cube);
                if explored_hash.element_exists(&y, ecube_compare_equal) {
                    continue;
                }
                if !unexplored_hash.element_exists(&y, ecube_compare_equal) {
                    unexplored.insert(y.clone());
                    unexplored_hash.insert(y, ecube_compare_equal);
                    count_unexplored += 1;
                }
            }
        }
    }
}
