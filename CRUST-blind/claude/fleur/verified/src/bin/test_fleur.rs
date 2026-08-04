use fleur::fleur::{BloomFilter, Header};
use std::fs::File;
use std::io::Write;

const FIXTURE_DIR: &str = "c_src/test/suite_2";

#[test]
fn test_initialize_basic() {
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    assert_eq!(bf.version, 1);
    assert_eq!(bf.h.version, 1);
    assert_eq!(bf.h.n, 10000);
    assert_eq!(bf.h.p, 0.001);
    assert_eq!(bf.h.k, 10);
    assert_eq!(bf.h.m, 143775);
    assert_eq!(bf.h.n_value, 0);
    assert_eq!(bf.m, 2247);
    assert_eq!(bf.v.len(), 2247);
    for word in &bf.v {
        assert_eq!(*word, 0);
    }
    assert_eq!(bf.modified, 0);
    assert_eq!(bf.error, 0);
    assert_eq!(bf.datasize, 0);
    assert!(bf.data.is_empty());
}

#[test]
fn test_initialize_with_data() {
    let bf = BloomFilter::initialize(100, 0.00001, b"toto");
    assert_eq!(bf.h.n, 100);
    assert_eq!(bf.h.p, 0.00001);
    assert_eq!(bf.h.k, 17);
    assert_eq!(bf.h.m, 2396);
    assert_eq!(bf.m, 38);
    assert_eq!(bf.v.len(), 38);
    assert_eq!(bf.data, b"toto");
    assert_eq!(bf.datasize, 4);
}

#[test]
fn test_initialize_titi() {
    let bf = BloomFilter::initialize(9000, 0.01, b"titi");
    assert_eq!(bf.h.n, 9000);
    assert_eq!(bf.h.p, 0.01);
    assert_eq!(bf.h.k, 7);
    assert_eq!(bf.h.m, 86265);
    assert_eq!(bf.m, 1348);
    assert_eq!(bf.data, b"titi");
}

#[test]
fn test_initialize_thousand() {
    let bf = BloomFilter::initialize(1000, 0.001, b"");
    assert_eq!(bf.h.k, 10);
    assert_eq!(bf.h.m, 14377);
    assert_eq!(bf.m, 225);
}

#[test]
fn test_initialize_100k_001() {
    let bf = BloomFilter::initialize(100000, 0.01, b"");
    assert_eq!(bf.h.k, 7);
    assert_eq!(bf.h.m, 958505);
    assert_eq!(bf.m, 14977);
}

#[test]
fn test_fingerprint_bar() {
    // From C suite_2: fp("bar") with n=100000, p=0.01 yields these values
    let bf = BloomFilter::initialize(100000, 0.01, b"");
    let fp = bf.fingerprint(b"bar");
    let expected: [u64; 7] = [20311, 36825, 412501, 835777, 658914, 853361, 307361];
    assert_eq!(fp.len(), 7);
    for i in 0..7 {
        assert_eq!(fp[i], expected[i], "mismatch at index {}", i);
    }
}

#[test]
fn test_fingerprint_foo() {
    // Reference values from running C reference
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    let fp = bf.fingerprint(b"foo");
    let expected: [u64; 10] = [
        133314, 134190, 102599, 81256, 48525, 20998, 60352, 97178, 14601, 65525,
    ];
    assert_eq!(fp.len(), 10);
    for i in 0..10 {
        assert_eq!(fp[i], expected[i]);
    }
}

#[test]
fn test_fingerprint_hello() {
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    let fp = bf.fingerprint(b"hello");
    let expected: [u64; 10] = [
        31028, 86598, 57165, 98504, 21276, 22246, 62081, 2935, 75903, 109764,
    ];
    assert_eq!(fp, expected.to_vec());
}

#[test]
fn test_add_and_check_basic() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    assert_eq!(bf.add(b"alpha"), 1);
    assert_eq!(bf.add(b"beta"), 1);
    assert_eq!(bf.add(b"gamma"), 1);

    assert_eq!(bf.h.n_value, 3);
    assert_eq!(bf.modified, 1);

    assert_eq!(bf.check(b"alpha"), 1);
    assert_eq!(bf.check(b"beta"), 1);
    assert_eq!(bf.check(b"gamma"), 1);
    assert_eq!(bf.check(b"delta"), 0);
}

#[test]
fn test_add_duplicate_returns_zero() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    assert_eq!(bf.add(b"alpha"), 1);
    // Adding the same item again returns 0 (already in filter), n_value not incremented.
    assert_eq!(bf.add(b"alpha"), 0);
    assert_eq!(bf.h.n_value, 1);
}

#[test]
fn test_add_full_filter_returns_minus_one() {
    // Capacity=2 filter
    let mut bf = BloomFilter::initialize(2, 0.01, b"");
    assert_eq!(bf.add(b"x"), 1);
    assert_eq!(bf.add(b"y"), 1);
    // Filter is full (N=2, capacity n=2), so further add returns -1.
    assert_eq!(bf.add(b"z"), -1);
    assert_eq!(bf.h.n, 2);
    assert_eq!(bf.h.n_value, 2);
}

