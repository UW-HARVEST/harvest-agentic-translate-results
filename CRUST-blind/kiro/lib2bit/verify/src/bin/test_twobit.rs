use lib2bit::twobit::{byte2base, bytes2bases, getByteMaskFromOffset, TwoBit};

const TEST_FILE: &str = "c_src/test/foo.2bit";

// ---- Helper function tests ----

#[test]
fn test_byte2base_all_zero() {
    assert_eq!(byte2base(0x00, 0), 'T');
    assert_eq!(byte2base(0x00, 1), 'T');
    assert_eq!(byte2base(0x00, 2), 'T');
    assert_eq!(byte2base(0x00, 3), 'T');
}

#[test]
fn test_byte2base_all_ones() {
    assert_eq!(byte2base(0xFF, 0), 'G');
    assert_eq!(byte2base(0xFF, 1), 'G');
    assert_eq!(byte2base(0xFF, 2), 'G');
    assert_eq!(byte2base(0xFF, 3), 'G');
}

#[test]
fn test_byte2base_mixed() {
    // 0x1B = 00 01 10 11 -> T C A G
    assert_eq!(byte2base(0x1B, 0), 'T');
    assert_eq!(byte2base(0x1B, 1), 'C');
    assert_eq!(byte2base(0x1B, 2), 'A');
    assert_eq!(byte2base(0x1B, 3), 'G');
}

#[test]
fn test_get_byte_mask_from_offset() {
    assert_eq!(getByteMaskFromOffset(0), 15);
    assert_eq!(getByteMaskFromOffset(1), 7);
    assert_eq!(getByteMaskFromOffset(2), 3);
    assert_eq!(getByteMaskFromOffset(3), 1);
}

#[test]
fn test_bytes2bases_full_byte() {
    // 0x1B = 00 01 10 11 -> T C A G
    let mut seq = vec!['\0'; 4];
    let mut bytes = vec![0x1Bu8];
    bytes2bases(&mut seq, &mut bytes, 4, 0);
    assert_eq!(seq, vec!['T', 'C', 'A', 'G']);
}

#[test]
fn test_bytes2bases_with_offset() {
    // 0x1B = 00 01 10 11, offset=2 -> A G
    let mut seq = vec!['\0'; 2];
    let mut bytes = vec![0x1Bu8];
    bytes2bases(&mut seq, &mut bytes, 2, 2);
    assert_eq!(seq, vec!['A', 'G']);
}

// ---- Header tests ----

#[test]
fn test_header() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.hdr.magic, 0x1A412743);
    assert_eq!(tb.hdr.version, 0);
    assert_eq!(tb.hdr.n_chroms, 2);
}

// ---- Chrom list tests ----

#[test]
fn test_chrom_list() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.cl.chrom.len(), 2);
    assert_eq!(tb.cl.chrom[0], "chr1");
    assert_eq!(tb.cl.chrom[1], "chr2");
    assert_eq!(tb.cl.offset[0], 34);
    assert_eq!(tb.cl.offset[1], 112);
}

// ---- Chrom length tests ----

#[test]
fn test_chrom_len() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.twobit_chrom_len("chr1"), 150);
    assert_eq!(tb.twobit_chrom_len("chr2"), 100);
    assert_eq!(tb.twobit_chrom_len("chrX"), 0);
}

// ---- Index tests ----

#[test]
fn test_index_chr1() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.idx.size[0], 150);
    assert_eq!(tb.idx.n_block_count[0], 2);
    assert_eq!(tb.idx.mask_block_count[0], 1);
    assert_eq!(tb.idx.offset[0], 74);
}

#[test]
fn test_index_chr2() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.idx.size[1], 100);
    assert_eq!(tb.idx.n_block_count[1], 1);
    assert_eq!(tb.idx.mask_block_count[1], 0);
    assert_eq!(tb.idx.offset[1], 136);
}

