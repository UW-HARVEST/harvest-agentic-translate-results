use lib2bit::twobit::TwoBit;

const FOO_2BIT: &str = "c_src/test/foo.2bit";

#[test]
fn test_chrom_len_chr1() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    assert_eq!(tb.twobit_chrom_len("chr1"), 150);
}

#[test]
fn test_chrom_len_chr2() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    assert_eq!(tb.twobit_chrom_len("chr2"), 100);
}

#[test]
fn test_chrom_len_missing() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    assert_eq!(tb.twobit_chrom_len("chr3"), 0);
    assert_eq!(tb.twobit_chrom_len(""), 0);
    assert_eq!(tb.twobit_chrom_len("Chr1"), 0); // case-sensitive
}

fn main() {}
