use fslib::sr;

#[test]
fn test_real_sum() {
    assert_eq!(sr::real_sum(1.0, 2.0), 3.0);
    assert_eq!(sr::real_sum(0.0, 5.5), 5.5);
}

#[test]
fn test_real_product() {
    assert_eq!(sr::real_product(3.0, 4.0), 12.0);
    assert_eq!(sr::real_product(2.5, 4.0), 10.0);
}

#[test]
fn test_tropical_sum() {
    assert_eq!(sr::tropical_sum(1.0, 2.0), 1.0);
    assert_eq!(sr::tropical_sum(5.0, 2.0), 2.0);
}

#[test]
fn test_tropical_product() {
    assert_eq!(sr::tropical_product(3.0, 4.0), 7.0);
    assert_eq!(sr::tropical_product(0.0, 5.0), 5.0);
}

#[test]
fn test_sr_get_tropical() {
    let s = sr::sr_get(0);
    assert_eq!(s.zero, f32::MAX);
    assert_eq!(s.one, 0.0);
    assert_eq!((s.sum)(1.0, 2.0), 1.0);
    assert_eq!((s.prod)(3.0, 4.0), 7.0);
}

#[test]
fn test_sr_get_real() {
    let s = sr::sr_get(1);
    assert_eq!(s.zero, 0.0);
    assert_eq!(s.one, 1.0);
    assert_eq!((s.sum)(1.0, 2.0), 3.0);
    assert_eq!((s.prod)(3.0, 4.0), 12.0);
}

#[test]
fn test_sr_get_default_is_tropical() {
    let s = sr::sr_get(99);
    assert_eq!(s.zero, f32::MAX);
    assert_eq!(s.one, 0.0);
}

fn main() {}
