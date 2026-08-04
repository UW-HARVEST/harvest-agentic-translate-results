use crate::rubik_model::ECube;
/// Compares two ECubes for equality.
/// They are equal if their entropies match and their cubes compare equal.
fn ecube_compare_equal(a: &ECube, b: &ECube) -> bool {
    a.entropy == b.entropy && crate::rubik_model::cube_compare_equal(&a.cube, &b.cube)
}
fn main() {
    let _ = ecube_compare_equal;
}
