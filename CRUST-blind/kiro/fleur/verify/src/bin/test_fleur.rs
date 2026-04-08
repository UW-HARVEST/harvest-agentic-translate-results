use fleur::fleur::BloomFilter;
use std::fs::File;

#[test]
fn test_initialize_10000() {
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    assert_eq!(bf.h.k, 10);
    assert_eq!(bf.h.m, 143775);
    assert_eq!(bf.m, 2247);
    assert_eq!(bf.h.n, 10000);
    assert_eq!(bf.h.n_value, 0);
    assert_eq!(bf.version, 1);
    assert_eq!(bf.error, 0);
    assert!(bf.v.iter().all(|&x| x == 0));
}

#[test]
fn test_initialize_100() {
    let bf = BloomFilter::initialize(100, 0.00001, b"toto");
    assert_eq!(bf.h.k, 17);
    assert_eq!(bf.h.m, 2396);
    assert_eq!(bf.m, 38);
}

#[test]
fn test_initialize_9000() {
    let bf = BloomFilter::initialize(9000, 0.01, b"titi");
    assert_eq!(bf.h.k, 7);
    assert_eq!(bf.h.m, 86265);
    assert_eq!(bf.m, 1348);
}

#[test]
fn test_fingerprint_bar() {
    let bf = BloomFilter::initialize(100000, 0.01, b"");
    let fp = bf.fingerprint(b"bar");
    assert_eq!(fp, vec![20311, 36825, 412501, 835777, 658914, 853361, 307361]);
}

#[test]
fn test_fingerprint_foo() {
    let bf = BloomFilter::initialize(100000, 0.01, b"");
    let fp = bf.fingerprint(b"foo");
    assert_eq!(fp, vec![888439, 783115, 68504, 105856, 325730, 942013, 799862]);
}

#[test]
fn test_add_new_element() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    assert_eq!(bf.add(b"hello"), 1);
    assert_eq!(bf.h.n_value, 1);
    assert_eq!(bf.modified, 1);
}

#[test]
fn test_add_duplicate() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    assert_eq!(bf.add(b"hello"), 1);
    assert_eq!(bf.add(b"hello"), 0);
    assert_eq!(bf.h.n_value, 1);
}

#[test]
fn test_add_multiple() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    assert_eq!(bf.add(b"hello"), 1);
    assert_eq!(bf.add(b"world"), 1);
    assert_eq!(bf.h.n_value, 2);
}

#[test]
fn test_add_full_filter() {
    let mut bf = BloomFilter::initialize(1, 0.5, b"");
    assert_eq!(bf.add(b"first"), 1);
    assert_eq!(bf.add(b"second"), -1);
}

#[test]
fn test_check_present() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    bf.add(b"hello");
    bf.add(b"world");
    assert_eq!(bf.check(b"hello"), 1);
    assert_eq!(bf.check(b"world"), 1);
}

#[test]
fn test_check_absent() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    bf.add(b"hello");
    bf.add(b"world");
    assert_eq!(bf.check(b"nothere"), 0);
}

#[test]
fn test_set_data() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    bf.set_data(b"foobar");
    assert_eq!(bf.datasize, 6);
    assert_eq!(bf.data, b"foobar");
}

#[test]
fn test_join_compatible() {
    let mut bf1 = BloomFilter::initialize(1000, 0.001, b"");
    let mut bf2 = BloomFilter::initialize(1000, 0.001, b"");
    bf1.add(b"hello");
    bf2.add(b"world");
    let ret = bf1.join(&bf2);
    assert_eq!(ret, 1);
    assert_eq!(bf1.h.n_value, 2);
}

#[test]
fn test_join_incompatible() {
    let mut bf1 = BloomFilter::initialize(1000, 0.001, b"");
    let bf2 = BloomFilter::initialize(2000, 0.001, b"");
    assert_eq!(bf1.join(&bf2), 0);
}

#[test]
fn test_header_check_valid() {
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    assert!(bf.h.check());
}

#[test]
fn test_from_file_datatest() {
    let f = File::open("c_src/test/suite_2/datatest.bloom").unwrap();
    let bf = BloomFilter::from_file(f);
    assert_eq!(bf.error, 0);
    assert_eq!(bf.m, 450);
    assert_eq!(bf.h.n, 1000);
    assert_eq!(bf.h.k, 20);
    assert_eq!(bf.h.m, 28755);
    assert_eq!(bf.h.n_value, 3);
    assert_eq!(bf.h.p, 0.000001);
    assert_eq!(bf.datasize, 5);
    assert_eq!(String::from_utf8_lossy(&bf.data), "toto\n");
}

#[test]
fn test_to_file_and_from_file_roundtrip() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    bf.add(b"hello");
    bf.add(b"world");
    bf.set_data(b"testdata");

    let path = "test_roundtrip.bloom";
    let f = File::create(path).unwrap();
    assert_eq!(bf.to_file(f), 1);

    let f2 = File::open(path).unwrap();
    let bf2 = BloomFilter::from_file(f2);
    assert_eq!(bf2.error, 0);
    assert_eq!(bf2.h.n, bf.h.n);
    assert_eq!(bf2.h.p, bf.h.p);
    assert_eq!(bf2.h.k, bf.h.k);
    assert_eq!(bf2.h.m, bf.h.m);
    assert_eq!(bf2.h.n_value, bf.h.n_value);
    assert_eq!(bf2.m, bf.m);
    assert_eq!(bf2.check(b"hello"), 1);
    assert_eq!(bf2.check(b"world"), 1);
    assert_eq!(bf2.check(b"nothere"), 0);
    assert_eq!(bf2.data, b"testdata");

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_from_file_bad_header() {
    let f = File::open("c_src/test/suite_2/hang2.bin").unwrap();
    let bf = BloomFilter::from_file(f);
    assert_eq!(bf.error, 1);
}

fn main() {}
