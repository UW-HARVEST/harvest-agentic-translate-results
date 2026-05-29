use morton::morton::{morton, unmortoner, Morton};

#[test]
fn test_morton_zero_zero() {
    assert_eq!(morton(0, 0), 0);
}

#[test]
fn test_morton_zero_one() {
    assert_eq!(morton(0, 1), 1);
}

#[test]
fn test_morton_one_zero() {
    assert_eq!(morton(1, 0), 2);
}

#[test]
fn test_morton_one_one() {
    assert_eq!(morton(1, 1), 3);
}

#[test]
fn test_morton_three_zero() {
    // morton(0b0011, 0b0000) == 0b1010 == 10
    assert_eq!(morton(0b0011, 0b0000), 0b1010);
    assert_eq!(morton(3, 0), 10);
}

#[test]
fn test_morton_zero_three() {
    // morton(0b0000, 0b0011) == 0b0101 == 5
    assert_eq!(morton(0b0000, 0b0011), 0b0101);
    assert_eq!(morton(0, 3), 5);
}

#[test]
fn test_morton_twelve_three() {
    // morton(0b1100, 0b0011) == 0b10100101 == 165
    assert_eq!(morton(0b1100, 0b0011), 0b10100101);
    assert_eq!(morton(12, 3), 165);
}

#[test]
fn test_morton_specific_known_value() {
    // From the C test suite
    assert_eq!(morton(0x347210d1u32, 0xc6843fadu32), 0x5a346a180755e653u64);
}

#[test]
fn test_morton_all_ones_both() {
    // morton(0xFFFFFFFF, 0xFFFFFFFF) -> 0xFFFFFFFFFFFFFFFF
    assert_eq!(morton(0xFFFFFFFFu32, 0xFFFFFFFFu32), 18446744073709551615u64);
}

#[test]
fn test_morton_all_ones_hi() {
    // morton(0xFFFFFFFF, 0) -> 0xAAAAAAAAAAAAAAAA == 12297829382473034410
    assert_eq!(morton(0xFFFFFFFFu32, 0), 12297829382473034410u64);
}

#[test]
fn test_morton_all_ones_lo() {
    // morton(0, 0xFFFFFFFF) -> 0x5555555555555555 == 6148914691236517205
    assert_eq!(morton(0, 0xFFFFFFFFu32), 6148914691236517205u64);
}

#[test]
fn test_morton_alternating_5() {
    // morton(0x55555555, 0x55555555) -> 0x3333333333333333 == 3689348814741910323
    assert_eq!(morton(0x55555555u32, 0x55555555u32), 3689348814741910323u64);
}

#[test]
fn test_morton_alternating_a() {
    // morton(0xAAAAAAAA, 0xAAAAAAAA) -> 0xCCCCCCCCCCCCCCCC == 14757395258967641292
    assert_eq!(morton(0xAAAAAAAAu32, 0xAAAAAAAAu32), 14757395258967641292u64);
}

#[test]
fn test_morton_high_bit() {
    // morton(0x80000000, 0x80000000) -> 0xC000000000000000 == 13835058055282163712
    assert_eq!(morton(0x80000000u32, 0x80000000u32), 13835058055282163712u64);
}

#[test]
fn test_morton_deadbeef_12345678() {
    // morton(0xDEADBEEF, 0x12345678) -> 11793957322433019370
    assert_eq!(morton(0xDEADBEEFu32, 0x12345678u32), 11793957322433019370u64);
}

#[test]
fn test_unmortoner_zero() {
    assert_eq!(unmortoner(0), 0);
}

#[test]
fn test_unmortoner_one() {
    assert_eq!(unmortoner(1), 1);
}

#[test]
fn test_unmortoner_two() {
    // 2 has bit 1 set, but mask 0x5555... only keeps even bits, so result 0
    assert_eq!(unmortoner(2), 0);
}

#[test]
fn test_unmortoner_three() {
    // bit 0 is in even-bit mask, bit 1 is not, result 1
    assert_eq!(unmortoner(3), 1);
}

