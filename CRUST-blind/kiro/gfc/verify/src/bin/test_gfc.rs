use gfc::gfc::GFC;

#[test]
fn test_encrypt_range10_rounds1_seed42() {
    let gfc = GFC::new(10, 1, 42);
    let expected = [2, 3, 0, 1, 5, 6, 7, 4, 8, 9];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(gfc.encrypt(i as u64), exp, "encrypt({i})");
    }
}

#[test]
fn test_encrypt_range10_rounds6_seed123() {
    let gfc = GFC::new(10, 6, 123);
    let expected = [7, 2, 9, 1, 5, 8, 3, 0, 6, 4];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(gfc.encrypt(i as u64), exp, "encrypt({i})");
    }
}

#[test]
fn test_encrypt_range100_rounds6_seed42() {
    let gfc = GFC::new(100, 6, 42);
    let expected: [u64; 100] = [
        12, 8, 50, 72, 43, 25, 68, 65, 19, 88,
        0, 41, 69, 23, 58, 21, 62, 95, 99, 96,
        66, 71, 11, 6, 7, 78, 59, 46, 80, 67,
        77, 18, 35, 31, 13, 55, 28, 2, 51, 38,
        93, 36, 56, 10, 97, 76, 63, 42, 79, 83,
        86, 82, 32, 37, 27, 44, 5, 84, 24, 98,
        33, 26, 91, 9, 16, 45, 73, 53, 4, 81,
        60, 15, 20, 52, 3, 17, 40, 87, 30, 54,
        74, 1, 34, 14, 48, 22, 85, 90, 70, 49,
        39, 57, 92, 47, 75, 29, 61, 64, 89, 94,
    ];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(gfc.encrypt(i as u64), exp, "encrypt({i})");
    }
}

#[test]
fn test_roundtrip_range100_rounds6_seed42() {
    let gfc = GFC::new(100, 6, 42);
    for i in 0u64..100 {
        let enc = gfc.encrypt(i);
        let dec = gfc.decrypt(enc);
        assert_eq!(dec, i, "roundtrip failed for {i}: encrypt={enc}, decrypt={dec}");
    }
}

#[test]
fn test_bijection_range100_rounds6_seed42() {
    let gfc = GFC::new(100, 6, 42);
    let mut counts = vec![0u64; 100];
    for i in 0u64..100 {
        let enc = gfc.encrypt(i);
        assert!(enc < 100, "encrypt({i}) = {enc} out of range");
        counts[enc as usize] += 1;
    }
    for (i, &c) in counts.iter().enumerate() {
        assert_eq!(c, 1, "counts[{i}] = {c}, expected 1");
    }
}

#[test]
fn test_edge_case_range1_rounds0() {
    let gfc = GFC::new(1, 0, 42);
    assert_eq!(gfc.encrypt(0), 0);
    assert_eq!(gfc.decrypt(0), 0);
}

#[test]
fn test_edge_case_range1_rounds1() {
    let gfc = GFC::new(1, 1, 42);
    assert_eq!(gfc.encrypt(0), 0);
    assert_eq!(gfc.decrypt(0), 0);
}

#[test]
fn test_roundtrip_range10_rounds1_seed42() {
    let gfc = GFC::new(10, 1, 42);
    for i in 0u64..10 {
        let enc = gfc.encrypt(i);
        let dec = gfc.decrypt(enc);
        assert_eq!(dec, i, "roundtrip failed for {i}");
    }
}

#[test]
fn test_roundtrip_range10_rounds6_seed123() {
    let gfc = GFC::new(10, 6, 123);
    for i in 0u64..10 {
        let enc = gfc.encrypt(i);
        let dec = gfc.decrypt(enc);
        assert_eq!(dec, i, "roundtrip failed for {i}");
    }
}

fn main() {}
