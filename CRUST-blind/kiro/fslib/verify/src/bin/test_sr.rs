use fslib::sr::*;

#[test]
fn test_tropical_sum() {
    assert_eq!(tropical_sum(3.0, 5.0), 3.0);
    assert_eq!(tropical_sum(5.0, 3.0), 3.0);
    assert_eq!(tropical_sum(0.0, 0.0), 0.0);
}

#[test]
fn test_tropical_product() {
    assert_eq!(tropical_product(3.0, 5.0), 8.0);
    assert_eq!(tropical_product(0.0, 0.0), 0.0);
}

#[test]
fn test_real_sum() {
    assert_eq!(real_sum(3.0, 5.0), 8.0);
    assert_eq!(real_sum(0.0, 0.0), 0.0);
}

#[test]
fn test_real_product() {
    assert_eq!(real_product(3.0, 5.0), 15.0);
    assert_eq!(real_product(1.0, 0.0), 0.0);
}

#[test]
fn test_sr_tropical_constants() {
    assert_eq!(SR_TROPICAL.zero, f32::MAX);
    assert_eq!(SR_TROPICAL.one, 0.0);
}

#[test]
fn test_sr_real_constants() {
    assert_eq!(SR_REAL.zero, 0.0);
    assert_eq!(SR_REAL.one, 1.0);
}

#[test]
fn test_sr_get() {
    let t = sr_get(0); // SR_TROPICAL
    assert_eq!(t.zero, f32::MAX);
    assert_eq!(t.one, 0.0);
    assert_eq!((t.sum)(3.0, 5.0), 3.0);
    assert_eq!((t.prod)(3.0, 5.0), 8.0);

    let r = sr_get(1); // SR_REAL
    assert_eq!(r.zero, 0.0);
    assert_eq!(r.one, 1.0);
    assert_eq!((r.sum)(3.0, 5.0), 8.0);
    assert_eq!((r.prod)(3.0, 5.0), 15.0);
}

#[test]
fn test_sr_get_default() {
    // Unknown type defaults to tropical
    let d = sr_get(99);
    assert_eq!(d.zero, f32::MAX);
    assert_eq!(d.one, 0.0);
}

fn main() {}
