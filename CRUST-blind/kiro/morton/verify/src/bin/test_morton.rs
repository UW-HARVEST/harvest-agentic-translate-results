use morton::morton::{morton, unmortoner, unmorton};

#[test]
fn test_morton_basic() {
    assert_eq!(morton(0, 0), 0);
    assert_eq!(morton(0, 1), 1);
    assert_eq!(morton(1, 0), 2);
    assert_eq!(morton(1, 1), 3);
}

#[test]
fn test_morton_bit_patterns() {
    assert_eq!(morton(3, 0), 10);
    assert_eq!(morton(0, 3), 5);
    assert_eq!(morton(12, 3), 165);
}

#[test]
fn test_morton_large() {
    assert_eq!(morton(0x347210d1, 0xc6843fad), 6499936813637297747);
}

#[test]
fn test_morton_edge_cases() {
    assert_eq!(morton(0xFFFFFFFF, 0xFFFFFFFF), 18446744073709551615);
    assert_eq!(morton(0xFFFFFFFF, 0), 12297829382473034410);
    assert_eq!(morton(0, 0xFFFFFFFF), 6148914691236517205);
}

#[test]
fn test_unmortoner() {
    // unmortoner extracts even-position bits from a u64
    assert_eq!(unmortoner(0), 0);
    assert_eq!(unmortoner(0x5555555555555555), 0xFFFFFFFF);
    assert_eq!(unmortoner(0xAAAAAAAAAAAAAAAA), 0);
    assert_eq!(unmortoner(0xFFFFFFFFFFFFFFFF), 0xFFFFFFFF);
    assert_eq!(unmortoner(1), 1);
    assert_eq!(unmortoner(3), 1); // bits 0b11 -> even bit is 1
}

#[test]
fn test_unmorton_basic() {
    let m = unmorton(0);
    assert_eq!(m.lo, 0);
    assert_eq!(m.hi, 0);

    let m = unmorton(1);
    assert_eq!(m.lo, 0);
    assert_eq!(m.hi, 1);

    let m = unmorton(2);
    assert_eq!(m.lo, 1);
    assert_eq!(m.hi, 0);

    let m = unmorton(3);
    assert_eq!(m.lo, 1);
    assert_eq!(m.hi, 1);
}

#[test]
fn test_unmorton_larger() {
    let m = unmorton(165);
    assert_eq!(m.lo, 12);
    assert_eq!(m.hi, 3);

    let m = unmorton(0x5a346a180755e653);
    assert_eq!(m.lo, 879890641);
    assert_eq!(m.hi, 3330555821);
}

#[test]
fn test_unmorton_edge_cases() {
    let m = unmorton(0xFFFFFFFFFFFFFFFF);
    assert_eq!(m.lo, 4294967295);
    assert_eq!(m.hi, 4294967295);

    let m = unmorton(0xAAAAAAAAAAAAAAAA);
    assert_eq!(m.lo, 4294967295);
    assert_eq!(m.hi, 0);

    let m = unmorton(0x5555555555555555);
    assert_eq!(m.lo, 0);
    assert_eq!(m.hi, 4294967295);
}

#[test]
fn test_roundtrip() {
    let z = morton(12345, 67890);
    assert_eq!(z, 4467003270);
    let m = unmorton(z);
    assert_eq!(m.lo, 12345);
    assert_eq!(m.hi, 67890);

    let z = morton(0, 0xDEADBEEF);
    assert_eq!(z, 5860384130962052181);
    let m = unmorton(z);
    assert_eq!(m.lo, 0);
    assert_eq!(m.hi, 3735928559);
}

fn main() {}
