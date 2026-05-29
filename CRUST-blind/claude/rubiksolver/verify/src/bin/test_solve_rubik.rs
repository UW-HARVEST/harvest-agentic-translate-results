// solve_rubik.rs has no public functions - the solver lives entirely in `main`.
// We re-build the solver logic using the public APIs (Hash, Heap, ECube,
// new_cube_rotate_face, populate_initial, populate_specific) to verify the
// pieces interoperate the same way the C `main()` does.

use rubiksolver::hash::Hash;
use rubiksolver::heap::Heap;
use rubiksolver::rubik_model::{
    cube_compare_equal, new_cube_rotate_face, populate_initial, Color, Cube, ECube, Rotation,
};

fn ecube_eq(a: &ECube, b: &ECube) -> bool {
    if a.entropy != b.entropy {
        return false;
    }
    cube_compare_equal(&a.cube, &b.cube)
}

/// Returns (found, iterations) and the goal ECube if reached.
fn solve(start: Cube, max_iter: i32) -> (bool, i32) {
    let mut count_explored: i32 = 0;
    let ecube = ECube::new(start);
    let mut unexplored_hash: Hash<ECube> = Hash::new(33554432, |e: &ECube| e.hash);
    let mut unexplored: Heap<ECube> =
        Heap::new(5000, |a: &ECube, b: &ECube| a.entropy < b.entropy);
    unexplored.insert(ecube.clone());
    unexplored_hash.insert(ecube.clone(), ecube_eq);
    let mut explored_hash: Hash<ECube> = Hash::new(33554432, |e: &ECube| e.hash);

    while !unexplored.is_empty() {
        let x = unexplored.delete_min().unwrap();
        unexplored_hash.delete(&x, ecube_eq);
        if x.entropy == 0 {
            return (true, count_explored);
        }
        let x_for_loop = x.clone();
        explored_hash.insert(x, ecube_eq);
        count_explored += 1;
        if count_explored > max_iter {
            return (false, count_explored);
        }
        for face_idx in 0..6 {
            let face = Color::from_usize(face_idx);
            for dir_idx in 0..2 {
                let dir = if dir_idx == 0 {
                    Rotation::CW
                } else {
                    Rotation::CCW
                };
                let new_cube = new_cube_rotate_face(&x_for_loop.cube, face, dir);
                let y = ECube::new(new_cube);
                if explored_hash.element_exists(&y, ecube_eq) {
                    continue;
                }
                if !unexplored_hash.element_exists(&y, ecube_eq) {
                    unexplored.insert(y.clone());
                    unexplored_hash.insert(y, ecube_eq);
                }
            }
        }
    }
    (false, count_explored)
}

#[test]
fn test_solve_already_solved() {
    // C ground truth: solved cube finds goal immediately at iter 0
    let solved = populate_initial();
    let (found, iters) = solve(solved, 1000);
    assert_eq!(found, true);
    assert_eq!(iters, 0);
}

#[test]
fn test_solve_one_move_away() {
    // C ground truth: cube one CW Red rotation away finds goal in 1 explored
    let solved = populate_initial();
    let one_step = new_cube_rotate_face(&solved, Color::Red, Rotation::CW);
    let (found, iters) = solve(one_step, 1000);
    assert_eq!(found, true);
    assert_eq!(iters, 1);
}

#[test]
fn test_solve_one_move_away_other_face() {
    // After a single CCW rotation on Yellow, must be solvable in 1 step
    let solved = populate_initial();
    let one_step = new_cube_rotate_face(&solved, Color::Yellow, Rotation::CCW);
    let (found, _iters) = solve(one_step, 1000);
    assert_eq!(found, true);
}

#[test]
fn test_solve_two_moves_away() {
    // Two CW rotations away from solved
    let solved = populate_initial();
    let intermediate = new_cube_rotate_face(&solved, Color::Red, Rotation::CW);
    let two_steps = new_cube_rotate_face(&intermediate, Color::Green, Rotation::CW);
    let (found, _iters) = solve(two_steps, 5000);
    assert_eq!(found, true);
}

#[test]
fn test_ecube_clone_independence() {
    let solved = populate_initial();
    let ecube = ECube::new(solved);
    let cloned = ecube.clone();
    assert_eq!(cloned.entropy, ecube.entropy);
    assert_eq!(cloned.hash, ecube.hash);
    assert!(cube_compare_equal(&cloned.cube, &ecube.cube));
}

#[test]
fn test_ecube_compare_equal_logic() {
    // Same cube => equal
    let s1 = ECube::new(populate_initial());
    let s2 = ECube::new(populate_initial());
    assert!(ecube_eq(&s1, &s2));

    // Rotated cube => not equal
    let solved = populate_initial();
    let rot = ECube::new(new_cube_rotate_face(&solved, Color::Red, Rotation::CW));
    assert!(!ecube_eq(&s1, &rot));
}

#[test]
fn test_heap_priority_with_ecubes() {
    // Insert cubes of various entropies, smallest entropy comes out first
    let solved = populate_initial();
    let e0 = ECube::new(solved); // entropy 0

    let one_step = new_cube_rotate_face(&solved, Color::Red, Rotation::CW);
    let e1 = ECube::new(one_step); // entropy 12

    let two_steps = new_cube_rotate_face(&one_step, Color::Green, Rotation::CW);
    let e2 = ECube::new(two_steps); // entropy higher

    let mut heap: Heap<ECube> = Heap::new(10, |a: &ECube, b: &ECube| a.entropy < b.entropy);
    heap.insert(e2.clone());
    heap.insert(e1.clone());
    heap.insert(e0.clone());

    let popped = heap.delete_min().unwrap();
    assert_eq!(popped.entropy, 0);
}

#[test]
fn test_hash_with_ecubes() {
    let mut hash: Hash<ECube> = Hash::new(33554432, |e: &ECube| e.hash);

    let solved = populate_initial();
    let e1 = ECube::new(solved);
    let e1_clone = e1.clone();

    let rotated = new_cube_rotate_face(&solved, Color::Red, Rotation::CW);
    let e2 = ECube::new(rotated);

    assert!(!hash.element_exists(&e1, ecube_eq));
    hash.insert(e1, ecube_eq);
    assert!(hash.element_exists(&e1_clone, ecube_eq));
    assert!(!hash.element_exists(&e2, ecube_eq));

    hash.delete(&e1_clone, ecube_eq);
    assert!(!hash.element_exists(&e1_clone, ecube_eq));
}

fn main() {}
