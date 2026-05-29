use lib2bit::twobit::TwoBit;

const FOO_2BIT: &str = "c_src/test/foo.2bit";

#[test]
fn test_construct_sequence_chr1_full_via_method() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    let chars = tb.constructSequence(0, 0, 150);
    let s: String = chars.iter().collect();
    assert_eq!(
        s,
        "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTagctagctGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
}

#[test]
fn test_construct_sequence_chr2_full_via_method() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    let chars = tb.constructSequence(1, 0, 100);
    let s: String = chars.iter().collect();
    assert_eq!(
        s,
        "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
}

#[test]
fn test_n_mask_directly() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    // chr1 has nBlocks at [0..50] and [100..150]
    // Set up an "all-A" buffer of length 150
    let mut seq: Vec<char> = vec!['A'; 150];
    tb.NMask(&mut seq, 0, 0, 150);
    let s: String = seq.iter().collect();
    let expected: String =
        std::iter::repeat('N').take(50).chain(std::iter::repeat('A').take(50)).chain(std::iter::repeat('N').take(50)).collect();
    assert_eq!(s, expected);
}

#[test]
fn test_n_mask_partial_range() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    // Range [40..60): 10 bases of N (40-49) then 10 bases of "A"
    let mut seq: Vec<char> = vec!['A'; 20];
    tb.NMask(&mut seq, 0, 40, 60);
    let s: String = seq.iter().collect();
    assert_eq!(s, "NNNNNNNNNNAAAAAAAAAA");
}

#[test]
fn test_soft_mask_directly() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    // chr1 maskBlock: start=62 size=8 -> [62..70)
    // Build a buffer of all 'A' bases for range [60..72) = 12 bases
    let mut seq: Vec<char> = vec!['A'; 12];
    tb.softMask(&mut seq, 0, 60, 72);
    let s: String = seq.iter().collect();
    // bases 0..1 (positions 60..61) = 'A', bases 2..9 (positions 62..69) lowercased -> 'a',
    // base 10..11 = 'A'
    assert_eq!(s, "AAaaaaaaaaAA");
}

#[test]
fn test_soft_mask_no_lowercase_when_not_stored() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, false);
    let mut seq: Vec<char> = vec!['A'; 12];
    tb.softMask(&mut seq, 0, 60, 72);
    let s: String = seq.iter().collect();
    // Without storeMasked, soft-mask is a no-op
    assert_eq!(s, "AAAAAAAAAAAA");
}

#[test]
fn test_get_mask_returns_invalid_dummy() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    // Public getMask returns (U32_INVALID, U32_INVALID, U32_INVALID)
    let (a, b, c) = tb.getMask(0, 0, 100);
    assert_eq!(a, u32::MAX);
    assert_eq!(b, u32::MAX);
    assert_eq!(c, u32::MAX);
}

#[test]
fn test_two_bit_bases_worker_signature() {
    // Public twoBitBasesWorker has return type () per the translation comment.
    // Just call it to confirm it does not panic.
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twoBitBasesWorker(0, 0, 100, 0);
    tb.twoBitBasesWorker(0, 24, 74, 1);
}

fn main() {}
