use emlang::data::{Data, DataType, DataValue};

#[test]
fn test_data_new_int_default() {
    let d = Data::new(DataType::Int);
    assert_eq!(d.dtype, DataType::Int);
    match d.value {
        DataValue::Int(v) => assert_eq!(v, 0),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_data_new_str_default() {
    let d = Data::new(DataType::Str);
    assert_eq!(d.dtype, DataType::Str);
    match d.value {
        DataValue::Str(ref s) => assert_eq!(s, ""),
        _ => panic!("expected Str"),
    }
}

#[test]
fn test_data_new_int_positive() {
    let d = Data::new_int(42);
    assert_eq!(d.dtype, DataType::Int);
    match d.value {
        DataValue::Int(v) => assert_eq!(v, 42),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_data_new_int_negative() {
    let d = Data::new_int(-100);
    assert_eq!(d.dtype, DataType::Int);
    match d.value {
        DataValue::Int(v) => assert_eq!(v, -100),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_data_new_str() {
    let d = Data::new_str("hello".to_string());
    assert_eq!(d.dtype, DataType::Str);
    match d.value {
        DataValue::Str(ref s) => assert_eq!(s, "hello"),
        _ => panic!("expected Str"),
    }
}

#[test]
fn test_data_type_display() {
    assert_eq!(format!("{}", DataType::Int), "int");
    assert_eq!(format!("{}", DataType::Str), "str");
}

#[test]
fn test_data_display_int() {
    // C uses (int) cast, so i64 displayed as i32
    assert_eq!(format!("{}", Data::new_int(123)), "123");
    assert_eq!(format!("{}", Data::new_int(-999)), "-999");
    assert_eq!(format!("{}", Data::new_int(0)), "0");
    assert_eq!(format!("{}", Data::new_int(2147483647)), "2147483647");
    assert_eq!(format!("{}", Data::new_int(-2147483648)), "-2147483648");
}

#[test]
fn test_data_display_int_overflow() {
    // 3000000000 as i64 cast to i32 = -1294967296
    assert_eq!(format!("{}", Data::new_int(3000000000)), "-1294967296");
}

#[test]
fn test_data_display_str() {
    assert_eq!(format!("{}", Data::new_str("hello".to_string())), "hello");
}

fn main() {}
