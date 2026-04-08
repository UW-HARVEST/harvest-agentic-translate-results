use ljmm::ljmm;

#[test]
fn test_parse_addr_hex_lower() {
    let (val, len) = ljmm::parse_addr(b"00400000-");
    assert_eq!(val, 0x400000);
    assert_eq!(len, 8);
}

#[test]
fn test_parse_addr_hex_618() {
    let (val, len) = ljmm::parse_addr(b"00618000-");
    assert_eq!(val, 0x618000);
    assert_eq!(len, 8);
}

#[test]
fn test_parse_addr_long() {
    let (val, len) = ljmm::parse_addr(b"7f2b4b200000-");
    assert_eq!(val, 0x7f2b4b200000);
    assert_eq!(len, 12);
}

#[test]
fn test_parse_addr_uppercase() {
    let (val, len) = ljmm::parse_addr(b"DEADBEEF ");
    assert_eq!(val, 0xdeadbeef);
    assert_eq!(len, 8);
}

#[test]
fn test_parse_addr_zero() {
    let (val, len) = ljmm::parse_addr(b"0 ");
    assert_eq!(val, 0x0);
    assert_eq!(len, 1);
}

#[test]
fn test_parse_addr_lowercase_af() {
    let (val, len) = ljmm::parse_addr(b"abcdef01-");
    assert_eq!(val, 0xabcdef01);
    assert_eq!(len, 8);
}

#[test]
fn test_parse_addr_empty() {
    let (val, len) = ljmm::parse_addr(b"");
    assert_eq!(val, 0x0);
    assert_eq!(len, 0);
}

#[test]
fn test_page_align_zero() {
    assert_eq!(ljmm::page_align_addr(0, 4096, 4095), 0);
}

#[test]
fn test_page_align_one() {
    assert_eq!(ljmm::page_align_addr(1, 4096, 4095), 0x1000);
}

#[test]
fn test_page_align_just_below_page() {
    assert_eq!(ljmm::page_align_addr(0xfff, 4096, 4095), 0x1000);
}

#[test]
fn test_page_align_exact_page() {
    assert_eq!(ljmm::page_align_addr(0x1000, 4096, 4095), 0x1000);
}

#[test]
fn test_page_align_one_over() {
    assert_eq!(ljmm::page_align_addr(0x1001, 4096, 4095), 0x2000);
}

#[test]
fn test_page_align_two_pages_minus_one() {
    assert_eq!(ljmm::page_align_addr(0x1fff, 4096, 4095), 0x2000);
}

#[test]
fn test_page_align_two_pages() {
    assert_eq!(ljmm::page_align_addr(0x2000, 4096, 4095), 0x2000);
}

#[test]
fn test_page_align_already_aligned() {
    assert_eq!(ljmm::page_align_addr(0x619000, 4096, 4095), 0x619000);
}

#[test]
fn test_page_align_32k_minus_1() {
    assert_eq!(ljmm::page_align_addr(0x7fff, 4096, 4095), 0x8000);
}

#[test]
fn test_page_align_32k() {
    assert_eq!(ljmm::page_align_addr(0x8000, 4096, 4095), 0x8000);
}

fn main() {}
