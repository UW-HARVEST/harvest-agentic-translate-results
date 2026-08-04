use gorilla_paper_encode::gorilla::BitReader;

fn fresh<'a>(data: &'a [u8]) -> BitReader<'a> {
    BitReader {
        data,
        len: data.len() as u32,
        v: 0,
        n: 0,
    }
}

fn fresh_initialized<'a>(data: &'a [u8]) -> BitReader<'a> {
    let mut br = fresh(data);
    br.bit_readbuf();
    br
}

#[test]
fn test_bit_readbuf_full_8_bytes() {
    // C: when 8 bytes available, br->v is loaded big-endian, n=64.
    let data = [0x00u8, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
    let mut br = fresh(&data);
    let rc = br.bit_readbuf();
    assert_eq!(rc, 0);
    assert_eq!(br.v, 0x0011223344556677u64);
    assert_eq!(br.n, 64);
}

#[test]
fn test_bit_readbuf_initial_state_after_reset_emulation() {
    // Mirrors what bitread_reset performs: zeroes n/v and calls bit_readbuf.
    let data: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
        0xFF,
    ];
    let br = fresh_initialized(&data);
    assert_eq!(br.v, 0x0011223344556677u64);
    assert_eq!(br.n, 64);
}

#[test]
fn test_read_bits_no_data_returns_all_ones() {
    // C: when br->n == 0 read_bits returns ~0.
    let empty: [u8; 0] = [];
    let mut br = BitReader {
        data: &empty,
        len: 0,
        v: 0,
        n: 0,
    };
    let r = br.read_bits(8);
    assert_eq!(r, !0u64);
}

#[test]
fn test_read_bits_chunks() {
    // Initial buf: 16 bytes from C probe. Reads 4, 12, 32, 16 bits return: 0, 0x11, 0x22334455, 0x6677.
    let data: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
        0xFF,
    ];
    let mut br = fresh_initialized(&data);
    assert_eq!(br.read_bits(4), 0x0u64);
    assert_eq!(br.read_bits(12), 0x011u64);
    assert_eq!(br.read_bits(32), 0x22334455u64);
    assert_eq!(br.read_bits(16), 0x6677u64);
}

#[test]
fn test_read_bits_64_then_32() {
    // C ground truth: read 64 then 32 from 1..=12 yields 0x0102030405060708 then 0x090a0b0c.
    let data: [u8; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let mut br = fresh_initialized(&data);
    assert_eq!(br.read_bits(64), 0x0102030405060708u64);
    assert_eq!(br.read_bits(32), 0x090a0b0cu64);
}

#[test]
fn test_read_bit_single() {
    // Only the highest bit of the first byte is 1.
    let data: [u8; 8] = [0x80, 0, 0, 0, 0, 0, 0, 0];
    let mut br = fresh_initialized(&data);
    assert_eq!(br.read_bit(), 1u64);
    assert_eq!(br.read_bit(), 0u64);
}

#[test]
fn test_can_read_bitfast_and_read_bitfast() {
    let data: [u8; 8] = [0x80, 0, 0, 0, 0, 0, 0, 0];
    let mut br = fresh_initialized(&data);
    assert!(br.can_read_bitfast());
    let b1 = br.read_bitfast();
    let b2 = br.read_bitfast();
    assert!(b1);
    assert!(!b2);
    // After two read_bitfast calls n decreases by 2.
    assert_eq!(br.n, 62);
}

#[test]
fn test_bitread_reset_emulation_via_assignment() {
    // bitread_reset takes &mut [u8] which Rust enforces lifetimes on; emulate the
    // reset behavior by re-initializing the reader manually and calling bit_readbuf.
    let data: [u8; 8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];
    let mut br = BitReader {
        data: &data,
        len: data.len() as u32,
        v: 0,
        n: 0,
    };
    br.bit_readbuf();
    assert_eq!(br.v, 0xAABBCCDDEEFF0011u64);
    assert_eq!(br.n, 64);
}

fn main() {}
