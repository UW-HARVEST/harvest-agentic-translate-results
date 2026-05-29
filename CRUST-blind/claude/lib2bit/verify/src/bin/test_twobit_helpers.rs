use lib2bit::twobit::{byte2base, bytes2bases, getByteMaskFromOffset, TwoBit};

const FOO_2BIT: &str = "c_src/test/foo.2bit";

#[test]
fn test_byte2base_0x1b() {
    // 0x1B = 0001_1011 -> T C A G across offsets 0..=3
    assert_eq!(byte2base(0x1B, 0), 'T');
    assert_eq!(byte2base(0x1B, 1), 'C');
    assert_eq!(byte2base(0x1B, 2), 'A');
    assert_eq!(byte2base(0x1B, 3), 'G');
}

#[test]
fn test_byte2base_uniform() {
    // 0x00 = TTTT
    assert_eq!(byte2base(0x00, 0), 'T');
    assert_eq!(byte2base(0x00, 1), 'T');
    assert_eq!(byte2base(0x00, 2), 'T');
    assert_eq!(byte2base(0x00, 3), 'T');

    // 0x55 = 01010101 = CCCC
    assert_eq!(byte2base(0x55, 0), 'C');
    assert_eq!(byte2base(0x55, 1), 'C');
    assert_eq!(byte2base(0x55, 2), 'C');
    assert_eq!(byte2base(0x55, 3), 'C');

    // 0xAA = 10101010 = AAAA
    assert_eq!(byte2base(0xAA, 0), 'A');
    assert_eq!(byte2base(0xAA, 1), 'A');
    assert_eq!(byte2base(0xAA, 2), 'A');
    assert_eq!(byte2base(0xAA, 3), 'A');

    // 0xFF = 11111111 = GGGG
    assert_eq!(byte2base(0xFF, 0), 'G');
    assert_eq!(byte2base(0xFF, 1), 'G');
    assert_eq!(byte2base(0xFF, 2), 'G');
    assert_eq!(byte2base(0xFF, 3), 'G');
}

#[test]
fn test_bytes2bases_full_byte_offset0() {
    // 0x1B -> "TCAG"
    let mut bts = [0x1Bu8];
    let mut out = ['\0'; 4];
    bytes2bases(&mut out, &mut bts, 4, 0);
    let s: String = out.iter().collect();
    assert_eq!(s, "TCAG");
}

#[test]
fn test_bytes2bases_other_byte() {
    // 0xE4 = 11_10_01_00 -> "GACT"
    let mut bts = [0xE4u8];
    let mut out = ['\0'; 4];
    bytes2bases(&mut out, &mut bts, 4, 0);
    let s: String = out.iter().collect();
    assert_eq!(s, "GACT");
}

#[test]
fn test_bytes2bases_two_bytes() {
    // 0x1B 0xE4 -> "TCAGGACT"
    let mut bts = [0x1Bu8, 0xE4u8];
    let mut out = ['\0'; 8];
    bytes2bases(&mut out, &mut bts, 8, 0);
    let s: String = out.iter().collect();
    assert_eq!(s, "TCAGGACT");
}

#[test]
fn test_bytes2bases_offset1() {
    // 0x1B 0xE4 with sz=5 offset=1 -> "CAGGA"
    let mut bts = [0x1Bu8, 0xE4u8];
    let mut out = ['\0'; 5];
    bytes2bases(&mut out, &mut bts, 5, 1);
    let s: String = out.iter().collect();
    assert_eq!(s, "CAGGA");
}

#[test]
fn test_bytes2bases_partial_sz2() {
    // sz=2 returns first 2 of "TCAG" -> "TC"
    let mut bts = [0x1Bu8];
    let mut out = ['\0'; 2];
    bytes2bases(&mut out, &mut bts, 2, 0);
    let s: String = out.iter().collect();
    assert_eq!(s, "TC");
}

#[test]
fn test_bytes2bases_partial_sz3() {
    // sz=3 returns first 3 of "TCAG" -> "TCA"
    let mut bts = [0x1Bu8];
    let mut out = ['\0'; 3];
    bytes2bases(&mut out, &mut bts, 3, 0);
    let s: String = out.iter().collect();
    assert_eq!(s, "TCA");
}

#[test]
fn test_get_byte_mask_from_offset() {
    // Public Rust signature returns (); just call it to ensure it doesn't panic.
    getByteMaskFromOffset(0);
    getByteMaskFromOffset(1);
    getByteMaskFromOffset(2);
    getByteMaskFromOffset(3);
    getByteMaskFromOffset(-1);
    getByteMaskFromOffset(99);
}

#[test]
fn test_twobit_tell_initial() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    // After open, all data has been consumed - offset should equal something past header etc.
    // The exact value depends on parsing flow; just confirm tell returns the same as offset.
    let t = tb.twobitTell();
    assert_eq!(t, tb.offset);
}

#[test]
fn test_twobit_seek() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twobitSeek(0);
    assert_eq!(tb.twobitTell(), 0);
    tb.twobitSeek(16);
    assert_eq!(tb.twobitTell(), 16);
}

#[test]
fn test_twobit_seek_past_end_no_change() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twobitSeek(10);
    let prev = tb.twobitTell();
    // Try to seek past end
    tb.twobitSeek(100_000);
    // Offset should remain unchanged when seeking past end (matching C's failure path)
    assert_eq!(tb.twobitTell(), prev);
}

#[test]
fn test_twobit_read_advances_offset() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twobitSeek(0);
    let buf = vec![0u8; 16];
    let n = tb.twobitRead(&buf, 4, 4);
    assert_eq!(n, 4);
    assert_eq!(tb.twobitTell(), 16);
}

#[test]
fn test_twobit_read_past_end() {
    let mut tb = TwoBit::twobit_open(FOO_2BIT, true);
    tb.twobitSeek(0);
    let buf = vec![0u8; 8];
    // Try reading more than the file contains
    let n = tb.twobitRead(&buf, 1, 1_000_000);
    assert_eq!(n, 0);
}

fn main() {}
