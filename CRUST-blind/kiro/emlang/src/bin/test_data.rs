use emlang::data::{Data, DataType, DataValue};

#[test]
fn test_new_default_int() {
    let d = Data::new(DataType::Int);
    assert_eq!(d.dtype, DataType::Int);
    assert!(matches!(d.value, DataValue::Int(0)));
}

#[test]
fn test_new_default_str() {
    let d = Data::new(DataType::Str);
    assert_eq!(d.dtype, DataType::Str);
    if let DataValue::Str(ref s) = d.value {
        assert_eq!(s, "");
    } else {
        panic!("expected Str variant");
    }
}

#[test]
fn test_new_int() {
    let d = Data::new_int(42);
    assert_eq!(d.dtype, DataType::Int);
    assert!(matches!(d.value, DataValue::Int(42)));
}

#[test]
fn test_new_int_negative() {
    let d = Data::new_int(-100);
    assert!(matches!(d.value, DataValue::Int(-100)));
}

#[test]
fn test_new_int_zero() {
    let d = Data::new_int(0);
    assert!(matches!(d.value, DataValue::Int(0)));
}

#[test]
fn test_new_str() {
    let d = Data::new_str("hello".to_string());
    assert_eq!(d.dtype, DataType::Str);
    if let DataValue::Str(ref s) = d.value {
        assert_eq!(s, "hello");
    } else {
        panic!("expected Str variant");
    }
}

#[test]
fn test_new_str_empty() {
    let d = Data::new_str(String::new());
    if let DataValue::Str(ref s) = d.value {
        assert_eq!(s, "");
    } else {
        panic!("expected Str variant");
    }
}

#[test]
fn test_display_int_as_i32() {
    // C code casts int64_t to (int) for printing, Rust should cast to i32
    let d = Data::new_int(42);
    assert_eq!(format!("{}", d), "42");

    let d = Data::new_int(-5);
    assert_eq!(format!("{}", d), "-5");

    let d = Data::new_int(0);
    assert_eq!(format!("{}", d), "0");
}

#[test]
fn test_display_int_i32_overflow() {
    // i64 value that overflows i32 should wrap
    let d = Data::new_int(0x1_0000_0000); // 2^32
    assert_eq!(format!("{}", d), "0");

    let d = Data::new_int(0x1_0000_0001);
    assert_eq!(format!("{}", d), "1");
}

#[test]
fn test_display_str() {
    let d = Data::new_str("Hello, world!".to_string());
    assert_eq!(format!("{}", d), "Hello, world!");
}

#[test]
fn test_display_datatype() {
    assert_eq!(format!("{}", DataType::Int), "int");
    assert_eq!(format!("{}", DataType::Str), "str");
}

#[test]
fn test_clone() {
    let d = Data::new_int(7);
    let d2 = d.clone();
    assert_eq!(format!("{}", d), format!("{}", d2));

    let d = Data::new_str("test".to_string());
    let d2 = d.clone();
    assert_eq!(format!("{}", d), format!("{}", d2));
}

fn main() {}
