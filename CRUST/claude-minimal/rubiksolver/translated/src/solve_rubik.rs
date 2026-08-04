use crate::hash::Hash;
use crate::heap::Heap;
use crate::rubik_model::{
    cube_compare_equal, new_cube_rotate_face, populate_specific, Color, Cube, ECube, Rotation,
};
use std::rc::Rc;

/// Compares two ECubes for equality.
/// They are equal if their entropies match and their cubes compare equal.
fn ecube_compare_equal(a: &Rc<ECube>, b: &Rc<ECube>) -> bool {
    if a.entropy != b.entropy {
        return false;
    }
    cube_compare_equal(&a.cube, &b.cube)
}

/// Returns true if `ec1` is "better" (has lower entropy) than `ec2`.
fn is_better(ec1: &Rc<ECube>, ec2: &Rc<ECube>) -> bool {
    ec1.entropy < ec2.entropy
}

/// Returns the hash of an ECube.
fn hash_function(ec: &Rc<ECube>) -> u32 {
    ec.hash
}

fn main() {
    let cube_data: Cube = [
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

    let mut count_explored: i32 = 0;
    let mut count_unexplored: i32 = 1;
    let cube = populate_specific(&cube_data);
    let ecube = Rc::new(ECube::new(cube));

    let mut unexplored_hash: Hash<Rc<ECube>> = Hash::new(33554432, hash_function);
    let mut unexplored: Heap<Rc<ECube>> = Heap::new(5000, is_better);

    unexplored.insert(Rc::clone(&ecube));
    unexplored_hash.insert(Rc::clone(&ecube), ecube_compare_equal);
    let mut explored_hash: Hash<Rc<ECube>> = Hash::new(33554432, hash_function);

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
        explored_hash.insert(Rc::clone(&x), ecube_compare_equal);
        count_explored += 1;
        println!("Explored nodes = {:6}", count_explored);

        for face_idx in 0..=5usize {
            let face = Color::from_usize(face_idx);
            for dir in [Rotation::CW, Rotation::CCW].iter() {
                let new_cube = new_cube_rotate_face(&x.cube, face, *dir);
                let y = Rc::new(ECube::new(new_cube));
                if explored_hash.element_exists(&y, ecube_compare_equal) {
                    continue;
                }
                if !unexplored_hash.element_exists(&y, ecube_compare_equal) {
                    unexplored.insert(Rc::clone(&y));
                    unexplored_hash.insert(Rc::clone(&y), ecube_compare_equal);
                    count_unexplored += 1;
                }
            }
        }
    }
}
