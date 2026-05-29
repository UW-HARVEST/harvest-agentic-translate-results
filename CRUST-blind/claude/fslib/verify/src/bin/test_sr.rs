use fslib::sr::{self, SR_REAL, SR_TROPICAL, real_product, real_sum, sr_get, tropical_product, tropical_sum};

#[test]
fn test_real_sum() {
    assert_eq!(real_sum(3.0, 5.0), 8.0);
    assert_eq!(real_sum(0.0, 5.0), 5.0);
    assert_eq!(real_sum(-1.0, 1.0), 0.0);
}

#[test]
fn test_real_product() {
    assert_eq!(real_product(3.0, 5.0), 15.0);
    assert_eq!(real_product(0.0, 5.0), 0.0);
    assert_eq!(real_product(2.0, 4.0), 8.0);
}

#[test]
fn test_tropical_sum() {
    // tropical sum is min
    assert_eq!(tropical_sum(3.0, 5.0), 3.0);
    assert_eq!(tropical_sum(5.0, 3.0), 3.0);
    assert_eq!(tropical_sum(-1.0, 0.0), -1.0);
}

#[test]
fn test_tropical_product() {
    // tropical product is sum
    assert_eq!(tropical_product(3.0, 5.0), 8.0);
    assert_eq!(tropical_product(0.0, 5.0), 5.0);
    assert_eq!(tropical_product(-1.0, 1.0), 0.0);
}

#[test]
fn test_sr_real_consts() {
    assert_eq!(SR_REAL.zero, 0.0);
    assert_eq!(SR_REAL.one, 1.0);
    assert_eq!((SR_REAL.sum)(3.0, 5.0), 8.0);
    assert_eq!((SR_REAL.prod)(3.0, 5.0), 15.0);
}

#[test]
fn test_sr_tropical_consts() {
    assert_eq!(SR_TROPICAL.zero, f32::MAX);
    assert_eq!(SR_TROPICAL.one, 0.0);
    assert_eq!((SR_TROPICAL.sum)(3.0, 5.0), 3.0);
    assert_eq!((SR_TROPICAL.prod)(3.0, 5.0), 8.0);
}

#[test]
fn test_sr_get_tropical() {
    let s = sr_get(0); // SR_TROPICAL
    assert_eq!(s.zero, f32::MAX);
    assert_eq!(s.one, 0.0);
    assert_eq!((s.sum)(3.0, 5.0), 3.0);
    assert_eq!((s.prod)(3.0, 5.0), 8.0);
}

#[test]
fn test_sr_get_real() {
    let s = sr_get(1); // SR_REAL
    assert_eq!(s.zero, 0.0);
    assert_eq!(s.one, 1.0);
    assert_eq!((s.sum)(3.0, 5.0), 8.0);
    assert_eq!((s.prod)(3.0, 5.0), 15.0);
}

#[test]
fn test_sr_get_unknown_defaults_tropical() {
    let s = sr_get(255);
    assert_eq!(s.zero, f32::MAX);
    assert_eq!(s.one, 0.0);
}

#[test]
fn test_sr_types_array() {
    assert_eq!(sr::SR_TYPES[0].zero, f32::MAX); // tropical
    assert_eq!(sr::SR_TYPES[0].one, 0.0);
    assert_eq!(sr::SR_TYPES[1].zero, 0.0); // real
    assert_eq!(sr::SR_TYPES[1].one, 1.0);
}

fn main() {}