#[test]
fn test_nblocks() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    // chr1 nblocks
    assert_eq!(tb.idx.n_block_start[0][0], 0);
    assert_eq!(tb.idx.n_block_sizes[0][0], 50);
    assert_eq!(tb.idx.n_block_start[0][1], 100);
    assert_eq!(tb.idx.n_block_sizes[0][1], 50);
    // chr2 nblocks
    assert_eq!(tb.idx.n_block_start[1][0], 50);
    assert_eq!(tb.idx.n_block_sizes[1][0], 50);
}

#[test]
fn test_maskblocks() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.idx.mask_block_start[0][0], 62);
    assert_eq!(tb.idx.mask_block_sizes[0][0], 8);
}

// ---- Sequence tests (with store_masked=true) ----

#[test]
fn test_sequence_chr1_full() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 0, 0);
    assert_eq!(seq.len(), 150);
    assert_eq!(
        seq,
        "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTagctagctGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
}

#[test]
fn test_sequence_chr2_full() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr2", 0, 0);
    assert_eq!(seq.len(), 100);
    assert_eq!(
        seq,
        "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
}

#[test]
fn test_sequence_chr2_full_n_count() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr2", 0, 0);
    let n_count = seq.chars().filter(|&c| c == 'N').count();
    // chr2 has 50 Ns (positions 50-99)
    assert_eq!(n_count, 50);
}

#[test]
fn test_sequence_chr1_24_74() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 24, 74);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTagctagctGATC");
}

#[test]
fn test_sequence_chr1_50_60() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 50, 60);
    assert_eq!(seq, "ACGTACGTAC");
}

#[test]
fn test_sequence_chr1_48_100() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 48, 100);
    assert_eq!(seq, "NNACGTACGTACGTagctagctGATCGATCGTAGCTAGCTAGCTAGCTGATC");
}

#[test]
fn test_sequence_chr2_0_50() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr2", 0, 50);
    assert_eq!(seq, "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATC");
}

#[test]
fn test_sequence_chr2_50_100() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr2", 50, 100);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN");
    assert_eq!(seq.len(), 50);
}

#[test]
fn test_sequence_single_bases() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.twobit_sequence("chr1", 50, 51), "A");
    assert_eq!(tb.twobit_sequence("chr1", 51, 52), "C");
    assert_eq!(tb.twobit_sequence("chr1", 52, 53), "G");
    assert_eq!(tb.twobit_sequence("chr1", 53, 54), "T");
}

// ---- Sequence edge cases ----

#[test]
fn test_sequence_out_of_bounds() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    // end > chrom size -> empty string (C returns NULL)
    assert_eq!(tb.twobit_sequence("chr1", 0, 200), "");
}

#[test]
fn test_sequence_start_ge_end() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    // start >= end -> empty string (C returns NULL)
    assert_eq!(tb.twobit_sequence("chr1", 100, 50), "");
}

#[test]
fn test_sequence_unknown_chrom() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.twobit_sequence("chrX", 0, 0), "");
}

// ---- Sequence without store_masked ----

#[test]
fn test_sequence_no_mask_chr1_full() {
    let tb = TwoBit::twobit_open(TEST_FILE, false);
    let seq = tb.twobit_sequence("chr1", 0, 0);
    assert_eq!(seq.len(), 150);
    // Without store_masked, soft masking is not applied -> all uppercase
    let expected = format!(
        "{}{}{}",
        "N".repeat(50),
        "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATC",
        "N".repeat(50)
    );
    assert_eq!(seq, expected);
}

// ---- Bases integer tests ----

fn extract_u32s(data: &[u8]) -> [u32; 4] {
    let mut result = [0u32; 4];
    for i in 0..4 {
        result[i] = u32::from_le_bytes([
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ]);
    }
    result
}

fn extract_f64s(data: &[u8]) -> [f64; 4] {
    let mut result = [0.0f64; 4];
    for i in 0..4 {
        let start = i * 8;
        result[i] = f64::from_le_bytes([
            data[start],
            data[start + 1],
            data[start + 2],
            data[start + 3],
            data[start + 4],
            data[start + 5],
            data[start + 6],
            data[start + 7],
        ]);
    }
    result
}

