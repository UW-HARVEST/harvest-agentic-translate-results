use fleur::fleur::BloomFilter;

#[test]
fn test_initialize_10000() {
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    assert_eq!(bf.h.k, 10);
    assert_eq!(bf.h.m, 143775);
    assert_eq!(bf.m, 2247);
    assert_eq!(bf.h.n, 10000);
    assert_eq!(bf.h.p, 0.001);
    assert_eq!(bf.h.n_value, 0);
    assert_eq!(bf.h.version, 1);
    assert_eq!(bf.version, 1);
    assert_eq!(bf.modified, 0);
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
    assert_eq!(bf.h.k, 7);
    assert_eq!(bf.h.m, 958505);
    assert_eq!(bf.m, 14977);
    let fp = bf.fingerprint(b"bar");
    assert_eq!(fp, vec![20311, 36825, 412501, 835777, 658914, 853361, 307361]);
}

#[test]
fn test_fingerprint_test() {
    let bf = BloomFilter::initialize(100000, 0.01, b"");
    let fp = bf.fingerprint(b"test");
    assert_eq!(fp, vec![930241, 109553, 788979, 549396, 53371, 299542, 347173]);
}

#[test]
fn test_add_and_check() {
    let mut bf = BloomFilter::initialize(1000, 0.01, b"");
    assert_eq!(bf.h.k, 7);
    assert_eq!(bf.h.m, 9585);
    assert_eq!(bf.m, 150);

    assert_eq!(bf.add(b"hello"), 1);
    assert_eq!(bf.add(b"world"), 1);
    assert_eq!(bf.add(b"hello"), 0); // duplicate
    assert_eq!(bf.h.n_value, 2);
    assert_eq!(bf.modified, 1);

    assert_eq!(bf.check(b"hello"), 1);
    assert_eq!(bf.check(b"world"), 1);
    assert_eq!(bf.check(b"nothere"), 0);
}

#[test]
fn test_add_full_filter() {
    let mut bf = BloomFilter::initialize(2, 0.1, b"");
    assert_eq!(bf.h.k, 4);
    assert_eq!(bf.h.m, 9);
    assert_eq!(bf.m, 1);

    assert_eq!(bf.add(b"aaa"), 1);
    assert_eq!(bf.add(b"bbb"), 1);
    assert_eq!(bf.add(b"ccc"), -1);
    assert_eq!(bf.h.n_value, 2);
}

#[test]
fn test_header_check_valid() {
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    assert!(bf.h.check());
}

#[test]
fn test_set_data() {
    let mut bf = BloomFilter::initialize(100, 0.01, b"");
    bf.set_data(b"mydata");
    assert_eq!(bf.data, b"mydata");
    assert_eq!(bf.datasize, 6);
}

#[test]
fn test_to_file_and_from_file() {
    use std::fs::File;
    let path = "/tmp/harvest-crust-verify-mLNNha/project/writing-test-rust.bloom";

    let mut bf = BloomFilter::initialize(1000, 0.01, b"");
    bf.add(b"hello");
    bf.add(b"world");

    let f = File::create(path).unwrap();
    assert_eq!(bf.to_file(f), 1);

    let f2 = File::open(path).unwrap();
    let bf2 = BloomFilter::from_file(f2);
    assert_eq!(bf2.error, 0);
    assert_eq!(bf2.h.n, 1000);
    assert_eq!(bf2.h.p, 0.01);
    assert_eq!(bf2.h.k, 7);
    assert_eq!(bf2.h.m, 9585);
    assert_eq!(bf2.h.n_value, 2);
    assert_eq!(bf2.m, 150);
    assert_eq!(bf2.v, bf.v);

    assert_eq!(bf2.check(b"hello"), 1);
    assert_eq!(bf2.check(b"world"), 1);
    assert_eq!(bf2.check(b"nothere"), 0);

    std::fs::remove_file(path).ok();
}

#[test]
fn test_from_file_datatest() {
    use std::fs::File;
    let f = File::open("/tmp/harvest-crust-verify-mLNNha/project/c_src/test/suite_2/datatest.bloom").unwrap();
    let bf = BloomFilter::from_file(f);
    assert_eq!(bf.error, 0);
    assert_eq!(bf.m, 450);
}

fn main() {}
