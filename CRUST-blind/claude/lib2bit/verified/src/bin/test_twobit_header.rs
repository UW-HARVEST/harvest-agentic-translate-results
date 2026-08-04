use lib2bit::twobit::TwoBit;

const FOO_2BIT: &str = "c_src/test/foo.2bit";

#[test]
fn test_open_and_header_masked() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    assert_eq!(tb.hdr.magic, 0x1A412743);
    assert_eq!(tb.hdr.version, 0);
    assert_eq!(tb.hdr.n_chroms, 2);
}

#[test]
fn test_open_and_header_unmasked() {
    let tb = TwoBit::twobit_open(FOO_2BIT, false);
    assert_eq!(tb.hdr.magic, 0x1A412743);
    assert_eq!(tb.hdr.version, 0);
    assert_eq!(tb.hdr.n_chroms, 2);
}

#[test]
fn test_header_destroy() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twobitHdrDestroy();
    assert_eq!(tb.hdr.magic, 0);
    assert_eq!(tb.hdr.version, 0);
    assert_eq!(tb.hdr.n_chroms, 0);
}

#[test]
fn test_chrom_list_open() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    assert_eq!(tb.cl.chrom.len(), 2);
    assert_eq!(tb.cl.chrom[0], "chr1");
    assert_eq!(tb.cl.chrom[1], "chr2");
    assert_eq!(tb.cl.offset.len(), 2);
    assert_eq!(tb.cl.offset[0], 34);
    assert_eq!(tb.cl.offset[1], 112);
}

#[test]
fn test_chrom_list_destroy() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twobitChromListDestroy();
    assert_eq!(tb.cl.chrom.len(), 0);
    assert_eq!(tb.cl.offset.len(), 0);
}

#[test]
fn test_index_masked() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    // chr1
    assert_eq!(tb.idx.size[0], 150);
    assert_eq!(tb.idx.n_block_count[0], 2);
    assert_eq!(tb.idx.n_block_start[0], vec![0u32, 100u32]);
    assert_eq!(tb.idx.n_block_sizes[0], vec![50u32, 50u32]);
    assert_eq!(tb.idx.mask_block_count[0], 1);
    assert_eq!(tb.idx.mask_block_start[0], vec![62u32]);
    assert_eq!(tb.idx.mask_block_sizes[0], vec![8u32]);
    assert_eq!(tb.idx.offset[0], 0x4a);

    // chr2
    assert_eq!(tb.idx.size[1], 100);
    assert_eq!(tb.idx.n_block_count[1], 1);
    assert_eq!(tb.idx.n_block_start[1], vec![50u32]);
    assert_eq!(tb.idx.n_block_sizes[1], vec![50u32]);
    assert_eq!(tb.idx.mask_block_count[1], 0);
    assert_eq!(tb.idx.mask_block_start[1], Vec::<u32>::new());
    assert_eq!(tb.idx.mask_block_sizes[1], Vec::<u32>::new());
    assert_eq!(tb.idx.offset[1], 0x88);
}

#[test]
fn test_index_unmasked_no_mask_storage() {
    let tb = TwoBit::twobit_open(FOO_2BIT, false);
    // sizes/n-blocks/maskBlockCount still populated
    assert_eq!(tb.idx.size[0], 150);
    assert_eq!(tb.idx.size[1], 100);
    assert_eq!(tb.idx.n_block_count[0], 2);
    assert_eq!(tb.idx.n_block_count[1], 1);
    assert_eq!(tb.idx.mask_block_count[0], 1);
    assert_eq!(tb.idx.mask_block_count[1], 0);
    // mask_block_start/sizes should be empty (not stored)
    assert_eq!(tb.idx.mask_block_start.len(), 0);
    assert_eq!(tb.idx.mask_block_sizes.len(), 0);
    assert_eq!(tb.idx.offset[0], 0x4a);
    assert_eq!(tb.idx.offset[1], 0x88);
}

#[test]
fn test_index_destroy() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twoBitIndexDestroy();
    assert_eq!(tb.idx.size.len(), 0);
    assert_eq!(tb.idx.n_block_count.len(), 0);
    assert_eq!(tb.idx.n_block_start.len(), 0);
    assert_eq!(tb.idx.n_block_sizes.len(), 0);
    assert_eq!(tb.idx.mask_block_count.len(), 0);
    assert_eq!(tb.idx.mask_block_start.len(), 0);
    assert_eq!(tb.idx.mask_block_sizes.len(), 0);
    assert_eq!(tb.idx.offset.len(), 0);
}

#[test]
fn test_close_clears_everything() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twobit_close();
    assert_eq!(tb.hdr.magic, 0);
    assert_eq!(tb.hdr.version, 0);
    assert_eq!(tb.hdr.n_chroms, 0);
    assert_eq!(tb.cl.chrom.len(), 0);
    assert_eq!(tb.idx.size.len(), 0);
}

fn main() {}
