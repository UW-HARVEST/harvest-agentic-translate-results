use lib2bit::twobit::{byte2base, bytes2bases, getByteMaskFromOffset, TwoBit};

const TEST_FILE: &str = "c_src/test/foo.2bit";

// ---- Helper to decode twobit_bases results ----

fn decode_f64_vec(v: &[u8]) -> Vec<f64> {
    v.chunks_exact(8)
        .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn decode_u32_vec(v: &[u8]) -> Vec<u32> {
    v.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

// ---- twobit_chrom_len ----

#[test]
fn test_chrom_len_chr1() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.twobit_chrom_len("chr1"), 150);
}

#[test]
fn test_chrom_len_chr2() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.twobit_chrom_len("chr2"), 100);
}

#[test]
fn test_chrom_len_nonexistent() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    assert_eq!(tb.twobit_chrom_len("chrX"), 0);
}

// ---- twobit_sequence: full chromosomes ----

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
    // chr2 has no soft masking in the non-N region; N-block at positions 50-99 is lowercase 'n' in 2bit? No, N-blocks are uppercase N.
    // C output: ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN
    // Wait, the C output shows lowercase 'n' for chr2 N-block? Let me check...
    // C output: "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    // Actually looking at the raw output, chr2 N-block uses lowercase 'n'? No - the C output shows:
    // chr2 full: [ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN]
    // Those look like lowercase 'n'. Let me count: 50 uppercase chars + 50 'n' chars? But the .fa file shows uppercase N.
    // Actually in the terminal output they could be either. The C NMask function sets 'N' (uppercase).
    // The expected output from c_src/test/expected shows lowercase 'n' for chr2... let me check.
    // From the C output captured above, chr2 full has lowercase 'n' at the end.
    // Wait no - looking more carefully at the raw C output:
    // "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    // In monospace these are ambiguous. The C NMask function uses 'N' (uppercase). So these should be uppercase N.
    // But wait - the .fa shows uppercase N, and the C code NMask sets seq[pos] = 'N'. So it's uppercase.
    // However chr2 has no soft masking in the .fa file, so no lowercase conversion happens.
    // Actually wait - does chr2 have a mask block? Let me check the 2bit file structure.
    // The C output for chr2 shows the N region. The NMask function sets 'N'. So uppercase.
    // But the raw terminal output I captured shows lowercase... Let me re-examine.
    // The C output line: "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    // Hmm, those 'n' chars after GATC... In the C code NMask sets 'N' (uppercase). So they must be uppercase N.
    // The font rendering in the output just makes them look similar. They are 'N'.
    assert_eq!(
        seq,
        "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
}

#[test]
fn test_sequence_chr1_subrange_50_100() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 50, 100);
    assert_eq!(seq, "ACGTACGTACGTagctagctGATCGATCGTAGCTAGCTAGCTAGCTGATC");
}

#[test]
fn test_sequence_chr1_subrange_24_74() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 24, 74);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTagctagctGATC");
}

#[test]
fn test_sequence_chr1_n_block_only_start() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 0, 50);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN");
    assert_eq!(seq.len(), 50);
}

#[test]
fn test_sequence_chr1_n_block_only_end() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 100, 150);
    assert_eq!(seq, "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN");
    assert_eq!(seq.len(), 50);
}

#[test]
fn test_sequence_single_base() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 50, 51);
    assert_eq!(seq, "A");
}

#[test]
fn test_sequence_chr2_first_half() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr2", 0, 50);
    assert_eq!(seq, "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATC");
    assert_eq!(seq.len(), 50);
}

#[test]
fn test_sequence_chr2_boundary() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr2", 48, 52);
    assert_eq!(seq, "TCNN");
}

// ---- twobit_sequence: error cases ----

#[test]
fn test_sequence_nonexistent_chrom() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chrX", 0, 0);
    assert_eq!(seq, "");
}

#[test]
fn test_sequence_out_of_bounds() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 0, 200);
    assert_eq!(seq, "");
}

#[test]
fn test_sequence_start_equals_end() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 50, 50);
    assert_eq!(seq, "");
}

#[test]
fn test_sequence_start_greater_than_end() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let seq = tb.twobit_sequence("chr1", 100, 50);
    assert_eq!(seq, "");
}

// ---- twobit_sequence: without soft masking ----

#[test]
fn test_sequence_no_soft_mask_full() {
    let tb = TwoBit::twobit_open(TEST_FILE, false);
    let seq = tb.twobit_sequence("chr1", 0, 0);
    // Without soft masking, lowercase letters become uppercase
    assert_eq!(
        seq,
        "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
}

#[test]
fn test_sequence_no_soft_mask_subrange() {
    let tb = TwoBit::twobit_open(TEST_FILE, false);
    let seq = tb.twobit_sequence("chr1", 50, 100);
    assert_eq!(seq, "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATC");
}

// ---- twobit_bases: fraction mode ----

#[test]
fn test_bases_chr1_full_fraction() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr1", 0, 0, 1);
    assert_eq!(raw.len(), 32); // 4 f64s
    let vals = decode_f64_vec(&raw);
    // ACTG order
    let eps = 1e-10;
    assert!((vals[0] - 12.0 / 150.0).abs() < eps, "A fraction"); // A
    assert!((vals[1] - 12.0 / 150.0).abs() < eps, "C fraction"); // C
    assert!((vals[2] - 13.0 / 150.0).abs() < eps, "T fraction"); // T
    assert!((vals[3] - 13.0 / 150.0).abs() < eps, "G fraction"); // G
}

