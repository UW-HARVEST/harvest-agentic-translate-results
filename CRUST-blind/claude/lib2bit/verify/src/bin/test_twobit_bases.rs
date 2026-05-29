use lib2bit::twobit::TwoBit;

const FOO_2BIT: &str = "c_src/test/foo.2bit";

fn parse_counts(bytes: &[u8]) -> [u32; 4] {
    assert_eq!(bytes.len(), 16);
    let mut out = [0u32; 4];
    for i in 0..4 {
        out[i] = u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
    }
    out
}

fn parse_fractions(bytes: &[u8]) -> [f64; 4] {
    assert_eq!(bytes.len(), 32);
    let mut out = [0f64; 4];
    for i in 0..4 {
        out[i] = f64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap());
    }
    out
}

#[test]
fn test_bases_chr1_full_counts() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 0, 0, 0);
    let cnts = parse_counts(&bytes);
    // From C: A=12 C=12 T=13 G=13
    assert_eq!(cnts, [12, 12, 13, 13]);
}

#[test]
fn test_bases_chr1_full_fractions() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 0, 0, 1);
    let fr = parse_fractions(&bytes);
    // From C: A=0.080000 C=0.080000 T=0.086667 G=0.086667
    let eps = 1e-9;
    assert!((fr[0] - 12.0 / 150.0).abs() < eps);
    assert!((fr[1] - 12.0 / 150.0).abs() < eps);
    assert!((fr[2] - 13.0 / 150.0).abs() < eps);
    assert!((fr[3] - 13.0 / 150.0).abs() < eps);
}

#[test]
fn test_bases_chr1_24_74_counts() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 24, 74, 0);
    let cnts = parse_counts(&bytes);
    // From C: A=6 C=6 T=6 G=6
    assert_eq!(cnts, [6, 6, 6, 6]);
}

#[test]
fn test_bases_chr1_24_74_fractions() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 24, 74, 1);
    let fr = parse_fractions(&bytes);
    let eps = 1e-9;
    assert!((fr[0] - 6.0 / 50.0).abs() < eps);
    assert!((fr[1] - 6.0 / 50.0).abs() < eps);
    assert!((fr[2] - 6.0 / 50.0).abs() < eps);
    assert!((fr[3] - 6.0 / 50.0).abs() < eps);
}

#[test]
fn test_bases_chr2_full_counts() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr2", 0, 0, 0);
    let cnts = parse_counts(&bytes);
    // From C: A=12 C=12 T=13 G=13
    assert_eq!(cnts, [12, 12, 13, 13]);
}

#[test]
fn test_bases_chr2_full_fractions() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr2", 0, 0, 1);
    let fr = parse_fractions(&bytes);
    let eps = 1e-9;
    assert!((fr[0] - 0.12).abs() < eps);
    assert!((fr[1] - 0.12).abs() < eps);
    assert!((fr[2] - 0.13).abs() < eps);
    assert!((fr[3] - 0.13).abs() < eps);
}

#[test]
fn test_bases_chr1_50_100() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 50, 100, 0);
    let cnts = parse_counts(&bytes);
    assert_eq!(cnts, [12, 12, 13, 13]);
}

#[test]
fn test_bases_chr1_100_150_all_n() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 100, 150, 0);
    let cnts = parse_counts(&bytes);
    // All N region
    assert_eq!(cnts, [0, 0, 0, 0]);
}

#[test]
fn test_bases_chr1_50_70() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 50, 70, 0);
    let cnts = parse_counts(&bytes);
    assert_eq!(cnts, [5, 5, 5, 5]);
}

#[test]
fn test_bases_chr1_60_80() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 60, 80, 0);
    let cnts = parse_counts(&bytes);
    assert_eq!(cnts, [4, 4, 6, 6]);
}

#[test]
fn test_bases_chr1_75_100() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 75, 100, 0);
    let cnts = parse_counts(&bytes);
    assert_eq!(cnts, [6, 6, 7, 6]);
}

#[test]
fn test_bases_chr2_0_50() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr2", 0, 50, 0);
    let cnts = parse_counts(&bytes);
    assert_eq!(cnts, [12, 12, 13, 13]);
}

#[test]
fn test_bases_invalid_start_ge_end() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 100, 50, 0);
    assert!(bytes.is_empty());
}

#[test]
fn test_bases_end_too_large() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chr1", 0, 200, 0);
    assert!(bytes.is_empty());
}

#[test]
fn test_bases_missing_chrom() {
    let tb = TwoBit::twobit_open(FOO_2BIT, true);
    let bytes = tb.twobit_bases("chrX", 0, 10, 0);
    assert!(bytes.is_empty());
}

fn main() {}