#[test]
fn test_check_empty_filter() {
    let bf = BloomFilter::initialize(100, 0.01, b"");
    assert_eq!(bf.check(b"anything"), 0);
    assert_eq!(bf.check(b""), 0);
}

#[test]
fn test_set_data() {
    let mut bf = BloomFilter::initialize(10, 0.01, b"");
    bf.set_data(b"newdata");
    assert_eq!(bf.data, b"newdata");
    assert_eq!(bf.datasize, 7);

    // Replace
    bf.set_data(b"x");
    assert_eq!(bf.data, b"x");
    assert_eq!(bf.datasize, 1);

    // Empty
    bf.set_data(b"");
    assert_eq!(bf.data.len(), 0);
    assert_eq!(bf.datasize, 0);
}

#[test]
fn test_header_check_good() {
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    assert!(bf.h.check());
}

#[test]
fn test_header_check_bad_version() {
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    bf.h.version = 2;
    assert!(!bf.h.check());
}

#[test]
fn test_header_check_k_too_high() {
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    bf.h.k = 9223372036854775807u64;
    assert!(!bf.h.check());
}

#[test]
fn test_header_check_p_too_small() {
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    bf.h.p = 0.0;
    assert!(!bf.h.check());
}

#[test]
fn test_header_check_p_too_large() {
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    bf.h.p = 1.5;
    assert!(!bf.h.check());
}

#[test]
fn test_header_check_n_value_exceeds_n() {
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    bf.h.n_value = bf.h.n + 1;
    assert!(!bf.h.check());
}

#[test]
fn test_header_check_m_inconsistent() {
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    bf.h.m += 1;
    assert!(!bf.h.check());
}

#[test]
fn test_header_check_k_inconsistent() {
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    bf.h.k += 1;
    assert!(!bf.h.check());
}

#[test]
fn test_header_check_p_equals_one() {
    // C: p > 1 fails. p == 1.0 should pass the p check, but the implied m/k will fail
    // because log(1.0) == 0 and m=0, etc. Reference output: check returns 0.
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    bf.h.p = 1.0;
    assert!(!bf.h.check());
}

#[test]
fn test_join_failure_dst_full() {
    // Use the join1.bloom (capacity=100, N=60), join2.bloom (capacity=100, N=60).
    // Joining j2 into j1 -> dst.N + src.N = 120 > 100 => returns -1 (filter full).
    let f1 = File::open(format!("{}/join1.bloom", FIXTURE_DIR)).unwrap();
    let f2 = File::open(format!("{}/join2.bloom", FIXTURE_DIR)).unwrap();
    let mut j1 = BloomFilter::from_file(f1);
    let j2 = BloomFilter::from_file(f2);
    assert_eq!(j1.error, 0);
    assert_eq!(j2.error, 0);
    assert_eq!(j1.h.n, 100);
    assert_eq!(j1.h.n_value, 60);
    assert_eq!(j2.h.n, 100);
    assert_eq!(j2.h.n_value, 60);
    assert_eq!(j1.join(&j2), -1);
}

#[test]
fn test_join_failure_mismatch() {
    // Join datatest.bloom (n=1000, p=1e-06, k=20, m=28755, M=450) into join1.bloom
    // (n=100, ...) -> mismatched characteristics => returns 0.
    let f0 = File::open(format!("{}/datatest.bloom", FIXTURE_DIR)).unwrap();
    let f1 = File::open(format!("{}/join1.bloom", FIXTURE_DIR)).unwrap();
    let j0 = BloomFilter::from_file(f0);
    let mut j1 = BloomFilter::from_file(f1);
    assert_eq!(j0.error, 0);
    assert_eq!(j0.h.n, 1000);
    assert_eq!(j1.h.n, 100);
    assert_eq!(j1.join(&j0), 0);
}

#[test]
fn test_join_success() {
    // Join j3 (N=30) into j1 (N=60). Total = 90 <= 100, characteristics match.
    let f1 = File::open(format!("{}/join1.bloom", FIXTURE_DIR)).unwrap();
    let f3 = File::open(format!("{}/join3.bloom", FIXTURE_DIR)).unwrap();
    let mut j1 = BloomFilter::from_file(f1);
    let j3 = BloomFilter::from_file(f3);
    assert_eq!(j3.h.n_value, 30);
    let pre_n = j1.h.n_value;
    assert_eq!(j1.join(&j3), 1);
    assert_eq!(j1.h.n_value, pre_n + 30);
}

#[test]
fn test_join_mismatch_n() {
    let mut a = BloomFilter::initialize(100, 0.01, b"");
    let b = BloomFilter::initialize(200, 0.01, b"");
    assert_eq!(a.join(&b), 0);
}

