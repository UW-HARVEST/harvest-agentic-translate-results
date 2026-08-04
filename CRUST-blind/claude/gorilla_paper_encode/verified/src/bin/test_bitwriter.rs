use gorilla_paper_encode::gorilla::BitWriter;

fn fresh() -> BitWriter {
    let mut w = BitWriter {
        cache: [0u8; 1024],
        pos: 0,
        byte: 0,
        bit_count: 0,
    };
    w.bitwriter_init();
    w
}

#[test]
fn test_init_state() {
    let w = fresh();
    assert_eq!(w.pos, 0);
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
    for byte in w.cache.iter() {
        assert_eq!(*byte, 0);
    }
}

#[test]
fn test_write_byte_aligned() {
    // C ground truth: bw1 pos=1 byte=00 bit_count=8 (after writing 0x10).
    let mut w = fresh();
    let rc = w.write_byte(0x10);
    assert_eq!(rc, 0);
    assert_eq!(w.pos, 1);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
    assert_eq!(w.cache[0], 0x10);
}

#[test]
fn test_write_three_bits() {
    // C ground truth: bw2 pos=0 byte=a0 bit_count=5 after writing 1,0,1.
    let mut w = fresh();
    assert_eq!(w.write_bit(true), 0);
    assert_eq!(w.write_bit(false), 0);
    assert_eq!(w.write_bit(true), 0);
    assert_eq!(w.pos, 0);
    assert_eq!(w.byte, 0xA0);
    assert_eq!(w.bit_count, 5);
}

#[test]
fn test_write_bits_16() {
    // C ground truth: bw3 pos=2 byte=00 bit_count=8 cache0=ab cache1=cd.
    let mut w = fresh();
    let rc = w.write_bits(0xABCDu64, 16);
    assert_eq!(rc, 0);
    assert_eq!(w.pos, 2);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
    assert_eq!(w.cache[0], 0xAB);
    assert_eq!(w.cache[1], 0xCD);
}

#[test]
fn test_write_bits_two_4bit_chunks() {
    // C ground truth: bw4 pos=1 byte=00 bit_count=8 cache0=53.
    let mut w = fresh();
    assert_eq!(w.write_bits(0x5u64, 4), 0);
    assert_eq!(w.write_bits(0x3u64, 4), 0);
    assert_eq!(w.pos, 1);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
    assert_eq!(w.cache[0], 0x53);
}

#[test]
fn test_write_bits_64_then_flush() {
    // C ground truth: bw5 pos=8 byte=00 bit_count=8 cache=de ad be ef ca fe ba be 00.
    let mut w = fresh();
    assert_eq!(w.write_bits(0xDEADBEEFCAFEBABEu64, 64), 0);
    assert_eq!(w.write_flush(false), 0);
    assert_eq!(w.pos, 8);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
    assert_eq!(w.cache[0], 0xDE);
    assert_eq!(w.cache[1], 0xAD);
    assert_eq!(w.cache[2], 0xBE);
    assert_eq!(w.cache[3], 0xEF);
    assert_eq!(w.cache[4], 0xCA);
    assert_eq!(w.cache[5], 0xFE);
    assert_eq!(w.cache[6], 0xBA);
    assert_eq!(w.cache[7], 0xBE);
    assert_eq!(w.cache[8], 0x00);
}

#[test]
fn test_write_bits_zero_nbits_noop() {
    // C ground truth: bw6 pos=0 byte=00 bit_count=8.
    let mut w = fresh();
    assert_eq!(w.write_bits(0u64, 0), 0);
    assert_eq!(w.pos, 0);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_write_bits_invalid_nbits() {
    // C: returns -1 for nbits>64 or nbits<0.
    let mut w = fresh();
    assert_eq!(w.write_bits(0u64, 65), -1);
    let mut w2 = fresh();
    assert_eq!(w2.write_bits(0u64, -1), -1);
}

#[test]
fn test_write_byte_unaligned() {
    // C ground truth: after write_bits(0x3,3) then write_byte(0xFF):
    // pos=1 byte=e0 bit_count=5 cache0=7f.
    let mut w = fresh();
    assert_eq!(w.write_bits(0x3u64, 3), 0);
    assert_eq!(w.write_byte(0xFF), 0);
    assert_eq!(w.pos, 1);
    assert_eq!(w.byte, 0xE0);
    assert_eq!(w.bit_count, 5);
    assert_eq!(w.cache[0], 0x7F);
}

#[test]
fn test_write_eight_bits_one_at_a_time() {
    // After 7 ones: pos=0 byte=fe bit_count=1.
    // After 7 ones then a zero: pos=1 byte=00 bit_count=8 cache0=fe.
    let mut w = fresh();
    for _ in 0..7 {
        assert_eq!(w.write_bit(true), 0);
    }
    assert_eq!(w.pos, 0);
    assert_eq!(w.byte, 0xFE);
    assert_eq!(w.bit_count, 1);

    assert_eq!(w.write_bit(false), 0);
    assert_eq!(w.pos, 1);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
    assert_eq!(w.cache[0], 0xFE);
}

#[test]
fn test_write_flush_already_aligned_noop() {
    let mut w = fresh();
    let rc = w.write_flush(false);
    assert_eq!(rc, 0);
    assert_eq!(w.pos, 0);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_append_to_cache_increments_pos() {
    let mut w = fresh();
    w.byte = 0x42;
    let rc = w.append_to_cache();
    assert_eq!(rc, 0);
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0x42);
}

fn main() {}
