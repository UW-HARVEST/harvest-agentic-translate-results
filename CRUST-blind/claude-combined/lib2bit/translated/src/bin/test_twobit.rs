use lib2bit::twobit::{byte2base, bytes2bases, getByteMaskFromOffset, TwoBit};

const FOO_2BIT: &str = "c_src/test/foo.2bit";

fn open_masked() -> TwoBit {
    TwoBit::twobit_open(FOO_2BIT, true)
}

fn open_unmasked() -> TwoBit {
    TwoBit::twobit_open(FOO_2BIT, false)
}

#[test]
fn test_open_header() {
    let tb = open_masked();
    assert_eq!(tb.hdr.magic, 0x1A412743);
    assert_eq!(tb.hdr.version, 0);
    assert_eq!(tb.hdr.n_chroms, 2);
}

#[test]
fn test_chrom_list() {
    let tb = open_masked();
    assert_eq!(tb.cl.chrom.len(), 2);
    assert_eq!(tb.cl.chrom[0], "chr1");
    assert_eq!(tb.cl.chrom[1], "chr2");
    assert_eq!(tb.cl.offset[0], 34);
    assert_eq!(tb.cl.offset[1], 112);
}

#[test]
fn test_index_chr1() {
    let tb = open_masked();
    assert_eq!(tb.idx.size[0], 150);
    assert_eq!(tb.idx.n_block_count[0], 2);
    assert_eq!(tb.idx.n_block_start[0], vec![0u32, 100]);
    assert_eq!(tb.idx.n_block_sizes[0], vec![50u32, 50]);
    assert_eq!(tb.idx.mask_block_count[0], 1);
    assert_eq!(tb.idx.mask_block_start[0], vec![62u32]);
    assert_eq!(tb.idx.mask_block_sizes[0], vec![8u32]);
    assert_eq!(tb.idx.offset[0], 74);
}

#[test]
fn test_index_chr2() {
    let tb = open_masked();
    assert_eq!(tb.idx.size[1], 100);
    assert_eq!(tb.idx.n_block_count[1], 1);
    assert_eq!(tb.idx.n_block_start[1], vec![50u32]);
    assert_eq!(tb.idx.n_block_sizes[1], vec![50u32]);
    assert_eq!(tb.idx.mask_block_count[1], 0);
    assert_eq!(tb.idx.offset[1], 136);
}

#[test]
fn test_chrom_len() {
    let tb = open_masked();
    assert_eq!(tb.twobit_chrom_len("chr1"), 150);
    assert_eq!(tb.twobit_chrom_len("chr2"), 100);
    assert_eq!(tb.twobit_chrom_len("nope"), 0);
}

