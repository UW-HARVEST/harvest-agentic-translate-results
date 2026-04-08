use gfc::gfc::GFC;

#[test]
fn test_encrypt_range10_rounds6_seed42() {
    let g = GFC::new(10, 6, 42);
    let expected = [1, 2, 9, 4, 7, 6, 5, 8, 0, 3];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(g.encrypt(i as u64), exp, "encrypt({i})");
    }
}

#[test]
fn test_decrypt_range10_rounds6_seed42() {
    let g = GFC::new(10, 6, 42);
    let enc = [1, 2, 9, 4, 7, 6, 5, 8, 0, 3];
    for (i, &e) in enc.iter().enumerate() {
        assert_eq!(g.decrypt(e), i as u64, "decrypt({e}) should be {i}");
    }
}

#[test]
fn test_encrypt_range100_rounds6_seed42() {
    let g = GFC::new(100, 6, 42);
    let expected = [12, 8, 50, 72, 43, 25, 68, 65, 19, 88];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(g.encrypt(i as u64), exp, "encrypt({i})");
    }
}

#[test]
fn test_encrypt_range1000_rounds6_seed123() {
    let g = GFC::new(1000, 6, 123);
    let expected = [690, 729, 326, 652, 525, 950, 809, 50, 704, 984];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(g.encrypt(i as u64), exp, "encrypt({i})");
    }
}

#[test]
fn test_encrypt_range1_rounds6_seed42() {
    let g = GFC::new(1, 6, 42);
    assert_eq!(g.encrypt(0), 0);
}

#[test]
fn test_encrypt_range10_rounds1_seed42() {
    let g = GFC::new(10, 1, 42);
    let expected = [2, 3, 0, 1, 5, 6, 7, 4, 8, 9];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(g.encrypt(i as u64), exp, "encrypt({i})");
    }
}

#[test]
fn test_decrypt_roundtrip_range10_rounds1() {
    let g = GFC::new(10, 1, 42);
    for i in 0..10u64 {
        assert_eq!(g.decrypt(g.encrypt(i)), i, "roundtrip({i})");
    }
}

#[test]
fn test_encrypt_range100_rounds6_seed0() {
    let g = GFC::new(100, 6, 0);
    let expected = [47, 41, 6, 50, 27];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(g.encrypt(i as u64), exp, "encrypt({i})");
    }
}

#[test]
fn test_bijection_range100() {
    let g = GFC::new(100, 6, 42);
    let mut seen = [false; 100];
    for i in 0..100u64 {
        let enc = g.encrypt(i);
        assert!(enc < 100, "encrypt({i}) = {enc} out of range");
        assert!(!seen[enc as usize], "duplicate output {enc}");
        seen[enc as usize] = true;
    }
}

#[test]
fn test_decrypt_roundtrip_range100() {
    let g = GFC::new(100, 6, 42);
    for i in 0..100u64 {
        assert_eq!(g.decrypt(g.encrypt(i)), i, "roundtrip({i})");
    }
}

fn main() {}
