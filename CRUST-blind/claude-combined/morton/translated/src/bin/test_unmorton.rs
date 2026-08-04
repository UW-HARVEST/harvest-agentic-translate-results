#![allow(unused_imports)]
use morton::morton::morton as morton_fn;
use morton::morton::unmorton;
use morton::morton::unmortoner;

#[test]
fn test_unmorton_zero() {
    let m = unmorton(0);
    assert_eq!(m.hi, 0);
    assert_eq!(m.lo, 0);
}

#[test]
fn test_unmorton_one() {
    // unmorton(0x1) -> hi=1, lo=0 (per C ground truth)
    let m = unmorton(0x1);
    assert_eq!(m.hi, 0x1);
    assert_eq!(m.lo, 0x0);
}

#[test]
fn test_unmorton_two() {
    let m = unmorton(0x2);
    assert_eq!(m.hi, 0x0);
    assert_eq!(m.lo, 0x1);
}

#[test]
fn test_unmorton_three() {
    let m = unmorton(0x3);
    assert_eq!(m.hi, 0x1);
    assert_eq!(m.lo, 0x1);
}

#[test]
fn test_unmorton_ff() {
    let m = unmorton(0xFF);
    assert_eq!(m.hi, 0xF);
    assert_eq!(m.lo, 0xF);
}

#[test]
fn test_unmorton_5555() {
    // 0x5555... has the even bits set (bit 0, 2, 4, ...).
    // C: hi = unmortoner(z) with mask 0x5555..., lo = unmortoner(z>>1).
    // unmorton(0x5555...) -> hi=0xFFFFFFFF, lo=0
    let m = unmorton(0x5555555555555555u64);
    assert_eq!(m.hi, 0xFFFFFFFFu32);
    assert_eq!(m.lo, 0u32);
}

#[test]
fn test_unmorton_aaaa() {
    let m = unmorton(0xAAAAAAAAAAAAAAAAu64);
    assert_eq!(m.hi, 0u32);
    assert_eq!(m.lo, 0xFFFFFFFFu32);
}

#[test]
fn test_unmorton_all_ones() {
    let m = unmorton(0xFFFFFFFFFFFFFFFFu64);
    assert_eq!(m.hi, 0xFFFFFFFFu32);
    assert_eq!(m.lo, 0xFFFFFFFFu32);
}

#[test]
fn test_unmorton_known() {
    // unmorton(0x5a346a180755e653) -> hi=0xc6843fad, lo=0x347210d1
    let m = unmorton(0x5a346a180755e653u64);
    assert_eq!(m.hi, 0xc6843fadu32);
    assert_eq!(m.lo, 0x347210d1u32);
}

#[test]
fn test_unmorton_misc() {
    let m = unmorton(0x123456789ABCDEF0u64);
    assert_eq!(m.hi, 0x46ec46ecu32);
    assert_eq!(m.lo, 0x1416bebcu32);
}

#[test]
fn test_morton_unmorton_roundtrip() {
    // Match C test semantics:
    //   z = morton(x, y);
    //   m = unmorton(z);
    //   assert(m.lo == x);
    //   assert(m.hi == y);
    // The morton function signature is morton(hi, lo); the C unmorton swaps them
    // when constructing the output struct, so m.lo gets the first arg and m.hi
    // gets the second.
    let pairs: [(u32, u32); 6] = [
        (0, 0),
        (1, 1),
        (0xFFFFFFFFu32, 0xFFFFFFFFu32),
        (0x347210d1u32, 0xc6843fadu32),
        (0xAAAAAAAAu32, 0x55555555u32),
        (12345, 67890),
    ];
    for (a, b) in pairs.iter() {
        let z = morton_fn(*a, *b);
        let m = unmorton(z);
        assert_eq!(m.lo, *a);
        assert_eq!(m.hi, *b);
    }
}

#[test]
fn test_unmortoner_zero() {
    assert_eq!(unmortoner(0), 0);
}

#[test]
fn test_unmortoner_one() {
    // unmortoner(0x1) = 0x1 (only bit 0 set, even -> stays)
    assert_eq!(unmortoner(0x1), 0x1);
}

#[test]
fn test_unmortoner_two() {
    // unmortoner(0x2) = 0x0 (bit 1 set, odd, masked away)
    assert_eq!(unmortoner(0x2), 0x0);
}

#[test]
fn test_unmortoner_three() {
    assert_eq!(unmortoner(0x3), 0x1);
}

#[test]
fn test_unmortoner_ff() {
    assert_eq!(unmortoner(0xFF), 0xF);
}

#[test]
fn test_unmortoner_aa() {
    // 0xAA has only odd bits, masked off -> 0
    assert_eq!(unmortoner(0xAA), 0x0);
}

#[test]
fn test_unmortoner_55() {
    // 0x55 has only even bits -> compresses to 0xF
    assert_eq!(unmortoner(0x55), 0xF);
}

#[test]
fn test_unmortoner_all_even_bits() {
    assert_eq!(unmortoner(0x5555555555555555u64), 0xFFFFFFFFu32);
}

#[test]
fn test_unmortoner_all_odd_bits() {
    assert_eq!(unmortoner(0xAAAAAAAAAAAAAAAAu64), 0u32);
}

#[test]
fn test_unmortoner_all_ones() {
    assert_eq!(unmortoner(0xFFFFFFFFFFFFFFFFu64), 0xFFFFFFFFu32);
}

#[test]
fn test_unmortoner_misc() {
    assert_eq!(unmortoner(0x123456789ABCDEF0u64), 0x46ec46ecu32);
}

#[test]
fn test_unmortoner_known() {
    assert_eq!(unmortoner(0x5a346a180755e653u64), 0xc6843fadu32);
}

fn main() {}
