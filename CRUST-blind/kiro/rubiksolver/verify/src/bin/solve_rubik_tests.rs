use rubiksolver::rubik_model::*;
use rubiksolver::hash::Hash;
use rubiksolver::heap::Heap;

// solve_rubik has no public API, but we test the integration of modules it uses

#[test]
fn test_ecube_in_heap() {
    let cube = populate_initial();
    let ecube = ECube::new(cube);
    assert_eq!(ecube.entropy, 0);
    assert_eq!(ecube.hash, 0);

    let mut heap: Heap<ECube> = Heap::new(10, |a: &ECube, b: &ECube| a.entropy < b.entropy);
    heap.insert(ecube);
    assert!(!heap.is_empty());
    let min = heap.delete_min().unwrap();
    assert_eq!(min.entropy, 0);
    assert!(heap.is_empty());
}

#[test]
fn test_ecube_in_hash() {
    let cube = populate_initial();
    let ecube = ECube::new(cube);

    let eq = |a: &ECube, b: &ECube| a.entropy == b.entropy && cube_compare_equal(&a.cube, &b.cube);
    let mut hash: Hash<ECube> = Hash::new(255, |e: &ECube| e.hash);
    assert!(hash.insert(ecube.clone(), eq));
    assert!(hash.element_exists(&ecube, eq));
    assert!(hash.delete(&ecube, eq));
    assert!(!hash.element_exists(&ecube, eq));
}

#[test]
fn test_solver_first_expansion() {
    // Simulate one iteration of the solver loop
    let data: Cube = [
        [Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red, Color::Red],
        [Color::Green, Color::Yellow, Color::Blue, Color::Orange, Color::Orange, Color::Yellow, Color::Green, Color::Green],
        [Color::Blue, Color::Blue, Color::Blue, Color::Orange, Color::Orange, Color::Green, Color::Orange, Color::Blue],
        [Color::White, Color::White, Color::White, Color::Green, Color::Green, Color::White, Color::Yellow, Color::Blue],
        [Color::Orange, Color::Green, Color::Blue, Color::Yellow, Color::Yellow, Color::Yellow, Color::Yellow, Color::Blue],
        [Color::Yellow, Color::Orange, Color::White, Color::White, Color::White, Color::White, Color::Green, Color::Orange],
    ];
    let cube = populate_specific(&data);
    let ecube = ECube::new(cube);
    assert_eq!(ecube.entropy, 25);
    assert_eq!(ecube.hash, 1900466);

    // Generate all 12 children (6 faces * 2 directions)
    let mut children = Vec::new();
    for face_i in 0..6u8 {
        let face = Color::from_usize(face_i as usize);
        for dir in &[Rotation::CW, Rotation::CCW] {
            let new_cube = new_cube_rotate_face(&ecube.cube, face, *dir);
            children.push(ECube::new(new_cube));
        }
    }
    assert_eq!(children.len(), 12);
    // The YELLOW CW child should match our ground truth
    let ycw = &children[8]; // face 4 (Yellow) * 2 + 0 (CW) = index 8
    assert_eq!(ycw.hash, 8191962);
    assert_eq!(ycw.entropy, 28);
}

fn main() {}
