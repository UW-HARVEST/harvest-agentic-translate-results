use fleur::fleur::{BloomFilter, Header};

#[test]
fn test_initialize_10000_0_001() {
    // From C ground truth: init(10000, 0.001) k=10 m=143775 M=2247 N=0 version=1
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    assert_eq!(bf.h.k, 10);
    assert_eq!(bf.h.m, 143775);
    assert_eq!(bf.m, 2247);
    assert_eq!(bf.h.n, 10000);
    assert_eq!(bf.h.p, 0.001);
    assert_eq!(bf.h.n_value, 0);
    assert_eq!(bf.h.version, 1);
    assert_eq!(bf.modified, 0);
    assert_eq!(bf.error, 0);
    assert_eq!(bf.v.len(), 2247);
    for &x in bf.v.iter() {
        assert_eq!(x, 0);
    }
}

#[test]
fn test_initialize_100000_0_01() {
    // From C: init(100000, 0.01) k=7 m=958505 M=14977 N=0
    let bf = BloomFilter::initialize(100000, 0.01, b"");
    assert_eq!(bf.h.k, 7);
    assert_eq!(bf.h.m, 958505);
    assert_eq!(bf.m, 14977);
    assert_eq!(bf.h.n_value, 0);
}

#[test]
fn test_initialize_100_0_00001() {
    // From C: init(100, 0.00001) k=17 m=2396 M=38
    let bf = BloomFilter::initialize(100, 0.00001, b"toto");
    assert_eq!(bf.h.k, 17);
    assert_eq!(bf.h.m, 2396);
    assert_eq!(bf.m, 38);
    assert_eq!(bf.data, b"toto");
    assert_eq!(bf.datasize, 4);
}

#[test]
fn test_initialize_9000_0_01() {
    // From C: init(9000, 0.01) k=7 m=86265 M=1348
    let bf = BloomFilter::initialize(9000, 0.01, b"titi");
    assert_eq!(bf.h.k, 7);
    assert_eq!(bf.h.m, 86265);
    assert_eq!(bf.m, 1348);
    assert_eq!(bf.data, b"titi");
}

#[test]
fn test_fingerprint_bar() {
    // From C test suite: fingerprint of "bar" with init(100000, 0.01) k=7
    // => [20311, 36825, 412501, 835777, 658914, 853361, 307361]
    let bf = BloomFilter::initialize(100000, 0.01, b"");
    let fp = bf.fingerprint(b"bar");
    let expected: [u64; 7] = [20311, 36825, 412501, 835777, 658914, 853361, 307361];
    assert_eq!(fp.len(), 7);
    for i in 0..7 {
        assert_eq!(fp[i], expected[i], "fingerprint[{}] mismatch", i);
    }
}

#[test]
fn test_fingerprint_hello() {
    // From C: fingerprint of "hello" with init(10000, 0.001) k=10
    // => [31028, 86598, 57165, 98504, 21276, 22246, 62081, 2935, 75903, 109764]
    let bf = BloomFilter::initialize(10000, 0.001, b"");
    let fp = bf.fingerprint(b"hello");
    let expected: [u64; 10] = [
        31028, 86598, 57165, 98504, 21276, 22246, 62081, 2935, 75903, 109764,
    ];
    assert_eq!(fp.len(), 10);
    for i in 0..10 {
        assert_eq!(fp[i], expected[i], "fingerprint[{}] mismatch", i);
    }
}

#[test]
fn test_add_check_simple() {
    // From C ground truth:
    // add(alpha)=1, add(alpha)=0 (duplicate), add(beta)=1, N=2
    // check(alpha)=1, check(beta)=1, check(gamma)=0, modified=1
    let mut bf = BloomFilter::initialize(10000, 0.001, b"");
    let r1 = bf.add(b"alpha");
    let r2 = bf.add(b"alpha");
    let r3 = bf.add(b"beta");
    assert_eq!(r1, 1);
    assert_eq!(r2, 0);
    assert_eq!(r3, 1);
    assert_eq!(bf.h.n_value, 2);
    assert_eq!(bf.modified, 1);
    assert_eq!(bf.check(b"alpha"), 1);
    assert_eq!(bf.check(b"beta"), 1);
    assert_eq!(bf.check(b"gamma"), 0);
}