#[test]
fn test_unmortoner_alternating_5() {
    // 0x5555555555555555 - all even bits set -> 0xFFFFFFFF
    assert_eq!(unmortoner(0x5555555555555555u64), 0xFFFFFFFFu32);
}

#[test]
fn test_unmortoner_alternating_a() {
    // 0xAAAAAAAAAAAAAAAA - all odd bits set, masked to 0
    assert_eq!(unmortoner(0xAAAAAAAAAAAAAAAAu64), 0);
}

#[test]
fn test_unmortoner_all_ones() {
    // 0xFFFFFFFFFFFFFFFF -> all even bits = 0xFFFFFFFF
    assert_eq!(unmortoner(0xFFFFFFFFFFFFFFFFu64), 0xFFFFFFFFu32);
}

#[test]
fn test_unmortoner_known() {
    // unmortoner(0x5a346a180755e653) = 3330555821 (this is the lo from C test)
    assert_eq!(unmortoner(0x5a346a180755e653u64), 3330555821u32);
}

#[test]
fn test_unmortoner_12345678() {
    assert_eq!(unmortoner(0x12345678u64), 18156u32);
}

#[test]
fn test_unmortoner_deadbeef() {
    assert_eq!(unmortoner(0xDEADBEEFu64), 58219u32);
}

#[test]
fn test_morton_roundtrip_simple() {
    // morton then deinterleave via unmortoner
    let hi = 0x12345678u32;
    let lo = 0xDEADBEEFu32;
    let z = morton(hi, lo);
    // lo recovered by unmortoner(z)
    assert_eq!(unmortoner(z), lo);
    // hi recovered by unmortoner(z >> 1)
    assert_eq!(unmortoner(z >> 1), hi);
}

#[test]
fn test_morton_roundtrip_random_values() {
    // Test using LCG similar to C test
    let mut lcg: u64 = 1;
    for _ in 0..200 {
        lcg = lcg.wrapping_mul(6364136223846793005);
        lcg = lcg.wrapping_add(1442695040888963407);
        let x = (lcg >> 32) as u32;
        lcg = lcg.wrapping_mul(6364136223846793005);
        lcg = lcg.wrapping_add(1442695040888963407);
        let y = (lcg >> 32) as u32;
        let z = morton(x, y);
        // x is hi, y is lo (matching C: morton(x,y) where x is first arg = hi)
        assert_eq!(unmortoner(z), y);
        assert_eq!(unmortoner(z >> 1), x);
    }
}

#[test]
fn test_morton_roundtrip_all_powers_of_two() {
    // morton(2^i, 2^j) and verify roundtrip
    for i in 0..32 {
        for j in 0..32 {
            let hi = 1u32 << i;
            let lo = 1u32 << j;
            let z = morton(hi, lo);
            assert_eq!(unmortoner(z), lo, "lo failed for hi={}, lo={}", hi, lo);
            assert_eq!(unmortoner(z >> 1), hi, "hi failed for hi={}, lo={}", hi, lo);
        }
    }
}

#[test]
fn test_morton_struct_fields() {
    // Verify the struct can hold both fields
    let m = Morton {
        hi: 12345u32,
        lo: 67890u32,
    };
    assert_eq!(m.hi, 12345);
    assert_eq!(m.lo, 67890);
}

#[test]
fn test_morton_2_3() {
    // morton(2, 3) -> hi=2 (10), lo=3 (11)
    // Interleaved: hi-bits in odd positions, lo-bits in even positions
    // hi=10: only bit 1 -> goes to position 3 -> 0b1000 = 8
    // lo=11: bits 0 and 1 -> positions 0,2 -> 0b0101 = 5
    // Result: 13
    assert_eq!(morton(2, 3), 13);
}

#[test]
fn test_morton_max_lo_only() {
    // Only lo set with various values
    assert_eq!(morton(0, 0xFFFF), 0x55555555u64);
}

#[test]
fn test_morton_max_hi_only() {
    assert_eq!(morton(0xFFFF, 0), 0xAAAAAAAAu64);
}

fn main() {}