#[test]
fn test_sequence_chr1_full() {
    let tb = open_masked();
    let seq = tb.twobit_sequence("chr1", 0, 0);
    assert_eq!(
        seq,
        "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTagctagctGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
    assert_eq!(seq.len(), 150);
}

#[test]
fn test_sequence_chr1_24_74() {
    let tb = open_masked();
    let seq = tb.twobit_sequence("chr1", 24, 74);
    assert_eq!(
        seq,
        "NNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTagctagctGATC"
    );
    assert_eq!(seq.len(), 50);
}

#[test]
fn test_sequence_chr2_full() {
    let tb = open_masked();
    let seq = tb.twobit_sequence("chr2", 0, 0);
    assert_eq!(
        seq,
        "ACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
    assert_eq!(seq.len(), 100);
}

#[test]
fn test_sequence_chr2_5_25() {
    let tb = open_masked();
    let seq = tb.twobit_sequence("chr2", 5, 25);
    assert_eq!(seq, "CGTACGTAGCTAGCTGATCG");
}

#[test]
fn test_sequence_chr1_50_60() {
    let tb = open_masked();
    let seq = tb.twobit_sequence("chr1", 50, 60);
    assert_eq!(seq, "ACGTACGTAC");
}

#[test]
fn test_sequence_chr1_60_70_mask_boundary() {
    let tb = open_masked();
    // Soft-mask block on chr1 is start=62 size=8; check that lowercase
    // appears starting at the right position.
    let seq = tb.twobit_sequence("chr1", 60, 70);
    assert_eq!(seq, "GTagctagct");
}

#[test]
fn test_sequence_invalid_chrom() {
    let tb = open_masked();
    let seq = tb.twobit_sequence("nope", 0, 0);
    assert_eq!(seq, "");
}

#[test]
fn test_sequence_invalid_range() {
    let tb = open_masked();
    // start >= end
    assert_eq!(tb.twobit_sequence("chr1", 50, 50), "");
    assert_eq!(tb.twobit_sequence("chr1", 60, 50), "");
    // end > size
    assert_eq!(tb.twobit_sequence("chr1", 0, 1000), "");
}

#[test]
fn test_sequence_unmasked() {
    let tb = open_unmasked();
    let seq = tb.twobit_sequence("chr1", 0, 0);
    // No soft-mask info available so all uppercase except for N blocks.
    assert_eq!(
        seq,
        "NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNACGTACGTACGTAGCTAGCTGATCGATCGTAGCTAGCTAGCTAGCTGATCNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN"
    );
}

fn parse_counts(buf: &[u8]) -> (u32, u32, u32, u32) {
    assert_eq!(buf.len(), 16);
    let a = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    let c = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    let t = u32::from_le_bytes(buf[8..12].try_into().unwrap());
    let g = u32::from_le_bytes(buf[12..16].try_into().unwrap());
    (a, c, t, g)
}

fn parse_fracs(buf: &[u8]) -> (f64, f64, f64, f64) {
    assert_eq!(buf.len(), 32);
    let a = f64::from_le_bytes(buf[0..8].try_into().unwrap());
    let c = f64::from_le_bytes(buf[8..16].try_into().unwrap());
    let t = f64::from_le_bytes(buf[16..24].try_into().unwrap());
    let g = f64::from_le_bytes(buf[24..32].try_into().unwrap());
    (a, c, t, g)
}

#[test]
fn test_bases_chr1_full_counts() {
    let tb = open_masked();
    let buf = tb.twobit_bases("chr1", 0, 0, 0);
    let (a, c, t, g) = parse_counts(&buf);
    assert_eq!(a, 12);
    assert_eq!(c, 12);
    assert_eq!(t, 13);
    assert_eq!(g, 13);
}

#[test]
fn test_bases_chr1_24_74_counts() {
    let tb = open_masked();
    let buf = tb.twobit_bases("chr1", 24, 74, 0);
    let (a, c, t, g) = parse_counts(&buf);
    assert_eq!(a, 6);
    assert_eq!(c, 6);
    assert_eq!(t, 6);
    assert_eq!(g, 6);
}

#[test]
fn test_bases_chr2_full_counts() {
    let tb = open_masked();
    let buf = tb.twobit_bases("chr2", 0, 0, 0);
    let (a, c, t, g) = parse_counts(&buf);
    assert_eq!(a, 12);
    assert_eq!(c, 12);
    assert_eq!(t, 13);
    assert_eq!(g, 13);
}

#[test]
fn test_bases_chr1_full_fractions() {
    let tb = open_masked();
    let buf = tb.twobit_bases("chr1", 0, 0, 1);
    let (a, c, t, g) = parse_fracs(&buf);
    let eps = 1e-9;
    assert!((a - 0.08).abs() < eps, "a was {}", a);
    assert!((c - 0.08).abs() < eps, "c was {}", c);
    assert!((t - (13.0_f64 / 150.0_f64)).abs() < eps, "t was {}", t);
    assert!((g - (13.0_f64 / 150.0_f64)).abs() < eps, "g was {}", g);
}

#[test]
fn test_bases_chr1_24_74_fractions() {
    let tb = open_masked();
    let buf = tb.twobit_bases("chr1", 24, 74, 1);
    let (a, c, t, g) = parse_fracs(&buf);
    let eps = 1e-9;
    assert!((a - 0.12).abs() < eps);
    assert!((c - 0.12).abs() < eps);
    assert!((t - 0.12).abs() < eps);
    assert!((g - 0.12).abs() < eps);
}

#[test]
fn test_bases_chr2_full_fractions() {
    let tb = open_masked();
    let buf = tb.twobit_bases("chr2", 0, 0, 1);
    let (a, c, t, g) = parse_fracs(&buf);
    let eps = 1e-9;
    assert!((a - 0.12).abs() < eps);
    assert!((c - 0.12).abs() < eps);
    assert!((t - 0.13).abs() < eps);
    assert!((g - 0.13).abs() < eps);
}

#[test]
fn test_bases_invalid_chrom() {
    let tb = open_masked();
    assert_eq!(tb.twobit_bases("nope", 0, 0, 0), Vec::<u8>::new());
}

#[test]
fn test_bases_invalid_range() {
    let tb = open_masked();
    assert_eq!(tb.twobit_bases("chr1", 50, 50, 0), Vec::<u8>::new());
    assert_eq!(tb.twobit_bases("chr1", 0, 999, 0), Vec::<u8>::new());
}

#[test]
fn test_byte2base() {
    // bases array is "TCAG", offset selects position from the highest two bits down.
    // For byte 0x1B (binary 00 01 10 11) = T C A G at offsets 0,1,2,3
    assert_eq!(byte2base(0x1B, 0), 'T');
    assert_eq!(byte2base(0x1B, 1), 'C');
    assert_eq!(byte2base(0x1B, 2), 'A');
    assert_eq!(byte2base(0x1B, 3), 'G');

    // 0x00 -> all T
    assert_eq!(byte2base(0x00, 0), 'T');
    assert_eq!(byte2base(0x00, 3), 'T');
    // 0xFF -> all G
    assert_eq!(byte2base(0xFF, 0), 'G');
    assert_eq!(byte2base(0xFF, 3), 'G');
}

#[test]
fn test_bytes2bases() {
    // 0x1B = TCAG, 0xE4 = 11 10 01 00 = GACT
    let mut bytes = [0x1Bu8, 0xE4u8];
    let mut seq = vec!['\0'; 8];
    bytes2bases(&mut seq, &mut bytes, 8, 0);
    assert_eq!(seq, vec!['T', 'C', 'A', 'G', 'G', 'A', 'C', 'T']);
}

#[test]
fn test_bytes2bases_with_offset() {
    // With offset=1, skip the first base of the first byte.
    let mut bytes = [0x1Bu8, 0xE4u8];
    let mut seq = vec!['\0'; 7];
    bytes2bases(&mut seq, &mut bytes, 7, 1);
    assert_eq!(seq, vec!['C', 'A', 'G', 'G', 'A', 'C', 'T']);
}

#[test]
fn test_get_byte_mask_from_offset() {
    // The Rust API has this returning unit; just exercise it.
    getByteMaskFromOffset(0);
    getByteMaskFromOffset(1);
    getByteMaskFromOffset(2);
    getByteMaskFromOffset(3);
}

#[test]
fn test_low_level_seek_tell_read() {
    let mut tb = open_masked();
    tb.twobitSeek(0);
    assert_eq!(tb.twobitTell(), 0);
    let buf = vec![0u8; 0];
    let n = tb.twobitRead(&buf, 4, 4);
    assert_eq!(n, 4);
    assert_eq!(tb.twobitTell(), 16);

    // Seek past end is a no-op
    let big = tb.sz + 1000;
    tb.twobitSeek(big);
    assert_eq!(tb.twobitTell(), 16);
}

#[test]
fn test_construct_sequence_method() {
    let mut tb = open_masked();
    let chars = tb.constructSequence(0, 50, 60);
    let s: String = chars.iter().collect();
    assert_eq!(s, "ACGTACGTAC");
}

#[test]
fn test_n_mask_method() {
    let tb_full = open_masked();
    // build a fresh tb to call mutating method
    let mut tb = open_masked();
    let mut seq: Vec<char> = vec!['A'; 50];
    // chr1 start=24 end=74; first N block ends at 50, soft-mask later
    tb.NMask(&mut seq, 0, 24, 74);
    let s: String = seq.iter().collect();
    // Expect Ns from positions 0..26 (block end 50 - start 24)
    assert_eq!(&s[..26], "NNNNNNNNNNNNNNNNNNNNNNNNNN");
    // Anything past 26 should remain 'A' (soft-mask not applied here)
    for ch in &seq[26..50] {
        assert_eq!(*ch, 'A');
    }
    let _ = tb_full;
}

#[test]
fn test_soft_mask_method() {
    let mut tb = open_masked();
    // chr1 has soft-mask block at 62..70; ask to mask between 60..70
    let mut seq: Vec<char> = vec!['A'; 10];
    tb.softMask(&mut seq, 0, 60, 70);
    let s: String = seq.iter().collect();
    // Positions 0..2 stay 'A', positions 2..10 should be lowercase 'a'
    assert_eq!(&s[..2], "AA");
    for ch in &seq[2..10] {
        assert_eq!(*ch, 'a');
    }
}

#[test]
fn test_get_mask_method() {
    let mut tb = open_masked();
    // chr1 has N blocks at start=0 size=50 and start=100 size=50.
    // Querying with start=0 end=150 should return the first block.
    let (idx, ms, me) = tb.getMask(0, 0, 150);
    assert_eq!(idx, 0);
    assert_eq!(ms, 0);
    assert_eq!(me, 50);
}

#[test]
fn test_destroy_methods() {
    let mut tb = open_masked();
    tb.twoBitIndexDestroy();
    assert_eq!(tb.idx.size.len(), 0);
    assert_eq!(tb.idx.n_block_count.len(), 0);
    assert_eq!(tb.idx.n_block_start.len(), 0);
    assert_eq!(tb.idx.offset.len(), 0);

    let mut tb = open_masked();
    tb.twobitChromListDestroy();
    assert_eq!(tb.cl.chrom.len(), 0);
    assert_eq!(tb.cl.offset.len(), 0);

    let mut tb = open_masked();
    tb.twobitHdrDestroy();
    assert_eq!(tb.hdr.magic, 0);
    assert_eq!(tb.hdr.version, 0);
    assert_eq!(tb.hdr.n_chroms, 0);
}

#[test]
fn test_twobit_close() {
    let mut tb = open_masked();
    tb.twobit_close();
    assert_eq!(tb.hdr.magic, 0);
    assert_eq!(tb.cl.chrom.len(), 0);
    assert_eq!(tb.idx.size.len(), 0);
}

fn main() {}