#[test]
fn test_add_full_capacity() {
    // From C: init(2, 0.001) - after adding 2, third add returns -1
    let mut bf = BloomFilter::initialize(2, 0.001, b"");
    let r1 = bf.add(b"a");
    let r2 = bf.add(b"b");
    let r3 = bf.add(b"c");
    assert_eq!(r1, 1);
    assert_eq!(r2, 1);
    assert_eq!(r3, -1);
    assert_eq!(bf.h.n_value, 2);
}

#[test]
fn test_header_check_good() {
    // From C: header {1, 10000, 0.001, 10, 143775, 0} -> check returns 1
    let h = Header {
        version: 1,
        n: 10000,
        p: 0.001,
        k: 10,
        m: 143775,
        n_value: 0,
    };
    assert!(h.check());
}

#[test]
fn test_header_check_bad_version() {
    let h = Header {
        version: 2,
        n: 10000,
        p: 0.001,
        k: 10,
        m: 143775,
        n_value: 0,
    };
    assert!(!h.check());
}

#[test]
fn test_header_check_p_zero() {
    let h = Header {
        version: 1,
        n: 10000,
        p: 0.0,
        k: 10,
        m: 143775,
        n_value: 0,
    };
    assert!(!h.check());
}

#[test]
fn test_header_check_p_too_big() {
    let h = Header {
        version: 1,
        n: 10000,
        p: 1.5,
        k: 10,
        m: 143775,
        n_value: 0,
    };
    assert!(!h.check());
}

#[test]
fn test_header_check_n_too_big() {
    let h = Header {
        version: 1,
        n: 10000,
        p: 0.001,
        k: 10,
        m: 143775,
        n_value: 999999999,
    };
    assert!(!h.check());
}

#[test]
fn test_set_data() {
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    bf.set_data(b"meta-info");
    assert_eq!(bf.datasize, 9);
    assert_eq!(bf.data, b"meta-info");
}

#[test]
fn test_join_basic() {
    // From C: joining b into a where capacities/p match works (returns 1)
    let mut a = BloomFilter::initialize(1000, 0.001, b"");
    let mut b = BloomFilter::initialize(1000, 0.001, b"");
    a.add(b"alpha");
    a.add(b"beta");
    b.add(b"gamma");
    b.add(b"delta");
    let r = a.join(&b);
    assert_eq!(r, 1);
    assert_eq!(a.h.n_value, 4);
    // After OR-merging, all elements should be present in a
    assert_eq!(a.check(b"alpha"), 1);
    assert_eq!(a.check(b"beta"), 1);
    assert_eq!(a.check(b"gamma"), 1);
    assert_eq!(a.check(b"delta"), 1);
}

#[test]
fn test_join_mismatch_n() {
    // From C: filters with different n => return 0
    let mut a = BloomFilter::initialize(1000, 0.001, b"");
    let b = BloomFilter::initialize(2000, 0.001, b"");
    let r = a.join(&b);
    assert_eq!(r, 0);
}

#[test]
fn test_join_mismatch_p() {
    let mut a = BloomFilter::initialize(1000, 0.001, b"");
    let b = BloomFilter::initialize(1000, 0.01, b"");
    let r = a.join(&b);
    assert_eq!(r, 0);
}

#[test]
fn test_join_destination_full() {
    // After filling a to capacity, joining must return -1
    let mut a = BloomFilter::initialize(2, 0.001, b"");
    let mut b = BloomFilter::initialize(2, 0.001, b"");
    a.add(b"x");
    a.add(b"y");
    b.add(b"z");
    let r = a.join(&b);
    assert_eq!(r, -1);
}

