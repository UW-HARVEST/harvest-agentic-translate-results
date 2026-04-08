use morton::morton::{morton, unmortoner, unmorton};

// --- morton tests ---

#[test]
fn test_morton_zeros() {
    assert_eq!(morton(0, 0), 0);
}

#[test]
fn test_morton_unit_lo() {
    assert_eq!(morton(0, 1), 1);
}

#[test]
fn test_morton_unit_hi() {
    assert_eq!(morton(1, 0), 2);
}

#[test]
fn test_morton_both_ones() {
    assert_eq!(morton(1, 1), 3);
}

#[test]
fn test_morton_hi_only_bits() {
    // morton(3,0) = 0b1010 = 10
    assert_eq!(morton(3, 0), 10);
}

#[test]
fn test_morton_lo_only_bits() {
    // morton(0,3) = 0b0101 = 5
    assert_eq!(morton(0, 3), 5);
}

#[test]
fn test_morton_mixed() {
    // morton(12,3) = 0b10100101 = 165
    assert_eq!(morton(12, 3), 165);
}

#[test]
fn test_morton_large() {
    assert_eq!(morton(0x347210d1, 0xc6843fad), 0x5a346a180755e653);
}

#[test]
fn test_morton_all_ones() {
    assert_eq!(morton(0xFFFFFFFF, 0xFFFFFFFF), 0xFFFFFFFFFFFFFFFF);
}

#[test]
fn test_morton_hi_max() {
    assert_eq!(morton(0xFFFFFFFF, 0), 0xAAAAAAAAAAAAAAAA);
}

#[test]
fn test_morton_lo_max() {
    assert_eq!(morton(0, 0xFFFFFFFF), 0x5555555555555555);
}

// --- unmortoner tests ---

#[test]
fn test_unmortoner_zero() {
    assert_eq!(unmortoner(0), 0);
}

#[test]
fn test_unmortoner_one() {
    // bit 0 is set → extracts to 1
    assert_eq!(unmortoner(1), 1);
}

#[test]
fn test_unmortoner_all_even_bits() {
    assert_eq!(unmortoner(0x5555555555555555), 0xFFFFFFFF);
}

#[test]
fn test_unmortoner_all_odd_bits() {
    // odd bits masked out by 0x5555... → 0
    assert_eq!(unmortoner(0xAAAAAAAAAAAAAAAA), 0);
}

#[test]
fn test_unmortoner_all_ones() {
    // same as all even bits since odd bits are masked
    assert_eq!(unmortoner(0xFFFFFFFFFFFFFFFF), 0xFFFFFFFF);
}

// --- unmorton tests ---
// C behavior: unmorton(z).hi = unmortoner(z), .lo = unmortoner(z >> 1)
// So unmorton is the inverse of morton, but hi/lo are swapped:
//   morton(hi, lo) → z  ⟹  unmorton(z).lo == hi, unmorton(z).hi == lo

#[test]
fn test_unmorton_zero() {
    let m = unmorton(0);
    assert_eq!(m.lo, 0);
    assert_eq!(m.hi, 0);
}

#[test]
fn test_unmorton_one() {
    // z=1: hi = unmortoner(1) = 1, lo = unmortoner(0) = 0
    let m = unmorton(1);
    assert_eq!(m.hi, 1);
    assert_eq!(m.lo, 0);
}

#[test]
fn test_unmorton_two() {
    // z=2: hi = unmortoner(2) = 0, lo = unmortoner(1) = 1
    let m = unmorton(2);
    assert_eq!(m.hi, 0);
    assert_eq!(m.lo, 1);
}

#[test]
fn test_unmorton_three() {
    let m = unmorton(3);
    assert_eq!(m.hi, 1);
    assert_eq!(m.lo, 1);
}

#[test]
fn test_unmorton_mixed() {
    // unmorton(165) where 165 = morton(12, 3)
    // C: lo=12, hi=3
    let m = unmorton(165);
    assert_eq!(m.lo, 12);
    assert_eq!(m.hi, 3);
}

#[test]
fn test_unmorton_large() {
    let m = unmorton(0x5a346a180755e653);
    assert_eq!(m.lo, 0x347210d1);
    assert_eq!(m.hi, 0xc6843fad);
}

#[test]
fn test_unmorton_all_ones() {
    let m = unmorton(0xFFFFFFFFFFFFFFFF);
    assert_eq!(m.lo, 0xFFFFFFFF);
    assert_eq!(m.hi, 0xFFFFFFFF);
}

#[test]
fn test_unmorton_even_bits_only() {
    // 0x5555... = all even bits set → hi=0xFFFFFFFF, lo=0
    let m = unmorton(0x5555555555555555);
    assert_eq!(m.hi, 0xFFFFFFFF);
    assert_eq!(m.lo, 0);
}

#[test]
fn test_unmorton_odd_bits_only() {
    // 0xAAAA... = all odd bits set → hi=0, lo=0xFFFFFFFF
    let m = unmorton(0xAAAAAAAAAAAAAAAA);
    assert_eq!(m.hi, 0);
    assert_eq!(m.lo, 0xFFFFFFFF);
}

// --- roundtrip tests ---

#[test]
fn test_roundtrip_basic() {
    for hi in [0u32, 1, 0xFF, 0xFFFF, 0xFFFFFFFF] {
        for lo in [0u32, 1, 0xFF, 0xFFFF, 0xFFFFFFFF] {
            let z = morton(hi, lo);
            let m = unmorton(z);
            assert_eq!(m.lo, hi, "roundtrip failed for hi={hi}, lo={lo}");
            assert_eq!(m.hi, lo, "roundtrip failed for hi={hi}, lo={lo}");
        }
    }
}

fn main() {}