#[test]
fn test_join_mismatch_p() {
    let mut a = BloomFilter::initialize(100, 0.01, b"");
    let b = BloomFilter::initialize(100, 0.001, b"");
    assert_eq!(a.join(&b), 0);
}

#[test]
fn test_from_file_datatest() {
    let f = File::open(format!("{}/datatest.bloom", FIXTURE_DIR)).unwrap();
    let bf = BloomFilter::from_file(f);
    assert_eq!(bf.error, 0);
    assert_eq!(bf.m, 450);
    assert_eq!(bf.h.n, 1000);
    assert_eq!(bf.h.p, 1e-06);
    assert_eq!(bf.h.k, 20);
    assert_eq!(bf.h.m, 28755);
    assert_eq!(bf.datasize, 5);
    assert_eq!(bf.v.len(), 450);
}

#[test]
fn test_from_file_hang2_invalid_p() {
    // hang2.bin has p > 1, so check_header returns 0 and error is set to 1.
    let f = File::open(format!("{}/hang2.bin", FIXTURE_DIR)).unwrap();
    let bf = BloomFilter::from_file(f);
    assert_eq!(bf.error, 1);
}

#[test]
fn test_header_fixture_good() {
    // header.bin is a 48-byte standalone header.
    let mut f = File::open(format!("{}/header.bin", FIXTURE_DIR)).unwrap();
    let mut bytes = [0u8; 48];
    use std::io::Read;
    f.read_exact(&mut bytes).unwrap();
    let h = Header {
        version: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        n: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        p: f64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        k: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        m: u64::from_le_bytes(bytes[32..40].try_into().unwrap()),
        n_value: u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
    };
    assert_eq!(h.version, 1);
    assert_eq!(h.n, 354253067);
    assert!((h.p - 0.0001).abs() < 1e-9);
    assert_eq!(h.k, 14);
    assert_eq!(h.m, 6791072655);
    assert_eq!(h.n_value, 354249652);
    assert!(h.check());
}

#[test]
fn test_header_fixture_hang0_p_too_small() {
    let mut f = File::open(format!("{}/hang0.bin", FIXTURE_DIR)).unwrap();
    let mut bytes = [0u8; 48];
    use std::io::Read;
    f.read_exact(&mut bytes).unwrap();
    let h = Header {
        version: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        n: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        p: f64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        k: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        m: u64::from_le_bytes(bytes[32..40].try_into().unwrap()),
        n_value: u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
    };
    assert!(!h.check());
}

#[test]
fn test_header_fixture_hang1_bad_version() {
    let mut f = File::open(format!("{}/hang1.bin", FIXTURE_DIR)).unwrap();
    let mut bytes = [0u8; 48];
    use std::io::Read;
    f.read_exact(&mut bytes).unwrap();
    let h = Header {
        version: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        n: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        p: f64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        k: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        m: u64::from_le_bytes(bytes[32..40].try_into().unwrap()),
        n_value: u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
    };
    assert!(!h.check());
}

#[test]
fn test_to_file_and_from_file_round_trip() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("crust_rt_{}.bloom", std::process::id()));
    {
        let mut bf = BloomFilter::initialize(100, 0.001, b"");
        bf.set_data(b"hi!");
        bf.add(b"one");
        bf.add(b"two");
        let f = File::create(&path).unwrap();
        let ret = bf.to_file(f);
        assert_eq!(ret, 1);
    }

    let f = File::open(&path).unwrap();
    let bf = BloomFilter::from_file(f);
    assert_eq!(bf.error, 0);
    assert_eq!(bf.h.n, 100);
    assert_eq!(bf.h.p, 0.001);
    assert_eq!(bf.h.k, 10);
    assert_eq!(bf.h.m, 1437);
    assert_eq!(bf.m, 23);
    assert_eq!(bf.h.n_value, 2);
    assert_eq!(bf.datasize, 3);
    assert_eq!(bf.data, b"hi!");
    assert_eq!(bf.check(b"one"), 1);
    assert_eq!(bf.check(b"two"), 1);
    assert_eq!(bf.check(b"three"), 0);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_to_file_no_data() {
    // round-trip an empty-data filter
    let dir = std::env::temp_dir();
    let path = dir.join(format!("crust_rt_nodata_{}.bloom", std::process::id()));
    {
        let mut bf = BloomFilter::initialize(50, 0.01, b"");
        bf.add(b"hello");
        let mut f = File::create(&path).unwrap();
        // touch f so it's referenced
        f.flush().ok();
        let f2 = File::create(&path).unwrap();
        let ret = bf.to_file(f2);
        assert_eq!(ret, 1);
    }

    let f = File::open(&path).unwrap();
    let bf = BloomFilter::from_file(f);
    assert_eq!(bf.error, 0);
    assert_eq!(bf.h.n, 50);
    assert_eq!(bf.h.n_value, 1);
    assert_eq!(bf.datasize, 0);
    assert_eq!(bf.check(b"hello"), 1);
    assert_eq!(bf.check(b"world"), 0);
    let _ = std::fs::remove_file(&path);
}

fn main() {}
