use crate::rubik_model::{cube_compare_equal, ECube};

/// Compares two ECubes for equality.
/// They are equal if their entropies match and their cubes compare equal.
pub fn ecube_compare_equal(a: &ECube, b: &ECube) -> bool {
    if a.entropy != b.entropy {
        return false;
    }
    cube_compare_equal(&a.cube, &b.cube)
}

/// Returns true if `ec1` is "better" (lower entropy) than `ec2`.
pub fn is_better(ec1: &ECube, ec2: &ECube) -> bool {
    ec1.entropy < ec2.entropy
}

/// Hash function for ecubes — returns the precomputed hash.
pub fn hash_function(ecube: &ECube) -> u32 {
    ecube.hash
}

fn main() {}