#[test]
fn test_write_then_read_roundtrip() {
    use std::fs::File;
    let mut bf = BloomFilter::initialize(1000, 0.001, b"");
    bf.add(b"alpha");
    bf.add(b"beta");
    bf.set_data(b"meta-info");

    let path = std::env::temp_dir().join("rust_fleur_test_roundtrip.bloom");
    let _ = std::fs::remove_file(&path);
    let f = File::create(&path).unwrap();
    let r = bf.to_file(f);
    assert_eq!(r, 1);

    let f2 = File::open(&path).unwrap();
    let bf2 = BloomFilter::from_file(f2);
    assert_eq!(bf2.error, 0);
    assert_eq!(bf2.h.n, bf.h.n);
    assert_eq!(bf2.h.p, bf.h.p);
    assert_eq!(bf2.h.k, bf.h.k);
    assert_eq!(bf2.h.m, bf.h.m);
    assert_eq!(bf2.h.n_value, bf.h.n_value);
    assert_eq!(bf2.m, bf.m);
    assert_eq!(bf2.v, bf.v);
    assert_eq!(bf2.datasize, 9);
    assert_eq!(bf2.data, b"meta-info");

    // Items should still be findable
    assert_eq!(bf2.check(b"alpha"), 1);
    assert_eq!(bf2.check(b"beta"), 1);
    assert_eq!(bf2.check(b"gamma"), 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_bad_version_file() {
    // hang1.bin has bad header version
    let p = "c_src/test/suite_2/hang1.bin";
    if let Ok(f) = std::fs::File::open(p) {
        let bf = BloomFilter::from_file(f);
        assert_eq!(bf.error, 1);
    }
}

#[test]
fn test_read_p_too_small_file() {
    // hang0.bin has p too small
    let p = "c_src/test/suite_2/hang0.bin";
    if let Ok(f) = std::fs::File::open(p) {
        let bf = BloomFilter::from_file(f);
        assert_eq!(bf.error, 1);
    }
}

#[test]
fn test_read_p_above_one_file() {
    // hang2.bin has p > 1
    let p = "c_src/test/suite_2/hang2.bin";
    if let Ok(f) = std::fs::File::open(p) {
        let bf = BloomFilter::from_file(f);
        assert_eq!(bf.error, 1);
    }
}

#[test]
fn test_read_header_file() {
    // header.bin: from C tests:
    // version=1, n=354253067, p=0.0001, k=14, m=6791072655, N=354249652
    let p = "c_src/test/suite_2/header.bin";
    if let Ok(mut f) = std::fs::File::open(p) {
        use std::io::Read;
        let mut buf = [0u8; 48];
        f.read_exact(&mut buf).unwrap();
        let version = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        let n = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        let p_val = f64::from_le_bytes(buf[16..24].try_into().unwrap());
        let k = u64::from_le_bytes(buf[24..32].try_into().unwrap());
        let m = u64::from_le_bytes(buf[32..40].try_into().unwrap());
        let nval = u64::from_le_bytes(buf[40..48].try_into().unwrap());
        assert_eq!(version, 1);
        assert_eq!(n, 354253067);
        assert!((p_val - 0.0001).abs() < 1e-9);
        assert_eq!(k, 14);
        assert_eq!(m, 6791072655);
        assert_eq!(nval, 354249652);

        let h = Header {
            version,
            n,
            p: p_val,
            k,
            m,
            n_value: nval,
        };
        assert!(h.check());
    }
}

#[test]
fn test_read_datatest_file() {
    // datatest.bloom: from C tests, M (number of u64s) = 450
    let p = "c_src/test/suite_2/datatest.bloom";
    if let Ok(f) = std::fs::File::open(p) {
        let bf = BloomFilter::from_file(f);
        assert_eq!(bf.error, 0);
        assert_eq!(bf.m, 450);
    }
}

#[test]
fn test_display_header() {
    // Verify Display works without panicking
    let h = Header {
        version: 1,
        n: 10000,
        p: 0.001,
        k: 10,
        m: 143775,
        n_value: 0,
    };
    let s = format!("{}", h);
    assert!(s.contains("Header details"));
    assert!(s.contains("10000"));
    assert!(s.contains("143775"));
}

#[test]
fn test_display_filter() {
    let bf = BloomFilter::initialize(10000, 0.001, b"hello");
    let s = format!("{}", bf);
    assert!(s.contains("Filter details"));
    assert!(s.contains("10000"));
    assert!(s.contains("143775"));
}

fn main() {}
