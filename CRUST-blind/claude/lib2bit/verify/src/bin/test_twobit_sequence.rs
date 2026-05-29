use lib2bit::twobit::TwoBit;

const FOO_2BIT: &str = "c_src/test/foo.2bit";

#[test]
fn test_sequence_chr1_full() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 0, 0);
    assert_eq!(
        seq,
        "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTagctagctGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
    assert_eq!(seq.len(), 150);
}

#[test]
fn test_sequence_chr2_full() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr2", 0, 0);
    assert_eq!(
        seq,
        "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
    assert_eq!(seq.len(), 100);
}

#[test]
fn test_sequence_chr1_24_74() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 24, 74);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTagctagctGATC");
    assert_eq!(seq.len(), 50);
}

#[test]
fn test_sequence_chr1_50_100() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 50, 100);
    assert_eq!(seq, "ACGTACGTACGTagctagctGATCGATCGTAGCTAGCTAGCTAGCTGATC");
}

#[test]
fn test_sequence_chr1_50_70() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 50, 70);
    assert_eq!(seq, "ACGTACGTACGTagctagct");
}

#[test]
fn test_sequence_chr1_0_50_all_n() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 0, 50);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN");
}

#[test]
fn test_sequence_chr1_100_150_all_n() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 100, 150);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN");
}

#[test]
fn test_sequence_chr1_60_70_lowercase() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 60, 70);
    assert_eq!(seq, "GTagctagct");
}

#[test]
fn test_sequence_chr1_1_5() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 1, 5);
    assert_eq!(seq, "NNNN");
}

#[test]
fn test_sequence_chr1_75_100() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 75, 100);
    assert_eq!(seq, "ATCGTAGCTAGCTAGCTAGCTGATC");
}

#[test]
fn test_sequence_chr1_60_80() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 60, 80);
    assert_eq!(seq, "GTagctagctGATCGATCGT");
}

#[test]
fn test_sequence_chr1_49_51_n_to_a() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 49, 51);
    assert_eq!(seq, "NA");
}

#[test]
fn test_sequence_chr1_54_62() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 54, 62);
    assert_eq!(seq, "ACGTACGT");
}

#[test]
fn test_sequence_chr2_50_100_all_n() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr2", 50, 100);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN");
}

#[test]
fn test_sequence_chr2_0_50() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr2", 0, 50);
    assert_eq!(seq, "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATC");
}

#[test]
fn test_sequence_invalid_start_ge_end() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 100, 50);
    assert_eq!(seq, "");
}

#[test]
fn test_sequence_end_too_large() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chr1", 0, 200);
    assert_eq!(seq, "");
}

#[test]
fn test_sequence_missing_chrom() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let seq = tb.twobit_sequence("chrX", 0, 10);
    assert_eq!(seq, "");
}

#[test]
fn test_sequence_unmasked_no_lowercase() {
    let tb = TwoBit::twobit_open(FOO_2BIT, false);
    let seq = tb.twobit_sequence("chr1", 0, 0);
    // Without storeMasked, the soft-masked region remains uppercase
    assert_eq!(
        seq,
        "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
}

#[test]
fn test_sequence_unmasked_chr1_24_74() {
    let tb = TwoBit::twobit_open(FOO_2BIT, false);
    let seq = tb.twobit_sequence("chr1", 24, 74);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTAGCTAGCTGATC");
}

#[test]
fn test_sequence_unmasked_chr1_60_70() {
    let tb = TwoBit::twobit_open(FOO_2BIT, false);
    let seq = tb.twobit_sequence("chr1", 60, 70);
    assert_eq!(seq, "GTAGCTAGCT");
}

fn main() {}
