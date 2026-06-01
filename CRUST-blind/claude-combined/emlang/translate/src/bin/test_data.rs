use emlang::data::{Data, DataType, DataValue};

#[test]
fn test_new_int() {
    let d = Data::new_int(42);
    assert_eq!(d.dtype, DataType::Int);
    match d.value {
        DataValue::Int(v) => assert_eq!(v, 42),
        _ => panic!("expected Int value"),
    }
}

#[test]
fn test_new_int_negative() {
    let d = Data::new_int(-99);
    assert_eq!(d.dtype, DataType::Int);
    match d.value {
        DataValue::Int(v) => assert_eq!(v, -99),
        _ => panic!("expected Int value"),
    }
}

#[test]
fn test_new_str() {
    let d = Data::new_str("hello".to_string());
    assert_eq!(d.dtype, DataType::Str);
    match d.value {
        DataValue::Str(s) => assert_eq!(s, "hello"),
        _ => panic!("expected Str value"),
    }
}

#[test]
fn test_new_default_int() {
    let d = Data::new(DataType::Int);
    assert_eq!(d.dtype, DataType::Int);
    match d.value {
        DataValue::Int(v) => assert_eq!(v, 0),
        _ => panic!("expected Int value"),
    }
}

#[test]
fn test_data_type_display() {
    assert_eq!(format!("{}", DataType::Int), "int");
    assert_eq!(format!("{}", DataType::Str), "str");
}

#[test]
fn test_data_display_int() {
    let d = Data::new_int(7);
    assert_eq!(format!("{}", d), "7");
}

#[test]
fn test_data_display_int_negative() {
    let d = Data::new_int(-15);
    assert_eq!(format!("{}", d), "-15");
}

#[test]
fn test_data_display_str() {
    let d = Data::new_str("Hello, world!".to_string());
    assert_eq!(format!("{}", d), "Hello, world!");
}

fn main() {}