#[test]
fn test_bases_int_chr1_full() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr1", 0, 0, 0);
    assert_eq!(data.len(), 16);
    let counts = extract_u32s(&data);
    // A=12, C=12, T=13, G=13
    assert_eq!(counts[0], 12); // A
    assert_eq!(counts[1], 12); // C
    assert_eq!(counts[2], 13); // T
    assert_eq!(counts[3], 13); // G
}

#[test]
fn test_bases_int_chr1_24_74() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr1", 24, 74, 0);
    let counts = extract_u32s(&data);
    assert_eq!(counts[0], 6); // A
    assert_eq!(counts[1], 6); // C
    assert_eq!(counts[2], 6); // T
    assert_eq!(counts[3], 6); // G
}

#[test]
fn test_bases_int_chr2_full() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr2", 0, 0, 0);
    let counts = extract_u32s(&data);
    assert_eq!(counts[0], 12); // A
    assert_eq!(counts[1], 12); // C
    assert_eq!(counts[2], 13); // T
    assert_eq!(counts[3], 13); // G
}

#[test]
fn test_bases_int_chr2_0_50() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr2", 0, 50, 0);
    let counts = extract_u32s(&data);
    assert_eq!(counts[0], 12); // A
    assert_eq!(counts[1], 12); // C
    assert_eq!(counts[2], 13); // T
    assert_eq!(counts[3], 13); // G
}

#[test]
fn test_bases_int_chr1_50_100() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr1", 50, 100, 0);
    let counts = extract_u32s(&data);
    assert_eq!(counts[0], 12); // A
    assert_eq!(counts[1], 12); // C
    assert_eq!(counts[2], 13); // T
    assert_eq!(counts[3], 13); // G
}

// ---- Bases fraction tests ----

#[test]
fn test_bases_frac_chr1_full() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr1", 0, 0, 1);
    assert_eq!(data.len(), 32);
    let fracs = extract_f64s(&data);
    // A=12/150, C=12/150, T=13/150, G=13/150
    assert!((fracs[0] - 0.08).abs() < 1e-10); // A
    assert!((fracs[1] - 0.08).abs() < 1e-10); // C
    assert!((fracs[2] - 13.0 / 150.0).abs() < 1e-10); // T
    assert!((fracs[3] - 13.0 / 150.0).abs() < 1e-10); // G
}

#[test]
fn test_bases_frac_chr1_24_74() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr1", 24, 74, 1);
    let fracs = extract_f64s(&data);
    assert!((fracs[0] - 0.12).abs() < 1e-10); // A
    assert!((fracs[1] - 0.12).abs() < 1e-10); // C
    assert!((fracs[2] - 0.12).abs() < 1e-10); // T
    assert!((fracs[3] - 0.12).abs() < 1e-10); // G
}

#[test]
fn test_bases_frac_chr2_0_50() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr2", 0, 50, 1);
    let fracs = extract_f64s(&data);
    assert!((fracs[0] - 0.24).abs() < 1e-10); // A
    assert!((fracs[1] - 0.24).abs() < 1e-10); // C
    assert!((fracs[2] - 0.26).abs() < 1e-10); // T
    assert!((fracs[3] - 0.26).abs() < 1e-10); // G
}

#[test]
fn test_bases_frac_chr1_50_100() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr1", 50, 100, 1);
    let fracs = extract_f64s(&data);
    assert!((fracs[0] - 0.24).abs() < 1e-10); // A
    assert!((fracs[1] - 0.24).abs() < 1e-10); // C
    assert!((fracs[2] - 0.26).abs() < 1e-10); // T
    assert!((fracs[3] - 0.26).abs() < 1e-10); // G
}

// ---- Bases edge cases ----

#[test]
fn test_bases_unknown_chrom() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chrX", 0, 0, 0);
    assert!(data.is_empty());
}

#[test]
fn test_bases_out_of_bounds() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr1", 0, 200, 0);
    assert!(data.is_empty());
}

#[test]
fn test_bases_start_ge_end() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let data = tb.twobit_bases("chr1", 100, 50, 0);
    assert!(data.is_empty());
}

fn main() {}
