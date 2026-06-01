use crate::rubik_model::{cube_compare_equal, ECube};
/// Compares two ECubes for equality.
/// They are equal if their entropies match and their cubes compare equal.
fn ecube_compare_equal(a: &ECube, b: &ECube) -> bool {
    if a.entropy != b.entropy {
        return false;
    }
    cube_compare_equal(&a.cube, &b.cube)
}
fn main() {
    // Solve_rubik main is not used in tests; provided here as a simple stub.
    let _ = ecube_compare_equal;
}