#[test]
fn test_bases_chr1_24_74_fraction() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr1", 24, 74, 1);
    let vals = decode_f64_vec(&raw);
    let eps = 1e-10;
    assert!((vals[0] - 6.0 / 50.0).abs() < eps); // A = 0.12
    assert!((vals[1] - 6.0 / 50.0).abs() < eps); // C = 0.12
    assert!((vals[2] - 6.0 / 50.0).abs() < eps); // T = 0.12
    assert!((vals[3] - 6.0 / 50.0).abs() < eps); // G = 0.12
}

#[test]
fn test_bases_chr1_50_100_fraction() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr1", 50, 100, 1);
    let vals = decode_f64_vec(&raw);
    let eps = 1e-10;
    assert!((vals[0] - 12.0 / 50.0).abs() < eps); // A = 0.24
    assert!((vals[1] - 12.0 / 50.0).abs() < eps); // C = 0.24
    assert!((vals[2] - 13.0 / 50.0).abs() < eps); // T = 0.26
    assert!((vals[3] - 13.0 / 50.0).abs() < eps); // G = 0.26
}

#[test]
fn test_bases_chr2_full_fraction() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr2", 0, 0, 1);
    let vals = decode_f64_vec(&raw);
    let eps = 1e-10;
    assert!((vals[0] - 12.0 / 100.0).abs() < eps);
    assert!((vals[1] - 12.0 / 100.0).abs() < eps);
    assert!((vals[2] - 13.0 / 100.0).abs() < eps);
    assert!((vals[3] - 13.0 / 100.0).abs() < eps);
}

#[test]
fn test_bases_chr2_0_50_fraction() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr2", 0, 50, 1);
    let vals = decode_f64_vec(&raw);
    let eps = 1e-10;
    assert!((vals[0] - 12.0 / 50.0).abs() < eps);
    assert!((vals[1] - 12.0 / 50.0).abs() < eps);
    assert!((vals[2] - 13.0 / 50.0).abs() < eps);
    assert!((vals[3] - 13.0 / 50.0).abs() < eps);
}

// ---- twobit_bases: count mode ----

#[test]
fn test_bases_chr1_full_count() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr1", 0, 0, 0);
    assert_eq!(raw.len(), 16); // 4 u32s
    let vals = decode_u32_vec(&raw);
    assert_eq!(vals, vec![12, 12, 13, 13]); // A, C, T, G
}

#[test]
fn test_bases_chr1_50_100_count() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr1", 50, 100, 0);
    let vals = decode_u32_vec(&raw);
    assert_eq!(vals, vec![12, 12, 13, 13]);
}

#[test]
fn test_bases_chr2_0_50_count() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr2", 0, 50, 0);
    let vals = decode_u32_vec(&raw);
    assert_eq!(vals, vec![12, 12, 13, 13]);
}

#[test]
fn test_bases_chr2_full_count() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr2", 0, 0, 0);
    let vals = decode_u32_vec(&raw);
    assert_eq!(vals, vec![12, 12, 13, 13]);
}

// ---- twobit_bases: error cases ----

#[test]
fn test_bases_nonexistent_chrom() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chrX", 0, 0, 1);
    assert!(raw.is_empty());
}

#[test]
fn test_bases_out_of_bounds() {
    let tb = TwoBit::twobit_open(TEST_FILE, true);
    let raw = tb.twobit_bases("chr1", 0, 200, 1);
    assert!(raw.is_empty());
}

// ---- byte2base ----

#[test]
fn test_byte2base_0x1b() {
    assert_eq!(byte2base(0x1B, 0), 'T');
    assert_eq!(byte2base(0x1B, 1), 'C');
    assert_eq!(byte2base(0x1B, 2), 'A');
    assert_eq!(byte2base(0x1B, 3), 'G');
}

#[test]
fn test_byte2base_0x00() {
    assert_eq!(byte2base(0x00, 0), 'T');
}

#[test]
fn test_byte2base_0xff() {
    assert_eq!(byte2base(0xFF, 0), 'G');
    assert_eq!(byte2base(0xFF, 3), 'G');
}

#[test]
fn test_byte2base_0xe4() {
    assert_eq!(byte2base(0xE4, 0), 'G');
    assert_eq!(byte2base(0xE4, 1), 'A');
    assert_eq!(byte2base(0xE4, 2), 'C');
    assert_eq!(byte2base(0xE4, 3), 'T');
}

// ---- bytes2bases ----

#[test]
fn test_bytes2bases_single_byte() {
    let mut seq = vec!['\0'; 4];
    let mut bytes = vec![0xE4u8]; // G A C T
    bytes2bases(&mut seq, &mut bytes, 4, 0);
    assert_eq!(seq, vec!['G', 'A', 'C', 'T']);
}

#[test]
fn test_bytes2bases_with_offset() {
    let mut seq = vec!['\0'; 2];
    let mut bytes = vec![0xE4u8]; // G A C T, offset 2 => C T
    bytes2bases(&mut seq, &mut bytes, 2, 2);
    assert_eq!(seq, vec!['C', 'T']);
}

// ---- getByteMaskFromOffset ----

#[test]
fn test_get_byte_mask_from_offset() {
    // The public function returns () but we test it doesn't panic
    getByteMaskFromOffset(0);
    getByteMaskFromOffset(1);
    getByteMaskFromOffset(2);
    getByteMaskFromOffset(3);
}

// ---- twobit_close ----

#[test]
fn test_twobit_close() {
    let mut tb = TwoBit::twobit_open(TEST_FILE, true);
    tb.twobit_close(); // should not panic
}

fn main() {}
